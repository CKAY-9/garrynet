import { Permission } from "./permissions";

/**
 * The admin dashboard's sections. Every one also needs Admin Access; `permission` is what
 * the section needs on top of that.
 */
export const adminPages = [
  { href: "/admin", label: "Dashboard", permission: Permission.AdminAccess },
  { href: "/admin/usergroups", label: "Usergroups", permission: Permission.UsergroupManagement },
  { href: "/admin/servers", label: "Servers", permission: Permission.ServerManagement },
  { href: "/admin/loading-screen", label: "Loading Screen", permission: Permission.ServerManagement },
  { href: "/admin/community", label: "Community", permission: Permission.CommunityManagement },
] as const;
