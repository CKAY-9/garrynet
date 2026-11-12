"use client";

import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import { PAGE_SIZE, pageParams } from "@/lib/messageEnums";
import type { PrivateMessage } from "@/lib/types";

/** How close to the bottom (px) still counts as reading the newest messages. */
const BOTTOM_SLACK = 64;

type Page<T> = { items: T[]; next_cursor: string | null };

/**
 * The messages of a chat, oldest first, in a scroll box that starts at the newest one: older
 * pages load from `endpoint` (a route handler taking `limit` and `cursor`) when the top
 * comes into view, keeping the view still, and the view stays at the bottom as content grows
 * while it was there. Attach `scroller`, `onScroll`, `sentinel` (above the list) and
 * `content` (the list); `merge` adds messages from elsewhere.
 */
export function useMessagePages<T extends PrivateMessage>(initial: Page<T>, endpoint: string) {
  const [messages, setMessages] = useState(() => mergeMessages<T>([], initial.items));
  const [cursor, setCursor] = useState(initial.next_cursor);
  const [loadingOlder, setLoadingOlder] = useState(false);
  const [olderFailed, setOlderFailed] = useState(false);

  const scroller = useRef<HTMLDivElement>(null);
  const content = useRef<HTMLOListElement>(null);
  const sentinel = useRef<HTMLDivElement>(null);
  /** Whether the view ends at the newest message, so it should stay there as content grows. */
  const atBottom = useRef(true);
  /** Set just before older messages are prepended: the scroll position to keep the view on. */
  const anchor = useRef<{ height: number; top: number } | null>(null);
  const loadingRef = useRef(false);

  // Keep the view still when older messages are prepended, or at the bottom when it was there.
  useLayoutEffect(() => {
    const el = scroller.current;
    if (!el) return;
    if (anchor.current) {
      el.scrollTop = anchor.current.top + (el.scrollHeight - anchor.current.height);
      anchor.current = null;
    } else if (atBottom.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages]);

  // Images and other late layout changes: stay at the bottom if the view was there.
  useEffect(() => {
    const el = scroller.current;
    const list = content.current;
    if (!el || !list) return;
    const observer = new ResizeObserver(() => {
      if (atBottom.current && !anchor.current) el.scrollTop = el.scrollHeight;
    });
    observer.observe(list);
    return () => observer.disconnect();
  }, []);

  function onScroll() {
    const el = scroller.current;
    if (el) atBottom.current = el.scrollHeight - el.scrollTop - el.clientHeight < BOTTOM_SLACK;
  }

  const loadOlder = useCallback(async () => {
    if (!cursor || loadingRef.current) return;
    loadingRef.current = true;
    setLoadingOlder(true);
    setOlderFailed(false);
    try {
      const res = await fetch(`${endpoint}?${pageParams(cursor, PAGE_SIZE)}`, { cache: "no-store" });
      if (!res.ok) throw new Error(`status ${res.status}`);
      const page = (await res.json()) as Page<T>;
      const el = scroller.current;
      if (el) anchor.current = { height: el.scrollHeight, top: el.scrollTop };
      setMessages((list) => mergeMessages(list, page.items));
      setCursor(page.next_cursor);
    } catch {
      setOlderFailed(true);
    } finally {
      loadingRef.current = false;
      setLoadingOlder(false);
    }
  }, [cursor, endpoint]);

  // Load older messages when the top comes into view. Re-observing after each page fires
  // again at once if the top is still in view, so a short chat fills the box.
  useEffect(() => {
    const root = scroller.current;
    const target = sentinel.current;
    if (!root || !target || !cursor || loadingOlder || olderFailed) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) void loadOlder();
      },
      { root, rootMargin: "200px 0px 0px 0px" },
    );
    observer.observe(target);
    return () => observer.disconnect();
  }, [cursor, loadingOlder, olderFailed, loadOlder]);

  /** Adds or updates messages, e.g. newer ones from a poll or one just sent. */
  const merge = useCallback((items: T[]) => setMessages((list) => mergeMessages(list, items)), []);

  /** Scroll to the newest message once the next change renders. */
  const stickToBottom = useCallback(() => {
    atBottom.current = true;
  }, []);

  return {
    messages,
    merge,
    stickToBottom,
    cursor,
    loadingOlder,
    olderFailed,
    loadOlder,
    scroller,
    content,
    sentinel,
    onScroll,
  };
}

/**
 * `list` with `incoming` merged in by id (the incoming copy wins, e.g. once a message is read),
 * oldest first. Returns `list` itself when nothing changed, so a quiet poll doesn't re-render.
 */
function mergeMessages<T extends PrivateMessage>(list: T[], incoming: T[]): T[] {
  const byId = new Map(list.map((m) => [m.id, m]));
  let changed = false;
  for (const m of incoming) {
    const current = byId.get(m.id);
    if (current && current.read_at === m.read_at) continue;
    byId.set(m.id, m);
    changed = true;
  }
  if (!changed) return list;
  return [...byId.values()].sort((a, b) => Date.parse(a.created_at) - Date.parse(b.created_at) || a.id - b.id);
}
