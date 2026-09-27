import * as React from "react";

/** Modal overlay; backdrop blurs the room behind it. Controlled. */
export interface DialogProps {
  open: boolean;
  onClose?: () => void;
  title?: string;
  description?: string;
  children?: React.ReactNode;
  /** Right-aligned action row (usually Buttons). */
  footer?: React.ReactNode;
  /** Panel width in px. @default 480 */
  width?: number;
  style?: React.CSSProperties;
}

export function Dialog(props: DialogProps): JSX.Element | null;
