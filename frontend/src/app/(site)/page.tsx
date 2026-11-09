import { Suspense } from "react";
import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { FALLBACK_COMMUNITY_NAME, getCommunity, linkLabel } from "@/lib/community";
import { defaultCommunityLogo } from "@/lib/config";
import type { KeyFigure } from "@/lib/types";
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
