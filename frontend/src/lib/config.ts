/** Backend URL for links and forms the browser follows (login, logout). */
export const publicApiUrl = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";
/** Starts the Steam login flow on the backend. */
export const steamLoginUrl = `${publicApiUrl}/auth/steam`;

/** Clears the auth cookie on the backend (POST). */
export const logoutUrl = `${publicApiUrl}/auth/logout`;

/** Shown when the community has no logo of its own (`public/logo.svg`, a copy of `assets/logo.svg`). */
export const defaultCommunityLogo = "/logo.svg";
