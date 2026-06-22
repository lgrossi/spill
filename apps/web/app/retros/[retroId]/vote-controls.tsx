import type { CSSProperties } from "react";
import { cardButtonClass, readableCardControlColor } from "@/components/spill-ui";
import type { RetroBoard, RetroCard } from "@/lib/api";
import { castVoteAction, removeVoteAction } from "@/lib/actions";

export function VoteControls({ board, card, color }: { board: RetroBoard; card: RetroCard; color: string }) {
  const controlColor = readableCardControlColor(color);
  return (
    <div className="flex items-center gap-1" data-spill-no-drag>
      <form action={removeVoteAction}>
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <input name="card_id" type="hidden" value={card.id} />
        <button className={cardButtonClass} disabled={card.current_user_vote_count <= 0} style={{ "--card-button-fg": controlColor } as CSSProperties} type="submit">-</button>
      </form>
      <span className="grid h-6 min-w-6 place-items-center rounded-full border border-white/35 bg-white/15 px-2 text-[11px] font-extrabold text-[var(--card-fg)]" aria-label={`${card.current_user_vote_count} of your votes on this card`}>
        {card.current_user_vote_count}
      </span>
      <form action={castVoteAction}>
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <input name="card_id" type="hidden" value={card.id} />
        <button className={cardButtonClass} disabled={board.voting.votes_remaining <= 0} style={{ "--card-button-fg": controlColor } as CSSProperties} type="submit">+</button>
      </form>
    </div>
  );
}
