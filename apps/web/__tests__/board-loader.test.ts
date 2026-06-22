import { afterEach, describe, expect, it, vi } from "vitest";
import { loadBoard } from "../app/retros/[retroId]/board-loader";
import * as api from "../app/lib/api";

const { ApiError } = api;

function fakeBoard(): api.RetroBoard {
  // Minimal shape: the loader returns whatever getRetro resolves with; tests
  // only assert object identity so the actual fields are irrelevant.
  return { sentinel: "board" } as unknown as api.RetroBoard;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("loadBoard", () => {
  it("returns the board when the API succeeds on the first call", async () => {
    const board = fakeBoard();
    const spy = vi.spyOn(api, "getRetro").mockResolvedValueOnce(board);

    const result = await loadBoard("retro-1");

    expect(result).toBe(board);
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it("returns null on a real 404 without retrying", async () => {
    const spy = vi
      .spyOn(api, "getRetro")
      .mockRejectedValueOnce(new ApiError("missing", 404));

    const result = await loadBoard("retro-1");

    expect(result).toBeNull();
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it("returns 'forbidden' on 403 without retrying", async () => {
    const spy = vi
      .spyOn(api, "getRetro")
      .mockRejectedValueOnce(new ApiError("denied", 403));

    const result = await loadBoard("retro-1");

    expect(result).toBe("forbidden");
    expect(spy).toHaveBeenCalledTimes(1);
  });

  it("retries once on a 5xx and returns the board if the retry succeeds", async () => {
    const board = fakeBoard();
    const spy = vi
      .spyOn(api, "getRetro")
      .mockRejectedValueOnce(new ApiError("upstream timeout", 504))
      .mockResolvedValueOnce(board);

    const result = await loadBoard("retro-1");

    expect(result).toBe(board);
    expect(spy).toHaveBeenCalledTimes(2);
  });

  it("retries once on a non-ApiError (network/identity) and returns on success", async () => {
    const board = fakeBoard();
    const spy = vi
      .spyOn(api, "getRetro")
      .mockRejectedValueOnce(new Error("network blip"))
      .mockResolvedValueOnce(board);

    const result = await loadBoard("retro-1");

    expect(result).toBe(board);
    expect(spy).toHaveBeenCalledTimes(2);
  });

  it("throws after a second transient failure instead of silently 404-ing", async () => {
    const spy = vi
      .spyOn(api, "getRetro")
      .mockRejectedValueOnce(new ApiError("upstream timeout", 502))
      .mockRejectedValueOnce(new ApiError("upstream timeout", 502));

    await expect(loadBoard("retro-1")).rejects.toThrow("upstream timeout");
    expect(spy).toHaveBeenCalledTimes(2);
  });

  it("respects a 404 surfaced on the retry attempt", async () => {
    const spy = vi
      .spyOn(api, "getRetro")
      .mockRejectedValueOnce(new Error("network blip"))
      .mockRejectedValueOnce(new ApiError("missing", 404));

    const result = await loadBoard("retro-1");

    expect(result).toBeNull();
    expect(spy).toHaveBeenCalledTimes(2);
  });

  it("respects a 403 surfaced on the retry attempt", async () => {
    const spy = vi
      .spyOn(api, "getRetro")
      .mockRejectedValueOnce(new Error("network blip"))
      .mockRejectedValueOnce(new ApiError("denied", 403));

    const result = await loadBoard("retro-1");

    expect(result).toBe("forbidden");
    expect(spy).toHaveBeenCalledTimes(2);
  });
});
