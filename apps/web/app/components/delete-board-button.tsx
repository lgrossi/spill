"use client";

import { useRef, useTransition } from "react";
import { deleteRetroAction } from "@/lib/actions";

export function DeleteBoardButton({
  retroId,
  boardTitle,
}: {
  retroId: string;
  boardTitle: string;
}) {
  const formRef = useRef<HTMLFormElement>(null);
  const [isPending, startTransition] = useTransition();

  function handleClick() {
    const confirmed = window.confirm(
      `Delete "${boardTitle}"? This removes the board, its cards, votes, and actions permanently.`,
    );
    if (!confirmed) return;
    const form = formRef.current;
    if (!form) return;
    startTransition(() => {
      form.requestSubmit();
    });
  }

  return (
    <form action={deleteRetroAction} ref={formRef} className="contents">
      <input name="retro_id" type="hidden" value={retroId} />
      <button
        aria-label="delete board"
        className="inline-flex h-7 w-7 items-center justify-center rounded-full border border-spill-line bg-transparent text-spill-muted transition hover:border-spill-wrong/40 hover:bg-[var(--paper-2)] hover:text-spill-wrong focus-visible:outline-none focus-visible:shadow-[var(--focus)] disabled:pointer-events-none disabled:opacity-45"
        disabled={isPending}
        onClick={handleClick}
        title="delete board"
        type="button"
      >
        <svg
          aria-hidden="true"
          className="h-3.5 w-3.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.75"
          viewBox="0 0 24 24"
        >
          <path d="M4 7h16" strokeLinecap="round" />
          <path d="M9 7V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" strokeLinecap="round" />
          <path d="M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12" strokeLinecap="round" strokeLinejoin="round" />
          <path d="M10 11v6M14 11v6" strokeLinecap="round" />
        </svg>
      </button>
    </form>
  );
}
