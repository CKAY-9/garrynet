"use client";

import { useEffect, useState, useTransition } from "react";
import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import type { UsergroupMember } from "@/lib/types";
import { dateFormat, fullFormat } from "@/lib/time";
import form from "../../community/editor.module.scss";
import styles from "../usergroups.module.scss";
import { addUsergroupMember, findUsersToAdd, removeUsergroupMember, type UserRef } from "../actions";

/** Members shown at once; the filter narrows the rest down. */
const MAX_SHOWN = 100;

type Props = {
  groupId: number;
  groupName: string;
  members: UsergroupMember[];
  /** The viewer's own user id, to mark them in the list. */
  viewerId: number;
  /** False if the viewer lacks some of the group's permissions, so can't change its members. */
  canEdit: boolean;
};

/** The group's members: a filterable list with remove buttons, and an "Add user" search. */
export function UsergroupMembers({ groupId, groupName, members, viewerId, canEdit }: Props) {
  const [filter, setFilter] = useState("");
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);
  const [pending, startTransition] = useTransition();

  const needle = filter.trim().toLowerCase();
  const matching = needle
    ? members.filter((m) => m.username.toLowerCase().includes(needle) || String(m.id) === needle)
    : members;
  const shown = matching.slice(0, MAX_SHOWN);

  function remove(member: UsergroupMember) {
    setError(null);
    setBusyId(member.id);
    startTransition(async () => {
      const res = await removeUsergroupMember(groupId, member.id);
      if (!res.ok) setError(res.message);
      setBusyId(null);
    });
  }

  function add(user: UserRef) {
    setError(null);
    setBusyId(user.id);
    startTransition(async () => {
      const res = await addUsergroupMember(groupId, user.id);
      if (!res.ok) setError(res.message);
      setBusyId(null);
    });
  }

  return (
    <div className={styles.members}>
      {canEdit && (
        <div className={styles.memberTools}>
          <button
            type="button"
            className={adding ? form.discard : form.save}
            onClick={() => setAdding((a) => !a)}
            aria-expanded={adding}
          >
            {adding ? "Done adding" : "+ Add user"}
          </button>
        </div>
      )}

      {!canEdit && (
        <p className={form.hint}>
          This group grants permissions you don&apos;t hold, so you can view its members but not change them.
        </p>
      )}

      {canEdit && adding && (
        <AddUserSearch members={members} busyId={pending ? busyId : null} onAdd={add} groupName={groupName} />
      )}

      {error && (
        <p className={`${form.message} ${form.error}`} role="status">
          {error}
        </p>
      )}

      {members.length === 0 ? (
        <p className={styles.none}>No members yet.</p>
      ) : (
        <div className={styles.memberBox}>
          <div className={styles.memberFilter}>
            <input
              type="search"
              className={form.input}
              value={filter}
              onChange={(e) => setFilter(e.target.value)}
              placeholder="Filter members by name or id..."
              aria-label="Filter members"
              autoComplete="off"
            />
          </div>
          {shown.length === 0 ? (
            <p className={`${styles.none} ${styles.memberEmpty}`}>No members match &ldquo;{filter.trim()}&rdquo;.</p>
          ) : (
            <ul className={styles.memberList} aria-label={`Members of ${groupName}`}>
              {shown.map((m) => {
                const busy = pending && busyId === m.id;
                return (
                  <li key={m.id} className={styles.member} aria-busy={busy}>
                    <Avatar name={m.username} url={m.avatar_url || null} size={32} shape="circle" />
                    <div className={styles.memberName}>
                      <Link href={`/users/${m.id}`}>{m.username}</Link>
                      <span className={styles.id}>
                        #{m.id}
                        {m.id === viewerId && " (you)"}
                      </span>
                    </div>
                    <time
                      className={styles.memberSince}
                      dateTime={m.added_at}
                      title={`Added ${fullFormat.format(new Date(m.added_at))}`}
                    >
                      {dateFormat.format(new Date(m.added_at))}
                    </time>
                    {canEdit && (
                      <button
                        type="button"
                        className={styles.removeMember}
                        onClick={() => remove(m)}
                        disabled={pending}
                        aria-label={`Remove ${m.username} from ${groupName}`}
                        title="Remove from group"
                      >
                        {busy ? "…" : "×"}
                      </button>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      )}
      {matching.length > shown.length && (
        <p className={form.hint}>
          Showing {shown.length} of {matching.length}. Filter to find others.
        </p>
      )}
    </div>
  );
}

/** Searches users by name or id (up to 10) and adds the one clicked. */
function AddUserSearch({
  members,
  busyId,
  onAdd,
  groupName,
}: {
  members: UsergroupMember[];
  busyId: number | null;
  onAdd: (user: UserRef) => void;
  groupName: string;
}) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<UserRef[]>([]);
  const [searching, setSearching] = useState(false);
  const [failed, setFailed] = useState<string | null>(null);

  // Search a moment after typing stops.
  useEffect(() => {
    const q = query.trim();
    if (!q) return;
    let cancelled = false;
    const timer = setTimeout(async () => {
      setSearching(true);
      const res = await findUsersToAdd(q);
      if (cancelled) return;
      setSearching(false);
      setFailed(res.ok ? null : res.message);
      setResults(res.ok ? res.data : []);
    }, 300);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [query]);

  const memberIds = new Set(members.map((m) => m.id));

  return (
    <div className={styles.addUser}>
      <input
        type="search"
        className={form.input}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Search users by name or id..."
        aria-label={`Search users to add to ${groupName}`}
        autoComplete="off"
        autoFocus
      />
      {query.trim() && (
        <ul className={styles.results} aria-label="Matching users">
          {results.map((u) => {
            const member = memberIds.has(u.id);
            return (
              <li key={u.id}>
                <button type="button" onClick={() => onAdd(u)} disabled={member || busyId !== null}>
                  <Avatar name={u.username} url={u.avatar_url || null} size={24} shape="circle" />
                  {u.username} <span className={styles.id}>#{u.id}</span>
                  <span className={styles.resultState}>
                    {member ? "Member" : busyId === u.id ? "Adding..." : "Add"}
                  </span>
                </button>
              </li>
            );
          })}
          {results.length === 0 && <li>{searching ? "Searching..." : (failed ?? "No users found.")}</li>}
        </ul>
      )}
    </div>
  );
}
