import * as React from "react";

/** Hover/focus label on a dark slab. Wraps its trigger element. */
export interface TooltipProps {
  label: string;
  /** @default "top" */
  placement?: "top" | "bottom" | "left" | "right";
  children: React.ReactNode;
  style?: React.CSSProperties;
}

export function Tooltip(props: TooltipProps): JSX.Element;
