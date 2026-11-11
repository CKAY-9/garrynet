import { Suspense } from "react";
import type { Metadata } from "next";
import Link from "next/link";
import { requirePermission } from "@/lib/auth";
import { Permission } from "@/lib/permissions";
import { SettingsForm } from "./SettingsForm";
import styles from "./settings.module.scss";

export const metadata: Metadata = {
  title: "Settings · GarryNet",
};

export default function SettingsPage() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>Settings</h1>
      </header>
      {/* The login check reads the session cookie, so it runs behind the boundary. */}
      <Suspense fallback={<div className={styles.skeleton} aria-busy aria-label="Loading settings" />}>
        <UserSettings />
      </Suspense>
    </div>
  );
}

/** Logged-in users who aren't banned (banned users lack Site Access). */
async function UserSettings() {
  const user = await requirePermission(Permission.SiteAccess);

  return (
    <section className={styles.section} aria-labelledby="user-settings-heading">
      <div className={styles.sectionHeader}>
        <h2 id="user-settings-heading" className={styles.sectionTitle}>
          User Settings
        </h2>
        <Link className={styles.view} href={`/users/${user.id}`}>
          View your profile
        </Link>
      </div>
      <SettingsForm settings={{ bio: user.bio, private: user.private }} />
    </section>
  );
}
