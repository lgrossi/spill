// Trusts Sec-Fetch-Site: same-origin/none, falls back to Origin vs Host matching.

export type CsrfResult = { ok: true } | { ok: false; reason: string };

export function assertSameOrigin(req: Request): CsrfResult {
  const fetchSite = req.headers.get('sec-fetch-site');
  if (fetchSite === 'same-origin' || fetchSite === 'none') {
    return { ok: true };
  }

  const origin = req.headers.get('origin');
  if (!origin) {
    return { ok: false, reason: 'missing origin' };
  }

  const host = req.headers.get('host');
  try {
    const originHost = new URL(origin).host;
    if (originHost === host) return { ok: true };
    return { ok: false, reason: `origin ${originHost} !== host ${host}` };
  } catch {
    return { ok: false, reason: 'invalid origin' };
  }
}
