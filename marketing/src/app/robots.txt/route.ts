import { siteUrl } from "@/lib/agent-docs";

const body = `User-agent: *
Allow: /

User-agent: GPTBot
Allow: /

User-agent: ChatGPT-User
Allow: /

User-agent: CCBot
Disallow: /

User-agent: ByteSpider
Disallow: /

User-agent: ClaudeBot
Allow: /

Sitemap: ${siteUrl}/sitemap.xml
Host: ${siteUrl}
Content-Signal: search=yes, ai-input=yes, ai-train=no
Schemamap: ${siteUrl}/schema-map.xml
`;

export function GET() {
  return new Response(body, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
