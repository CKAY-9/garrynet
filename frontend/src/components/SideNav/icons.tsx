import type { SVGProps } from "react";

function Icon(props: SVGProps<SVGSVGElement>) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={20}
      height={20}
      fill="none"
      stroke="currentColor"
      strokeWidth={2}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      {...props}
    />
  );
}

export const HomeIcon = () => (
  <Icon>
    <path d="M3 11l9-8 9 8" />
    <path d="M5 10v10h14V10" />
  </Icon>
);

export const ForumsIcon = () => (
  <Icon>
    <path d="M21 15a2 2 0 0 1-2 2H8l-5 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
  </Icon>
);

export const StoreIcon = () => (
  <Icon>
    <path d="M6 2L3 6v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V6l-3-4z" />
    <path d="M3 6h18" />
    <path d="M16 10a4 4 0 0 1-8 0" />
  </Icon>
);

export const UsersIcon = () => (
  <Icon>
    <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
    <circle cx="9" cy="7" r="4" />
    <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
    <path d="M16 3.13a4 4 0 0 1 0 7.75" />
  </Icon>
);

export const SearchIcon = () => (
  <Icon width={16} height={16}>
    <circle cx="11" cy="11" r="7" />
    <path d="M21 21l-4.35-4.35" />
  </Icon>
);

export const SupportIcon = () => (
  <Icon>
    <circle cx="12" cy="12" r="10" />
    <path d="M9.1 9a3 3 0 0 1 5.8 1c0 2-3 3-3 3" />
    <path d="M12 17h.01" />
  </Icon>
);

export const AdminIcon = () => (
  <Icon>
    <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
  </Icon>
);

export const LogoutIcon = () => (
  <Icon>
    <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
    <path d="M16 17l5-5-5-5" />
    <path d="M21 12H9" />
  </Icon>
);

/** Simplified Steam mark (filled, not stroked). */
export const SteamIcon = () => (
  <Icon fill="currentColor" stroke="none">
    <path d="M12 2a10 10 0 0 0-9.96 9.1l5.35 2.2a2.8 2.8 0 0 1 1.6-.5h.1l2.4-3.45v-.05a3.75 3.75 0 1 1 3.75 3.75h-.09l-3.4 2.45a2.8 2.8 0 0 1-5.55.45L2.3 14.4A10 10 0 1 0 12 2zm-4.5 15.2l-1.2-.5a2.1 2.1 0 1 0 1.1-2.8l1.25.5a1.55 1.55 0 1 1-1.15 2.8zm9.1-5.7a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5zm0-.95a1.55 1.55 0 1 1 0-3.1 1.55 1.55 0 0 1 0 3.1z" />
  </Icon>
);

export const MenuIcon = () => (
  <Icon>
    <path d="M3 6h18M3 12h18M3 18h18" />
  </Icon>
);

export const CloseIcon = () => (
  <Icon>
    <path d="M18 6L6 18M6 6l12 12" />
  </Icon>
);
