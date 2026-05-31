import Link from "next/link";
import { ThemeToggle } from "@/components/theme-toggle";
import type { CSSProperties, ReactNode } from "react";
import { IntentCardText, IntentSearch } from "./intent-controls";

export const spillColors = {
  mood: "#cf8a3f",
  well: "#2f9469",
  wrong: "#cf4f4f",
  action: "#8757b6",
  muted: "#86755a",
  paper: "#f3e8cf",
  panel: "#fbf3df",
  line: "#d9c89e",
} as const;

export type ColumnAccent = "mood" | "well" | "wrong" | "action";

export function AppChrome({
  title,
  subtitle,
  center,
  children,
  actions,
  presence,
}: {
  title?: string;
  subtitle?: ReactNode;
  center?: ReactNode;
  children: ReactNode;
  actions?: ReactNode;
  presence?: ReactNode;
}) {
  return (
    <main className="sp-paper flex min-h-dvh flex-col text-spill-fg">
      <TopBar title={title} subtitle={subtitle} center={center} actions={actions} presence={presence} />
      <section className="mx-auto flex w-full max-w-[1680px] flex-1 flex-col overflow-hidden">
        {children}
      </section>
    </main>
  );
}

export function TopBar({
  title,
  subtitle,
  center,
  actions,
  presence,
}: {
  title?: string;
  subtitle?: ReactNode;
  center?: ReactNode;
  actions?: ReactNode;
  presence?: ReactNode;
}) {
  return (
    <header className="sp-panel-grain relative h-14 shrink-0 border-b border-spill-line bg-spill-panel shadow-[inset_0_1px_0_rgba(255,255,255,0.45),inset_0_-1px_0_rgba(0,0,0,0.04)]">
      <div className="flex h-full w-full items-center gap-3 px-4 md:px-8 lg:px-10">
        <Link href="/" aria-label="Spill home">
          <SpillLogo />
        </Link>
        {title ? (
          <>
            <div className="h-6 w-px bg-spill-line" />
            <div className="min-w-0">
              <h1 className="truncate text-[13.5px] font-semibold leading-tight tracking-[-0.01em] text-spill-fg">{title}</h1>
              {subtitle ? <div className="flex min-w-0 items-center gap-1.5 truncate text-[10.5px] leading-tight text-spill-muted">{subtitle}</div> : null}
            </div>
          </>
        ) : null}
        <div className="ml-auto flex items-center gap-1.5 [&>form]:contents">
          <div className="hidden items-center md:flex">{presence}</div>
          {actions}
          <ThemeToggle />
        </div>
      </div>
      {center ? (
        <div className="pointer-events-none absolute inset-0 hidden items-center justify-center md:flex">
          <div className="pointer-events-auto">{center}</div>
        </div>
      ) : null}
    </header>
  );
}

export function SpillLogo({ compact = false, size = 18 }: { compact?: boolean; size?: number }) {
  const markSize = size * 1.22;
  return (
    <span className="inline-flex items-center gap-2">
      <svg width={markSize} height={markSize} viewBox="0 0 30 30" aria-hidden="true" className="block shrink-0">
        <defs>
          <linearGradient id="spill-logo-red" x1="0" x2="1" y1="0" y2="1">
            <stop offset="0" stopColor={spillColors.wrong} />
            <stop offset="1" stopColor="#a83232" />
          </linearGradient>
        </defs>
        <g transform="rotate(-22 15 16)">
          <rect x="6" y="8" width="14" height="13" rx="2" fill={spillColors.paper} stroke={spillColors.wrong === "#cf4f4f" ? "#1f1812" : spillColors.wrong} strokeWidth="1.6" />
          <path d="M20 11 q 4 0 4 4 q 0 4 -4 4" fill="none" stroke="#1f1812" strokeWidth="1.6" />
          <ellipse cx="13" cy="8" rx="7" ry="1.4" fill="#1f1812" />
        </g>
        <ellipse cx="22" cy="24" rx="6" ry="2" fill="url(#spill-logo-red)" stroke="#1f1812" strokeWidth="0.9" />
        <circle cx="17" cy="21" r="1.4" fill="url(#spill-logo-red)" stroke="#1f1812" strokeWidth="0.6" />
        <circle cx="26" cy="20" r="1" fill="url(#spill-logo-red)" stroke="#1f1812" strokeWidth="0.5" />
      </svg>
      {!compact ? (
        <span className="inline-flex items-baseline font-extrabold leading-none tracking-[-0.065em] text-spill-fg" style={{ fontSize: size }}>
          Spill
          <span className="ml-0.5 translate-y-[2px] rounded-full bg-spill-wrong" style={{ width: size * 0.18, height: size * 0.18 }} />
        </span>
      ) : null}
    </span>
  );
}

export function Btn({
  children,
  href,
  kind = "ghost",
  accent = spillColors.wrong,
  type,
  form,
  disabled,
  className = "",
  style,
  "aria-label": ariaLabel,
  title,
}: {
  children: ReactNode;
  href?: string;
  kind?: "primary" | "secondary" | "ghost" | "dashed";
  accent?: string;
  type?: "button" | "submit";
  form?: string;
  disabled?: boolean;
  className?: string;
  style?: CSSProperties;
  "aria-label"?: string;
  title?: string;
}) {
  const base =
    "inline-flex h-8 items-center justify-center gap-1.5 whitespace-nowrap rounded-[8px] border px-3 text-[12.5px] font-semibold leading-none transition hover:brightness-[0.98] focus-visible:outline-none focus-visible:shadow-[var(--focus)] disabled:pointer-events-none disabled:opacity-45";
  const variants: Record<NonNullable<Parameters<typeof Btn>[0]["kind"]>, string> = {
    primary: "border-[color:var(--btn-border)] bg-[linear-gradient(180deg,var(--btn-accent)_0%,var(--btn-shade)_100%)] text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.22),0_1px_0_rgba(74,52,20,0.12),0_2px_6px_var(--btn-glow)]",
    secondary: "border-spill-line bg-[var(--panel-hi)] text-[var(--fg-2)] shadow-[inset_0_1px_0_rgba(255,255,255,0.6),0_1px_0_rgba(74,52,20,0.06)]",
    ghost: "border-spill-line bg-[var(--paper)] text-[var(--fg-2)] shadow-[inset_0_1px_0_rgba(255,255,255,0.5)]",
    dashed: "border-dashed border-spill-line bg-transparent text-spill-muted shadow-none",
  };
  const cssVars = {
    color: kind === "primary" ? "#fffaf0" : undefined,
    "--btn-accent": accent,
    "--btn-shade": shade(accent, -8),
    "--btn-border": shade(accent, -16),
    "--btn-glow": `${accent}40`,
    ...style,
  } as CSSProperties;
  const classes = `${base} ${variants[kind]} ${className}`;

  if (href) {
    return (
      <Link aria-label={ariaLabel} className={classes} href={href} style={cssVars} title={title}>
        {children}
      </Link>
    );
  }

  return (
    <button aria-label={ariaLabel} className={classes} disabled={disabled} form={form} style={cssVars} title={title} type={type ?? "button"}>
      {children}
    </button>
  );
}

export function Pill({
  children,
  href,
  tone = "neutral",
  accent = spillColors.wrong,
  dashed = false,
  type,
  disabled,
  className = "",
}: {
  children: ReactNode;
  href?: string;
  tone?: "neutral" | "solid" | "soft" | "ghost" | "danger" | "success" | "action" | "mood";
  accent?: string;
  dashed?: boolean;
  type?: "button" | "submit";
  disabled?: boolean;
  className?: string;
}) {
  const actualAccent =
    tone === "danger" ? spillColors.wrong : tone === "success" ? spillColors.well : tone === "action" ? spillColors.action : tone === "mood" ? spillColors.mood : accent;
  const normalizedTone = tone === "danger" || tone === "success" || tone === "action" || tone === "mood" ? "solid" : tone;
  const variants = {
    neutral: "border-spill-line bg-[var(--panel-hi)] text-[var(--fg-2)]",
    solid: "border-[color:var(--pill-border)] bg-[var(--pill-accent)] text-white",
    soft: "border-[color:var(--pill-soft-border)] bg-[var(--pill-soft-bg)] text-[var(--pill-accent)]",
    ghost: "border-[var(--line-2)] bg-transparent text-spill-muted",
  };
  const cssVars = {
    "--pill-accent": actualAccent,
    "--pill-border": shade(actualAccent, -14),
    "--pill-soft-bg": `${actualAccent}1f`,
    "--pill-soft-border": `${actualAccent}55`,
  } as CSSProperties;
  const classes = `inline-flex items-center justify-center gap-1.5 whitespace-nowrap rounded-full border px-2.5 py-1 text-[11px] font-semibold leading-[1.3] tracking-[0.01em] ${dashed ? "border-dashed" : ""} ${variants[normalizedTone]} ${className}`;

  if (href) {
    return (
      <Link className={classes} href={href} style={cssVars}>
        {children}
      </Link>
    );
  }

  return (
    <button className={classes} disabled={disabled} style={cssVars} type={type ?? "button"}>
      {children}
    </button>
  );
}

export function Avatar({
  k,
  color = spillColors.muted,
  size = 26,
  status,
  ring = "var(--panel)",
}: {
  k: string;
  color?: string;
  size?: number;
  status?: "ready" | "writing" | "voting" | "away";
  ring?: string;
}) {
  const statusColor = status === "ready" ? spillColors.well : status === "writing" ? spillColors.mood : status === "voting" ? spillColors.action : status === "away" ? spillColors.muted : undefined;
  return (
    <span
      className="relative inline-flex shrink-0 items-center justify-center rounded-full text-white shadow-[0_1px_2px_rgba(0,0,0,0.12),inset_0_1px_0_rgba(255,255,255,0.15)]"
      style={{
        width: size,
        height: size,
        border: `2px solid ${ring}`,
        background: `linear-gradient(135deg, ${color} 0%, ${shade(color, -18)} 100%)`,
        fontSize: size * 0.38,
        fontWeight: 700,
        letterSpacing: -0.3,
      }}
      title={status}
    >
      {k.slice(0, 2).toLowerCase()}
      {statusColor ? (
        <span
          className={status === "writing" || status === "voting" ? "sp-live-dot absolute" : "absolute rounded-full"}
          style={{
            right: -1,
            bottom: -1,
            width: size * 0.34,
            height: size * 0.34,
            border: `1.6px solid ${ring}`,
            background: statusColor,
          }}
        />
      ) : null}
    </span>
  );
}

export function avatarInitials(label: string | null | undefined) {
  const value = label?.trim();
  if (!value) return "??";

  const base = value.includes("@") ? value.split("@")[0] : value;
  const parts = base.split(/[\s._-]+/).filter(Boolean);
  if (parts.length >= 2) {
    return `${parts[0][0]}${parts[1][0]}`.toLowerCase();
  }
  return (parts[0] ?? base).slice(0, 2).toLowerCase();
}

// "Alice Grossi" -> "Alice G.", "alice.g" -> "Alice G.", "alice" -> "Alice"
// Used on cards to attribute the writer without burning two lines on a
// long full name.
export function shortAuthorName(label: string | null | undefined): string {
  const value = label?.trim();
  if (!value) return "anonymous";
  const base = value.includes("@") ? value.split("@")[0] : value;
  const parts = base.split(/[\s._-]+/).filter(Boolean);
  const first = capitalize(parts[0]);
  if (parts.length === 1) return first;
  const last = parts[parts.length - 1];
  return `${first} ${last[0].toUpperCase()}.`;
}

function capitalize(value: string): string {
  if (!value) return value;
  return value[0].toUpperCase() + value.slice(1);
}

export function avatarColorForSeed(seed: string | null | undefined) {
  const colors = [spillColors.wrong, spillColors.mood, spillColors.action, spillColors.well, spillColors.muted];
  const value = seed?.trim() || "spill-user";
  let hash = 0;
  for (let index = 0; index < value.length; index += 1) {
    hash = (hash << 5) - hash + value.charCodeAt(index);
    hash |= 0;
  }
  return colors[Math.abs(hash) % colors.length];
}

export function Stack({
  people,
  size = 26,
  ring = "var(--panel)",
}: {
  people: { k: string; color: string; status?: "ready" | "writing" | "voting" | "away" }[];
  size?: number;
  ring?: string;
}) {
  return (
    <div className="inline-flex">
      {people.map((person, index) => (
        <span className={index ? "-ml-2" : ""} key={`${person.k}-${index}`}>
          <Avatar {...person} ring={ring} size={size} />
        </span>
      ))}
    </div>
  );
}

export function SectionTitle({ children, kicker }: { children: ReactNode; kicker?: ReactNode }) {
  return (
    <div className="flex items-baseline gap-3">
      <h2 className="m-0 text-[32px] font-extrabold leading-none tracking-[-0.025em] text-spill-fg">{children}</h2>
      {kicker ? <p className="-rotate-2 font-hand text-[22px] leading-none text-spill-muted">{kicker}</p> : null}
    </div>
  );
}

export function Tile({
  children,
  className = "",
  style,
  hi = false,
  id,
}: {
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
  hi?: boolean;
  id?: string;
}) {
  return (
    <div className={`sp-panel-grain rounded-[10px] border border-spill-line p-3.5 shadow-[var(--shadow-1)] ${hi ? "bg-[var(--panel-hi)]" : "bg-spill-panel"} ${className}`} id={id} style={style}>
      {children}
    </div>
  );
}

export function SpillCard({
  accent,
  children,
  className = "",
  style,
}: {
  accent: string;
  children: ReactNode;
  className?: string;
  style?: CSSProperties;
}) {
  return (
    <div
      className={`sp-card-grain rounded-[8px] p-3 text-[13.5px] font-medium leading-[1.38] text-white shadow-[var(--shadow-2),var(--card-inset-hi),var(--card-inset-lo)] ${className}`}
      style={{
        background: `linear-gradient(180deg, ${shade(accent, 4)} 0%, ${accent} 60%, ${shade(accent, -6)} 100%)`,
        "--card-button-fg": accent,
        ...style,
      } as CSSProperties}
    >
      {children}
    </div>
  );
}

export function CardFooter({
  author = "??",
  authorName,
  color = spillColors.muted,
  tag,
  trailing,
  votes,
}: {
  author?: string;
  authorName?: string;
  color?: string;
  tag?: string;
  trailing?: ReactNode;
  votes?: number;
}) {
  return (
    <div className="mt-2 flex items-center gap-1.5 border-t border-white/20 pt-1.5 text-[11px]">
      <Avatar k={author} color={color} ring="rgba(255,255,255,0.55)" size={18} />
      {authorName ? (
        <span className="truncate text-[11px] font-semibold text-white/85" title={authorName}>
          {authorName}
        </span>
      ) : null}
      {tag ? <span className="rounded-full bg-white/20 px-2 py-0.5 text-[9.5px] font-semibold tracking-[0.03em] text-white">#{tag}</span> : null}
      <div className="flex-1" />
      {trailing ?? (votes !== undefined ? (
        <span className="inline-flex h-6 min-w-6 items-center justify-center rounded-full bg-white px-2 text-[10.5px] font-bold text-[var(--card-button-fg)]" aria-label={`${votes} total votes`}>
          {votes === 0 ? "no votes" : `${votes} ${votes === 1 ? "vote" : "votes"}`}
        </span>
      ) : null)}
    </div>
  );
}

export function ColumnHeader({
  name,
  count,
  accent,
  sub,
}: {
  name: string;
  count: ReactNode;
  accent: string;
  sub?: string;
}) {
  return (
    <div className="flex items-baseline gap-2 px-0.5 pb-2.5">
      <span className="h-2 w-2 self-center rounded-full shadow-[0_0_0_3px_var(--col-glow)]" style={{ backgroundColor: accent, "--col-glow": `${accent}22` } as CSSProperties} />
      <span className="text-[13.5px] font-bold leading-none tracking-[-0.01em] text-spill-fg">{name}</span>
      {sub ? <span className="text-[11px] italic text-spill-muted">{sub}</span> : null}
      <span className="ml-auto rounded-full border border-spill-line bg-[var(--panel-hi)] px-2 py-0.5 text-[11px] font-semibold text-spill-muted">{count}</span>
    </div>
  );
}

export function HiddenDraft({ accent }: { accent: string }) {
  return (
    <div
      className="grid h-[52px] place-items-center rounded-[8px] border border-dashed text-[11px] font-semibold italic tracking-[0.03em]"
      style={{
        color: accent,
        borderColor: `${accent}55`,
        background: `repeating-linear-gradient(45deg, ${accent}14 0 8px, ${accent}06 8px 16px)`,
      }}
    >
      . . . someone's draft . . .
    </div>
  );
}

export function GifTile({ label = "GIF", className = "" }: { label?: string; className?: string }) {
  return (
    <div className={`relative h-[70px] overflow-hidden rounded-[6px] bg-[radial-gradient(circle_at_30%_40%,#f4cdb0,#a85a3a_70%)] shadow-[inset_0_0_0_1px_rgba(255,255,255,0.15),inset_0_-8px_16px_rgba(0,0,0,0.18)] ${className}`}>
      <span className="absolute left-1 top-1 rounded-[3px] bg-black/45 px-1.5 py-0.5 text-[9px] font-bold tracking-[0.05em] text-white backdrop-blur">{label}</span>
    </div>
  );
}

export function Field({
  label,
  hint,
  children,
  className = "",
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <label className={`block ${className}`}>
      <span className="flex items-baseline justify-between gap-3">
        <span className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">{label}</span>
        {hint ? <span className="text-[11px] text-spill-muted">{hint}</span> : null}
      </span>
      <span className="mt-1.5 block">{children}</span>
    </label>
  );
}

export const fieldControlClass =
  "min-h-10 rounded-[8px] border border-spill-line bg-[var(--panel-hi)] px-3 py-2 text-[13px] font-semibold text-spill-fg shadow-[inset_0_1px_0_rgba(255,255,255,0.55)] outline-none placeholder:text-spill-muted focus:border-spill-wrong focus:shadow-[var(--focus)]";

export const chipButtonClass =
  "inline-flex h-7 min-w-7 items-center justify-center gap-1.5 whitespace-nowrap rounded-[7px] border border-spill-line bg-[var(--paper)] px-2.5 text-[11.5px] font-extrabold leading-none text-[var(--fg-2)] shadow-[inset_0_1px_0_rgba(255,255,255,0.55),0_1px_0_rgba(74,52,20,0.06)] transition hover:brightness-[0.98] focus-visible:outline-none focus-visible:shadow-[var(--focus)] disabled:pointer-events-none disabled:opacity-45";

export const cardButtonClass =
  "inline-flex h-6 min-w-6 items-center justify-center gap-1 whitespace-nowrap rounded-[999px] border border-white/45 bg-white px-2.5 text-[10.5px] font-extrabold leading-none text-[var(--card-button-fg)] shadow-[0_1px_2px_rgba(0,0,0,0.14)] transition hover:brightness-[0.98] disabled:pointer-events-none disabled:opacity-45";

export const cardGhostButtonClass =
  "inline-flex h-6 min-w-6 items-center justify-center gap-1 whitespace-nowrap rounded-[999px] border border-white/25 bg-white/20 px-2.5 text-[10.5px] font-extrabold leading-none text-white transition hover:bg-white/25";

export function SearchField({
  name = "q",
  defaultValue,
  placeholder = "Search boards",
}: {
  name?: string;
  defaultValue?: string;
  placeholder?: string;
}) {
  return (
    <div className="relative w-full">
      <svg className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-spill-muted" viewBox="0 0 20 20" fill="none" aria-hidden="true">
        <circle cx="8.5" cy="8.5" r="5.25" stroke="currentColor" strokeWidth="2" />
        <path d="m12.5 12.5 4 4" stroke="currentColor" strokeLinecap="round" strokeWidth="2" />
      </svg>
      <IntentSearch className={`${fieldControlClass} pl-8`} defaultValue={defaultValue} name={name} placeholder={placeholder} style={{ width: "100%" }} />
    </div>
  );
}

export function CardComposer({
  retroId,
  columnId,
  placeholder,
  accent,
  draftText = "",
  before,
  after,
  actions,
}: {
  retroId: string;
  columnId: string;
  placeholder: string;
  accent: string;
  draftText?: string;
  before?: ReactNode;
  after?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <div className="sp-card-grain w-full min-w-0 overflow-hidden rounded-[8px] p-3 text-white shadow-[0_0_0_3px_var(--composer-glow),var(--shadow-2)]" style={{ background: `linear-gradient(180deg, ${shade(accent, 4)} 0%, ${accent} 100%)`, "--card-button-fg": accent, "--composer-glow": `${accent}33` } as CSSProperties}>
      <input name="retro_id" type="hidden" value={retroId} />
      <input name="column_id" type="hidden" value={columnId} />
      {before}
      <IntentCardText
        className="block min-h-[76px] w-full resize-none rounded-[6px] border border-white/35 bg-black/15 px-3 py-2 text-[13.5px] font-medium leading-5 text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.12)] placeholder:font-hand placeholder:text-2xl placeholder:font-bold placeholder:text-white/95 focus:border-white/60 focus:shadow-none"
        defaultValue={draftText}
        name="body_text"
        placeholder={placeholder}
        rows={3}
      />
      {after}
      <div className="mt-2.5 flex flex-wrap items-center gap-1.5">
        <div className="flex-1" />
        {actions}
      </div>
    </div>
  );
}

export function PhaseBadge({ phase, color }: { phase: string; color?: string }) {
  return (
    <span className="inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-[10px] font-extrabold uppercase tracking-[0.09em] text-white shadow-[0_2px_4px_rgba(0,0,0,0.12)]" style={{ backgroundColor: color ?? spillColors.muted }}>
      {phase}
    </span>
  );
}

export function phaseColor(phase: string) {
  if (phase === "scheduled") return spillColors.mood;
  if (phase === "writing") return spillColors.mood;
  if (phase === "discussion") return spillColors.action;
  if (phase === "voting") return spillColors.action;
  if (phase === "action_discussion") return spillColors.wrong;
  if (phase === "completed") return spillColors.well;
  return spillColors.muted;
}

export function phaseLabel(phase: string) {
  if (phase === "scheduled") return "planned";
  if (phase === "discussion") return "review";
  if (phase === "action_discussion") return "action";
  if (phase === "completed") return "done";
  return phase.replaceAll("_", " ");
}

export function shade(hex: string, percent: number) {
  const c = hex.replace("#", "");
  const num = Number.parseInt(c, 16);
  let r = (num >> 16) + Math.round((255 * percent) / 100);
  let g = ((num >> 8) & 0xff) + Math.round((255 * percent) / 100);
  let b = (num & 0xff) + Math.round((255 * percent) / 100);
  r = Math.max(0, Math.min(255, r));
  g = Math.max(0, Math.min(255, g));
  b = Math.max(0, Math.min(255, b));
  return `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
}
