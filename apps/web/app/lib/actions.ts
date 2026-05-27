"use server";

import { redirect } from "next/navigation";
import { searchDirectory, type DirectoryUser } from "./directory";
import {
  createDraftCardCommand,
  deleteDraftCardCommand,
  removeClusterMemberCommand,
  updateDraftCardCommand,
} from "./commands/card-commands";
import {
  confirmActionItemCommand,
  completeActionItemCommand,
  proposeActionItemCommand,
  rejectActionItemCommand,
  updateActionItemCommand,
} from "./commands/action-commands";
import {
  castVoteCommand,
  completeRetroCommand,
  markReadyCommand,
  removeVoteCommand,
  revealRetroCommand,
  startActionDiscussionCommand,
  startVotingCommand,
  unmarkReadyCommand,
} from "./commands/board-phase-commands";
import { createRetroCommand } from "./commands/retro-commands";
import {
  createDeliveryCommand,
  createMeetingNoteCommand,
  retryDeliveryCommand,
} from "./commands/delivery-commands";
import { clearLocalIdentity, setLocalIdentity } from "./identity";
import { addGrant, listGrants, removeGrant, type Grant } from "./api";

export type { Grant };

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
  return createRetroCommand(formData);
}

export async function createDraftCardAction(formData: FormData) {
  return createDraftCardCommand(formData);
}

export async function updateDraftCardAction(formData: FormData) {
  return updateDraftCardCommand(formData);
}

export async function removeClusterMemberAction(formData: FormData) {
  return removeClusterMemberCommand(formData);
}

export async function markReadyAction(formData: FormData) {
  return markReadyCommand(formData);
}

export async function unmarkReadyAction(formData: FormData) {
  return unmarkReadyCommand(formData);
}

export async function revealRetroAction(formData: FormData) {
  return revealRetroCommand(formData);
}

export async function startVotingAction(formData: FormData) {
  return startVotingCommand(formData);
}

export async function castVoteAction(formData: FormData) {
  return castVoteCommand(formData);
}

export async function removeVoteAction(formData: FormData) {
  return removeVoteCommand(formData);
}

export async function startActionDiscussionAction(formData: FormData) {
  return startActionDiscussionCommand(formData);
}

export async function updateActionItemAction(formData: FormData) {
  return updateActionItemCommand(formData);
}

export async function confirmActionItemAction(formData: FormData) {
  return confirmActionItemCommand(formData);
}

export async function completeActionItemAction(formData: FormData) {
  return completeActionItemCommand(formData);
}

export async function rejectActionItemAction(formData: FormData) {
  return rejectActionItemCommand(formData);
}

export async function proposeActionItemAction(formData: FormData) {
  return proposeActionItemCommand(formData);
}

export async function deleteDraftCardAction(formData: FormData) {
  return deleteDraftCardCommand(formData);
}

export async function completeRetroAction(formData: FormData) {
  return completeRetroCommand(formData);
}

export async function createMeetingNoteAction(formData: FormData) {
  return createMeetingNoteCommand(formData);
}

export async function createDeliveryAction(formData: FormData) {
  return createDeliveryCommand(formData);
}

export async function retryDeliveryAction(formData: FormData) {
  return retryDeliveryCommand(formData);
}

export async function searchDirectoryAction(query: string): Promise<DirectoryUser[]> {
  return searchDirectory(query);
}

export async function listGrantsAction(retroId: string): Promise<Grant[]> {
  return listGrants(retroId);
}

export async function addGrantAction(retroId: string, email: string): Promise<void> {
  return addGrant(retroId, email);
}

export async function removeGrantAction(retroId: string, email: string): Promise<void> {
  return removeGrant(retroId, email);
}
