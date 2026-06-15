"use client";

import { useRef } from "react";
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
  const pending = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Single blur owner for the whole card. relatedTarget is unreliable when focus
  // crosses into the GIF picker's portaled overlay (it even starts visibility:
  // hidden), so instead defer one tick and decide from where focus actually
  // landed. Autosave only when focus truly left the form and the overlay.
  function handleBlur(event: FocusEvent<HTMLFormElement>) {
    const form = event.currentTarget;
    if (pending.current) {
      clearTimeout(pending.current);
    }
    pending.current = setTimeout(() => {
      pending.current = null;
      if (!form.isConnected) {
        return;
      }
      const active = document.activeElement;
      if (active instanceof HTMLElement && (form.contains(active) || active.closest("[data-gif-overlay]"))) {
        return;
      }
      if (cardFormHasContent(form)) {
        requestCardSubmit(form);
      }
    }, 0);
  }

  return (
    <form action={action} className={className} onBlurCapture={handleBlur}>
      {children}
    </form>
  );
}
