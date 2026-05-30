import { updateRetroMetadataAction } from "@/lib/actions";
import type { RetroBoard } from "@/lib/contracts";
import { fieldControlClass } from "./spill-ui";

export function RetroMetadataEditor({ board }: { board: RetroBoard }) {
  return (
    <details className="relative">
      <summary className="list-none rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-2.5 py-1 text-[11.5px] font-extrabold text-spill-fg shadow-[var(--shadow-1)] transition hover:border-spill-wrong/50 [&::-webkit-details-marker]:hidden">
        edit
      </summary>
      <form
        action={updateRetroMetadataAction}
        className="absolute right-0 top-9 z-20 grid w-[280px] gap-2 rounded-[12px] border border-spill-line bg-spill-panel p-3 text-left shadow-[var(--shadow-3)]"
      >
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <label className="grid gap-1">
          <span className="text-[10px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">title</span>
          <input className={fieldControlClass} name="title" required defaultValue={board.retro.title} />
        </label>
        <label className="grid gap-1">
          <span className="text-[10px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">scheduled</span>
          <input className={fieldControlClass} name="scheduled_at" type="datetime-local" defaultValue={datetimeLocalValue(board.retro.scheduled_at)} />
        </label>
        <label className="grid gap-1">
          <span className="text-[10px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">cover GIF/image URL</span>
          <input className={fieldControlClass} name="cover_gif_url" type="url" defaultValue={board.retro.cover_gif_url ?? ""} />
        </label>
        <label className="grid gap-1">
          <span className="text-[10px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">cover alt text</span>
          <input className={fieldControlClass} name="cover_gif_alt_text" defaultValue={board.retro.cover_gif_alt_text ?? ""} />
        </label>
        <button className="rounded-[8px] bg-spill-wrong px-3 py-2 text-[12px] font-extrabold text-white shadow-[var(--shadow-1)]" type="submit">
          save
        </button>
      </form>
    </details>
  );
}

function datetimeLocalValue(value: string | null) {
  if (!value) return "";
  return value.slice(0, 16);
}
