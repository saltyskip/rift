import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "Deferred Deep Linking — Rift Docs",
  description: "Route users to specific content even if they didn't have the app installed when they clicked.",
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
              <li>App calls <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">POST /v1/attribution/install</code> to record the attribution</li>
              <li>App routes user to the deep link destination</li>
            </ol>
          </Step>

          <Step n={2} title="Using the native SDKs (recommended)">
            <p>
              The <a href="/docs/ios-sdk" className="text-[#2dd4bf] hover:underline">iOS SDK</a> and{" "}
              <a href="/docs/android-sdk" className="text-[#2dd4bf] hover:underline">Android SDK</a> provide{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">parseClipboardLink()</code> and{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">parseReferrerLink()</code> helpers
              that handle URL parsing for you. See those pages for full integration guides.
            </p>
          </Step>

          <Step n={3} title="iOS — Manual integration">
            <p>
              The landing page copies the full link URL to the clipboard. On first launch after install,
              read the clipboard, extract the link ID from the URL path, and resolve:
            </p>
            <CodeBlock lang="swift">{`func checkDeferredDeepLink() async {
    guard let clipboard = UIPasteboard.general.string,
          let linkId = parseClipboardLink(text: clipboard) else { return }

    UIPasteboard.general.string = ""  // Clear after reading

    // Fetch link data
    let url = URL(string: "https://api.riftl.ink/r/\\(linkId)")!
    var request = URLRequest(url: url)
    request.setValue("application/json", forHTTPHeaderField: "Accept")

    guard let (data, _) = try? await URLSession.shared.data(for: request),
          let link = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
        return
    }

    // Report attribution
    let installId = UIDevice.current.identifierForVendor?.uuidString ?? UUID().uuidString
    let appVersion = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "unknown"

    let rift = RiftSdk(publishableKey: "pk_live_YOUR_KEY")
    let _ = try? await rift.reportAttribution(
        linkId: linkId,
        installId: installId,
        appVersion: appVersion
    )

    // Navigate to the deep link
    if let deepLink = link["ios_deep_link"] as? String {
        handleDeepLink(deepLink)
    }
}`}</CodeBlock>
            <Callout type="info">
              The <code>parseClipboardLink()</code> function handles both the URL format
              (<code>https://go.example.com/summer-sale</code>) and the legacy <code>rift:&lt;link_id&gt;</code> format.
            </Callout>
          </Step>

          <Step n={4} title="Android — Manual integration">
            <CodeBlock lang="kotlin">{`import com.android.installreferrer.api.*

fun checkDeferredDeepLink() {
    val client = InstallReferrerClient.newBuilder(this).build()
    client.startConnection(object : InstallReferrerStateListener {
        override fun onInstallReferrerSetupFinished(code: Int) {
            if (code == InstallReferrerResponse.OK) {
                val referrer = client.installReferrer.installReferrer
                val linkId = parseReferrerLink(referrer)
                if (linkId != null) {
                    resolveAndAttribute(linkId)
                }
            }
            client.endConnection()
        }
        override fun onInstallReferrerServiceDisconnected() {}
    })
}

suspend fun resolveAndAttribute(linkId: String) {
    // Fetch link data
    val url = URL("https://api.riftl.ink/r/$linkId")
    val conn = url.openConnection() as HttpURLConnection
    conn.setRequestProperty("Accept", "application/json")
    val link = JSONObject(conn.inputStream.bufferedReader().readText())

    // Report attribution
    val rift = RiftSdk(publishableKey = "pk_live_YOUR_KEY")
    rift.reportAttribution(
        linkId = linkId,
        installId = getInstallId(),
        appVersion = BuildConfig.VERSION_NAME
    )

    // Navigate to the deep link
    link.optString("android_deep_link")?.let { handleDeepLink(it) }
}`}</CodeBlock>
          </Step>

          <Step n={5} title="Resolve a link (API)">
            <p>To get the link data for routing, send a JSON request to the public resolve endpoint:</p>
            <CodeBlock>{`curl https://api.riftl.ink/r/summer-sale \\
  -H "Accept: application/json"`}</CodeBlock>
            <p>Response:</p>
            <CodeBlock lang="json">{`{
  "link_id": "summer-sale",
  "ios_deep_link": "myapp://promo/summer-sale",
  "android_deep_link": "myapp://promo/summer-sale",
  "web_url": "https://example.com/promo/summer-sale",
  "metadata": { "title": "Summer Sale — 50% Off" },
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
