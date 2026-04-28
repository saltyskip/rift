import { apiUrl, siteUrl } from "@/lib/agent-docs";

const body = {
  serverUrl: `${apiUrl}/mcp`,
  manifestUrl: `${siteUrl}/.well-known/mcp.json`,
  documentationUrl: `${siteUrl}/docs/mcp`,
  transport: "streamable-http",
};

export function GET() {
  return new Response(JSON.stringify(body, null, 2), {
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
