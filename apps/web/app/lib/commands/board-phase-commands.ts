"use server";

import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import {
  castVote,
  completeRetro,
  forceRevealRetro,
  markReady,
  removeVote,
  revealRetro,
  startActionDiscussion,
  startVoting,
  unmarkReady,
} from "@/lib/api";
import { field } from "./form-utils";

export async function markReadyCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await markReady(retroId);
  redirect(`/retros/${retroId}`);
}

export async function unmarkReadyCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await unmarkReady(retroId);
  redirect(`/retros/${retroId}`);
}

export async function revealRetroCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await revealRetro(retroId);
  redirect(`/retros/${retroId}`);
}

export async function forceRevealRetroCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await forceRevealRetro(retroId);
  redirect(`/retros/${retroId}`);
}

export async function startVotingCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await startVoting(retroId);
  redirect(`/retros/${retroId}`);
}

export async function castVoteCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const cardId = field(formData, "card_id");

  await castVote(retroId, cardId, 1);
  redirect(`/retros/${retroId}`);
}

export async function removeVoteCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const cardId = field(formData, "card_id");

  await removeVote(retroId, cardId);
  redirect(`/retros/${retroId}`);
}

export async function startActionDiscussionCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await startActionDiscussion(retroId);
  redirect(`/retros/${retroId}`);
}

export async function completeRetroCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await completeRetro(retroId);
  revalidatePath("/history");
  redirect(`/retros/${retroId}`);
}
