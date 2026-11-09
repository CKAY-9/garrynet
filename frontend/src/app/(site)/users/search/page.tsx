import { Suspense } from "react";
import { UserCard, UserCardGridSkeleton, UserSearchForm, UserSearchFormSkeleton } from "@/components/UserSearch";
import { getUsergroups, searchUsers, UserSearchError } from "@/lib/users";
import type { LastActivity, UserSearchFilters, UserSort } from "@/lib/types";
import cardStyles from "@/components/UserSearch/UserSearch.module.scss";
import styles from "./search.module.scss";

type SearchParams = PageProps<"/users/search">["searchParams"];

export default function UserSearchPage({ searchParams }: PageProps<"/users/search">) {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>Users</h1>
        <p className={styles.subtitle}>Find members of the community and view their profiles.</p>
      </header>
      {/* The filters and results depend on the URL and request-time data, so they stream in. */}
      <Suspense fallback={<UserSearchFormSkeleton />}>
        <Filters searchParams={searchParams} />
      </Suspense>
      <Suspense fallback={<UserCardGridSkeleton />}>
        <Results searchParams={searchParams} />
      </Suspense>
    </div>
  );
}

async function Filters({ searchParams }: { searchParams: SearchParams }) {
  const [filters, usergroups] = await Promise.all([readFilters(searchParams), getUsergroups()]);
  return <UserSearchForm filters={filters} usergroups={usergroups} />;
}

async function Results({ searchParams }: { searchParams: SearchParams }) {
  const filters = await readFilters(searchParams);

  let users;
  try {
    users = await searchUsers(filters);
  } catch (err) {
    if (err instanceof UserSearchError) {
      return <p className={styles.error}>Couldn&apos;t search: {err.message}.</p>;
    }
    throw err;
  }

  const filtered = Object.values(filters).some((v) => v?.trim());
  if (users.length === 0) {
    return <p className={styles.empty}>{filtered ? "No users match these filters." : "No users yet."}</p>;
  }

  return (
    <section className={styles.results} aria-label="Results">
      <p className={styles.summary}>
        {filtered
          ? `${users.length} ${users.length === 1 ? "user" : "users"} found`
          : "A few random members of the community"}
      </p>
      <div className={cardStyles.grid}>
        {users.map((user) => (
          <UserCard key={user.id} user={user} />
        ))}
      </div>
    </section>
  );
}

const lastActivities: LastActivity[] = ["online", "hour", "day", "week", "month"];
const sorts: UserSort[] = ["reputation+", "reputation-", "posts"];

/** The supported filters from the URL; unknown select values are dropped. */
async function readFilters(searchParams: SearchParams): Promise<UserSearchFilters> {
  const params = await searchParams;
  const first = (key: string) => {
    const value = params[key];
    return (Array.isArray(value) ? value[0] : value)?.trim() ?? "";
  };
  const oneOf = <T extends string>(value: string, allowed: T[]): T | "" =>
    allowed.includes(value as T) ? (value as T) : "";

  return {
    name: first("name"),
    id: first("id"),
    usergroup: first("usergroup"),
    last_activity: oneOf(first("last_activity"), lastActivities),
    // An unencoded `+` arrives as a trailing space, which `first` trims.
    sort_by: oneOf(first("sort_by") === "reputation" ? "reputation+" : first("sort_by"), sorts),
  };
}
