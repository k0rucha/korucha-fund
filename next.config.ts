import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Self-contained Node server output for single-instance self-hosting.
  output: "standalone",
  // better-sqlite3 is a native module; keep it out of the bundler so the
  // prebuilt binary is required at runtime instead of being traced/inlined.
  serverExternalPackages: ["better-sqlite3"],
  // Allow HMR dev resources when viewing via these hosts (dev-only).
  allowedDevOrigins: ["127.0.0.1", "192.168.1.51"],
  // The OGP routes read Noto font files via fs at runtime; make sure the
  // standalone trace ships them.
  outputFileTracingIncludes: {
    "/share/[id]/opengraph-image": ["./assets/fonts/**/*"],
    "/ticker/[id]/opengraph-image": ["./assets/fonts/**/*"],
  },
};

export default nextConfig;
