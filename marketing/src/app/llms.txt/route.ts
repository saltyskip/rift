const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";
const apiUrl = process.env.NEXT_PUBLIC_API_URL || "https://api.riftl.ink";

const body = `# Riftl.ink (Rift)

> Deep linking and attribution API. Built for humans, ready for agents. One link, two audiences: humans click and get redirected, agents resolve the same URL into structured JSON.

Riftl.ink is API-first link infrastructure for iOS, Android, and web. Rift handles short links, deep links, deferred deep linking, install attribution, and conversion tracking. Developer resources for Rift are published on \`riftl.ink\` and \`api.riftl.ink\`.

Pricing is usage-based ($0.01/request) with a free tier (100 links, 1,000 clicks/month). No sales calls, no contracts — sign up, get an \`rl_live_\` API key, and POST to \`/v1/links\`.

## Developer Resources

- [OpenAPI JSON](${siteUrl}/openapi.json): machine-readable API schema for Rift
- [Well-Known OpenAPI](${siteUrl}/.well-known/openapi.json): predictable OpenAPI URL for agents
- [API Catalog](${siteUrl}/.well-known/api-catalog): RFC 9727 linkset catalog for Rift APIs
- [API Reference](${siteUrl}/api-reference): human-readable OpenAPI-backed reference
- [MCP Endpoint](${apiUrl}/mcp): Rift Model Context Protocol server for agents
- [Health Check](${apiUrl}/health): API status endpoint

## Docs

- [Quick Start](${siteUrl}/docs): CLI and manual setup paths to a working link in ~30 seconds
- [Manual Setup](${siteUrl}/docs/manual-setup): raw HTTP setup path for developers and agents
- [Links](${siteUrl}/docs/links): create, update, and resolve deep links
- [Attribution](${siteUrl}/docs/attribution): click, install, and user attribution
- [Conversions](${siteUrl}/docs/conversions): post-install event tracking
- [Webhooks](${siteUrl}/docs/webhooks): outbound event delivery and signature verification
- [Deferred Deep Linking](${siteUrl}/docs/deferred): route users into the app after install
- [Universal Links](${siteUrl}/docs/universal-links): iOS and Android app-link setup
- [Domains](${siteUrl}/docs/domains): primary and alternate custom domains
- [Apps](${siteUrl}/docs/apps): registering iOS and Android apps
- [Publishable Keys](${siteUrl}/docs/publishable-keys): client-side SDK keys
- [iOS SDK](${siteUrl}/docs/ios-sdk)
- [Android SDK](${siteUrl}/docs/android-sdk)
- [Web SDK](${siteUrl}/docs/web-sdk)

## Authentication

- [Quick Start](${siteUrl}/docs): create an account and get a Rift API key
- Authentication uses Bearer tokens with secret keys that start with \`rl_live_\`
- Client-side SDKs use publishable keys that start with \`pk_live_\`
- [Publishable Keys](${siteUrl}/docs/publishable-keys): client-side SDK key management

## Integrations

- [Webhooks](${siteUrl}/docs/webhooks): click, attribution, and conversion events
- [MCP Endpoint](${apiUrl}/mcp): create and manage Rift links from MCP clients
- [Apps](${siteUrl}/docs/apps): iOS and Android app association setup

## Optional

- [Sitemap](${siteUrl}/sitemap.xml)
`;

export function GET() {
  return new Response(body, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
