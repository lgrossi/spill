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
  unresolved_action_count: number;
  recurring_tags: string[];
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
  vote_count: number;
  current_user_vote_count: number;
  cluster_id: string | null;
  cluster_title: string | null;
  cluster_category: string | null;
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
  voting: {
    vote_limit: number;
    votes_used: number;
    votes_remaining: number;
  };
  clusters: {
    id: string;
    retro_id: string;
    title: string | null;
    category: string | null;
    tags: string[];
  }[];
  actions: RetroActionItem[];
  deck: IngestedItem[];
  ai_artifacts: AiArtifact[];
  meeting_notes: MeetingNote[];
};

export type RetroActionItem = {
  id: string;
  retro_id: string;
  source_card_id: string | null;
  source_cluster_id: string | null;
  title: string;
  details: string | null;
  status: "proposed" | "confirmed" | "rejected";
  position: number;
  tags: string[];
};

export type IngestedItem = {
  id: string;
  retro_id: string | null;
  source: "pi" | "claude_code" | "upload" | "other";
  placement: "user_deck" | "retro_draft";
  target_column_id: string | null;
  suggested_text: string | null;
  gif_url: string | null;
  status: "pending" | "accepted" | "dismissed";
};

export type AiArtifact = {
  id: string;
  retro_id: string;
  kind: "gif_suggestions" | "clustering" | "action_suggestions" | "summary" | "mood" | "tagging";
  status: "pending" | "running" | "succeeded" | "failed";
  input: unknown;
  output: unknown | null;
  error_message: string | null;
  retry_count: number;
};

export type MeetingNote = {
  id: string;
  retro_id: string;
  title: string;
  body_text: string;
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

export async function startVoting(retroId: string): Promise<RetroBoard> {
  return apiFetch(`/api/retros/${retroId}/voting/start`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function castVote(retroId: string, cardId: string, count = 1): Promise<RetroBoard["voting"]> {
  return apiFetch(`/api/retros/${retroId}/votes`, {
    method: "POST",
    body: JSON.stringify({ card_id: cardId, count }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function clusterBoard(retroId: string): Promise<RetroBoard> {
  return apiFetch(`/api/retros/${retroId}/cluster`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function startActionDiscussion(retroId: string): Promise<RetroBoard> {
  return apiFetch(`/api/retros/${retroId}/actions/start`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function updateActionItem(retroId: string, actionId: string, title: string, details: string): Promise<RetroActionItem> {
  return apiFetch(`/api/retros/${retroId}/actions/${actionId}`, {
    method: "PATCH",
    body: JSON.stringify({ title, details: details || null }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function confirmActionItem(retroId: string, actionId: string): Promise<RetroActionItem> {
  return apiFetch(`/api/retros/${retroId}/actions/${actionId}/confirm`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function rejectActionItem(retroId: string, actionId: string): Promise<RetroActionItem> {
  return apiFetch(`/api/retros/${retroId}/actions/${actionId}/reject`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function completeRetro(retroId: string): Promise<RetroBoard> {
  return apiFetch(`/api/retros/${retroId}/complete`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function acceptDeckItem(retroId: string, itemId: string, columnId: string): Promise<RetroCard> {
  return apiFetch(`/api/retros/${retroId}/deck/${itemId}/accept`, {
    method: "POST",
    body: JSON.stringify({ column_id: columnId }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function startAiJob(retroId: string, kind: AiArtifact["kind"], fail = false): Promise<AiArtifact> {
  return apiFetch(`/api/retros/${retroId}/ai-jobs`, {
    method: "POST",
    body: JSON.stringify({ kind, fail }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function retryAiJob(retroId: string, artifactId: string): Promise<AiArtifact> {
  return apiFetch(`/api/retros/${retroId}/ai-jobs/${artifactId}/retry`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function createMeetingNote(retroId: string, title: string, bodyText: string): Promise<MeetingNote> {
  return apiFetch(`/api/retros/${retroId}/meeting-notes`, {
    method: "POST",
    body: JSON.stringify({ title: title || null, body_text: bodyText }),
    headers: { "content-type": "application/json" },
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
