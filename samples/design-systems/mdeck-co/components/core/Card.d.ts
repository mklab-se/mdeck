import * as React from "react";

/**
 * Graphite surface panel.
 * @startingPoint section="Core" subtitle="Panels: default, inset, outline & ember-glow" viewport="700x260"
 */
export interface CardProps {
  children?: React.ReactNode;
  /** @default "default" */
  variant?: "default" | "inset" | "outline" | "glow";
  /** Lifts + gains a stronger border on hover. @default false */
  interactive?: boolean;
  /** @default "lg" */
  padding?: "none" | "sm" | "md" | "lg";
  onClick?: (e: React.MouseEvent<HTMLDivElement>) => void;
  style?: React.CSSProperties;
}

export function Card(props: CardProps): JSX.Element;
