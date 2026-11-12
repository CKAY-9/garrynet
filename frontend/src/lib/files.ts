import { publicApiUrl } from "./config";
import type { Attachment } from "./types";

/*
 * File storage (docs/STORAGE.md). Safe to import from client components. Uploading is in
 * `@/components/Upload`.
 */

/** Content types the backend accepts (src/routes/files.rs). */
export const IMAGE_TYPES = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/avif"];
export const UPLOAD_TYPES = [
  ...IMAGE_TYPES,
  "audio/mpeg",
  "audio/ogg",
  "audio/wav",
  "video/mp4",
  "video/webm",
  "application/pdf",
  "text/plain",
];

/** Limits enforced by the backend (src/routes/forum.rs, src/routes/support.rs, src/routes/messages.rs). */
export const MAX_POST_ATTACHMENTS = 5;
export const MAX_MESSAGE_ATTACHMENTS = 10;
/** Per private message. */
export const MAX_PM_ATTACHMENTS = 3;

/** Where the browser fetches a file. */
export function fileUrl(file: Pick<Attachment, "owner_id" | "id">): string {
  return `${publicApiUrl}/file/${file.owner_id}/${file.id}`;
}

export const isImage = (file: Pick<Attachment, "content_type">) => IMAGE_TYPES.includes(file.content_type);

/** "512 B", "340 KB", "1.4 MB". */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
