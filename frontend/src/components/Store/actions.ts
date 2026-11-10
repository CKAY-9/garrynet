"use server";

import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { Checkout } from "@/lib/types";

/**
 * Starts buying an item: the backend reserves stock and opens a Stripe Checkout Session.
 * The result holds the Checkout URL to send the buyer to.
 */
export async function startCheckout(storeId: number, itemId: number): Promise<ActionResult<Checkout>> {
  return runAction(
    "start the purchase",
    async () => {
      const { data } = await backend.post<Checkout>(`/store/${storeId}/items/${itemId}/checkout`, null, {
        headers: await authHeaders(),
      });
      return data;
    },
    {
      401: "Log in with Steam to buy items.",
      403: "Your account can't make purchases.",
      423: "This item can't be bought right now.",
      502: "The payment provider couldn't be reached. Try again in a moment.",
    },
  );
}
