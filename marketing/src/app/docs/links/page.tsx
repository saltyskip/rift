import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsSetupTabs } from "@/components/docs-setup-tabs";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "Create Links — Rift Docs",
  description: "Create deep links with per-platform destinations, metadata, and smart resolution.",
  alternates: { canonical: "/docs/links" },
};

export default function LinksPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Deep Linking</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Create Links</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Create deep links with per-platform destinations and metadata for rich social previews.
        </p>
      </div>

      <div className="space-y-10">
        <DocsSetupTabs
          title="Create a link"
          tabs={[
            {
              id: "cli",
              label: "CLI (Recommended)",
              children: (
                <div className="space-y-6">
                  <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
                    <p>The CLI walks you through link creation interactively, or you can pass flags directly:</p>
                    <CodeBlock lang="bash">{`# Interactive — prompts for each field
rift links create

# Non-interactive — pass flags directly
rift links create \\
  --web-url https://example.com/promo/summer-sale \\
  --ios-deep-link myapp://promo/summer-sale \\
  --android-deep-link myapp://promo/summer-sale \\
  --custom-id summer-sale \\
  --preview-title "Summer Sale — 50% Off" \\
  --preview-description "Limited time offer on all products" \\
  --preview-image-url https://example.com/promo-banner.jpg`}</CodeBlock>
                    <p>The CLI outputs the link ID and URL, and suggests <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rift links test</code> to preview how it resolves across platforms:</p>
                    <CodeBlock lang="bash">{`rift links test summer-sale`}</CodeBlock>
                  </div>
                </div>
              ),
            },
            {
              id: "http",
              label: "HTTP",
              children: (
                <div className="space-y-6">
                  <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
                    <p>Specify where each platform should go — deep link URI, store URL, and web fallback:</p>
                    <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/links \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "custom_id": "summer-sale",
    "ios_deep_link": "myapp://promo/summer-sale",
    "android_deep_link": "myapp://promo/summer-sale",
    "web_url": "https://example.com/promo/summer-sale",
    "ios_store_url": "https://apps.apple.com/app/id123456789",
    "android_store_url": "https://play.google.com/store/apps/details?id=com.example.myapp",
    "social_preview": {
      "title": "Summer Sale — 50% Off",
      "description": "Limited time offer on all products",
      "image_url": "https://example.com/promo-banner.jpg"
    },
    "metadata": {
      "campaign": "summer-sale"
    },
    "agent_context": {
      "action": "purchase",
      "cta": "Get 50% Off",
      "description": "Summer clearance sale on all products with free shipping"
    }
  }'`}</CodeBlock>
                    <p>Response:</p>
                    <CodeBlock lang="json">{`{
  "link_id": "summer-sale",
  "url": "https://api.riftl.ink/r/summer-sale"
}`}</CodeBlock>
                  </div>
                </div>
              ),
            },
          ]}
        />

        <section className="space-y-6">
          <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
            <Callout type="warning">
              Custom IDs (vanity slugs like <code>summer-sale</code>) require a{" "}
              <a href="/docs/domains" className="underline">verified custom domain</a>.
              They are unique per tenant — different tenants can use the same slug on their own domains
              (e.g. <code>go.acme.com/summer-sale</code> and <code>go.brand.com/summer-sale</code>).
              Links with custom IDs resolve via your custom domain, not the primary <code>riftl.ink/r/</code> path.
            </Callout>
            <p>
              If you omit <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">custom_id</code>,
              Rift auto-generates a short ID (e.g. <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">A1B2C3D4</code>).
              Auto-generated links work for all tenants and resolve via <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">riftl.ink/r/A1B2C3D4</code> —
              no custom domain required.
            </p>
          </div>

          <Step n={2} title="How resolution works">
            <p>When a user clicks the link, Rift detects their platform and serves a smart landing page that:</p>
            <ul className="list-disc pl-5 space-y-1">
              <li><strong className="text-[#fafafa]">iOS</strong> — attempts to open the deep link, falls back to the App Store</li>
              <li><strong className="text-[#fafafa]">Android</strong> — attempts to open the deep link, falls back to the Play Store</li>
              <li><strong className="text-[#fafafa]">Desktop</strong> — redirects to the web URL</li>
            </ul>
            <p>
              The landing page includes your app branding (from app registration) and OG tags from <code>social_preview</code>
              for rich social previews.
            </p>
          </Step>

          <Step n={3} title="Download QR codes">
            <p>
              QR codes are generated from the link&apos;s canonical Rift URL and support Dub-style styling parameters.
            </p>
            <CodeBlock lang="bash">{`# CLI
rift links qr summer-sale \\
  --format png \\
  --output summer-sale.png \\
  --size 600 \\
  --level H \\
  --fg-color '#111827' \\
  --bg-color '#ffffff' \\
  --logo https://example.com/logo.png

# HTTP
curl -L "https://api.riftl.ink/v1/links/summer-sale/qr.png?size=600&level=H&fgColor=%23111827&bgColor=%23ffffff&logo=https%3A%2F%2Fexample.com%2Flogo.png" \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -o summer-sale.png`}</CodeBlock>
            <p>
              Use <code>qr.svg</code> instead of <code>qr.png</code> for SVG output. Supported options are
              <code> logo</code>, <code>size</code>, <code>level</code>, <code>fgColor</code>,
              <code>bgColor</code>, <code>hideLogo</code>, and <code>margin</code>.
            </p>
          </Step>

          <Step n={4} title="JSON resolution for agents">
            <p>
              Agents sending <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">Accept: application/json</code> receive
              all destinations, social preview fields, and metadata as JSON. Use your custom domain for custom IDs:
            </p>
            <CodeBlock lang="json">{`# Custom ID via custom domain
curl https://go.yourcompany.com/summer-sale \\
  -H "Accept: application/json"

# Auto-generated ID via primary domain
curl https://api.riftl.ink/r/A1B2C3D4 \\
  -H "Accept: application/json"

{
  "link_id": "summer-sale",
  "ios_deep_link": "myapp://promo/summer-sale",
  "android_deep_link": "myapp://promo/summer-sale",
  "web_url": "https://example.com/promo/summer-sale",
  "ios_store_url": "https://apps.apple.com/app/id123456789",
  "android_store_url": "https://play.google.com/store/apps/details?id=com.example.myapp",
  "social_preview": {
    "title": "Summer Sale — 50% Off",
    "description": "Limited time offer on all products",
    "image_url": "https://example.com/promo-banner.jpg"
  },
  "metadata": { "campaign": "summer-sale" },
  "agent_context": {
    "action": "purchase",
    "cta": "Get 50% Off",
    "description": "Summer clearance sale on all products with free shipping"
  },
  "_rift_meta": {
    "context": "This is a Rift deep link...",
    "source": "tenant_asserted",
    "status": "active",
    "tenant_domain": "go.yourcompany.com",
    "tenant_verified": true
  }
}`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Handle incoming links</h2>

          <Step n={5} title="iOS — SceneDelegate or AppDelegate">
            <CodeBlock lang="swift">{`// SceneDelegate.swift
func scene(_ scene: UIScene,
           continue userActivity: NSUserActivity) {
    guard userActivity.activityType ==
              NSUserActivityTypeBrowsingWeb,
          let url = userActivity.webpageURL else { return }

    let linkId = url.path
        .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
    handleDeepLink(linkId: linkId)
}`}</CodeBlock>
          </Step>

          <Step n={6} title="Android — Intent handling">
            <CodeBlock lang="kotlin">{`// MainActivity.kt
override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    intent?.data?.let { uri ->
        val linkId = uri.path?.trimStart('/')
        if (linkId != null) handleDeepLink(linkId)
    }
}`}</CodeBlock>
          </Step>
        </section>
      </div>
    </div>
  );
}
