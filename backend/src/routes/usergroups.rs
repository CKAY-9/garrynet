use actix_web::{HttpResponse, delete, get, post, put, web};
use serde::Deserialize;

use crate::{
    auth::AuthUser,
    error::ApiError,
    models::{
        Permissions, Usergroup,
        usergroup::{EditError, Editor, UsergroupDetails, UsergroupEffect},
    },
    notify,
    routes::validate::{bad, hex_color, optional_url, required_text},
    state::AppState,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list)
        .service(list_full)
        .service(get_full)
        .service(create)
        .service(update)
        .service(remove)
        .service(list_members)
        .service(add_member)
        .service(remove_member);
}

/// Like the other admin pages, managing usergroups needs Admin Access as well as the
/// specific permission.
const MANAGE: Permissions = Permissions::ADMIN_ACCESS.union(Permissions::USERGROUP_CONTROL);

/// Every usergroup's public details (no members or permissions), e.g. for filtering users
/// by group. No login required.
#[get("/usergroups")]
async fn list(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(Usergroup::badges(&state.db).await?))
}

/// Every usergroup in full, including members and permissions. Requires Admin Access and
/// Usergroup Management.
#[get("/admin/usergroups")]
async fn list_full(state: web::Data<AppState>, user: AuthUser) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    Ok(HttpResponse::Ok().json(Usergroup::list(&state.db).await?))
}

/// One usergroup in full. Requires Admin Access and Usergroup Management.
#[get("/admin/usergroups/{id}")]
async fn get_full(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let group = Usergroup::find_by_id(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("usergroup"))?;
    Ok(HttpResponse::Ok().json(group))
}

/// Creates a usergroup. Requires Admin Access and Usergroup Management; the group may only
/// have permissions the creator holds.
#[post("/admin/usergroups")]
async fn create(
    state: web::Data<AppState>,
    user: AuthUser,
    body: web::Json<UsergroupBody>,
) -> Result<HttpResponse, ApiError> {
    let editor = editor(&state, &user).await?;
    let details = normalize(body.into_inner())?;
    let group = Usergroup::create(&state.db, &details, editor)
        .await
        .map_err(edit_error)?;
    log::info!(
        "user {} created usergroup {} ({:?}) with permissions {}",
        user.0.id,
        group.id,
        group.name,
        group.permissions
    );
    Ok(HttpResponse::Created().json(group))
}

/// Replaces a usergroup's details. Requires Admin Access and Usergroup Management; only
/// permissions the editor holds can be added or removed, and the default groups can't be
/// renamed.
#[put("/admin/usergroups/{id}")]
async fn update(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<UsergroupBody>,
) -> Result<HttpResponse, ApiError> {
    let editor = editor(&state, &user).await?;
    let details = normalize(body.into_inner())?;
    let group = Usergroup::update(&state.db, id.into_inner(), &details, editor)
        .await
        .map_err(edit_error)?;
    log::info!(
        "user {} updated usergroup {} ({:?}), permissions now {}",
        user.0.id,
        group.id,
        group.name,
        group.permissions
    );
    Ok(HttpResponse::Ok().json(group))
}

/// Deletes a usergroup; its members leave it. Requires Admin Access and Usergroup
/// Management, and every permission the group grants. The default groups can't be deleted.
#[delete("/admin/usergroups/{id}")]
async fn remove(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let editor = editor(&state, &user).await?;
    let id = id.into_inner();
    let name = Usergroup::delete(&state.db, id, editor)
        .await
        .map_err(edit_error)?;
    log::info!("user {} deleted usergroup {id} ({name:?})", user.0.id);
    Ok(HttpResponse::NoContent().finish())
}

/// A usergroup's members with their names and avatars. Requires Admin Access and Usergroup
/// Management.
#[get("/admin/usergroups/{id}/members")]
async fn list_members(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    user.require(&state.db, MANAGE).await?;
    let members = Usergroup::members(&state.db, id.into_inner())
        .await?
        .ok_or(ApiError::NotFound("usergroup"))?;
    Ok(HttpResponse::Ok().json(members))
}

#[derive(Deserialize)]
struct AddMemberBody {
    user_id: i64,
}

/// Adds a user to a usergroup; returns the new member (201). Requires Admin Access and
/// Usergroup Management, and every permission the group grants.
#[post("/admin/usergroups/{id}/members")]
async fn add_member(
    state: web::Data<AppState>,
    user: AuthUser,
    id: web::Path<i64>,
    body: web::Json<AddMemberBody>,
) -> Result<HttpResponse, ApiError> {
    let editor = editor(&state, &user).await?;
    let group_id = id.into_inner();
    let user_id = body.user_id;
    let member = Usergroup::add_member_as(&state.db, group_id, user_id, editor)
        .await
        .map_err(edit_error)?;
    log::info!(
        "user {} added user {user_id} to usergroup {group_id}; their effective permissions are now {}",
        user.0.id,
        Usergroup::permissions_for_user(&state.db, user_id).await?
    );
    notify::usergroup_membership(&state.db, group_id, user_id, true, Some(user.0.id)).await;
    Ok(HttpResponse::Created().json(member))
}

/// Removes a user from a usergroup (204). Requires Admin Access and Usergroup Management,
/// and every permission the group grants. Refused if it would lock the editor out.
#[delete("/admin/usergroups/{id}/members/{user_id}")]
async fn remove_member(
    state: web::Data<AppState>,
    user: AuthUser,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, ApiError> {
    let editor = editor(&state, &user).await?;
    let (group_id, user_id) = path.into_inner();
    Usergroup::remove_member_as(&state.db, group_id, user_id, editor)
        .await
        .map_err(edit_error)?;
    let permissions = Usergroup::permissions_for_user(&state.db, user_id).await?;
    log::info!(
        "user {} removed user {user_id} from usergroup {group_id}; their effective permissions are now {permissions}",
        user.0.id,
    );
    if !permissions.contains(Permissions::SITE_ACCESS) {
        log::warn!("user {user_id} no longer has Site Access and can only use support pages");
    }
    notify::usergroup_membership(&state.db, group_id, user_id, false, Some(user.0.id)).await;
    Ok(HttpResponse::NoContent().finish())
}

/// Checks the user may manage usergroups and describes them for the model's checks.
async fn editor(state: &AppState, user: &AuthUser) -> Result<Editor, ApiError> {
    let permissions = user.require(&state.db, MANAGE).await?;
    Ok(Editor {
        user_id: user.0.id,
        permissions,
        must_keep: MANAGE,
    })
}

fn edit_error(e: EditError) -> ApiError {
    match e {
        EditError::NotFound => ApiError::NotFound("usergroup"),
        EditError::NameTaken => ApiError::Conflict("a usergroup with this name"),
        EditError::UserNotFound => ApiError::NotFound("user"),
        EditError::AlreadyMember => ApiError::Conflict("this membership"),
        EditError::NotMember => ApiError::NotFound("usergroup member"),
        EditError::DefaultGroup => {
            bad("the default user and admin groups can't be renamed or deleted".to_owned())
        }
        EditError::NotHeld(missing) => ApiError::Forbidden(missing.names().collect::<Vec<_>>().join(", ")),
        EditError::LocksOut => bad(
            "this would remove your own Admin Access or Usergroup Management, locking you out of usergroups"
                .to_owned(),
        ),
        EditError::Database(e) => e.into(),
    }
}

#[derive(Deserialize)]
struct UsergroupBody {
    name: String,
    effect: UsergroupEffect,
    color: String,
    permissions: i64,
    icon_url: Option<String>,
}

const MAX_NAME: usize = 32;

/// Validates and normalizes (trims, turns a blank icon URL into `None`) a request body.
fn normalize(body: UsergroupBody) -> Result<UsergroupDetails, ApiError> {
    let name = required_text("name", &body.name, MAX_NAME)?;
    // The shell takes a group id or name, so an all-digit name would be ambiguous.
    if name.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("name must not be only digits".to_owned()));
    }
    let permissions = Permissions::from_bits(body.permissions);
    if body.permissions < 0 || !permissions.difference(Permissions::ALL).is_empty() {
        return Err(bad(format!("permissions has unknown bits: {:#x}", body.permissions)));
    }
    Ok(UsergroupDetails {
        name,
        effect: body.effect,
        color: hex_color("color", &body.color)?,
        permissions,
        icon_url: optional_url("icon_url", body.icon_url.as_deref())?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body() -> UsergroupBody {
        UsergroupBody {
            name: " Moderator ".into(),
            effect: UsergroupEffect::Halo,
            color: "#3B82F6".into(),
            permissions: 0x207,
            icon_url: Some(" ".into()),
        }
    }

    #[test]
    fn normalizes() {
        let d = normalize(body()).unwrap_or_else(|_| panic!("valid body rejected"));
        assert_eq!(d.name, "Moderator");
        assert_eq!(d.color, "#3b82f6");
        assert_eq!(d.icon_url, None);
        assert_eq!(d.permissions, Permissions::from_bits(0x207));
    }

    #[test]
    fn rejects_bad_input() {
        let mut b = body();
        b.name = "  ".into();
        assert!(normalize(b).is_err());

        let mut b = body();
        b.name = "123".into();
        assert!(normalize(b).is_err());

        let mut b = body();
        b.name = "x".repeat(MAX_NAME + 1);
        assert!(normalize(b).is_err());

        let mut b = body();
        b.permissions = 0x8000;
        assert!(normalize(b).is_err());

        let mut b = body();
        b.permissions = -1;
        assert!(normalize(b).is_err());

        let mut b = body();
        b.color = "red".into();
        assert!(normalize(b).is_err());
    }
}
