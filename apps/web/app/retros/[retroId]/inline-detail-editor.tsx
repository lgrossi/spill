"use client";

import { useRef, useState, useTransition } from "react";
import { updateRetroDetailsAction } from "@/lib/actions";

export function InlineDetailEditor({
  field,
  label,
  retroId,
  value,
  returnTo,
}: {
  field: "title" | "group_name";
  label: string;
  retroId: string;
  value: string;
  returnTo: string;
}) {
  const ref = useRef<HTMLSpanElement | null>(null);
  const [currentValue, setCurrentValue] = useState(value);
  const [isPending, startTransition] = useTransition();

  function save(nextValue: string) {
    const trimmed = nextValue.trim();
    if (!trimmed || trimmed === value) {
      setCurrentValue(value);
      if (ref.current) ref.current.textContent = value;
      return;
    }
    setCurrentValue(trimmed);
    const formData = new FormData();
    formData.set("retro_id", retroId);
    formData.set("return_to", returnTo);
    formData.set(field, trimmed);
    startTransition(() => {
      void updateRetroDetailsAction(formData);
    });
  }

  return (
    <span
      aria-label={label}
      className={`inline-block min-w-[8ch] max-w-full border-b border-dashed border-spill-muted/70 px-0.5 text-[inherit] font-[inherit] leading-[inherit] tracking-[inherit] text-spill-fg outline-none transition focus:border-spill-fg ${isPending ? "cursor-wait opacity-70" : "cursor-text"}`}
      contentEditable={!isPending}
      onBlur={(event) => save(event.currentTarget.textContent ?? "")}
      onInput={(event) => setCurrentValue(event.currentTarget.textContent ?? "")}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          event.currentTarget.blur();
        }
        if (event.key === "Escape") {
          setCurrentValue(value);
          event.currentTarget.textContent = value;
          event.currentTarget.blur();
        }
      }}
      ref={ref}
      role="textbox"
      style={{ width: `${Math.max(currentValue.length + 1, 8)}ch` }}
      suppressContentEditableWarning
    >
      {currentValue}
    </span>
  );
}
