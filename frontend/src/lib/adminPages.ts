import { Permission } from "./permissions";

/**
 * The admin dashboard's sections. Every one also needs Admin Access; `permission` is what
 * the section needs on top of that (any one of its flags when `any` is set).
 */
export const adminPages: { href: string; label: string; permission: number; any?: boolean }[] = [
  { href: "/admin", label: "Dashboard", permission: Permission.AdminAccess },
  { href: "/admin/usergroups", label: "Usergroups", permission: Permission.UsergroupManagement },
  { href: "/admin/servers", label: "Servers", permission: Permission.ServerManagement },
  { href: "/admin/loading-screen", label: "Loading Screen", permission: Permission.ServerManagement },
  { href: "/admin/community", label: "Community", permission: Permission.CommunityManagement },
  {
    href: "/admin/forum",
    label: "Forum",
    permission: Permission.SectionManagement | Permission.CategoryManagement,
    any: true,
  },
  {
    href: "/admin/store",
    label: "Store",
    permission: Permission.StoreManagement | Permission.StoreFinance,
    any: true,
  },
];
