import { StatusPage } from "@/components/StatusPage";
import { steamLoginUrl } from "@/lib/config";

// Shown when a `(site)` page calls `unauthorized()`. It renders inside the `(site)` layout, which
// already has the navigation. Without this file Next reuses `app/unauthorized.tsx` here, whose
// own navigation would then appear inside the content area.
export default function Unauthorized() {
  return (
    <StatusPage code={401} title="Login required">
      <p>You need to be logged in to view this page.</p>
      {/* Plain anchor: a full navigation to the backend, which redirects to Steam. */}
      <a href={steamLoginUrl}>Login with Steam</a>
    </StatusPage>
  );
}
