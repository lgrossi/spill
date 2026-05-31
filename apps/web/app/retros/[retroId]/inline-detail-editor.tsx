"use client";

import { useState, useTransition } from "react";
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
  const [currentValue, setCurrentValue] = useState(value);
  const [isPending, startTransition] = useTransition();

  function save(nextValue: string) {
    const trimmed = nextValue.trim();
    if (!trimmed || trimmed === value) {
      setCurrentValue(value);
      return;
    }
    const formData = new FormData();
    formData.set("retro_id", retroId);
    formData.set("return_to", returnTo);
    formData.set(field, trimmed);
    startTransition(() => {
      void updateRetroDetailsAction(formData);
    });
  }

  return (
    <input
      aria-label={label}
      className="min-w-0 rounded-none border-0 border-b border-dashed border-spill-muted/70 bg-transparent px-0.5 py-0 text-[inherit] font-[inherit] leading-[inherit] tracking-[inherit] text-spill-fg outline-none transition placeholder:text-spill-muted/70 focus:border-spill-fg focus:bg-[rgba(207,138,63,0.08)] disabled:cursor-wait disabled:opacity-70"
      disabled={isPending}
      onBlur={(event) => save(event.currentTarget.value)}
      onChange={(event) => setCurrentValue(event.currentTarget.value)}
      onKeyDown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          event.currentTarget.blur();
        }
        if (event.key === "Escape") {
          setCurrentValue(value);
          event.currentTarget.blur();
        }
      }}
      value={currentValue}
    />
  );
}
