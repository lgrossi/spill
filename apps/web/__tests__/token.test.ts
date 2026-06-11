import { describe, it, expect } from "vitest";
import { createHmac } from "crypto";
import { mintToken } from "../app/lib/token";
import { WS_SUBPROTOCOL } from "../app/lib/ws-protocol";

function decode(part: string) {
  return JSON.parse(Buffer.from(part, "base64url").toString("utf8"));
}

describe("mintToken", () => {
  it("produces a verifiable HS256 JWT with the expected claims", () => {
    const secret = "test-secret";
    const token = mintToken(secret, { email: "ava@example.com", name: "Ava" }, 120);
    const [header, payload, signature] = token.split(".");

    expect(decode(header)).toEqual({ alg: "HS256", typ: "JWT" });
    const claims = decode(payload);
    expect(claims.email).toBe("ava@example.com");
    expect(claims.name).toBe("Ava");
    expect(claims.retro).toBeUndefined();
    expect(claims.exp).toBeGreaterThan(Math.floor(Date.now() / 1000));

    const expected = createHmac("sha256", secret)
      .update(`${header}.${payload}`)
      .digest("base64url");
    expect(signature).toBe(expected);
  });

  it("includes the board scope for WS tokens", () => {
    const token = mintToken("s", { email: "a@x.com", retro: "retro-1" });
    expect(decode(token.split(".")[1]).retro).toBe("retro-1");
  });

  it("signature changes with a different secret", () => {
    const a = mintToken("secret-a", { email: "a@x.com" });
    const b = mintToken("secret-b", { email: "a@x.com" });
    expect(a.split(".")[2]).not.toBe(b.split(".")[2]);
  });

  it("WS marker subprotocol is stable", () => {
    expect(WS_SUBPROTOCOL).toBe("spillio.ws.v1");
  });
});
