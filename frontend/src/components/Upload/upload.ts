import { UPLOAD_TYPES } from "@/lib/files";
import type { StoredFile } from "@/lib/types";
import { completeUpload, startUpload } from "./actions";

export type UploadResult = { ok: true; file: StoredFile } | { ok: false; message: string };

/**
 * Uploads a file picked in the browser (docs/STORAGE.md): asks the backend where to send
 * it, sends the bytes straight there, then has the backend verify them. `types` narrows
 * what is accepted, e.g. to images.
 */
export async function uploadFile(
  file: File,
  { community = false, types = UPLOAD_TYPES }: { community?: boolean; types?: string[] } = {},
): Promise<UploadResult> {
  if (!types.includes(file.type)) {
    return { ok: false, message: `${file.name} can't be uploaded: that type of file isn't allowed.` };
  }
  if (file.size === 0) return { ok: false, message: `${file.name} is empty.` };

  const started = await startUpload({ name: file.name, content_type: file.type, size: file.size, community });
  if (!started.ok) return { ok: false, message: `${file.name}: ${started.message}` };
  const { file: pending, upload } = started.data;

  try {
    const res = await fetch(upload.url, { method: upload.method, headers: upload.headers, body: file });
    if (!res.ok) throw new Error(`upload answered ${res.status}`);
  } catch (err) {
    console.error("Upload failed:", err);
    return { ok: false, message: `Couldn't upload ${file.name}. Try again.` };
  }

  const completed = await completeUpload(pending.owner_id, pending.id);
  if (!completed.ok) return { ok: false, message: `${file.name}: ${completed.message}` };
  return { ok: true, file: completed.data };
}
