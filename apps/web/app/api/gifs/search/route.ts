import type { GifSearchResponse } from "@/lib/contracts";

const API_BASE_URL = process.env.SPILLIO_API_URL ?? "http://127.0.0.1:4000";

export async function GET(request: Request) {
  const { searchParams } = new URL(request.url);
  const query = String(searchParams.get("q") ?? "").trim();
  const page = Math.max(0, Number(searchParams.get("page") ?? 0) || 0);
  const kind = String(searchParams.get("kind") ?? "gif");

  if (!query) {
    return Response.json({ results: [], degraded: false } satisfies GifSearchResponse);
  }

  try {
    const response = await fetch(`${API_BASE_URL}/api/gifs/search?q=${encodeURIComponent(query)}&kind=${encodeURIComponent(kind)}&page=${page}`, {
      cache: "no-store",
    });
    if (!response.ok) {
      throw new Error(`GIF search failed with ${response.status}`);
    }
    return Response.json((await response.json()) as GifSearchResponse);
  } catch {
    return Response.json({ results: [], degraded: true } satisfies GifSearchResponse);
  }
}
