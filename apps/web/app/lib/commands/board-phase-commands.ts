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
  revealColumn,
  startScheduledRetro,
  startActionDiscussion,
  startVoting,
  unmarkReady,
  setParticipation,
} from "@/lib/api";
import { applyClustering, retryClustering } from "@/lib/api";
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

export async function setParticipationCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const participantId = field(formData, "participant_id");
  // Hidden marker controlled by the toggle: "1" means participating, "0" means sitting out.
  const isParticipating = String(formData.get("is_participating") ?? "1") === "1";

  await setParticipation(retroId, participantId, isParticipating);
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

export async function revealColumnCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const columnId = field(formData, "column_id");
  // The host-side ready check has already passed by the time this affordance
  // renders; force=true so a late-joiner showing as 'not ready' doesn't block
  // the host (matches the existing force-reveal escape hatch on the global
  // reveal button).
  await revealColumn(retroId, columnId, { force: true });
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

export async function applyClusteringCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await applyClustering(retroId);
  redirect(`/retros/${retroId}`);
}

export async function retryClusteringCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");

  await retryClustering(retroId);
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
        !(
          board.retro.clustering_mode === "auto_on_vote_start" &&
          (board.retro.clustering_status === "computing" ||
            board.retro.clustering_status === "running" ||
            board.retro.clustering_status === "ready")
        )
      ) {
        await startActionDiscussion(retroId);
      }
      revalidatePath(`/retros/${retroId}`);
    }
  } catch {
  }
  redirect(`/retros/${retroId}`);
}
