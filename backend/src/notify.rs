//! Creates the notifications of docs/NOTIFICATIONS.md: one function per event, called
//! after the action that caused it has committed.
//!
//! Nothing here returns an error. A notification that can't be written is logged and
//! dropped, so the triggering action (replying to a ticket, a Stripe webhook) never fails
//! because of it. `actor` is whoever caused the event; they're never notified about it.

use sqlx::PgPool;

use crate::models::{
    Permissions, User, Usergroup,
    forum::{ForumPost, ForumThread},
    notification::{MAX_BODY, MAX_TITLE, NewNotification, Notification, NotificationType},
    store::{Fulfillment, Receipt},
    support::{AUTO_CLOSE_AFTER_DAYS, SupportStatus, SupportTicket},
};

/// Writes the notification for every recipient except `actor`.
#[allow(clippy::too_many_arguments)]
async fn send(
    db: &PgPool,
    recipients: &[i64],
    actor: Option<i64>,
    kind: NotificationType,
    event: &'static str,
    title: String,
    body: String,
    url: Option<String>,
) {
    let recipients: Vec<i64> = recipients.iter().copied().filter(|id| Some(*id) != actor).collect();
    if recipients.is_empty() {
        return;
    }
    let new = NewNotification {
        kind,
        event,
        title: clip(&title, MAX_TITLE),
        body: Some(clip(&body, MAX_BODY)),
        url,
    };
    if let Err(e) = Notification::create(db, &recipients, &new).await {
        log::error!("failed to write {event} notification for user(s) {recipients:?}: {e}");
    }
}

/// `text` cut to `max` characters, ending in an ellipsis if anything was cut.
fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut clipped: String = text.chars().take(max - 1).collect();
    clipped.push('…');
    clipped
}

fn ticket_url(ticket: &SupportTicket) -> Option<String> {
    Some(format!("/support/ticket/{}", ticket.id))
}

fn thread_url(thread: &ForumThread) -> Option<String> {
    Some(format!("/forum/threads/{}", thread.id))
}

const PURCHASES_URL: &str = "/store/purchases";

/// The Support Team replied to a ticket (not an internal note).
pub async fn ticket_reply(db: &PgPool, ticket: &SupportTicket, author: &User) {
    send(
        db,
        &[ticket.user_id],
        Some(author.id),
        NotificationType::Support,
        "ticket_reply",
        format!("Support replied: {}", ticket.title),
        format!("{} from the support team replied to your ticket.", author.username),
        ticket_url(ticket),
    )
    .await;
}

/// The ticket's owner replied: tells the support team members who have responded to it.
pub async fn ticket_owner_reply(db: &PgPool, ticket: &SupportTicket, owner: &User) {
    // A responder who has since left the Support Team can't open the ticket any more.
    let mut recipients = Vec::with_capacity(ticket.responders.len());
    for &id in &ticket.responders {
        match Usergroup::permissions_for_user(db, id).await {
            Ok(permissions) if permissions.contains(Permissions::SUPPORT_TEAM) => recipients.push(id),
            Ok(_) => {}
            Err(e) => log::error!("failed to check whether user {id} is still on the support team: {e}"),
        }
    }
    send(
        db,
        &recipients,
        Some(owner.id),
        NotificationType::Support,
        "ticket_owner_reply",
        format!("Ticket reply: {}", ticket.title),
        format!("{} replied to a ticket you responded to.", owner.username),
        ticket_url(ticket),
    )
    .await;
}

/// The Support Team changed a ticket's status from `before` to `ticket.status`. Only
/// resolving, closing and re-opening notify the owner.
pub async fn ticket_status(db: &PgPool, ticket: &SupportTicket, before: SupportStatus, by: i64) {
    let (what, body) = match ticket.status {
        SupportStatus::Resolved => ("resolved", "The support team marked your ticket as resolved."),
        SupportStatus::Closed => ("closed", "The support team closed your ticket."),
        _ if before.is_done() => ("re-opened", "The support team re-opened your ticket."),
        _ => return,
    };
    send(
        db,
        &[ticket.user_id],
        Some(by),
        NotificationType::Support,
        "ticket_status",
        format!("Ticket {what}: {}", ticket.title),
        body.to_owned(),
        ticket_url(ticket),
    )
    .await;
}

/// A ticket was closed automatically after waiting on its owner for too long.
pub async fn ticket_auto_closed(db: &PgPool, ticket: &SupportTicket) {
    send(
        db,
        &[ticket.user_id],
        None,
        NotificationType::Support,
        "ticket_status",
        format!("Ticket closed: {}", ticket.title),
        format!("Your ticket was closed automatically after {AUTO_CLOSE_AFTER_DAYS} days without a reply from you."),
        ticket_url(ticket),
    )
    .await;
}

/// A purchase was paid.
pub async fn purchase_paid(db: &PgPool, receipt: &Receipt) {
    send(
        db,
        &[receipt.buyer_id],
        None,
        NotificationType::Store,
        "purchase_paid",
        format!("Purchase complete: {}", receipt.item_name),
        format!(
            "Your payment was received. {} will be delivered in-game on {}.",
            receipt.item_name, receipt.store_name
        ),
        Some(PURCHASES_URL.to_owned()),
    )
    .await;
}

/// A purchase was refunded, revoking its fulfillment. `by` is the admin who refunded it,
/// if it wasn't done from Stripe.
pub async fn purchase_refunded(db: &PgPool, receipt: &Receipt, by: Option<i64>) {
    send(
        db,
        &[receipt.buyer_id],
        by,
        NotificationType::Store,
        "purchase_refunded",
        format!("Refunded: {}", receipt.item_name),
        format!(
            "Your purchase of {} from {} was refunded and its rewards were revoked.",
            receipt.item_name, receipt.store_name
        ),
        Some(PURCHASES_URL.to_owned()),
    )
    .await;
}

/// The server's addon delivered a purchase's rewards in-game.
pub async fn fulfillment_delivered(db: &PgPool, fulfillment: &Fulfillment) {
    let Some(receipt) = receipt_of(db, fulfillment).await else { return };
    send(
        db,
        &[fulfillment.buyer_id],
        None,
        NotificationType::Store,
        "fulfillment_delivered",
        format!("Delivered: {}", receipt.item_name),
        format!("Your purchase of {} was delivered in-game on {}.", receipt.item_name, receipt.store_name),
        Some(PURCHASES_URL.to_owned()),
    )
    .await;
}

/// The addon failed to deliver a purchase's rewards and has stopped trying.
pub async fn fulfillment_failed(db: &PgPool, fulfillment: &Fulfillment) {
    let Some(receipt) = receipt_of(db, fulfillment).await else { return };
    send(
        db,
        &[fulfillment.buyer_id],
        None,
        NotificationType::Store,
        "fulfillment_failed",
        format!("Delivery failed: {}", receipt.item_name),
        format!(
            "We couldn't deliver your purchase of {} in-game on {}. Open a support ticket and we'll sort it out.",
            receipt.item_name, receipt.store_name
        ),
        Some("/support/new".to_owned()),
    )
    .await;
}

/// The receipt a fulfillment delivers, for the item's and store's names.
async fn receipt_of(db: &PgPool, fulfillment: &Fulfillment) -> Option<Receipt> {
    match Receipt::find_by_id(db, fulfillment.receipt_id).await {
        Ok(Some(view)) => Some(view.receipt),
        Ok(None) => None,
        Err(e) => {
            log::error!("failed to load receipt {} to notify its buyer: {e}", fulfillment.receipt_id);
            None
        }
    }
}

/// Someone replied to a thread.
pub async fn thread_reply(db: &PgPool, thread: &ForumThread, author: &User) {
    send(
        db,
        &[thread.author_id],
        Some(author.id),
        NotificationType::Forum,
        "thread_reply",
        format!("New reply: {}", thread.title),
        format!("{} replied to your thread.", author.username),
        thread_url(thread),
    )
    .await;
}

/// A moderator locked a thread.
pub async fn thread_locked(db: &PgPool, thread: &ForumThread, by: i64) {
    send(
        db,
        &[thread.author_id],
        Some(by),
        NotificationType::Forum,
        "thread_locked",
        format!("Thread locked: {}", thread.title),
        "A moderator locked your thread. It can't be replied to or changed.".to_owned(),
        thread_url(thread),
    )
    .await;
}

/// A moderator deleted a thread. There is nothing left to link to.
pub async fn thread_deleted(db: &PgPool, thread: &ForumThread, by: i64) {
    send(
        db,
        &[thread.author_id],
        Some(by),
        NotificationType::Forum,
        "thread_deleted",
        format!("Thread deleted: {}", thread.title),
        "A moderator deleted your thread.".to_owned(),
        None,
    )
    .await;
}

/// A moderator deleted a reply in `thread`.
pub async fn post_deleted(db: &PgPool, post: &ForumPost, thread: &ForumThread, by: i64) {
    send(
        db,
        &[post.author_id],
        Some(by),
        NotificationType::Forum,
        "post_deleted",
        format!("Post deleted: {}", thread.title),
        "A moderator deleted your post in this thread.".to_owned(),
        thread_url(thread),
    )
    .await;
}

/// A user was added to (`added`) or removed from a usergroup. `by` is `None` for the
/// backend shell.
pub async fn usergroup_membership(db: &PgPool, group_id: i64, user_id: i64, added: bool, by: Option<i64>) {
    let name = match Usergroup::find_by_id(db, group_id).await {
        Ok(Some(group)) => group.name,
        Ok(None) => return,
        Err(e) => {
            log::error!("failed to load usergroup {group_id} to notify user {user_id}: {e}");
            return;
        }
    };
    let (event, title, body) = if added {
        (
            "usergroup_added",
            format!("Added to usergroup: {name}"),
            format!("You are now a member of the {name} usergroup."),
        )
    } else {
        (
            "usergroup_removed",
            format!("Removed from usergroup: {name}"),
            format!("You are no longer a member of the {name} usergroup."),
        )
    };
    send(
        db,
        &[user_id],
        by,
        NotificationType::Account,
        event,
        title,
        body,
        Some(format!("/users/{user_id}")),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips() {
        assert_eq!(clip("short", 10), "short");
        assert_eq!(clip("exactly ten", 11), "exactly ten");
        assert_eq!(clip("one too many", 11), "one too ma…");
        let clipped = clip(&"é".repeat(200), MAX_TITLE);
        assert_eq!(clipped.chars().count(), MAX_TITLE);
        assert!(clipped.ends_with('…'));
    }
}
