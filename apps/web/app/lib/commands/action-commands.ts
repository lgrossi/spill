"use server";

import { revalidatePath } from "next/cache";
import { completeActionItem, confirmActionItem, proposeActionItem, rejectActionItem, updateActionItem } from "../api";

export async function updateActionItemCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const actionId = field(formData, "action_id");
  const title = field(formData, "title").trim();
  const details = field(formData, "details").trim();

  await updateActionItem(retroId, actionId, title, details);
  revalidatePath(`/retros/${retroId}`);
}

export async function confirmActionItemCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const actionId = field(formData, "action_id");

  await confirmActionItem(retroId, actionId);
  revalidatePath(`/retros/${retroId}`);
}

export async function completeActionItemCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const actionId = field(formData, "action_id");

  await completeActionItem(retroId, actionId);
  revalidatePath(`/retros/${retroId}`);
  revalidatePath("/");
}

export async function rejectActionItemCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const actionId = field(formData, "action_id");

  await rejectActionItem(retroId, actionId);
  revalidatePath(`/retros/${retroId}`);
}

export async function proposeActionItemCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const actionId = field(formData, "action_id");

  await proposeActionItem(retroId, actionId);
  revalidatePath(`/retros/${retroId}`);
}

function field(formData: FormData, name: string) {
  return String(formData.get(name) ?? "");
}
