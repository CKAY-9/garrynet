import type { LegalSlug } from "./types";

/**
 * The legal documents, in the order they're listed. Fixed by the backend
 * (src/models/legal.rs), so links don't need a request. Safe for client components.
 */
export const legalPages: { slug: LegalSlug; label: string }[] = [
  { slug: "terms", label: "Terms" },
  { slug: "privacy", label: "Privacy" },
  { slug: "refunds", label: "Refunds" },
];

export const legalHref = (slug: LegalSlug) => `/legal/${slug}`;

export function isLegalSlug(slug: string): slug is LegalSlug {
  return legalPages.some((p) => p.slug === slug);
}
