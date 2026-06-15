"use client";

import { useRef, useState } from "react";
import { GifPickerOverlay } from "@/components/gif-picker-overlay";
import { useGifDraft } from "./gif-draft";

export function GifSearchPicker({ columnTitle }: { columnTitle: string }) {
  const { selectedGif, selectGif } = useGifDraft();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  function suppressCardAutosubmit(event: React.PointerEvent<HTMLElement>) {
    suppressFormAutosubmit(event.currentTarget.closest("form"));
  }

  function suppressFormAutosubmit(form: HTMLFormElement | null) {
    if (!form) {
      return;
    }
    form.dataset.suppressCardAutosubmit = "1";
    window.setTimeout(() => {
      delete form.dataset.suppressCardAutosubmit;
    }, 500);
  }

  function submitCardIfLeavingForm(event: React.FocusEvent<HTMLElement>) {
    // The picker lives in a portal outside the form, so focus moving into it
    // looks like leaving the form. Never auto-submit while it is open.
    if (open) {
      return;
    }
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
    <div className="relative" onBlurCapture={submitCardIfLeavingForm} onPointerDown={suppressCardAutosubmit} ref={rootRef}>
      <button
        className={`inline-flex h-7 items-center justify-center gap-1.5 rounded-full border border-white/35 px-2.5 text-[11px] font-extrabold uppercase tracking-[0.06em] text-white/85 shadow-[0_1px_2px_rgba(0,0,0,0.12)] transition hover:bg-white/15 ${open ? "bg-white/15" : "bg-white/10"}`}
        onClick={(event) => {
          suppressFormAutosubmit(event.currentTarget.form);
          setOpen((value) => !value);
        }}
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
          onClose={() => {
            // Closing must not submit: it would re-create or duplicate the card.
            suppressFormAutosubmit(rootRef.current?.closest("form") ?? null);
            setOpen(false);
          }}
          placeholder={`search ${columnTitle}`}
          selected={(gif) => selectedGif?.id === gif.id}
          title="Pick a GIF"
          renderResult={(gif, selected, className, image) => (
            <label aria-label={`Choose card GIF: ${gif.alt_text || "GIF"}`} className={`${className} grid cursor-pointer ${selected ? "border-spill-wrong ring-2 ring-spill-wrong/45" : "border-spill-line"}`} key={`${gif.id}-${gif.url}`}>
              <input
                checked={selected}
                className="sr-only"
                onChange={() => {
                  // Stage the gif onto the open card and close; the user confirms
                  // with the card's ✓ (or the single blur-autosubmit) so a gif
                  // never spawns its own standalone card.
                  suppressFormAutosubmit(rootRef.current?.closest("form") ?? null);
                  selectGif(gif);
                  setOpen(false);
                }}
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
