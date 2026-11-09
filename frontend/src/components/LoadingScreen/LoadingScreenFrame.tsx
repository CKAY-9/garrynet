import type { ReactNode } from "react";
import styles from "./LoadingScreen.module.scss";

type Props = {
  children: ReactNode;
  label: string;
  /** Fill the whole viewport (in game) instead of a 16:9 box inside a page (previews). */
  fullscreen?: boolean;
};

/** The box a `<LoadingScreen>` fills. */
export function LoadingScreenFrame({ children, label, fullscreen = false }: Props) {
  return (
    <figure className={fullscreen ? styles.fullscreen : styles.frame} aria-label={label}>
      {children}
    </figure>
  );
}
