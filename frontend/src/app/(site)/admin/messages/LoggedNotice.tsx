import styles from "./messages.module.scss";

/** Every staff read of private messages is logged on the server (docs/PRIVATE_MESSAGES.md#admins). */
export function LoggedNotice() {
  return (
    <p className={styles.logged} role="note">
      <strong>Views are logged.</strong> Opening a user&apos;s chats or a chat records who looked and at whose messages.
      Read private messages only when there is a reason to.
    </p>
  );
}
