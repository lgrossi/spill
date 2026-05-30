"use client";

import { useTransition } from "react";
import { cloneRetroAction } from "@/lib/actions";

export function CloneRetroButton({
  retroId,
  title,
  scheduledAt,
  createdAt,
}: {
  retroId: string;
  title: string;
  scheduledAt: string | null;
  createdAt: string;
}) {
  const [isPending, startTransition] = useTransition();
  return (
    <form action={cloneRetroAction} className="contents">
      <input name="source_retro_id" type="hidden" value={retroId} />
      <input name="title" type="hidden" value={`Next: ${title}`} />
      <input name="scheduled_at" type="hidden" value={nextDatetimeLocal(scheduledAt ?? createdAt)} />
      <button
        aria-label="create next retro"
        className="inline-flex h-7 items-center justify-center rounded-full border border-spill-line bg-transparent px-2 text-[10px] font-extrabold text-spill-muted transition hover:border-spill-well/50 hover:bg-[var(--paper-2)] hover:text-spill-well disabled:pointer-events-none disabled:opacity-45"
        disabled={isPending}
        onClick={(event) => {
          event.preventDefault();
          const form = event.currentTarget.form;
          startTransition(() => form?.requestSubmit());
        }}
        title="create next retro"
        type="button"
      >
        next
      </button>
    </form>
  );
}

function nextDatetimeLocal(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";
  date.setDate(date.getDate() + 14);
  return date.toISOString().slice(0, 16);
}
