//! A small Stripe client for the store (docs/STORE.md): hosted Checkout Sessions, refunds
//! and webhook signature verification. Talks to Stripe's REST API directly, so there's no
//! SDK to keep in step with.

use hmac::{Hmac, Mac};
use serde::{Deserialize, de::DeserializeOwned};
use sha2::Sha256;

use crate::{config::StripeConfig, error::ApiError, models::store::PaymentRefunds};

const API: &str = "https://api.stripe.com/v1";

/// Webhook events older than this are rejected, as Stripe's own libraries do.
const SIGNATURE_TOLERANCE_SECS: i64 = 300;

/// Stripe shows at most this much of a product description.
const MAX_DESCRIPTION: usize = 500;

/// Metadata key naming the deployment that opened a Checkout Session (see
/// `Config::deployment`), on the session and its PaymentIntent.
pub const DEPLOYMENT_METADATA: &str = "gravnet_deployment";

#[derive(Clone)]
pub struct Stripe {
    http: reqwest::Client,
    config: StripeConfig,
    /// See `Config::deployment`.
    deployment: String,
}

/// A Checkout Session, as far as the store needs it.
#[derive(Debug, Deserialize)]
pub struct CheckoutSession {
    pub id: String,
    /// Where to send the buyer. `None` once the session is no longer open.
    pub url: Option<String>,
    /// `open`, `complete` or `expired`.
    #[serde(default)]
    pub status: Option<String>,
    /// `paid`, `unpaid` (e.g. a bank debit still clearing) or `no_payment_required`.
    #[serde(default)]
    pub payment_status: Option<String>,
    /// The PaymentIntent's id, once there is one.
    #[serde(default)]
    pub payment_intent: Option<String>,
    /// The total before tax and shipping, in minor units of `currency`.
    #[serde(default)]
    pub amount_subtotal: Option<i64>,
    /// ISO 4217, lowercase.
    #[serde(default)]
    pub currency: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Refund {
    pub id: String,
    /// `pending`, `requires_action`, `succeeded`, `failed` or `canceled`.
    pub status: Option<String>,
    /// Why it failed, if it did (e.g. `expired_or_canceled_card`).
    #[serde(default)]
    pub failure_reason: Option<String>,
}

impl Refund {
    /// Whether it failed or was cancelled, so no money goes back.
    pub fn failed(&self) -> bool {
        refund_failed(self.status.as_deref())
    }
}

fn refund_failed(status: Option<&str>) -> bool {
    matches!(status, Some("failed" | "canceled"))
}

/// A page of a Stripe list.
#[derive(Debug, Deserialize)]
struct List<T> {
    data: Vec<T>,
    #[serde(default)]
    has_more: bool,
}

/// A Charge, as far as its refunds go.
#[derive(Debug, Deserialize)]
struct Charge {
    #[serde(default)]
    amount_captured: i64,
    /// Stripe's total, pending refunds included.
    #[serde(default)]
    amount_refunded: i64,
    /// Present when expanded (older API versions include it anyway).
    #[serde(default)]
    refunds: Option<List<ChargeRefund>>,
}

#[derive(Debug, Deserialize)]
struct ChargeRefund {
    amount: i64,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    failure_reason: Option<String>,
    /// Unix seconds.
    #[serde(default)]
    created: i64,
}

/// Adds up a payment's charges and their refunds. Refunds count unless they failed or were
/// cancelled; when a charge's refunds aren't all listed, Stripe's own total for it is used.
fn summarize(charges: &[Charge]) -> PaymentRefunds {
    let mut summary = PaymentRefunds::default();
    let mut latest: Option<&ChargeRefund> = None;
    for charge in charges {
        summary.amount_charged += charge.amount_captured;
        summary.amount_refunded += match &charge.refunds {
            Some(list) if !list.has_more => list
                .data
                .iter()
                .filter(|r| !refund_failed(r.status.as_deref()))
                .map(|r| r.amount)
                .sum(),
            _ => charge.amount_refunded,
        };
        for refund in charge.refunds.iter().flat_map(|list| &list.data) {
            if refund_failed(refund.status.as_deref()) {
                summary.failed_refunds += 1;
            }
            if latest.is_none_or(|l| refund.created >= l.created) {
                latest = Some(refund);
            }
        }
    }
    summary.latest_status = latest.and_then(|r| r.status.clone());
    summary.latest_failure_reason = latest.and_then(|r| r.failure_reason.clone());
    summary
}

/// What a Checkout Session is for. Prices are owned by GravNet and passed inline, so no
/// Stripe Product or Price has to exist.
pub struct CheckoutParams<'a> {
    pub purchase_id: i64,
    pub buyer_id: i64,
    pub item_id: i64,
    /// ISO 4217, lowercase.
    pub currency: &'a str,
    /// Minor units.
    pub unit_amount: i64,
    pub name: &'a str,
    pub description: &'a str,
    pub image_url: Option<&'a str>,
    pub success_url: &'a str,
    pub cancel_url: &'a str,
    /// Unix seconds; Stripe allows 30 minutes to 24 hours from now.
    pub expires_at: i64,
}

/// A webhook event. `object` is the event's payload (a Checkout Session, Charge, Dispute…).
#[derive(Debug, Deserialize)]
pub struct Event {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// Whether the event is from live mode (real money) rather than test mode.
    #[serde(default)]
    pub livemode: bool,
    pub data: EventData,
}

#[derive(Debug, Deserialize)]
pub struct EventData {
    pub object: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    error: StripeError,
}

#[derive(Debug, Deserialize)]
struct StripeError {
    #[serde(default)]
    message: String,
    /// e.g. `charge_already_refunded`.
    #[serde(default)]
    code: Option<String>,
}

/// Why a request to Stripe failed.
enum Failure {
    /// Stripe refused the request (a 4xx other than 429).
    Rejected { code: Option<String>, message: String },
    /// Stripe couldn't be reached or failed.
    Unavailable(String),
}

impl From<Failure> for ApiError {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Rejected { message, .. } => ApiError::StripeRejected(message),
            Failure::Unavailable(message) => ApiError::StripeUnavailable(message),
        }
    }
}

impl Stripe {
    pub fn new(http: reqwest::Client, config: StripeConfig, deployment: String) -> Self {
        Self { http, config, deployment }
    }

    /// The deployment this client opens Checkout Sessions for (see `Config::deployment`).
    pub fn deployment(&self) -> &str {
        &self.deployment
    }

    /// An idempotency key, prefixed with the deployment so that deployments sharing a Stripe
    /// account (whose ids overlap) never reuse each other's.
    fn idempotency_key(&self, name: &str) -> String {
        format!("{}:{name}", self.deployment)
    }

    /// Opens a hosted Checkout Session for one unit of an item. Idempotent per purchase.
    pub async fn create_checkout_session(&self, p: &CheckoutParams<'_>) -> Result<CheckoutSession, ApiError> {
        let description: String = p.description.chars().take(MAX_DESCRIPTION).collect();
        let mut form: Vec<(&str, String)> = vec![
            ("mode", "payment".into()),
            ("success_url", p.success_url.into()),
            ("cancel_url", p.cancel_url.into()),
            ("client_reference_id", p.purchase_id.to_string()),
            ("expires_at", p.expires_at.to_string()),
            ("metadata[purchase_id]", p.purchase_id.to_string()),
            ("metadata[buyer_id]", p.buyer_id.to_string()),
            ("metadata[item_id]", p.item_id.to_string()),
            ("metadata[gravnet_deployment]", self.deployment.clone()),
            ("payment_intent_data[metadata][purchase_id]", p.purchase_id.to_string()),
            ("payment_intent_data[metadata][gravnet_deployment]", self.deployment.clone()),
            ("line_items[0][quantity]", "1".into()),
            ("line_items[0][price_data][currency]", p.currency.into()),
            ("line_items[0][price_data][unit_amount]", p.unit_amount.to_string()),
            ("line_items[0][price_data][product_data][name]", p.name.into()),
        ];
        if !description.trim().is_empty() {
            form.push(("line_items[0][price_data][product_data][description]", description));
        }
        if let Some(url) = p.image_url.filter(|u| u.starts_with("https://")) {
            form.push(("line_items[0][price_data][product_data][images][0]", url.into()));
        }
        if self.config.tax {
            form.push(("automatic_tax[enabled]", "true".into()));
            // Whether `unit_amount` already includes tax. Stripe Tax needs to know, and
            // without it falls back to the account's default, failing if there is none.
            form.push(("line_items[0][price_data][tax_behavior]", self.config.tax_behavior.as_str().into()));
        }
        let key = self.idempotency_key(&format!("purchase-{}", p.purchase_id));
        Ok(self.post("/checkout/sessions", &form, Some(&key)).await?)
    }

    /// Closes an open Checkout Session so it can't be paid any more. Stripe answers 400 if
    /// the session has already completed or expired; that's reported as `StripeRejected`.
    pub async fn expire_checkout_session(&self, id: &str) -> Result<CheckoutSession, ApiError> {
        Ok(self.post(&format!("/checkout/sessions/{id}/expire"), &[], None).await?)
    }

    /// The Checkout Session as Stripe has it now.
    pub async fn retrieve_checkout_session(&self, id: &str) -> Result<CheckoutSession, ApiError> {
        let req = self.http.get(format!("{API}/checkout/sessions/{id}"));
        Ok(self.send(req).await?)
    }

    /// Refunds a payment in full. Idempotent per `idempotency_key` (which is namespaced by
    /// deployment). `None` if the payment had already been refunded in full (e.g. from the
    /// Stripe dashboard).
    pub async fn refund(&self, payment_intent: &str, idempotency_key: &str) -> Result<Option<Refund>, ApiError> {
        let form = [("payment_intent", payment_intent.to_owned())];
        match self.post("/refunds", &form, Some(&self.idempotency_key(idempotency_key))).await {
            Ok(refund) => Ok(Some(refund)),
            Err(Failure::Rejected { code: Some(code), .. }) if code == "charge_already_refunded" => Ok(None),
            Err(failure) => Err(failure.into()),
        }
    }

    /// What has been charged and refunded of a payment, as Stripe stands now: its charges
    /// with their refunds.
    pub async fn payment_refunds(&self, payment_intent: &str) -> Result<PaymentRefunds, ApiError> {
        let url = url::Url::parse_with_params(
            &format!("{API}/charges"),
            [("payment_intent", payment_intent), ("limit", "100"), ("expand[]", "data.refunds")],
        )
        .map_err(|e| ApiError::Internal(format!("stripe url: {e}")))?;
        let charges: List<Charge> = self.send(self.http.get(url.as_str())).await?;
        Ok(summarize(&charges.data))
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        form: &[(&str, String)],
        idempotency_key: Option<&str>,
    ) -> Result<T, Failure> {
        let mut req = self.http.post(format!("{API}{path}")).form(form);
        if let Some(key) = idempotency_key {
            req = req.header("Idempotency-Key", key);
        }
        self.send(req).await
    }

    async fn send<T: DeserializeOwned>(&self, req: reqwest::RequestBuilder) -> Result<T, Failure> {
        let res = req
            .bearer_auth(&self.config.secret_key)
            .send()
            .await
            .map_err(|e| Failure::Unavailable(e.to_string()))?;
        let status = res.status();
        let body = res.bytes().await.map_err(|e| Failure::Unavailable(e.to_string()))?;
        if status.is_success() {
            return serde_json::from_slice(&body)
                .map_err(|e| Failure::Unavailable(format!("unexpected response: {e}")));
        }
        let (message, code) = match serde_json::from_slice::<ErrorBody>(&body) {
            Ok(b) => (b.error.message, b.error.code),
            Err(_) => (format!("HTTP {status}"), None),
        };
        // A bad or under-privileged key is GravNet's problem, not the request's, and
        // Stripe's message for it quotes part of the key, so it mustn't reach a client.
        if matches!(status.as_u16(), 401 | 403) {
            Err(Failure::Unavailable(format!("{status}: Stripe refused GravNet's API key (check STRIPE_SECRET_KEY)")))
        } else if status.is_client_error() && status.as_u16() != 429 {
            Err(Failure::Rejected { code, message })
        } else {
            Err(Failure::Unavailable(format!("{status}: {message}")))
        }
    }

    /// Checks a webhook's `Stripe-Signature` header against the raw body and returns the
    /// event. Unsigned, mis-signed and stale requests are rejected.
    pub fn verify_webhook(&self, payload: &[u8], signature: &str, now: i64) -> Result<Event, ApiError> {
        verify_signature(self.config.webhook_secret.as_bytes(), payload, signature, now)?;
        serde_json::from_slice(payload).map_err(|e| ApiError::BadRequest(format!("invalid stripe event: {e}")))
    }
}

/// The scheme (https://docs.stripe.com/webhooks#verify-manually): the header holds a
/// timestamp `t` and one or more `v1` signatures, each the hex HMAC-SHA256 of
/// `"{t}.{payload}"` under the endpoint's secret.
fn verify_signature(secret: &[u8], payload: &[u8], header: &str, now: i64) -> Result<(), ApiError> {
    let invalid = || ApiError::BadRequest("invalid stripe signature".to_owned());
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", t)) => timestamp = t.parse::<i64>().ok(),
            Some(("v1", sig)) => signatures.push(sig),
            _ => {}
        }
    }
    let timestamp = timestamp.ok_or_else(invalid)?;
    if signatures.is_empty() {
        return Err(invalid());
    }
    if (now - timestamp).abs() > SIGNATURE_TOLERANCE_SECS {
        return Err(ApiError::BadRequest("stripe signature timestamp outside tolerance".to_owned()));
    }

    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| invalid())?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload);
    // `verify_slice` compares in constant time.
    let verified = signatures
        .iter()
        .filter_map(|sig| crate::hex::decode(sig))
        .any(|sig| mac.clone().verify_slice(&sig).is_ok());
    if verified { Ok(()) } else { Err(invalid()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"whsec_test_secret";
    const PAYLOAD: &[u8] = br#"{"id":"evt_1","type":"checkout.session.completed","data":{"object":{}}}"#;

    fn sign(secret: &[u8], timestamp: i64, payload: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
        mac.update(format!("{timestamp}.").as_bytes());
        mac.update(payload);
        mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn accepts_valid_signature() {
        let header = format!("t=1000,v1={}", sign(SECRET, 1000, PAYLOAD));
        assert!(verify_signature(SECRET, PAYLOAD, &header, 1100).is_ok());
        // Several signatures (key rollover): any valid one passes.
        let header = format!("t=1000,v1=deadbeef,v1={}", sign(SECRET, 1000, PAYLOAD));
        assert!(verify_signature(SECRET, PAYLOAD, &header, 1000).is_ok());
    }

    #[test]
    fn rejects_bad_signatures() {
        let good = sign(SECRET, 1000, PAYLOAD);
        assert!(verify_signature(b"other", PAYLOAD, &format!("t=1000,v1={good}"), 1000).is_err());
        assert!(verify_signature(SECRET, b"tampered", &format!("t=1000,v1={good}"), 1000).is_err());
        assert!(verify_signature(SECRET, PAYLOAD, &format!("t=1001,v1={good}"), 1000).is_err());
        assert!(verify_signature(SECRET, PAYLOAD, &format!("v1={good}"), 1000).is_err());
        assert!(verify_signature(SECRET, PAYLOAD, "t=1000", 1000).is_err());
        assert!(verify_signature(SECRET, PAYLOAD, "", 1000).is_err());
        // Too old.
        assert!(verify_signature(SECRET, PAYLOAD, &format!("t=1000,v1={good}"), 1000 + 301).is_err());
    }

    #[test]
    fn summarizes_refunds() {
        let charges: List<Charge> = serde_json::from_value(serde_json::json!({
            "data": [
                { "amount_captured": 0, "amount_refunded": 0, "refunds": { "data": [], "has_more": false } },
                {
                    "amount_captured": 1100,
                    "amount_refunded": 600,
                    "refunds": { "has_more": false, "data": [
                        { "amount": 100, "status": "succeeded", "created": 1 },
                        { "amount": 1000, "status": "failed", "failure_reason": "expired_or_canceled_card", "created": 3 },
                        { "amount": 500, "status": "pending", "created": 2 }
                    ] }
                }
            ],
            "has_more": false
        }))
        .unwrap();
        let summary = summarize(&charges.data);
        assert_eq!(
            summary,
            PaymentRefunds {
                amount_charged: 1100,
                amount_refunded: 600,
                latest_status: Some("failed".into()),
                latest_failure_reason: Some("expired_or_canceled_card".into()),
                failed_refunds: 1,
            }
        );
        assert!(!summary.full());
        assert!(summary.falls_short());

        // Refunds not expanded: Stripe's total.
        let charges: List<Charge> = serde_json::from_value(serde_json::json!({
            "data": [{ "amount_captured": 1100, "amount_refunded": 1100 }]
        }))
        .unwrap();
        let summary = summarize(&charges.data);
        assert!(summary.full());
        assert_eq!(summary.latest_status, None);
        assert!(summarize(&[]).amount_charged == 0 && !summarize(&[]).full() && !summarize(&[]).falls_short());
    }

    #[test]
    fn parses_events() {
        let header = format!("t=5,v1={}", sign(SECRET, 5, PAYLOAD));
        // reqwest needs a crypto provider to build a client, as `main` installs one.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let stripe = Stripe::new(
            reqwest::Client::new(),
            StripeConfig {
                secret_key: "sk_test".into(),
                webhook_secret: String::from_utf8(SECRET.to_vec()).unwrap(),
                tax: false,
                tax_behavior: crate::config::TaxBehavior::Exclusive,
            },
            "api.example.com".into(),
        );
        assert_eq!(stripe.idempotency_key("purchase-7"), "api.example.com:purchase-7");
        let event = stripe.verify_webhook(PAYLOAD, &header, 5).unwrap();
        assert_eq!(event.id, "evt_1");
        assert_eq!(event.kind, "checkout.session.completed");
        assert!(stripe.verify_webhook(PAYLOAD, &header, 5000).is_err());
    }
}
