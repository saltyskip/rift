const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";

const body = `# Rift

> Deep linking and attribution API. Built for humans, ready for agents. One link, two audiences: humans click and get redirected, agents resolve the same URL into structured JSON.

Rift is API-first link infrastructure for iOS, Android, and web. It handles short links, deep links, deferred deep linking, install attribution, and conversion tracking. Pricing is usage-based ($0.01/request) with a free tier (100 links, 1,000 clicks/month). No sales calls, no contracts — sign up, get an \`rl_live_\` API key, and POST to \`/v1/links\`.

## Docs

- [Quick Start](${siteUrl}/docs): CLI and manual setup paths to a working link in ~30 seconds
- [Links](${siteUrl}/docs/links): create, update, and resolve deep links
- [Attribution](${siteUrl}/docs/attribution): click, install, and user attribution
- [Conversions](${siteUrl}/docs/conversions): post-install event tracking
- [Deferred Deep Linking](${siteUrl}/docs/deferred): route users into the app after install
- [Universal Links](${siteUrl}/docs/universal-links): iOS and Android app-link setup
- [Webhooks](${siteUrl}/docs/webhooks): outbound event delivery
- [Domains](${siteUrl}/docs/domains): primary and alternate custom domains
- [Apps](${siteUrl}/docs/apps): registering iOS and Android apps
- [Publishable Keys](${siteUrl}/docs/publishable-keys): client-side SDK keys
- [iOS SDK](${siteUrl}/docs/ios-sdk)
- [Android SDK](${siteUrl}/docs/android-sdk)
- [Web SDK](${siteUrl}/docs/web-sdk)
- [Manual Setup](${siteUrl}/docs/manual-setup)

## API

- [API Reference](${siteUrl}/api-reference): full OpenAPI-backed reference

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
