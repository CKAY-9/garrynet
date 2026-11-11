import { SiteShell } from "@/components/SiteShell";
import SiteForbidden from "./(site)/forbidden";

// Rendered in place of the `(site)` layout (only for errors thrown above `(site)`'s own
// boundary), so it brings its own navigation.
export default function Forbidden() {
  return (
    <SiteShell>
      <SiteForbidden />
    </SiteShell>
  );
}
