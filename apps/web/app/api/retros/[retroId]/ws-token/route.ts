import { NextResponse } from "next/server";
import { canMintTokenForIdentity, currentIdentity } from "@/lib/identity";
import { mintToken } from "@/lib/token";

// Runs behind the web app's IAP, so the caller is an authenticated user.
export async function GET(
  _request: Request,
  { params }: { params: Promise<{ retroId: string }> },
) {
  const { retroId } = await params;
  const identity = await currentIdentity();
  if (!identity?.email) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }

  const secret = process.env.SPILLIO_TOKEN_SECRET?.trim();
  if (!secret) {
    return NextResponse.json({ error: "ws token unavailable" }, { status: 503 });
  }
  if (!canMintTokenForIdentity(identity)) {
    return NextResponse.json({ error: "trusted identity required" }, { status: 401 });
  }

  const token = mintToken(secret, { email: identity.email, retro: retroId });
  return NextResponse.json({ token });
}
