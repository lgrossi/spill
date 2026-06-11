import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@/lib/identity", () => ({
  currentIdentity: vi.fn(),
  canMintTokenForIdentity: vi.fn(),
}));

vi.mock("@/lib/token", () => ({
  mintToken: vi.fn(() => "signed-token"),
}));

import { GET } from "../app/cli/login/route";
import { canMintTokenForIdentity, currentIdentity } from "@/lib/identity";
import { mintToken } from "@/lib/token";

const identity = {
  subject: "email:abc",
  email: "ava@example.com",
  displayName: "Ava",
  source: "upstream" as const,
};

describe("CLI login route", () => {
  beforeEach(() => {
    process.env.SPILLIO_TOKEN_SECRET = "test-secret";
    vi.mocked(currentIdentity).mockResolvedValue(identity);
    vi.mocked(canMintTokenForIdentity).mockReturnValue(true);
    vi.mocked(mintToken).mockClear();
  });

  it("refuses to mint a CLI token for an untrusted local identity", async () => {
    vi.mocked(canMintTokenForIdentity).mockReturnValue(false);

    const response = await GET(
      new Request("https://spill.test/cli/login?cb=http://127.0.0.1:4321/callback"),
    );

    expect(response.status).toBe(401);
    expect(await response.json()).toEqual({ error: "trusted identity required" });
    expect(mintToken).not.toHaveBeenCalled();
  });

  it("redirects a trusted identity back to the loopback callback with a token", async () => {
    const response = await GET(
      new Request("https://spill.test/cli/login?cb=http://127.0.0.1:4321/callback&state=nonce"),
    );

    expect(response.status).toBe(302);
    const location = new URL(response.headers.get("location") ?? "");
    expect(location.origin).toBe("http://127.0.0.1:4321");
    expect(location.searchParams.get("token")).toBe("signed-token");
    expect(location.searchParams.get("state")).toBe("nonce");
    expect(mintToken).toHaveBeenCalledWith(
      "test-secret",
      { email: "ava@example.com", name: "Ava" },
      8 * 60 * 60,
    );
  });
});
