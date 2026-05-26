import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import type { GifResult, GifSearchResponse } from "../../../lib/api";

const KLIPY_LIMIT = 8;

type KlipyResponse = {
  data?: KlipyResult[];
};

type KlipyResult = {
  id: string;
  title?: string;
  alt_text?: string;
  images?: {
    original?: KlipyImage;
    fixed_width?: KlipyImage;
    downsized?: KlipyImage;
    preview_gif?: KlipyImage;
  };
};

type KlipyImage = {
  url?: string;
  webp?: string;
  mp4?: string;
};

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const query = String(searchParams.get("q") ?? "").trim();
  const page = Math.max(0, Number(searchParams.get("page") ?? 0) || 0);

  if (!query) {
    return Response.json({ results: [], degraded: false } satisfies GifSearchResponse);
  }

  const directResults = await searchKlipy(query, page);
  if (directResults) {
    return Response.json({ results: directResults, degraded: false } satisfies GifSearchResponse);
  }

  return Response.json({ results: [], degraded: true } satisfies GifSearchResponse);
}

async function searchKlipy(query: string, page: number): Promise<GifResult[] | null> {
  const apiKey = await getKlipyApiKey();
  if (!apiKey) {
    return null;
  }

  try {
    const response = await fetch(
      `https://api.klipy.com/v2/gifs/search?q=${encodeURIComponent(query)}&key=${encodeURIComponent(apiKey)}&limit=${KLIPY_LIMIT}&offset=${page * KLIPY_LIMIT}`,
      { cache: "no-store" },
    );
    if (!response.ok) {
      return null;
    }
    const payload = (await response.json()) as KlipyResponse;
    const results = (payload.data ?? []).map((item): GifResult | null => {
      const original = item.images?.original;
      const preview = item.images?.fixed_width ?? item.images?.downsized ?? item.images?.preview_gif ?? original;
      const url = original?.url;
      if (!url) {
        return null;
      }
      return {
        id: `klipy-${item.id}`,
        url,
        preview_url: preview?.webp ?? preview?.url ?? url,
        alt_text: item.alt_text || item.title || `${query} GIF`,
        media_type: "image",
        kind: "gif",
      };
    }).filter((item): item is GifResult => Boolean(item));
    return results.length ? results : null;
  } catch {
    return null;
  }
}

async function getKlipyApiKey() {
  if (process.env.SPILLIO_KLIPY_API_KEY) {
    return process.env.SPILLIO_KLIPY_API_KEY;
  }

  try {
    const envFile = await readFile(resolve(process.cwd(), "../../.env"), "utf8");
    const line = envFile.split(/\r?\n/).find((entry) => entry.startsWith("SPILLIO_KLIPY_API_KEY="));
    return line?.slice("SPILLIO_KLIPY_API_KEY=".length).trim() || undefined;
  } catch {
    return undefined;
  }
}
