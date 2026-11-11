//! Banning and suspending users (docs/USERS.md "User Punishment"). Issuing and lifting need
//! Admin Access and User Management, and can't target yourself or anyone holding
//! permissions you don't.

use actix_web::{HttpResponse, get, post, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, Punishment, User, Usergroup,
        punishment::{NewPunishment, PunishmentKind, PunishmentRecord},
    },
    notify,
    routes::validate::{bad, required_text, text},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list).service(issue).service(lift);
}

/// Like file moderation, managing users needs Admin Access as well as User Management.
const MANAGE: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::USER_MANAGEMENT);

const MAX_REASON: usize = 500;

#[derive(Serialize)]
struct UserRef {
    id: i64,
    username: String,
}

/// A punishment as returned by the API.
#[derive(Serialize)]
struct PunishmentView {
    id: i64,
    kind: PunishmentKind,
    reason: String,
    created_at: DateTime<Utc>,
    ends: Option<DateTime<Utc>>,
    /// Enforced right now.
    in_effect: bool,
    /// Who issued it; only shown to staff. `None` for the shell or a deleted account.
    issuer: Option<UserRef>,
    /// When it was lifted early, if it was.
    lifted_at: Option<DateTime<Utc>>,
    lifted_by: Option<UserRef>,
    lift_reason: Option<String>,
}

impl PunishmentView {
    /// `staff` decides whether the staff members involved are named.
    fn new(record: PunishmentRecord, staff: bool) -> Self {
        let p = record.punishment;
        let user_ref = |id: Option<i64>, name: Option<String>| match (staff, id, name) {
            (true, Some(id), Some(username)) => Some(UserRef { id, username }),
            _ => None,
        };
        Self {
            in_effect: p.in_effect(),
            issuer: user_ref(p.issuer_id, record.issuer_name),
            lifted_by: user_ref(p.lifted_by, record.lifter_name),
            id: p.id,
            kind: p.kind,
            reason: p.reason,
            created_at: p.created_at,
            ends: p.ends,
            lifted_at: p.lifted_at,
            lift_reason: p.lift_reason,
        }
    }

    fn issued(punishment: Punishment, issuer: &User) -> Self {
        Self::new(
            PunishmentRecord {
                punishment,
                issuer_name: Some(issuer.username.clone()),
                lifter_name: None,
            },
            true,
        )
    }
}

/// The user's punishments, newest first. Staff (Admin Access and User Management) see the
/// whole history with who issued and lifted each one; users see only their own punishments
/// in effect, without staff names. 403 for anyone else.
#[get("/users/{id}/punishments")]
async fn list(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let id = id.into_inner();
    let staff = Usergroup::permissions_for_user(&state.db, user.0.id)
        .await?
        .contains(MANAGE);
    if !staff && user.0.id != id {
        user.require(&state.db, MANAGE).await?;
    }
    if staff && User::find_by_id(&state.db, id).await?.is_none() {
        return Err(ApiError::NotFound("user"));
    }

    let views: Vec<PunishmentView> = Punishment::history(&state.db, id)
        .await?
        .into_iter()
        .map(|record| PunishmentView::new(record, staff))
        .filter(|view| staff || view.in_effect)
        .collect();
    Ok(HttpResponse::Ok().json(views))
}

#[derive(Deserialize)]
struct IssueBody {
    kind: PunishmentKind,
    reason: String,
    /// When it ends; omitted or null for a permanent ban. Required for suspensions.
    ends: Option<DateTime<Utc>>,
}

/// Bans or suspends the user (201, the new punishment). 409 if one of that kind is already
/// in effect: lift it first to change it.
#[post("/users/{id}/punishments")]
async fn issue(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<IssueBody>,
) -> Result<HttpResponse, ApiError> {
    let target = check_target(&state, &user, id.into_inner()).await?;
    let body = body.into_inner();
    let reason = required_text("reason", &body.reason, MAX_REASON)?;
    check_ends(body.kind, body.ends, Utc::now())?;

    let punishment = Punishment::issue(
        &state.db,
        &NewPunishment {
            user_id: target.id,
            kind: body.kind,
            reason: &reason,
            issuer_id: Some(user.0.id),
            ends: body.ends,
        },
    )
    .await?
    .ok_or(ApiError::Conflict(match body.kind {
        PunishmentKind::Ban => "an active ban for this user",
        PunishmentKind::Suspend => "an active suspension for this user",
    }))?;

    log::info!(
        "user {} issued {} {} to user {} ({}), ending {}: {reason}",
        user.0.id,
        punishment.kind.noun(),
        punishment.id,
        target.id,
        target.steam_id,
        punishment.ends.map_or("never".to_owned(), |e| e.to_rfc3339()),
    );
    notify::punishment_issued(&state.db, &punishment).await;
    Ok(HttpResponse::Created().json(PunishmentView::issued(punishment, &user.0)))
}

#[derive(Deserialize)]
struct LiftBody {
    kind: PunishmentKind,
    /// Optional note on why it was lifted.
    #[serde(default)]
    reason: String,
}

/// Lifts the user's ban or suspension early (200, the lifted punishment). 404 if none of
/// that kind is in effect.
#[post("/users/{id}/punishments/lift")]
async fn lift(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<LiftBody>,
) -> Result<HttpResponse, ApiError> {
    let target = check_target(&state, &user, id.into_inner()).await?;
    let body = body.into_inner();
    let reason = text("reason", &body.reason, MAX_REASON)?;
    let reason = (!reason.is_empty()).then_some(reason);

    let punishment = Punishment::lift(&state.db, target.id, body.kind, Some(user.0.id), reason.as_deref())
        .await?
        .ok_or(ApiError::NotFound(match body.kind {
            PunishmentKind::Ban => "active ban",
            PunishmentKind::Suspend => "active suspension",
        }))?;

    log::info!(
        "user {} lifted {} {} of user {} ({}){}",
        user.0.id,
        punishment.kind.noun(),
        punishment.id,
        target.id,
        target.steam_id,
        reason.as_deref().map(|r| format!(": {r}")).unwrap_or_default(),
    );
    notify::punishment_lifted(&state.db, &punishment, Some(user.0.id)).await;
    Ok(HttpResponse::Ok().json(PunishmentView::new(
        PunishmentRecord {
            issuer_name: match punishment.issuer_id {
                Some(id) => User::find_by_id(&state.db, id).await?.map(|u| u.username),
                None => None,
            },
            lifter_name: Some(user.0.username.clone()),
            punishment,
        },
        true,
    )))
}

/// The user to punish or pardon, once `editor` is allowed to: they need Admin Access and
/// User Management, can't target themselves, and must hold every permission the target's
/// usergroups grant (punishments aside, so a banned admin still outranks a moderator).
async fn check_target(state: &AppState, editor: &AuthUser, id: i64) -> Result<User, ApiError> {
    let permissions = editor.require(&state.db, MANAGE).await?;
    if editor.0.id == id {
        return Err(bad("you can't ban or suspend yourself, or lift your own".to_owned()));
    }
    let target = User::find_by_id(&state.db, id)
        .await?
        .ok_or(ApiError::NotFound("user"))?;
    let outranking = Usergroup::group_permissions_for_user(&state.db, target.id)
        .await?
        .difference(permissions);
    if !outranking.is_empty() {
        return Err(ApiError::Forbidden(format!(
            "{} (this user holds permissions you don't)",
            outranking.names().collect::<Vec<_>>().join(", ")
        )));
    }
    Ok(target)
}

/// An end must be in the future. Bans may be permanent; suspensions always end.
fn check_ends(kind: PunishmentKind, ends: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Result<(), ApiError> {
    match ends {
        None if kind == PunishmentKind::Suspend => Err(bad("a suspension needs an end date".to_owned())),
        Some(ends) if ends <= now => Err(bad("the end date must be in the future".to_owned())),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;

    use super::*;

    #[test]
    fn ends() {
        let now = Utc::now();
        let later = Some(now + TimeDelta::days(1));
        let earlier = Some(now - TimeDelta::days(1));
        assert!(check_ends(PunishmentKind::Ban, None, now).is_ok());
        assert!(check_ends(PunishmentKind::Ban, later, now).is_ok());
        assert!(check_ends(PunishmentKind::Ban, earlier, now).is_err());
        assert!(check_ends(PunishmentKind::Suspend, None, now).is_err());
        assert!(check_ends(PunishmentKind::Suspend, later, now).is_ok());
        assert!(check_ends(PunishmentKind::Suspend, Some(now), now).is_err());
    }
}
