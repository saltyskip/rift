import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

export const metadata: Metadata = {
  title: "Deferred Deep Linking — Rift Docs",
  description: "Route users to specific content even if they didn't have the app installed when they clicked.",
};

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

export default function DeferredPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Deep Linking</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Deferred Deep Linking</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Route users to specific content even if they didn&apos;t have the app installed when they clicked.
          Rift passes the link ID through the install flow and delivers it to the app after install.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <Step n={1} title="How it works">
            <ol className="list-decimal pl-5 space-y-1">
              <li>User clicks a Rift link on mobile</li>
              <li>Rift copies the link ID to the clipboard or install referrer</li>
              <li><strong className="text-[#fafafa]">iOS:</strong> link ID is copied to clipboard as <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">rift:&lt;link_id&gt;</code></li>
              <li><strong className="text-[#fafafa]">Android:</strong> link ID is appended to the Play Store URL as <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">rift_link=&lt;link_id&gt;</code></li>
              <li>User installs the app and opens it</li>
              <li>App reads the link ID and sends it to <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">POST /v1/deferred</code></li>
            </ol>
          </Step>

          <Step n={2} title="Using the native SDKs (recommended)">
            <p>
              The <a href="/docs/ios-sdk" className="text-[#2dd4bf] hover:underline">iOS SDK</a> and{" "}
              <a href="/docs/android-sdk" className="text-[#2dd4bf] hover:underline">Android SDK</a> handle
              clipboard/referrer reading and API calls for you. See those pages for full integration guides.
            </p>
          </Step>

          <Step n={3} title="iOS — Manual integration">
            <CodeBlock lang="swift">{`func checkDeferredDeepLink() {
    guard let clipboard = UIPasteboard.general.string,
          clipboard.hasPrefix("rift:") else { return }

    let linkId = String(clipboard.dropFirst("rift:".count))
    UIPasteboard.general.string = ""  // Clear after reading
    resolveDeferred(linkId: linkId)
}`}</CodeBlock>
          </Step>

          <Step n={4} title="Android — Manual integration">
            <CodeBlock lang="kotlin">{`val client = InstallReferrerClient.newBuilder(this).build()
client.startConnection(object : InstallReferrerStateListener {
    override fun onInstallReferrerSetupFinished(code: Int) {
        if (code == InstallReferrerResponse.OK) {
            val referrer = client.installReferrer.installReferrer
            val linkId = Uri.parse("?\$referrer")
                .getQueryParameter("rift_link")
            if (linkId != null) resolveDeferred(linkId)
        }
        client.endConnection()
    }
    override fun onInstallReferrerServiceDisconnected() {}
})`}</CodeBlock>
          </Step>

          <Step n={5} title="Resolve the link (API)">
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/deferred \\
  -H "Content-Type: application/json" \\
  -d '{
    "link_id": "summer-sale",
    "install_id": "device-uuid-here",
    "domain": "go.yourcompany.com"
  }'`}</CodeBlock>
            <p>Response (matched):</p>
            <CodeBlock lang="json">{`{
  "matched": true,
  "link_id": "summer-sale",
  "ios_deep_link": "myapp://promo/summer-sale",
  "android_deep_link": "myapp://promo/summer-sale",
  "metadata": { "title": "Summer Sale — 50% Off" }
}`}</CodeBlock>
            <p>Response (not matched):</p>
            <CodeBlock lang="json">{`{ "matched": false }`}</CodeBlock>
          </Step>
        </section>
      </div>
    </div>
  );
}
