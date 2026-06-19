import Link from "next/link";
import { Avatar, CardFooter, GifTile, HiddenDraft, SpillCard, avatarColorForSeed, avatarInitials, shortAuthorName } from "@/components/spill-ui";
import type { RetroBoard, RetroCard } from "@/lib/api";
import { deleteDraftCardAction, removeClusterMemberAction, updateDraftCardAction } from "@/lib/actions";
import { BoardMedia } from "./media-card";
import { DraggableCard } from "./board-dnd";
import { DraftCardEditor } from "./card-composer";
import { VoteControls } from "./vote-controls";

export function CardView({
  board,
  card,
  color,
  draggable,
  editing,
  moving,
  clustering,
  semanticLabel,
}: {
  board: RetroBoard;
  card: RetroCard;
  color: string;
  draggable: boolean;
  editing: boolean;
  moving: boolean;
  clustering: boolean;
  semanticLabel: string;
}) {
  if (card.hidden) {
    return <HiddenDraft accent={color} />;
  }

  const isEditingGroup = editing && board.retro.phase !== "completed" && card.parent_card_id === null && card.cluster_id !== null;
  const author = participantById(board, card.author_participant_id);

  if (editing && !isEditingGroup && board.retro.phase !== "completed" && card.parent_card_id === null) {
    return <DraftCardEditor board={board} card={card} color={color} semanticLabel={semanticLabel} />;
  }

  const hasMedia = !card.cluster_id && Boolean(card.gif_url);
  const hasActions = board.retro.phase !== "completed" && card.parent_card_id === null && !isEditingGroup;

  return (
    <DraggableCard accent={color} cardId={card.id} columnId={card.column_id} enabled={draggable} clusteringEnabled={clustering} movingEnabled={moving} retroId={board.retro.id}>
      <SpillCard accent={color}>
        {hasActions ? <CardActions board={board} card={card} /> : null}
        {hasMedia ? card.gif_url === "demo-gif" ? <GifTile className="mb-2" /> : <BoardMedia alt={card.gif_alt_text ?? "Attached media"} src={card.gif_url!} /> : null}
        {isEditingGroup ? <GroupTitleEditor board={board} card={card} /> : card.body_text ? <p className="whitespace-pre-wrap break-words">{card.body_text}</p> : null}
        {!isEditingGroup && card.cluster_id && card.cluster_details ? <p className="mt-2 whitespace-pre-wrap break-words text-[12px] italic text-white/80">{card.cluster_details}</p> : null}
        <ClusterMembers board={board} card={card} />
        <CardFooter
          author={avatarInitials(author?.display_name)}
          authorName={shortAuthorName(author?.display_name)}
          color={avatarColorForSeed(author?.id)}
          trailing={board.retro.phase === "voting" ? <VoteControls board={board} card={card} color={color} /> : undefined}
          votes={board.retro.phase === "action_discussion" ? card.vote_count : undefined}
        />
      </SpillCard>
    </DraggableCard>
  );
}

function GroupTitleEditor({ board, card }: { board: RetroBoard; card: RetroCard }) {
  return (
    <form action={updateDraftCardAction} className="mb-2 pr-16" data-spill-no-drag>
      <input name="retro_id" type="hidden" value={board.retro.id} />
      <input name="column_id" type="hidden" value={card.column_id} />
      <input name="card_id" type="hidden" value={card.id} />
      <input name="editing_group_title" type="hidden" value="1" />
      <div className="flex items-center gap-1.5">
        <input
          autoFocus
          className="min-w-0 flex-1 rounded-[6px] border border-white/25 bg-white/15 px-2 py-1.5 text-[13px] font-extrabold leading-5 text-white outline-none placeholder:text-white/60"
          defaultValue={card.body_text ?? ""}
          name="body_text"
          placeholder="group title"
          required
        />
        <button aria-label="Save group" className="grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/20 text-[12px] font-extrabold leading-none text-white/90 transition hover:bg-black/30" type="submit">✓</button>
        <Link aria-label="Cancel edit" className="grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/20 text-[12px] font-extrabold leading-none text-white/90 transition hover:bg-black/30" href={`/retros/${board.retro.id}`}>×</Link>
      </div>
      <textarea
        className="mt-1.5 w-full resize-none rounded-[6px] border border-white/25 bg-white/15 px-2 py-1.5 text-[12px] leading-4 text-white outline-none placeholder:text-white/60"
        defaultValue={card.cluster_details ?? ""}
        name="cluster_details"
        placeholder="add a note or the action for this group (optional)"
        rows={2}
      />
    </form>
  );
}

function ClusterMembers({ board, card }: { board: RetroBoard; card: RetroCard }) {
  if (card.cluster_members.length === 0) {
    return card.cluster_title ? <p className="mt-2 border-t border-white/20 pt-2 text-[10px] font-extrabold uppercase tracking-[0.1em] text-white/85">{card.cluster_title}</p> : null;
  }

  return (
    <div className="mt-2 space-y-1.5 border-t border-white/20 pt-2">
      {card.cluster_members.map((member) => (
        <div className="rounded-[6px] bg-white/15 px-2 py-1.5 text-[11.5px] leading-4 text-white/90" key={member.id}>
          {member.gif_url ? <BoardMedia alt={member.gif_alt_text ?? "Grouped media"} src={member.gif_url} /> : null}
          <div className="mt-1 flex items-start gap-2">
            <Avatar
              color={avatarColorForSeed(participantById(board, member.author_participant_id)?.id)}
              k={avatarInitials(participantById(board, member.author_participant_id)?.display_name)}
              ring="rgba(255,255,255,0.35)"
              size={16}
            />
            <span className="min-w-0 flex-1">{member.hidden ? ". . . someone's draft . . ." : member.body_text || member.gif_alt_text || "media card"}</span>
            <form action={removeClusterMemberAction} data-spill-no-drag>
              <input name="retro_id" type="hidden" value={board.retro.id} />
              <input name="card_id" type="hidden" value={member.id} />
              <button aria-label="Remove from group" className="grid h-5 w-5 place-items-center rounded-full border border-white/35 text-[11px] font-extrabold text-white/85 transition hover:bg-white/20" title="Remove from group" type="submit">↗</button>
            </form>
          </div>
        </div>
      ))}
    </div>
  );
}

function participantById(board: RetroBoard, participantId: string) {
  return board.participants.find((participant) => participant.id === participantId);
}

function CardActions({ board, card }: { board: RetroBoard; card: RetroCard }) {
  const editHref = card.cluster_id ? `/retros/${board.retro.id}?editCard=${card.id}` : `/retros/${board.retro.id}?addColumn=${card.column_id}&editCard=${card.id}`;

  return (
    <div className="mb-1.5 flex justify-end gap-1">
      <Link aria-label="Edit card" className="grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/20 text-[12px] font-extrabold leading-none text-white/90 shadow-[0_1px_2px_rgba(0,0,0,0.16)] transition hover:bg-black/30" data-spill-no-drag href={editHref}>✎</Link>
      <form action={deleteDraftCardAction} data-spill-no-drag>
        <input name="retro_id" type="hidden" value={board.retro.id} />
        <input name="card_id" type="hidden" value={card.id} />
        <button aria-label="Delete card" className="grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/20 text-[13px] font-extrabold leading-none text-white/90 shadow-[0_1px_2px_rgba(0,0,0,0.16)] transition hover:bg-black/30" type="submit">×</button>
      </form>
    </div>
  );
}
