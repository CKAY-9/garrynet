"use client";

import { useState } from "react";
import type { Server } from "@/lib/types";
import { loadingUrlLine } from "@/lib/loadingUrl";
import admin from "../admin.module.scss";
import styles from "./editor.module.scss";

const PLACEHOLDER = "<SERVER_ID>";

/** Server picker that fills the picked server's id into the `sv_loadingurl` line. */
export function ServerSetupLine({ origin, servers }: { origin: string; servers: Pick<Server, "id" | "name">[] }) {
  const [serverId, setServerId] = useState("");
  const line = loadingUrlLine(origin, serverId || PLACEHOLDER);

  return (
    <>
      {servers.length > 0 && (
        <label className={styles.serverPicker}>
          <span>Server</span>
          <select className={styles.select} value={serverId} onChange={(event) => setServerId(event.target.value)}>
            <option value="">Pick a server…</option>
            {servers.map((server) => (
              <option key={server.id} value={server.id}>
                {server.name} (#{server.id})
              </option>
            ))}
          </select>
        </label>
      )}
      <pre className={admin.code}>
        <code>{line}</code>
      </pre>
    </>
  );
}
