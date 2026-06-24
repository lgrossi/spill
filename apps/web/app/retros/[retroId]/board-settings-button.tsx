"use client";

import { useRef } from "react";
import type { RetroBoard } from "@/lib/contracts";
import { BoardConfigForm } from "./board-config-editor";

type CardEditPolicy = RetroBoard["retro"]["card_edit_policy"];
type RevealMode = RetroBoard["retro"]["reveal_mode"];

export function BoardSettingsButton({
  retroId,
  phase,
  returnTo,
  voteLimit,
  actionDiscussionLimit,
  clusteringMode,
  hasActionColumn,
  cardEditPolicy,
  anonymousAuthors,
  revealMode,
}: {
  retroId: string;
  phase: string;
  returnTo: string;
  voteLimit: number;
  actionDiscussionLimit: number;
  clusteringMode: string;
  hasActionColumn: boolean;
  cardEditPolicy: CardEditPolicy;
  anonymousAuthors: boolean;
  revealMode: RevealMode;
}) {
  const dialogRef = useRef<HTMLDialogElement>(null);

  function open() {
    dialogRef.current?.showModal();
  }

  function close() {
    dialogRef.current?.close();
  }

  function handleBackdropClick(e: React.MouseEvent<HTMLDialogElement>) {
    if (e.target === dialogRef.current) close();
  }

  return (
    <>
      <button
        aria-label="board settings"
        className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-spill-line bg-transparent text-spill-muted transition hover:border-spill-fg/30 hover:bg-[var(--paper-2)] hover:text-spill-fg focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
        onClick={open}
        title="board settings"
        type="button"
      >
        <svg
          aria-hidden="true"
          className="h-3.5 w-3.5"
          fill="none"
          stroke="currentColor"
          strokeWidth={2}
          viewBox="0 0 24 24"
        >
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      </button>

      <dialog
        ref={dialogRef}
        className="m-auto w-full max-w-md rounded-[14px] border border-spill-line bg-spill-panel p-0 shadow-[var(--shadow-3)] backdrop:bg-[#1f1812]/50 open:flex open:flex-col"
        onClick={handleBackdropClick}
      >
        <div className="p-5">
          <div className="mb-4 flex items-center justify-between">
            <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">
              board settings
            </p>
            <button
              aria-label="Close"
              className="grid h-7 w-7 place-items-center rounded-[7px] border border-spill-line bg-[var(--paper)] text-[16px] font-extrabold leading-none text-spill-muted shadow-[inset_0_1px_0_rgba(255,255,255,0.5)] transition hover:text-spill-wrong"
              onClick={close}
              type="button"
            >
              &times;
            </button>
          </div>
          <BoardConfigForm
            retroId={retroId}
            phase={phase}
            returnTo={returnTo}
            voteLimit={voteLimit}
            actionDiscussionLimit={actionDiscussionLimit}
            clusteringMode={clusteringMode}
            hasActionColumn={hasActionColumn}
            cardEditPolicy={cardEditPolicy}
            anonymousAuthors={anonymousAuthors}
            revealMode={revealMode}
            onCancel={close}
          />
        </div>
      </dialog>
    </>
  );
}
