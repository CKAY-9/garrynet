use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;
use sqlx::PgPool;

use super::Usergroup;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, sqlx::Type)]
#[sqlx(type_name = "user_status", rename_all = "UPPERCASE")]
#[serde(rename_all = "UPPERCASE")]
pub enum UserStatus {
    Online,
    Idle,
    Offline,
}

/// How long after their last access a user counts as idle, then offline.
const IDLE_AFTER: TimeDelta = TimeDelta::minutes(5);
const OFFLINE_AFTER: TimeDelta = TimeDelta::minutes(15);

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: Option<String>,
    pub avatar_url: String,
    pub steam_id: String,
    pub bio: String,
    pub reputation: i32,
    pub status: UserStatus,
    pub joined: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

impl User {
    /// The user's status as of now. The stored status is set to ONLINE on access and
    /// never decays on its own, so idle/offline are derived from `last_seen`.
    pub fn current_status(&self) -> UserStatus {
        let elapsed = Utc::now() - self.last_seen;
        match self.status {
            UserStatus::Offline => UserStatus::Offline,
            _ if elapsed >= OFFLINE_AFTER => UserStatus::Offline,
            _ if elapsed >= IDLE_AFTER => UserStatus::Idle,
            status => status,
        }
    }

    /// Creates the user for `steam_id` if none exists, otherwise refreshes their Steam
    /// username and avatar. Either way the user is marked online.
    ///
    /// New users join the default usergroup. Returning users keep whatever groups they
    /// have, so removing someone from `user` isn't undone by their next login.
    pub async fn upsert_from_steam(
        db: &PgPool,
        steam_id: &str,
        username: &str,
        avatar_url: &str,
    ) -> sqlx::Result<User> {
        let mut tx = db.begin().await?;
        let created: Option<User> = sqlx::query_as(
            "INSERT INTO users (steam_id, username, avatar_url, status, last_seen)
             VALUES ($1, $2, $3, 'ONLINE', now())
             ON CONFLICT (steam_id) DO NOTHING
             RETURNING *",
        )
        .bind(steam_id)
        .bind(username)
        .bind(avatar_url)
        .fetch_optional(&mut *tx)
        .await?;

        let user = match created {
            Some(user) => {
                Usergroup::join_default(&mut tx, user.id).await?;
                log::info!("registered new user {} ({})", user.id, user.steam_id);
                user
            }
            None => {
                sqlx::query_as(
                    "UPDATE users
                        SET username = $2, avatar_url = $3, status = 'ONLINE', last_seen = now()
                      WHERE steam_id = $1
                      RETURNING *",
                )
                .bind(steam_id)
                .bind(username)
                .bind(avatar_url)
                .fetch_one(&mut *tx)
                .await?
            }
        };
        tx.commit().await?;
        Ok(user)
    }

    pub async fn find_by_id(db: &PgPool, id: i64) -> sqlx::Result<Option<User>> {
        sqlx::query_as("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(db)
            .await
    }

    pub async fn find_by_steam_id(db: &PgPool, steam_id: &str) -> sqlx::Result<Option<User>> {
        sqlx::query_as("SELECT * FROM users WHERE steam_id = $1")
            .bind(steam_id)
            .fetch_optional(db)
            .await
    }

    /// Members of a usergroup, by id.
    pub async fn in_usergroup(db: &PgPool, group_id: i64) -> sqlx::Result<Vec<User>> {
        sqlx::query_as(
            "SELECT u.* FROM users u
             JOIN usergroup_members um ON um.user_id = u.id
             WHERE um.usergroup_id = $1
             ORDER BY u.id",
        )
        .bind(group_id)
        .fetch_all(db)
        .await
    }

    /// Records that the user accessed a route: marks them online and bumps `last_seen`.
    /// Returns `None` if the user no longer exists.
    pub async fn touch(db: &PgPool, id: i64) -> sqlx::Result<Option<User>> {
        sqlx::query_as(
            "UPDATE users SET status = 'ONLINE', last_seen = now()
             WHERE id = $1
             RETURNING *",
        )
        .bind(id)
        .fetch_optional(db)
        .await
    }
}
