import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { ImageResponse } from "next/og";
import { FALLBACK_COMMUNITY_NAME, getCommunity } from "@/lib/community";
import { backendUrl } from "@/lib/backend";
import { publicApiUrl } from "@/lib/config";
import { truncate } from "@/lib/text";

/*
 * The link preview image (Discord, X, Facebook, ...) for every website page: the
 * community's logo, name and description over its splash background. Rendered per
 * request, so changes in Admin › Community show up as soon as the sites re-fetch it.
 */

export const alt = "Community banner";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

const BACKGROUND = "#0f1115";
/** Formats the image renderer can draw. WebP and AVIF logos fall back to the default. */
const DRAWABLE = ["image/png", "image/jpeg", "image/svg+xml"];
const MAX_IMAGE_BYTES = 8 * 1024 * 1024;
const MAX_DESCRIPTION = 160;

/** `public/logo.svg`, for communities without a drawable logo of their own. */
const defaultLogo = readFile(join(process.cwd(), "public/logo.svg"), "base64").then(
  (data) => `data:image/svg+xml;base64,${data}`,
);

export default async function Image() {
  const community = await getCommunity();
  const name = community?.name ?? FALLBACK_COMMUNITY_NAME;
  const description = truncate(community?.description ?? "", MAX_DESCRIPTION);

  const [logo, splash] = await Promise.all([
    community?.logo_url ? loadImage(community.logo_url) : null,
    community?.splash_background ? loadImage(community.splash_background) : null,
  ]);
  const logoSrc = logo ?? (await defaultLogo);

  return new ImageResponse(
    (
      <div style={{ display: "flex", width: "100%", height: "100%", background: BACKGROUND, position: "relative" }}>
        {splash && (
          <img
            src={splash}
            alt=""
            width={size.width}
            height={size.height}
            style={{ position: "absolute", top: 0, left: 0, width: "100%", height: "100%", objectFit: "cover" }}
          />
        )}
        {/* Darkens the splash so the text stays readable over any picture. */}
        <div
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            width: "100%",
            height: "100%",
            display: "flex",
            backgroundImage: splash
              ? "linear-gradient(90deg, rgba(15,17,21,0.92) 0%, rgba(15,17,21,0.75) 55%, rgba(15,17,21,0.45) 100%)"
              : "radial-gradient(circle at 20% 30%, rgba(59,130,246,0.18), rgba(15,17,21,0) 60%)",
          }}
        />
        <div style={{ display: "flex", alignItems: "center", gap: 56, padding: "0 88px", width: "100%" }}>
          <img
            src={logoSrc}
            alt={name}
            width={240}
            height={240}
            style={{ borderRadius: 40, objectFit: "cover", flexShrink: 0 }}
          />
          <div style={{ display: "flex", flexDirection: "column", gap: 20, minWidth: 0, flex: 1 }}>
            <div style={{ display: "flex", fontSize: name.length > 24 ? 60 : 76, color: "#f5f6f8", lineHeight: 1.1 }}>
              {name}
            </div>
            {description && (
              <div style={{ display: "flex", fontSize: 32, color: "#b5bac4", lineHeight: 1.4 }}>{description}</div>
            )}
          </div>
        </div>
      </div>
    ),
    { ...size },
  );
}

/**
 * An image as a data URI, or `null` if it can't be fetched or drawn. Backend files are
 * fetched through `API_URL`, since the public backend URL may not resolve from inside
 * the compose network.
 */
async function loadImage(url: string): Promise<string | null> {
  const target = url.startsWith(`${publicApiUrl}/`) ? backendUrl + url.slice(publicApiUrl.length) : url;
  try {
    const res = await fetch(target, { signal: AbortSignal.timeout(5000) });
    const type = res.headers.get("content-type")?.split(";")[0].trim() ?? "";
    if (!res.ok || !DRAWABLE.includes(type)) return null;
    if (Number(res.headers.get("content-length") ?? 0) > MAX_IMAGE_BYTES) return null;
    const bytes = Buffer.from(await res.arrayBuffer());
    if (bytes.length > MAX_IMAGE_BYTES) return null;
    return `data:${type};base64,${bytes.toString("base64")}`;
  } catch (err) {
    console.error("Failed to load a preview image:", err instanceof Error ? err.message : err);
    return null;
  }
}
