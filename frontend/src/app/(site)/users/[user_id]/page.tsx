import { Suspense } from "react";
import { notFound } from "next/navigation";
import { Profile, ProfileSkeleton, PunishmentControls, Punishments } from "@/components/Profile";
import { getCurrentUser } from "@/lib/auth";
import { Permission, hasPermission } from "@/lib/permissions";
import { getUserProfile, getUserPunishments } from "@/lib/users";

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

/** Banning and suspending, like other user management, needs both (docs/USERS.md). */
const MANAGE_USERS = Permission.AdminAccess | Permission.UserManagement;

async function UserProfile({ id }: { id: string }) {
  const [user, viewer] = await Promise.all([getUserProfile(id), getCurrentUser()]);
  if (!user) notFound();

  const self = viewer?.id === user.id;
  const staff = viewer !== null && hasPermission(viewer.permissions, MANAGE_USERS);
  // Staff see the history; users see their own punishments in effect.
  const punishments = staff || self ? await getUserPunishments(user.id) : null;
  const current = (kind: "BAN" | "SUSPEND") => punishments?.find((p) => p.in_effect && p.kind === kind) ?? null;

  return (
    <Profile
      user={user}
      // The backend refuses punishing yourself, and anyone holding permissions the viewer doesn't.
      actions={
        staff && !self && punishments ? (
          <PunishmentControls
            userId={user.id}
            username={user.username}
            ban={current("BAN")}
            suspension={current("SUSPEND")}
          />
        ) : null
      }
      notices={punishments ? <Punishments punishments={punishments} staff={staff} /> : null}
    />
  );
}
