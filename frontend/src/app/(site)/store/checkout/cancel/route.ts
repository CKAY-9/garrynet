import { redirect } from "next/navigation";
import type { NextRequest } from "next/server";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { Purchase } from "@/lib/types";

/**
 * Stripe sends buyers who back out of Checkout here (`?purchase=<id>&token=<token>`). The
 * purchase is cancelled on the backend, which returns its reserved stock and closes the
 * Stripe session, and the buyer goes back to the item. Only the buyer's own purchases can be
 * cancelled, only with the token the backend put in this link (so another site can't cancel
 * a buyer's checkout by sending them here), and a purchase that is no longer pending is left
 * alone.
 */
export async function GET(request: NextRequest) {
  const id = request.nextUrl.searchParams.get("purchase");
  const token = request.nextUrl.searchParams.get("token");
  let to = "/store";
  if (id && /^\d+$/.test(id) && token && /^[0-9a-f]{64}$/.test(token)) {
    try {
      const { data } = await backend.post<Purchase>(`/store/purchases/${id}/cancel`, { token }, {
        headers: await authHeaders(),
      });
      to = `/store/${data.store_id}/items/${data.item_id}?cancelled=1`;
    } catch (err) {
      // Log only the message: the axios error carries the request config, including the token.
      console.error("Failed to cancel purchase:", err instanceof Error ? err.message : err);
    }
  }
  redirect(to);
}
