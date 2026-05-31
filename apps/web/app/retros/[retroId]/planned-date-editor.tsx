"use client";

import { useRef, useState, useTransition } from "react";
import { rescheduleRetroAction } from "@/lib/actions";
import { formatDateOnly } from "@/lib/retro-dates";

export function PlannedDateEditor({
  plannedFor,
  retroId,
}: {
  plannedFor: string;
  retroId: string;
}) {
  const inputRef = useRef<HTMLInputElement | null>(null);
  const [value, setValue] = useState(plannedFor);
  const [isPending, startTransition] = useTransition();

  function openPicker() {
    const input = inputRef.current;
    if (!input || isPending) return;
    const picker = input as HTMLInputElement & { showPicker?: () => void };
    if (picker.showPicker) {
      picker.showPicker();
      return;
    }
    input.focus();
    input.click();
  }

  return (
    <span className="relative inline-flex align-baseline">
      <button
        aria-label={`Change planned retro date, currently ${formatDateOnly(value)}`}
        className="inline rounded-[5px] border-b border-dashed border-spill-muted/70 px-0.5 text-[inherit] font-extrabold leading-none tracking-[-0.03em] text-spill-fg decoration-spill-muted underline-offset-[5px] transition hover:border-spill-fg hover:bg-[rgba(207,138,63,0.08)] focus-visible:outline-none focus-visible:shadow-[var(--focus)] disabled:cursor-wait disabled:opacity-70"
        disabled={isPending}
        onClick={openPicker}
        title="Change planned date"
        type="button"
      >
        {formatDateOnly(value)}
      </button>
      <input
        aria-hidden="true"
        className="pointer-events-none absolute h-px w-px opacity-0"
        onChange={(event) => {
          const plannedForValue = event.currentTarget.value;
          if (!plannedForValue) {
            event.currentTarget.value = value;
            return;
          }
          setValue(plannedForValue);
          const formData = new FormData();
          formData.set("retro_id", retroId);
          formData.set("planned_for", plannedForValue);
          startTransition(() => {
            void rescheduleRetroAction(formData);
          });
        }}
        ref={inputRef}
        required
        tabIndex={-1}
        type="date"
        value={value}
      />
    </span>
  );
}
