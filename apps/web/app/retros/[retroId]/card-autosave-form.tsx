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

  function cancelPending() {
    if (pending.current) {
      clearTimeout(pending.current);
      pending.current = null;
    }
  }

  function saveIfDirty(form: HTMLFormElement) {
    if (cardFormHasContent(form)) {
      requestCardSubmit(form);
    }
  }

  // Single blur owner for the whole card.
  function handleBlur(event: FocusEvent<HTMLFormElement>) {
    const form = event.currentTarget;
    const next = event.relatedTarget;
    // Definite exit: focus moved to a real element outside the form and outside
    // the GIF overlay. Save synchronously, so a click that also unmounts the
    // editor (route or phase change) still persists the draft.
    if (next instanceof HTMLElement && !form.contains(next) && !next.closest("[data-gif-overlay]")) {
      cancelPending();
      saveIfDirty(form);
      return;
    }
    // Ambiguous (null relatedTarget, or focus into the form/overlay — e.g. the
    // picker opening, whose portaled panel even starts visibility:hidden): defer
    // one tick and decide from where focus actually settled, since relatedTarget
    // is unreliable across the portal.
    cancelPending();
    pending.current = setTimeout(() => {
      pending.current = null;
      if (!form.isConnected) {
        return;
      }
      const active = document.activeElement;
      if (active instanceof HTMLElement && (form.contains(active) || active.closest("[data-gif-overlay]"))) {
        return;
      }
      saveIfDirty(form);
    }, 0);
  }

  return (
    <form action={action} className={className} onBlurCapture={handleBlur}>
      {children}
    </form>
  );
}
