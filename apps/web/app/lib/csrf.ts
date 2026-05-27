// Next.js 14+ automatically enforces CSRF protection for Server Actions by validating
// that the `Origin` header matches the server host on every POST request that invokes
// an action (see https://nextjs.org/blog/security-nextjs-server-components-actions).
// `assertSameOrigin` is therefore NOT wired into the server actions in `actions.ts` —
// the framework already covers them.
//
// It IS applied to the plain route handlers under `app/api/` that mutate state
// (the `cluster` and `move` PATCH endpoints), because those are standard HTTP
// endpoints that Next.js does not protect automatically.
//
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
