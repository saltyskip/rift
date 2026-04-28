import { apiUrl, siteUrl } from "@/lib/agent-docs";

const body = {
  name: "Riftl.ink Agent Card",
  description:
    "Agent-facing description of Riftl.ink capabilities for deep links, attribution, and MCP access.",
  version: "0.1.0",
  url: siteUrl,
  contact: "hello@riftl.ink",
  capabilities: [
    "Create and manage deep links",
    "Resolve links into structured JSON",
    "Track attribution and conversions",
    "Use MCP tools over Streamable HTTP",
  ],
  skills: [
    {
      name: "create_link",
      description: "Create a deep link with per-platform destinations.",
    },
    {
      name: "resolve_link",
      description:
        "Resolve a link for agent consumption with Accept: application/json.",
    },
    {
      name: "track_conversion",
      description: "Track post-install conversion events.",
    },
  ],
  endpoints: {
    docs: `${siteUrl}/docs`,
    openapi: `${siteUrl}/openapi.json`,
    mcp: `${apiUrl}/mcp`,
  },
};

export function GET() {
  return new Response(JSON.stringify(body, null, 2), {
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
