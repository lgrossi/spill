"use client";

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { ReactNode } from "react";
import type { GifResult } from "@/lib/contracts";
import { useGifSearch } from "@/retros/[retroId]/gif-search-data";

// Margins keep the floating panel clear of the viewport edges; the anchor gap
// matches the previous inline `mt-3` spacing under the trigger.
const VIEWPORT_MARGIN = 8;
const ANCHOR_GAP = 12;
const PANEL_MAX_WIDTH = 470;

export function GifPickerOverlay({
  ariaLabel,
  columns = "cover",
  emptyText,
  kicker,
  onClose,
  renderResult,
  selected,
  title,
}: {
  ariaLabel: string;
  columns?: "cover" | "card";
  emptyText: string;
  kicker: string;
  onClose: () => void;
  renderResult: (gif: GifResult, selected: boolean, className: string, image: ReactNode) => ReactNode;
  selected: (gif: GifResult) => boolean;
  title: string;
}) {
  const { degraded, hasMore, loadMore, loading, query, results, setQuery } = useGifSearch(true);
  const scrollRef = useRef<HTMLDivElement>(null);
  const sentinelRef = useRef<HTMLDivElement>(null);
  const anchorRef = useRef<HTMLSpanElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const [mounted, setMounted] = useState(false);
  const [position, setPosition] = useState<{ top: number; left: number; width: number } | null>(null);
  const gridColumns = columns === "cover" ? "grid-cols-2 sm:grid-cols-4" : "grid-cols-3";
  const maxHeight = columns === "cover" ? "max-h-[330px]" : "max-h-[280px]";

  // Portals require the DOM; only render into document.body after mount.
  useEffect(() => setMounted(true), []);

  // The overlay floats in a body portal anchored to an in-flow marker that sits
  // just under the trigger. Positioning is recomputed on scroll/resize and when
  // the panel changes height (GIF results loading) so it tracks the trigger and
  // never overflows the viewport — fixing both the clipped-in-column and the
  // off-screen last-column cases.
  useLayoutEffect(() => {
    const anchor = anchorRef.current;
    const panel = panelRef.current;
    if (!anchor || !panel) {
      return;
    }

    function reposition() {
      if (!anchor || !panel) {
        return;
      }
      const rect = anchor.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;
      const width = Math.min(PANEL_MAX_WIDTH, viewportWidth - VIEWPORT_MARGIN * 2);
      const left = Math.min(
        Math.max(rect.left, VIEWPORT_MARGIN),
        viewportWidth - width - VIEWPORT_MARGIN,
      );
      const panelHeight = panel.offsetHeight;
      let top = rect.bottom + ANCHOR_GAP;
      if (top + panelHeight > viewportHeight - VIEWPORT_MARGIN) {
        const above = rect.top - ANCHOR_GAP - panelHeight;
        top = above >= VIEWPORT_MARGIN
          ? above
          : Math.max(VIEWPORT_MARGIN, viewportHeight - panelHeight - VIEWPORT_MARGIN);
      }
      setPosition({ top, left, width });
    }

    reposition();
    const resizeObserver = new ResizeObserver(reposition);
    resizeObserver.observe(panel);
    window.addEventListener("scroll", reposition, true);
    window.addEventListener("resize", reposition);
    return () => {
      resizeObserver.disconnect();
      window.removeEventListener("scroll", reposition, true);
      window.removeEventListener("resize", reposition);
    };
  }, [mounted]);

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

  const panel = (
    <div
      ref={panelRef}
      className="sp-panel-grain fixed z-[200] rounded-[14px] border border-[var(--line-2)] bg-spill-panel p-4 text-spill-fg shadow-[var(--shadow-3)]"
      style={{
        top: position?.top ?? 0,
        left: position?.left ?? 0,
        width: position?.width ?? "min(470px, calc(100vw - 2rem))",
        visibility: position ? "visible" : "hidden",
      }}
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
        placeholder="Search KLIPY"
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

  return (
    <>
      <span ref={anchorRef} aria-hidden className="block h-0 w-0" />
      {mounted ? createPortal(panel, document.body) : null}
    </>
  );
}
