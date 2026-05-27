"use client";

import { useEffect, useState } from "react";
import { addGrantAction, listGrantsAction, removeGrantAction } from "@/lib/actions";
import { type Grant } from "@/lib/api";
import { Avatar, avatarColorForSeed, avatarInitials } from "./spill-ui";
import { UserAutocomplete, type Picked } from "./user-autocomplete";

export type InvitePanelProps =
  | { mode: "create"; onInviteesChange: (emails: string[]) => void }
  | { mode: "board"; retroId: string; currentUserEmail: string };

export function InvitePanel(props: InvitePanelProps) {
  if (props.mode === "create") {
    return <CreateInvitePanel onInviteesChange={props.onInviteesChange} />;
  }
  return (
    <BoardInvitePanel
      retroId={props.retroId}
      currentUserEmail={props.currentUserEmail}
    />
  );
}

function CreateInvitePanel({
  onInviteesChange,
}: {
  onInviteesChange: (emails: string[]) => void;
}) {
  const [picked, setPicked] = useState<Picked[]>([]);

  function handlePick(users: Picked[]) {
    const next = [...picked];
    for (const u of users) {
      if (!next.some((p) => p.email === u.email)) next.push(u);
    }
    setPicked(next);
    onInviteesChange(next.map((p) => p.email));
  }

  function handleRemove(email: string) {
    const next = picked.filter((p) => p.email !== email);
    setPicked(next);
    onInviteesChange(next.map((p) => p.email));
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
              onRemove={handleRemove}
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
}: {
  retroId: string;
  currentUserEmail: string;
}) {
  const [grants, setGrants] = useState<Grant[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

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
    try {
      await Promise.all(users.map((u) => addGrantAction(retroId, u.email)));
      await reload();
    } catch {
      setError("Could not add member.");
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

  return (
    <div className="space-y-3">
      <UserAutocomplete onPick={handlePick} />
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
              onRemove={
                g.principal_email !== currentUserEmail
                  ? handleRemove
                  : undefined
              }
            />
          ))}
        </div>
      )}
    </div>
  );
}

function MemberBadge({
  email,
  label,
  role,
  onRemove,
}: {
  email: string;
  label: string;
  role?: "host" | "member";
  onRemove?: (email: string) => void;
}) {
  const color = avatarColorForSeed(email);
  const initials = avatarInitials(label);
  return (
    <span className="inline-flex items-center gap-1.5 rounded-full border border-spill-line bg-[var(--panel-hi)] py-1 pl-1 pr-2 text-[11px] font-semibold text-[var(--fg-2)]">
      <Avatar k={initials} color={color} size={18} />
      <span className="max-w-[120px] truncate">{label}</span>
      {role === "host" && (
        <span className="text-[9px] font-extrabold uppercase tracking-[0.1em] text-spill-muted">
          host
        </span>
      )}
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
