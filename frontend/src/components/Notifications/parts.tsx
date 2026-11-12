import { AccountIcon, ForumsIcon, MessagesIcon, StoreIcon, SupportIcon, SystemIcon } from "@/components/SideNav/icons";
import { notificationTypeLabels } from "@/lib/notificationEnums";
import { fullFormat, timeAgo } from "@/lib/time";
import type { NotificationType } from "@/lib/types";

/* Pieces shared by the bell's dropdown and the `/notifications` list. */

const typeIcons: Record<NotificationType, () => React.ReactElement> = {
  SUPPORT: SupportIcon,
  STORE: StoreIcon,
  FORUM: ForumsIcon,
  MESSAGE: MessagesIcon,
  ACCOUNT: AccountIcon,
  SYSTEM: SystemIcon,
};

/** The type's icon, labelled for screen readers. */
export function TypeIcon({ type, className }: { type: NotificationType; className?: string }) {
  const Icon = typeIcons[type];
  return (
    <span className={className} role="img" aria-label={notificationTypeLabels[type]} title={notificationTypeLabels[type]}>
      <Icon />
    </span>
  );
}

/**
 * "3 minutes ago", with the full date on hover. Rendered on the server and again on the
 * client, which may be a minute apart, so the text is allowed to differ on hydration.
 */
export function Ago({ iso }: { iso: string }) {
  const date = new Date(iso);
  return (
    <time dateTime={iso} title={fullFormat.format(date)} suppressHydrationWarning>
      {timeAgo(date)}
    </time>
  );
}
