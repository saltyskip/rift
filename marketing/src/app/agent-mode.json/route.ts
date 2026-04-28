import { apiUrl, siteUrl } from "@/lib/agent-docs";

const body = {
  product: "Riftl.ink",
  description:
    "Deep links for humans and agents. Resolve links as redirects for browsers or structured JSON for agents.",
  endpoints: {
    docs: `${siteUrl}/docs`,
    apiReference: `${siteUrl}/api-reference`,
    openapi: `${siteUrl}/openapi.json`,
    apiCatalog: `${siteUrl}/.well-known/api-catalog`,
    mcp: `${apiUrl}/mcp`,
    mcpManifest: `${siteUrl}/.well-known/mcp.json`,
    pricing: `${siteUrl}/pricing.md`,
    llms: `${siteUrl}/llms.txt`,
  },
  authentication: {
    type: "bearer",
    secretKeyPrefix: "rl_live_",
    publishableKeyPrefix: "pk_live_",
  },
  capabilities: [
    "create deep links",
    "resolve links as JSON for agents",
    "track clicks and installs",
    "track conversions",
    "manage links over MCP",
  ],
};

export function GET() {
  return new Response(JSON.stringify(body, null, 2), {
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
