/** Permission bit flags. Must match `Permissions` in the backend (src/models/permissions.rs). */
export const Permission = {
  SiteAccess: 0x1,
  UserPosts: 0x2,
  ThreadManagement: 0x4,
  CategoryManagement: 0x8,
  SectionManagement: 0x10,
  UserManagement: 0x20,
  AdminAccess: 0x40,
  ElevatedControl: 0x80,
  UsergroupManagement: 0x100,
  UserMessages: 0x200,
  ServerManagement: 0x400,
  CommunityManagement: 0x800,
  SupportTeam: 0x1000,
} as const;

/** True if `permissions` includes every flag in `required`. */
export function hasPermission(permissions: number, required: number): boolean {
  return (permissions & required) === required;
}

/** Display names for each flag, lowest bit first (see docs/PERMISSIONS.md). */
export const PERMISSION_LABELS: readonly { flag: number; label: string }[] = [
  { flag: Permission.SiteAccess, label: "Site Access" },
  { flag: Permission.UserPosts, label: "User Posts" },
  { flag: Permission.ThreadManagement, label: "Thread Management" },
  { flag: Permission.CategoryManagement, label: "Category Management" },
  { flag: Permission.SectionManagement, label: "Section Management" },
  { flag: Permission.UserManagement, label: "User Management" },
  { flag: Permission.AdminAccess, label: "Admin Access" },
  { flag: Permission.ElevatedControl, label: "Elevated Control" },
  { flag: Permission.UsergroupManagement, label: "Usergroup Management" },
  { flag: Permission.UserMessages, label: "User Messages" },
  { flag: Permission.ServerManagement, label: "Server Management" },
  { flag: Permission.CommunityManagement, label: "Community Management" },
  { flag: Permission.SupportTeam, label: "Support Team" },
];
