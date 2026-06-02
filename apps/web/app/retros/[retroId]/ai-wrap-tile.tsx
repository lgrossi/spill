import { Tile } from "@/components/spill-ui";
import type { AiArtifact } from "@/lib/contracts";
import { retryAiJobAction } from "@/lib/actions";
import { AiWrapTilePoller } from "./ai-wrap-tile-poller";

/**
 * AI wrap-up tile.
 *
 * Server component that reads the summary AI artifact (kind === "summary")
 * off the board payload — the backend auto-triggers it when the retro
 * is completed. Status updates reach the UI through two paths:
 *   1. The board-sync WebSocket — refreshes the page when the runner
 *      publishes `card_changed` on terminal status.
 *   2. `AiWrapTilePoller` — mounted only while the artifact is
 *      non-terminal, drives `router.refresh()` on a short cadence as
 *      a fallback for deployments where the WS path is not reachable
 *      from the browser. Self-disposes once this component re-renders
 *      with a terminal status (so no longer mounts the poller).
 *
 * Visibility:
 *   - no artifact          → tile hidden (AI provider not configured,
 *                            or auto-trigger has not run yet)
 *   - pending / running    → "Generating…" placeholder
 *   - succeeded            → render the summary text
 *   - failed               → error message + retry form (server action)
 */
export function AiWrapTile({
  retroId,
  artifacts,
  categories = [],
}: {
  retroId: string;
  artifacts: AiArtifact[];
  categories?: string[];
}) {
  const summary = artifacts.find((artifact) => artifact.kind === "summary");
  if (!summary) {
    return null;
  }
  const isPending = summary.status === "pending" || summary.status === "running";

  return (
    <Tile className="border-spill-action/50 bg-spill-action/10">
      <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-action">
        AI Summary
      </p>
      {categories.length > 0 ? (
        <div className="mt-2 flex flex-wrap gap-1.5">
          {categories.slice(0, 2).map((category) => (
            <span className="rounded-full border border-spill-action/30 bg-white/40 px-2.5 py-1 text-[10.5px] font-extrabold text-spill-action" key={category}>
              #{category}
            </span>
          ))}
        </div>
      ) : null}
      {isPending ? (
        <p className="mt-2 text-[11.5px] leading-5 text-spill-muted">
          Generating summary…
        </p>
      ) : null}
      {summary.status === "succeeded" ? (
        <p className="mt-2 whitespace-pre-wrap text-[12.5px] leading-5 text-spill-fg">
          {summaryText(summary.output) ?? "(empty summary)"}
        </p>
      ) : null}
      {summary.status === "failed" ? (
        <>
          <p className="mt-2 text-[11.5px] leading-5 text-spill-wrong">
            Could not generate the summary
            {summary.error_message ? `: ${summary.error_message}` : "."}
          </p>
          <form action={retryAiJobAction} className="mt-3">
            <input type="hidden" name="retro_id" value={retroId} />
            <input type="hidden" name="artifact_id" value={summary.id} />
            <button
              aria-label="retry summary"
              className="rounded-[8px] border border-spill-action/40 bg-spill-action/10 px-2.5 py-1 text-[11.5px] font-semibold text-spill-action transition hover:bg-spill-action/20"
              type="submit"
            >
              retry
            </button>
          </form>
        </>
      ) : null}
      {isPending ? <AiWrapTilePoller /> : null}
    </Tile>
  );
}

function summaryText(output: unknown): string | null {
  if (!output || typeof output !== "object") return null;
  const value = (output as Record<string, unknown>).summary;
  return typeof value === "string" ? value : null;
}
