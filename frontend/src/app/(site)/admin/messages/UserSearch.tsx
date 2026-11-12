"use client";

import { useRouter } from "next/navigation";
import { UserPicker } from "@/components/UserPicker";
import { adminChatsHref } from "@/lib/messageEnums";
import type { UserRef } from "@/lib/types";

/** Find a user by name or id, then open their chats. */
export function UserSearch() {
  const router = useRouter();
  return (
    <UserPicker
      mode="single"
      onSelect={(user: UserRef) => router.push(adminChatsHref(user.id))}
      hint="Pick whose chats to read."
      autoFocus
    />
  );
}
