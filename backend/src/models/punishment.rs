//! Website punishments (docs/USERS.md "User Punishment"): bans and suspensions. They work by
//! taking permissions away; see [`PunishmentKind::revokes`] and
//! [`Usergroup::permissions_for_user`](super::Usergroup::permissions_for_user).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::Permissions;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "punishment_kind", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum PunishmentKind {
    /// Every permission is taken away: the user can only use support (docs/AUTH.md), and,
    /// if the community enables it, can't join its game servers.
    Ban,
    /// Temporary: the user can still browse and buy, but not post or send messages.
    Suspend,
}

impl PunishmentKind {
    /// Permissions taken away while a punishment of this kind is in effect.
    pub const fn revokes(self) -> Permissions {
        match self {
            Self::Ban => Permissions::ALL,
            Self::Suspend => Permissions::USER_POSTS.union(Permissions::USER_MESSAGES),
        }
    }

    /// "ban" or "suspension".
    pub const fn noun(self) -> &'static str {
        match self {
            Self::Ban => "ban",
            Self::Suspend => "suspension",
        }
    }
}

/// `$before`, then the SQL condition (on `user_punishments` aliased `p`) for a punishment
/// that's enforced now, then `$after`. A macro so queries stay `&'static str`.
macro_rules! in_effect {
    ($before:literal, $after:literal) => {
        concat!($before, " p.active AND (p.ends IS NULL OR p.ends > now()) ", $after)
    };
}
pub(crate) use in_effect;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Punishment {
    pub id: i64,
    pub user_id: i64,
    pub kind: PunishmentKind,
    pub reason: String,
    /// `None` for the backend shell, or an issuer whose account is gone.
    pub issuer_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    /// `None` for permanent. Suspensions always end.
    pub ends: Option<DateTime<Utc>>,
    /// False once lifted early (or once an expired one was replaced).
    pub active: bool,
    pub lifted_by: Option<i64>,
    pub lifted_at: Option<DateTime<Utc>>,
    pub lift_reason: Option<String>,
}

/// A punishment with the names of who issued and lifted it, for its history.
#[derive(Debug, sqlx::FromRow)]
pub struct PunishmentRecord {
    #[sqlx(flatten)]
    pub punishment: Punishment,
    pub issuer_name: Option<String>,
    pub lifter_name: Option<String>,
}

/// A punishment to issue. Already validated (see `routes::punishments`).
pub struct NewPunishment<'a> {
    pub user_id: i64,
    pub kind: PunishmentKind,
    pub reason: &'a str,
    /// `None` for the backend shell.
    pub issuer_id: Option<i64>,
    pub ends: Option<DateTime<Utc>>,
}

/// A banned player, as the addon needs it to refuse them (docs/ADDON.md "Ban Enforcement").
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ServerBan {
    pub steam_id: String,
    pub reason: String,
    /// Unix seconds; `None` for permanent. Lua has no date parser, so not a timestamp.
    pub ends: Option<i64>,
}

/// Most history entries returned for one user.
const HISTORY_LIMIT: i64 = 100;

impl Punishment {
    /// Whether it's enforced right now: active and not yet ended.
    pub fn in_effect(&self) -> bool {
        self.active && self.ends.is_none_or(|ends| ends > Utc::now())
    }

    /// The user's punishments, newest first, at most [`HISTORY_LIMIT`].
    pub async fn history(db: &PgPool, user_id: i64) -> sqlx::Result<Vec<PunishmentRecord>> {
        sqlx::query_as(
            "SELECT p.*, issuer.username AS issuer_name, lifter.username AS lifter_name
             FROM user_punishments p
             LEFT JOIN users issuer ON issuer.id = p.issuer_id
             LEFT JOIN users lifter ON lifter.id = p.lifted_by
             WHERE p.user_id = $1
             ORDER BY p.created_at DESC, p.id DESC
             LIMIT $2",
        )
        .bind(user_id)
        .bind(HISTORY_LIMIT)
        .fetch_all(db)
        .await
    }

    /// The user's punishments that are in effect now.
    pub async fn in_effect_for(db: &PgPool, user_id: i64) -> sqlx::Result<Vec<Punishment>> {
        sqlx::query_as(in_effect!(
            "SELECT p.* FROM user_punishments p WHERE p.user_id = $1 AND",
            "ORDER BY p.kind"
        ))
        .bind(user_id)
        .fetch_all(db)
        .await
    }

    /// Issues the punishment. Returns `None` if one of the same kind is already in effect
    /// for the user. An expired one is retired (made inactive) first, keeping its history.
    pub async fn issue(db: &PgPool, new: &NewPunishment<'_>) -> sqlx::Result<Option<Punishment>> {
        let mut tx = db.begin().await?;
        sqlx::query(
            "UPDATE user_punishments SET active = false
             WHERE user_id = $1 AND kind = $2 AND active AND ends <= now()",
        )
        .bind(new.user_id)
        .bind(new.kind)
        .execute(&mut *tx)
        .await?;
        // The partial unique index allows one active punishment per kind, so a conflict
        // means one is in effect.
        let created = sqlx::query_as(
            "INSERT INTO user_punishments (user_id, kind, reason, issuer_id, ends)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT DO NOTHING
             RETURNING *",
        )
        .bind(new.user_id)
        .bind(new.kind)
        .bind(new.reason)
        .bind(new.issuer_id)
        .bind(new.ends)
        .fetch_optional(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(created)
    }

    /// Lifts the user's punishment of this kind early. Returns `None` if none is in effect.
    /// `by` is `None` for the backend shell.
    pub async fn lift(
        db: &PgPool,
        user_id: i64,
        kind: PunishmentKind,
        by: Option<i64>,
        reason: Option<&str>,
    ) -> sqlx::Result<Option<Punishment>> {
        sqlx::query_as(in_effect!(
            "UPDATE user_punishments p
             SET active = false, lifted_by = $3, lifted_at = now(), lift_reason = $4
             WHERE p.user_id = $1 AND p.kind = $2 AND",
            "RETURNING p.*"
        ))
        .bind(user_id)
        .bind(kind)
        .bind(by)
        .bind(reason)
        .fetch_optional(db)
        .await
    }

    /// Every user banned right now, by SteamID64, for the addon.
    pub async fn server_bans(db: &PgPool) -> sqlx::Result<Vec<ServerBan>> {
        sqlx::query_as(in_effect!(
            "SELECT u.steam_id, p.reason, extract(epoch FROM p.ends)::BIGINT AS ends
             FROM user_punishments p
             JOIN users u ON u.id = p.user_id
             WHERE p.kind = 'BAN' AND",
            "ORDER BY u.steam_id"
        ))
        .fetch_all(db)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revokes() {
        assert_eq!(PunishmentKind::Ban.revokes(), Permissions::ALL);
        let suspended = Permissions::BASIC.difference(PunishmentKind::Suspend.revokes());
        assert!(suspended.contains(Permissions::SITE_ACCESS));
        assert!(!suspended.contains(Permissions::USER_POSTS));
        assert!(!suspended.contains(Permissions::USER_MESSAGES));
    }
}
