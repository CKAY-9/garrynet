/*
 * Keeps the navigation's unread badge in step without waiting for its next poll. Whatever
 * reads or sends messages announces it, and the badge refetches the unread count.
 */

const EVENT = "garrynet:messages";

export function announceMessagesChange() {
  window.dispatchEvent(new Event(EVENT));
}

/** Calls `listener` on every announced change. Returns the unsubscribe function. */
export function subscribeToMessagesChanges(listener: () => void): () => void {
  window.addEventListener(EVENT, listener);
  return () => window.removeEventListener(EVENT, listener);
}
