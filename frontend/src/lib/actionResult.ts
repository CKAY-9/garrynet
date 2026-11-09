import "server-only";
import axios from "axios";

export type ActionResult<T> = { ok: true; data: T } | { ok: false; message: string };

/**
 * Runs a backend request for a server action, turning failures into a message for the
 * form. `what` completes "Couldn't ...", e.g. "post the reply"; `messages` overrides the
 * message for a status code.
 */
export async function runAction<T>(
  what: string,
  request: () => Promise<T>,
  messages: Partial<Record<number, string>> = {},
): Promise<ActionResult<T>> {
  try {
    return { ok: true, data: await request() };
  } catch (err) {
    const status = axios.isAxiosError(err) ? err.response?.status : undefined;
    const error = axios.isAxiosError(err) ? err.response?.data?.error : undefined;
    if (status !== undefined && messages[status]) return { ok: false, message: messages[status] };
    if (status === 400) {
      return { ok: false, message: typeof error === "string" ? capitalize(error) + "." : "Check the details and try again." };
    }
    if (status === 401) return { ok: false, message: `Your session has expired. Log in again to ${what}.` };
    if (status === 403) return { ok: false, message: `You don't have permission to ${what}.` };
    if (status === 404 || status === 423) {
      return { ok: false, message: typeof error === "string" ? capitalize(error) + "." : `Couldn't ${what}.` };
    }
    // Log only the message: the axios error carries the request config, including the token.
    console.error(`Failed to ${what}:`, err instanceof Error ? err.message : err);
    return { ok: false, message: `Couldn't ${what}. Try again.` };
  }
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
