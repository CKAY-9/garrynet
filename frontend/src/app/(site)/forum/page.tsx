import { Suspense } from "react";
import { ForumHome } from "@/components/Forum";
import { getForumHome } from "@/lib/forum";

export default function ForumPage() {
  return (
    // The forum is fetched at request time, so it streams in behind the boundary.
    <Suspense>
      <Home />
    </Suspense>
  );
}

async function Home() {
  return <ForumHome {...await getForumHome()} />;
}
