"use client";

import { useEffect, useState } from "react";

export type GifResult = {
  id: string;
  url: string;
  preview_url: string;
  alt_text: string;
};

type GifSearchResponse = {
  results: GifResult[];
  degraded: boolean;
};

export function useGifSearch(open: boolean) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<GifResult[]>([]);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(false);
  const [degraded, setDegraded] = useState(false);

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

  return {
    degraded,
    loading,
    query,
    results,
    setQuery,
    loadMore: () => {
      const trimmed = query.trim();
      if (trimmed.length >= 2) {
        void searchPage(trimmed, page + 1);
      }
    },
  };
}
