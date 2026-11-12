import { Suspense } from "react";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Thread } from "@/components/Messages";
import { requirePermission } from "@/lib/auth";
import { MESSAGING } from "@/lib/messageEnums";
import { getThread, hasSentMessage } from "@/lib/messages";
import styles from "../messages.module.scss";

export const metadata: Metadata = {
  title: "Messages · GarryNet",
};

export default function ChatPage({ params }: PageProps<"/messages/[user_id]">) {
  return (
    // Params, the session and the messages are request-time, so they stream in behind the boundary.
    <Suspense fallback={<div className={styles.skeleton} aria-busy aria-label="Loading the chat" />}>
      {params.then(({ user_id }) => (
        <Chat id={user_id} />
      ))}
    </Suspense>
  );
}

/** /messages/{user_id}: the chat with that user. Needs User Messages; 404 for an unknown user or yourself. */
async function Chat({ id }: { id: string }) {
  const viewer = await requirePermission(MESSAGING);
  if (id === String(viewer.id)) notFound();
  // `has_sent` comes with the chat list, so ask for its smallest page.
  const [thread, hasSent] = await Promise.all([getThread(id), hasSentMessage()]);
  if (!thread) notFound();
  return <Thread key={thread.user.id} viewerId={viewer.id} initial={thread} hasSent={hasSent} />;
}
