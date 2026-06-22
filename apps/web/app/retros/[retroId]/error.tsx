"use client";

import { useEffect } from "react";
import type { CSSProperties } from "react";
import { AppChrome, shade, spillColors, Tile } from "@/components/spill-ui";

// Route error boundary for /retros/[retroId]. The board page rerenders on
// every WebSocket event and on a safety poll, so a single transient API blip
// (gateway timeout, cold start, network jitter) could repaint a working board
// as a 404 / crash. This boundary catches non-404 failures from loadBoard and
// lets the user retry instead of seeing a dead end.
//
// Intentionally no auto-retry: if the API is truly down, hammering reset()
// in a loop helps nobody. A single click on Retry re-runs the server
// component, which already retries once internally before throwing here.
export default function BoardError({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    // Surface the underlying cause in the browser console for debugging.
    // Next strips messages on the server-side digest, so console is the only
    // place a user-side dev tool can see what actually failed.
    console.error("[retro-board] render failed", error);
  }, [error]);

  return (
    <AppChrome>
      <div className="flex flex-1 items-center justify-center p-6">
        <Tile className="w-full max-w-lg">
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">
            connection hiccup
          </p>
          <h1 className="mt-3 text-[24px] font-extrabold tracking-[-0.02em] text-spill-fg">
            Lost the board for a sec.
          </h1>
          <p className="mt-2 text-[13.5px] leading-6 text-[var(--fg-2)]">
            The API didn&apos;t answer in time. Your work is safe — drafts stay on
            the server. Try again in a moment.
          </p>
          <div className="mt-4">
            <button
              className="inline-flex h-8 items-center justify-center gap-1.5 whitespace-nowrap rounded-[8px] border border-[color:var(--btn-border)] bg-[linear-gradient(180deg,var(--btn-accent)_0%,var(--btn-shade)_100%)] px-3 text-[12.5px] font-semibold leading-none text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.22),0_1px_0_rgba(74,52,20,0.12),0_2px_6px_var(--btn-glow)] transition hover:brightness-[0.98] focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
              onClick={reset}
              style={retryButtonStyle}
              type="button"
            >
              retry
            </button>
          </div>
        </Tile>
      </div>
    </AppChrome>
  );
}

const retryButtonStyle: CSSProperties = {
  "--btn-accent": spillColors.well,
  "--btn-shade": shade(spillColors.well, -8),
  "--btn-border": shade(spillColors.well, -16),
  "--btn-glow": `${spillColors.well}40`,
} as CSSProperties;
