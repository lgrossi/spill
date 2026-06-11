import { describe, it, expect } from "vitest";
import { loopbackTarget } from "../app/lib/loopback";

describe("loopbackTarget", () => {
  it("accepts loopback http callbacks", () => {
    for (const cb of [
      "http://127.0.0.1:51234",
      "http://localhost:8080/cb",
      "http://[::1]:9000",
    ]) {
      expect(loopbackTarget(cb)?.toString()).toBeDefined();
    }
  });

  it("rejects non-loopback or non-http callbacks", () => {
    for (const cb of [
      "https://127.0.0.1:443",
      "http://evil.example.com:51234",
      "http://user:pass@127.0.0.1:51234",
      "http://127.0.0.1.evil.com",
      "ftp://127.0.0.1",
      "not a url",
      "",
    ]) {
      expect(loopbackTarget(cb)).toBeNull();
    }
  });
});
