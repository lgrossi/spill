"use client";

import { createContext, useContext, useEffect, useState } from "react";
import type { ReactNode } from "react";

type GifResult = {
  id: string;
  url: string;
  preview_url: string;
  alt_text: string;
};

type GifSearchResponse = {
  results: GifResult[];
  degraded: boolean;
};

type GifSelection = {
  id: string;
  url: string;
  preview_url: string;
  alt_text: string;
};

type GifDraftContextValue = {
  selectedGif: GifSelection | null;
  removed: boolean;
  hasSelectedGif: boolean;
  selectGif: (gif: GifSelection) => void;
  removeGif: () => void;
};

const GifDraftContext = createContext<GifDraftContextValue | null>(null);

export function GifDraftProvider({ children, initialGif }: { children: ReactNode; initialGif?: GifSelection | null }) {
  const [selectedGif, setSelectedGif] = useState<GifSelection | null>(initialGif ?? null);
  const [removed, setRemoved] = useState(false);

  return (
    <GifDraftContext.Provider
      value={{
        selectedGif,
        removed,
        hasSelectedGif: Boolean(selectedGif),
        selectGif: (gif) => {
          setSelectedGif(gif);
          setRemoved(false);
        },
        removeGif: () => {
          setSelectedGif(null);
          setRemoved(true);
        },
      }}
    >
      {children}
    </GifDraftContext.Provider>
  );
}

export function GifSelectedPreview() {
  const { selectedGif, removed, removeGif } = useGifDraft();

  return (
    <>
      {selectedGif ? (
        <>
          <input name="gif_choice" type="hidden" value={JSON.stringify({ url: selectedGif.url, altText: selectedGif.alt_text })} />
          <div className="relative mb-2 aspect-video w-full min-w-0 max-w-full overflow-hidden rounded-[6px] bg-black/15 shadow-[inset_0_0_0_1px_rgba(255,255,255,0.14)]">
            <img alt="" className="block h-full w-full min-w-0 max-w-full object-contain" loading="lazy" src={selectedGif.preview_url || selectedGif.url} />
            <button aria-label="Remove selected GIF" className="absolute right-2 top-2 grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/35 text-[13px] font-extrabold leading-none text-white/90 shadow-[0_1px_2px_rgba(0,0,0,0.16)] transition hover:bg-black/45" onClick={removeGif} type="button">×</button>
          </div>
        </>
      ) : null}
      {removed ? <input name="gif_remove" type="hidden" value="1" /> : null}
    </>
  );
}

export function GifSearchPicker({ columnTitle }: { columnTitle: string }) {
  const { selectedGif, selectGif } = useGifDraft();
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<GifResult[]>([]);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(false);
  const [degraded, setDegraded] = useState(false);

  useEffect(() => {
    const trimmed = query.trim();
    setPage(0);

    if (!open || trimmed.length < 2) {
      setResults([]);
      setLoading(false);
      setDegraded(false);
      return;
    }

    const controller = new AbortController();
    const timer = window.setTimeout(async () => {
      void searchPage(trimmed, 0, controller.signal);
    }, 260);

    return () => {
      window.clearTimeout(timer);
      controller.abort();
    };
  }, [query, open]);

  async function searchPage(trimmed: string, nextPage: number, signal?: AbortSignal) {
    setLoading(true);
    try {
      const response = await fetch(`/api/gifs/search?q=${encodeURIComponent(trimmed)}&kind=gif&page=${nextPage}`, {
        signal,
      });
      if (!response.ok) {
        throw new Error("GIF search failed");
      }
      const payload = (await response.json()) as GifSearchResponse;
      const incoming = payload.results.slice(0, 8);
      setResults((current) => nextPage === 0 ? incoming : [...current, ...incoming]);
      setPage(nextPage);
      setDegraded(payload.degraded);
    } catch {
      if (!signal?.aborted) {
        if (nextPage === 0) {
          setResults([]);
        }
        setDegraded(true);
      }
    } finally {
      if (!signal?.aborted) {
        setLoading(false);
      }
    }
  }

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
                  onClick={() => {
                    const trimmed = query.trim();
                    if (trimmed.length >= 2) {
                      void searchPage(trimmed, page + 1);
                    }
                  }}
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

export function useHasSelectedGif() {
  return useGifDraft().hasSelectedGif;
}

export function useGifDraftStatus() {
  const { hasSelectedGif, removed } = useGifDraft();
  return { hasSelectedGif, removed };
}

function useGifDraft() {
  const context = useContext(GifDraftContext);
  if (!context) {
    throw new Error("GIF draft controls must be rendered inside GifDraftProvider");
  }
  return context;
}
