import Link from "next/link";
import { Avatar } from "@/components/Avatar";
import { FALLBACK_COMMUNITY_NAME, getCommunity } from "@/lib/community";
import { defaultCommunityLogo } from "@/lib/config";
import styles from "./SideNav.module.scss";

/** The community's logo and name, linking home. Request-time, so render it inside `<Suspense>`. */
export async function CommunityBrand() {
  const community = await getCommunity();
  const name = community?.name ?? FALLBACK_COMMUNITY_NAME;
  return (
    <Link className={styles.brand} href="/">
      <Avatar name={name} url={community?.logo_url ?? defaultCommunityLogo} size={36} />
      <span className={styles.brandName}>{name}</span>
    </Link>
  );
}

export function CommunityBrandSkeleton() {
  return (
    <div className={styles.brand} aria-hidden>
      <span className={styles.brandSkeletonLogo} />
      <span className={styles.brandSkeletonName} />
    </div>
  );
}
