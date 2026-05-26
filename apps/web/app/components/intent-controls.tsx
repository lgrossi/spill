"use client";

import { useRef } from "react";
import type { CSSProperties, ReactNode } from "react";

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
  function submitIfComplete(target: HTMLTextAreaElement) {
    const form = target.form;
    if (form?.dataset.suppressCardAutosubmit === "1") {
      return;
    }
    const hasGif = Boolean(form?.querySelector<HTMLInputElement>('input[name="gif_choice"]:checked'));
    const existingGif = form?.querySelector<HTMLInputElement>('input[name="existing_gif_url"]')?.value.trim();
    if (target.value.trim() || hasGif || existingGif) {
      const submitter = form?.querySelector<HTMLButtonElement>("[data-intent-card-submit]");
      form?.requestSubmit(submitter ?? undefined);
    }
  }

  return (
    <textarea
      className={className}
      defaultValue={defaultValue}
      name={name}
      onBlur={(event) => {
        const next = event.relatedTarget;
        if (next instanceof HTMLElement && event.currentTarget.form?.contains(next)) {
          return;
        }
        submitIfComplete(event.currentTarget);
      }}
      onKeyDown={(event) => {
        if (event.key === "Enter" && !event.shiftKey) {
          event.preventDefault();
          submitIfComplete(event.currentTarget);
        }
      }}
      placeholder={placeholder}
      rows={rows}
    />
  );
}
