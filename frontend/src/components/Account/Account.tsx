import Image from "next/image";
import Link from "next/link";
import { getCurrentUser } from "@/lib/auth";
import { logoutUrl, steamLoginUrl } from "@/lib/config";
import { legalHref } from "@/lib/legalPages";
import { LogoutIcon, SettingsIcon, SteamIcon } from "@/components/SideNav/icons";
import styles from "./Account.module.scss";

/**
 * "Login with Steam" for logged-out users, otherwise the user's profile link, a link to
 * their settings and a logout button. Reads the session cookie, so render it inside a `<Suspense>` boundary.
 */
export async function Account() {
  const user = await getCurrentUser();

  if (!user) {
    return (
      <div className={styles.loggedOut}>
        {/* Plain anchor: this is a full navigation to the backend, which redirects to Steam. */}
        <a className={styles.login} href={steamLoginUrl}>
          <SteamIcon />
          Login with Steam
        </a>
        <p className={styles.consent}>
          By logging in you agree to our <Link href={legalHref("terms")}>Terms</Link> and{" "}
          <Link href={legalHref("privacy")}>Privacy Policy</Link>.
        </p>
      </div>
    );
  }

  return (
    <div className={styles.account}>
      <Link className={styles.profile} href={`/users/${user.id}`}>
        <Image
          className={styles.avatar}
          src={user.avatar_url}
          alt=""
          width={32}
          height={32}
        />
        <span className={styles.name}>{user.username}</span>
      </Link>
      <Link className={styles.settings} href="/settings">
        <SettingsIcon />
        Settings
      </Link>
      <form action={logoutUrl} method="post">
        <button className={styles.logout} type="submit">
          <LogoutIcon />
          Log out
        </button>
      </form>
    </div>
  );
}

export function AccountSkeleton() {
  return <div className={styles.skeleton} aria-hidden />;
}
