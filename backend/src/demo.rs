//! Demo mode (docs/DEMO.md). Every day at midnight UTC, everything users without Admin
//! Access created is cleared: forum threads and replies, likes, support tickets and
//! messages, FAQs, private messages, store purchases, uploads and notifications. Their
//! accounts (profile, usergroups, sessions) are kept.

use std::{fmt, time::Duration};

use chrono::{DateTime, NaiveTime, TimeDelta, Utc};
use sqlx::PgConnection;

use crate::{
    models::{File, Permissions, file::FileStatus, store::Purchase},
    state::AppState,
    storage,
};

/// Users whose usergroups grant Admin Access (`$1`), whatever punishments they're under.
/// Everyone else is a visitor whose content the reset clears.
macro_rules! admins {
    () => {
        "(SELECT um.user_id FROM usergroup_members um
            JOIN usergroups g ON g.id = um.usergroup_id
           WHERE g.permissions & $1 <> 0)"
    };
}

/// When the next reset is due after `now`: the following midnight UTC.
pub fn next_reset(now: DateTime<Utc>) -> DateTime<Utc> {
    (now.date_naive() + TimeDelta::days(1)).and_time(NaiveTime::MIN).and_utc()
}

/// Resets the demo every day at midnight UTC (see [`reset`]). Not on startup, so
/// restarting or redeploying doesn't clear anything.
pub fn spawn_daily_reset(state: AppState) {
    actix_web::rt::spawn(async move {
        loop {
            let now = Utc::now();
            let wait = (next_reset(now) - now).to_std().unwrap_or(Duration::ZERO);
            tokio::time::sleep(wait).await;
            match reset(&state).await {
                Ok(summary) => log::info!("demo reset: {summary}"),
                Err(e) => log::error!("demo reset failed: {e}"),
            }
            // Past midnight now; never run twice for the same day.
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

#[derive(Debug, thiserror::Error)]
pub enum ResetError {
    #[error("demo mode is off: refusing to delete user content (set DEMO_MODE=true to enable it)")]
    Disabled,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// What a reset deleted.
#[derive(Debug, Default)]
pub struct ResetSummary {
    pub threads: u64,
    pub replies: u64,
    pub tickets: u64,
    pub support_messages: u64,
    pub faqs: u64,
    pub private_messages: u64,
    pub purchases: u64,
    pub receipts: u64,
    pub notifications: u64,
    pub files: usize,
}

impl fmt::Display for ResetSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "deleted {} thread(s), {} reply(ies), {} ticket(s), {} support message(s), {} FAQ(s), \
             {} private message(s), {} purchase(s), {} receipt(s), {} notification(s) and {} file(s)",
            self.threads,
            self.replies,
            self.tickets,
            self.support_messages,
            self.faqs,
            self.private_messages,
            self.purchases,
            self.receipts,
            self.notifications,
            self.files
        )
    }
}

/// Deletes everything users without Admin Access created, in one transaction, then the
/// bytes of their files. Refuses unless demo mode is on.
pub async fn reset(state: &AppState) -> Result<ResetSummary, ResetError> {
    if !state.config.demo {
        return Err(ResetError::Disabled);
    }
    close_checkouts(state).await?;

    let mut tx = state.db.begin().await?;
    let mut summary = ResetSummary::default();
    // Collected first: deleting what files are attached to only detaches them.
    let files = delete_files(&mut tx).await?;
    summary.files = files.len();
    reset_forum(&mut tx, &mut summary).await?;
    reset_support(&mut tx, &mut summary).await?;
    summary.private_messages = exec(
        &mut tx,
        concat!(
            "DELETE FROM private_messages
              WHERE author_id NOT IN ", admins!(), " OR recipient_id NOT IN ", admins!()
        ),
    )
    .await?;
    reset_store(&mut tx, &mut summary).await?;
    // Most point at what was just deleted, and they don't record who caused them.
    summary.notifications = sqlx::query("DELETE FROM notifications").execute(&mut *tx).await?.rows_affected();
    tx.commit().await?;

    for file in files {
        let Some(backend) = storage::Backend::parse(&file.backend) else { continue };
        let result = match file.status {
            // A pending upload may have bytes waiting at its staging key.
            FileStatus::Pending => state.storage.delete_file(backend, &file.key).await,
            FileStatus::Ready => state.storage.delete(backend, &file.key).await,
        };
        if let Err(e) = result {
            log::error!("demo reset: failed to delete object {} of file {}/{}: {e}", file.key, file.owner_id, file.id);
        }
    }
    Ok(summary)
}

/// Runs `sql` with Admin Access bound as `$1`; returns the rows affected.
async fn exec(conn: &mut PgConnection, sql: &'static str) -> sqlx::Result<u64> {
    Ok(sqlx::query(sql).bind(Permissions::ADMIN_ACCESS).execute(conn).await?.rows_affected())
}

/// Removes visitors from a `BIGINT[]` column of user ids.
macro_rules! keep_admins {
    ($column:literal) => {
        concat!($column, " = ARRAY(SELECT x FROM unnest(", $column, ") x WHERE x IN ", admins!(), ")")
    };
}

/// Closes visitors' open Checkout Sessions, so none can be paid after its purchase is
/// deleted. Best-effort: the demo only takes test payments, and a session that still
/// completes is logged by the webhook as having no purchase.
async fn close_checkouts(state: &AppState) -> sqlx::Result<()> {
    let Some(stripe) = &state.stripe else { return Ok(()) };
    let pending: Vec<Purchase> = sqlx::query_as(concat!(
        "SELECT * FROM store_purchases
          WHERE status = 'PENDING' AND stripe_checkout_session_id IS NOT NULL
            AND buyer_id NOT IN ", admins!()
    ))
    .bind(Permissions::ADMIN_ACCESS)
    .fetch_all(&state.db)
    .await?;
    for purchase in pending {
        let Some(session) = &purchase.stripe_checkout_session_id else { continue };
        if let Err(e) = stripe.expire_checkout_session(session).await {
            log::warn!("demo reset: couldn't expire checkout session {session} of purchase {}: {e}", purchase.id);
        }
    }
    Ok(())
}

/// Deletes visitors' files, and any file attached to a post, support message or private
/// message the reset deletes. Community files are kept: they may be a logo or splash.
async fn delete_files(conn: &mut PgConnection) -> sqlx::Result<Vec<File>> {
    sqlx::query_as(concat!(
        "DELETE FROM files f
          WHERE f.owner_id <> -1 AND (
                f.owner_id NOT IN ", admins!(), "
             OR f.uploaded_by NOT IN ", admins!(), "
             OR f.forum_post_id IN (
                    SELECT p.id FROM forum_posts p JOIN forum_threads t ON t.id = p.thread_id
                     WHERE t.author_id NOT IN ", admins!(), " OR p.author_id NOT IN ", admins!(), ")
             OR f.support_message_id IN (
                    SELECT m.id FROM support_messages m JOIN support_tickets t ON t.id = m.ticket_id
                     WHERE t.user_id NOT IN ", admins!(), " OR m.author_id NOT IN ", admins!(), ")
             OR f.private_message_id IN (
                    SELECT id FROM private_messages
                     WHERE author_id NOT IN ", admins!(), " OR recipient_id NOT IN ", admins!(), "))
          RETURNING *"
    ))
    .bind(Permissions::ADMIN_ACCESS)
    .fetch_all(conn)
    .await
}

/// Visitors' threads (with every reply in them), their replies in admins' threads, and
/// their likes and dislikes.
async fn reset_forum(conn: &mut PgConnection, summary: &mut ResetSummary) -> sqlx::Result<()> {
    summary.threads = exec(conn, concat!("DELETE FROM forum_threads WHERE author_id NOT IN ", admins!())).await?;

    let threads: Vec<i64> = sqlx::query_scalar(concat!(
        "DELETE FROM forum_posts p USING forum_threads t
          WHERE t.id = p.thread_id AND p.id <> t.thread_post_id
            AND p.author_id NOT IN ", admins!(), "
          RETURNING p.thread_id"
    ))
    .bind(Permissions::ADMIN_ACCESS)
    .fetch_all(&mut *conn)
    .await?;
    summary.replies = threads.len() as u64;
    // As deleting a reply does: back to the newest remaining post.
    sqlx::query(
        "UPDATE forum_threads t
            SET last_activity = COALESCE(
                (SELECT max(p.created_at) FROM forum_posts p WHERE p.thread_id = t.id),
                t.created_at)
          WHERE t.id = ANY($1)",
    )
    .bind(&threads)
    .execute(&mut *conn)
    .await?;

    exec(
        conn,
        concat!(
            "UPDATE forum_posts SET ", keep_admins!("likes"), ", ", keep_admins!("dislikes"), "
              WHERE EXISTS (SELECT 1 FROM unnest(likes || dislikes) x WHERE x NOT IN ", admins!(), ")"
        ),
    )
    .await?;
    Ok(())
}

/// Visitors' tickets (with every message in them), their messages in admins' tickets,
/// their FAQs, and their place in tickets' and FAQs' user lists.
async fn reset_support(conn: &mut PgConnection, summary: &mut ResetSummary) -> sqlx::Result<()> {
    summary.tickets = exec(conn, concat!("DELETE FROM support_tickets WHERE user_id NOT IN ", admins!())).await?;
    summary.support_messages = exec(
        conn,
        concat!(
            "DELETE FROM support_messages m USING support_tickets t
              WHERE t.id = m.ticket_id AND m.id <> t.initial_message_id
                AND m.author_id NOT IN ", admins!()
        ),
    )
    .await?;
    exec(
        conn,
        concat!(
            "UPDATE support_tickets SET ", keep_admins!("associated_user_ids"), ", ", keep_admins!("responders"), "
              WHERE EXISTS (SELECT 1 FROM unnest(associated_user_ids || responders) x WHERE x NOT IN ", admins!(), ")"
        ),
    )
    .await?;
    summary.faqs = exec(conn, concat!("DELETE FROM faqs WHERE created_by NOT IN ", admins!())).await?;
    exec(
        conn,
        concat!(
            "UPDATE faqs SET ", keep_admins!("editors"), "
              WHERE EXISTS (SELECT 1 FROM unnest(editors) x WHERE x NOT IN ", admins!(), ")"
        ),
    )
    .await?;
    Ok(())
}

/// Visitors' purchases, receipts and deliveries, returning the stock they hold and taking
/// them off the items' purchase counts first.
async fn reset_store(conn: &mut PgConnection, summary: &mut ResetSummary) -> sqlx::Result<()> {
    // Pending and paid purchases hold the unit they reserved; cancelled ones gave it back.
    exec(
        conn,
        concat!(
            "UPDATE store_items i
                SET remaining_stock = LEAST(i.starting_stock, i.remaining_stock + h.held)
               FROM (SELECT item_id, count(*)::int AS held FROM store_purchases
                      WHERE stock_reserved AND status IN ('PENDING', 'PAID')
                        AND buyer_id NOT IN ", admins!(), "
                      GROUP BY item_id) h
              WHERE i.id = h.item_id AND i.remaining_stock IS NOT NULL"
        ),
    )
    .await?;
    // Refunds already came off the count.
    exec(
        conn,
        concat!(
            "UPDATE store_items i
                SET all_time_purchases = GREATEST(0, i.all_time_purchases - r.bought)
               FROM (SELECT item_id, sum(quantity)::int AS bought FROM store_receipts
                      WHERE status <> 'REFUNDED' AND item_id IS NOT NULL
                        AND buyer_id NOT IN ", admins!(), "
                      GROUP BY item_id) r
              WHERE i.id = r.item_id"
        ),
    )
    .await?;
    // Deliveries go with their receipts.
    summary.receipts = exec(conn, concat!("DELETE FROM store_receipts WHERE buyer_id NOT IN ", admins!())).await?;
    exec(conn, concat!("DELETE FROM store_fulfillments WHERE buyer_id NOT IN ", admins!())).await?;
    summary.purchases = exec(conn, concat!("DELETE FROM store_purchases WHERE buyer_id NOT IN ", admins!())).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn next_reset_is_the_following_midnight_utc() {
        let at = |d, h, m, s| Utc.with_ymd_and_hms(2026, 11, d, h, m, s).unwrap();
        assert_eq!(next_reset(at(14, 0, 0, 0)), at(15, 0, 0, 0));
        assert_eq!(next_reset(at(14, 13, 30, 0)), at(15, 0, 0, 0));
        assert_eq!(next_reset(at(14, 23, 59, 59)), at(15, 0, 0, 0));
        assert_eq!(
            next_reset(Utc.with_ymd_and_hms(2026, 12, 31, 12, 0, 0).unwrap()),
            Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap()
        );
    }
}
