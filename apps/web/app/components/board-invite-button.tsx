"use client";

import { useRef } from "react";
import { InvitePanel } from "./invite-panel";

export function BoardInviteButton({
  retroId,
  currentUserEmail,
  isHost,
}: {
  retroId: string;
  currentUserEmail: string;
  isHost: boolean;
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
        aria-label="people"
        className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-spill-line bg-transparent text-spill-muted transition hover:border-spill-fg/30 hover:bg-[var(--paper-2)] hover:text-spill-fg focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
        onClick={open}
        title="people"
        type="button"
      >
        <svg
          aria-hidden="true"
          className="h-3.5 w-3.5"
          fill="none"
          stroke="currentColor"
          strokeWidth={2}
          viewBox="0 0 20 20"
        >
          <circle cx="8" cy="7" r="3" />
          <path d="M2 19c0-3.314 2.686-6 6-6s6 2.686 6 6" strokeLinecap="round" />
          <circle cx="15" cy="7" r="2.5" />
          <path d="M19 19c0-2.761-1.791-5-4-5.5" strokeLinecap="round" />
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
              people
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
          <InvitePanel
            mode="board"
            retroId={retroId}
            currentUserEmail={currentUserEmail}
            isHost={isHost}
          />
        </div>
      </dialog>
    </>
  );
}
