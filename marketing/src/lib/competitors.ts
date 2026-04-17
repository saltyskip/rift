export type FeatureValue =
  | { kind: "yes"; note?: string }
  | { kind: "no"; note?: string }
  | { kind: "partial"; note: string }
  | { kind: "text"; value: string };

export interface FeatureRow {
  label: string;
  rift: FeatureValue;
  competitor: FeatureValue;
}

export interface PricingScenario {
  scale: string;
  rift: string;
  competitor: string;
}

export interface FaqEntry {
  q: string;
  a: string;
}

export interface Competitor {
  slug: string;
  name: string;
  domain: string;
  category: string;
  headline: string;
  metaDescription: string;
  tagline: string;
  oneLiner: string;
  targetKeyword: string;
  secondaryKeywords: string[];
  features: FeatureRow[];
  whyLeave: string[];
  whereBetter: string[];
  pricing: PricingScenario[];
  migrationSteps: string[];
  faqs: FaqEntry[];
  relatedBlogPosts?: Array<{ title: string; href: string }>;
}

const YES: FeatureValue = { kind: "yes" };
const NO: FeatureValue = { kind: "no" };

export const branch: Competitor = {
  slug: "branch",
  name: "Branch",
  domain: "branch.io",
  category: "Mobile Measurement Platform",
  headline: "The Branch.io Alternative Built for Developers",
  metaDescription:
    "Rift is the Branch.io alternative with MCP-native access for AI agents, a transparent free tier, pay-per-request pricing, and a self-hostable Rust core — without the MMP sales cycle.",
  tagline: "Branch without the enterprise sales cycle.",
  oneLiner:
    "Rift covers Branch's deep-linking, AASA hosting, custom domains, attribution, and conversion webhooks — plus MCP for AI agents, x402 per-request billing, and a self-hostable core. Skip the contract. Use the API.",
  targetKeyword: "branch.io alternative",
  secondaryKeywords: [
    "branch alternative",
    "branch metrics alternative",
    "branch io deep linking alternative",
    "branch replacement developer",
  ],
  features: [
    { label: "Universal Links / App Links (auto AASA + assetlinks)", rift: YES, competitor: YES },
    { label: "Deferred deep linking", rift: YES, competitor: YES },
    { label: "Click + install + conversion attribution", rift: YES, competitor: YES },
    { label: "Custom short domains on free tier", rift: YES, competitor: NO },
    { label: "Conversion webhooks (signed HMAC)", rift: YES, competitor: YES },
    { label: "Mobile SDKs", rift: { kind: "text", value: "iOS, Android, Web" }, competitor: { kind: "text", value: "iOS, Android, Web, RN, Flutter, Unity" } },
    { label: "MCP server for AI agents", rift: YES, competitor: NO },
    { label: "Pay-per-request pricing (x402)", rift: YES, competitor: NO },
    { label: "Self-hostable", rift: { kind: "text", value: "Yes — Rust + MongoDB" }, competitor: NO },
    { label: "Free tier", rift: { kind: "text", value: "100 links, 1k clicks/mo" }, competitor: { kind: "text", value: "<10k MAUs only" } },
    { label: "SKAN postbacks + ad-network fraud", rift: { kind: "partial", note: "Basic" }, competitor: YES },
    { label: "Transparent pricing page", rift: YES, competitor: { kind: "no", note: "Contact sales" } },
  ],
  whyLeave: [
    "Branch gates custom domains, API access, and conversion webhooks behind an enterprise contract. Rift ships them on the free tier.",
    "Pricing on Branch's paid plans is quote-only. You talk to sales, get a number, and it scales with MAUs even when your link volume doesn't.",
    "Branch is built for marketing teams. The dashboard is the product. If your team is engineering-led and the API is the product, Rift's ergonomics feel native and Branch's feel like a wrapper.",
    "Branch has no MCP server. Agent-first integrations have to wrap the REST API by hand.",
  ],
  whereBetter: [
    "SKAdNetwork postbacks at ad-network scale, Branch has years of infrastructure Rift does not replicate.",
    "Probabilistic attribution and ad-network fraud detection tuned by a dedicated team.",
    "30+ out-of-box integrations into AppsFlyer-adjacent platforms (Singular, Kochava, etc.).",
    "Approval workflows, templates, and dashboards for non-engineering stakeholders.",
  ],
  pricing: [
    { scale: "Small app (50k MAUs, 200k clicks/mo)", rift: "$49/mo flat, or $0.01/req pay-as-you-go", competitor: "~$300–500/mo (Grow plan, quote-based)" },
    { scale: "Mid-size (500k MAUs, 2M clicks/mo)", rift: "Volume contract or ~$20k at per-request", competitor: "~$2k–5k/mo (Scale plan)" },
    { scale: "Agent workload (100k calls/day via MCP)", rift: "x402 per-request at 0.01 USDC/call", competitor: "Not supported natively" },
  ],
  migrationSteps: [
    "Export your existing Branch links via their API and re-create them on Rift with a one-time backfill script. Rift accepts a `custom_slug` on creation so the short URL stays stable.",
    "Swap the Branch iOS/Android SDK for the Rift SDK. The integration surface is similar: one initializer, one handler for Universal Links, one for cold start.",
    "Point your custom domain's CNAME at Rift. AASA and assetlinks.json auto-provision within minutes; TLS via Let's Encrypt.",
    "Update webhook consumers. Branch's payload shape differs from Rift's — write a small adapter, or change downstream consumers to read Rift's shape (it's simpler).",
    "Run a two-week parallel period with both services live, comparing click + install counts. Cut over once the numbers match within 2%.",
    "Keep Branch running for any legacy SKAN postback paths if you rely on ad-network fraud detection — Rift composes with MMPs, it does not replace them.",
  ],
  faqs: [
    {
      q: "Is Rift a full Branch.io replacement?",
      a: "Rift replaces Branch's deep linking, AASA hosting, custom domains, click + install attribution, and conversion webhooks. It does not replace Branch's SKAdNetwork postback infrastructure, fraud detection, or ad-network integrations. If you use Branch primarily for deep linking and attribution, Rift is a direct swap. If Branch is your MMP, keep it — or move to AppsFlyer or Adjust — and use Rift for the link layer only.",
    },
    {
      q: "Is Branch still a good choice?",
      a: "Yes, if you are running paid user-acquisition campaigns at scale and need ad-network integrations, SKAN postbacks, and fraud detection. Branch is a mature mobile measurement platform and that is its strength. If you just need deep links that open your app and attribute installs, you are paying for features you do not use.",
    },
    {
      q: "How much cheaper is Rift than Branch?",
      a: "For most small-to-midsize apps, Rift is between 3x and 10x cheaper because Branch's pricing scales with monthly active users while Rift scales with actual link traffic. A 50k-MAU app with 200k link clicks per month pays roughly $300–500/month on Branch and $49/month on Rift, or less with pay-per-request.",
    },
    {
      q: "Can I import my Branch link history?",
      a: "You can import the links themselves via Rift's bulk-create API, preserving the short URLs via the `custom_slug` field. Historical click and attribution data stays in Branch — export it to your warehouse before cutting over if you need it for retrospective analysis.",
    },
    {
      q: "Does Rift support AI agents like Branch does?",
      a: "Rift is the only one of the two with a native MCP server. Any MCP-capable agent — Claude, ChatGPT, Cursor, Gemini — can call Rift's create_link, get_link, list_links, update_link, and delete_link tools directly. Branch does not ship an MCP server; integrating it with an AI agent requires writing a custom wrapper around their REST API.",
    },
  ],
  relatedBlogPosts: [
    { title: "Migrating from Firebase Dynamic Links to Rift", href: "/blog/firebase-dynamic-links-migration" },
    { title: "Deep Linking for AI Agents: The MCP Pattern", href: "/blog/deep-linking-for-ai-agents" },
  ],
};

export const bitly: Competitor = {
  slug: "bitly",
  name: "Bitly",
  domain: "bitly.com",
  category: "URL shortener",
  headline: "The Bitly Alternative Built for Developers",
  metaDescription:
    "Rift is the Bitly alternative for developers — deep linking, native Universal Links, attribution, custom domains, and an MCP server for AI agents. Free tier, pay-per-request, self-hostable.",
  tagline: "Short links that also open your app.",
  oneLiner:
    "Rift gives you everything Bitly does — custom domains, analytics, branded short URLs — and then actually opens your iOS and Android apps on tap, attributes installs, and exposes the whole thing as an MCP server for AI agents.",
  targetKeyword: "bitly alternative",
  secondaryKeywords: [
    "bitly alternative developer",
    "bitly alternative open source",
    "bitly alternative free",
    "bitly replacement",
  ],
  features: [
    { label: "Short URLs on custom domain", rift: YES, competitor: YES },
    { label: "Click analytics dashboard", rift: YES, competitor: YES },
    { label: "Branded short domains on free tier", rift: YES, competitor: NO },
    { label: "QR code generation", rift: { kind: "partial", note: "API only" }, competitor: YES },
    { label: "iOS Universal Links / App Links (auto-hosted AASA)", rift: YES, competitor: NO },
    { label: "Deferred deep linking", rift: YES, competitor: NO },
    { label: "Click + install + conversion attribution", rift: YES, competitor: { kind: "no", note: "Clicks only" } },
    { label: "Conversion webhooks", rift: YES, competitor: { kind: "partial", note: "Enterprise tier" } },
    { label: "MCP server for AI agents", rift: YES, competitor: NO },
    { label: "Pay-per-request pricing", rift: YES, competitor: NO },
    { label: "Self-hostable", rift: YES, competitor: NO },
    { label: "Free tier", rift: { kind: "text", value: "100 links, 1k clicks/mo" }, competitor: { kind: "text", value: "10 links/mo" } },
  ],
  whyLeave: [
    "Bitly's free tier gives you 10 branded links per month. Rift's gives you 100, with a custom domain, no card required.",
    "Bitly tracks clicks only. If a user taps a link on their phone and installs your app, Bitly sees a click and nothing else. Rift follows the full funnel.",
    "Bitly has no SDK, no AASA hosting, and no native mobile support. For a developer building a mobile product, that means writing all of the deep-linking plumbing yourself.",
    "Bitly's pricing is per-link and scales aggressively past the free tier. Rift's is per-request and stays predictable for API workloads.",
  ],
  whereBetter: [
    "QR code generation with design customization is built in and more polished.",
    "Non-technical users get a richer dashboard — Bitly optimizes for marketers, which matters if the person managing links is not in engineering.",
    "Mature integrations with mainstream marketing tools (HubSpot, Salesforce, Slack) for non-mobile link distribution.",
  ],
  pricing: [
    { scale: "Individual / indie (100 links/mo)", rift: "Free tier covers it", competitor: "$35/mo (Starter) for 500 links" },
    { scale: "Small team (5k links/mo, 50k clicks)", rift: "$49/mo flat", competitor: "$199/mo (Premium)" },
    { scale: "High volume (100k links, 1M clicks)", rift: "Volume contract or ~$10k at per-request", competitor: "Enterprise — quote only" },
  ],
  migrationSteps: [
    "Export all Bitly links via their API. The response includes the slug, destination, created_at, and custom domain.",
    "POST each link to Rift's `/v1/links` endpoint with `custom_slug` set to preserve the short URL. Batch 100 at a time to stay under rate limits.",
    "Point your Bitly custom domain's CNAME at Rift. TLS re-provisions via Let's Encrypt; AASA auto-serves if you also register an iOS app.",
    "Update any scheduled exports or integrations that read Bitly's click data to consume Rift's analytics API instead.",
    "For mobile apps, add the Rift iOS or Android SDK. This is a new capability — Bitly did not offer it — so you are gaining, not replacing.",
    "Run a short overlap period to confirm click counts match, then sunset the Bitly account.",
  ],
  faqs: [
    {
      q: "Is Rift open source like some Bitly alternatives?",
      a: "Yes. Rift's server is open source and self-hostable — built in Rust on top of MongoDB. You can run it on your own infrastructure for internal use or as a managed service. Bitly is closed-source SaaS. If you need to self-host for compliance, data-residency, or cost reasons, Rift is the only one of the two that supports it.",
    },
    {
      q: "Can Rift replace Bitly for a marketing team?",
      a: "Yes for short links, QR codes, and click analytics. The API and dashboard do what Bitly does for those use cases. Where Bitly still has an edge is in polish for non-technical users — the marketing UI is more mature. If your team is developer-led, Rift will feel natural. If it is marketing-led and nobody will touch an API, Bitly's dashboard may be worth the price.",
    },
    {
      q: "What is the cheapest Bitly alternative with a custom domain?",
      a: "Rift's free tier. You get 100 branded links per month on your own domain, with TLS auto-provisioned via Let's Encrypt, no credit card required. Bitly charges $35/month minimum to get branded domains at all.",
    },
    {
      q: "Does Rift do everything Bitly does?",
      a: "Rift covers short URLs, custom domains, click analytics, and API access — the core Bitly feature set. Rift's QR code support is API-only and less polished than Bitly's. Rift adds native mobile deep linking, install attribution, conversion webhooks, and MCP support — features Bitly does not offer at any tier.",
    },
    {
      q: "Can I migrate my Bitly links without losing the short URLs?",
      a: "Yes. Rift's POST /v1/links endpoint accepts a `custom_slug` parameter. Export your Bitly links, script a bulk import with each slug preserved, and update your domain's CNAME to point at Rift. Existing short URLs keep working; nothing breaks in production.",
    },
  ],
  relatedBlogPosts: [
    { title: "Rift vs Branch vs Bitly: An Honest Comparison", href: "/blog/rift-vs-branch-vs-bitly" },
  ],
};

export const COMPETITORS: Record<string, Competitor> = {
  branch,
  bitly,
};

export function getCompetitor(slug: string): Competitor | null {
  return COMPETITORS[slug] ?? null;
}

export function getAllCompetitors(): Competitor[] {
  return Object.values(COMPETITORS);
}

export function getAllCompetitorSlugs(): string[] {
  return Object.keys(COMPETITORS);
}
