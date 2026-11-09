use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgConnection, PgPool};

use super::Permissions;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, sqlx::Type)]
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

/// Name of the group every new user joins on their first login.
pub const DEFAULT_GROUP: &str = "user";

struct DefaultGroup {
    name: &'static str,
    effect: UsergroupEffect,
    color: &'static str,
    permissions: Permissions,
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
        effect: UsergroupEffect::None,
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

    /// Adds the user to the group. Returns `false` if they were already a member.
    pub async fn add_member(db: &PgPool, group_id: i64, user_id: i64) -> sqlx::Result<bool> {
        let mut tx = db.begin().await?;
        let added = sqlx::query(
            "INSERT INTO usergroup_members (usergroup_id, user_id)
             VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0;
        if added {
            touch(&mut tx, group_id).await?;
        }
        tx.commit().await?;
        Ok(added)
    }

    /// Removes the user from the group. Returns `false` if they weren't a member.
    pub async fn remove_member(db: &PgPool, group_id: i64, user_id: i64) -> sqlx::Result<bool> {
        let mut tx = db.begin().await?;
        let removed = sqlx::query(
            "DELETE FROM usergroup_members WHERE usergroup_id = $1 AND user_id = $2",
        )
        .bind(group_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
            > 0;
        if removed {
            touch(&mut tx, group_id).await?;
        }
        tx.commit().await?;
        Ok(removed)
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

/// Membership is part of the group, so changing it bumps `last_updated`.
async fn touch(conn: &mut PgConnection, group_id: i64) -> sqlx::Result<()> {
    sqlx::query("UPDATE usergroups SET last_updated = now() WHERE id = $1")
        .bind(group_id)
        .execute(conn)
        .await?;
    Ok(())
}
