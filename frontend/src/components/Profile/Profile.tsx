import Image from "next/image";
import { UsergroupChip } from "@/components/UsergroupChip";
import { dateFormat, fullFormat, timeAgo } from "@/lib/time";
import type { UserProfile, UserStatus } from "@/lib/types";
import styles from "./Profile.module.scss";

const statusLabels: Record<UserStatus, string> = {
  ONLINE: "Online",
  IDLE: "Idle",
  OFFLINE: "Offline",
};

/** A user's profile: avatar, name, status, usergroups, bio and stats. */
export function Profile({ user }: { user: UserProfile }) {
  const joined = new Date(user.joined);
  const lastSeen = new Date(user.last_seen);
  const online = user.status === "ONLINE";

  return (
    <article className={styles.profile}>
      <header className={styles.header}>
        <Image
          className={styles.avatar}
          src={user.avatar_url}
          alt=""
          width={128}
          height={128}
          priority
        />
        <div className={styles.identity}>
          <h1 className={styles.name}>{user.username}</h1>
          <span className={`${styles.status} ${styles[user.status.toLowerCase()]}`}>
            <span className={styles.statusDot} aria-hidden />
            {statusLabels[user.status]}
          </span>
          {user.usergroups.length > 0 && (
            <ul className={styles.groups} aria-label="Usergroups">
              {user.usergroups.map((group) => (
                <li key={group.id}>
                  <UsergroupChip group={group} />
                </li>
              ))}
            </ul>
          )}
        </div>
      </header>

      <section className={styles.bio}>
        {user.bio ? <p>{user.bio}</p> : <p className={styles.empty}>No description yet.</p>}
      </section>

      <dl className={styles.stats}>
        <div>
          <dt>Reputation</dt>
          <dd>{user.reputation.toLocaleString("en")}</dd>
        </div>
        <div>
          <dt>Joined</dt>
          <dd>
            <time dateTime={user.joined} title={fullFormat.format(joined)}>
              {dateFormat.format(joined)}
            </time>
          </dd>
        </div>
        <div>
          <dt>Last seen</dt>
          <dd>
            {online ? (
              "Now"
            ) : (
              <time dateTime={user.last_seen} title={fullFormat.format(lastSeen)}>
                {timeAgo(lastSeen)}
              </time>
            )}
          </dd>
        </div>
      </dl>
    </article>
  );
}

export function ProfileSkeleton() {
  return (
    <div className={styles.profile} aria-busy aria-label="Loading profile">
      <div className={styles.header}>
        <div className={`${styles.avatar} ${styles.skeleton}`} />
        <div className={styles.identity}>
          <div className={`${styles.skeleton} ${styles.skeletonLine}`} />
          <div className={`${styles.skeleton} ${styles.skeletonLine} ${styles.short}`} />
        </div>
      </div>
    </div>
  );
}
