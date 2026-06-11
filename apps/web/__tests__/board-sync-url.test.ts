import { describe, expect, it } from "vitest";
import { resolveApiBaseUrl, toWebSocketUrl } from "../app/retros/[retroId]/board-sync";

describe("board sync websocket URL", () => {
  it("uses the browser API URL when configured", () => {
    expect(resolveApiBaseUrl("https://api.example.test")).toBe("https://api.example.test");
    expect(toWebSocketUrl("https://api.example.test")).toBe("wss://api.example.test");
  });

  it("falls back to the local API port, not the web page origin", () => {
    expect(resolveApiBaseUrl("")).toBe("http://127.0.0.1:4000");
    expect(toWebSocketUrl(resolveApiBaseUrl(""))).toBe("ws://127.0.0.1:4000");
  });
});
