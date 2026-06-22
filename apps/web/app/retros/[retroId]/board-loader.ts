import { ApiError, getRetro, type RetroBoard } from "@/lib/api";

const TRANSIENT_RETRY_DELAY_MS = 400;

// The board page rerenders on every WebSocket event / safety poll (see
// BoardSync). A single transient hiccup (cold start, gateway timeout, token
// mint blip, network jitter) previously surfaced as a hard 404 because every
// non-403 was collapsed into null. Classify the failure instead:
//
//   - 404 from the API => real "board does not exist", caller renders 404
//   - 403 => forbidden screen
//   - anything else => retry once with a short backoff; if it still fails,
//     throw so the route's error.tsx renders a retry screen instead of
//     silently 404'ing a working board.
export async function loadBoard(
  retroId: string,
): Promise<RetroBoard | "forbidden" | null> {
  try {
    return await getRetro(retroId);
  } catch (firstError) {
    const classified = classify(firstError);
    if (classified !== "transient") return throwOrReturn(firstError, classified);

    await sleep(TRANSIENT_RETRY_DELAY_MS);
    try {
      return await getRetro(retroId);
    } catch (secondError) {
      const secondClassified = classify(secondError);
      if (secondClassified !== "transient") {
        return throwOrReturn(secondError, secondClassified);
      }
      throw secondError;
    }
  }
}

type Classification = "missing" | "forbidden" | "transient";

function classify(error: unknown): Classification {
  if (error instanceof ApiError) {
    if (error.status === 404) return "missing";
    if (error.status === 403) return "forbidden";
  }
  return "transient";
}

// Narrow helper so the two retry branches share a single mapping table.
function throwOrReturn(
  error: unknown,
  classification: Classification,
): RetroBoard | "forbidden" | null {
  if (classification === "missing") return null;
  if (classification === "forbidden") return "forbidden";
  throw error;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
