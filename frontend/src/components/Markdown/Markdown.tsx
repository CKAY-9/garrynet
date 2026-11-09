import ReactMarkdown, { type Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";
import styles from "./Markdown.module.scss";

/** Links leaving the site open in a new tab and pass no ranking credit to user content. */
const components: Components = {
  // `node` is react-markdown's syntax-tree node; pulled out so it isn't passed to the DOM.
  a({ node, href, ...props }) {
    void node;
    const external = href !== undefined && /^[a-z][a-z0-9+.-]*:/i.test(href);
    return external ? (
      <a href={href} target="_blank" rel="noopener noreferrer nofollow ugc" {...props} />
    ) : (
      <a href={href} {...props} />
    );
  },
  img({ node, alt, ...props }) {
    void node;
    // User images come from arbitrary hosts, so next/image's optimizer can't be used.
    // eslint-disable-next-line @next/next/no-img-element
    return <img alt={alt ?? ""} loading="lazy" {...props} />;
  },
};

/**
 * Renders user-written Markdown (GitHub-flavored; single newlines break lines). The
 * backend escapes raw HTML and rejects unsafe link schemes before storing it; as a second
 * layer, raw HTML is skipped here and react-markdown drops unsafe URLs.
 */
export function Markdown({ children, className }: { children: string; className?: string }) {
  return (
    <div className={className ? `${styles.markdown} ${className}` : styles.markdown}>
      <ReactMarkdown remarkPlugins={[remarkGfm, remarkBreaks]} components={components} skipHtml>
        {children}
      </ReactMarkdown>
    </div>
  );
}
