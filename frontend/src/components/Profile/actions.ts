"use server";

import { refresh } from "next/cache";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { Punishment, PunishmentKind } from "@/lib/types";

/*
 * Banning and suspending (docs/USERS.md "User Punishment"). The backend checks Admin Access
 * and User Management, refuses targeting yourself or anyone who holds permissions you
 * don't, and validates the details.
 */

const nouns: Record<PunishmentKind, string> = { BAN: "ban", SUSPEND: "suspension" };

/** Bans or suspends the user. `ends` is an ISO timestamp, or `null` for a permanent ban. */
export async function punishUser(
  userId: number,
  kind: PunishmentKind,
  reason: string,
  ends: string | null,
): Promise<ActionResult<Punishment>> {
  return runAction(
    kind === "BAN" ? "ban this user" : "suspend this user",
    async () => {
      const { data } = await backend.post<Punishment>(
        `/users/${userId}/punishments`,
        { kind, reason, ends },
        { headers: await authHeaders() },
      );
      refresh();
      return data;
    },
    { 409: `This user already has an active ${nouns[kind]}. Refresh the page.` },
  );
}

/** Lifts the user's ban or suspension early, with an optional note on why. */
export async function liftPunishment(userId: number, kind: PunishmentKind, reason: string): Promise<ActionResult<Punishment>> {
  return runAction(
    kind === "BAN" ? "unban this user" : "lift this suspension",
    async () => {
      const { data } = await backend.post<Punishment>(
        `/users/${userId}/punishments/lift`,
        { kind, reason },
        { headers: await authHeaders() },
      );
      refresh();
      return data;
    },
    { 404: `This user has no active ${nouns[kind]}. Refresh the page.` },
  );
}
