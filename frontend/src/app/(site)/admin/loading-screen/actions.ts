"use server";

import axios from "axios";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { LoadingScreenConfig, LoadingScreenStyle } from "@/lib/types";

export type SaveStyleState = {
  /** The style as last saved. */
  style: LoadingScreenStyle;
  result: "idle" | "saved" | "error";
  message?: string;
};

/** Saves the loading screen style. The backend checks the permissions. */
export async function saveLoadingScreenStyle(prev: SaveStyleState, form: FormData): Promise<SaveStyleState> {
  const style = form.get("style");
  if (style !== "aero" && style !== "sleek") {
    return { ...prev, result: "error", message: "Pick a style." };
  }

  try {
    const { data } = await backend.put<LoadingScreenConfig>(
      "/loading-screen",
      { style },
      { headers: await authHeaders() },
    );
    return { style: data.style, result: "saved" };
  } catch (err) {
    const status = axios.isAxiosError(err) ? err.response?.status : undefined;
    let message = "Couldn't save the style. Try again.";
    if (status === 401) message = "Your session has expired. Log in again to save.";
    else if (status === 403) message = "You don't have permission to change the loading screen.";
    // Log only the message: the axios error carries the request config, including the token.
    else console.error("Failed to save loading screen style:", err instanceof Error ? err.message : err);
    return { ...prev, result: "error", message };
  }
}
