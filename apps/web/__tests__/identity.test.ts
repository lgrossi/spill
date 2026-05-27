import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('next/headers', () => ({
  headers: vi.fn(),
  cookies: vi.fn(),
}));

import { headers, cookies } from 'next/headers';
import { currentIdentity } from '../app/lib/identity';

// Returns a Headers-like stub with a simple key→value map.
function stubHeaders(entries: Record<string, string>) {
  return { get: (name: string) => entries[name] ?? null } as unknown as Awaited<ReturnType<typeof headers>>;
}

// Returns a ReadonlyRequestCookies-like stub.
function stubCookies(entries: Record<string, string>) {
  return {
    get: (name: string) => (entries[name] !== undefined ? { value: entries[name] } : undefined),
  } as unknown as Awaited<ReturnType<typeof cookies>>;
}

describe('currentIdentity (email normalization, subject, display name)', () => {
  beforeEach(() => {
    vi.mocked(headers).mockResolvedValue(stubHeaders({}));
    vi.mocked(cookies).mockResolvedValue(stubCookies({}));
    // Ensure we are in local auth mode (non-production default)
    delete process.env.SPILLIO_AUTH_MODE;
  });

  it('lowercases the email from x-spillio-user-email header', async () => {
    vi.mocked(headers).mockResolvedValue(
      stubHeaders({ 'x-spillio-user-email': 'Alice@Example.COM' }),
    );
    const identity = await currentIdentity();
    expect(identity?.email).toBe('alice@example.com');
  });

  it('strips the accounts.google.com: prefix from x-goog-authenticated-user-email', async () => {
    vi.mocked(headers).mockResolvedValue(
      stubHeaders({ 'x-goog-authenticated-user-email': 'accounts.google.com:user@example.com' }),
    );
    const identity = await currentIdentity();
    expect(identity?.email).toBe('user@example.com');
  });

  it('falls back to email local part as displayName when no name header is provided', async () => {
    vi.mocked(headers).mockResolvedValue(
      stubHeaders({ 'x-spillio-user-email': 'john@example.com' }),
    );
    const identity = await currentIdentity();
    expect(identity?.displayName).toBe('john');
  });

  it('uses the name header as displayName when present', async () => {
    vi.mocked(headers).mockResolvedValue(
      stubHeaders({
        'x-spillio-user-email': 'john@example.com',
        'x-spillio-user-name': 'John Doe',
      }),
    );
    const identity = await currentIdentity();
    expect(identity?.displayName).toBe('John Doe');
  });

  it('produces a stable email:-prefixed hex subject for the same email', async () => {
    vi.mocked(headers).mockResolvedValue(
      stubHeaders({ 'x-spillio-user-email': 'stable@example.com' }),
    );
    const a = await currentIdentity();
    const b = await currentIdentity();
    expect(a?.subject).toBe(b?.subject);
    expect(a?.subject).toMatch(/^email:[a-f0-9]{64}$/);
  });

  it('produces different subjects for different emails', async () => {
    vi.mocked(headers).mockResolvedValue(
      stubHeaders({ 'x-spillio-user-email': 'a@example.com' }),
    );
    const a = await currentIdentity();

    vi.mocked(headers).mockResolvedValue(
      stubHeaders({ 'x-spillio-user-email': 'b@example.com' }),
    );
    const b = await currentIdentity();

    expect(a?.subject).not.toBe(b?.subject);
  });

  it('reads identity from cookies when no email header is present', async () => {
    vi.mocked(cookies).mockResolvedValue(
      stubCookies({ spillio_identity_email: 'cookie@example.com' }),
    );
    const identity = await currentIdentity();
    expect(identity?.email).toBe('cookie@example.com');
    expect(identity?.source).toBe('local');
  });

  it('returns null when no email in headers or cookies', async () => {
    const identity = await currentIdentity();
    expect(identity).toBeNull();
  });

  it('returns null in proxy mode when no email header is present', async () => {
    process.env.SPILLIO_AUTH_MODE = 'proxy';
    vi.mocked(cookies).mockResolvedValue(
      stubCookies({ spillio_identity_email: 'cookie@example.com' }),
    );
    const identity = await currentIdentity();
    expect(identity).toBeNull();
  });
});
