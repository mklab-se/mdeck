import * as React from "react";

/** Text field: inset well that lights an ember hairline on focus. */
export interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "size"> {
  /** Tracked mono caps label above the field. */
  label?: string;
  /** Helper text below the field. */
  hint?: string;
  /** Error message; overrides hint and turns the field critical. */
  error?: string;
  iconLeft?: React.ReactNode;
  iconRight?: React.ReactNode;
  /** @default "md" */
  size?: "sm" | "md" | "lg";
  disabled?: boolean;
  style?: React.CSSProperties;
}

export function Input(props: InputProps): JSX.Element;
