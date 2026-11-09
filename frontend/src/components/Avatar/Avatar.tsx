import Image from "next/image";
import styles from "./Avatar.module.scss";

type Props = {
  /** Used for the initial when there is no image. */
  name: string;
  url: string | null;
  /** Pixels. */
  size: number;
  shape?: "rounded" | "circle";
  className?: string;
};

/**
 * An image from an admin-supplied URL (community logo, key figure picture), or the name's
 * initial on a tile if there is none. These URLs can point anywhere, so they skip Next's
 * image optimizer, which only accepts allow-listed hosts.
 */
export function Avatar({ name, url, size, shape = "rounded", className = "" }: Props) {
  const classes = `${styles.avatar} ${styles[shape]} ${className}`;
  if (url) {
    return <Image className={classes} src={url} alt="" width={size} height={size} unoptimized />;
  }
  return (
    <span
      className={`${classes} ${styles.initial}`}
      style={{ width: size, height: size, fontSize: size * 0.45 }}
      aria-hidden
    >
      {name.trim().charAt(0).toUpperCase() || "?"}
    </span>
  );
}
