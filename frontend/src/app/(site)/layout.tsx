import { SiteShell } from "@/components/SiteShell";

/** Every website page: side navigation plus content. */
export default function SiteLayout({ children }: LayoutProps<"/">) {
  return <SiteShell>{children}</SiteShell>;
}
