import { NextResponse } from "next/server";
import { canMintTokenForIdentity, currentIdentity } from "@/lib/identity";
import { mintToken } from "@/lib/token";

// Companion access token. A signed-in user (authenticated by the platform in
// front of this app) fetches a short-lived token here and pastes it into the
// companion CLI (SPILLIO_API_TOKEN). The token carries their identity; the API
// trusts it without any service account or on-behalf-of header.
const COMPANION_TOKEN_TTL_SECONDS = 8 * 60 * 60;

export async function GET() {
  const identity = await currentIdentity();
  if (!identity?.email) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }

  const secret = process.env.SPILLIO_TOKEN_SECRET?.trim();
  if (!secret) {
    return NextResponse.json({ error: "token unavailable" }, { status: 503 });
  }
  if (!canMintTokenForIdentity(identity)) {
    return NextResponse.json({ error: "trusted identity required" }, { status: 401 });
  }

  const token = mintToken(
    secret,
    { email: identity.email, name: identity.displayName },
    COMPANION_TOKEN_TTL_SECONDS,
  );
  const expiresAt = new Date(Date.now() + COMPANION_TOKEN_TTL_SECONDS * 1000).toISOString();
  return NextResponse.json({ token, email: identity.email, expires_at: expiresAt });
}
