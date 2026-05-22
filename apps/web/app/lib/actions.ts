"use server";

import { redirect } from "next/navigation";
import { revalidatePath } from "next/cache";
import { createDraftCard, createRetro, markReady, revealRetro, type CreateRetroPayload } from "./api";

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

  await createDraftCard(retroId, columnId, bodyText);
  revalidatePath(`/retros/${retroId}`);
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
