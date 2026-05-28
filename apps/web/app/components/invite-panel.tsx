"use client";

import { useEffect, useState } from "react";
import { addGrantAction, leaveRetroAction, listGrantsAction, removeGrantAction } from "@/lib/actions";
import { type Grant } from "@/lib/api";
import { Avatar, avatarColorForSeed, avatarInitials } from "./spill-ui";
import { UserAutocomplete, type Picked } from "./user-autocomplete";

export type InvitePanelProps =
  | { mode: "create"; onInviteesChange: (invitees: { email: string; role: "host" | "member" }[]) => void }
  | { mode: "board"; retroId: string; currentUserEmail: string; isHost: boolean };

export function InvitePanel(props: InvitePanelProps) {
  if (props.mode === "create") {
    return <CreateInvitePanel onInviteesChange={props.onInviteesChange} />;
  }
  return (
    <BoardInvitePanel
      retroId={props.retroId}
      currentUserEmail={props.currentUserEmail}
      isHost={props.isHost}
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
}: {
  retroId: string;
  currentUserEmail: string;
  isHost: boolean;
}) {
  const [grants, setGrants] = useState<Grant[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [staged, setStaged] = useState<Array<Picked & { role: "host" | "member" }>>([]);

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
    const next = [...staged];
    for (const u of users) {
      if (
        !next.some((p) => p.email === u.email) &&
        !grants.some((g) => g.principal_email === u.email)
      ) {
        next.push({ ...u, role: "member" });
      }
    }
    setStaged(next);
  }

  function handleStagedRoleChange(email: string, role: "host" | "member") {
    setStaged(staged.map((p) => (p.email === email ? { ...p, role } : p)));
  }

  function handleStagedRemove(email: string) {
    setStaged(staged.filter((p) => p.email !== email));
  }

  async function handleConfirmInvites() {
    try {
      await Promise.all(staged.map((u) => addGrantAction(retroId, u.email, u.role)));
      setStaged([]);
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
      {staged.length > 0 && (
        <div className="space-y-2">
          <div className="flex flex-wrap gap-2">
            {staged.map((u) => (
              <MemberBadge
                key={u.email}
                email={u.email}
                label={u.name || u.email.split("@")[0]}
                role={u.role}
                onRemove={handleStagedRemove}
                onRoleChange={handleStagedRoleChange}
              />
            ))}
          </div>
          <button
            type="button"
            onClick={handleConfirmInvites}
            className="rounded-[8px] border border-spill-well bg-spill-well/10 px-3 py-1.5 text-[11px] font-bold text-spill-well transition hover:bg-spill-well/20"
          >
            Invite {staged.length === 1 ? "1 person" : `${staged.length} people`}
          </button>
        </div>
      )}
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
      {isSelf && retroId && (
        <button
          type="button"
          title="Leave this board session"
          onClick={() => leaveRetroAction(retroId)}
          className="ml-0.5 rounded px-1 py-0.5 text-[9px] font-extrabold uppercase tracking-[0.08em] text-spill-muted transition hover:bg-spill-wrong/15 hover:text-spill-wrong"
        >
          leave
        </button>
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
