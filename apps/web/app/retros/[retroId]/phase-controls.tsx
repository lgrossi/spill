import {
  completeRetroAction,
  markReadyAction,
  revealRetroAction,
  forceRevealRetroAction,
  startActionDiscussionAction,
  startVotingAction,
  unmarkReadyAction,
} from "@/lib/actions";
import type { RetroBoard } from "@/lib/contracts";
import { Btn, PhaseBadge, Pill, StageIndicator, spillColors } from "@/components/spill-ui";
import { BoardInviteButton } from "@/components/board-invite-button";

export function PhaseControls({
  board,
  isHost = false,
  currentUserEmail = "",
}: {
  board: RetroBoard;
  isHost?: boolean;
  currentUserEmail?: string;
}) {
  const peopleButton = currentUserEmail ? (
    <BoardInviteButton
      retroId={board.retro.id}
      currentUserEmail={currentUserEmail}
      isHost={isHost}
    />
  ) : null;

  if (board.retro.phase === "writing") {
    const allReady = board.ready.participant_count > 0 && board.ready.ready_count >= board.ready.participant_count;
    const someButNotAllReady = board.ready.ready_count > 0 && !allReady;
    // Hosts use forceReveal so they can advance even when not everyone is ready.
    // Non-hosts use the regular reveal which the backend gates on readiness.
    const revealAction = isHost ? forceRevealRetroAction : revealRetroAction;
    return (
      <>
        <StageIndicator phase={board.retro.phase} retroId={board.retro.id} />
        {peopleButton}
        <Pill tone="soft" accent={spillColors.mood}>
          <span className="sp-live-dot h-1.5 w-1.5 bg-spill-mood" />
          writing
        </Pill>
        <form action={board.ready.current_user_ready ? unmarkReadyAction : markReadyAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind={board.ready.current_user_ready ? "secondary" : "primary"} type="submit">{board.ready.current_user_ready ? "not ready" : "I'm ready"}</Btn>
        </form>
        <form action={revealAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind={someButNotAllReady ? "dashed" : "primary"} accent={spillColors.well} type="submit" disabled={!isHost && !allReady}>reveal -&gt;</Btn>
        </form>
      </>
    );
  }

  if (board.retro.phase === "discussion") {
    return (
      <>
        <StageIndicator phase={board.retro.phase} retroId={board.retro.id} />
        {peopleButton}
        <Pill tone="soft" accent={spillColors.action}>review</Pill>
        <form action={startVotingAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind="dashed" type="submit" disabled={board.retro.vote_limit <= 0}>vote -&gt;</Btn>
        </form>
        {board.retro.vote_limit <= 0 ? (
          <form action={startActionDiscussionAction}>
            <input name="retro_id" type="hidden" value={board.retro.id} />
            <Btn kind="primary" accent={spillColors.action} type="submit">act -&gt;</Btn>
          </form>
        ) : null}
      </>
    );
  }

  if (board.retro.phase === "voting") {
    const allReady = board.ready.participant_count > 0 && board.ready.ready_count >= board.ready.participant_count;
    // Dashed only when someone has marked ready but not everyone — signals "wait".
    // When nobody has marked ready yet (ready_count === 0) keep primary as default.
    const someButNotAll = board.ready.ready_count > 0 && !allReady;
    return (
      <>
        <StageIndicator phase={board.retro.phase} retroId={board.retro.id} />
        {peopleButton}
        <form action={board.ready.current_user_ready ? unmarkReadyAction : markReadyAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind={board.ready.current_user_ready ? "secondary" : "primary"} type="submit">{board.ready.current_user_ready ? "not ready" : "I'm ready"}</Btn>
        </form>
        <form action={startActionDiscussionAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind={someButNotAll ? "dashed" : "primary"} accent={spillColors.action} type="submit" disabled={!isHost && !allReady || board.retro.action_discussion_limit <= 0}>act -&gt;</Btn>
        </form>
      </>
    );
  }

  if (board.retro.phase === "action_discussion") {
    return (
      <>
        <StageIndicator phase={board.retro.phase} retroId={board.retro.id} />
        {peopleButton}
        <Pill tone="soft" accent={spillColors.action}>action</Pill>
        <form action={completeRetroAction}>
          <input name="retro_id" type="hidden" value={board.retro.id} />
          <Btn kind="dashed" type="submit">wrap retro</Btn>
        </form>
      </>
    );
  }

  return (
    <>
      <PhaseBadge color={spillColors.well} phase="wrapped" />
      <Btn href="/" kind="primary">home</Btn>
    </>
  );
}
