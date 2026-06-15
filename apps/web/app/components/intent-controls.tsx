"use client";

import { useRef } from "react";
import type { CSSProperties, ReactNode } from "react";
import { cardFormHasContent, requestCardSubmit } from "@/lib/card-submit";

export function IntentSearch({
  className,
  defaultValue,
  name,
  placeholder,
  style,
}: {
  className: string;
  defaultValue?: string;
  name: string;
  placeholder?: string;
  style?: CSSProperties;
}) {
  const timer = useRef<number | null>(null);

  return (
    <input
      className={className}
      defaultValue={defaultValue}
      name={name}
      onInput={(event) => {
        if (timer.current) {
          window.clearTimeout(timer.current);
        }
        const form = event.currentTarget.form;
        timer.current = window.setTimeout(() => form?.requestSubmit(), 220);
      }}
      placeholder={placeholder}
      style={style}
    />
  );
}

export function IntentSelect({
  children,
  className,
  defaultValue,
  name,
  style,
}: {
  children: ReactNode;
  className: string;
  defaultValue?: string;
  name: string;
  style?: CSSProperties;
}) {
  return (
    <select className={className} defaultValue={defaultValue} name={name} onChange={(event) => event.currentTarget.form?.requestSubmit()} style={style}>
      {children}
    </select>
  );
}

export function IntentCardText({
  className,
  defaultValue,
  name,
  placeholder,
  rows,
}: {
  className: string;
  defaultValue?: string;
  name: string;
  placeholder?: string;
  rows?: number;
}) {
  return (
    <textarea
      className={className}
      defaultValue={defaultValue}
      name={name}
      onKeyDown={(event) => {
        if (event.key === "Enter" && !event.shiftKey) {
          event.preventDefault();
          const form = event.currentTarget.form;
          if (form && cardFormHasContent(form)) {
            requestCardSubmit(form);
          }
        }
      }}
      placeholder={placeholder}
      rows={rows}
    />
  );
}
