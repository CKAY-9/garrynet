import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import { getServer, serverStatus } from "@/lib/servers";
import { loadingUrlLine } from "@/lib/loadingUrl";
import { siteOrigin } from "@/lib/site";
import { dateFormat, fullFormat, timeAgo } from "@/lib/time";
import styles from "../../admin.module.scss";
import { StatusBadge } from "../StatusBadge";
import servers from "../servers.module.scss";
import { AccessKeyPanel } from "./AccessKeyPanel";
import { DeleteServer } from "./DeleteServer";
import { ServerEditor } from "./ServerEditor";

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
  await requirePermission(Permission.AdminAccess | Permission.ServerManagement);
  const server = await getServer(id);
  if (!server) notFound();
  const origin = await siteOrigin();

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
          Add this to the server&apos;s <code>server.cfg</code> to use the GarryNet{" "}
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
        <DeleteServer server={server} />
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
