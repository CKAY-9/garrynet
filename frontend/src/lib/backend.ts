import "server-only";
import axios from "axios";

/** The backend as the Next.js server reaches it (on the compose network, `http://backend:8080`). */
export const backendUrl = process.env.API_URL ?? process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

/** Axios client for server-side requests from Next.js to the GarryNet backend. */
export const backend = axios.create({
  baseURL: backendUrl,
  timeout: 5000,
});
