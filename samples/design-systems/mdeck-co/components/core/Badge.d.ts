import * as React from "react";

/** Small mono status pill. */
export interface BadgeProps {
  children?: React.ReactNode;
  /** @default "neutral" */
  tone?: "neutral" | "accent" | "positive" | "warning" | "critical";
  /** @default "soft" */
  variant?: "soft" | "solid";
  /** Leading status dot. @default false */
  dot?: boolean;
  style?: React.CSSProperties;
}

export function Badge(props: BadgeProps): JSX.Element;
