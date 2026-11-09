import { Suspense } from "react";
import { headers } from "next/headers";
import { previewPlayer } from "@/components/LoadingScreen";
import { requirePermission } from "@/lib/auth";
import { getLoadingScreenConfig } from "@/lib/loadingScreen";
import { Permission } from "@/lib/permissions";
import styles from "../admin.module.scss";
import { LoadingScreenEditor } from "./LoadingScreenEditor";

export default function LoadingScreenPage() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <LoadingScreenSettings />
    </Suspense>
  );
}

async function LoadingScreenSettings() {
  await requirePermission(Permission.AdminAccess | Permission.ServerManagement);
  const config = await getLoadingScreenConfig();
  const loadingUrl = `${await siteOrigin()}/loading-screen?server=<SERVER_ID>&steamid=%s&mapname=%m`;

  return (
    <>
      <section className={styles.section} aria-labelledby="loading-screen-heading">
        <h2 id="loading-screen-heading" className={styles.sectionTitle}>
          Loading Screen
        </h2>
        <LoadingScreenEditor saved={config.style} player={previewPlayer()} />
      </section>

      <section className={styles.section} aria-labelledby="loading-screen-setup-heading">
        <h2 id="loading-screen-setup-heading" className={styles.sectionTitle}>
          Server setup
        </h2>
        <p className={styles.note}>
          Add this to each server&apos;s <code>server.cfg</code>, with the server&apos;s GarryNet id in place
          of <code>&lt;SERVER_ID&gt;</code>. Garry&apos;s Mod fills in the player&apos;s SteamID (
          <code>%s</code>) and the map (<code>%m</code>).
        </p>
        <pre className={styles.code}>
          <code>sv_loadingurl &quot;{loadingUrl}&quot;</code>
        </pre>
      </section>
    </>
  );
}

/** This site's origin as the browser sees it, e.g. `https://example.com`. */
async function siteOrigin(): Promise<string> {
  const h = await headers();
  const host = h.get("x-forwarded-host") ?? h.get("host") ?? "localhost:3000";
  const proto = h.get("x-forwarded-proto") ?? (host.startsWith("localhost") ? "http" : "https");
  return `${proto}://${host}`;
}
