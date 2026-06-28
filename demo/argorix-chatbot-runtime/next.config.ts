import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // The /api/chat route shells out to the local Argorix binaries, so it must
  // run on the Node.js runtime (not the Edge runtime).
  reactStrictMode: true,
};

export default nextConfig;
