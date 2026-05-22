const API_BASE_URL = process.env.SPILLIO_API_URL ?? "http://127.0.0.1:4000";
const DEV_USER_SUBJECT = process.env.SPILLIO_DEV_USER_SUBJECT ?? "local-dev";
const DEV_USER_NAME = process.env.SPILLIO_DEV_USER_NAME ?? "Local Dev";

export type RetroSummary = {
  id: string;
  title: string;
  phase: "writing" | "discussion" | "voting" | "action_discussion" | "completed";
  vote_limit: number;
  action_discussion_limit: number;
  participant_count: number;
  column_count: number;
};

export type RetroOverview = {
  active: RetroSummary[];
  completed: RetroSummary[];
};

export type RetroColumn = {
  id: string;
  retro_id: string;
  column_key: string;
  title: string;
  position: number;
  order_direction: string;
  cards: RetroCard[];
};

export type RetroCard = {
  id: string;
  retro_id: string;
  column_id: string;
  body_text: string | null;
  gif_url: string | null;
  gif_alt_text: string | null;
  state: "draft" | "revealed";
  position: number;
  hidden: boolean;
};

export type GifResult = {
  id: string;
  url: string;
  preview_url: string;
  alt_text: string;
};

export type GifSearchResponse = {
  results: GifResult[];
  degraded: boolean;
};

export type RetroBoard = {
  retro: {
    id: string;
    title: string;
    phase: RetroSummary["phase"];
    vote_limit: number;
    action_discussion_limit: number;
  };
  columns: RetroColumn[];
  ready: {
    participant_count: number;
    ready_count: number;
    current_user_ready: boolean;
  };
};

export type CreateRetroPayload =
  | {
      title: string;
      template: "standard";
      vote_limit: number;
      action_discussion_limit: number;
    }
  | {
      title: string;
      template: "custom";
      columns: string[];
      vote_limit: number;
      action_discussion_limit: number;
    };

export async function listRetros(): Promise<RetroOverview> {
  return apiFetch("/api/retros", { cache: "no-store" });
}

export async function createRetro(payload: CreateRetroPayload): Promise<RetroBoard> {
  return apiFetch("/api/retros", {
    method: "POST",
    body: JSON.stringify(payload),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function getRetro(retroId: string): Promise<RetroBoard> {
  return apiFetch(`/api/retros/${retroId}`, { cache: "no-store" });
}

export async function createDraftCard(retroId: string, columnId: string, bodyText: string, gifUrl?: string, gifAltText?: string): Promise<RetroCard> {
  return apiFetch(`/api/retros/${retroId}/cards`, {
    method: "POST",
    body: JSON.stringify({
      column_id: columnId,
      body_text: bodyText || null,
      gif_url: gifUrl || null,
      gif_alt_text: gifAltText || null,
    }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function searchGifs(query: string): Promise<GifSearchResponse> {
  return apiFetch(`/api/gifs/search?q=${encodeURIComponent(query)}`, {
    cache: "no-store",
  });
}

export async function markReady(retroId: string): Promise<RetroBoard> {
  return apiFetch(`/api/retros/${retroId}/ready`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function revealRetro(retroId: string): Promise<RetroBoard> {
  return apiFetch(`/api/retros/${retroId}/reveal`, {
    method: "POST",
    cache: "no-store",
  });
}

async function apiFetch<T>(path: string, init: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers: {
      "x-spillio-user-subject": DEV_USER_SUBJECT,
      "x-spillio-user-name": DEV_USER_NAME,
      ...init.headers,
    },
  });

  if (!response.ok) {
    const body = await response.json().catch(() => null);
    const message = body?.error?.message ?? `SpillItOut API request failed with ${response.status}`;
    throw new Error(message);
  }

  return response.json() as Promise<T>;
}
