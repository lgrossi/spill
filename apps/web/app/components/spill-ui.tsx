import Link from "next/link";
import type { ReactNode } from "react";

export const spillColors = {
  mood: "#d49a5c",
  well: "#3aa676",
  wrong: "#dd5c5c",
  action: "#9e6cc4",
  muted: "#7a6c54",
} as const;

export function SpillLogo({ compact = false }: { compact?: boolean }) {
  return (
    <div className="flex items-center gap-2">
      <SpilledMug className="h-8 w-8" />
      {!compact ? (
        <span className="inline-flex items-baseline text-2xl font-extrabold tracking-[-0.08em] text-spill-fg">
          Spill
          <span className="ml-1 h-2.5 w-2.5 translate-y-0.5 rounded-full bg-spill-wrong shadow-[3px_4px_0_rgba(221,92,92,0.45)]" />
        </span>
      ) : null}
    </div>
  );
}

export function SpilledMug({ className = "h-10 w-10" }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 100 100" aria-hidden="true">
      <g transform="rotate(-26 50 56)">
        <rect x="22" y="32" width="44" height="40" rx="3" fill="#f4ead4" stroke="#2a221b" strokeWidth="3" />
        <path d="M66 40 q 12 0 12 12 q 0 12 -12 12" fill="none" stroke="#2a221b" strokeWidth="3" />
        <ellipse cx="44" cy="32" rx="22" ry="4" fill="#f4ead4" stroke="#2a221b" strokeWidth="2.4" />
        <ellipse cx="44" cy="32" rx="18" ry="3" fill="#3b2818" />
      </g>
      <path d="M58 64 q 8 14 22 18 q 14 4 18 12" fill="none" stroke="#dd5c5c" strokeWidth="4" strokeLinecap="round" />
      <ellipse cx="82" cy="86" rx="14" ry="5" fill="#dd5c5c" stroke="#2a221b" strokeWidth="2" />
      <circle cx="68" cy="74" r="3" fill="#dd5c5c" stroke="#2a221b" strokeWidth="1.4" />
      <circle cx="95" cy="80" r="2.4" fill="#dd5c5c" stroke="#2a221b" strokeWidth="1.4" />
    </svg>
  );
}

export function AppChrome({
  title,
  subtitle,
  children,
  actions,
}: {
  title: string;
  subtitle?: string;
  children: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <main className="min-h-dvh p-2 text-spill-fg">
      <section className="mx-auto min-h-[calc(100dvh-1rem)] max-w-[1600px] overflow-hidden rounded-lg border border-spill-line bg-spill-bg shadow-[0_8px_22px_rgba(42,34,27,0.12)]">
        <header className="flex h-16 items-center gap-4 border-b border-spill-line bg-spill-panel px-5">
          <Link href="/" className="flex items-center gap-3">
            <SpillLogo />
          </Link>
          <div className="h-7 w-px bg-spill-line" />
          <div className="min-w-0">
            <h1 className="truncate text-base font-bold leading-tight">{title}</h1>
            {subtitle ? <p className="truncate text-xs text-spill-muted">{subtitle}</p> : null}
          </div>
          <div className="ml-auto flex items-center gap-2">{actions}</div>
        </header>
        {children}
      </section>
    </main>
  );
}

export function Pill({
  children,
  href,
  tone = "neutral",
  dashed = false,
  type,
  disabled,
}: {
  children: ReactNode;
  href?: string;
  tone?: "neutral" | "danger" | "success" | "action" | "mood";
  dashed?: boolean;
  type?: "button" | "submit";
  disabled?: boolean;
}) {
  const color =
    tone === "danger"
      ? "border-spill-wrong bg-spill-wrong text-white"
      : tone === "success"
        ? "border-spill-well bg-spill-well text-white"
        : tone === "action"
          ? "border-spill-action bg-spill-action text-white"
          : tone === "mood"
            ? "border-spill-mood bg-spill-mood text-white"
            : "border-spill-line bg-transparent text-spill-fg";
  const className = `inline-flex items-center justify-center rounded-full border px-3 py-1.5 text-sm font-medium leading-none transition hover:brightness-95 ${dashed ? "border-dashed" : ""} ${color}`;
  if (href) {
    return (
      <Link href={href} className={className}>
        {children}
      </Link>
    );
  }
  return (
    <button className={className} type={type ?? "button"} disabled={disabled}>
      {children}
    </button>
  );
}

export function StatusPill({
  children,
  tone = "neutral",
}: {
  children: ReactNode;
  tone?: "neutral" | "danger" | "success" | "action" | "mood";
}) {
  const color =
    tone === "danger"
      ? "border-spill-wrong bg-spill-wrong text-white"
      : tone === "success"
        ? "border-spill-well bg-spill-well text-white"
        : tone === "action"
          ? "border-spill-action bg-spill-action text-white"
          : tone === "mood"
            ? "border-spill-mood bg-spill-mood text-white"
            : "border-spill-line bg-transparent text-spill-fg";
  return (
    <span className={`inline-flex items-center justify-center rounded-full border px-3 py-1.5 text-sm font-medium leading-none ${color}`}>
      {children}
    </span>
  );
}

export function SectionTitle({ children, kicker }: { children: ReactNode; kicker?: ReactNode }) {
  return (
    <div className="flex items-baseline gap-2">
      <h2 className="font-hand text-4xl leading-none text-spill-fg">{children}</h2>
      {kicker ? <p className="text-sm italic text-spill-muted">{kicker}</p> : null}
    </div>
  );
}

export function Tile({ children, className = "" }: { children: ReactNode; className?: string }) {
  return (
    <div className={`rounded-xl border border-spill-line bg-spill-panel p-4 shadow-[0_1px_0_#d4c39d,0_5px_12px_rgba(42,34,27,0.07)] ${className}`}>
      {children}
    </div>
  );
}

export function PhaseBadge({ phase, color }: { phase: string; color?: string }) {
  return (
    <span
      className="inline-flex rounded-full px-3 py-1 text-[10px] font-extrabold uppercase tracking-wider text-white"
      style={{ backgroundColor: color ?? spillColors.muted }}
    >
      {phase}
    </span>
  );
}

export function phaseColor(phase: string) {
  if (phase === "writing") return spillColors.mood;
  if (phase === "discussion") return spillColors.well;
  if (phase === "voting") return spillColors.action;
  if (phase === "action_discussion") return spillColors.wrong;
  if (phase === "completed") return spillColors.well;
  return spillColors.muted;
}
