import * as React from "react";

/** Selectable / removable chip for filters, skills, categories. */
export interface TagProps {
  children?: React.ReactNode;
  /** Renders in the ember-quiet selected state. @default false */
  selected?: boolean;
  /** Adds a remove (×) affordance; called when clicked. */
  onRemove?: (e: React.MouseEvent) => void;
  onClick?: (e: React.MouseEvent) => void;
  style?: React.CSSProperties;
}

export function Tag(props: TagProps): JSX.Element;
