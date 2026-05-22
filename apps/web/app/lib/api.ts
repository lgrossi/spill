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
