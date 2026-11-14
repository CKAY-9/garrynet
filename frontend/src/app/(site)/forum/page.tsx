import { Suspense } from "react";
import type { Metadata } from "next";
import { ForumHome } from "@/components/Forum";
import { getForumHome } from "@/lib/forum";

export const metadata: Metadata = {
  title: "Forums",
};

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
