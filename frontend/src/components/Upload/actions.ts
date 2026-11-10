"use server";

import { runAction, type ActionResult } from "@/lib/actionResult";
import { authHeaders } from "@/lib/auth";
import { backend } from "@/lib/backend";
import type { StoredFile, UploadTarget } from "@/lib/types";

/** A file to upload. `community` stores it as a community file (admin pages only). */
export type NewUpload = { name: string; content_type: string; size: number; community: boolean };

/**
 * Records a pending file and returns where the browser should send its bytes. The backend
 * checks the type, size and upload rate.
 */
export async function startUpload(upload: NewUpload): Promise<ActionResult<{ file: StoredFile; upload: UploadTarget }>> {
  return runAction("upload the file", async () => {
    const { data } = await backend.post<{ file: StoredFile; upload: UploadTarget }>("/file/upload", upload, {
      headers: await authHeaders(),
    });
    return data;
  });
}

/** Verifies the bytes arrived and makes the file readable. */
export async function completeUpload(ownerId: number, id: string): Promise<ActionResult<StoredFile>> {
  return runAction("upload the file", async () => {
    const { data } = await backend.post<StoredFile>(`/file/${ownerId}/${encodeURIComponent(id)}/complete`, null, {
      headers: await authHeaders(),
    });
    return data;
  });
}

/** Deletes a file the user uploaded but won't use after all. */
export async function discardUpload(ownerId: number, id: string): Promise<ActionResult<void>> {
  return runAction("remove the file", async () => {
    await backend.delete(`/file/${ownerId}/${encodeURIComponent(id)}`, { headers: await authHeaders() });
  });
}
