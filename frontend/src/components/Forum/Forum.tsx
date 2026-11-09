import type { CSSProperties, ReactNode } from "react";
import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { Markdown } from "@/components/Markdown";
import { StatCard } from "@/components/StatCard";
import { steamLoginUrl } from "@/lib/config";
import type { PostingAccess } from "@/lib/forum";
import { fullFormat, timeAgo } from "@/lib/time";
import type {
  ForumCategory,
  ForumCategoryPage as CategoryData,
  ForumHome as HomeData,
  ForumThread,
  ForumThreadPage as ThreadData,
} from "@/lib/types";
import { NewThreadForm, ReplyForm } from "./Composer";
import styles from "./Forum.module.scss";

type Crumb = { href?: string; label: string };

function Breadcrumbs({ items }: { items: Crumb[] }) {
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
function When({ iso }: { iso: string }) {
  const date = new Date(iso);
  return (
    <time dateTime={iso} title={fullFormat.format(date)}>
      {timeAgo(date)}
    </time>
  );
}

const accent = (color: string) => ({ "--accent": color }) as CSSProperties;

/** /forum: every section with its categories, plus forum statistics. */
export function ForumHome({ sections, stats }: HomeData) {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1>Forum</h1>
        <p>Talk with the rest of the community.</p>
      </header>
      <dl className={styles.stats}>
        <StatCard label="Threads" value={stats.threads} />
        <StatCard label="Posts" value={stats.posts} />
        <StatCard label="Members" value={stats.members} />
        <StatCard label="Online now" value={stats.online} />
      </dl>
      {sections.length === 0 && <p className={styles.notice}>The forum has no sections yet.</p>}
      {sections.map((section) => (
        <section
          key={section.id}
          className={styles.section}
          style={accent(section.accent)}
          aria-labelledby={`section-${section.id}`}
        >
          <div className={styles.sectionHeader}>
            {section.icon_url && <Avatar name={section.name} url={section.icon_url} size={32} />}
            <div>
              <h2 id={`section-${section.id}`}>{section.name}</h2>
              {section.description && <p>{section.description}</p>}
            </div>
          </div>
          <ul className={styles.rows}>
            {section.categories.map((c) => (
              <li key={c.id}>
                <Link href={`/forum/${c.id}`} className={styles.row}>
                  <span className={styles.rowMain}>
                    <span className={styles.rowTitle}>
                      {c.name}
                      {c.locked && <span className={styles.badge}>Locked</span>}
                    </span>
                    {c.description && <span className={styles.rowMeta}>{c.description}</span>}
                    {c.latest && (
                      <span className={styles.latest}>
                        Latest: {c.latest.title} · {c.latest.author.username} · <When iso={c.latest.last_activity} />
                      </span>
                    )}
                  </span>
                  <span className={styles.counts}>
                    <span>
                      <strong>{c.thread_count.toLocaleString("en")}</strong>threads
                    </span>
                    <span>
                      <strong>{c.post_count.toLocaleString("en")}</strong>posts
                    </span>
                  </span>
                </Link>
              </li>
            ))}
            {section.categories.length === 0 && <li className={styles.empty}>No categories yet.</li>}
          </ul>
        </section>
      ))}
    </div>
  );
}

/** /forum/{category_id}: the category's threads, stickies first. */
export function ForumCategoryPage({ category, section, threads, access }: CategoryData & { access: PostingAccess }) {
  return (
    <div className={styles.page}>
      <Breadcrumbs items={[{ href: "/forum", label: "Forum" }, { label: section.name }, { label: category.name }]} />
      <div className={styles.toolbar}>
        <header className={styles.header}>
          <h1>
            {category.name}
            {category.locked && <span className={styles.badge}>Locked</span>}
          </h1>
          {category.description && <p>{category.description}</p>}
        </header>
        {!category.locked && access === "allowed" && <NewThreadForm categoryId={category.id} />}
      </div>
      {category.locked ? (
        <p className={styles.notice}>This category is locked. New threads can&apos;t be created.</p>
      ) : (
        <PostingNotice access={access}>to start a thread</PostingNotice>
      )}
      <section className={styles.section} style={accent(section.accent)} aria-label="Threads">
        <ul className={styles.rows}>
          {threads.map((t) => (
            <li key={t.id}>
              <Link href={`/forum/${category.id}/${t.id}`} className={styles.row}>
                <span className={styles.rowMain}>
                  <span className={styles.rowTitle}>
                    {t.title}
                    <ThreadBadges thread={t} />
                  </span>
                  <span className={styles.rowMeta}>
                    by {t.author.username} · <When iso={t.created_at} />
                  </span>
                </span>
                <span className={styles.counts}>
                  <span>
                    <strong>{t.replies.toLocaleString("en")}</strong>
                    {t.replies === 1 ? "reply" : "replies"}
                  </span>
                  <span>
                    last activity
                    <br />
                    <When iso={t.last_activity} />
                  </span>
                </span>
              </Link>
            </li>
          ))}
          {threads.length === 0 && <li className={styles.empty}>No threads yet.</li>}
        </ul>
      </section>
    </div>
  );
}

function ThreadBadges({ thread }: { thread: ForumThread }) {
  return (
    <>
      {thread.sticky && <span className={`${styles.badge} ${styles.sticky}`}>Sticky</span>}
      {thread.locked && <span className={styles.badge}>Locked</span>}
    </>
  );
}

/** /forum/{category_id}/{thread_id}: every post, the thread's first post at the top. */
export function ForumThreadPage({ category, section, thread, posts, access }: ThreadData & { access: PostingAccess }) {
  return (
    <div className={styles.page}>
      <Breadcrumbs
        items={[
          { href: "/forum", label: "Forum" },
          { label: section.name },
          { href: `/forum/${category.id}`, label: category.name },
          { label: thread.title },
        ]}
      />
      <header className={styles.header}>
        <h1>
          {thread.title}
          <ThreadBadges thread={thread} />
        </h1>
      </header>
      {posts.map((post) => (
        <article
          key={post.id}
          id={`post-${post.id}`}
          className={`${styles.post} ${post.id === thread.thread_post_id ? styles.principal : ""}`}
        >
          <div className={styles.author}>
            <Avatar name={post.author.username} url={post.author.avatar_url || null} size={56} shape="circle" />
            <Link href={`/users/${post.author.id}`}>{post.author.username}</Link>
          </div>
          <div className={styles.body}>
            <div className={styles.postMeta}>
              Posted <When iso={post.created_at} />
              {post.edited && (
                <>
                  {" "}
                  · edited <When iso={post.edited} />
                </>
              )}
            </div>
            <Markdown className={styles.content}>{post.content}</Markdown>
            <div className={styles.reactions} aria-label="Reactions">
              <span title="Likes">👍 {post.likes.length}</span>
              <span title="Dislikes">👎 {post.dislikes.length}</span>
            </div>
          </div>
        </article>
      ))}
      <ReplyArea category={category} thread={thread} access={access} />
    </div>
  );
}

function ReplyArea({ category, thread, access }: { category: ForumCategory; thread: ForumThread; access: PostingAccess }) {
  if (thread.locked) return <p className={styles.notice}>This thread is locked. No further replies can be made.</p>;
  if (category.locked) return <p className={styles.notice}>This category is locked. No further replies can be made.</p>;
  if (access !== "allowed") return <PostingNotice access={access}>to reply</PostingNotice>;
  return <ReplyForm threadId={thread.id} />;
}

/** Why the user can't post, if they can't. `children` completes "Log in with Steam ...". */
function PostingNotice({ access, children }: { access: PostingAccess; children: ReactNode }) {
  if (access === "guest") {
    return (
      <p className={styles.notice}>
        {/* Plain anchor: a full navigation to the backend, which redirects to Steam. */}
        <a href={steamLoginUrl}>Log in with Steam</a> {children}.
      </p>
    );
  }
  if (access === "forbidden") return <p className={styles.notice}>You don&apos;t have permission to post on the forum.</p>;
  return null;
}
