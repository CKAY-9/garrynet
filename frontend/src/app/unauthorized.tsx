import { SiteShell } from "@/components/SiteShell";
import { StatusPage } from "@/components/StatusPage";
import { steamLoginUrl } from "@/lib/config";

// Rendered in place of the `(site)` layout, so it brings its own navigation.
export default function Unauthorized() {
  return (
    <SiteShell>
      <StatusPage code={401} title="Login required">
        <p>You need to be logged in to view this page.</p>
        {/* Plain anchor: a full navigation to the backend, which redirects to Steam. */}
        <a href={steamLoginUrl}>Login with Steam</a>
      </StatusPage>
    </SiteShell>
  );
}
