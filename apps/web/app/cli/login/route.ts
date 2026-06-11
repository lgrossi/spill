import { NextResponse } from "next/server";
import { currentIdentity } from "@/lib/identity";
import { mintToken } from "@/lib/token";
import { loopbackTarget } from "@/lib/loopback";

const COMPANION_TOKEN_TTL_SECONDS = 8 * 60 * 60;

export async function GET(request: Request) {
  const identity = await currentIdentity();
  if (!identity?.email) {
    return NextResponse.json({ error: "unauthorized" }, { status: 401 });
  }

  const secret = process.env.SPILLIO_TOKEN_SECRET;
  if (!secret) {
    return NextResponse.json({ error: "token unavailable" }, { status: 503 });
  }

  const params = new URL(request.url).searchParams;
  const cb = params.get("cb");
  if (!cb) {
    return NextResponse.json({ error: "missing cb" }, { status: 400 });
  }
  const target = loopbackTarget(cb);
  if (!target) {
    return NextResponse.json({ error: "cb must be a loopback http url" }, { status: 400 });
  }

  const token = mintToken(
    secret,
    { email: identity.email, name: identity.displayName },
    COMPANION_TOKEN_TTL_SECONDS,
  );
  const expiresAt = new Date(Date.now() + COMPANION_TOKEN_TTL_SECONDS * 1000).toISOString();

  target.searchParams.set("token", token);
  target.searchParams.set("expires_at", expiresAt);
  target.searchParams.set("state", params.get("state") ?? "");
  return NextResponse.redirect(target.toString(), 302);
}
