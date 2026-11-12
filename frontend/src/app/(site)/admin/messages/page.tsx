import { Suspense } from "react";
import { requirePermission } from "@/lib/auth";
import { READ_ANY_MESSAGES } from "@/lib/messageEnums";
import styles from "../admin.module.scss";
import { LoggedNotice } from "./LoggedNotice";
import page from "./messages.module.scss";
import { UserSearch } from "./UserSearch";

export default function AdminMessagesPage() {
  return (
    // The permission check reads the session cookie, so it runs behind the boundary.
    <Suspense>
      <AdminMessages />
    </Suspense>
  );
}

/** /admin/messages: pick a user to read their chats. */
async function AdminMessages() {
  await requirePermission(READ_ANY_MESSAGES);
  return (
    <section className={styles.section} aria-labelledby="messages-heading">
      <h2 id="messages-heading" className={styles.sectionTitle}>
        Private messages
      </h2>
      <p className={styles.note}>
        Read any user&apos;s chats, including ones they deleted and temporary messages past 24 hours. This is read-only.
      </p>
      <LoggedNotice />
      <div className={page.search}>
        <UserSearch />
      </div>
    </section>
  );
}
