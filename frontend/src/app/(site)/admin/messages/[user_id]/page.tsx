import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { ChatList } from "@/components/Messages";
import { getAdminChats } from "@/lib/adminMessages";
import { requirePermission } from "@/lib/auth";
import { adminChatsHref, READ_ANY_MESSAGES } from "@/lib/messageEnums";
import styles from "../../admin.module.scss";
import servers from "../../servers/servers.module.scss";
import { LoggedNotice } from "../LoggedNotice";
import page from "../messages.module.scss";

export default function AdminUserChatsPage({ params }: PageProps<"/admin/messages/[user_id]">) {
  return (
    // Params, the permission check and the chats are request-time, so they stream in behind the boundary.
    <Suspense fallback={<div className={page.skeleton} aria-busy aria-label="Loading chats" />}>
      {params.then(({ user_id }) => (
        <UserChats id={user_id} />
      ))}
    </Suspense>
  );
}

/**
 * /admin/messages/{user_id}: every chat of that user, read-only, including ones they deleted
 * and ones whose messages expired. 404 for an unknown user. The backend logs the read.
 */
async function UserChats({ id }: { id: string }) {
  await requirePermission(READ_ANY_MESSAGES);
  const chats = await getAdminChats(id);
  if (!chats) notFound();
  const { user } = chats;

  return (
    <>
      <Link href="/admin/messages" className={servers.back}>
        ← Pick another user
      </Link>

      <section className={styles.section} aria-labelledby="chats-heading">
        <div className={servers.titleRow}>
          <h2 id="chats-heading" className={styles.sectionTitle}>
            <Link href={`/users/${user.id}`}>{user.username}</Link>&apos;s chats{" "}
            <span className={servers.id}>#{user.id}</span>
          </h2>
        </div>
        <p className={styles.note}>
          Includes chats {user.username} deleted and chats whose messages have expired. Unread counts are what{" "}
          {user.username} hasn&apos;t read.
        </p>
        <LoggedNotice />
        <ChatList
          key={user.id}
          initial={chats}
          viewerId={user.id}
          endpoint={`/api/admin/messages/${user.id}`}
          chatBase={adminChatsHref(user.id)}
          selfLabel={user.username}
          empty={`${user.username} has no chats.`}
        />
      </section>
    </>
  );
}
