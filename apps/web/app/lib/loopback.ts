// Only ever redirect a minted token back to a loopback address the local CLI
// owns. Anything else would make the CLI-login route an open redirect that
// leaks identity tokens to arbitrary hosts.
export function loopbackTarget(cb: string): URL | null {
  let url: URL;
  try {
    url = new URL(cb);
  } catch {
    return null;
  }
  if (url.protocol !== "http:") return null;
  if (url.username || url.password) return null;
  // url.hostname yields the bracketed form "[::1]" for IPv6 loopback.
  if (!["127.0.0.1", "localhost", "[::1]"].includes(url.hostname)) return null;
  return url;
}
