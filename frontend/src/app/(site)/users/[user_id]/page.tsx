import { Suspense } from "react";
import Link from "next/link";
import { notFound } from "next/navigation";
import { Activity, ActivitySkeleton, Profile, ProfileSkeleton, PunishmentControls, Punishments } from "@/components/Profile";
import profileStyles from "@/components/Profile/Profile.module.scss";
import { getCurrentUser } from "@/lib/auth";
import { Permission, hasPermission } from "@/lib/permissions";
import type { AuthoredFilters, AuthoredKind, AuthoredSort } from "@/lib/types";
import { getUserPosts, getUserProfile, getUserPunishments } from "@/lib/users";

type SearchParams = PageProps<"/users/[user_id]">["searchParams"];

export default function UserPage({ params, searchParams }: PageProps<"/users/[user_id]">) {
  return (
    // Params and the profile are request-time data, so they stream in behind the boundary.
    <Suspense fallback={<ProfileSkeleton />}>
      {params.then(({ user_id }) => (
        <UserProfile id={user_id} searchParams={searchParams} />
      ))}
    </Suspense>
  );
}

/** Banning and suspending, like other user management, needs both (docs/USERS.md). */
const MANAGE_USERS = Permission.AdminAccess | Permission.UserManagement;

async function UserProfile({ id, searchParams }: { id: string; searchParams: SearchParams }) {
  const [user, viewer] = await Promise.all([getUserProfile(id), getCurrentUser()]);
  if (!user) notFound();

  const self = viewer?.id === user.id;
  const staff = viewer !== null && hasPermission(viewer.permissions, MANAGE_USERS);
  // Staff see the history; users see their own punishments in effect.
  const punishments = staff || self ? await getUserPunishments(user.id) : null;
  const current = (kind: "BAN" | "SUSPEND") => punishments?.find((p) => p.in_effect && p.kind === kind) ?? null;
  // The backend leaves out a private profile's bio for anyone who may not see its activity either.
  const hidden = user.private && user.bio === null;

  return (
    <Profile
      user={user}
      // The backend refuses punishing yourself, and anyone holding permissions the viewer doesn't.
      actions={
        self ? (
          <Link className={profileStyles.edit} href="/settings">
            Edit profile
          </Link>
        ) : staff && punishments ? (
          <PunishmentControls
            userId={user.id}
            username={user.username}
            ban={current("BAN")}
            suspension={current("SUSPEND")}
          />
        ) : null
      }
      notices={punishments ? <Punishments punishments={punishments} staff={staff} /> : null}
      activity={
        hidden ? null : (
          <Suspense fallback={<ActivitySkeleton />}>
            <UserActivity userId={user.id} self={self} searchParams={searchParams} />
          </Suspense>
        )
      }
    />
  );
}

async function UserActivity({ userId, self, searchParams }: { userId: number; self: boolean; searchParams: SearchParams }) {
  const { filters, page } = await readActivityParams(searchParams);
  const data = await getUserPosts(userId, filters, page);
  if (!data) return null;
  return <Activity userId={userId} self={self} filters={filters} page={page} data={data} />;
}

const kinds: AuthoredKind[] = ["thread", "post"];
const sorts: AuthoredSort[] = ["new", "old", "reputation+", "reputation-"];
/** Far enough for anyone; keeps the backend's offset sane. */
const MAX_PAGE = 1000;

/** The activity filters and page from the URL; unknown values are dropped. */
async function readActivityParams(searchParams: SearchParams): Promise<{ filters: AuthoredFilters; page: number }> {
  const params = await searchParams;
  const first = (key: string) => {
    const value = params[key];
    return (Array.isArray(value) ? value[0] : value)?.trim() ?? "";
  };
  const oneOf = <T extends string>(value: string, allowed: T[]): T | "" =>
    allowed.includes(value as T) ? (value as T) : "";

  const page = Number(first("page"));
  return {
    filters: {
      kind: oneOf(first("kind"), kinds),
      q: first("q").slice(0, 100),
      // An unencoded `+` arrives as a trailing space, which `first` trims.
      sort: oneOf(first("sort") === "reputation" ? "reputation+" : first("sort"), sorts),
    },
    page: Number.isInteger(page) && page >= 1 ? Math.min(page, MAX_PAGE) : 1,
  };
}
