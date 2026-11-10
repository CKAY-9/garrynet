"use client";

import { useRef, useState, type ChangeEvent } from "react";
import { formatSize, UPLOAD_TYPES } from "@/lib/files";
import type { StoredFile } from "@/lib/types";
import { discardUpload } from "./actions";
import { uploadFile } from "./upload";
import styles from "./Upload.module.scss";

/**
 * The files attached to a post or message being written. Give `picker` to an
 * `<AttachmentPicker>`, send `ids` with the post, and don't submit while `busy`.
 */
export function useAttachments(max: number) {
  const [files, setFiles] = useState<StoredFile[]>([]);
  const [uploading, setUploading] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  /** Uploads the picked files one after another, up to `max` in total. */
  async function add(picked: File[]) {
    setError(null);
    const room = max - files.length;
    if (picked.length > room) setError(`You can attach up to ${max} files.`);
    for (const file of picked.slice(0, room)) {
      setUploading(file.name);
      const res = await uploadFile(file);
      if (res.ok) setFiles((current) => [...current, res.file]);
      else setError(res.message);
    }
    setUploading(null);
  }

  /** Detaches a file and deletes it from storage. */
  function remove(file: StoredFile) {
    setFiles((current) => current.filter((f) => f.id !== file.id));
    void discardUpload(file.owner_id, file.id);
  }

  /** Empties the list: after posting (the files now belong to the post), or with `discard` when cancelling. */
  function clear({ discard = false } = {}) {
    if (discard) files.forEach((f) => void discardUpload(f.owner_id, f.id));
    setFiles([]);
    setError(null);
  }

  return {
    ids: files.map((f) => f.id),
    busy: uploading !== null,
    clear,
    picker: { files, uploading, error, max, add, remove },
  };
}

type PickerProps = ReturnType<typeof useAttachments>["picker"] & { disabled?: boolean };

/** "Attach files" button with the list of files attached so far. See `useAttachments`. */
export function AttachmentPicker({ files, uploading, error, max, add, remove, disabled = false }: PickerProps) {
  const input = useRef<HTMLInputElement>(null);
  const full = files.length >= max;

  function onPick(event: ChangeEvent<HTMLInputElement>) {
    const picked = Array.from(event.target.files ?? []);
    // Reset so picking the same file again fires a change.
    event.target.value = "";
    if (picked.length > 0) void add(picked);
  }

  return (
    <div className={styles.picker}>
      {files.length > 0 && (
        <ul className={styles.picked} aria-label="Attached files">
          {files.map((file) => (
            <li key={file.id} className={styles.pickedFile}>
              <span className={styles.fileName}>{file.name}</span>
              <span className={styles.fileSize}>{formatSize(file.size)}</span>
              <button type="button" onClick={() => remove(file)} disabled={disabled} aria-label={`Remove ${file.name}`}>
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className={styles.pickerRow}>
        <input
          ref={input}
          type="file"
          className={styles.hiddenInput}
          accept={UPLOAD_TYPES.join(",")}
          multiple
          onChange={onPick}
          tabIndex={-1}
          aria-hidden
        />
        <button
          type="button"
          className={styles.button}
          onClick={() => input.current?.click()}
          disabled={disabled || full || uploading !== null}
        >
          {uploading !== null ? "Uploading..." : "Attach files"}
        </button>
        <span className={styles.hint} role="status">
          {uploading !== null ? uploading : `${files.length} of ${max} files. Images, audio, video, PDF or text.`}
        </span>
      </div>
      {error && (
        <p className={styles.error} role="alert">
          {error}
        </p>
      )}
    </div>
  );
}
