import Image from "next/image";
import { fileUrl, formatSize, isImage } from "@/lib/files";
import type { Attachment } from "@/lib/types";
import styles from "./Upload.module.scss";

/** The files attached to a post or message: images as thumbnails, the rest as links. */
export function Attachments({ files }: { files: Attachment[] }) {
  if (files.length === 0) return null;
  const images = files.filter(isImage);
  const others = files.filter((f) => !isImage(f));
  return (
    <div className={styles.attachments} aria-label="Attachments">
      {images.length > 0 && (
        <ul className={styles.thumbs}>
          {images.map((file) => (
            <li key={file.id}>
              <a href={fileUrl(file)} target="_blank" rel="noopener noreferrer" title={file.name}>
                {/* User uploads on the backend's host: skip Next's image optimizer, like Avatar. */}
                <Image src={fileUrl(file)} alt={file.name} width={160} height={120} unoptimized />
              </a>
            </li>
          ))}
        </ul>
      )}
      {others.length > 0 && (
        <ul className={styles.fileList}>
          {others.map((file) => (
            <li key={file.id}>
              <a className={styles.fileLink} href={fileUrl(file)} target="_blank" rel="noopener noreferrer">
                <span className={styles.fileName}>{file.name}</span>
                <span className={styles.fileSize}>{formatSize(file.size)}</span>
              </a>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
