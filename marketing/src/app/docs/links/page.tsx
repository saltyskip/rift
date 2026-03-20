import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Create Links — Rift Docs",
  description: "Create deep links with per-platform destinations, metadata, and smart resolution.",
};

function CodeBlock({ children }: { children: string }) {
  return (
    <pre className="bg-[#0c0c0e] border border-[#1e1e22] rounded-lg p-4 overflow-x-auto text-[13px] leading-relaxed font-mono text-[#a1a1aa]">
      <code>{children}</code>
    </pre>
  );
}

function Step({ n, title, children }: { n: number; title: string; children: React.ReactNode }) {
  return (
    <div className="relative pl-10">
      <div className="absolute left-0 top-0 flex h-7 w-7 items-center justify-center rounded-full bg-[#2dd4bf]/10 text-[#2dd4bf] text-sm font-semibold border border-[#2dd4bf]/20">
        {n}
      </div>
      <h3 className="text-lg font-semibold text-[#fafafa] mb-3">{title}</h3>
      <div className="space-y-3 text-[15px] text-[#a1a1aa] leading-relaxed">{children}</div>
    </div>
  );
}

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
        <section className="space-y-6">
          <Step n={1} title="Create a link with per-platform destinations">
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
    "metadata": {
      "title": "Summer Sale — 50% Off",
      "description": "Limited time offer on all products",
      "image": "https://example.com/promo-banner.jpg"
    }
  }'`}</CodeBlock>
            <p>Response:</p>
            <CodeBlock>{`{
  "link_id": "summer-sale",
  "url": "https://api.riftl.ink/r/summer-sale"
}`}</CodeBlock>
          </Step>

          <Step n={2} title="How resolution works">
            <p>When a user clicks the link, Relay detects their platform and serves a smart landing page that:</p>
            <ul className="list-disc pl-5 space-y-1">
              <li><strong className="text-[#fafafa]">iOS</strong> — attempts to open the deep link, falls back to the App Store</li>
              <li><strong className="text-[#fafafa]">Android</strong> — attempts to open the deep link, falls back to the Play Store</li>
              <li><strong className="text-[#fafafa]">Desktop</strong> — redirects to the web URL</li>
            </ul>
            <p>
              The landing page includes your app branding (from app registration) and OG tags from link metadata
              for rich social previews.
            </p>
          </Step>

          <Step n={3} title="JSON resolution for agents">
            <p>
              Agents sending <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">Accept: application/json</code> receive
              all destinations and metadata as JSON:
            </p>
            <CodeBlock>{`curl https://api.riftl.ink/r/summer-sale \\
  -H "Accept: application/json"

{
  "link_id": "summer-sale",
  "ios_deep_link": "myapp://promo/summer-sale",
  "android_deep_link": "myapp://promo/summer-sale",
  "web_url": "https://example.com/promo/summer-sale",
  "ios_store_url": "https://apps.apple.com/app/id123456789",
  "android_store_url": "https://play.google.com/store/apps/details?id=com.example.myapp",
  "metadata": { "title": "Summer Sale — 50% Off", ... }
}`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Handle incoming links</h2>

          <Step n={4} title="iOS — SceneDelegate or AppDelegate">
            <CodeBlock>{`// SceneDelegate.swift
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

          <Step n={5} title="Android — Intent handling">
            <CodeBlock>{`// MainActivity.kt
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
