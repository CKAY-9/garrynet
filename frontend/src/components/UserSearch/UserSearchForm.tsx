import Form from "next/form";
import Link from "next/link";
import type { LastActivity, UsergroupBadge, UserSearchFilters, UserSort } from "@/lib/types";
import styles from "./UserSearch.module.scss";

const activityOptions: [LastActivity, string][] = [
  ["online", "Online now"],
  ["hour", "Past hour"],
  ["day", "Past day"],
  ["week", "Past week"],
  ["month", "Past month"],
];

const sortOptions: [UserSort, string][] = [
  ["reputation+", "Most reputation"],
  ["reputation-", "Least reputation"],
  ["posts", "Most posts"],
];

type Props = { filters: UserSearchFilters; usergroups: UsergroupBadge[] };

/**
 * Filters for `/users/search`, submitted as a GET form so the filters live in the URL.
 * Blank fields are ignored by the backend.
 */
export function UserSearchForm({ filters, usergroups }: Props) {
  return (
    // Keyed by the filters so the fields reset to the URL's values after each search.
    <Form key={JSON.stringify(filters)} action="/users/search" className={styles.form} role="search">
      <label className={`${styles.field} ${styles.wide}`}>
        <span>Name</span>
        <input type="search" name="name" defaultValue={filters.name} placeholder="Any username" autoComplete="off" />
      </label>
      <label className={styles.field}>
        <span>User ID</span>
        <input type="text" name="id" inputMode="numeric" pattern="\d*" defaultValue={filters.id} placeholder="Any" />
      </label>
      <label className={styles.field}>
        <span>Usergroup</span>
        <select name="usergroup" defaultValue={filters.usergroup ?? ""}>
          <option value="">Any group</option>
          {usergroups.map((group) => (
            <option key={group.id} value={group.id}>
              {group.name}
            </option>
          ))}
        </select>
      </label>
      <label className={styles.field}>
        <span>Last active</span>
        <select name="last_activity" defaultValue={filters.last_activity ?? ""}>
          <option value="">Any time</option>
          {activityOptions.map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </select>
      </label>
      <label className={styles.field}>
        <span>Sort by</span>
        <select name="sort_by" defaultValue={filters.sort_by ?? ""}>
          <option value="">Best match</option>
          {sortOptions.map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </select>
      </label>
      <div className={styles.actions}>
        <button className={styles.submit} type="submit">
          Search
        </button>
        <Link className={styles.clear} href="/users/search">
          Clear
        </Link>
      </div>
    </Form>
  );
}

export function UserSearchFormSkeleton() {
  return <div className={`${styles.form} ${styles.formSkeleton}`} aria-busy aria-label="Loading filters" />;
}
