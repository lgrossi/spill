import { Tile } from "@/components/spill-ui";
import type { AiArtifact } from "@/lib/contracts";
import { retryAiJobAction } from "@/lib/actions";

/**
 * AI wrap-up tile.
 *
 * Server component that reads the summary AI artifact (kind === "summary")
 * off the board payload — the backend auto-triggers it when the retro
 * is completed and the board WebSocket pushes status updates as it
 * progresses (board-sync.tsx calls router.refresh() on CardChanged,
 * which re-renders this server component with the new artifact state).
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
}: {
  retroId: string;
  artifacts: AiArtifact[];
}) {
  const summary = artifacts.find((artifact) => artifact.kind === "summary");
  if (!summary) {
    return null;
  }

  return (
    <Tile className="border-spill-action/50 bg-spill-action/10">
      <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-action">
        AI wrap-up
      </p>
      {summary.status === "pending" || summary.status === "running" ? (
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
    </Tile>
  );
}

function summaryText(output: unknown): string | null {
  if (!output || typeof output !== "object") return null;
  const value = (output as Record<string, unknown>).summary;
  return typeof value === "string" ? value : null;
}
