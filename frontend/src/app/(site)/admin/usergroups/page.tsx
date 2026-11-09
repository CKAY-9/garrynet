import { Suspense, type CSSProperties } from "react";
import { UsergroupChip } from "@/components/UsergroupChip";
import { requirePermission } from "@/lib/auth";
import { Permission, PERMISSION_LABELS, hasPermission } from "@/lib/permissions";
import { fullFormat, timeAgo } from "@/lib/time";
import { getAdminUsergroups } from "@/lib/users";
import styles from "../admin.module.scss";
import list from "./usergroups.module.scss";

export default function UsergroupsPage() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <Usergroups />
    </Suspense>
  );
}

async function Usergroups() {
  await requirePermission(Permission.AdminAccess | Permission.UsergroupManagement);
  const groups = await getAdminUsergroups();

  return (
    <section className={styles.section} aria-labelledby="usergroups-heading">
      <h2 id="usergroups-heading" className={styles.sectionTitle}>
        Usergroups
      </h2>
      {/* TODO: create, edit and remove usergroups. */}
      {groups.length === 0 ? (
        <p className={styles.placeholder}>No usergroups yet.</p>
      ) : (
        <ul className={list.list}>
          {groups.map((group) => {
            const granted = PERMISSION_LABELS.filter(({ flag }) => hasPermission(group.permissions, flag));
            return (
              <li key={group.id} className={list.card} style={{ "--group-color": group.color } as CSSProperties}>
                <div className={list.head}>
                  <UsergroupChip group={group} />
                  <span className={list.id}>#{group.id}</span>
                  <span className={list.meta}>
                    {group.members.length} {group.members.length === 1 ? "member" : "members"} · Created{" "}
                    <time dateTime={group.created_at} title={fullFormat.format(new Date(group.created_at))}>
                      {timeAgo(new Date(group.created_at))}
                    </time>
                    {group.last_updated && (
                      <>
                        {" "}
                        · Updated{" "}
                        <time dateTime={group.last_updated} title={fullFormat.format(new Date(group.last_updated))}>
                          {timeAgo(new Date(group.last_updated))}
                        </time>
                      </>
                    )}
                  </span>
                </div>
                {granted.length === 0 ? (
                  <span className={list.none}>No permissions</span>
                ) : (
                  <ul className={list.perms} aria-label={`${group.name} permissions`}>
                    {granted.map(({ flag, label }) => (
                      <li key={flag} className={list.perm}>
                        {label}
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
