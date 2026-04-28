const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";
const apiUrl = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

const body = {
  linkset: [
    {
      anchor: `${siteUrl}/.well-known/api-catalog`,
      item: [
        {
          href: `${apiUrl}/v1/links`,
          title: "Rift links API",
        },
        {
          href: `${apiUrl}/v1/webhooks`,
          title: "Rift webhooks API",
        },
        {
          href: `${apiUrl}/v1/conversions/sources`,
          title: "Rift conversions API",
        },
        {
          href: `${apiUrl}/mcp`,
          title: "Rift MCP endpoint",
        },
      ],
      "service-desc": [
        {
          href: `${siteUrl}/.well-known/openapi.json`,
          type: "application/vnd.oai.openapi+json;version=3.1",
          title: "Rift OpenAPI description",
        },
        {
          href: `${siteUrl}/openapi.json`,
          type: "application/vnd.oai.openapi+json;version=3.1",
          title: "Rift OpenAPI alias",
        },
      ],
      "service-doc": [
        {
          href: `${siteUrl}/api-reference`,
          type: "text/html",
          title: "Rift API reference",
        },
        {
          href: `${siteUrl}/docs`,
          type: "text/html",
          title: "Rift developer docs",
        },
      ],
      status: [
        {
          href: `${apiUrl}/health`,
          type: "application/json",
          title: "Rift API health",
        },
      ],
    },
  ],
};

export function GET() {
  return new Response(JSON.stringify(body, null, 2), {
    headers: {
      "Content-Type":
        'application/linkset+json;profile="https://www.rfc-editor.org/info/rfc9727"',
      "Cache-Control": "public, max-age=3600",
    },
  });
}
