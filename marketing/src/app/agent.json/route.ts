import { apiUrl, siteUrl } from "@/lib/agent-docs";

const body = {
  name: "Riftl.ink Agent Discovery",
  version: "0.1.0",
  description:
    "Discovery metadata for Riftl.ink deep link, attribution, and MCP capabilities.",
  homepage: siteUrl,
  capabilities: [
    "deep_link_creation",
    "link_resolution",
    "attribution_tracking",
    "conversion_tracking",
    "mcp_tools",
  ],
  endpoints: {
    openapi: `${siteUrl}/.well-known/openapi.json`,
    apiCatalog: `${siteUrl}/.well-known/api-catalog`,
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
