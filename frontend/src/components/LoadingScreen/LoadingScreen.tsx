import type { CSSProperties } from "react";
import { dateFormat, formatMinutes, fullFormat, timeAgo } from "@/lib/time";
import type { LoadingScreenPlayer, LoadingScreenStyle, Server } from "@/lib/types";
import { LoadingProgress } from "./LoadingProgress";
import styles from "./LoadingScreen.module.scss";

/** The server details the loading screen shows. */
export type LoadingScreenServer = Pick<Server, "name" | "gamemode" | "map" | "icon_url" | "color">;

type Props = {
  style: LoadingScreenStyle;
  server: LoadingScreenServer;
  /** `null` when the player isn't known (e.g. the page was opened outside the game). */
  player: LoadingScreenPlayer | null;
  /** `live` follows Garry's Mod's loading progress; `demo` loops a fake one for previews. */
  mode: "live" | "demo";
};

/** How many gamemode stats to show, so the card stays compact. */
const MAX_STATS = 4;

/**
 * A server's loading screen. It fills its parent and scales with it (sizes are relative to
 * the parent's height), so the in-game page and the website previews share it.
 *
 * The structure is the same for every style and server; the style picks the background,
 * animations and look, and the server sets the accent color and icon.
 */
export function LoadingScreen({ style, server, player, mode }: Props) {
  const accent = { "--accent": server.color } as CSSProperties;
  const details = [server.gamemode, server.map].filter(Boolean).join(" · ");

  return (
    <div className={`${styles.screen} ${styles[style]}`} style={accent}>
      <Backdrop />
      <div className={styles.stage}>
        <header className={styles.server}>
          <ServerIcon server={server} />
          <div className={styles.serverText}>
            <h1 className={styles.serverName}>{server.name}</h1>
            {details && <p className={styles.serverDetails}>{details}</p>}
          </div>
        </header>

        {player && <PlayerCard player={player} gamemode={server.gamemode} />}

        <footer className={styles.footer}>
          <LoadingProgress mode={mode} />
        </footer>
      </div>
    </div>
  );
}

/** Purely decorative layers; each style shows the ones it uses. */
function Backdrop() {
  return (
    <div className={styles.backdrop} aria-hidden>
      <div className={styles.glow} />
      <div className={styles.rays} />
      <div className={styles.ribbons} />
      <div className={styles.hills} />
      <div className={styles.bubbles}>
        {BUBBLES.map(([left, size, duration, delay], i) => (
          <span
            key={i}
            style={
              {
                "--left": `${left}%`,
                "--size": `${size}em`,
                "--duration": `${duration}s`,
                "--delay": `${delay}s`,
              } as CSSProperties
            }
          />
        ))}
      </div>
      <div className={styles.grid} />
    </div>
  );
}

/** [left %, size em, rise duration s, delay s]: fixed, so server and client render alike. */
const BUBBLES: [number, number, number, number][] = [
  [6, 2.2, 19, -3],
  [14, 1.1, 14, -9],
  [23, 3.4, 24, -15],
  [31, 1.6, 17, -1],
  [42, 0.9, 12, -6],
  [53, 2.8, 22, -12],
  [61, 1.3, 15, -4],
  [70, 4, 27, -20],
  [79, 1.8, 18, -8],
  [88, 1, 13, -2],
  [94, 2.5, 21, -16],
];

function ServerIcon({ server }: { server: LoadingScreenServer }) {
  if (!server.icon_url) {
    return (
      <span className={styles.serverIcon} aria-hidden>
        <span className={styles.iconLetter}>{initial(server.name)}</span>
      </span>
    );
  }
  // A plain <img>: server icons can be hosted anywhere, so next/image can't optimize them.
  // eslint-disable-next-line @next/next/no-img-element
  return <img className={styles.serverIcon} src={server.icon_url} alt="" />;
}

function PlayerCard({ player, gamemode }: { player: LoadingScreenPlayer; gamemode: string }) {
  const record = player.record;
  // `last_logon` equals `first_joined` until the player comes back.
  const returning = record !== null && record.last_logon !== record.first_joined;
  const stats = record ? displayStats(record.stats, gamemode) : [];

  return (
    <section className={styles.player} aria-label="Player">
      {player.avatar_url ? (
        // eslint-disable-next-line @next/next/no-img-element
        <img className={styles.avatar} src={player.avatar_url} alt="" />
      ) : (
        <span className={styles.avatar} aria-hidden>
          <span className={styles.avatarLetter}>{initial(player.name)}</span>
        </span>
      )}

      <div className={styles.playerText}>
        <p className={styles.greeting}>{returning ? "Welcome back," : "Welcome,"}</p>
        <h2 className={styles.playerName}>{player.name}</h2>

        {record ? (
          <dl className={styles.facts}>
            {returning && (
              <div>
                <dt>Last logon</dt>
                <dd>
                  <TimeAgo iso={record.last_logon} />
                </dd>
              </div>
            )}
            <div>
              <dt>First joined</dt>
              <dd>
                <time dateTime={record.first_joined} title={fullFormat.format(new Date(record.first_joined))}>
                  {dateFormat.format(new Date(record.first_joined))}
                </time>
              </dd>
            </div>
            <div>
              <dt>Time played</dt>
              <dd>{formatMinutes(record.time_played)}</dd>
            </div>
            <div>
              <dt>Rank</dt>
              <dd className={styles.rank}>{record.ingame_usergroup}</dd>
            </div>
          </dl>
        ) : (
          <p className={styles.firstVisit}>First time here. Enjoy your stay!</p>
        )}

        {stats.length > 0 && (
          <ul className={styles.stats} aria-label="Stats">
            {stats.map(([label, value]) => (
              <li key={label}>
                <span className={styles.statLabel}>{label}</span>
                <span className={styles.statValue}>{value}</span>
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  );
}

function TimeAgo({ iso }: { iso: string }) {
  const date = new Date(iso);
  return (
    <time dateTime={iso} title={fullFormat.format(date)}>
      {timeAgo(date)}
    </time>
  );
}

/**
 * The first few displayable gamemode stats as [label, value]. Keys lose their gamemode
 * prefix, so `darkrp_money` on DarkRP reads "Money".
 */
function displayStats(stats: Record<string, unknown>, gamemode: string): [string, string][] {
  const prefix = gamemode ? `${gamemode.toLowerCase()}_` : "";
  const out: [string, string][] = [];
  for (const [key, value] of Object.entries(stats ?? {})) {
    if (out.length >= MAX_STATS) break;
    let text: string;
    if (typeof value === "number") text = value.toLocaleString("en");
    else if (typeof value === "string" && value) text = value;
    else if (typeof value === "boolean") text = value ? "Yes" : "No";
    else continue;

    const name = prefix && key.toLowerCase().startsWith(prefix) ? key.slice(prefix.length) : key;
    const label = name.replace(/_/g, " ").trim();
    out.push([label.charAt(0).toUpperCase() + label.slice(1), text]);
  }
  return out;
}

function initial(name: string): string {
  return Array.from(name.trim())[0]?.toUpperCase() ?? "?";
}
