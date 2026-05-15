import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "Deferred Deep Linking — Rift Docs",
  description: "Route users to specific content even if they didn't have the app installed when they clicked.",
  alternates: { canonical: "/docs/deferred" },
};

export default function DeferredPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Deep Linking</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Deferred Deep Linking</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Route users to specific content even if they didn&apos;t have the app installed when they clicked.
          Rift passes the link URL through the install flow and delivers it to the app after install.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <Step n={1} title="How it works">
            <ol className="list-decimal pl-5 space-y-1">
              <li>User clicks a Rift link on mobile (via web SDK or landing page)</li>
              <li>
                <strong className="text-[#fafafa]">iOS:</strong> the full link URL is copied to the clipboard
                (e.g. <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">https://go.yourcompany.com/summer-sale</code>)
              </li>
              <li>
                <strong className="text-[#fafafa]">Android:</strong> the link ID is appended to the Play Store URL
                as <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">referrer=rift_link%3Dsummer-sale</code> in the install referrer
              </li>
              <li>User installs the app and opens it</li>
              <li>App reads the clipboard (iOS) or install referrer (Android) to get the link URL/ID</li>
              <li>App extracts the link ID and calls <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">GET /r/&#123;link_id&#125;</code> with <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">Accept: application/json</code> to get link data</li>
              <li>App calls <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">POST /v1/lifecycle/attribute</code> to record the attribution</li>
              <li>App routes user to the deep link destination</li>
            </ol>
          </Step>

          <Step n={2} title="iOS — SDK (recommended)">
            <p>
              The SDK does everything in one call: parses the clipboard, reports attribution,
              and returns the link data for navigation.
            </p>
            <CodeBlock lang="swift">{`// On first launch
if let result = try await rift.checkDeferredDeepLink(
    clipboardText: UIPasteboard.general.string
) {
    UIPasteboard.general.string = ""  // clear after reading
    if let deepLink = result.iosDeepLink {
        handleDeepLink(deepLink)
    }
}`}</CodeBlock>
            <Callout type="info">
              The caller reads the clipboard explicitly because iOS 16+ shows a paste
              permission dialog. The SDK does NOT read the clipboard itself — your app
              controls when that dialog appears.
            </Callout>
          </Step>

          <Step n={3} title="Android — SDK (recommended)">
            <p>
              On Android, use the install referrer to get the link ID, then resolve and attribute:
            </p>
            <CodeBlock lang="kotlin">{`import com.android.installreferrer.api.*

fun checkDeferredDeepLink() {
    val client = InstallReferrerClient.newBuilder(this).build()
    client.startConnection(object : InstallReferrerStateListener {
        override fun onInstallReferrerSetupFinished(code: Int) {
            if (code == InstallReferrerResponse.OK) {
                val referrer = client.installReferrer.installReferrer
                val linkId = parseReferrerLink(referrer)
                if (linkId != null) {
                    lifecycleScope.launch {
                        rift.reportAttributionForLink(linkId = linkId)
                        val link = rift.getLink(linkId = linkId)
                        link.androidDeepLink?.let { handleDeepLink(it) }
                    }
                }
            }
            client.endConnection()
        }
        override fun onInstallReferrerServiceDisconnected() {}
    })
}`}</CodeBlock>
          </Step>

          <Step n={4} title="Resolve a link (API)">
            <p>To get the link data for routing, send a JSON request to the public resolve endpoint:</p>
            <CodeBlock>{`curl https://api.riftl.ink/r/summer-sale \\
  -H "Accept: application/json"`}</CodeBlock>
            <p>Response:</p>
            <CodeBlock lang="json">{`{
  "link_id": "summer-sale",
  "ios_deep_link": "myapp://promo/summer-sale",
  "android_deep_link": "myapp://promo/summer-sale",
  "web_url": "https://example.com/promo/summer-sale",
  "social_preview": { "title": "Summer Sale — 50% Off" },
  "agent_context": {
    "action": "purchase",
    "cta": "Get 50% Off"
  },
  "_rift_meta": {
    "status": "active",
    "tenant_domain": "go.yourcompany.com",
    "tenant_verified": true
  }
}`}</CodeBlock>
          </Step>
        </section>
      </div>
    </div>
  );
}
