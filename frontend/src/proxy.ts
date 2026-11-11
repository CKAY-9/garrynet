import { NextResponse, type NextRequest } from "next/server";
import { Permission, hasPermission } from "@/lib/permissions";

/*
 * Banned users (no Site Access) can only use support (docs/PERMISSIONS.md), so their page
 * requests elsewhere are redirected to /support. This is an optimistic, UX-level check: the
 * backend enforces what a ban forbids on every write, and the pages it hides are public to
 * logged-out visitors anyway. If the backend can't be asked, the request goes through.
 */

/** Must match `AUTH_COOKIE` in `@/lib/auth` (not imported: that module is server-render only). */
const AUTH_COOKIE = "garrynet_token";

/** What banned users may still open: support, the notifications about it, and their receipts. */
const ALLOWED = ["/support", "/notifications", "/store/purchases", "/store/checkout"];

const API_URL = process.env.API_URL ?? process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

/** Don't hold a navigation up for long if the backend is slow. */
const TIMEOUT_MS = 2000;

const isAllowed = (path: string) => ALLOWED.some((prefix) => path === prefix || path.startsWith(`${prefix}/`));

/** The user's effective permissions, or `null` if they can't be read (logged out, backend down). */
async function permissionsFor(token: string): Promise<number | null> {
  try {
    const res = await fetch(`${API_URL}/users/me`, {
      headers: { Authorization: `Bearer ${token}` },
      cache: "no-store",
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
    if (!res.ok) return null;
    const { permissions } = (await res.json()) as { permissions?: unknown };
    return typeof permissions === "number" ? permissions : null;
  } catch {
    return null;
  }
}

export async function proxy(request: NextRequest) {
  // Only page loads: server actions and other writes are refused by the backend, and
  // redirecting a POST would replay it against /support.
  if (request.method !== "GET" && request.method !== "HEAD") return NextResponse.next();
  if (isAllowed(request.nextUrl.pathname)) return NextResponse.next();

  const token = request.cookies.get(AUTH_COOKIE)?.value;
  if (!token) return NextResponse.next();

  const permissions = await permissionsFor(token);
  if (permissions === null || hasPermission(permissions, Permission.SiteAccess)) return NextResponse.next();

  const url = request.nextUrl.clone();
  url.pathname = "/support";
  url.search = "";
  return NextResponse.redirect(url);
}

export const config = {
  matcher: [
    {
      // Pages only: not API routes, Next's assets, the in-game loading screen or files with
      // an extension (public/ assets such as /logo.svg).
      source: "/((?!api/|_next/|loading-screen|.*\\.[^/]+$).*)",
      // Logged-out requests never need the check.
      has: [{ type: "cookie", key: "garrynet_token" }],
    },
  ],
};
