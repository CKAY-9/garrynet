use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};

use super::Permissions;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "usergroup_effect", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UsergroupEffect {
    Halo,
    Rainbow,
    Flash,
    None,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Usergroup {
    pub id: i64,
    pub name: String,
    pub effect: UsergroupEffect,
    /// Hex color, e.g. `#e5484d`.
    pub color: String,
    /// Ids of the users in this group.
    pub members: Vec<i64>,
    pub permissions: Permissions,
    pub icon_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_updated: Option<DateTime<Utc>>,
}

/// What's needed to display a usergroup, e.g. as a chip (no member list or permissions).
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct UsergroupBadge {
    pub id: i64,
    pub name: String,
    pub effect: UsergroupEffect,
    pub color: String,
    pub icon_url: Option<String>,
}

impl From<Usergroup> for UsergroupBadge {
    fn from(g: Usergroup) -> Self {
        Self {
            id: g.id,
            name: g.name,
            effect: g.effect,
            color: g.color,
            icon_url: g.icon_url,
        }
    }
}

/// A usergroup badge tagged with one of its members, from [`Usergroup::badges_for_users`].
#[derive(sqlx::FromRow)]
pub struct MemberBadge {
    pub user_id: i64,
    #[sqlx(flatten)]
    pub badge: UsergroupBadge,
}

/// A member of a usergroup, as listed on its admin page.
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Member {
    pub id: i64,
    pub username: String,
    pub avatar_url: String,
    /// When they joined the group.
    pub added_at: DateTime<Utc>,
}

/// A usergroup's editable fields, validated by the route.
#[derive(Clone, Debug)]
pub struct UsergroupDetails {
    pub name: String,
    pub effect: UsergroupEffect,
    pub color: String,
    pub permissions: Permissions,
    pub icon_url: Option<String>,
}

/// Who is changing a usergroup, for the checks in [`Usergroup::create`],
/// [`Usergroup::update`], [`Usergroup::delete`] and the membership changes.
#[derive(Clone, Copy, Debug)]
pub struct Editor {
    pub user_id: i64,
    /// The editor's effective permissions before the change. They can only grant or revoke
    /// permissions they hold.
    pub permissions: Permissions,
    /// Permissions the editor must still have after the change, so they can't lock
    /// themselves out of usergroup management.
    pub must_keep: Permissions,
}

/// Why a usergroup change was refused. Nothing is changed when one is returned.
#[derive(Debug)]
pub enum EditError {
    NotFound,
    /// Another group already has this name (case-insensitively).
    NameTaken,
    /// The user to add doesn't exist.
    UserNotFound,
    /// The user to add is already in the group.
    AlreadyMember,
    /// The user to remove isn't in the group.
    NotMember,
    /// The default groups can't be deleted or renamed.
    DefaultGroup,
    /// The change grants or revokes permissions the editor doesn't hold.
    NotHeld(Permissions),
    /// The change would take [`Editor::must_keep`] permissions away from the editor.
    LocksOut,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for EditError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::Database(db) if db.is_unique_violation() => Self::NameTaken,
            _ => Self::Database(e),
        }
    }
}

/// Name of the group every new user joins on their first login.
pub const DEFAULT_GROUP: &str = "user";

struct DefaultGroup {
    name: &'static str,
    effect: UsergroupEffect,
    color: &'static str,
    permissions: Permissions,
}

/// True for the default `user` and `admin` groups (matched by name, case-insensitively).
pub fn is_default_group(name: &str) -> bool {
    DEFAULT_GROUPS.iter().any(|g| g.name.eq_ignore_ascii_case(name))
}

const DEFAULT_GROUPS: [DefaultGroup; 2] = [
    DefaultGroup {
        name: DEFAULT_GROUP,
        effect: UsergroupEffect::None,
        color: "#9ba1a6",
        permissions: Permissions::BASIC,
    },
    DefaultGroup {
        name: "admin",
        effect: UsergroupEffect::Rainbow,
        color: "#e5484d",
        permissions: Permissions::ALL,
    },
];

/// Selects a group's columns plus its member ids, from `usergroups g`.
macro_rules! select_usergroup {
    ($rest:literal) => {
        concat!(
            "SELECT g.id, g.name, g.effect, g.color, g.permissions, g.icon_url,
                    g.created_at, g.last_updated,
                    ARRAY(SELECT m.user_id FROM usergroup_members m
                          WHERE m.usergroup_id = g.id ORDER BY m.user_id) AS members
             FROM usergroups g ",
            $rest
        )
    };
}

impl Usergroup {
    /// Creates the default `user` and `admin` groups if they don't exist yet (matched by
    /// name, case-insensitively). Existing groups are left untouched, so edits to them
    /// survive restarts. When the `user` group is newly created, every existing user is
    /// added to it.
    pub async fn ensure_defaults(db: &PgPool) -> sqlx::Result<()> {
        let mut tx = db.begin().await?;
        for group in &DEFAULT_GROUPS {
            let created: Option<i64> = sqlx::query_scalar(
                "INSERT INTO usergroups (name, effect, color, permissions)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (lower(name)) DO NOTHING
                 RETURNING id",
            )
            .bind(group.name)
            .bind(group.effect)
            .bind(group.color)
            .bind(group.permissions)
            .fetch_optional(&mut *tx)
            .await?;

            let Some(id) = created else { continue };
            log::info!(
                "created default usergroup `{}` (id {id}) with permissions {}",
                group.name,
                group.permissions
            );

            if group.name == DEFAULT_GROUP {
                let added = sqlx::query(
                    "INSERT INTO usergroup_members (usergroup_id, user_id)
                     SELECT $1, id FROM users
                     ON CONFLICT DO NOTHING",
                )
                .bind(id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
                if added > 0 {
                    log::info!("added {added} existing user(s) to usergroup `{}`", group.name);
                }
            }
        }
        tx.commit().await
    }

    pub async fn list(db: &PgPool) -> sqlx::Result<Vec<Usergroup>> {
        sqlx::query_as(select_usergroup!("ORDER BY g.id"))
            .fetch_all(db)
            .await
    }

    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<Usergroup>> {
        sqlx::query_as(select_usergroup!("WHERE g.id = $1"))
            .bind(id)
            .fetch_optional(db)
            .await
    }

    /// Case-insensitive.
    pub async fn find_by_name(db: &PgPool, name: &str) -> sqlx::Result<Option<Usergroup>> {
        sqlx::query_as(select_usergroup!("WHERE lower(g.name) = lower($1)"))
            .bind(name)
            .fetch_optional(db)
            .await
    }

    /// The groups `user_id` belongs to.
    pub async fn for_user(db: &PgPool, user_id: i64) -> sqlx::Result<Vec<Usergroup>> {
        sqlx::query_as(select_usergroup!(
            "JOIN usergroup_members um ON um.usergroup_id = g.id
             WHERE um.user_id = $1
             ORDER BY g.id"
        ))
        .bind(user_id)
        .fetch_all(db)
        .await
    }

    /// Badges for every group in the database, in id order.
    pub async fn badges(db: &PgPool) -> sqlx::Result<Vec<UsergroupBadge>> {
        sqlx::query_as("SELECT id, name, effect, color, icon_url FROM usergroups ORDER BY id")
            .fetch_all(db)
            .await
    }

    /// The groups of each user in `user_ids`, in one query. Ordered by user, then group id.
    pub async fn badges_for_users(db: &PgPool, user_ids: &[i64]) -> sqlx::Result<Vec<MemberBadge>> {
        sqlx::query_as(
            "SELECT um.user_id, g.id, g.name, g.effect, g.color, g.icon_url
             FROM usergroup_members um
             JOIN usergroups g ON g.id = um.usergroup_id
             WHERE um.user_id = ANY($1)
             ORDER BY um.user_id, g.id",
        )
        .bind(user_ids)
        .fetch_all(db)
        .await
    }

    /// The user's effective permissions: the OR of all their groups' permissions.
    pub async fn permissions_for_user(db: &PgPool, user_id: i64) -> sqlx::Result<Permissions> {
        let bits: i64 = sqlx::query_scalar(
            "SELECT COALESCE(bit_or(g.permissions), 0)
             FROM usergroups g
             JOIN usergroup_members um ON um.usergroup_id = g.id
             WHERE um.user_id = $1",
        )
        .bind(user_id)
        .fetch_one(db)
        .await?;
        Ok(Permissions::from_bits(bits))
    }

    /// Adds the user to the group. Returns `false` if they were already a member. No
    /// permission checks: for the backend shell. See [`Usergroup::add_member_as`].
    pub async fn add_member(db: &PgPool, group_id: i64, user_id: i64) -> sqlx::Result<bool> {
        let mut tx = db.begin().await?;
        let added = insert_member(&mut tx, group_id, user_id).await?;
        if added {
            touch(&mut tx, group_id).await?;
        }
        tx.commit().await?;
        Ok(added)
    }

    /// Removes the user from the group. Returns `false` if they weren't a member. No
    /// permission checks: for the backend shell. See [`Usergroup::remove_member_as`].
    pub async fn remove_member(db: &PgPool, group_id: i64, user_id: i64) -> sqlx::Result<bool> {
        let mut tx = db.begin().await?;
        let removed = delete_member(&mut tx, group_id, user_id).await?;
        if removed {
            touch(&mut tx, group_id).await?;
        }
        tx.commit().await?;
        Ok(removed)
    }

    /// The group's members, by name. `None` if there is no such group.
    pub async fn members(db: &PgPool, group_id: i64) -> sqlx::Result<Option<Vec<Member>>> {
        let mut tx = db.begin().await?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM usergroups WHERE id = $1)")
            .bind(group_id)
            .fetch_one(&mut *tx)
            .await?;
        if !exists {
            return Ok(None);
        }
        let members = sqlx::query_as(
            "SELECT u.id, u.username, u.avatar_url, um.added_at
             FROM usergroup_members um
             JOIN users u ON u.id = um.user_id
             WHERE um.usergroup_id = $1
             ORDER BY lower(u.username), u.id",
        )
        .bind(group_id)
        .fetch_all(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(Some(members))
    }

    /// Adds the user to the group on the editor's behalf. The editor must hold every
    /// permission the group grants, so membership can't be used to escalate.
    pub async fn add_member_as(
        db: &PgPool,
        group_id: i64,
        user_id: i64,
        editor: Editor,
    ) -> Result<Member, EditError> {
        let mut tx = db.begin().await?;
        let (_, permissions) = lock(&mut tx, group_id).await?;
        check_held(editor, permissions)?;
        let user_exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE id = $1)")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;
        if !user_exists {
            return Err(EditError::UserNotFound);
        }
        if !insert_member(&mut tx, group_id, user_id).await? {
            return Err(EditError::AlreadyMember);
        }
        touch(&mut tx, group_id).await?;
        let member = sqlx::query_as(
            "SELECT u.id, u.username, u.avatar_url, um.added_at
             FROM usergroup_members um
             JOIN users u ON u.id = um.user_id
             WHERE um.usergroup_id = $1 AND um.user_id = $2",
        )
        .bind(group_id)
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(member)
    }

    /// Removes the user from the group on the editor's behalf. The editor must hold every
    /// permission the group grants, and must keep [`Editor::must_keep`] if they're
    /// removing themselves.
    pub async fn remove_member_as(
        db: &PgPool,
        group_id: i64,
        user_id: i64,
        editor: Editor,
    ) -> Result<(), EditError> {
        let mut tx = db.begin().await?;
        let (_, permissions) = lock(&mut tx, group_id).await?;
        check_held(editor, permissions)?;
        if !delete_member(&mut tx, group_id, user_id).await? {
            return Err(EditError::NotMember);
        }
        check_kept(&mut tx, editor).await?;
        touch(&mut tx, group_id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Creates a group. Fails if the name is taken or `details.permissions` includes
    /// permissions the editor doesn't hold.
    pub async fn create(db: &PgPool, details: &UsergroupDetails, editor: Editor) -> Result<Usergroup, EditError> {
        check_held(editor, details.permissions)?;
        let mut tx = db.begin().await?;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO usergroups (name, effect, color, permissions, icon_url)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id",
        )
        .bind(&details.name)
        .bind(details.effect)
        .bind(&details.color)
        .bind(details.permissions)
        .bind(&details.icon_url)
        .fetch_one(&mut *tx)
        .await?;
        let group = fetch(&mut tx, id).await?;
        tx.commit().await?;
        Ok(group)
    }

    /// Replaces a group's details. Default groups keep their name (case may change).
    /// Permissions added or removed must be ones the editor holds, and the editor must keep
    /// [`Editor::must_keep`] afterwards.
    pub async fn update(
        db: &PgPool,
        id: i64,
        details: &UsergroupDetails,
        editor: Editor,
    ) -> Result<Usergroup, EditError> {
        let mut tx = db.begin().await?;
        let (name, permissions) = lock(&mut tx, id).await?;
        if is_default_group(&name) && !name.eq_ignore_ascii_case(&details.name) {
            return Err(EditError::DefaultGroup);
        }
        check_held(editor, permissions.symmetric_difference(details.permissions))?;
        sqlx::query(
            "UPDATE usergroups
             SET name = $2, effect = $3, color = $4, permissions = $5, icon_url = $6,
                 last_updated = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(&details.name)
        .bind(details.effect)
        .bind(&details.color)
        .bind(details.permissions)
        .bind(&details.icon_url)
        .execute(&mut *tx)
        .await?;
        check_kept(&mut tx, editor).await?;
        let group = fetch(&mut tx, id).await?;
        tx.commit().await?;
        Ok(group)
    }

    /// Deletes a group, removing its members from it. Returns the deleted group's name.
    /// Default groups can't be deleted, nor can a group granting permissions the editor
    /// doesn't hold (deleting it revokes them from its members, like removing them in
    /// [`Usergroup::update`]), nor one whose removal would take [`Editor::must_keep`] away
    /// from the editor.
    pub async fn delete(db: &PgPool, id: i64, editor: Editor) -> Result<String, EditError> {
        let mut tx = db.begin().await?;
        let (name, permissions) = lock(&mut tx, id).await?;
        if is_default_group(&name) {
            return Err(EditError::DefaultGroup);
        }
        check_held(editor, permissions)?;
        sqlx::query("DELETE FROM usergroups WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        check_kept(&mut tx, editor).await?;
        tx.commit().await?;
        Ok(name)
    }

    /// Adds a brand-new user to the default group, if it exists. Runs inside the
    /// transaction that creates the user.
    pub async fn join_default(conn: &mut PgConnection, user_id: i64) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO usergroup_members (usergroup_id, user_id)
             SELECT id, $2 FROM usergroups WHERE lower(name) = $1
             ON CONFLICT DO NOTHING",
        )
        .bind(DEFAULT_GROUP)
        .bind(user_id)
        .execute(conn)
        .await?;
        Ok(())
    }
}

/// Locks the group's row for the rest of the transaction; returns its name and permissions.
async fn lock(conn: &mut PgConnection, id: i64) -> Result<(String, Permissions), EditError> {
    sqlx::query_as("SELECT name, permissions FROM usergroups WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(conn)
        .await?
        .ok_or(EditError::NotFound)
}

async fn fetch(conn: &mut PgConnection, id: i64) -> sqlx::Result<Usergroup> {
    sqlx::query_as(select_usergroup!("WHERE g.id = $1"))
        .bind(id)
        .fetch_one(conn)
        .await
}

/// `changed` must only contain permissions the editor holds.
fn check_held(editor: Editor, changed: Permissions) -> Result<(), EditError> {
    let not_held = changed.difference(editor.permissions);
    if not_held.is_empty() { Ok(()) } else { Err(EditError::NotHeld(not_held)) }
}

/// The editor must still hold [`Editor::must_keep`] after the transaction's changes.
async fn check_kept(conn: &mut PgConnection, editor: Editor) -> Result<(), EditError> {
    let bits: i64 = sqlx::query_scalar(
        "SELECT COALESCE(bit_or(g.permissions), 0)
         FROM usergroups g
         JOIN usergroup_members um ON um.usergroup_id = g.id
         WHERE um.user_id = $1",
    )
    .bind(editor.user_id)
    .fetch_one(conn)
    .await?;
    if Permissions::from_bits(bits).contains(editor.must_keep) {
        Ok(())
    } else {
        Err(EditError::LocksOut)
    }
}

/// Returns `false` if the user was already a member.
async fn insert_member(conn: &mut PgConnection, group_id: i64, user_id: i64) -> sqlx::Result<bool> {
    Ok(sqlx::query(
        "INSERT INTO usergroup_members (usergroup_id, user_id)
         VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
    )
    .bind(group_id)
    .bind(user_id)
    .execute(conn)
    .await?
    .rows_affected()
        > 0)
}

/// Returns `false` if the user wasn't a member.
async fn delete_member(conn: &mut PgConnection, group_id: i64, user_id: i64) -> sqlx::Result<bool> {
    Ok(sqlx::query("DELETE FROM usergroup_members WHERE usergroup_id = $1 AND user_id = $2")
        .bind(group_id)
        .bind(user_id)
        .execute(conn)
        .await?
        .rows_affected()
        > 0)
}

/// Membership is part of the group, so changing it bumps `last_updated`.
async fn touch(conn: &mut PgConnection, group_id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE usergroups SET last_updated = now() WHERE id = $1")
        .bind(group_id)
        .execute(conn)
        .await?;
    Ok(())
}
