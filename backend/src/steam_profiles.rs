//! A cache of Steam profiles for the public loading screen (`routes::loading_screen`),
//! which anyone can call with any SteamID64. Without it, each call for a player without a
//! GarryNet account would spend a Steam Web API request, so a script could burn through the
//! API key's daily quota (which login also needs). Profiles are cached, and fresh lookups are
//! capped per minute across everyone; past the cap the loading screen goes without an
//! avatar, as it does when Steam is slow.

use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

use crate::auth::steam::PlayerSummary;

/// How long a profile (or "no such profile") is reused.
const TTL: Duration = Duration::from_secs(60 * 60);
/// How long a failed lookup is remembered, so an outage isn't retried on every request but
/// doesn't hide avatars for long either.
const FAILURE_TTL: Duration = Duration::from_secs(5 * 60);
/// Fresh lookups allowed per minute, for everyone together. A server's players join a few
/// at a time, so this only bites when someone is scripting the endpoint.
const LOOKUPS_PER_MINUTE: u32 = 60;
const WINDOW: Duration = Duration::from_secs(60);
/// Bounds memory: once full, expired entries are dropped, and everything if that's not enough.
const MAX_ENTRIES: usize = 10_000;

/// What to do about a player's profile.
#[derive(Debug)]
pub enum Lookup {
    /// Known: the profile, or `None` if Steam had none (or failed recently).
    Cached(Option<PlayerSummary>),
    /// Not known; the caller may ask Steam and should [`SteamProfiles::store`] the outcome.
    Fetch,
    /// Not known, and the lookup budget is spent; go without.
    OverBudget,
}

#[derive(Default)]
pub struct SteamProfiles {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    /// SteamID64 → (expiry, profile).
    entries: HashMap<String, (Instant, Option<PlayerSummary>)>,
    window_start: Option<Instant>,
    lookups: u32,
}

impl SteamProfiles {
    /// The cached profile of `steam_id`, or whether it may be fetched now. Counts a `Fetch`
    /// against the budget.
    pub fn lookup(&self, steam_id: &str, now: Instant) -> Lookup {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((expires, profile)) = inner.entries.get(steam_id)
            && *expires > now
        {
            return Lookup::Cached(profile.clone());
        }
        if inner.window_start.is_none_or(|start| now.duration_since(start) >= WINDOW) {
            inner.window_start = Some(now);
            inner.lookups = 0;
        }
        if inner.lookups >= LOOKUPS_PER_MINUTE {
            return Lookup::OverBudget;
        }
        inner.lookups += 1;
        Lookup::Fetch
    }

    /// Records what Steam answered for `steam_id`: `Ok(profile)` (`None` if it has none) or
    /// `Err(())` if the lookup failed.
    pub fn store(&self, steam_id: &str, result: Result<Option<PlayerSummary>, ()>, now: Instant) {
        let (ttl, profile) = match result {
            Ok(profile) => (TTL, profile),
            Err(()) => (FAILURE_TTL, None),
        };
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.entries.len() >= MAX_ENTRIES {
            inner.entries.retain(|_, (expires, _)| *expires > now);
            if inner.entries.len() >= MAX_ENTRIES {
                inner.entries.clear();
            }
        }
        inner.entries.insert(steam_id.to_owned(), (now + ttl, profile));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str) -> PlayerSummary {
        PlayerSummary { steamid: id.into(), personaname: "Garry".into(), avatarfull: "https://a/b.jpg".into() }
    }

    #[test]
    fn caches_and_expires() {
        let cache = SteamProfiles::default();
        let t0 = Instant::now();
        assert!(matches!(cache.lookup("1", t0), Lookup::Fetch));
        cache.store("1", Ok(Some(profile("1"))), t0);
        assert!(matches!(cache.lookup("1", t0 + TTL / 2), Lookup::Cached(Some(_))));
        assert!(matches!(cache.lookup("1", t0 + TTL + WINDOW), Lookup::Fetch));

        cache.store("2", Err(()), t0);
        assert!(matches!(cache.lookup("2", t0), Lookup::Cached(None)));
        assert!(matches!(cache.lookup("2", t0 + FAILURE_TTL + WINDOW), Lookup::Fetch));
    }

    #[test]
    fn caps_fresh_lookups_per_minute() {
        let cache = SteamProfiles::default();
        let t0 = Instant::now();
        for i in 0..LOOKUPS_PER_MINUTE {
            assert!(matches!(cache.lookup(&i.to_string(), t0), Lookup::Fetch));
        }
        assert!(matches!(cache.lookup("next", t0), Lookup::OverBudget));
        // Cached profiles are still served over budget.
        cache.store("0", Ok(Some(profile("0"))), t0);
        assert!(matches!(cache.lookup("0", t0), Lookup::Cached(Some(_))));
        // A new minute brings a new budget.
        assert!(matches!(cache.lookup("next", t0 + WINDOW), Lookup::Fetch));
    }
}
