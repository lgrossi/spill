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
      headers['proxy-authorization'] = `Bearer ${token}`;
    }

    const get = (path: string) =>
      fetch(`${baseUrl}${path}`, { headers, cache: 'no-store' });

    // Run user prefix search and group prefix search in parallel.
    // /groups with includeUsers+recursive expands team emails to members.
    const [usersRes, groupsRes] = await Promise.all([
      get(`/users?emails=${encodeURIComponent(query)}*`),
      get(`/groups?emails=${encodeURIComponent(query)}*&includeUsers=true&recursive=true`),
    ]);

    const users: DirectoryUser[] = usersRes.ok
      ? ((await usersRes.json()) as DirectoryApiUser[]).map(({ email, name }) => ({ email, name }))
      : [];

    if (!usersRes.ok) console.warn(`[directory] users search failed: ${usersRes.status}`);
    if (!groupsRes.ok) console.warn(`[directory] groups search failed: ${groupsRes.status}`);

    // Build email→name index from the users result for fast name resolution.
    const nameByEmail = new Map(users.map((u) => [u.email, u.name]));

    // Expand each matched group into its member list, resolving names where known.
    const memberEmails = new Set<string>();
    if (groupsRes.ok) {
      const groups: { email: string; name: string; users: string[] }[] = await groupsRes.json();
      for (const group of groups) {
        for (const memberEmail of group.users) {
          memberEmails.add(memberEmail);
        }
      }
    }

    // Merge: start with direct user hits, then add any group members not already present.
    const seen = new Set(users.map((u) => u.email));
    const merged: DirectoryUser[] = [...users];
    for (const email of memberEmails) {
      if (!seen.has(email)) {
        merged.push({ email, name: nameByEmail.get(email) ?? email });
        seen.add(email);
      }
    }

    return merged;
  } catch (err) {
    console.warn('[directory] search error:', err);
    return [];
  }
}
