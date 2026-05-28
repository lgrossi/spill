"use client";

import { useEffect, useState } from "react";

const COOKIE = "spillio-theme";
const ONE_YEAR = 60 * 60 * 24 * 365;

function readTheme(): "light" | "dark" {
  if (typeof document === "undefined") return "light";
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

function writeCookie(value: "light" | "dark") {
  document.cookie = `${COOKIE}=${value}; path=/; max-age=${ONE_YEAR}; samesite=lax`;
}

function SunIcon() {
  return (
    <svg aria-hidden="true" fill="none" height="14" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" viewBox="0 0 24 24" width="14">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg aria-hidden="true" fill="none" height="14" stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth="1.8" viewBox="0 0 24 24" width="14">
      <path d="M21 12.79A9 9 0 1 1 11.21 3a7 7 0 0 0 9.79 9.79z" />
    </svg>
  );
}

export function ThemeToggle() {
  // Initial state is light so SSR + first client render agree; the effect
  // immediately reconciles to the real attribute on mount.
  const [theme, setTheme] = useState<"light" | "dark">("light");
  useEffect(() => {
    setTheme(readTheme());
  }, []);

  function toggle() {
    const next = theme === "dark" ? "light" : "dark";
    document.documentElement.dataset.theme = next;
    writeCookie(next);
    setTheme(next);
  }

  const label = theme === "dark" ? "switch to light mode" : "switch to dark mode";

  return (
    <button
      aria-label={label}
      className="grid h-7 w-7 place-items-center rounded-full border border-spill-line bg-[var(--panel-hi)] text-[var(--fg-2)] shadow-[inset_0_1px_0_rgba(255,255,255,0.45),0_1px_0_rgba(74,52,20,0.06)] transition hover:border-[var(--line-2)] focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
      onClick={toggle}
      title={label}
      type="button"
    >
      {theme === "dark" ? <SunIcon /> : <MoonIcon />}
    </button>
  );
}
