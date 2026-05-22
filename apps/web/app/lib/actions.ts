"use server";

import { redirect } from "next/navigation";
import { revalidatePath } from "next/cache";
import { acceptDeckItem, castVote, clusterBoard, completeRetro, confirmActionItem, createDraftCard, createRetro, markReady, rejectActionItem, retryAiJob, revealRetro, searchGifs, startActionDiscussion, startAiJob, startVoting, type AiArtifact, type CreateRetroPayload, updateActionItem } from "./api";

export async function createRetroAction(formData: FormData) {
  const template = String(formData.get("template") ?? "standard");
  const title = String(formData.get("title") ?? "").trim();
  const voteLimit = Number(formData.get("vote_limit") ?? 3);
  const actionDiscussionLimit = Number(formData.get("action_discussion_limit") ?? 3);

  const payload: CreateRetroPayload =
    template === "custom"
      ? {
          title,
          template: "custom",
          columns: String(formData.get("columns") ?? "")
            .split("\n")
            .map((column) => column.trim())
            .filter(Boolean),
          vote_limit: voteLimit,
          action_discussion_limit: actionDiscussionLimit,
        }
      : {
          title,
          template: "standard",
          vote_limit: voteLimit,
          action_discussion_limit: actionDiscussionLimit,
        };

  const board = await createRetro(payload);
  redirect(`/retros/${board.retro.id}`);
}

export async function createDraftCardAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const columnId = String(formData.get("column_id") ?? "");
  const bodyText = String(formData.get("body_text") ?? "").trim();
  const gifChoice = parseGifChoice(String(formData.get("gif_choice") ?? ""));

  await createDraftCard(retroId, columnId, bodyText, gifChoice?.url, gifChoice?.altText);
  revalidatePath(`/retros/${retroId}`);
}

export async function searchGifsAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const query = String(formData.get("gif_query") ?? "").trim();
  const results = query ? await searchGifs(query) : { results: [], degraded: false };

  redirect(`/retros/${retroId}?gif=${encodeURIComponent(query)}&gifDegraded=${results.degraded ? "1" : "0"}&gifResults=${encodeURIComponent(JSON.stringify(results.results))}`);
}

function parseGifChoice(value: string): { url: string; altText: string } | null {
  if (!value) {
    return null;
  }
  try {
    const parsed = JSON.parse(value);
    if (typeof parsed?.url === "string" && typeof parsed?.altText === "string") {
      return parsed;
    }
  } catch {
    return null;
  }
  return null;
}

export async function markReadyAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await markReady(retroId);
  revalidatePath(`/retros/${retroId}`);
}

export async function revealRetroAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await revealRetro(retroId);
  revalidatePath(`/retros/${retroId}`);
}

export async function startVotingAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await startVoting(retroId);
  revalidatePath(`/retros/${retroId}`);
}

export async function castVoteAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const cardId = String(formData.get("card_id") ?? "");

  await castVote(retroId, cardId, 1);
  revalidatePath(`/retros/${retroId}`);
}

export async function clusterBoardAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await clusterBoard(retroId);
  revalidatePath(`/retros/${retroId}`);
}

export async function startActionDiscussionAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await startActionDiscussion(retroId);
  revalidatePath(`/retros/${retroId}`);
}

export async function updateActionItemAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const actionId = String(formData.get("action_id") ?? "");
  const title = String(formData.get("title") ?? "").trim();
  const details = String(formData.get("details") ?? "").trim();

  await updateActionItem(retroId, actionId, title, details);
  revalidatePath(`/retros/${retroId}`);
}

export async function confirmActionItemAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const actionId = String(formData.get("action_id") ?? "");

  await confirmActionItem(retroId, actionId);
  revalidatePath(`/retros/${retroId}`);
}

export async function rejectActionItemAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const actionId = String(formData.get("action_id") ?? "");

  await rejectActionItem(retroId, actionId);
  revalidatePath(`/retros/${retroId}`);
}

export async function completeRetroAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await completeRetro(retroId);
  revalidatePath(`/retros/${retroId}`);
  revalidatePath("/history");
}

export async function acceptDeckItemAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const itemId = String(formData.get("item_id") ?? "");
  const columnId = String(formData.get("column_id") ?? "");

  await acceptDeckItem(retroId, itemId, columnId);
  revalidatePath(`/retros/${retroId}`);
}

export async function startAiJobAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const kind = String(formData.get("kind") ?? "") as AiArtifact["kind"];
  const fail = formData.get("fail") === "on";

  await startAiJob(retroId, kind, fail);
  revalidatePath(`/retros/${retroId}`);
}

export async function retryAiJobAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const artifactId = String(formData.get("artifact_id") ?? "");

  await retryAiJob(retroId, artifactId);
  revalidatePath(`/retros/${retroId}`);
}
