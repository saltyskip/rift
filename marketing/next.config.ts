import type { NextConfig } from "next";
import createMDX from "@next/mdx";

const withMDX = createMDX({
  extension: /\.mdx?$/,
  options: {
    remarkPlugins: [["remark-frontmatter", ["yaml"]], ["remark-gfm"]],
    rehypePlugins: [],
  },
});

const nextConfig: NextConfig = {
  pageExtensions: ["ts", "tsx", "md", "mdx"],
  async redirects() {
    // /signup was the old dedicated signup page; the magic-link flow at
    // /signin now handles both first-time and returning users. Keep the
    // URL alive as a 301 so existing inbound links + SEO equity survive.
    return [
      {
        source: "/signup",
        destination: "/signin",
        permanent: true,
      },
    ];
  },
  async rewrites() {
    const apiOrigin =
      process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

    return [
      {
        source: "/openapi.json",
        destination: `${apiOrigin}/openapi.json`,
      },
      {
        source: "/.well-known/openapi.json",
        destination: `${apiOrigin}/openapi.json`,
      },
      {
        source: "/.well-known/api-catalog",
        destination: "/api-catalog",
      },
      {
        source: "/.well-known/mcp.json",
        destination: "/mcp-server.json",
      },
      {
        source: "/.well-known/mcp",
        destination: "/mcp-discovery",
      },
      {
        source: "/mcp/server.json",
        destination: "/mcp-server.json",
      },
      {
        source: "/.well-known/mcp/server-card.json",
        destination: "/mcp/server-card.json",
      },
      {
        source: "/.well-known/ai-plugin.json",
        destination: "/ai-plugin.json",
      },
      {
        source: "/.well-known/agent.json",
        destination: "/agent.json",
      },
      {
        source: "/.well-known/agent-card.json",
        destination: "/agent-card.json",
      },
    ];
  },
};

export default withMDX(nextConfig);
