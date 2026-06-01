"use client";

import { useState } from "react";
import { GifPickerOverlay } from "@/components/gif-picker-overlay";
import { useGifDraft } from "./gif-draft";

export function GifSearchPicker({ columnTitle }: { columnTitle: string }) {
  const { selectedGif, selectGif } = useGifDraft();
  const [open, setOpen] = useState(false);

  function suppressCardAutosubmit(event: React.PointerEvent<HTMLElement>) {
    const form = event.currentTarget.closest("form");
    if (!form) {
      return;
    }
    form.dataset.suppressCardAutosubmit = "1";
    window.setTimeout(() => {
      delete form.dataset.suppressCardAutosubmit;
    }, 500);
  }

  function submitCardIfLeavingForm(event: React.FocusEvent<HTMLElement>) {
    const form = event.currentTarget.closest("form");
    const next = event.relatedTarget;
    if (!form || (next instanceof HTMLElement && form.contains(next))) {
      return;
    }
    if (form.dataset.suppressCardAutosubmit === "1") {
      return;
    }
    if (selectedGif) {
      const submitter = form.querySelector<HTMLButtonElement>("[data-intent-card-submit]");
      form.requestSubmit(submitter ?? undefined);
    }
  }

  return (
    <div className="relative" onBlurCapture={submitCardIfLeavingForm} onPointerDown={suppressCardAutosubmit}>
      <button
        className={`inline-flex h-7 items-center justify-center gap-1.5 rounded-full border border-white/35 px-2.5 text-[11px] font-extrabold uppercase tracking-[0.06em] text-white/85 shadow-[0_1px_2px_rgba(0,0,0,0.12)] transition hover:bg-white/15 ${open ? "bg-white/15" : "bg-white/10"}`}
        onClick={() => setOpen((value) => !value)}
        type="button"
      >
        <span>gif</span>
      </button>

      {open ? (
        <GifPickerOverlay
          ariaLabel="Card GIF picker"
          columns="card"
          emptyText="Search for a GIF to add to this card."
          kicker="opened from card"
          onClose={() => setOpen(false)}
          placeholder={`search ${columnTitle}`}
          selected={(gif) => selectedGif?.id === gif.id}
          title="Pick a GIF"
          renderResult={(gif, selected, className, image) => (
            <label className={`${className} grid cursor-pointer ${selected ? "border-spill-wrong ring-2 ring-spill-wrong/45" : "border-spill-line"}`} key={`${gif.id}-${gif.url}`}>
              <input
                checked={selected}
                className="sr-only"
                onChange={() => selectGif(gif)}
                type="radio"
              />
              {selected ? <span className="absolute right-2 top-2 z-10 grid h-5 w-5 place-items-center rounded-full bg-spill-wrong text-[12px] font-extrabold text-white">✓</span> : null}
              {image}
            </label>
          )}
        />
      ) : null}
    </div>
  );
}
