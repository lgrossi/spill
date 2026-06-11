import { createHmac } from "crypto";

// First-party token minter. The web tier (behind IAP) vouches for the
// authenticated user by signing a short-lived HS256 token the Spill API
// verifies with the shared `SPILLIO_TOKEN_SECRET`. One mechanism for every
// caller: REST/CLI tokens carry { email, name }; board-scoped WS tokens add
// { retro }. Identity is bound into the signed token, not a spoofable header.
export type TokenClaims = {
  email: string;
  name?: string;
  retro?: string;
};

function base64url(input: string): string {
  return Buffer.from(input).toString("base64url");
}

export function mintToken(secret: string, claims: TokenClaims, ttlSeconds = 120): string {
  const header = base64url(JSON.stringify({ alg: "HS256", typ: "JWT" }));
  const payload = base64url(
    JSON.stringify({
      ...claims,
      exp: Math.floor(Date.now() / 1000) + ttlSeconds,
    }),
  );
  const data = `${header}.${payload}`;
  const signature = createHmac("sha256", secret).update(data).digest("base64url");
  return `${data}.${signature}`;
}
