export type DirectoryUser = { email: string; name: string };

type DirectoryApiUser = { email: string; name: string; groups?: string[] };

// Mint a Google IAP identity token via the GCE metadata server.
async function iapToken(audience: string): Promise<string> {
  const url =
    `http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/identity` +
    `?audience=${encodeURIComponent(audience)}&format=full`;
  const res = await fetch(url, { headers: { "Metadata-Flavor": "Google" } });
  if (!res.ok) throw new Error(`IAP token fetch failed: ${res.status}`);
  return res.text();
}

// Search the configured directory for users matching the given prefix query.
// Returns [] when SPILLIO_DIRECTORY_URL is not set or the query is too short.
// Any network or auth error degrades gracefully to [].
//
// Wire this into the board invite/grant UI inside
//   apps/web/app/retros/[retroId]/ (e.g. a new invite-members panel in phase-controls.tsx
//   or a dedicated invite-panel.tsx), calling searchDirectoryAction from a client component.
export async function searchDirectory(query: string): Promise<DirectoryUser[]> {
  const baseUrl = process.env.SPILLIO_DIRECTORY_URL;
  if (!baseUrl || query.length < 2) return [];

  try {
    const headers: Record<string, string> = {};

    const sa = process.env.SPILLIO_DIRECTORY_IAP_SA;
    const audience = process.env.SPILLIO_DIRECTORY_IAP_AUDIENCE;
    if (sa && audience) {
      const token = await iapToken(audience);
      headers["Authorization"] = `Bearer ${token}`;
    }

    const url = `${baseUrl}/api/v1/users?emails=${encodeURIComponent(query)}*`;
    const res = await fetch(url, { headers, cache: "no-store" });
    if (!res.ok) {
      console.warn(`[directory] search failed: ${res.status} ${res.statusText}`);
      return [];
    }

    const users: DirectoryApiUser[] = await res.json();
    return users.map(({ email, name }) => ({ email, name }));
  } catch (err) {
    console.warn("[directory] search error:", err);
    return [];
  }
}
