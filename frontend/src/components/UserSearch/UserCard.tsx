import Image from "next/image";
import Link from "next/link";
import { UsergroupChip } from "@/components/UsergroupChip";
import { fullFormat, timeAgo } from "@/lib/time";
import type { UserSearchResult, UserStatus } from "@/lib/types";
import styles from "./UserSearch.module.scss";

const statusLabels: Record<UserStatus, string> = {
  ONLINE: "Online",
  IDLE: "Idle",
  OFFLINE: "Offline",
};

/** Chips shown before collapsing the rest into "+N". */
const MAX_CHIPS = 3;

/** A search result: avatar, name, status and groups, linking to the user's profile. */
export function UserCard({ user }: { user: UserSearchResult }) {
  const lastSeen = new Date(user.last_seen);
  const shown = user.usergroups.slice(0, MAX_CHIPS);
  const hidden = user.usergroups.length - shown.length;

  return (
    <Link className={styles.card} href={`/users/${user.id}`}>
      <div className={styles.cardHeader}>
        <span className={`${styles.avatarWrap} ${styles[user.status.toLowerCase()]}`}>
          <Image className={styles.avatar} src={user.avatar_url} alt="" width={56} height={56} />
          <span className={styles.statusDot} title={statusLabels[user.status]} />
        </span>
        <div className={styles.identity}>
          <span className={styles.name}>{user.username}</span>
          <span className={styles.meta}>
            {statusLabels[user.status]} ·{" "}
            {user.status === "ONLINE" ? (
              "active now"
            ) : (
              <time dateTime={user.last_seen} title={fullFormat.format(lastSeen)}>
                seen {timeAgo(lastSeen)}
              </time>
            )}
          </span>
        </div>
      </div>

      {shown.length > 0 && (
        <ul className={styles.groups} aria-label="Usergroups">
          {shown.map((group) => (
            <li key={group.id}>
              <UsergroupChip group={group} />
            </li>
          ))}
          {hidden > 0 && <li className={styles.more}>+{hidden}</li>}
        </ul>
      )}

      <dl className={styles.stats}>
        <div>
          <dt>Reputation</dt>
          <dd>{user.reputation?.toLocaleString("en") ?? <Hidden />}</dd>
        </div>
        <div>
          <dt>Posts</dt>
          <dd>{user.post_count?.toLocaleString("en") ?? <Hidden />}</dd>
        </div>
        <div>
          <dt>ID</dt>
          <dd>#{user.id}</dd>
        </div>
      </dl>
    </Link>
  );
}

/** A private user's reputation or post count. */
function Hidden() {
  return (
    <span className={styles.hidden} title="This profile is private">
      Private
    </span>
  );
}

export function UserCardGridSkeleton({ count = 6 }: { count?: number }) {
  return (
    <div className={styles.grid} aria-busy aria-label="Loading users">
      {Array.from({ length: count }, (_, i) => (
        <div key={i} className={`${styles.card} ${styles.cardSkeleton}`} />
      ))}
    </div>
  );
}
