import { Suspense } from "react";
import { notFound } from "next/navigation";
import { Profile, ProfileSkeleton } from "@/components/Profile";
import { getUserProfile } from "@/lib/users";

export default function UserPage({ params }: PageProps<"/users/[user_id]">) {
  return (
    // Params and the profile are request-time data, so they stream in behind the boundary.
    <Suspense fallback={<ProfileSkeleton />}>
      {params.then(({ user_id }) => (
        <UserProfile id={user_id} />
      ))}
    </Suspense>
  );
}

async function UserProfile({ id }: { id: string }) {
  const user = await getUserProfile(id);
  if (!user) notFound();
  return <Profile user={user} />;
}
