import type { ReactNode } from "react";
import Form from "next/form";
import Link from "next/link";
import { fullFormat, timeAgo } from "@/lib/time";
import type { Authored, AuthoredFilters, AuthoredKind, AuthoredPage, AuthoredSort } from "@/lib/types";
import styles from "./Activity.module.scss";

const kindOptions: [AuthoredKind, string][] = [
  ["thread", "Threads"],
  ["post", "Posts"],
];

const sortOptions: [AuthoredSort, string][] = [
  ["old", "Oldest"],
  ["reputation+", "Most reputation"],
  ["reputation-", "Least reputation"],
];

/** Characters the backend sends of each post; an excerpt this long was cut short. */
const EXCERPT_CHARS = 300;

type Props = {
  userId: number;
  /** Changes the empty-state wording. */
  self: boolean;
  filters: AuthoredFilters;
  /** From 1. */
  page: number;
  data: AuthoredPage;
};

/**
 * A user's threads and posts on their profile, with filters (kind, title or content, sort)
 * submitted as a GET form so they live in the URL, and pages linked below.
 */
export function Activity({ userId, self, filters, page, data }: Props) {
  const filtered = Object.values(filters).some((v) => v.trim());

  return (
    <ActivityFrame>
      {/* Keyed by the filters so the fields reset to the URL's values after each search. */}
      <Form
        key={JSON.stringify(filters)}
        action={`/users/${userId}`}
        scroll={false}
        className={styles.filters}
        role="search"
        aria-label="Filter threads and posts"
      >
        <label className={styles.field}>
          <span>Show</span>
          <select name="kind" defaultValue={filters.kind}>
            <option value="">Threads and posts</option>
            {kindOptions.map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <label className={`${styles.field} ${styles.wide}`}>
          <span>Search</span>
          <input type="search" name="q" defaultValue={filters.q} placeholder="Title or content" maxLength={100} autoComplete="off" />
        </label>
        <label className={styles.field}>
          <span>Sort by</span>
          <select name="sort" defaultValue={filters.sort}>
            <option value="">Newest</option>
            {sortOptions.map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <div className={styles.actions}>
          <button className={styles.apply} type="submit">
            Apply
          </button>
          {filtered && (
            <Link className={styles.clear} href={`/users/${userId}#activity`} scroll={false}>
              Clear
            </Link>
          )}
        </div>
      </Form>

      {data.items.length === 0 ? (
        <p className={styles.empty}>
          {page > 1 ? (
            <>
              Nothing more here. <Link href={activityHref(userId, filters, 1)}>Back to the first page</Link>
            </>
          ) : filtered ? (
            "Nothing matches these filters."
          ) : self ? (
            "You haven't posted on the forums yet."
          ) : (
            "They haven't posted on the forums yet."
          )}
        </p>
      ) : (
        <ol className={styles.list}>
          {data.items.map((item) => (
            <AuthoredPreview key={item.post_id} item={item} />
          ))}
        </ol>
      )}

      {(page > 1 || data.has_more) && (
        <nav className={styles.pages} aria-label="Pages">
          {page > 1 ? (
            <Link className={styles.pageLink} href={activityHref(userId, filters, page - 1)} rel="prev">
              Previous
            </Link>
          ) : (
            <span />
          )}
          <span className={styles.pageNumber}>Page {page}</span>
          {data.has_more ? (
            <Link className={styles.pageLink} href={activityHref(userId, filters, page + 1)} rel="next">
              Next
            </Link>
          ) : (
            <span />
          )}
        </nav>
      )}
    </ActivityFrame>
  );
}

/** The section's heading around its contents, shared with the skeleton. */
function ActivityFrame({ children }: { children: ReactNode }) {
  return (
    <section id="activity" className={styles.activity} aria-labelledby="activity-heading">
      <h2 id="activity-heading" className={styles.heading}>
        Threads and posts
      </h2>
      {children}
    </section>
  );
}

export function ActivitySkeleton() {
  return (
    <ActivityFrame>
      <div className={`${styles.filters} ${styles.skeleton}`} aria-busy aria-label="Loading threads and posts" />
    </ActivityFrame>
  );
}

function AuthoredPreview({ item }: { item: Authored }) {
  const created = new Date(item.created_at);
  const threadHref = `/forum/${item.category_id}/${item.thread_id}`;
  const text = plainText(item.excerpt);
  const cut = item.excerpt.length >= EXCERPT_CHARS;

  return (
    <li className={styles.item}>
      <div className={styles.itemHeader}>
        <span className={item.is_thread ? styles.threadKind : styles.postKind}>{item.is_thread ? "Thread" : "Post"}</span>
        <Link className={styles.title} href={item.is_thread ? threadHref : `${threadHref}#post-${item.post_id}`}>
          {item.is_thread ? item.thread_title : `Re: ${item.thread_title}`}
        </Link>
      </div>
      {text && (
        <p className={styles.excerpt}>
          {text}
          {cut && "…"}
        </p>
      )}
      <div className={styles.meta}>
        <span>
          in <Link href={`/forum/${item.category_id}`}>{item.category_name}</Link>
        </span>
        <time dateTime={item.created_at} title={fullFormat.format(created)}>
          {timeAgo(created)}
        </time>
        <span className={item.reputation > 0 ? styles.positive : item.reputation < 0 ? styles.negative : undefined}>
          {item.reputation > 0 ? `+${item.reputation}` : item.reputation} reputation
        </span>
      </div>
    </li>
  );
}

/** Link to page `page` of the activity with `filters`, scrolled to the section. */
function activityHref(userId: number, filters: AuthoredFilters, page: number): string {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(filters)) {
    if (value.trim()) params.set(key, value.trim());
  }
  if (page > 1) params.set("page", String(page));
  const query = params.size > 0 ? `?${params}` : "";
  return `/users/${userId}${query}#activity`;
}

/**
 * Rough plain text of a Markdown excerpt for a one-glance preview: drops images, code fences
 * and block markers (headings, quotes, lists), keeps link text, and collapses whitespace.
 */
function plainText(markdown: string): string {
  return markdown
    .replace(/^\s*(```|~~~).*$/gm, " ")
    .replace(/!\[[^\]]*\]\([^)]*\)?/g, " ")
    .replace(/\[([^\]]*)\]\([^)]*\)?/g, "$1")
    .replace(/^\s{0,3}(#{1,6}|>+|[-*+]|\d+[.)])\s+/gm, "")
    .replace(/[*`]|~~/g, "")
    .replace(/\s+/g, " ")
    .trim();
}
