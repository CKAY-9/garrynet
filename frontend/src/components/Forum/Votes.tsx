"use client";

import { useOptimistic, useState, useTransition } from "react";
import type { PostVote } from "@/lib/types";
import { votePost } from "./actions";
import styles from "./Forum.module.scss";

/**
 * A post's like and dislike counts. With `viewerId` and no `disabledReason`, they're
 * toggles: clicking your current vote takes it back, clicking the other one switches. The
 * vote shows at once and the page refreshes with the saved counts.
 */
export function PostVotes({
  postId,
  likes,
  dislikes,
  viewerId,
  disabledReason,
}: {
  postId: number;
  likes: number[];
  dislikes: number[];
  /** `null` when logged out. */
  viewerId: number | null;
  /** Why the viewer can't vote, shown on hover; `null` if they can. */
  disabledReason: string | null;
}) {
  const saved: PostVote | null =
    viewerId === null ? null : likes.includes(viewerId) ? "like" : dislikes.includes(viewerId) ? "dislike" : null;
  const [vote, setOptimisticVote] = useOptimistic(saved);
  const [pending, startTransition] = useTransition();
  const [error, setError] = useState<string | null>(null);

  // The saved counts, with the viewer's saved vote swapped for the optimistic one.
  const count = (kind: PostVote, ids: number[]) => ids.length - (saved === kind ? 1 : 0) + (vote === kind ? 1 : 0);
  const likeCount = count("like", likes);
  const dislikeCount = count("dislike", dislikes);

  if (disabledReason !== null) {
    return (
      <div className={styles.reactions} aria-label="Reactions" title={disabledReason}>
        <span title="Likes">👍 {likeCount}</span>
        <span title="Dislikes">👎 {dislikeCount}</span>
      </div>
    );
  }

  function choose(kind: PostVote) {
    const next = vote === kind ? null : kind;
    startTransition(async () => {
      setOptimisticVote(next);
      const res = await votePost(postId, next);
      setError(res.ok ? null : res.message);
    });
  }

  return (
    <div className={styles.reactions} role="group" aria-label="Reactions" aria-busy={pending}>
      <button
        type="button"
        className={vote === "like" ? styles.liked : undefined}
        aria-pressed={vote === "like"}
        title={vote === "like" ? "Take back your like" : "Like"}
        onClick={() => choose("like")}
        disabled={pending}
      >
        👍 {likeCount}
      </button>
      <button
        type="button"
        className={vote === "dislike" ? styles.disliked : undefined}
        aria-pressed={vote === "dislike"}
        title={vote === "dislike" ? "Take back your dislike" : "Dislike"}
        onClick={() => choose("dislike")}
        disabled={pending}
      >
        👎 {dislikeCount}
      </button>
      {error && (
        <p className={styles.error} role="status">
          {error}
        </p>
      )}
    </div>
  );
}
