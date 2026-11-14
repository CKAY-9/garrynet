import { getDemo } from "@/lib/demo";
import { fullFormat, timeAgo } from "@/lib/time";
import styles from "./DemoBanner.module.scss";

/** Above every page of a demo deployment: payments are test-only and visitors' content is cleared daily. */
export async function DemoBanner() {
  const demo = await getDemo();
  if (!demo.enabled) return null;
  const reset = demo.next_reset;
  return (
    <aside className={styles.banner} aria-label="Demo site">
      <strong>Demo site.</strong> Payments use Stripe test mode, so nothing is charged. Everything posted, uploaded or
      bought by non-admins is cleared daily at midnight UTC
      {reset && (
        <>
          {" "}
          (next:{" "}
          <time dateTime={reset} title={fullFormat.format(new Date(reset))}>
            {timeAgo(new Date(reset))}
          </time>
          )
        </>
      )}
      .
    </aside>
  );
}
