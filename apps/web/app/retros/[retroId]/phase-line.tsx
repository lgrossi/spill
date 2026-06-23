"use client";

import { useEffect, useRef, useState } from "react";
import {
  autoAdvanceAction,
  completeRetroAction,
  forceRevealRetroAction,
  startActionDiscussionAction,
  startVotingAction,
} from "@/lib/actions";
import { applyClusteringAction, retryClusteringAction } from "@/lib/actions";

const GRACE_MS = 5000;

const PHASE_VERB: Record<string, string> = {
  writing: "Writing",
  discussion: "Discussing",
  voting: "Voting",
  action_discussion: "Wrapping up",
};

type Props = {
  retroId: string;
  phase: string;
  clusteringMode: string;
  clusteringStatus: string;
  isHost: boolean;
  participantCount: number;
  readyCount: number;
  allReady: boolean;
  revealMode: "per_column" | "big_bang";
};

// Replaces the old stepper. Renders the current phase as a verb + ellipsis,
// with a right-side hint that does double duty:
//   * gated phases (writing, voting): 5s countdown when all ready, else "X of N ready"
//   * non-gated phases (discussion, action_discussion): host-only text link to advance
// The completed phase has its own page; this component is not rendered there.
export function PhaseLine({
  retroId,
  phase,
  clusteringMode,
  clusteringStatus,
  isHost,
  participantCount,
  readyCount,
  allReady,
  revealMode,
}: Props) {
  const verb = PHASE_VERB[phase] ?? phase;
  const gated = phase === "writing" || phase === "voting";

  return (
    <div className="flex flex-col items-center gap-1.5 leading-none">
      <span className="text-[14px] font-semibold text-spill-fg">{verb}…</span>
      <PhaseHint
        retroId={retroId}
        phase={phase}
        clusteringMode={clusteringMode}
        clusteringStatus={clusteringStatus}
        isHost={isHost}
        gated={gated}
        participantCount={participantCount}
        readyCount={readyCount}
        allReady={allReady}
        revealMode={revealMode}
      />
    </div>
  );
}

function PhaseHint({
  retroId,
  phase,
  clusteringMode,
  clusteringStatus,
  isHost,
  gated,
  participantCount,
  readyCount,
  allReady,
  revealMode,
}: Props & { gated: boolean }) {
  const autoCluster = clusteringMode === "auto_on_vote_start";
  // Discussion-phase organization: compute happens here and the host applies it
  // explicitly (or starts voting, which auto-applies). Surface live status.
  if (
    autoCluster &&
    phase === "discussion" &&
    (clusteringStatus === "computing" ||
      clusteringStatus === "ready" ||
      clusteringStatus === "failed")
  ) {
    return (
      <OrganizeHint
        retroId={retroId}
        isHost={isHost}
        status={clusteringStatus as "computing" | "ready" | "failed"}
      />
    );
  }
  if (gated) {
    if (participantCount === 0) return null;
    if (phase === "voting" && autoCluster) {
      if (clusteringStatus === "computing") {
        return (
          <span className="inline-flex items-center gap-1.5 text-[10px] text-spill-muted">
            <span>Organizing...</span>
            {isHost ? (
              <>
                <span aria-hidden="true">|</span>
                <HostAdvanceLink retroId={retroId} action={startActionDiscussionAction} label="wrap up" />
              </>
            ) : null}
          </span>
        );
      }
      if (clusteringStatus === "failed") {
        return (
          <span className="inline-flex items-center gap-1.5 text-[10px] text-spill-muted">
            <span>organizing failed</span>
            {isHost ? (
              <>
                <span aria-hidden="true">|</span>
                <HostAdvanceLink retroId={retroId} action={retryClusteringAction} label="retry organizing" pendingLabel="organizing..." />
                <span aria-hidden="true">|</span>
                <HostAdvanceLink retroId={retroId} action={startActionDiscussionAction} label="wrap up" />
              </>
            ) : null}
          </span>
        );
      }
    }
    if (allReady && !(phase === "writing" && revealMode === "per_column")) {
      return <Countdown retroId={retroId} />;
    }
    // In per_column reveal mode the writing-phase 'start discussing' link is
    // hidden: the host walks one column at a time via the per-column pills,
    // and the last reveal auto-advances the retro. Voting-phase 'wrap up'
    // is unaffected (reveal_mode only governs writing -> discussion).
    const hostLink = isHost ? gatedHostAdvance(phase, revealMode) : null;
    return (
      <span className="inline-flex items-center gap-1.5 text-[10px] text-spill-muted">
        <span>{readyCount} of {participantCount} ready</span>
        {hostLink ? (
          <>
            <span aria-hidden="true">|</span>
            <HostAdvanceLink retroId={retroId} action={hostLink.action} label={hostLink.label} />
          </>
        ) : null}
      </span>
    );
  }
  if (!isHost) return null;
  if (phase === "discussion") {
    return <HostAdvanceLink retroId={retroId} action={startVotingAction} label="start voting" />;
  }
  if (phase === "action_discussion") {
    return <HostAdvanceLink retroId={retroId} action={completeRetroAction} label="finish retro" />;
  }
  return null;
}

function gatedHostAdvance(
  phase: string,
  revealMode: "per_column" | "big_bang",
): { action: (formData: FormData) => void | Promise<void>; label: string } | null {
  if (phase === "writing") {
    if (revealMode === "per_column") return null;
    return { action: forceRevealRetroAction, label: "start discussing" };
  }
  if (phase === "voting") return { action: startActionDiscussionAction, label: "wrap up" };
  return null;
}

// Discussion-phase organization status. Hosts apply a ready proposal or retry a
// failed one; either way they can still start voting (which auto-applies).
function OrganizeHint({
  retroId,
  isHost,
  status,
}: {
  retroId: string;
  isHost: boolean;
  status: "computing" | "ready" | "failed";
}) {
  const text = status === "failed" ? "organizing failed" : status === "ready" ? "organized" : "Organizing…";
  if (!isHost) {
    return <span className="text-[10px] text-spill-muted">{text}</span>;
  }
  return (
    <span className="inline-flex items-center gap-1.5 text-[10px] text-spill-muted">
      {status === "computing" ? <span>{text}</span> : null}
      {status === "ready" ? (
        <HostAdvanceLink retroId={retroId} action={applyClusteringAction} label="apply organizing" pendingLabel="applying..." />
      ) : null}
      {status === "failed" ? (
        <HostAdvanceLink retroId={retroId} action={retryClusteringAction} label="retry organizing" pendingLabel="organizing..." />
      ) : null}
      <span aria-hidden="true">|</span>
      <HostAdvanceLink retroId={retroId} action={startVotingAction} label="start voting" />
    </span>
  );
}

function Countdown({ retroId }: { retroId: string }) {
  const formRef = useRef<HTMLFormElement | null>(null);
  const [remaining, setRemaining] = useState(Math.ceil(GRACE_MS / 1000));

  useEffect(() => {
    const start = Date.now();
    const tick = window.setInterval(() => {
      setRemaining(Math.max(0, Math.ceil((GRACE_MS - (Date.now() - start)) / 1000)));
    }, 200);
    const fire = window.setTimeout(() => {
      formRef.current?.requestSubmit();
    }, GRACE_MS);
    return () => {
      window.clearInterval(tick);
      window.clearTimeout(fire);
    };
  }, [retroId]);

  return (
    <form action={autoAdvanceAction} ref={formRef} className="contents">
      <input name="retro_id" type="hidden" value={retroId} />
      <span className="text-[10px] text-spill-muted">advancing in {remaining}s</span>
    </form>
  );
}

function HostAdvanceLink({
  retroId,
  action,
  label,
  pendingLabel,
}: {
  retroId: string;
  action: (formData: FormData) => void | Promise<void>;
  label: string;
  pendingLabel?: string;
}) {
  const [pending, setPending] = useState(false);
  return (
    <form action={action} className="contents" onSubmit={() => setPending(true)}>
      <input name="retro_id" type="hidden" value={retroId} />
      <button
        aria-label={label}
        disabled={pending}
        title={label}
        type="submit"
        style={{ display: "contents" }}
      >
        <span className="text-[10px] text-spill-muted underline-offset-2 hover:text-spill-fg hover:underline cursor-pointer">
          {pending ? pendingLabel ?? "working..." : `${label} →`}
        </span>
      </button>
    </form>
  );
}
