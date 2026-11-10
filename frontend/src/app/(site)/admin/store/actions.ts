"use server";

import { refresh } from "next/cache";
import { redirect } from "next/navigation";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type {
  DiscountDetails,
  ItemDetails,
  ItemStore,
  PricedItem,
  ReceiptView,
  StoreDetails,
  StoreDiscount,
} from "@/lib/types";

/*
 * Store management (docs/STORE.md). The backend checks Admin Access plus Store Management
 * (Store Finance for refunds, Elevated Control for grantable usergroups) and validates
 * everything; these just relay the result to the forms.
 */

/** Creates a server's store, then opens its management page. 409 if it already has one. */
export async function createStore(serverId: number, details: StoreDetails): Promise<ActionResult<never>> {
  const result = await runAction(
    "create the store",
    async () => {
      const { data } = await backend.post<ItemStore>("/admin/store", { server_id: serverId, ...details }, {
        headers: await authHeaders(),
      });
      return data;
    },
    { 409: "This server already has a store." },
  );
  if (!result.ok) return result;
  redirect(`/admin/store/${result.data.id}`);
}

export async function updateStore(id: number, details: StoreDetails): Promise<ActionResult<ItemStore>> {
  return runAction("save the store", async () => {
    const { data } = await backend.put<ItemStore>(`/admin/store/${id}`, details, { headers: await authHeaders() });
    refresh();
    return data;
  });
}

/** Replaces which usergroups the store's items may grant. Needs Elevated Control. */
export async function setGrantableUsergroups(id: number, usergroups: string[]): Promise<ActionResult<ItemStore>> {
  return runAction(
    "change the grantable usergroups",
    async () => {
      const { data } = await backend.put<ItemStore>(
        `/admin/store/${id}/grantable-usergroups`,
        { grantable_usergroups: usergroups },
        { headers: await authHeaders() },
      );
      refresh();
      return data;
    },
    { 403: "Only users with Elevated Control can change which usergroups a store may grant." },
  );
}

/** Adds an item, then opens the store's management page. */
export async function createItem(storeId: number, details: ItemDetails): Promise<ActionResult<never>> {
  const result = await runAction("create the item", async () => {
    const { data } = await backend.post<PricedItem>(`/admin/store/${storeId}/items`, details, {
      headers: await authHeaders(),
    });
    return data;
  });
  if (!result.ok) return result;
  redirect(`/admin/store/${storeId}`);
}

export async function updateItem(storeId: number, itemId: number, details: ItemDetails): Promise<ActionResult<PricedItem>> {
  return runAction("save the item", async () => {
    const { data } = await backend.put<PricedItem>(`/admin/store/${storeId}/items/${itemId}`, details, {
      headers: await authHeaders(),
    });
    refresh();
    return data;
  });
}

/** Archives an item: hidden from players, kept for receipts. Then opens the store's page. */
export async function archiveItem(storeId: number, itemId: number): Promise<ActionResult<never>> {
  const result = await runAction("archive the item", async () => {
    await backend.delete(`/admin/store/${storeId}/items/${itemId}`, { headers: await authHeaders() });
  });
  if (!result.ok) return result;
  redirect(`/admin/store/${storeId}`);
}

export async function createDiscount(storeId: number, details: DiscountDetails): Promise<ActionResult<StoreDiscount>> {
  return runAction("create the discount", async () => {
    const { data } = await backend.post<StoreDiscount>(`/admin/store/${storeId}/discounts`, details, {
      headers: await authHeaders(),
    });
    refresh();
    return data;
  });
}

export async function updateDiscount(storeId: number, id: number, details: DiscountDetails): Promise<ActionResult<StoreDiscount>> {
  return runAction("save the discount", async () => {
    const { data } = await backend.put<StoreDiscount>(`/admin/store/${storeId}/discounts/${id}`, details, {
      headers: await authHeaders(),
    });
    refresh();
    return data;
  });
}

export async function deleteDiscount(storeId: number, id: number): Promise<ActionResult<void>> {
  return runAction("delete the discount", async () => {
    await backend.delete(`/admin/store/${storeId}/discounts/${id}`, { headers: await authHeaders() });
    refresh();
  });
}

/** Refunds a receipt in full through Stripe and revokes its fulfillment. Needs Store Finance. */
export async function refundReceipt(id: number): Promise<ActionResult<ReceiptView>> {
  return runAction(
    "refund the purchase",
    async () => {
      const { data } = await backend.post<ReceiptView>(`/admin/store/receipts/${id}/refund`, null, {
        headers: await authHeaders(),
      });
      refresh();
      return data;
    },
    { 502: "The payment provider couldn't be reached. Try again in a moment." },
  );
}
