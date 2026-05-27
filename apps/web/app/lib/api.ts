import { apiIdentityHeaders } from "./identity";
export type {
  AiArtifact,
  CreateRetroPayload,
  Delivery,
  GifResult,
  GifSearchResponse,
  Grant,
  IngestedItem,
  MeetingNote,
  RetroActionItem,
  RetroActionSummary,
  RetroBoard,
  RetroCard,
  RetroColumn,
  RetroOverview,
  RetroParticipant,
  RetroPhase,
  RetroSummary,
} from "./contracts";
import type {
  AiArtifact,
  CreateRetroPayload,
  Delivery,
  GifSearchResponse,
  Grant,
  MeetingNote,
  RetroActionItem,
  RetroBoard,
  RetroCard,
  RetroOverview,
} from "./contracts";

const API_BASE_URL = process.env.SPILLIO_API_URL ?? "http://127.0.0.1:4000";

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

export async function searchGifs(query: string, page = 0, kind = "all"): Promise<GifSearchResponse> {
  return apiFetch(`/api/gifs/search?q=${encodeURIComponent(query)}&page=${page}&kind=${encodeURIComponent(kind)}`, {
    cache: "no-store",
  });
}

export async function markReady(retroId: string): Promise<RetroBoard> {
  return apiFetch(`/api/retros/${retroId}/ready`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function unmarkReady(retroId: string): Promise<RetroBoard> {
  return apiFetch(`/api/retros/${retroId}/ready`, {
    method: "DELETE",
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

export async function removeVote(retroId: string, cardId: string): Promise<RetroBoard["voting"]> {
  return apiFetch(`/api/retros/${retroId}/votes/${cardId}`, {
    method: "DELETE",
    cache: "no-store",
  });
}

export async function updateDraftCard(retroId: string, cardId: string, bodyText: string, gifUrl?: string, gifAltText?: string, clusterDetails?: string): Promise<RetroCard> {
  return apiFetch(`/api/retros/${retroId}/cards/${cardId}`, {
    method: "PATCH",
    body: JSON.stringify({
      body_text: bodyText || null,
      gif_url: gifUrl || null,
      gif_alt_text: gifAltText || null,
      cluster_details: clusterDetails || null,
    }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function deleteDraftCard(retroId: string, cardId: string): Promise<void> {
  await apiFetchNoJson(`/api/retros/${retroId}/cards/${cardId}`, {
    method: "DELETE",
    cache: "no-store",
  });
}

export async function removeClusterMember(retroId: string, cardId: string): Promise<RetroCard> {
  return apiFetch(`/api/retros/${retroId}/cards/${cardId}/cluster-member`, {
    method: "DELETE",
    cache: "no-store",
  });
}

export async function moveDraftCard(retroId: string, cardId: string, columnId: string, beforeCardId?: string): Promise<RetroCard> {
  return apiFetch(`/api/retros/${retroId}/cards/${cardId}/move`, {
    method: "PATCH",
    body: JSON.stringify({ column_id: columnId, before_card_id: beforeCardId || null }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function clusterCards(retroId: string, cardId: string, targetCardId: string): Promise<RetroBoard["clusters"][number]> {
  return apiFetch(`/api/retros/${retroId}/cards/${cardId}/cluster`, {
    method: "PATCH",
    body: JSON.stringify({ target_card_id: targetCardId }),
    headers: { "content-type": "application/json" },
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

export async function completeActionItem(retroId: string, actionId: string): Promise<RetroActionItem> {
  return apiFetch(`/api/retros/${retroId}/actions/${actionId}/done`, {
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

export async function proposeActionItem(retroId: string, actionId: string): Promise<RetroActionItem> {
  return apiFetch(`/api/retros/${retroId}/actions/${actionId}/propose`, {
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

export async function createDelivery(retroId: string, kind: Delivery["kind"], fail = false): Promise<Delivery> {
  return apiFetch(`/api/retros/${retroId}/deliveries`, {
    method: "POST",
    body: JSON.stringify({ kind, fail }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function retryDelivery(retroId: string, deliveryId: string): Promise<Delivery> {
  return apiFetch(`/api/retros/${retroId}/deliveries/${deliveryId}/retry`, {
    method: "POST",
    cache: "no-store",
  });
}

export async function listGrants(retroId: string): Promise<Grant[]> {
  return apiFetch(`/api/retros/${retroId}/grants`, { cache: "no-store" });
}

export async function addGrant(retroId: string, email: string): Promise<void> {
  await apiFetchNoJson(`/api/retros/${retroId}/grants`, {
    method: "POST",
    body: JSON.stringify({ email }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export async function removeGrant(retroId: string, email: string): Promise<void> {
  await apiFetchNoJson(`/api/retros/${retroId}/grants/remove`, {
    method: "POST",
    body: JSON.stringify({ email }),
    headers: { "content-type": "application/json" },
    cache: "no-store",
  });
}

export class ApiError extends Error {
  constructor(
    message: string,
    public readonly status: number,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function apiRequest(path: string, init: RequestInit): Promise<Response> {
  const identityHeaders = await apiIdentityHeaders();
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...init,
    headers: {
      ...identityHeaders,
      ...init.headers,
    },
  });

  if (!response.ok) {
    const body = await response.json().catch(() => null);
    const message = body?.error?.message ?? `Spill. API request failed with ${response.status}`;
    throw new ApiError(message, response.status);
  }

  return response;
}

async function apiFetch<T>(path: string, init: RequestInit): Promise<T> {
  return (await apiRequest(path, init)).json() as Promise<T>;
}

async function apiFetchNoJson(path: string, init: RequestInit): Promise<void> {
  await apiRequest(path, init);
}
