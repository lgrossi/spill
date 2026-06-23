"use client";

import { useEffect, useState } from "react";
import { addGrantAction, listGrantsAction, removeGrantAction, setParticipationAction } from "@/lib/actions";
import { type Grant } from "@/lib/api";
import type { RetroParticipant } from "@/lib/contracts";
import { Avatar, avatarColorForSeed, avatarInitials } from "./spill-ui";
import { UserAutocomplete, type Picked } from "./user-autocomplete";

export type InvitePanelProps =
  | { mode: "create"; onInviteesChange: (invitees: { email: string; role: "host" | "member" }[]) => void }
  | {
      mode: "board";
      retroId: string;
      currentUserEmail: string;
      isHost: boolean;
      participants: RetroParticipant[];
      currentUserParticipantId: string | null;
    };

export function InvitePanel(props: InvitePanelProps) {
  if (props.mode === "create") {
    return <CreateInvitePanel onInviteesChange={props.onInviteesChange} />;
  }
  return (
    <BoardInvitePanel
      retroId={props.retroId}
      currentUserEmail={props.currentUserEmail}
      isHost={props.isHost}
      participants={props.participants}
      currentUserParticipantId={props.currentUserParticipantId}
    />
  );
}

function CreateInvitePanel({
  onInviteesChange,
}: {
  onInviteesChange: (invitees: { email: string; role: "host" | "member" }[]) => void;
}) {
  const [picked, setPicked] = useState<Array<Picked & { role: "host" | "member" }>>([]);

  function handlePick(users: Picked[]) {
    const next = [...picked];
    for (const u of users) {
      if (!next.some((p) => p.email === u.email)) {
        next.push({ ...u, role: u.role ?? "member" });
      }
    }
    setPicked(next);
    onInviteesChange(next.map((p) => ({ email: p.email, role: p.role })));
  }

  function handleRemove(email: string) {
    const next = picked.filter((p) => p.email !== email);
    setPicked(next);
    onInviteesChange(next.map((p) => ({ email: p.email, role: p.role })));
  }

  function handleRoleChange(email: string, role: "host" | "member") {
    const next = picked.map((p) => (p.email === email ? { ...p, role } : p));
    setPicked(next);
    onInviteesChange(next.map((p) => ({ email: p.email, role: p.role })));
  }

  return (
    <div className="space-y-3">
      <UserAutocomplete onPick={handlePick} />
      {picked.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {picked.map((u) => (
            <MemberBadge
              key={u.email}
              email={u.email}
              label={u.name || u.email.split("@")[0]}
              role={u.role}
              onRemove={handleRemove}
              onRoleChange={handleRoleChange}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function BoardInvitePanel({
  retroId,
  currentUserEmail,
  isHost,
  participants,
  currentUserParticipantId,
}: {
  retroId: string;
  currentUserEmail: string;
  isHost: boolean;
  participants: RetroParticipant[];
  currentUserParticipantId: string | null;
}) {
  const [grants, setGrants] = useState<Grant[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [adding, setAdding] = useState(false);

  async function reload() {
    try {
      const g = await listGrantsAction(retroId);
      setGrants(g);
      setError(null);
    } catch {
      setError("Could not load members.");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [retroId]);

  async function handlePick(users: Picked[]) {
    const emails = users
      .map((user) => user.email.trim().toLowerCase())
      .filter((email) => email.length > 0)
      .filter((email, index, all) => all.indexOf(email) === index)
      .filter((email) => !grants.some((grant) => grant.principal_email === email));
    if (emails.length === 0) {
      return;
    }
    setAdding(true);
    try {
      await Promise.all(emails.map((email) => addGrantAction(retroId, email, "member")));
      await reload();
    } catch {
      setError("Could not add member.");
    } finally {
      setAdding(false);
    }
  }

  async function handleRemove(email: string) {
    try {
      await removeGrantAction(retroId, email);
      await reload();
    } catch {
      setError("Could not remove member.");
    }
  }

  async function handleRoleChange(email: string, role: "host" | "member") {
    try {
      await addGrantAction(retroId, email, role);
      await reload();
    } catch {
      setError("Could not update role.");
    }
  }

  return (
    <div className="space-y-3">
      {isHost && <UserAutocomplete onPick={handlePick} />}
      {adding ? <p className="text-[11px] text-spill-muted">Inviting...</p> : null}
      {error && <p className="text-[11px] text-spill-wrong">{error}</p>}
      {loading ? (
        <p className="text-[11px] text-spill-muted">Loading members...</p>
      ) : (
        <div className="flex flex-wrap gap-2">
          {grants.map((g) => (
            <MemberBadge
              key={g.principal_email}
              email={g.principal_email}
              label={g.principal_email.split("@")[0]}
              role={g.role}
              onRemove={isHost && g.principal_email !== currentUserEmail ? handleRemove : undefined}
              onRoleChange={
                isHost && g.principal_email !== currentUserEmail && g.role !== "host"
                  ? handleRoleChange
                  : undefined
              }
              isSelf={g.principal_email === currentUserEmail}
              retroId={retroId}
            />
          ))}
        </div>
      )}
      <ParticipationSection
        retroId={retroId}
        participants={participants}
        currentUserParticipantId={currentUserParticipantId}
        isHost={isHost}
      />
    </div>
  );
}

/// Per-participant "in this round" toggle.
/// Self can always flip; host can flip anyone (server enforces both).
function ParticipationSection({
  retroId,
  participants,
  currentUserParticipantId,
  isHost,
}: {
  retroId: string;
  participants: RetroParticipant[];
  currentUserParticipantId: string | null;
  isHost: boolean;
}) {
  if (participants.length === 0) return null;
  return (
    <div className="space-y-2 border-t border-spill-line pt-3">
      <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">
        in this round
      </p>
      <ul className="space-y-1">
        {participants.map((participant) => {
          const isSelf = currentUserParticipantId === participant.id;
          const canToggle = isSelf || isHost;
          const next = participant.is_participating ? "0" : "1";
          const labelSelf = participant.is_participating ? "sit out" : "rejoin";
          const labelOther = participant.is_participating ? "remove from round" : "add to round";
          return (
            <li
              key={participant.id}
              className="flex items-center justify-between gap-3 rounded-md border border-spill-line bg-[var(--panel-hi)] px-2 py-1.5"
            >
              <span className="flex min-w-0 items-center gap-2">
                <Avatar
                  color={avatarColorForSeed(participant.id)}
                  k={avatarInitials(participant.display_name)}
                  size={18}
                  status={participant.is_participating ? undefined : "away"}
                />
                <span
                  className={`truncate text-[11.5px] font-semibold ${
                    participant.is_participating ? "text-[var(--fg-2)]" : "text-spill-muted line-through"
                  }`}
                >
                  {participant.display_name}
                  {isSelf ? " (you)" : ""}
                </span>
              </span>
              {canToggle ? (
                <form action={setParticipationAction} className="contents">
                  <input name="retro_id" type="hidden" value={retroId} />
                  <input name="participant_id" type="hidden" value={participant.id} />
                  <input name="is_participating" type="hidden" value={next} />
                  <button
                    type="submit"
                    className="text-[10.5px] font-extrabold uppercase tracking-[0.1em] text-spill-muted transition hover:text-spill-fg"
                  >
                    {isSelf ? labelSelf : labelOther}
                  </button>
                </form>
              ) : (
                <span className="text-[10.5px] font-semibold uppercase tracking-[0.1em] text-spill-muted">
                  {participant.is_participating ? "in" : "out"}
                </span>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function MemberBadge({
  email,
  label,
  role,
  onRemove,
  onRoleChange,
  isSelf,
  retroId,
}: {
  email: string;
  label: string;
  role?: "host" | "member";
  onRemove?: (email: string) => void;
  onRoleChange?: (email: string, role: "host" | "member") => void;
  isSelf?: boolean;
  retroId?: string;
}) {
  const color = avatarColorForSeed(email);
  const initials = avatarInitials(label);
  const effectiveRole = role ?? "member";
  return (
    <span className="inline-flex items-center gap-1.5 rounded-full border border-spill-line bg-[var(--panel-hi)] py-1 pl-1 pr-2 text-[11px] font-semibold text-[var(--fg-2)]">
      <Avatar k={initials} color={color} size={18} />
      <span className="max-w-[120px] truncate">{label}</span>
      {onRoleChange ? (
        <button
          type="button"
          title={`Switch to ${effectiveRole === "host" ? "member" : "host"}`}
          onClick={() => onRoleChange(email, effectiveRole === "host" ? "member" : "host")}
          className={`text-[9px] font-extrabold uppercase tracking-[0.1em] transition hover:opacity-75 ${
            effectiveRole === "host" ? "text-spill-action" : "text-spill-muted"
          }`}
        >
          {effectiveRole}
        </button>
      ) : effectiveRole === "host" ? (
        <span className="text-[9px] font-extrabold uppercase tracking-[0.1em] text-spill-muted">
          host
        </span>
      ) : null}
      {onRemove && (
        <button
          aria-label={`Remove ${label}`}
          className="ml-0.5 grid h-4 w-4 place-items-center rounded-full text-spill-muted transition hover:bg-spill-wrong/15 hover:text-spill-wrong"
          onClick={() => onRemove(email)}
          type="button"
        >
          <span aria-hidden="true" className="text-[12px] leading-none">
            &times;
          </span>
        </button>
      )}
    </span>
  );
}
