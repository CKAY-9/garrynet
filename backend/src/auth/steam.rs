//! Steam sign-in via OpenID 2.0, plus profile lookup through the Steam Web API.

use std::collections::HashMap;

use serde::Deserialize;
use url::Url;

use crate::error::ApiError;

pub const OPENID_ENDPOINT: &str = "https://steamcommunity.com/openid/login";
const OPENID_NS: &str = "http://specs.openid.net/auth/2.0";
const IDENTIFIER_SELECT: &str = "http://specs.openid.net/auth/2.0/identifier_select";
const CLAIMED_ID_PREFIX: &str = "https://steamcommunity.com/openid/id/";
const PLAYER_SUMMARIES_URL: &str =
    "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v2/";

/// Fields that must be covered by Steam's signature for us to trust them.
const REQUIRED_SIGNED: [&str; 5] = [
    "op_endpoint",
    "claimed_id",
    "identity",
    "return_to",
    "response_nonce",
];

/// Builds the Steam login URL the user is redirected to.
pub fn login_url(return_to: &str, realm: &str) -> String {
    let mut url = Url::parse(OPENID_ENDPOINT).expect("valid constant URL");
    url.query_pairs_mut()
        .append_pair("openid.ns", OPENID_NS)
        .append_pair("openid.mode", "checkid_setup")
        .append_pair("openid.return_to", return_to)
        .append_pair("openid.realm", realm)
        .append_pair("openid.identity", IDENTIFIER_SELECT)
        .append_pair("openid.claimed_id", IDENTIFIER_SELECT);
    url.into()
}

/// What Steam sent back to the callback.
#[derive(Debug, PartialEq, Eq)]
pub enum Assertion {
    /// The user backed out of the Steam login page.
    Cancelled,
    /// A well-formed positive assertion for this SteamID64. It still has to be confirmed
    /// with Steam via [`verify_with_steam`] before it can be trusted.
    Unverified { steam_id: String },
}

/// Checks the callback's OpenID parameters locally (mode, endpoint, return URL, signed
/// fields, claimed id). Doesn't contact Steam.
pub fn parse_assertion(
    params: &[(String, String)],
    expected_return_to: &str,
) -> Result<Assertion, ApiError> {
    let fail = |msg: &str| Err(ApiError::SteamVerification(msg.to_owned()));

    let mut map = HashMap::with_capacity(params.len());
    for (k, v) in params {
        // Reject duplicates so the params we check are exactly the params Steam verifies.
        if map.insert(k.as_str(), v.as_str()).is_some() {
            return fail("duplicate OpenID parameter");
        }
    }
    let get = |k: &str| map.get(k).copied();

    match get("openid.mode") {
        Some("cancel") => return Ok(Assertion::Cancelled),
        Some("id_res") => {}
        _ => return fail("unexpected openid.mode"),
    }
    if get("openid.ns") != Some(OPENID_NS) {
        return fail("unexpected openid.ns");
    }
    if get("openid.op_endpoint") != Some(OPENID_ENDPOINT) {
        return fail("unexpected openid.op_endpoint");
    }
    if get("openid.return_to") != Some(expected_return_to) {
        return fail("openid.return_to does not match");
    }

    let signed: Vec<&str> = get("openid.signed").unwrap_or_default().split(',').collect();
    if !REQUIRED_SIGNED.iter().all(|f| signed.contains(f)) {
        return fail("required fields are not signed");
    }

    let claimed_id = get("openid.claimed_id").unwrap_or_default();
    if get("openid.identity") != Some(claimed_id) {
        return fail("openid.identity does not match openid.claimed_id");
    }
    let steam_id = claimed_id
        .strip_prefix(CLAIMED_ID_PREFIX)
        .filter(|id| id.len() == 17 && id.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| ApiError::SteamVerification("invalid openid.claimed_id".into()))?;

    Ok(Assertion::Unverified {
        steam_id: steam_id.to_owned(),
    })
}

/// Asks Steam to confirm it really issued this assertion (OpenID `check_authentication`).
/// Steam also rejects a response nonce it has already verified, which prevents replays.
pub async fn verify_with_steam(
    http: &reqwest::Client,
    params: &[(String, String)],
) -> Result<(), ApiError> {
    let form: Vec<(&str, &str)> = params
        .iter()
        .filter(|(k, _)| k.starts_with("openid."))
        .map(|(k, v)| {
            if k == "openid.mode" {
                (k.as_str(), "check_authentication")
            } else {
                (k.as_str(), v.as_str())
            }
        })
        .collect();

    let body = http
        .post(OPENID_ENDPOINT)
        .form(&form)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| ApiError::SteamUnavailable(e.to_string()))?
        .text()
        .await
        .map_err(|e| ApiError::SteamUnavailable(e.to_string()))?;

    if body.lines().any(|line| line.trim() == "is_valid:true") {
        Ok(())
    } else {
        Err(ApiError::SteamVerification(
            "steam rejected the assertion".into(),
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct PlayerSummary {
    pub steamid: String,
    pub personaname: String,
    pub avatarfull: String,
}

#[derive(Deserialize)]
struct PlayerSummariesResponse {
    response: PlayerSummariesInner,
}

#[derive(Deserialize)]
struct PlayerSummariesInner {
    players: Vec<PlayerSummary>,
}

/// Fetches the user's public Steam profile (display name and avatar).
pub async fn player_summary(
    http: &reqwest::Client,
    api_key: &str,
    steam_id: &str,
) -> Result<PlayerSummary, ApiError> {
    let res: PlayerSummariesResponse = http
        .get(PLAYER_SUMMARIES_URL)
        .query(&[("key", api_key), ("steamids", steam_id)])
        .send()
        .await
        .and_then(|r| r.error_for_status())
        // reqwest errors include the URL, which contains the API key.
        .map_err(|e| ApiError::SteamUnavailable(e.without_url().to_string()))?
        .json()
        .await
        .map_err(|e| ApiError::SteamUnavailable(e.without_url().to_string()))?;

    res.response
        .players
        .into_iter()
        .find(|p| p.steamid == steam_id)
        .ok_or_else(|| ApiError::SteamUnavailable(format!("no profile returned for {steam_id}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RETURN_TO: &str = "http://localhost:8080/auth/steam/callback";
    const STEAM_ID: &str = "76561197960287930";

    fn valid() -> Vec<(String, String)> {
        let claimed = format!("{CLAIMED_ID_PREFIX}{STEAM_ID}");
        [
            ("openid.ns", OPENID_NS),
            ("openid.mode", "id_res"),
            ("openid.op_endpoint", OPENID_ENDPOINT),
            ("openid.claimed_id", &claimed),
            ("openid.identity", &claimed),
            ("openid.return_to", RETURN_TO),
            ("openid.response_nonce", "2026-11-08T12:00:00ZabcdEFGH"),
            ("openid.assoc_handle", "1234567890"),
            (
                "openid.signed",
                "signed,op_endpoint,claimed_id,identity,return_to,response_nonce,assoc_handle",
            ),
            ("openid.sig", "c2lnbmF0dXJl"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    }

    fn with(key: &str, value: &str) -> Vec<(String, String)> {
        let mut p = valid();
        p.iter_mut().find(|(k, _)| k == key).unwrap().1 = value.to_owned();
        p
    }

    fn rejected(params: Vec<(String, String)>) -> bool {
        matches!(
            parse_assertion(&params, RETURN_TO),
            Err(ApiError::SteamVerification(_))
        )
    }

    #[test]
    fn login_url_has_openid_params() {
        let url = Url::parse(&login_url(RETURN_TO, "http://localhost:8080")).unwrap();
        let q: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(q["openid.mode"], "checkid_setup");
        assert_eq!(q["openid.return_to"], RETURN_TO);
        assert_eq!(q["openid.realm"], "http://localhost:8080");
        assert_eq!(q["openid.claimed_id"], IDENTIFIER_SELECT);
    }

    #[test]
    fn accepts_valid_assertion() {
        assert_eq!(
            parse_assertion(&valid(), RETURN_TO).unwrap(),
            Assertion::Unverified {
                steam_id: STEAM_ID.into()
            }
        );
    }

    #[test]
    fn cancel() {
        let params = vec![("openid.mode".to_owned(), "cancel".to_owned())];
        assert_eq!(
            parse_assertion(&params, RETURN_TO).unwrap(),
            Assertion::Cancelled
        );
    }

    #[test]
    fn rejects_tampering() {
        assert!(rejected(with("openid.mode", "checkid_setup")));
        assert!(rejected(with("openid.op_endpoint", "https://evil.example/openid")));
        assert!(rejected(with("openid.return_to", "https://evil.example/callback")));
        assert!(rejected(with("openid.signed", "signed,op_endpoint,identity")));
        assert!(rejected(with(
            "openid.identity",
            "https://steamcommunity.com/openid/id/76561197960287931"
        )));
        let other = "https://evil.example/openid/id/76561197960287930";
        assert!(rejected(with("openid.claimed_id", other)));
        let mut both = with("openid.claimed_id", other);
        both.iter_mut().find(|(k, _)| k == "openid.identity").unwrap().1 = other.into();
        assert!(rejected(both));
        let short = "https://steamcommunity.com/openid/id/123";
        let mut both = with("openid.claimed_id", short);
        both.iter_mut().find(|(k, _)| k == "openid.identity").unwrap().1 = short.into();
        assert!(rejected(both));
    }

    #[test]
    fn rejects_duplicate_params() {
        let mut p = valid();
        p.push((
            "openid.claimed_id".into(),
            "https://steamcommunity.com/openid/id/76561197960287931".into(),
        ));
        assert!(rejected(p));
    }
}
