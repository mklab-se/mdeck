import * as React from "react";

/** Single-choice radio with ember center dot. Group by shared `name`. */
export interface RadioProps {
  label?: string;
  description?: string;
  checked?: boolean;
  disabled?: boolean;
  name?: string;
  value?: string;
  onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
  id?: string;
  style?: React.CSSProperties;
}

export function Radio(props: RadioProps): JSX.Element;
