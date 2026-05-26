"use server";

import { redirect } from "next/navigation";
import { revalidatePath } from "next/cache";
import { castVote, completeActionItem, completeRetro, confirmActionItem, createDelivery, createDraftCard, createMeetingNote, createRetro, deleteDraftCard, markReady, proposeActionItem, rejectActionItem, removeClusterMember, removeVote, retryDelivery, revealRetro, startActionDiscussion, startVoting, type CreateRetroPayload, type Delivery, unmarkReady, updateActionItem, updateDraftCard } from "./api";
import { clearLocalIdentity, setLocalIdentity } from "./identity";

export async function setIdentityAction(formData: FormData) {
  const email = String(formData.get("email") ?? "");
  const displayName = String(formData.get("display_name") ?? "");
  const returnTo = safeReturnTo(String(formData.get("return_to") ?? "/"));

  await setLocalIdentity(email, displayName);
  redirect(returnTo);
}

export async function clearIdentityAction(formData: FormData) {
  const returnTo = safeReturnTo(String(formData.get("return_to") ?? "/"));

  await clearLocalIdentity();
  redirect(returnTo);
}

function safeReturnTo(value: string) {
  return value.startsWith("/") && !value.startsWith("//") ? value : "/";
}

export async function createRetroAction(formData: FormData) {
  const template = String(formData.get("template") ?? "standard");
  const title = String(formData.get("title") ?? "").trim();
  const votingEnabled = formData.getAll("voting_enabled").at(-1) !== "0";
  const voteLimit = votingEnabled ? Number(formData.get("vote_limit") ?? 3) : 0;
  const actionDiscussionEnabled = formData.getAll("action_discussion_enabled").at(-1) === "1";
  const actionDiscussionLimit = actionDiscussionEnabled ? Number(formData.get("action_discussion_limit") ?? 3) : 0;

  const customColumns = formData
    .getAll("custom_column")
    .map((column) => String(column).trim())
    .filter(Boolean);
  const customColumnColors = formData.getAll("custom_column_color").map((color) => String(color).trim());
  const standardColumns = ["How are you feeling?", "Went well", "To improve"];
  const standardColors = ["#0f5f72", "#2f9469", "#cf4f4f"];
  const fourLColumns = ["Liked", "Lacked", "Learned", "Longed for"];
  const fourLColors = ["#2f9469", "#cf4f4f", "#0f5f72", "#cf8a3f"];
  const withActionColumn = (columns: string[], colors: string[]) => {
    if (!actionDiscussionEnabled) return { columns, colors };
    const pairs = columns.map((column, index) => ({ column, color: colors[index] ?? "#cf8a3f" })).filter((item) => item.column.toLowerCase() !== "actions");
    return { columns: [...pairs.map((item) => item.column), "Actions"], colors: [...pairs.map((item) => item.color), "#8757b6"] };
  };
  const custom = withActionColumn(customColumns, customColumnColors);
  const standard = withActionColumn(standardColumns, standardColors);
  const fourLs = withActionColumn(fourLColumns, fourLColors);

  const payload: CreateRetroPayload =
    template === "sailboat"
      ? {
          title,
          template: "custom",
          columns: ["Wind", "Anchor", "Rocks", "Island"],
          vote_limit: voteLimit,
          action_discussion_limit: actionDiscussionLimit,
        }
      : template === "ssc"
      ? {
          title,
          template: "custom",
          columns: ["Start", "Stop", "Continue"],
          vote_limit: voteLimit,
          action_discussion_limit: actionDiscussionLimit,
        }
      : template === "msg" || template === "4ls"
      ? {
          title,
          template: "custom",
          columns: template === "4ls" ? fourLs.columns : ["Mad", "Sad", "Glad"],
          column_colors: template === "4ls" ? fourLs.colors : ["#cf4f4f", "#cf4f4f", "#2f9469"],
          vote_limit: voteLimit,
          action_discussion_limit: actionDiscussionLimit,
        }
      : template === "custom"
      ? {
          title,
          template: "custom",
          columns: custom.columns,
          column_colors: custom.colors,
          vote_limit: voteLimit,
          action_discussion_limit: actionDiscussionLimit,
        }
      : {
          title,
          template: "custom",
          columns: standard.columns,
          column_colors: standard.colors,
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
  redirect(`/retros/${retroId}?addColumn=${encodeURIComponent(columnId)}`);
}

export async function updateDraftCardAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const columnId = String(formData.get("column_id") ?? "");
  const cardId = String(formData.get("card_id") ?? "");
  const bodyText = String(formData.get("body_text") ?? "").trim();
  const editingGroupTitle = formData.get("editing_group_title") === "1";
  const clusterDetails = String(formData.get("cluster_details") ?? "").trim();
  const gifChoice = parseGifChoice(String(formData.get("gif_choice") ?? ""));
  const removeGif = formData.get("gif_remove") === "1";
  const existingGifUrl = String(formData.get("existing_gif_url") ?? "").trim();
  const existingGifAltText = String(formData.get("existing_gif_alt_text") ?? "").trim();
  const gifUrl = removeGif ? undefined : (gifChoice?.url ?? existingGifUrl) || undefined;
  const gifAltText = removeGif ? undefined : (gifChoice?.altText ?? existingGifAltText) || undefined;

  await updateDraftCard(retroId, cardId, bodyText, gifUrl, gifAltText, clusterDetails);
  revalidatePath(`/retros/${retroId}`);
  if (editingGroupTitle) {
    redirect(`/retros/${retroId}`);
  }
  redirect(`/retros/${retroId}?addColumn=${encodeURIComponent(columnId)}`);
}

export async function removeClusterMemberAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const cardId = String(formData.get("card_id") ?? "");

  await removeClusterMember(retroId, cardId);
  revalidatePath(`/retros/${retroId}`);
  redirect(`/retros/${retroId}`);
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
  redirect(`/retros/${retroId}`);
}

export async function unmarkReadyAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await unmarkReady(retroId);
  redirect(`/retros/${retroId}`);
}

export async function revealRetroAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await revealRetro(retroId);
  redirect(`/retros/${retroId}`);
}

export async function startVotingAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await startVoting(retroId);
  redirect(`/retros/${retroId}`);
}

export async function castVoteAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const cardId = String(formData.get("card_id") ?? "");

  await castVote(retroId, cardId, 1);
  redirect(`/retros/${retroId}`);
}

export async function removeVoteAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const cardId = String(formData.get("card_id") ?? "");

  await removeVote(retroId, cardId);
  redirect(`/retros/${retroId}`);
}

export async function startActionDiscussionAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await startActionDiscussion(retroId);
  redirect(`/retros/${retroId}`);
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

export async function completeActionItemAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const actionId = String(formData.get("action_id") ?? "");

  await completeActionItem(retroId, actionId);
  revalidatePath(`/retros/${retroId}`);
  revalidatePath("/");
}

export async function rejectActionItemAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const actionId = String(formData.get("action_id") ?? "");

  await rejectActionItem(retroId, actionId);
  revalidatePath(`/retros/${retroId}`);
}

export async function proposeActionItemAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const actionId = String(formData.get("action_id") ?? "");

  await proposeActionItem(retroId, actionId);
  revalidatePath(`/retros/${retroId}`);
}

export async function deleteDraftCardAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const cardId = String(formData.get("card_id") ?? "");

  await deleteDraftCard(retroId, cardId);
  revalidatePath(`/retros/${retroId}`);
}

export async function completeRetroAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");

  await completeRetro(retroId);
  revalidatePath("/history");
  redirect(`/retros/${retroId}`);
}

export async function createMeetingNoteAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const title = String(formData.get("title") ?? "").trim();
  const bodyText = String(formData.get("body_text") ?? "").trim();

  await createMeetingNote(retroId, title, bodyText);
  revalidatePath(`/retros/${retroId}`);
}

export async function createDeliveryAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const kind = String(formData.get("kind") ?? "") as Delivery["kind"];
  const fail = formData.get("fail") === "on";

  await createDelivery(retroId, kind, fail);
  revalidatePath(`/retros/${retroId}`);
}

export async function retryDeliveryAction(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const deliveryId = String(formData.get("delivery_id") ?? "");

  await retryDelivery(retroId, deliveryId);
  revalidatePath(`/retros/${retroId}`);
}
