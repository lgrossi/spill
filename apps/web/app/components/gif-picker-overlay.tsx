"use client";

import { useEffect, useRef } from "react";
import type { CSSProperties } from "react";
import type { ReactNode } from "react";
import type { GifResult } from "@/lib/contracts";
import { useGifSearch } from "@/retros/[retroId]/gif-search-data";

export function GifPickerOverlay({
  ariaLabel,
  columns = "cover",
  emptyText,
  kicker,
  onClose,
  placeholder,
  renderResult,
  selected,
  title,
}: {
  ariaLabel: string;
  columns?: "cover" | "card";
  emptyText: string;
  kicker: string;
  onClose: () => void;
  placeholder: string;
  renderResult: (gif: GifResult, selected: boolean, className: string, image: ReactNode) => ReactNode;
  selected: (gif: GifResult) => boolean;
  title: string;
}) {
  const { degraded, hasMore, loadMore, loading, query, results, setQuery } = useGifSearch(true);
  const scrollRef = useRef<HTMLDivElement>(null);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const gridColumns = columns === "cover" ? "grid-cols-2 sm:grid-cols-4" : "grid-cols-3";
  const maxHeight = columns === "cover" ? "max-h-[330px]" : "max-h-[280px]";

  useEffect(() => {
    const root = scrollRef.current;
    const sentinel = sentinelRef.current;
    if (!root || !sentinel || !hasMore || loading || degraded || results.length === 0) {
      return;
    }

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry?.isIntersecting) {
          loadMore();
        }
      },
      { root, rootMargin: "96px" },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [degraded, hasMore, loadMore, loading, results.length]);

  return (
    <div
      className="sp-panel-grain absolute left-0 top-full z-[200] mt-3 rounded-[14px] border border-[var(--line-2)] bg-spill-panel p-4 text-spill-fg shadow-[var(--shadow-3)]"
      style={{ width: "min(470px, calc(100vw - 2rem))" } as CSSProperties}
      role="region"
      aria-label={ariaLabel}
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">{kicker}</p>
          <h2 className="mt-1 text-[19px] font-extrabold tracking-[-0.02em] text-spill-fg">{title}</h2>
        </div>
        <button className="grid h-8 w-8 place-items-center rounded-[8px] border border-spill-line bg-[var(--panel-hi)] text-[18px] font-extrabold leading-none text-spill-muted transition hover:text-spill-fg" onClick={onClose} type="button">
          x
        </button>
      </div>

      <input
        autoFocus
        className="mt-3 h-10 w-full rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 text-[13px] font-bold text-spill-fg shadow-[inset_0_1px_0_rgba(255,255,255,0.55)] focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
        onChange={(event) => setQuery(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            event.stopPropagation();
          }
        }}
        placeholder={placeholder}
        type="search"
        value={query}
      />

      {results.length > 0 ? (
          <div className={`sp-scroll mt-3 grid ${maxHeight} ${gridColumns} gap-2 overflow-y-auto pr-1`} ref={scrollRef}>
            {results.map((gif) =>
              renderResult(
                gif,
                selected(gif),
                "relative aspect-square w-full overflow-hidden rounded-[9px] border bg-[var(--panel-hi)] p-1 shadow-[var(--shadow-1)] transition hover:-translate-y-0.5 hover:shadow-[var(--shadow-2)]",
                <img alt="" className="h-full w-full rounded-[7px] object-cover" loading="lazy" src={gif.preview_url || gif.url} />,
              )
            )}
            {hasMore ? (
              <div className="col-span-full py-1 text-center text-[11px] font-semibold text-spill-muted" ref={sentinelRef}>
                {loading ? "Loading..." : "Scroll for more"}
              </div>
            ) : null}
          </div>
      ) : (
        <div className="mt-3 rounded-[10px] border border-dashed border-spill-line bg-[var(--panel-hi)] p-4 text-center text-[12px] font-semibold text-spill-muted">
          {query.trim().length < 2 ? emptyText : loading ? "Searching..." : "No GIFs found."}
        </div>
      )}

      {degraded ? <p className="mt-2 text-[11px] font-semibold text-spill-wrong">GIF search unavailable.</p> : null}
    </div>
  );
}
