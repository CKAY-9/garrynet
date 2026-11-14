import type { Metadata } from "next";
import { SiteShell } from "@/components/SiteShell";
import { FALLBACK_COMMUNITY_NAME, getCommunity } from "@/lib/community";
import { siteOrigin } from "@/lib/site";

const FALLBACK_DESCRIPTION = "An all-in-one web solution for Garry's Mod communities.";

/**
 * Titles ("Settings · <community>") and link previews (Discord, X, Facebook, ...) carry
 * the community's name and description. The preview image is `opengraph-image.tsx`.
 */
export async function generateMetadata(): Promise<Metadata> {
  const [community, origin] = await Promise.all([getCommunity(), siteOrigin()]);
  const name = community?.name ?? FALLBACK_COMMUNITY_NAME;
  const description = community?.description || FALLBACK_DESCRIPTION;

  return {
    // Preview images need absolute URLs.
    metadataBase: new URL(origin),
    title: { default: name, template: `%s · ${name}` },
    description,
    openGraph: { type: "website", siteName: name, title: name, description },
    twitter: { card: "summary_large_image", title: name, description },
  };
}

/** Every website page: side navigation plus content. */
export default function SiteLayout({ children }: LayoutProps<"/">) {
  return <SiteShell>{children}</SiteShell>;
}
