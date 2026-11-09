import Form from "next/form";
import { SearchIcon } from "./icons";
import styles from "./SideNav.module.scss";

/**
 * Searches users by name from anywhere on the site, opening the results on `/users/search`.
 * A plain GET form (`next/form` adds client-side navigation), so it works without JavaScript.
 */
export function NavSearch() {
  return (
    <Form action="/users/search" className={styles.search} role="search">
      <span className={styles.searchIcon}>
        <SearchIcon />
      </span>
      <input
        className={styles.searchInput}
        type="search"
        name="name"
        placeholder="Search users…"
        aria-label="Search users by name"
        autoComplete="off"
        enterKeyHint="search"
      />
    </Form>
  );
}
