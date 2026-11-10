//! A small Stripe client for the store (docs/STORE.md): hosted Checkout Sessions, refunds
//! and webhook signature verification. Talks to Stripe's REST API directly, so there's no
//! SDK to keep in step with.

use hmac::{Hmac, Mac};
use serde::{Deserialize, de::DeserializeOwned};
use sha2::Sha256;

use crate::{config::StripeConfig, error::ApiError};

const API: &str = "https://api.stripe.com/v1";

/// Webhook events older than this are rejected, as Stripe's own libraries do.
const SIGNATURE_TOLERANCE_SECS: i64 = 300;

/// Stripe shows at most this much of a product description.
const MAX_DESCRIPTION: usize = 500;

#[derive(Clone)]
pub struct Stripe {
    http: reqwest::Client,
    config: StripeConfig,
}

/// A Checkout Session, as far as the store needs it.
#[derive(Debug, Deserialize)]
pub struct CheckoutSession {
    pub id: String,
    /// Where to send the buyer. `None` once the session is no longer open.
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Refund {
    pub id: String,
    pub status: Option<String>,
}

/// What a Checkout Session is for. Prices are owned by GarryNet and passed inline, so no
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
}

impl Stripe {
    pub fn new(http: reqwest::Client, config: StripeConfig) -> Self {
        Self { http, config }
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
            ("payment_intent_data[metadata][purchase_id]", p.purchase_id.to_string()),
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
        }
        self.post("/checkout/sessions", &form, Some(&format!("purchase-{}", p.purchase_id)))
            .await
    }

    /// Closes an open Checkout Session so it can't be paid any more. Stripe answers 400 if
    /// the session has already completed or expired; that's reported as `StripeRejected`.
    pub async fn expire_checkout_session(&self, id: &str) -> Result<CheckoutSession, ApiError> {
        self.post(&format!("/checkout/sessions/{id}/expire"), &[], None).await
    }

    /// Refunds a payment in full. Idempotent per `idempotency_key`.
    pub async fn refund(&self, payment_intent: &str, idempotency_key: &str) -> Result<Refund, ApiError> {
        let form = [("payment_intent", payment_intent.to_owned())];
        self.post("/refunds", &form, Some(idempotency_key)).await
    }

    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        form: &[(&str, String)],
        idempotency_key: Option<&str>,
    ) -> Result<T, ApiError> {
        let mut req = self
            .http
            .post(format!("{API}{path}"))
            .bearer_auth(&self.config.secret_key)
            .form(form);
        if let Some(key) = idempotency_key {
            req = req.header("Idempotency-Key", key);
        }
        let res = req
            .send()
            .await
            .map_err(|e| ApiError::StripeUnavailable(e.to_string()))?;
        let status = res.status();
        let body = res
            .bytes()
            .await
            .map_err(|e| ApiError::StripeUnavailable(e.to_string()))?;
        if status.is_success() {
            return serde_json::from_slice(&body)
                .map_err(|e| ApiError::StripeUnavailable(format!("unexpected response: {e}")));
        }
        let message = serde_json::from_slice::<ErrorBody>(&body)
            .map(|b| b.error.message)
            .unwrap_or_else(|_| format!("HTTP {status}"));
        if status.is_client_error() && status.as_u16() != 429 {
            Err(ApiError::StripeRejected(message))
        } else {
            Err(ApiError::StripeUnavailable(format!("{status}: {message}")))
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
        .filter_map(|sig| decode_hex(sig))
        .any(|sig| mac.clone().verify_slice(&sig).is_ok());
    if verified { Ok(()) } else { Err(invalid()) }
}

fn decode_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
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
            },
        );
        let event = stripe.verify_webhook(PAYLOAD, &header, 5).unwrap();
        assert_eq!(event.id, "evt_1");
        assert_eq!(event.kind, "checkout.session.completed");
        assert!(stripe.verify_webhook(PAYLOAD, &header, 5000).is_err());
    }

    #[test]
    fn hex() {
        assert_eq!(decode_hex("00ff"), Some(vec![0, 255]));
        assert_eq!(decode_hex("0"), None);
        assert_eq!(decode_hex("zz"), None);
    }
}
