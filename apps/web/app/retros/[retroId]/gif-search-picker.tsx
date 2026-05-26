"use client";

import { useState } from "react";
import { useGifDraft } from "./gif-draft";
import { useGifSearch } from "./gif-search-data";

export function GifSearchPicker({ columnTitle }: { columnTitle: string }) {
  const { selectedGif, selectGif } = useGifDraft();
  const [open, setOpen] = useState(false);
  const { degraded, loadMore, loading, query, results, setQuery } = useGifSearch(open);

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
    <div className="mt-2 min-w-0" onBlurCapture={submitCardIfLeavingForm} onPointerDown={suppressCardAutosubmit}>
      <button
        className={`flex h-7 w-full items-center justify-between rounded-[7px] border border-white/25 px-2.5 text-[11.5px] font-extrabold text-white/85 transition hover:bg-white/15 ${open ? "rounded-b-none bg-white/15" : "bg-white/10"}`}
        onClick={() => setOpen((value) => !value)}
        type="button"
      >
        <span>GIF</span>
        <span>{open ? "−" : "+"}</span>
      </button>

      {open ? (
        <div className="rounded-b-[7px] border-x border-b border-white/25 bg-white/10 p-2">
          <div className="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-2">
            <span className="rounded-[4px] bg-black/45 px-1.5 py-0.5 text-[9px] font-extrabold tracking-[0.05em] text-white">GIF</span>
            <input
              className="h-7 rounded-[7px] border border-white/25 bg-white px-2.5 text-[11.5px] font-semibold text-[var(--fg-2)] focus:shadow-[var(--focus)]"
              onChange={(event) => setQuery(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  event.stopPropagation();
                  if (selectedGif) {
                    const form = event.currentTarget.form;
                    const submitter = form?.querySelector<HTMLButtonElement>("[data-intent-card-submit]");
                    form?.requestSubmit(submitter ?? undefined);
                  }
                }
              }}
              placeholder={`search ${columnTitle}`}
              type="search"
              value={query}
            />
            <span className="w-12 text-right text-[10.5px] text-white/65">{loading ? "..." : results.length ? `${results.length}` : ""}</span>
          </div>

          {results.length > 0 ? (
            <div className="mt-2">
              <div className="mb-2 flex items-center justify-between gap-2">
                <span className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-white/75">GIF results</span>
                <span className="text-[11px] text-white/65">select to add to card</span>
              </div>
              <div className="sp-scroll grid max-h-[138px] grid-cols-4 gap-2 overflow-y-auto pr-1">
                {results.map((gif) => (
                  <label className="grid cursor-pointer gap-1 rounded-[7px] border border-transparent bg-white p-1 text-[9px] text-spill-fg has-[:checked]:border-white has-[:checked]:shadow-[0_0_0_2px_rgba(255,255,255,0.35)]" key={`${gif.id}-${gif.url}`}>
                    <input
                      checked={selectedGif?.id === gif.id}
                      className="sr-only"
                      onChange={() => selectGif(gif)}
                      type="radio"
                    />
                    <img alt="" className="h-10 w-full rounded object-cover" loading="lazy" src={gif.preview_url || gif.url} />
                    <span className="truncate">{gif.alt_text}</span>
                  </label>
                ))}
              </div>
              <div className="mt-2 flex items-center gap-2">
                <button
                  className="inline-flex h-8 items-center justify-center gap-1.5 rounded-[7px] border border-white/45 bg-white/15 px-3 text-[11.5px] font-extrabold leading-none text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.12),0_1px_2px_rgba(0,0,0,0.1)] transition hover:bg-white/25 disabled:pointer-events-none disabled:opacity-50"
                  disabled={loading || degraded}
                  onClick={loadMore}
                  type="button"
                >
                  <span className="text-[15px] leading-none">+</span>
                  more
                </button>
              </div>
            </div>
          ) : null}

          {degraded ? <p className="mt-1.5 text-[11px] text-white/80">GIF search unavailable.</p> : null}
        </div>
      ) : null}
    </div>
  );
}
