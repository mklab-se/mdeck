import * as React from "react";

/**
 * Thin wrapper over Lucide icons (loaded from CDN).
 * Requires `<script src="https://unpkg.com/lucide@latest"></script>` on the page.
 */
export interface IconProps {
  /** Lucide icon name, e.g. "arrow-right", "check", "search". */
  name: string;
  /** Pixel size (square). @default 18 */
  size?: number;
  /** @default 1.5 */
  strokeWidth?: number;
  color?: string;
  style?: React.CSSProperties;
}

export function Icon(props: IconProps): JSX.Element;
