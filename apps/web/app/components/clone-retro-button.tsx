import { cloneRetroAction } from "@/lib/actions";

export function CloneRetroButton({
  retroId,
  title,
}: {
  retroId: string;
  title: string;
  scheduledAt: string | null;
  createdAt: string;
}) {
  return (
    <details className="relative">
      <summary className="list-none rounded-full border border-spill-line bg-transparent px-2 py-1 text-[10px] font-extrabold text-spill-muted transition hover:border-spill-well/50 hover:bg-[var(--paper-2)] hover:text-spill-well [&::-webkit-details-marker]:hidden">
        next
      </summary>
      <form action={cloneRetroAction} className="absolute right-0 top-8 z-20 grid w-[260px] gap-2 rounded-[12px] border border-spill-line bg-spill-panel p-3 shadow-[var(--shadow-3)]">
        <input name="source_retro_id" type="hidden" value={retroId} />
        <label className="grid gap-1 text-[10px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">
          title
          <input className="rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 py-2 text-[12px] font-semibold normal-case tracking-normal text-spill-fg" name="title" defaultValue={`Next: ${title}`} />
        </label>
        <label className="grid gap-1 text-[10px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">
          scheduled
          <input className="rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 py-2 text-[12px] font-semibold normal-case tracking-normal text-spill-fg" name="scheduled_at" type="datetime-local" aria-describedby={`clone-${retroId}-schedule-help`} />
          <span id={`clone-${retroId}-schedule-help`} className="text-[10.5px] font-semibold normal-case tracking-normal text-spill-muted">Leave blank to infer the next date from cadence.</span>
        </label>
        <button className="rounded-[8px] bg-spill-wrong px-3 py-2 text-[12px] font-extrabold text-white shadow-[var(--shadow-1)]" type="submit">
          create
        </button>
      </form>
    </details>
  );
}
