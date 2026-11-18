import { Suspense } from "react";
import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { requirePermission } from "@/lib/auth";
import { hasPermission, Permission } from "@/lib/permissions";
import { getServer, serverStatus } from "@/lib/servers";
import { getUndelivered } from "@/lib/store";
import { loadingUrlLine } from "@/lib/loadingUrl";
import { siteOrigin } from "@/lib/site";
import { dateFormat, fullFormat, timeAgo } from "@/lib/time";
import styles from "../../admin.module.scss";
import { StatusBadge } from "../StatusBadge";
import servers from "../servers.module.scss";
import { AccessKeyPanel } from "./AccessKeyPanel";
import { DeleteServer } from "./DeleteServer";
import { ServerEditor } from "./ServerEditor";
import { UndeliveredPurchases } from "./UndeliveredPurchases";

export const metadata: Metadata = {
  title: "Server",
};

export default function ServerPage({ params }: PageProps<"/admin/servers/[server_id]">) {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      {params.then(({ server_id }) => (
        <ServerSettings id={server_id} />
      ))}
    </Suspense>
  );
}

async function ServerSettings({ id }: { id: string }) {
  const user = await requirePermission(Permission.AdminAccess | Permission.ServerManagement);
  const server = await getServer(id);
  if (!server) notFound();
  const [origin, undelivered] = await Promise.all([siteOrigin(), getUndelivered(server.id)]);

  return (
    <>
      <Link href="/admin/servers" className={servers.back}>
        ← All servers
      </Link>

      <section className={styles.section} aria-labelledby="server-heading">
        <div className={servers.titleRow}>
          <h2 id="server-heading" className={styles.sectionTitle}>
            {server.name} <span className={servers.id}>#{server.id}</span>
          </h2>
          <StatusBadge status={serverStatus(server)} />
        </div>
        <dl className={servers.facts}>
          <Fact label="Gamemode" value={server.gamemode} />
          <Fact label="Map" value={server.map} />
          <div>
            <dt>Last heartbeat</dt>
            <dd>
              {server.last_heartbeat ? (
                <time dateTime={server.last_heartbeat} title={fullFormat.format(new Date(server.last_heartbeat))}>
                  {timeAgo(new Date(server.last_heartbeat))}
                </time>
              ) : (
                <span className={servers.muted}>Never</span>
              )}
            </dd>
          </div>
          <div>
            <dt>Registered</dt>
            <dd>
              <time dateTime={server.created_at} title={fullFormat.format(new Date(server.created_at))}>
                {dateFormat.format(new Date(server.created_at))}
              </time>
            </dd>
          </div>
        </dl>
        <ServerEditor server={server} />
      </section>

      <section className={styles.section} aria-labelledby="access-key-heading">
        <h2 id="access-key-heading" className={styles.sectionTitle}>
          Access key
        </h2>
        <AccessKeyPanel server={server} />
      </section>

      <section className={styles.section} aria-labelledby="setup-heading">
        <h2 id="setup-heading" className={styles.sectionTitle}>
          Loading screen
        </h2>
        <p className={styles.note}>
          Add this to the server&apos;s <code>server.cfg</code> to use the GravNet{" "}
          <Link href="/admin/loading-screen">loading screen</Link>.
        </p>
        <pre className={styles.code}>
          <code>{loadingUrlLine(origin, server.id)}</code>
        </pre>
      </section>

      <section className={styles.section} aria-labelledby="danger-heading">
        <h2 id="danger-heading" className={styles.sectionTitle}>
          Danger zone
        </h2>
        <UndeliveredPurchases
          serverId={server.id}
          count={undelivered.count}
          storeId={undelivered.store_id}
          canRefund={hasPermission(user.permissions, Permission.AdminAccess | Permission.StoreFinance)}
        />
        <DeleteServer server={server} blocked={undelivered.count > 0} />
      </section>
    </>
  );
}

function Fact({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <dt>{label}</dt>
      <dd>{value || <span className={servers.muted}>Unknown until the first heartbeat</span>}</dd>
    </div>
  );
}
