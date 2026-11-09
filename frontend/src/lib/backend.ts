import "server-only";
import axios from "axios";

/** Axios client for server-side requests from Next.js to the GarryNet backend. */
export const backend = axios.create({
  baseURL: process.env.API_URL ?? process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080",
  timeout: 5000,
});
