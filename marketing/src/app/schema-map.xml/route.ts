import { siteUrl } from "@/lib/agent-docs";

const body = `<?xml version="1.0" encoding="UTF-8"?>
<schemamap xmlns="https://nlweb.ai/schemamap/1.0">
  <feed url="${siteUrl}/llms.txt" type="text/plain" />
  <feed url="${siteUrl}/llms-full.txt" type="text/plain" />
  <feed url="${siteUrl}/index.md" type="text/markdown" />
  <feed url="${siteUrl}/pricing.md" type="text/markdown" />
  <feed url="${siteUrl}/.well-known/api-catalog" type="application/linkset+json" />
  <feed url="${siteUrl}/.well-known/openapi.json" type="application/json" />
</schemamap>
`;

export function GET() {
  return new Response(body, {
    headers: {
      "Content-Type": "application/xml; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
