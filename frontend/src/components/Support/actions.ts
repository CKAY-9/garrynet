"use server";

import { refresh } from "next/cache";
import { redirect } from "next/navigation";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type {
  FaqDetails,
  FaqWithPeople,
  SupportMessage,
  SupportStatus,
  SupportTicket,
  TicketDetails,
} from "@/lib/types";

/** Opens a ticket, then shows it. The backend validates it and enforces the 5-ticket limit. */
export async function createTicket(details: TicketDetails): Promise<ActionResult<never>> {
  const result = await runAction("open the ticket", async () => {
    const { data } = await backend.post<SupportTicket>("/support/tickets", details, { headers: await authHeaders() });
    return data;
  });
  if (!result.ok) return result;
  redirect(`/support/ticket/${result.data.id}`);
}

/**
 * Adds a message (or, for the Support Team, an internal note) with the uploaded files
 * `attachments`, and refreshes the page.
 */
export async function replyToTicket(
  ticketId: number,
  content: string,
  isInternal = false,
  attachments: string[] = [],
): Promise<ActionResult<SupportMessage>> {
  return runAction(
    "send the message",
    async () => {
      const { data } = await backend.post<SupportMessage>(
        `/support/tickets/${ticketId}/messages`,
        { content, is_internal: isInternal, attachments },
        { headers: await authHeaders() },
      );
      refresh();
      return data;
    },
    { 423: "This ticket is closed. Only the support team can continue it." },
  );
}

/** Edits a message (allowed once) and refreshes the page. */
export async function editMessage(messageId: number, content: string): Promise<ActionResult<SupportMessage>> {
  return runAction("edit the message", async () => {
    const { data } = await backend.patch<SupportMessage>(`/support/messages/${messageId}`, { content }, {
      headers: await authHeaders(),
    });
    refresh();
    return data;
  });
}

/** Resolves, closes or re-opens a ticket (Support Team) and refreshes the page. */
export async function setTicketStatus(ticketId: number, status: SupportStatus): Promise<ActionResult<SupportTicket>> {
  return runAction("change the ticket's status", async () => {
    const { data } = await backend.patch<SupportTicket>(`/support/tickets/${ticketId}/status`, { status }, {
      headers: await authHeaders(),
    });
    refresh();
    return data;
  });
}

/** Creates an FAQ (Support Team), then shows it. */
export async function createFaq(details: FaqDetails): Promise<ActionResult<never>> {
  const result = await runAction("create the FAQ", async () => {
    const { data } = await backend.post<FaqWithPeople>("/support/faqs", details, { headers: await authHeaders() });
    return data;
  });
  if (!result.ok) return result;
  redirect(`/support/faq/${result.data.id}`);
}

/** Replaces an FAQ's details (Support Team) and refreshes the page. */
export async function updateFaq(id: number, details: FaqDetails): Promise<ActionResult<FaqWithPeople>> {
  return runAction("save the FAQ", async () => {
    const { data } = await backend.put<FaqWithPeople>(`/support/faqs/${id}`, details, { headers: await authHeaders() });
    refresh();
    return data;
  });
}

/** Deletes an FAQ (Support Team), then opens the FAQ list. */
export async function deleteFaq(id: number): Promise<ActionResult<never>> {
  const result = await runAction("delete the FAQ", async () => {
    await backend.delete(`/support/faqs/${id}`, { headers: await authHeaders() });
  });
  if (!result.ok) return result;
  redirect("/support/faq");
}
