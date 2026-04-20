import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "Android SDK — Rift Docs",
  description: "Integrate Rift deep linking into your Android app with the native Kotlin SDK.",
};

export default function AndroidSdkPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Mobile SDK</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Android SDK</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Native Kotlin SDK for click tracking and attribution.
          Built with Rust and compiled to a Kotlin library via UniFFI.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Installation</h2>

          <Step n={1} title="Add the library">
            <p>
              Download the latest <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rift-android-sdk-*.tar.gz</code> from{" "}
              <a href="https://github.com/saltyskip/rift/releases" target="_blank" rel="noopener noreferrer" className="text-[#2dd4bf] hover:underline">GitHub Releases</a>.
              Extract into your project and add it as a module:
            </p>
            <CodeBlock lang="kotlin">{`// settings.gradle.kts
include(":rift-sdk")
project(":rift-sdk").projectDir = file("libs/android")

// app/build.gradle.kts
dependencies {
    implementation(project(":rift-sdk"))
}`}</CodeBlock>
          </Step>

          <Step n={2} title="Initialize">
            <p>
              You need a <a href="/docs/publishable-keys" className="text-[#2dd4bf] hover:underline">publishable key</a> to initialize the SDK.
            </p>
            <CodeBlock lang="kotlin">{`import ink.riftl.sdk.*

// Initialize once (e.g., in Application.onCreate).
val rift = RiftSdk(config = RiftConfig(publishableKey = "pk_live_YOUR_KEY", baseUrl = null, logLevel = null))

// Or point to a self-hosted instance:
val rift = RiftSdk(config = RiftConfig(publishableKey = "pk_live_YOUR_KEY", baseUrl = "https://api.yourcompany.com", logLevel = null))`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Click Tracking</h2>

          <Step n={3} title="Record a click">
            <p>If your app opens Rift links internally, record the click:</p>
            <CodeBlock lang="kotlin">{`suspend fun trackClick(linkId: String) {
    try {
        val result = rift.click(linkId = linkId)
        Log.d("Rift", "Platform: \${result.platform}")
        Log.d("Rift", "Deep link: \${result.androidDeepLink}")
    } catch (e: RiftError) {
        Log.e("Rift", "Click error", e)
    }
}`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Post-Install Attribution</h2>

          <Step n={4} title="Read link ID from install referrer">
            <p>
              When a user clicks a Rift link on Android, the link ID is appended to the Play Store URL
              as <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">rift_link=&lt;link_id&gt;</code> in the install referrer.
              Read it and resolve the link after install:
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
                    resolveAndAttribute(linkId)
                }
            }
            client.endConnection()
        }
        override fun onInstallReferrerServiceDisconnected() {}
    })
}`}</CodeBlock>
          </Step>

          <Step n={5} title="Resolve and report attribution">
            <CodeBlock lang="kotlin">{`suspend fun resolveAndAttribute(linkId: String) {
    // Report attribution
    try {
        val success = rift.reportAttribution(
            linkId = linkId,
            installId = getInstallId(),
            appVersion = BuildConfig.VERSION_NAME
        )
        Log.d("Rift", "Attribution reported: $success")
    } catch (e: RiftError) {
        Log.e("Rift", "Attribution error", e)
    }

    // Fetch link data for navigation (public endpoint, no auth needed)
    val url = URL("https://api.riftl.ink/r/$linkId")
    val conn = url.openConnection() as HttpURLConnection
    conn.setRequestProperty("Accept", "application/json")
    val link = JSONObject(conn.inputStream.bufferedReader().readText())

    link.optString("android_deep_link")?.let { deepLink ->
        handleDeepLink(deepLink)
    }
}`}</CodeBlock>
            <Callout type="info">
              The SDK methods are <code>suspend</code> functions — call them from a coroutine scope.
            </Callout>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">API Reference</h2>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-2 py-1 rounded text-[15px]">RiftSdk(config: RiftConfig)</code>
            </h3>
            <p className="text-[15px] text-[#a1a1aa]">
              Constructor. Pass a <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">RiftConfig</code> with the required <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">publishableKey</code> field.
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">baseUrl</code> and <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">logLevel</code> are optional (<code className="text-[#71717a]">baseUrl</code> defaults to <code className="text-[#71717a]">https://api.riftl.ink</code>).
            </p>
          </div>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">Methods</h3>
            <div className="overflow-x-auto">
              <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Method</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Returns</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Description</th>
                  </tr>
                </thead>
                <tbody className="text-[#a1a1aa]">
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">click(linkId)</td>
                    <td className="px-4 py-2.5 font-mono">ClickResult</td>
                    <td className="px-4 py-2.5">Records a click and returns link data.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">reportAttribution(linkId, installId, appVersion)</td>
                    <td className="px-4 py-2.5 font-mono">Boolean</td>
                    <td className="px-4 py-2.5">Reports an install attribution.</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">Free functions</h3>
            <div className="overflow-x-auto">
              <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Function</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Description</th>
                  </tr>
                </thead>
                <tbody className="text-[#a1a1aa]">
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">parseReferrerLink(referrer)</td>
                    <td className="px-4 py-2.5">Extracts link ID from <code>rift_link=&lt;link_id&gt;</code> referrer string. Returns <code>null</code> if not found.</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}
