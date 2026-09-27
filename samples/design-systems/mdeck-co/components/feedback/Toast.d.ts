import * as React from "react";

/** Transient notification slab with a tone-colored left accent. */
export interface ToastProps {
  title?: string;
  message?: string;
  /** @default "neutral" */
  tone?: "neutral" | "accent" | "positive" | "warning" | "critical";
  /** Override the default tone glyph (Lucide name). */
  icon?: string;
  onClose?: () => void;
  style?: React.CSSProperties;
}

export function Toast(props: ToastProps): JSX.Element;
