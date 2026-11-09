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
} as const;

/** True if `permissions` includes every flag in `required`. */
export function hasPermission(permissions: number, required: number): boolean {
  return (permissions & required) === required;
}
