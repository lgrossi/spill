"use client";

import { useState } from "react";
import type { GifResult, GifSearchResponse } from "@/lib/contracts";

export function RetroCoverPicker({
  initialAltText,
  initialUrl,
  title,
}: {
  initialAltText: string;
  initialUrl: string;
  title: string;
}) {
  const [selected, setSelected] = useState<{ url: string; altText: string } | null>(
    initialUrl ? { url: initialUrl, altText: initialAltText } : null,
  );
  const [query, setQuery] = useState(title);
  const [results, setResults] = useState<GifResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [degraded, setDegraded] = useState(false);

  async function search() {
    const trimmed = query.trim();
    if (trimmed.length < 2) return;
    setLoading(true);
    setDegraded(false);
    try {
      const response = await fetch(`/api/gifs/search?q=${encodeURIComponent(trimmed)}&kind=gif&page=0`);
      if (!response.ok) throw new Error("GIF search failed");
      const payload = (await response.json()) as GifSearchResponse;
      setResults(payload.results.slice(0, 9));
      setDegraded(payload.degraded);
    } catch {
      setResults([]);
      setDegraded(true);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="grid gap-2">
      <input name="cover_gif_url" type="hidden" value={selected?.url ?? ""} />
      <input name="cover_gif_alt_text" type="hidden" value={selected?.altText ?? ""} />
      <div className="flex items-center gap-2">
        {selected ? (
          <img alt={selected.altText} className="h-12 w-12 shrink-0 rounded-[10px] border border-spill-line object-cover" src={selected.url} />
        ) : (
          <span className="grid h-12 w-12 shrink-0 place-items-center rounded-[10px] border border-dashed border-spill-line text-[10px] font-extrabold text-spill-muted">GIF</span>
        )}
        <div className="min-w-0 flex-1">
          <p className="truncate text-[11px] font-semibold text-spill-fg">{selected?.altText || "No cover selected"}</p>
          {selected ? (
            <button className="mt-1 text-[10.5px] font-extrabold text-spill-muted hover:text-spill-wrong" onClick={() => setSelected(null)} type="button">
              clear cover
            </button>
          ) : null}
        </div>
      </div>
      <div className="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
        <input
          className="rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 py-2 text-[12px] font-semibold text-spill-fg"
          onChange={(event) => setQuery(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void search();
            }
          }}
          placeholder="search GIFs"
          type="search"
          value={query}
        />
        <button className="rounded-[8px] border border-spill-line bg-[var(--paper)] px-3 text-[11px] font-extrabold text-spill-fg" disabled={loading} onClick={search} type="button">
          {loading ? "..." : "search"}
        </button>
      </div>
      {results.length > 0 ? (
        <div className="grid max-h-[220px] grid-cols-3 gap-2 overflow-y-auto pr-1">
          {results.map((gif) => (
            <button
              className="grid gap-1 rounded-[7px] border border-spill-line bg-white p-1 text-left text-[9px] text-spill-fg transition hover:border-spill-wrong"
              key={`${gif.id}-${gif.url}`}
              onClick={() => setSelected({ url: gif.url, altText: gif.alt_text })}
              type="button"
            >
              <img alt="" className="h-16 w-full rounded object-cover" loading="lazy" src={gif.preview_url || gif.url} />
              <span className="truncate">{gif.alt_text}</span>
            </button>
          ))}
        </div>
      ) : null}
      {degraded ? <p className="text-[11px] text-spill-muted">GIF search unavailable. Try again later.</p> : null}
    </div>
  );
}
