import { apiUrl } from "@/lib/agent-docs";

const body = {
  name: "Riftl.ink MCP Server",
  description:
    "Streamable HTTP MCP server for creating, listing, updating, and deleting Rift deep links.",
  version: "0.1.0",
  serverUrl: `${apiUrl}/mcp`,
  tools: [
    {
      name: "create_link",
      description: "Create a new Rift deep link with platform-specific destinations.",
    },
    {
      name: "get_link",
      description: "Get details of a Rift deep link by ID.",
    },
    {
      name: "list_links",
      description: "List Rift deep links with cursor-based pagination.",
    },
    {
      name: "update_link",
      description: "Update an existing Rift deep link.",
    },
    {
      name: "delete_link",
      description: "Delete a Rift deep link permanently.",
    },
    {
      name: "create_source",
      description: "Create a conversion tracking source and get a webhook URL.",
    },
    {
      name: "list_sources",
      description: "List conversion sources for the current tenant.",
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
