"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

/**
 * Client-side refresh loop for the AI wrap-up tile.
 *
 * Mounted only while the summary artifact is in a non-terminal state
 * (`pending` / `running`). The board-sync WebSocket would normally
 * trigger refreshes, but in deployments where the WS path is not
 * reachable from the browser (cross-host IAP, no public WS endpoint)
 * the tile would otherwise stay on "Generating summary…" indefinitely
 * even though the artifact has long since landed in the database.
 *
 * Calls `router.refresh()` on a steady cadence; the parent server
 * component re-renders and stops mounting this component once the
 * artifact reaches a terminal status. A hard ceiling prevents an
 * abandoned runner from polling forever.
 */
const TICK_MS = 2500;
const MAX_TICKS = 96; // ~4 minutes — well past the 8s gateway budget + retry headroom.

export function AiWrapTilePoller() {
  const router = useRouter();

  useEffect(() => {
    let ticks = 0;
    const id = window.setInterval(() => {
      ticks += 1;
      router.refresh();
      if (ticks >= MAX_TICKS) {
        window.clearInterval(id);
      }
    }, TICK_MS);

    return () => window.clearInterval(id);
  }, [router]);

  return null;
}
