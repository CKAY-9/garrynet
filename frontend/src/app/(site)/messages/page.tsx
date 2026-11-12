import { Suspense } from "react";
import type { Metadata } from "next";
import { ChatList, IntroDialog, NewMessage } from "@/components/Messages";
import { requirePermission } from "@/lib/auth";
import { MESSAGING } from "@/lib/messageEnums";
import { getChats } from "@/lib/messages";
import styles from "./messages.module.scss";

export const metadata: Metadata = {
  title: "Messages · GarryNet",
};

export default function MessagesPage() {
  return (
    // The session and chats are request-time, so they stream in behind the boundary.
    <Suspense fallback={<div className={styles.skeleton} aria-busy aria-label="Loading messages" />}>
      <Messages />
    </Suspense>
  );
}

/** /messages: the user's chats and the "New message" button. Needs User Messages. */
async function Messages() {
  const user = await requirePermission(MESSAGING);
  const page = await getChats();
  return (
    <div className={styles.page}>
      <div className={styles.toolbar}>
        <header className={styles.header}>
          <h1>Messages</h1>
          <p>Private chats with other members.</p>
        </header>
        <NewMessage viewerId={user.id} />
      </div>
      <IntroDialog hasSent={page.has_sent} />
      <ChatList initial={page} viewerId={user.id} />
    </div>
  );
}
