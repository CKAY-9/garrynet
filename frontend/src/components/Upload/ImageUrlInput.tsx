"use client";

import { useRef, useState, type ChangeEvent } from "react";
import { IMAGE_TYPES } from "@/lib/files";
import { uploadFile } from "./upload";
import styles from "./Upload.module.scss";

type Props = {
  value: string;
  onChange: (url: string) => void;
  /** Class of the form's own text inputs, so this one matches them. */
  className?: string;
  placeholder?: string;
};

/**
 * A link input for an image, with an Upload button beside it: uploading an image stores it
 * as a community file and fills the input with its URL. For the admin pages (uploading
 * community files needs Admin Access).
 */
export function ImageUrlInput({ value, onChange, className, placeholder }: Props) {
  const input = useRef<HTMLInputElement>(null);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onPick(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    // Reset so picking the same file again fires a change.
    event.target.value = "";
    if (!file) return;
    setUploading(true);
    setError(null);
    const res = await uploadFile(file, { community: true, types: IMAGE_TYPES });
    setUploading(false);
    if (res.ok) onChange(res.file.url);
    else setError(res.message);
  }

  return (
    <span className={styles.urlInput}>
      <span className={styles.urlRow}>
        <input
          className={className}
          type="url"
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
        />
        <input
          ref={input}
          type="file"
          className={styles.hiddenInput}
          accept={IMAGE_TYPES.join(",")}
          onChange={onPick}
          tabIndex={-1}
          aria-hidden
        />
        <button type="button" className={styles.button} onClick={() => input.current?.click()} disabled={uploading}>
          {uploading ? "Uploading..." : "Upload"}
        </button>
      </span>
      {error && (
        <span className={styles.error} role="alert">
          {error}
        </span>
      )}
    </span>
  );
}
