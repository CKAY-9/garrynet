import type { Metadata, ResolvingMetadata } from "next";
import { truncate } from "./text";

/** Link preview descriptions longer than this are cut; previews show about two lines. */
const MAX_DESCRIPTION = 200;

/**
 * Metadata for a page about one thing (a thread, a profile, an item): the tab title, and a
 * link preview titled after it, with `description` when there is one. Keeps the community's
 * preview image and site name, which a page setting `openGraph` would otherwise drop.
 */
export async function pageMetadata(
  parent: ResolvingMetadata,
  title: string,
  description?: string | null,
): Promise<Metadata> {
  const { openGraph, twitter } = await parent;
  const about = description?.trim() ? { description: truncate(description, MAX_DESCRIPTION) } : {};
  return {
    title,
    ...about,
    openGraph: { ...openGraph, title, ...about },
    twitter: { ...twitter, title, ...about },
  };
}
