const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";
const apiUrl = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

const body = {
  $schema:
    "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json",
  name: "ink.riftl/rift",
  title: "Riftl.ink MCP Server",
  description:
    "Create, inspect, update, list, and delete Rift deep links over MCP using the Streamable HTTP transport.",
  version: "0.1.0",
  homepage: siteUrl,
  documentationUrl: `${siteUrl}/docs/mcp`,
  remotes: [
    {
      type: "streamable-http",
      url: `${apiUrl}/mcp`,
      headers: [
        {
          name: "x-api-key",
          description: "Rift secret API key with the rl_live_ prefix",
          isRequired: true,
          isSecret: true,
        },
      ],
    },
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
