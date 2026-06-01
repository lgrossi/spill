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
    <div className="relative" onBlurCapture={submitCardIfLeavingForm} onPointerDown={suppressCardAutosubmit}>
      <button
        className={`inline-flex h-7 items-center justify-center gap-1.5 rounded-full border border-white/35 px-2.5 text-[11px] font-extrabold uppercase tracking-[0.06em] text-white/85 shadow-[0_1px_2px_rgba(0,0,0,0.12)] transition hover:bg-white/15 ${open ? "bg-white/15" : "bg-white/10"}`}
        onClick={() => setOpen((value) => !value)}
        type="button"
      >
        <span>gif</span>
      </button>

      {open ? (
        <div className="sp-panel-grain absolute left-0 top-full z-50 mt-2 w-[min(390px,calc(100vw-2rem))] rounded-[14px] border border-[var(--line-2)] bg-spill-panel p-3.5 text-spill-fg shadow-[var(--shadow-3)]">
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">opened from card</p>
              <h2 className="mt-1 text-[17px] font-extrabold tracking-[-0.02em] text-spill-fg">Pick a GIF</h2>
            </div>
            <button className="grid h-7 w-7 place-items-center rounded-[8px] border border-spill-line bg-[var(--panel-hi)] text-[16px] font-extrabold leading-none text-spill-muted transition hover:text-spill-fg" onClick={() => setOpen(false)} type="button">
              ×
            </button>
          </div>
          <div className="mt-3 grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2">
            <input
              autoFocus
              className="h-9 rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 text-[12.5px] font-bold text-spill-fg shadow-[inset_0_1px_0_rgba(255,255,255,0.55)] focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
              onChange={(event) => setQuery(event.currentTarget.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") {
                  event.preventDefault();
                  event.stopPropagation();
                }
              }}
              placeholder={`search ${columnTitle}`}
              type="search"
              value={query}
            />
            <span className="w-12 text-right text-[10.5px] font-semibold text-spill-muted">{loading ? "..." : results.length ? `${results.length}` : ""}</span>
          </div>

          {results.length > 0 ? (
            <div className="mt-3">
              <div className="sp-scroll grid max-h-[280px] grid-cols-3 gap-2 overflow-y-auto pr-1">
                {results.map((gif) => (
                  <label className={`relative grid aspect-square cursor-pointer overflow-hidden rounded-[9px] border bg-[var(--panel-hi)] p-1 shadow-[var(--shadow-1)] transition hover:-translate-y-0.5 hover:shadow-[var(--shadow-2)] ${selectedGif?.id === gif.id ? "border-spill-wrong ring-2 ring-spill-wrong/45" : "border-spill-line"}`} key={`${gif.id}-${gif.url}`}>
                    <input
                      checked={selectedGif?.id === gif.id}
                      className="sr-only"
                      onChange={() => selectGif(gif)}
                      type="radio"
                    />
                    {selectedGif?.id === gif.id ? <span className="absolute right-2 top-2 z-10 grid h-5 w-5 place-items-center rounded-full bg-spill-wrong text-[12px] font-extrabold text-white">✓</span> : null}
                    <img alt="" className="h-full w-full rounded-[7px] object-cover" loading="lazy" src={gif.preview_url || gif.url} />
                  </label>
                ))}
              </div>
              <div className="mt-2 flex items-center gap-2">
                <button
                  className="inline-flex h-8 items-center justify-center gap-1.5 rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 text-[11.5px] font-extrabold leading-none text-spill-fg transition hover:border-spill-wrong/50 disabled:pointer-events-none disabled:opacity-50"
                  disabled={loading || degraded}
                  onClick={loadMore}
                  type="button"
                >
                  <span className="text-[15px] leading-none">+</span>
                  more
                </button>
              </div>
            </div>
          ) : (
            <div className="mt-3 rounded-[10px] border border-dashed border-spill-line bg-[var(--panel-hi)] p-4 text-center text-[12px] font-semibold text-spill-muted">
              {query.trim().length < 2 ? "Search for a GIF to add to this card." : loading ? "Searching..." : "No GIFs found."}
            </div>
          )}

          {degraded ? <p className="mt-2 text-[11px] font-semibold text-spill-wrong">GIF search unavailable.</p> : null}
        </div>
      ) : null}
    </div>
  );
}
