"use client";

import { useEffect, useRef, useState } from "react";
import {
  autoAdvanceAction,
  completeRetroAction,
  forceRevealRetroAction,
  startActionDiscussionAction,
  startVotingAction,
} from "@/lib/actions";

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
  isHost: boolean;
  participantCount: number;
  readyCount: number;
  allReady: boolean;
};

// Replaces the old stepper. Renders the current phase as a verb + ellipsis,
// with a right-side hint that does double duty:
//   * gated phases (writing, voting): 5s countdown when all ready, else "X of N ready"
//   * non-gated phases (discussion, action_discussion): host-only text link to advance
// The completed phase has its own page; this component is not rendered there.
export function PhaseLine({
  retroId,
  phase,
  isHost,
  participantCount,
  readyCount,
  allReady,
}: Props) {
  const verb = PHASE_VERB[phase] ?? phase;
  const gated = phase === "writing" || phase === "voting";

  return (
    <div className="flex flex-col items-center gap-1.5 leading-none">
      <span className="text-[14px] font-semibold text-spill-fg">{verb}…</span>
      <PhaseHint
        retroId={retroId}
        phase={phase}
        isHost={isHost}
        gated={gated}
        participantCount={participantCount}
        readyCount={readyCount}
        allReady={allReady}
      />
    </div>
  );
}

function PhaseHint({
  retroId,
  phase,
  isHost,
  gated,
  participantCount,
  readyCount,
  allReady,
}: Props & { gated: boolean }) {
  if (gated) {
    if (participantCount === 0) return null;
    if (allReady) return <Countdown retroId={retroId} />;
    const hostLink = isHost ? gatedHostAdvance(phase) : null;
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

function gatedHostAdvance(phase: string): { action: (formData: FormData) => void | Promise<void>; label: string } | null {
  if (phase === "writing") return { action: forceRevealRetroAction, label: "start discussing" };
  if (phase === "voting") return { action: startActionDiscussionAction, label: "wrap up" };
  return null;
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
}: {
  retroId: string;
  action: (formData: FormData) => void | Promise<void>;
  label: string;
}) {
  return (
    <form action={action} className="contents">
      <input name="retro_id" type="hidden" value={retroId} />
      <button
        aria-label={label}
        title={label}
        type="submit"
        style={{ display: "contents" }}
      >
        <span className="text-[10px] text-spill-muted underline-offset-2 hover:text-spill-fg hover:underline cursor-pointer">
          {label} →
        </span>
      </button>
    </form>
  );
}
