import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { UsergroupChip } from "@/components/UsergroupChip";
import { requirePermission } from "@/lib/auth";
import { Permission, hasPermission, isDefaultGroup } from "@/lib/permissions";
import { dateFormat, fullFormat, timeAgo } from "@/lib/time";
import { getAdminUsergroup, getUsergroupMembers } from "@/lib/users";
import styles from "../../admin.module.scss";
import servers from "../../servers/servers.module.scss";
import { DeleteUsergroup } from "./DeleteUsergroup";
import { UsergroupEditor } from "./UsergroupEditor";
import { UsergroupMembers } from "./UsergroupMembers";

export default function UsergroupPage({ params }: PageProps<"/admin/usergroups/[usergroup_id]">) {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      {params.then(({ usergroup_id }) => (
        <UsergroupSettings id={usergroup_id} />
      ))}
    </Suspense>
  );
}

async function UsergroupSettings({ id }: { id: string }) {
  const user = await requirePermission(Permission.AdminAccess | Permission.UsergroupManagement);
  const group = await getAdminUsergroup(id);
  if (!group) notFound();
  const members = await getUsergroupMembers(group.id);

  return (
    <>
      <Link href="/admin/usergroups" className={servers.back}>
        ← All usergroups
      </Link>

      <section className={styles.section} aria-labelledby="usergroup-heading">
        <div className={servers.titleRow}>
          <h2 id="usergroup-heading" className={styles.sectionTitle}>
            {group.name} <span className={servers.id}>#{group.id}</span>
          </h2>
          <UsergroupChip group={group} />
        </div>
        <dl className={servers.facts}>
          <div>
            <dt>Members</dt>
            <dd>{group.members.length}</dd>
          </div>
          <div>
            <dt>Created</dt>
            <dd>
              <time dateTime={group.created_at} title={fullFormat.format(new Date(group.created_at))}>
                {dateFormat.format(new Date(group.created_at))}
              </time>
            </dd>
          </div>
          <div>
            <dt>Last updated</dt>
            <dd>
              {group.last_updated ? (
                <time dateTime={group.last_updated} title={fullFormat.format(new Date(group.last_updated))}>
                  {timeAgo(new Date(group.last_updated))}
                </time>
              ) : (
                <span className={servers.muted}>Never</span>
              )}
            </dd>
          </div>
        </dl>
        <UsergroupEditor group={group} held={user.permissions} />
      </section>

      <section className={styles.section} aria-labelledby="members-heading">
        <h2 id="members-heading" className={styles.sectionTitle}>
          Members <span className={servers.id}>{members.length}</span>
        </h2>
        <UsergroupMembers
          groupId={group.id}
          groupName={group.name}
          members={members}
          viewerId={user.id}
          canEdit={hasPermission(user.permissions, group.permissions)}
        />
      </section>

      {!isDefaultGroup(group.name) && (
        <section className={styles.section} aria-labelledby="danger-heading">
          <h2 id="danger-heading" className={styles.sectionTitle}>
            Danger zone
          </h2>
          <DeleteUsergroup group={group} />
        </section>
      )}
    </>
  );
}
