import * as React from "react";

/** Hairline separator, optionally with a tracked centered label. */
export interface DividerProps {
  /** Centered mono label, e.g. "OR". */
  label?: string;
  /** @default "horizontal" */
  orientation?: "horizontal" | "vertical";
  style?: React.CSSProperties;
}

export function Divider(props: DividerProps): JSX.Element;
