import "server-only";
import { connection } from "next/server";
import axios from "axios";
import { authHeaders, getCurrentUser } from "./auth";
import { backend } from "./backend";
import { hasPermission, Permission } from "./permissions";
import { faqCategories, supportStatuses, ticketActivities, ticketCategories, ticketSorts } from "./supportEnums";
import type { FaqCategory, FaqWithPeople, TicketFilters, TicketPage, TicketSummary } from "./types";

/*
 * Support reads (docs/SUPPORT.md). All request-time, so render anything that uses these
 * inside a `<Suspense>` boundary. Writes are server actions in `@/components/Support/actions`.
 */

/** A page's `searchParams`. */
export type SearchParams = Promise<Record<string, string | string[] | undefined>>;

function firstParam(params: Record<string, string | string[] | undefined>, key: string): string {
  const value = params[key];
  return (Array.isArray(value) ? value[0] : value)?.trim() ?? "";
}

const oneOf = <T extends string>(value: string, allowed: T[]): T | "" =>
  allowed.includes(value as T) ? (value as T) : "";

/** The ticket filters from the URL; unknown values are dropped. */
export async function readTicketFilters(searchParams: SearchParams): Promise<TicketFilters> {
  const params = await searchParams;
  return {
    status: oneOf(firstParam(params, "status"), supportStatuses),
    category: oneOf(firstParam(params, "category"), ticketCategories),
    activity: oneOf(firstParam(params, "activity"), ticketActivities),
    sort: oneOf(firstParam(params, "sort"), ticketSorts),
  };
}

/** The FAQ category from the URL, or `""` for all. */
export async function readFaqCategory(searchParams: SearchParams): Promise<FaqCategory | ""> {
  return oneOf(firstParam(await searchParams, "category"), faqCategories);
}

/**
 * What acting as the Support Team takes, as on the backend (`SUPPORT_TEAM` in
 * src/routes/support.rs): Site Access too, so a banned team member is a regular user here.
 */
export const SUPPORT_TEAM = Permission.SiteAccess | Permission.SupportTeam;

/** Who the current user is to the support system. Reads the session cookie. */
export type SupportViewer = {
  /** `null` when logged out. */
  userId: number | null;
  /** Has Support Team: sees every ticket, internal notes and the status menu; manages FAQs. */
  support: boolean;
  /** Logged in without Site Access: support is the only part of the site they can use. */
  banned: boolean;
};

export async function getSupportViewer(): Promise<SupportViewer> {
  const user = await getCurrentUser();
  if (!user) return { userId: null, support: false, banned: false };
  return {
    userId: user.id,
    support: hasPermission(user.permissions, SUPPORT_TEAM),
    banned: !hasPermission(user.permissions, Permission.SiteAccess),
  };
}

function ticketParams(filters: TicketFilters, state?: "active" | "done"): URLSearchParams {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters)) {
    const trimmed = value?.trim();
    if (trimmed) params.set(key, trimmed);
  }
  if (state) params.set("state", state);
  return params;
}

/** The current user's tickets. `state` picks the in-progress or finished ones. */
export async function getMyTickets(filters: TicketFilters, state?: "active" | "done"): Promise<TicketSummary[]> {
  await connection();
  const { data } = await backend.get<TicketSummary[]>(`/support/tickets?${ticketParams(filters, state)}`, {
    headers: await authHeaders(),
  });
  return data;
}

/** Every ticket. The backend requires Support Team, so call `requirePermission` first. */
export async function getAllTickets(filters: TicketFilters): Promise<TicketSummary[]> {
  await connection();
  const { data } = await backend.get<TicketSummary[]>(`/support/admin/tickets?${ticketParams(filters)}`, {
    headers: await authHeaders(),
  });
  return data;
}

/**
 * The ticket with id `id`, or `null` if there is none or it isn't the current user's (the
 * backend answers 404 for other people's tickets).
 */
export async function getTicket(id: string): Promise<TicketPage | null> {
  if (!/^\d+$/.test(id)) return null;
  await connection();
  try {
    const { data } = await backend.get<TicketPage>(`/support/tickets/${id}`, { headers: await authHeaders() });
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}

/** FAQs, most recently updated first, optionally in one category or capped at `limit`. */
export async function getFaqs(category?: FaqCategory | "", limit?: number): Promise<FaqWithPeople[]> {
  await connection();
  const params = new URLSearchParams();
  if (category) params.set("category", category);
  if (limit) params.set("limit", String(limit));
  const { data } = await backend.get<FaqWithPeople[]>(`/support/faqs?${params}`);
  return data;
}

/** The FAQ with id `id`, or `null` if there is none. */
export async function getFaq(id: string): Promise<FaqWithPeople | null> {
  if (!/^\d+$/.test(id)) return null;
  await connection();
  try {
    const { data } = await backend.get<FaqWithPeople>(`/support/faqs/${id}`);
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}
