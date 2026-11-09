import { Suspense } from "react";
import Link from "next/link";
import { previewPlayer } from "@/components/LoadingScreen";
import { requirePermission } from "@/lib/auth";
import { getLoadingScreenConfig } from "@/lib/loadingScreen";
import { Permission } from "@/lib/permissions";
import { getServers } from "@/lib/servers";
import { siteOrigin } from "@/lib/site";
import styles from "../admin.module.scss";
import { LoadingScreenEditor } from "./LoadingScreenEditor";
import { ServerSetupLine } from "./ServerSetupLine";

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
  const [config, servers, origin] = await Promise.all([getLoadingScreenConfig(), getServers(), siteOrigin()]);

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
          Pick a server to fill in its GarryNet id, then add the line to that server&apos;s <code>server.cfg</code>
          (each server&apos;s page under <Link href="/admin/servers">Servers</Link> shows its own line too).
          Garry&apos;s Mod fills in the player&apos;s SteamID (<code>%s</code>) and the map (<code>%m</code>).
        </p>
        <ServerSetupLine origin={origin} servers={servers.map(({ id, name }) => ({ id, name }))} />
      </section>
    </>
  );
}
