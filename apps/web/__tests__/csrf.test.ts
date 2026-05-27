import { describe, it, expect } from 'vitest';
import { assertSameOrigin } from '../app/lib/csrf';

function makeRequest(hdrs: Record<string, string>): Request {
  return new Request('http://example.com/api/action', { method: 'POST', headers: hdrs });
}

describe('assertSameOrigin', () => {
  it('returns ok:true for sec-fetch-site: same-origin', () => {
    const result = assertSameOrigin(makeRequest({ 'sec-fetch-site': 'same-origin' }));
    expect(result).toEqual({ ok: true });
  });

  it('returns ok:true for sec-fetch-site: none', () => {
    const result = assertSameOrigin(makeRequest({ 'sec-fetch-site': 'none' }));
    expect(result).toEqual({ ok: true });
  });

  it('returns ok:false for sec-fetch-site: cross-site (no origin header)', () => {
    const result = assertSameOrigin(makeRequest({ 'sec-fetch-site': 'cross-site' }));
    expect(result.ok).toBe(false);
  });

  it('returns ok:true when origin host matches host header', () => {
    const result = assertSameOrigin(
      makeRequest({ origin: 'https://app.example.com', host: 'app.example.com' }),
    );
    expect(result).toEqual({ ok: true });
  });

  it('returns ok:false when origin host differs from host header', () => {
    const result = assertSameOrigin(
      makeRequest({ origin: 'https://evil.example.com', host: 'app.example.com' }),
    );
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.reason).toMatch(/evil\.example\.com/);
    }
  });

  it('returns ok:false with reason "missing origin" when no headers present', () => {
    const result = assertSameOrigin(makeRequest({}));
    expect(result).toEqual({ ok: false, reason: 'missing origin' });
  });

  it('returns ok:false with reason "invalid origin" for a malformed origin header', () => {
    const result = assertSameOrigin(
      makeRequest({ origin: 'not-a-url', host: 'app.example.com' }),
    );
    expect(result).toEqual({ ok: false, reason: 'invalid origin' });
  });
});
