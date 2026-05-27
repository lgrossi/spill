import gcpMetadata from 'gcp-metadata';
import { GoogleAuth } from 'google-auth-library';

export type DirectoryUser = { email: string; name: string };

type DirectoryApiUser = { email: string; name: string; groups?: string[] };

const TOKEN_TTL_MS = 55 * 60 * 1000;
let cachedToken: string | null = null;
let tokenExpiresAt = 0;

async function fetchIapToken(audience: string): Promise<string> {
  const now = Date.now();
  if (cachedToken && now < tokenExpiresAt) return cachedToken;

  let token: string;
  if (await gcpMetadata.isAvailable()) {
    token = await gcpMetadata.instance(
      `service-accounts/default/identity?audience=${encodeURIComponent(audience)}&format=full`,
    );
  } else {
    const sa = process.env.SPILLIO_DIRECTORY_IAP_SA;
    if (!sa) throw new Error('SPILLIO_DIRECTORY_IAP_SA required for local IAP auth');
    const auth = new GoogleAuth();
    const client = await auth.getClient();
    const iamUrl = `https://iamcredentials.googleapis.com/v1/projects/-/serviceAccounts/${sa}:generateIdToken`;
    const response = await client.request<{ token: string }>({
      url: iamUrl,
      method: 'POST',
      data: { audience, includeEmail: true },
    });
    token = response.data.token;
  }

  cachedToken = token;
  tokenExpiresAt = now + TOKEN_TTL_MS;
  return token;
}

export async function searchDirectory(query: string): Promise<DirectoryUser[]> {
  const baseUrl = process.env.SPILLIO_DIRECTORY_URL;
  if (!baseUrl || query.length < 2) return [];

  try {
    const headers: Record<string, string> = {};

    const audience = process.env.SPILLIO_DIRECTORY_IAP_AUDIENCE;
    if (audience) {
      const token = await fetchIapToken(audience);
      // IAP expects the identity token on proxy-authorization, not authorization
      headers['proxy-authorization'] = `Bearer ${token}`;
    }

    const url = `${baseUrl}/users?emails=${encodeURIComponent(query)}*`;
    const res = await fetch(url, { headers, cache: 'no-store' });
    if (!res.ok) {
      console.warn(`[directory] search failed: ${res.status} ${res.statusText}`);
      return [];
    }

    const users: DirectoryApiUser[] = await res.json();
    return users.map(({ email, name }) => ({ email, name }));
  } catch (err) {
    console.warn('[directory] search error:', err);
    return [];
  }
}
