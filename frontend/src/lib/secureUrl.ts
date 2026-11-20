/**
 * HTTPS is required outside this machine in production builds (`next dev` allows plain HTTP):
 * the backend URLs carry login tokens. Mirrors `require_https` in the backend's config.rs.
 *
 * - `public`: reached by browsers. Plain HTTP only for `localhost`, `*.localhost` or a
 *   loopback address.
 * - `internal`: reached only by this server. Also a single-label name (`backend`), as services
 *   on a private network such as Docker Compose's are named.
 */
export type Reach = "public" | "internal";

function isLocal(url: URL, reach: Reach): boolean {
  const host = url.hostname.toLowerCase().replace(/\.$/, "");
  if (host === "localhost" || host.endsWith(".localhost")) return true;
  if (host === "[::1]" || /^127\.\d+\.\d+\.\d+$/.test(host)) return true;
  return reach === "internal" && host !== "" && !host.includes(".") && !host.includes(":");
}

/** Why `url` (from env var `name`) isn't acceptable, or `null` if it is. */
export function insecureUrlProblem(name: string, url: string, reach: Reach): string | null {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return `${name} isn't a valid URL: ${JSON.stringify(url)}`;
  }
  if (parsed.protocol === "https:" || isLocal(parsed, reach)) return null;
  return `${name} must be an https:// URL outside localhost, got ${JSON.stringify(url)}`;
}

/** Throws in production if any of `urls` (env var name, value, reach) is insecure. */
export function requireHttps(urls: [name: string, url: string | undefined, reach: Reach][]): void {
  if (process.env.NODE_ENV !== "production") return;
  const problems = urls
    .filter((entry): entry is [string, string, Reach] => !!entry[1])
    .map(([name, url, reach]) => insecureUrlProblem(name, url, reach))
    .filter((p): p is string => p !== null);
  if (problems.length > 0) throw new Error(`Configuration error: ${problems.join("; ")}`);
}
