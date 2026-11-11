use chrono::{DateTime, TimeDelta, Utc};
use serde::Serialize;
use sqlx::{PgPool, Postgres, QueryBuilder};

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
/// How stale `last_seen` may get before an access writes it again; well under [`IDLE_AFTER`].
const TOUCH_EVERY: TimeDelta = TimeDelta::minutes(1);

/// How recently a searched-for user must have been seen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LastActivity {
    /// Currently ONLINE (seen within the idle threshold).
    Online,
    Hour,
    Day,
    Week,
    Month,
}

impl LastActivity {
    fn window(self) -> TimeDelta {
        match self {
            Self::Online => IDLE_AFTER,
            Self::Hour => TimeDelta::hours(1),
            Self::Day => TimeDelta::days(1),
            Self::Week => TimeDelta::weeks(1),
            Self::Month => TimeDelta::days(30),
        }
    }
}

/// Order of user search results.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserSort {
    /// Exact username matches, then prefix matches, then the rest alphabetically. Random
    /// when no name is given, so an empty search shows a few random users.
    BestMatch,
    ReputationDesc,
    ReputationAsc,
    /// Most forum posts first.
    Posts,
}

/// Filters for [`User::search`]. Every filter is optional; together they must all match.
#[derive(Debug)]
pub struct UserSearch {
    /// Case-insensitive substring of the username.
    pub name: Option<String>,
    pub id: Option<i64>,
    /// Id of a usergroup the user must belong to.
    pub usergroup: Option<i64>,
    pub last_activity: Option<LastActivity>,
    pub sort: UserSort,
    pub limit: i64,
}

/// A user search result: the user plus how many forum posts they've made.
#[derive(Debug, sqlx::FromRow)]
pub struct UserSearchRow {
    #[sqlx(flatten)]
    pub user: User,
    pub post_count: i64,
}

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
    /// The JWT version currently accepted for this user; see [`User::revoke_tokens`].
    #[serde(skip)]
    pub token_version: i64,
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

    /// The ids in `ids` that don't belong to any user.
    pub async fn missing_ids(db: &PgPool, ids: &[i64]) -> sqlx::Result<Vec<i64>> {
        sqlx::query_scalar(
            "SELECT id FROM unnest($1::BIGINT[]) AS t(id)
             WHERE NOT EXISTS (SELECT 1 FROM users u WHERE u.id = t.id)",
        )
        .bind(ids)
        .fetch_all(db)
        .await
    }

    /// Users matching every filter in `search`, at most `search.limit` of them.
    pub async fn search(db: &PgPool, search: &UserSearch) -> sqlx::Result<Vec<UserSearchRow>> {
        let mut q: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT u.*,
                    (SELECT count(*) FROM forum_posts p WHERE p.author_id = u.id) AS post_count
             FROM users u
             WHERE true",
        );

        // Escaped so `%` and `_` in the search are matched literally.
        let pattern = search.name.as_deref().map(|name| {
            name.replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        });
        if let Some(pattern) = &pattern {
            q.push(" AND u.username ILIKE '%' || ")
                .push_bind(pattern.clone())
                .push(" || '%'");
        }
        if let Some(id) = search.id {
            q.push(" AND u.id = ").push_bind(id);
        }
        if let Some(group) = search.usergroup {
            q.push(
                " AND EXISTS (SELECT 1 FROM usergroup_members um
                              WHERE um.user_id = u.id AND um.usergroup_id = ",
            )
            .push_bind(group)
            .push(")");
        }
        if let Some(activity) = search.last_activity {
            if activity == LastActivity::Online {
                q.push(" AND u.status = 'ONLINE'");
            }
            q.push(" AND u.last_seen > now() - make_interval(secs => ")
                .push_bind(activity.window().num_seconds() as f64)
                .push(")");
        }

        match (search.sort, &pattern) {
            (UserSort::BestMatch, Some(pattern)) => {
                q.push(" ORDER BY lower(u.username) = lower(")
                    .push_bind(search.name.clone())
                    .push(") DESC, u.username ILIKE ")
                    .push_bind(pattern.clone())
                    .push(" || '%' DESC, lower(u.username), u.id");
            }
            (UserSort::BestMatch, None) => {
                q.push(" ORDER BY random()");
            }
            (UserSort::ReputationDesc, _) => {
                q.push(" ORDER BY u.reputation DESC, u.id");
            }
            (UserSort::ReputationAsc, _) => {
                q.push(" ORDER BY u.reputation ASC, u.id");
            }
            (UserSort::Posts, _) => {
                q.push(" ORDER BY post_count DESC, u.id");
            }
        }
        q.push(" LIMIT ").push_bind(search.limit);

        q.build_query_as().fetch_all(db).await
    }

    /// Number of registered users.
    pub async fn count(db: &PgPool) -> sqlx::Result<i64> {
        sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(db)
            .await
    }

    /// Number of users whose [`current_status`](Self::current_status) is ONLINE.
    pub async fn count_online(db: &PgPool) -> sqlx::Result<i64> {
        sqlx::query_scalar(
            "SELECT count(*) FROM users
             WHERE status = 'ONLINE' AND last_seen > now() - make_interval(secs => $1)",
        )
        .bind(IDLE_AFTER.num_seconds() as f64)
        .fetch_one(db)
        .await
    }

    /// Records that the user accessed a route with a token of version `token_version`: marks
    /// them online and bumps `last_seen`. Returns `None` (changing nothing) if the user no
    /// longer exists or the token has been revoked.
    ///
    /// Statuses work in minutes (see [`IDLE_AFTER`]), so the row is only written when the
    /// user isn't marked online or was last seen over [`TOUCH_EVERY`] ago; otherwise it's
    /// just read. That spares a write on every request (the notification badge polls).
    pub async fn touch(db: &PgPool, id: i64, token_version: i64) -> sqlx::Result<Option<User>> {
        sqlx::query_as(
            "WITH touched AS (
                 UPDATE users SET status = 'ONLINE', last_seen = now()
                 WHERE id = $1 AND token_version = $2
                   AND (status <> 'ONLINE' OR last_seen < now() - make_interval(secs => $3))
                 RETURNING *
             )
             SELECT * FROM touched
             UNION ALL
             SELECT * FROM users
             WHERE id = $1 AND token_version = $2 AND NOT EXISTS (SELECT 1 FROM touched)",
        )
        .bind(id)
        .bind(token_version)
        .bind(TOUCH_EVERY.num_seconds() as f64)
        .fetch_optional(db)
        .await
    }

    /// Revokes every token issued to the user so far, signing them out everywhere, and marks
    /// them offline. Returns `false` if there is no such user.
    pub async fn revoke_tokens(db: &PgPool, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query(
            "UPDATE users SET token_version = token_version + 1, status = 'OFFLINE' WHERE id = $1",
        )
        .bind(id)
        .execute(db)
        .await?;
        Ok(result.rows_affected() > 0)
    }
}
