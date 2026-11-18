import "server-only";
import { cache } from "react";
import { connection } from "next/server";
import axios from "axios";
import { authHeaders, getCurrentUser } from "./auth";
import { backend } from "./backend";
import { hasPermission, Permission } from "./permissions";
import { receiptStatuses } from "./storeEnums";
import type {
  AdminStorePage,
  ItemPage,
  ReceiptFilters,
  ReceiptPage,
  ReceiptView,
  StorePage,
  StoreWithServer,
  UndeliveredPurchases,
} from "./types";

/*
 * Store reads (docs/STORE.md). All request-time, so render anything that uses these inside
 * a `<Suspense>` boundary. Writes are server actions in `@/components/Store/actions` and
 * `@/app/(site)/admin/store/actions`.
 */

/** A page's `searchParams`. */
export type SearchParams = Promise<Record<string, string | string[] | undefined>>;

export function firstParam(params: Record<string, string | string[] | undefined>, key: string): string {
  const value = params[key];
  return (Array.isArray(value) ? value[0] : value)?.trim() ?? "";
}

/** The receipt filters from the URL; unknown values are dropped. */
export async function readReceiptFilters(searchParams: SearchParams): Promise<ReceiptFilters> {
  const params = await searchParams;
  const status = firstParam(params, "status");
  const digits = (v: string) => (/^\d+$/.test(v) ? v : "");
  return {
    status: receiptStatuses.includes(status as never) ? (status as ReceiptFilters["status"]) : "",
    store: digits(firstParam(params, "store")),
    buyer: digits(firstParam(params, "buyer")),
  };
}

/** Who the current user is to the store. Reads the session cookie. */
export type StoreViewer = {
  /** `null` when logged out. */
  userId: number | null;
  /** Has Admin Access and Store Management. */
  manage: boolean;
  /** Has Admin Access and Store Finance. */
  finance: boolean;
  /** Has Elevated Control (may edit a store's grantable usergroups). */
  elevated: boolean;
};

export async function getStoreViewer(): Promise<StoreViewer> {
  const user = await getCurrentUser();
  if (!user) return { userId: null, manage: false, finance: false, elevated: false };
  const admin = hasPermission(user.permissions, Permission.AdminAccess);
  return {
    userId: user.id,
    manage: admin && hasPermission(user.permissions, Permission.StoreManagement),
    finance: admin && hasPermission(user.permissions, Permission.StoreFinance),
    elevated: hasPermission(user.permissions, Permission.ElevatedControl),
  };
}

async function getOr404<T>(request: () => Promise<{ data: T }>): Promise<T | null> {
  try {
    return (await request()).data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}

/** Every enabled store with its server. */
export async function getStores(): Promise<StoreWithServer[]> {
  // Axios reads the clock, which prerendering forbids.
  await connection();
  const { data } = await backend.get<StoreWithServer[]>("/store");
  return data;
}

/** An enabled store with its items, or `null` if there is none (or it's disabled). */
export const getStore = cache(async (id: string): Promise<StorePage | null> => {
  if (!/^\d+$/.test(id)) return null;
  await connection();
  return getOr404(() => backend.get<StorePage>(`/store/${id}`));
});

/**
 * An item with, for logged-in users, whether they can buy it. `null` if the store or item
 * doesn't exist, is archived or the store is disabled.
 */
export const getItem = cache(async (storeId: string, itemId: string): Promise<ItemPage | null> => {
  if (!/^\d+$/.test(storeId) || !/^\d+$/.test(itemId)) return null;
  await connection();
  return getOr404(async () =>
    backend.get<ItemPage>(`/store/${storeId}/items/${itemId}`, { headers: await authHeaders() }),
  );
});

/** The current user's receipts, newest first. Login required. */
export async function getMyPurchases(): Promise<ReceiptView[]> {
  await connection();
  const { data } = await backend.get<ReceiptView[]>("/store/purchases", { headers: await authHeaders() });
  return data;
}

/** Every store, enabled or not. The backend requires Admin Access and Store Management or Store Finance. */
export async function getAdminStores(): Promise<StoreWithServer[]> {
  await connection();
  const { data } = await backend.get<StoreWithServer[]>("/admin/store", { headers: await authHeaders() });
  return data;
}

/** A store's management page, or `null` if there is no such store. Requires Admin Access and Store Management. */
export async function getAdminStore(id: string): Promise<AdminStorePage | null> {
  if (!/^\d+$/.test(id)) return null;
  await connection();
  return getOr404(async () => backend.get<AdminStorePage>(`/admin/store/${id}`, { headers: await authHeaders() }));
}

/**
 * How many paid purchases from server `serverId`'s store haven't been delivered, which keeps
 * the server from being deleted. Requires Admin Access and Server Management or Store Finance.
 */
export async function getUndelivered(serverId: number): Promise<UndeliveredPurchases> {
  await connection();
  const { data } = await backend.get<UndeliveredPurchases>(`/admin/store/servers/${serverId}/undelivered`, {
    headers: await authHeaders(),
  });
  return data;
}

/**
 * A page of the receipts matching the filters, newest first; `cursor` is the previous page's
 * `next_cursor`. Requires Admin Access and Store Finance.
 */
export async function getReceipts(filters: ReceiptFilters, cursor = ""): Promise<ReceiptPage> {
  await connection();
  const params = receiptParams(filters, cursor);
  const { data } = await backend.get<ReceiptPage>(`/admin/store/receipts?${params}`, { headers: await authHeaders() });
  return data;
}

/** The query string of a page of receipts: the filters that are set, and the cursor if any. */
export function receiptParams(filters: ReceiptFilters, cursor = ""): URLSearchParams {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters)) {
    const trimmed = value?.trim();
    if (trimmed) params.set(key, trimmed);
  }
  if (cursor) params.set("cursor", cursor);
  return params;
}
