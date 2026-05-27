import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

vi.mock('gcp-metadata', () => ({
  default: { isAvailable: vi.fn().mockResolvedValue(false), instance: vi.fn() },
}));

vi.mock('google-auth-library', () => ({
  GoogleAuth: vi.fn().mockImplementation(() => ({
    getClient: vi.fn().mockResolvedValue({ request: vi.fn() }),
  })),
}));

import { searchDirectory } from '../app/lib/directory';

const okResponse = (users: object[]) =>
  ({ ok: true, json: () => Promise.resolve(users) }) as unknown as Response;

const errResponse = (status = 500) =>
  ({ ok: false, status, statusText: 'Error' }) as unknown as Response;

describe('searchDirectory', () => {
  let fetchSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchSpy = vi.fn();
    vi.stubGlobal('fetch', fetchSpy);
    delete process.env.SPILLIO_DIRECTORY_URL;
    delete process.env.SPILLIO_DIRECTORY_IAP_AUDIENCE;
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns [] when SPILLIO_DIRECTORY_URL is not set', async () => {
    const result = await searchDirectory('alice');
    expect(result).toEqual([]);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('returns [] when query is shorter than 2 characters', async () => {
    process.env.SPILLIO_DIRECTORY_URL = 'https://dir.example.com';
    const result = await searchDirectory('a');
    expect(result).toEqual([]);
    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('calls the right URL with the encoded query param', async () => {
    process.env.SPILLIO_DIRECTORY_URL = 'https://dir.example.com';
    fetchSpy.mockResolvedValue(
      okResponse([{ email: 'alice@example.com', name: 'Alice' }]),
    );

    const result = await searchDirectory('ali');

    expect(fetchSpy).toHaveBeenCalledWith(
      'https://dir.example.com/api/v1/users?emails=ali*',
      expect.objectContaining({ cache: 'no-store' }),
    );
    expect(result).toEqual([{ email: 'alice@example.com', name: 'Alice' }]);
  });

  it('strips extra fields from the API response (groups etc.)', async () => {
    process.env.SPILLIO_DIRECTORY_URL = 'https://dir.example.com';
    fetchSpy.mockResolvedValue(
      okResponse([{ email: 'bob@example.com', name: 'Bob', groups: ['eng'] }]),
    );

    const result = await searchDirectory('bo');
    expect(result).toEqual([{ email: 'bob@example.com', name: 'Bob' }]);
  });

  it('returns [] gracefully when fetch throws', async () => {
    process.env.SPILLIO_DIRECTORY_URL = 'https://dir.example.com';
    fetchSpy.mockRejectedValue(new Error('network failure'));

    const result = await searchDirectory('al');
    expect(result).toEqual([]);
  });

  it('returns [] gracefully when the API responds with an error status', async () => {
    process.env.SPILLIO_DIRECTORY_URL = 'https://dir.example.com';
    fetchSpy.mockResolvedValue(errResponse(503));

    const result = await searchDirectory('al');
    expect(result).toEqual([]);
  });
});
