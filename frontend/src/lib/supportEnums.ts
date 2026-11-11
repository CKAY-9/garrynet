import type { FaqCategory, SupportStatus, TicketActivity, TicketCategory, TicketSort } from "./types";

/*
 * The support enums (docs/SUPPORT.md) in display order, with their labels. Safe to import
 * from client components.
 */

export const ticketCategories: TicketCategory[] = ["OTHER", "USER_REPORT", "SUGGESTION", "BUG_REPORT", "USER_ISSUE"];
export const supportStatuses: SupportStatus[] = ["OPEN", "AWAITING_USER", "AWAITING_ADMIN", "RESOLVED", "CLOSED"];
export const faqCategories: FaqCategory[] = ["GENERAL", "SUPPORT", "STORE", "FORUM", "USER", "COMMUNITY"];
export const ticketActivities: TicketActivity[] = ["day", "week", "month"];
export const ticketSorts: TicketSort[] = ["latest", "oldest", "created"];

export const ticketCategoryLabels: Record<TicketCategory, string> = {
  OTHER: "Other",
  USER_REPORT: "User report",
  SUGGESTION: "Suggestion",
  BUG_REPORT: "Bug report",
  USER_ISSUE: "Account or purchase issue",
};

export const statusLabels: Record<SupportStatus, string> = {
  OPEN: "Open",
  AWAITING_USER: "Awaiting user",
  AWAITING_ADMIN: "Awaiting support",
  RESOLVED: "Resolved",
  CLOSED: "Closed",
};

export const faqCategoryLabels: Record<FaqCategory, string> = {
  GENERAL: "General",
  SUPPORT: "Support",
  STORE: "Store",
  FORUM: "Forum",
  USER: "Users",
  COMMUNITY: "Community",
};

export const ticketActivityLabels: Record<TicketActivity, string> = {
  day: "Past day",
  week: "Past week",
  month: "Past month",
};

export const ticketSortLabels: Record<TicketSort, string> = {
  latest: "Latest activity",
  oldest: "Oldest activity",
  created: "Newest first",
};

/** Whether a ticket is finished: only the support team can continue it. */
export function isDone(status: SupportStatus): boolean {
  return status === "RESOLVED" || status === "CLOSED";
}

/** Limits enforced by the backend (src/routes/support.rs). */
export const MAX_TITLE = 128;
export const MAX_SUBJECT = 256;
export const MAX_CONTENT = 20_000;
export const MAX_ASSOCIATED_USERS = 10;
export const MAX_ACTIVE_TICKETS = 5;
