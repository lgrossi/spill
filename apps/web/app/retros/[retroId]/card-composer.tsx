import Link from "next/link";
import { CardComposer, type ColumnAccent } from "@/components/spill-ui";
import type { RetroBoard, RetroCard } from "@/lib/api";
import { createDraftCardAction, updateDraftCardAction } from "@/lib/actions";
import { ComposerSubmit } from "./composer-submit";
import { GifDraftProvider, GifSelectedPreview } from "./gif-draft";
import { GifSearchPicker } from "./gif-search-picker";

export function DraftCardEditor({ board, card, color, semantic }: { board: RetroBoard; card: RetroCard; color: string; semantic: ColumnAccent }) {
  return (
    <form action={updateDraftCardAction} className="grid min-w-0 gap-2">
      <input name="card_id" type="hidden" value={card.id} />
      <input name="existing_gif_url" type="hidden" value={card.gif_url ?? ""} />
      <input name="existing_gif_alt_text" type="hidden" value={card.gif_alt_text ?? ""} />
      <GifDraftProvider initialGif={card.gif_url ? { id: card.id, url: card.gif_url, preview_url: card.gif_url, alt_text: card.gif_alt_text ?? "Attached media" } : null}>
        <CardComposer
          accent={color}
          before={<GifSelectedPreview />}
          after={<GifSearchPicker columnTitle={semantic} />}
          actions={
            <>
              <Link aria-label="Cancel edit" className={composerButtonClass("ghost")} href={`/retros/${board.retro.id}?addColumn=${card.column_id}`}>×</Link>
              <ComposerSubmit className={composerButtonClass("solid")} existingGif={Boolean(card.gif_url)} />
            </>
          }
          columnId={card.column_id}
          draftText={card.body_text ?? ""}
          placeholder="edit this card"
          retroId={board.retro.id}
        />
      </GifDraftProvider>
    </form>
  );
}

export function InlineComposer({
  columnId,
  columnTitle,
  color,
  draftText,
  retroId,
}: {
  columnId: string;
  columnTitle: string;
  color: string;
  draftText: string;
  retroId: string;
}) {
  return (
    <form action={createDraftCardAction} className="grid min-w-0 gap-2">
      <GifDraftProvider>
        <CardComposer
          accent={color}
          before={<GifSelectedPreview />}
          after={<GifSearchPicker columnTitle={columnTitle} />}
          actions={
            <>
              <Link aria-label="Cancel card" className={composerButtonClass("ghost")} href={`/retros/${retroId}`}>×</Link>
              <ComposerSubmit className={composerButtonClass("solid")} />
            </>
          }
          columnId={columnId}
          draftText={draftText}
          placeholder="What's on your mind?"
          retroId={retroId}
        />
      </GifDraftProvider>
    </form>
  );
}

function composerButtonClass(kind: "ghost" | "solid") {
  const base = "grid h-7 w-7 place-items-center rounded-full border text-[13px] font-extrabold leading-none shadow-[0_1px_2px_rgba(0,0,0,0.14)] transition focus-visible:outline-none focus-visible:shadow-[var(--focus)]";
  if (kind === "solid") {
    return `${base} border-white bg-white text-[var(--card-button-fg)] hover:bg-white/90`;
  }
  return `${base} border-white/35 bg-white/15 text-white/90 hover:bg-white/25`;
}
