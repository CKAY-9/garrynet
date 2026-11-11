import { SiteShell } from "@/components/SiteShell";
import SiteUnauthorized from "./(site)/unauthorized";

// Rendered in place of the `(site)` layout (only for errors thrown above `(site)`'s own
// boundary), so it brings its own navigation.
export default function Unauthorized() {
  return (
    <SiteShell>
      <SiteUnauthorized />
    </SiteShell>
  );
}
