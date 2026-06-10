import path from "node:path";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  allowedDevOrigins: ["127.0.0.1", "192.168.68.61"],
  reactStrictMode: true,
  // Emit a self-contained server bundle so the runtime image can be distroless
  // (no npm/pnpm, no full node_modules) — that toolchain is what drags the
  // Sysdig-flagged transitive CVEs into the image.
  output: "standalone",
  // pnpm workspace: trace from the repo root so the standalone bundle picks up
  // hoisted workspace dependencies, not just apps/web/node_modules.
  outputFileTracingRoot: path.join(__dirname, "../../"),
};

export default nextConfig;
