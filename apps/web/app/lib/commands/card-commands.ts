"use server";

import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import { createDraftCard, deleteDraftCard, removeClusterMember, updateDraftCard } from "../api";

export async function createDraftCardCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const columnId = field(formData, "column_id");
  const bodyText = field(formData, "body_text").trim();
  const gifChoice = parseGifChoice(field(formData, "gif_choice"));

  await createDraftCard(retroId, columnId, bodyText, gifChoice?.url, gifChoice?.altText);
  revalidatePath(`/retros/${retroId}`);
  redirect(`/retros/${retroId}?addColumn=${encodeURIComponent(columnId)}`);
}

export async function updateDraftCardCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const columnId = field(formData, "column_id");
  const cardId = field(formData, "card_id");
  const bodyText = field(formData, "body_text").trim();
  const editingGroupTitle = formData.get("editing_group_title") === "1";
  const clusterDetails = field(formData, "cluster_details").trim();
  const gifChoice = parseGifChoice(field(formData, "gif_choice"));
  const removeGif = formData.get("gif_remove") === "1";
  const existingGifUrl = field(formData, "existing_gif_url").trim();
  const existingGifAltText = field(formData, "existing_gif_alt_text").trim();
  const gifUrl = removeGif ? undefined : (gifChoice?.url ?? existingGifUrl) || undefined;
  const gifAltText = removeGif ? undefined : (gifChoice?.altText ?? existingGifAltText) || undefined;

  await updateDraftCard(retroId, cardId, bodyText, gifUrl, gifAltText, clusterDetails);
  revalidatePath(`/retros/${retroId}`);
  if (editingGroupTitle) {
    redirect(`/retros/${retroId}`);
  }
  redirect(`/retros/${retroId}?addColumn=${encodeURIComponent(columnId)}`);
}

export async function removeClusterMemberCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const cardId = field(formData, "card_id");

  await removeClusterMember(retroId, cardId);
  revalidatePath(`/retros/${retroId}`);
  redirect(`/retros/${retroId}`);
}

export async function deleteDraftCardCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const cardId = field(formData, "card_id");

  await deleteDraftCard(retroId, cardId);
  revalidatePath(`/retros/${retroId}`);
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

function field(formData: FormData, name: string) {
  return String(formData.get(name) ?? "");
}
