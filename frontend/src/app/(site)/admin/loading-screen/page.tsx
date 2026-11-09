import { Suspense } from "react";
import Link from "next/link";
import { previewPlayer } from "@/components/LoadingScreen";
import { requirePermission } from "@/lib/auth";
import { getLoadingScreenConfig } from "@/lib/loadingScreen";
import { Permission } from "@/lib/permissions";
import { loadingUrlLine, siteOrigin } from "@/lib/site";
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
  const loadingUrl = loadingUrlLine(await siteOrigin(), "<SERVER_ID>");

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
          of <code>&lt;SERVER_ID&gt;</code> (each server&apos;s page under <Link href="/admin/servers">Servers</Link> shows its
          own line). Garry&apos;s Mod fills in the player&apos;s SteamID (
          <code>%s</code>) and the map (<code>%m</code>).
        </p>
        <pre className={styles.code}>
          <code>{loadingUrl}</code>
        </pre>
      </section>
    </>
  );
}
