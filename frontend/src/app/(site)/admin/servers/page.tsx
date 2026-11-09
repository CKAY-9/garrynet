import { Suspense } from "react";
import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import { getServers, serverStatus } from "@/lib/servers";
import { fullFormat, timeAgo } from "@/lib/time";
import styles from "../admin.module.scss";
import { RegisterServer } from "./RegisterServer";
import { StatusBadge } from "./StatusBadge";
import list from "./servers.module.scss";

export default function ServersPage() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <Servers />
    </Suspense>
  );
}

async function Servers() {
  await requirePermission(Permission.AdminAccess | Permission.ServerManagement);
  const servers = await getServers();
  return (
    <>
      <section className={styles.section} aria-labelledby="servers-heading">
        <h2 id="servers-heading" className={styles.sectionTitle}>
          Servers
        </h2>
        {servers.length === 0 ? (
          <p className={styles.placeholder}>No servers yet. Register one below to get its access key.</p>
        ) : (
          <ul className={list.list}>
            {servers.map((server) => (
              <li key={server.id}>
                <Link href={`/admin/servers/${server.id}`} className={list.card} style={{ borderLeftColor: server.color }}>
                  <Avatar name={server.name} url={server.icon_url} size={44} />
                  <span className={list.cardBody}>
                    <span className={list.cardTitle}>
                      {server.name}
                      <span className={list.id}>#{server.id}</span>
                    </span>
                    <span className={list.cardMeta}>
                      {[server.gamemode, server.map, server.connection_address].filter(Boolean).join(" · ") ||
                        "No details from the server yet"}
                    </span>
                  </span>
                  <span className={list.cardStatus}>
                    <StatusBadge status={serverStatus(server)} />
                    {server.last_heartbeat && (
                      <time
                        className={list.heartbeat}
                        dateTime={server.last_heartbeat}
                        title={fullFormat.format(new Date(server.last_heartbeat))}
                      >
                        {timeAgo(new Date(server.last_heartbeat))}
                      </time>
                    )}
                  </span>
                </Link>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className={styles.section} aria-labelledby="register-heading">
        <h2 id="register-heading" className={styles.sectionTitle}>
          Register a server
        </h2>
        <RegisterServer />
      </section>
    </>
  );
}
