"use server";

import { refresh } from "next/cache";
import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { LegalDocument, LegalSlug } from "@/lib/types";

/** Saves a legal document. The backend sanitizes the Markdown and checks the permissions. */
export async function saveLegalDocument(
  slug: LegalSlug,
  details: { title: string; body: string },
): Promise<ActionResult<LegalDocument>> {
  return runAction("save the document", async () => {
    const { data } = await backend.put<LegalDocument>(`/legal/${slug}`, details, { headers: await authHeaders() });
    refresh();
    return data;
  });
}

/** The default template GarryNet ships for a document. */
export async function loadLegalTemplate(slug: LegalSlug): Promise<ActionResult<{ title: string; body: string }>> {
  return runAction("load the template", async () => {
    const { data } = await backend.get<{ title: string; body: string }>(`/legal/${slug}/template`, {
      headers: await authHeaders(),
    });
    return { title: data.title, body: data.body.trim() };
  });
}
