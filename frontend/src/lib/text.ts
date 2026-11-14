/**
 * Rough plain text of Markdown for a one-glance preview: drops images, code fences and block
 * markers (headings, quotes, lists), keeps link text, and collapses whitespace.
 */
export function plainText(markdown: string): string {
  return markdown
    .replace(/^\s*(```|~~~).*$/gm, " ")
    .replace(/!\[[^\]]*\]\([^)]*\)?/g, " ")
    .replace(/\[([^\]]*)\]\([^)]*\)?/g, "$1")
    .replace(/^\s{0,3}(#{1,6}|>+|[-*+]|\d+[.)])\s+/gm, "")
    .replace(/[*`]|~~/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

/** `text` on one line, cut to at most `max` characters with an ellipsis. */
export function truncate(text: string, max: number): string {
  const flat = text.replace(/\s+/g, " ").trim();
  return flat.length <= max ? flat : `${flat.slice(0, max - 1).trimEnd()}…`;
}
