import * as React from "react";

export interface TabItem {
  value: string;
  label: string;
  /** Optional trailing count (mono). */
  count?: number;
}

/** Underline tab navigation; active tab lights an ember bar. Controlled. */
export interface TabsProps {
  tabs: TabItem[];
  value: string;
  onChange?: (value: string) => void;
  style?: React.CSSProperties;
}

export function Tabs(props: TabsProps): JSX.Element;
