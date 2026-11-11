import "server-only";
import axios from "axios";
import { connection } from "next/server";
import { backend } from "./backend";
import { isLegalSlug } from "./legalPages";
import type { LegalDocument } from "./types";

/** A legal document, or `null` if there's no such document. Public. */
export async function getLegalDocument(slug: string): Promise<LegalDocument | null> {
  if (!isLegalSlug(slug)) return null;
  // Axios reads the clock, which prerendering forbids.
  await connection();
  try {
    const { data } = await backend.get<LegalDocument>(`/legal/${slug}`);
    return data;
  } catch (err) {
    if (axios.isAxiosError(err) && err.response?.status === 404) return null;
    throw err;
  }
}
