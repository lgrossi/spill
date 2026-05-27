import { NextResponse } from 'next/server';
import { isShuttingDown } from '@/lib/shutdown';

export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';

export function GET() {
  if (isShuttingDown()) {
    return NextResponse.json(
      { ok: false, shutting_down: true },
      { status: 503, headers: { 'cache-control': 'no-store' } },
    );
  }
  return NextResponse.json({ ok: true });
}
