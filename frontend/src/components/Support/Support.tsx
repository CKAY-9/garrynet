import Form from "next/form";
import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { Markdown } from "@/components/Markdown";
import type { SupportViewer } from "@/lib/support";
import {
  faqCategoryLabels,
  isDone,
  statusLabels,
  supportStatuses,
  ticketActivities,
  ticketActivityLabels,
  ticketCategories,
  ticketCategoryLabels,
  ticketSortLabels,
  ticketSorts,
} from "@/lib/supportEnums";
import { fullFormat, timeAgo } from "@/lib/time";
import type {
  FaqWithPeople,
  SupportMessageWithAuthor,
  SupportStatus,
  SupportTicket,
  TicketFilters,
  TicketPage,
  TicketSummary,
  UserRef,
} from "@/lib/types";
import { FaqBody } from "./FaqEditor";
import { MessageBody, ReplyForm, StatusMenu } from "./TicketTools";
import styles from "./Support.module.scss";

type Crumb = { href?: string; label: string };

export function Breadcrumbs({ items }: { items: Crumb[] }) {
  return (
    <nav aria-label="Breadcrumb">
      <ol className={styles.crumbs}>
        {items.map((c, i) => (
          <li key={i}>{c.href ? <Link href={c.href}>{c.label}</Link> : c.label}</li>
        ))}
      </ol>
    </nav>
  );
}

/** "3 hours ago", with the full date on hover. */
export function When({ iso }: { iso: string }) {
  const date = new Date(iso);
  return (
    <time dateTime={iso} title={fullFormat.format(date)}>
      {timeAgo(date)}
    </time>
  );
}

/**
 * Tabs between the user's own tickets, every ticket (Support Team only) and the FAQs.
 * `active` is the tab for the current page.
 */
export function SupportNav({ support, active }: { support: boolean; active: "mine" | "admin" | "faq" }) {
  const tabs = [
    { key: "mine", href: "/support", label: "My tickets" },
    ...(support ? [{ key: "admin", href: "/support/admin", label: "All tickets" }] : []),
    { key: "faq", href: "/support/faq", label: "FAQs" },
  ];
  return (
    <nav aria-label="Support">
      <ul className={styles.tabs}>
        {tabs.map((tab) => (
          <li key={tab.key}>
            <Link
              className={`${styles.tab} ${tab.key === active ? styles.activeTab : ""}`}
              href={tab.href}
              aria-current={tab.key === active ? "page" : undefined}
            >
              {tab.label}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}

export function StatusBadge({ status }: { status: SupportStatus }) {
  return <span className={`${styles.badge} ${styles[status.toLowerCase()]}`}>{statusLabels[status]}</span>;
}

function CategoryBadge({ label }: { label: string }) {
  return <span className={`${styles.badge} ${styles.category}`}>{label}</span>;
}

/** Tickets as rows linking to their pages. `showAuthor` for the Support Team's list. */
export function TicketRows({
  tickets,
  empty,
  showAuthor = false,
}: {
  tickets: TicketSummary[];
  empty: string;
  showAuthor?: boolean;
}) {
  return (
    <ul className={styles.rows}>
      {tickets.map((t) => (
        <li key={t.id}>
          <Link href={`/support/ticket/${t.id}`} className={styles.row}>
            <span className={styles.rowMain}>
              <span className={styles.rowTitle}>
                <span className={styles.id}>#{t.id}</span>
                {t.title}
                <StatusBadge status={t.status} />
                <CategoryBadge label={ticketCategoryLabels[t.category]} />
              </span>
              <span className={styles.rowMeta}>{t.subject}</span>
              <span className={styles.rowMeta}>
                {showAuthor && <>by {t.author.username} · </>}
                opened <When iso={t.created_at} />
              </span>
            </span>
            <span className={styles.counts}>
              <span>
                <strong>{t.message_count.toLocaleString("en")}</strong>
                {t.message_count === 1 ? "message" : "messages"}
              </span>
              <span>
                last activity
                <br />
                <When iso={t.last_updated} />
              </span>
            </span>
          </Link>
        </li>
      ))}
      {tickets.length === 0 && <li className={styles.empty}>{empty}</li>}
    </ul>
  );
}

/**
 * Status, category, activity and sort filters for a ticket list, submitted as a GET form
 * to `action` so the filters live in the URL. Blank fields are ignored by the backend.
 */
export function TicketFilterForm({ filters, action }: { filters: TicketFilters; action: string }) {
  return (
    // Keyed by the filters so the fields reset to the URL's values after each submit.
    <Form key={JSON.stringify(filters)} action={action} className={styles.filters} role="search">
      <label className={styles.field}>
        <span className={styles.label}>Status</span>
        <select className={styles.input} name="status" defaultValue={filters.status ?? ""}>
          <option value="">Any status</option>
          {supportStatuses.map((s) => (
            <option key={s} value={s}>
              {statusLabels[s]}
            </option>
          ))}
        </select>
      </label>
      <label className={styles.field}>
        <span className={styles.label}>Category</span>
        <select className={styles.input} name="category" defaultValue={filters.category ?? ""}>
          <option value="">Any category</option>
          {ticketCategories.map((c) => (
            <option key={c} value={c}>
              {ticketCategoryLabels[c]}
            </option>
          ))}
        </select>
      </label>
      <label className={styles.field}>
        <span className={styles.label}>Latest activity</span>
        <select className={styles.input} name="activity" defaultValue={filters.activity ?? ""}>
          <option value="">Any time</option>
          {ticketActivities.map((a) => (
            <option key={a} value={a}>
              {ticketActivityLabels[a]}
            </option>
          ))}
        </select>
      </label>
      <label className={styles.field}>
        <span className={styles.label}>Sort by</span>
        <select className={styles.input} name="sort" defaultValue={filters.sort ?? ""}>
          {ticketSorts.map((s) => (
            <option key={s} value={s}>
              {ticketSortLabels[s]}
            </option>
          ))}
        </select>
      </label>
      <div className={styles.actions}>
        <button className={styles.primary} type="submit">
          Apply
        </button>
        <Link className={styles.clear} href={action}>
          Clear
        </Link>
      </div>
    </Form>
  );
}

/** Overlapping avatars of everyone in `users` (duplicates removed). */
function People({ users, label }: { users: UserRef[]; label: string }) {
  const unique = users.filter((u, i) => users.findIndex((o) => o.id === u.id) === i);
  return (
    <ul className={styles.people} aria-label={label}>
      {unique.map((u) => (
        <li key={u.id} title={u.username}>
          <Avatar name={u.username} url={u.avatar_url || null} size={22} shape="circle" />
        </li>
      ))}
    </ul>
  );
}

/** FAQ cards linking to their pages. */
export function FaqGrid({ faqs, empty }: { faqs: FaqWithPeople[]; empty: string }) {
  if (faqs.length === 0) return <p className={styles.notice}>{empty}</p>;
  return (
    <div className={styles.grid}>
      {faqs.map((f) => (
        <Link key={f.id} href={`/support/faq/${f.id}`} className={styles.card}>
          <CategoryBadge label={faqCategoryLabels[f.category]} />
          <h3>{f.title}</h3>
          <p>{f.subject}</p>
          <span className={styles.cardFoot}>
            <span>
              Updated <When iso={f.last_updated} />
            </span>
            <People users={[f.creator, ...f.editor_users]} label="Written and edited by" />
          </span>
        </Link>
      ))}
    </div>
  );
}

function UserList({ users }: { users: UserRef[] }) {
  return (
    <ul className={styles.userList}>
      {users.map((u) => (
        <li key={u.id}>
          <Link href={`/users/${u.id}`}>
            <Avatar name={u.username} url={u.avatar_url || null} size={24} shape="circle" />
            {u.username}
          </Link>
        </li>
      ))}
    </ul>
  );
}

/** /support/ticket/{id}: the ticket's messages, with its details and participants beside them. */
export function TicketView({ page, viewer }: { page: TicketPage; viewer: SupportViewer }) {
  const { ticket, author, associated_users, responders, closed_by_user, messages, support } = page;
  const done = isDone(ticket.status);
  return (
    <div className={styles.page}>
      <Breadcrumbs
        items={[
          { href: "/support", label: "Support" },
          ...(support && ticket.user_id !== viewer.userId ? [{ href: "/support/admin", label: "All tickets" }] : []),
          { label: `Ticket #${ticket.id}` },
        ]}
      />
      <header className={styles.header}>
        <h1>
          {ticket.title}
          <StatusBadge status={ticket.status} />
        </h1>
        <p>{ticket.subject}</p>
      </header>
      {support && <StatusMenu ticket={ticket} />}
      <div className={styles.layout}>
        <div className={styles.thread}>
          {messages.map((m) => (
            <Message key={m.id} message={m} ticket={ticket} viewer={viewer} support={support} />
          ))}
          {done && !support ? (
            <p className={styles.notice}>
              This ticket is {statusLabels[ticket.status].toLowerCase()}. Only the support team can continue it.
            </p>
          ) : (
            <ReplyForm ticketId={ticket.id} support={support} />
          )}
        </div>
        <aside className={styles.aside} aria-label="Ticket details">
          <section>
            <h2>Details</h2>
            <dl>
              <div>
                <dt>Category</dt>
                <dd>{ticketCategoryLabels[ticket.category]}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd>
                  <StatusBadge status={ticket.status} />
                </dd>
              </div>
              <div>
                <dt>Opened</dt>
                <dd>
                  <When iso={ticket.created_at} />
                </dd>
              </div>
              <div>
                <dt>Last activity</dt>
                <dd>
                  <When iso={ticket.last_updated} />
                </dd>
              </div>
              {ticket.resolved_at && (
                <div>
                  <dt>Resolved</dt>
                  <dd>
                    <When iso={ticket.resolved_at} />
                    {closed_by_user && ` by ${closed_by_user.username}`}
                  </dd>
                </div>
              )}
              {ticket.closed_at && (
                <div>
                  <dt>Closed</dt>
                  <dd>
                    <When iso={ticket.closed_at} /> {closed_by_user ? `by ${closed_by_user.username}` : "automatically"}
                  </dd>
                </div>
              )}
            </dl>
          </section>
          <section>
            <h2>Opened by</h2>
            <UserList users={[author]} />
          </section>
          {associated_users.length > 0 && (
            <section>
              <h2>Involved users</h2>
              <UserList users={associated_users} />
            </section>
          )}
          <section>
            <h2>Responders</h2>
            {responders.length > 0 ? (
              <UserList users={responders} />
            ) : (
              <p className={styles.hint}>No one from the support team yet.</p>
            )}
          </section>
        </aside>
      </div>
    </div>
  );
}

function Message({
  message,
  ticket,
  viewer,
  support,
}: {
  message: SupportMessageWithAuthor;
  ticket: SupportTicket;
  viewer: SupportViewer;
  support: boolean;
}) {
  // Anyone but the ticket's owner writing in it is on the support team.
  const staff = message.author_id !== ticket.user_id;
  const edited = message.previous_content !== null;
  // Mirrors the backend: authors edit their own messages once; owners not once the ticket is done.
  const canEdit = viewer.userId === message.author_id && !edited && (support || !isDone(ticket.status));
  return (
    <article
      id={`message-${message.id}`}
      className={`${styles.message} ${staff ? styles.staff : ""} ${message.is_internal ? styles.internal : ""}`}
    >
      <div className={styles.author}>
        <Avatar name={message.author.username} url={message.author.avatar_url || null} size={56} shape="circle" />
        <Link href={`/users/${message.author.id}`}>{message.author.username}</Link>
        {staff && <span className={styles.badge}>Support</span>}
      </div>
      <div className={styles.body}>
        <div className={styles.messageMeta}>
          {message.is_internal && <span className={`${styles.badge} ${styles.internalBadge}`}>Internal note</span>}
          <span>
            Sent <When iso={message.created_at} />
          </span>
          {edited && (
            <span>
              · edited <When iso={message.last_updated} />
            </span>
          )}
        </div>
        <MessageBody message={message} canEdit={canEdit}>
          <Markdown className={styles.content}>{message.content}</Markdown>
          {message.previous_content !== null && (
            <details className={styles.previous}>
              <summary>Previous version</summary>
              <Markdown>{message.previous_content}</Markdown>
            </details>
          )}
        </MessageBody>
      </div>
    </article>
  );
}

/** /support/faq/{id}: the FAQ in full, with editing tools for the Support Team. */
export function FaqView({ faq, support }: { faq: FaqWithPeople; support: boolean }) {
  return (
    <div className={styles.page}>
      <Breadcrumbs items={[{ href: "/support", label: "Support" }, { href: "/support/faq", label: "FAQs" }, { label: faq.title }]} />
      <header className={styles.header}>
        <h1>
          {faq.title}
          <CategoryBadge label={faqCategoryLabels[faq.category]} />
        </h1>
        <p>{faq.subject}</p>
      </header>
      <div className={styles.byline}>
        <People users={[faq.creator, ...faq.editor_users]} label="Written and edited by" />
        <span>
          Written by <Link href={`/users/${faq.creator.id}`}>{faq.creator.username}</Link> <When iso={faq.created_at} />
        </span>
        {faq.editor_users.length > 0 && (
          <span>
            · edited by {faq.editor_users.map((u) => u.username).join(", ")} · updated <When iso={faq.last_updated} />
          </span>
        )}
      </div>
      <FaqBody faq={faq} canManage={support}>
        <article className={styles.article}>
          <Markdown>{faq.description}</Markdown>
        </article>
      </FaqBody>
    </div>
  );
}
