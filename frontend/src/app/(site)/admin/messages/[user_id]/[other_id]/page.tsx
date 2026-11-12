import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { AdminThread } from "@/components/Messages";
import { getAdminThread } from "@/lib/adminMessages";
import { requirePermission } from "@/lib/auth";
import { adminChatsHref, READ_ANY_MESSAGES } from "@/lib/messageEnums";
import styles from "../../../admin.module.scss";
import servers from "../../../servers/servers.module.scss";
import { LoggedNotice } from "../../LoggedNotice";
import page from "../../messages.module.scss";

export default function AdminChatPage({ params }: PageProps<"/admin/messages/[user_id]/[other_id]">) {
  return (
    // Params, the permission check and the messages are request-time, so they stream in behind the boundary.
    <Suspense fallback={<div className={page.skeleton} aria-busy aria-label="Loading the chat" />}>
      {params.then(({ user_id, other_id }) => (
        <Chat userId={user_id} otherId={other_id} />
      ))}
    </Suspense>
  );
}

/**
 * /admin/messages/{user_id}/{other_id}: the chat between two users, read-only, with deleted
 * and expired messages marked. 404 if either user doesn't exist (or they're the same user).
 * Viewing never marks anything read. The backend logs the read.
 */
async function Chat({ userId, otherId }: { userId: string; otherId: string }) {
  await requirePermission(READ_ANY_MESSAGES);
  if (userId === otherId) notFound();
  const thread = await getAdminThread(userId, otherId);
  if (!thread) notFound();
  const { user, other } = thread;

  return (
    <>
      <Link href={adminChatsHref(user.id)} className={servers.back}>
        ← {user.username}&apos;s chats
      </Link>

      <section className={styles.section} aria-labelledby="chat-heading">
        <h2 id="chat-heading" className={styles.sectionTitle}>
          Chat between {user.username} and {other.username}
        </h2>
        <LoggedNotice />
        <AdminThread key={`${user.id}-${other.id}`} initial={thread} />
      </section>
    </>
  );
}
