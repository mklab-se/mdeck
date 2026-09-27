import * as React from "react";

/** Square control for a single glyph action (close, more, nav). */
export interface IconButtonProps {
  /** A rendered icon node. Prefer `name` for Lucide glyphs. */
  icon?: React.ReactNode;
  /** Lucide icon name (used if `icon` is not given). */
  name?: string;
  /** @default "ghost" */
  variant?: "ghost" | "secondary" | "primary";
  /** @default "md" */
  size?: "sm" | "md" | "lg";
  disabled?: boolean;
  "aria-label": string;
  onClick?: (e: React.MouseEvent<HTMLButtonElement>) => void;
  style?: React.CSSProperties;
}

export function IconButton(props: IconButtonProps): JSX.Element;
