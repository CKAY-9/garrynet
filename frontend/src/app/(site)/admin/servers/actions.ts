"use server";

import axios from "axios";
import { refresh } from "next/cache";
import { redirect } from "next/navigation";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { Server, ServerDetails, ServerWithAccessKey } from "@/lib/types";

export type ActionResult<T> = { ok: true; data: T } | { ok: false; message: string };

/** Registers a server. The result holds its access key, which is never shown again. */
export async function registerServer(details: ServerDetails): Promise<ActionResult<ServerWithAccessKey>> {
  return run("register the server", async () => {
    const { data } = await backend.post<ServerWithAccessKey>("/servers", details, { headers: await authHeaders() });
    refresh();
    return data;
  });
}

/** Saves a server's details. The backend validates them and checks the permissions. */
export async function updateServer(id: number, details: ServerDetails): Promise<ActionResult<Server>> {
  return run("save the server", async () => {
    const { data } = await backend.put<Server>(`/servers/${id}`, details, { headers: await authHeaders() });
    refresh();
    return data;
  });
}

/** Generates a new access key, which unlinks the server until it links again with the new key. */
export async function regenerateAccessKey(id: number): Promise<ActionResult<ServerWithAccessKey>> {
  return run("regenerate the access key", async () => {
    const { data } = await backend.post<ServerWithAccessKey>(`/servers/${id}/access-key`, null, {
      headers: await authHeaders(),
    });
    refresh();
    return data;
  });
}

/** Deletes a server and its player records, then returns to the server list. */
export async function deleteServer(id: number): Promise<ActionResult<never>> {
  const result = await run("delete the server", async () => {
    await backend.delete(`/servers/${id}`, { headers: await authHeaders() });
  });
  if (!result.ok) return result;
  redirect("/admin/servers");
}

async function run<T>(what: string, request: () => Promise<T>): Promise<ActionResult<T>> {
  try {
    return { ok: true, data: await request() };
  } catch (err) {
    const status = axios.isAxiosError(err) ? err.response?.status : undefined;
    if (status === 400) {
      const error = axios.isAxiosError(err) ? err.response?.data?.error : undefined;
      return { ok: false, message: typeof error === "string" ? capitalize(error) + "." : "Check the details and try again." };
    }
    if (status === 401) return { ok: false, message: `Your session has expired. Log in again to ${what}.` };
    if (status === 403) return { ok: false, message: "You don't have permission to manage servers." };
    if (status === 404) return { ok: false, message: "This server no longer exists." };
    // Log only the message: the axios error carries the request config, including the token.
    console.error(`Failed to ${what}:`, err instanceof Error ? err.message : err);
    return { ok: false, message: `Couldn't ${what}. Try again.` };
  }
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
