import { Suspense, type CSSProperties } from "react";
import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { FALLBACK_COMMUNITY_NAME, getCommunity, linkLabel } from "@/lib/community";
import { defaultCommunityLogo } from "@/lib/config";
import { getServers, serverStatus } from "@/lib/servers";
import type { KeyFigure, Server } from "@/lib/types";
import styles from "./home.module.scss";

export default function Home() {
  return (
    // The community is fetched at request time, so it streams in behind the boundary.
    <Suspense fallback={<HomeSkeleton />}>
      <CommunityHome />
    </Suspense>
  );
}

async function CommunityHome() {
  const community = await getCommunity();

  if (!community) {
    return (
      <div className={styles.home}>
        <header className={styles.hero}>
          <div className={styles.heroContent}>
            <Avatar name={FALLBACK_COMMUNITY_NAME} url={defaultCommunityLogo} size={88} className={styles.logo} />
            <div className={styles.heroText}>
              <h1 className={styles.name}>{FALLBACK_COMMUNITY_NAME}</h1>
              <p className={styles.description}>The community couldn&apos;t be loaded. Try again shortly.</p>
            </div>
          </div>
        </header>
      </div>
    );
  }

  const { name, description, logo_url, splash_background, external_links, key_figures } = community;

  return (
    <div className={styles.home}>
      <header
        className={`${styles.hero} ${splash_background ? styles.splash : ""}`}
        style={splash_background ? { backgroundImage: `url(${JSON.stringify(splash_background)})` } : undefined}
      >
        <div className={styles.heroContent}>
          <Avatar name={name} url={logo_url ?? defaultCommunityLogo} size={88} className={styles.logo} />
          <div className={styles.heroText}>
            <h1 className={styles.name}>{name}</h1>
            {description && <p className={styles.description}>{description}</p>}
          </div>
        </div>
        {external_links.length > 0 && (
          <ul className={styles.links} aria-label="Community links">
            {external_links.map((url) => (
              <li key={url}>
                <a className={styles.link} href={url} target="_blank" rel="noopener noreferrer">
                  {linkLabel(url)}
                  <ExternalIcon />
                </a>
              </li>
            ))}
          </ul>
        )}
      </header>

      {key_figures.length > 0 && (
        <section className={styles.section} aria-labelledby="key-figures-heading">
          <h2 id="key-figures-heading" className={styles.sectionTitle}>
            Key figures
          </h2>
          <ul className={styles.figures}>
            {key_figures.map((figure, i) => (
              <li key={i}>
                <FigureCard figure={figure} />
              </li>
            ))}
          </ul>
        </section>
      )}

      {/* Separate boundary: the servers stream in on their own and can fail without the rest. */}
      <Suspense fallback={<ServersSkeleton />}>
        <ServersSection />
      </Suspense>
    </div>
  );
}

async function ServersSection() {
  let servers: Server[];
  try {
    servers = await getServers();
  } catch (err) {
    console.error("Failed to fetch servers:", err instanceof Error ? err.message : err);
    return null;
  }

  // Only servers the addon has connected; unlinked ones are still being set up.
  const connected = servers.filter((server) => server.linked);
  if (connected.length === 0) return null;

  return (
    <section className={styles.section} aria-labelledby="servers-heading">
      <h2 id="servers-heading" className={styles.sectionTitle}>
        Servers
      </h2>
      <ul className={styles.servers}>
        {connected.map((server) => (
          <li key={server.id}>
            <ServerCard server={server} online={serverStatus(server) === "online"} />
          </li>
        ))}
      </ul>
    </section>
  );
}

function ServerCard({ server, online }: { server: Server; online: boolean }) {
  const details = [server.gamemode, server.map].filter(Boolean).join(" · ");
  return (
    <article
      className={`${styles.server} ${server.splash_url ? styles.serverSplash : ""}`}
      style={
        {
          "--server-color": server.color,
          ...(server.splash_url && { backgroundImage: `url(${JSON.stringify(server.splash_url)})` }),
        } as CSSProperties
      }
    >
      <div className={styles.serverHeader}>
        <Avatar name={server.name} url={server.icon_url} size={48} />
        <div className={styles.serverIdentity}>
          <span className={styles.serverName}>{server.name}</span>
          {details && <span className={styles.serverDetails}>{details}</span>}
        </div>
        <span className={`${styles.serverStatus} ${online ? styles.online : ""}`}>
          <span className={styles.dot} aria-hidden />
          {online ? "Online" : "Offline"}
        </span>
      </div>
      {server.description && <p className={styles.serverDescription}>{server.description}</p>}
      {server.connection_address && (
        <div className={styles.serverFooter}>
          <code className={styles.serverAddress}>{server.connection_address}</code>
          <a className={styles.connect} href={`steam://connect/${server.connection_address}`}>
            <PlayIcon />
            Connect
          </a>
        </div>
      )}
    </article>
  );
}

function PlayIcon() {
  return (
    <svg viewBox="0 0 24 24" width={14} height={14} fill="currentColor" aria-hidden>
      <path d="M7 4.5v15a1 1 0 0 0 1.5.86l12.5-7.5a1 1 0 0 0 0-1.72L8.5 3.64A1 1 0 0 0 7 4.5z" />
    </svg>
  );
}

function ServersSkeleton() {
  return (
    <div className={styles.section} aria-busy aria-label="Loading servers">
      <div className={`${styles.server} ${styles.skeleton} ${styles.serverSkeleton}`} />
    </div>
  );
}

function FigureCard({ figure }: { figure: KeyFigure }) {
  const picture = <Avatar name={figure.name} url={figure.profile_picture || null} size={56} shape="circle" />;
  return (
    <article className={styles.figure}>
      {figure.user_id ? (
        <Link href={`/users/${figure.user_id}`} className={styles.figureHeader}>
          {picture}
          <FigureName figure={figure} />
        </Link>
      ) : (
        <div className={styles.figureHeader}>
          {picture}
          <FigureName figure={figure} />
        </div>
      )}
      {figure.description && <p className={styles.figureDescription}>{figure.description}</p>}
    </article>
  );
}

function FigureName({ figure }: { figure: KeyFigure }) {
  return (
    <span className={styles.figureIdentity}>
      <span className={styles.figureName}>{figure.name}</span>
      {figure.role && <span className={styles.figureRole}>{figure.role}</span>}
    </span>
  );
}

function ExternalIcon() {
  return (
    <svg viewBox="0 0 24 24" width={14} height={14} fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <path d="M14 4h6v6" />
      <path d="M20 4l-9 9" />
      <path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5" />
    </svg>
  );
}

function HomeSkeleton() {
  return (
    <div className={styles.home} aria-busy aria-label="Loading">
      <div className={`${styles.hero} ${styles.skeleton}`} />
    </div>
  );
}
