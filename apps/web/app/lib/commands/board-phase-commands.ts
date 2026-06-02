"use server";

import { revalidatePath } from "next/cache";
import { redirect } from "next/navigation";
import {
  castVote,
  completeRetro,
  forceRevealRetro,
  getRetro,
  markReady,
  removeVote,
  revealRetro,
  startScheduledRetro,
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

export async function startScheduledRetroCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await startScheduledRetro(retroId);
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
  await syncReadinessFromVotes(retroId);
  redirect(`/retros/${retroId}`);
}

export async function removeVoteCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const cardId = field(formData, "card_id");

  await removeVote(retroId, cardId);
  await syncReadinessFromVotes(retroId);
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

// Auto-track readiness from vote spend: spending the last vote marks ready,
// un-voting back from zero clears the implicit ready signal.
async function syncReadinessFromVotes(retroId: string) {
  try {
    const board = await getRetro(retroId);
    if (board.retro.phase !== "voting") return;
    const noVotesLeft = board.voting.votes_remaining === 0;
    if (noVotesLeft && !board.ready.current_user_ready) {
      await markReady(retroId);
    } else if (!noVotesLeft && board.ready.current_user_ready) {
      await unmarkReady(retroId);
    }
  } catch {
    // best-effort; the next page render will reflect current state
  }
}

// Fires from the AutoAdvanceCountdown after the 5s grace window.
// Re-checks all-ready server-side so a last-second un-ready cancels the
// transition even if the client's timer already fired. Errors (already
// advanced, race with another client) are swallowed.
export async function autoAdvanceCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  try {
    const board = await getRetro(retroId);
    const allReady =
      board.ready.participant_count > 0 &&
      board.ready.ready_count >= board.ready.participant_count;
    if (allReady) {
      if (board.retro.phase === "writing") {
        await revealRetro(retroId);
      } else if (
        board.retro.phase === "voting" &&
        !(board.retro.clustering_mode === "auto_on_vote_start" && board.retro.clustering_status === "running")
      ) {
        await startActionDiscussion(retroId);
      }
      revalidatePath(`/retros/${retroId}`);
    }
  } catch {
  }
  redirect(`/retros/${retroId}`);
}
