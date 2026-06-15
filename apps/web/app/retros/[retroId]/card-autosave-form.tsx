"use client";

import type { ComponentProps, FocusEvent, ReactNode } from "react";
import { cardFormHasContent, requestCardSubmit } from "@/lib/card-submit";

export function CardAutosaveForm({
  action,
  className,
  children,
}: {
  action: ComponentProps<"form">["action"];
  className?: string;
  children: ReactNode;
}) {
  // Single blur owner for the whole card. When focus truly leaves the form — and
  // not into the GIF picker's portaled overlay (rendered outside the form DOM but
  // marked with data-gif-overlay) — autosave if there is anything to save.
  function handleBlur(event: FocusEvent<HTMLFormElement>) {
    const form = event.currentTarget;
    const next = event.relatedTarget;
    if (next instanceof HTMLElement && (form.contains(next) || next.closest("[data-gif-overlay]"))) {
      return;
    }
    if (cardFormHasContent(form)) {
      requestCardSubmit(form);
    }
  }

  return (
    <form action={action} className={className} onBlurCapture={handleBlur}>
      {children}
    </form>
  );
}
