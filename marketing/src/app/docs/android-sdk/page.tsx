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
          Native Kotlin SDK for deep linking, attribution, user binding, and conversion tracking.
          Built with Rust and compiled to a Kotlin library via UniFFI.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Quick Start</h2>

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

          <Step n={2} title="Initialize (one line)">
            <p>
              You need a <a href="/docs/publishable-keys" className="text-[#2dd4bf] hover:underline">publishable key</a>.
              The convenience constructor auto-wires SharedPreferences storage and reads the
              app version from <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">PackageManager</code>.
            </p>
            <CodeBlock lang="kotlin">{`import ink.riftl.sdk.*

// One line — SharedPreferences storage, app version, all defaults.
val rift = RiftSdk.create("pk_live_YOUR_KEY", applicationContext)

// If you're tracking conversions, pass the source URL too:
val rift = RiftSdk.create(
    "pk_live_YOUR_KEY",
    applicationContext,
    conversionSourceUrl = "https://api.riftl.ink/w/YOUR_SOURCE_TOKEN"
)`}</CodeBlock>
            <Callout type="info">
              The SDK generates a persistent <code>install_id</code> (UUID) on first launch
              and stores it in SharedPreferences. On Android, app data is wiped on uninstall —
              the install ID does not survive reinstallation. For fresh-install attribution,
              use the{" "}
              <a href="https://developer.android.com/google/play/installreferrer" target="_blank" rel="noopener noreferrer" className="text-[#2dd4bf] hover:underline">
                Google Play Install Referrer
              </a>.
            </Callout>
          </Step>

          <Step n={3} title="Bind the user (one line)">
            <p>
              Call{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">setUserId</code>{" "}
              wherever you handle your user session. Safe to call on every launch.
              The SDK handles persistence, sync, and retry.
            </p>
            <CodeBlock lang="kotlin">{`// Wherever you know the user is signed in:
lifecycleScope.launch {
    runCatching { rift.setUserId(userId = currentUser.id) }
}`}</CodeBlock>
          </Step>

          <Step n={4} title="Track conversions (one line)">
            <p>
              Fire a conversion event whenever a user does something worth counting.
            </p>
            <CodeBlock lang="kotlin">{`// On trade completion, purchase, signup — whatever you're measuring:
rift.trackConversion(
    conversionType = "trade",
    idempotencyKey = orderId,
    metadata = mapOf("asset" to "ETH", "side" to "buy")
)`}</CodeBlock>
            <p className="text-[13px] text-[#71717a]">
              Fire-and-forget — the method returns immediately. The server dedupes via{" "}
              <code>idempotencyKey</code>.
            </p>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Deferred Deep Linking</h2>

          <Step n={5} title="Read link ID from install referrer">
            <p>
              When a user clicks a Rift link on Android, the link ID is appended to the Play Store URL.
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
                    lifecycleScope.launch {
                        // One call: reports attribution + fetches link data
                        val result = rift.checkDeferredDeepLink(clipboardText = null)
                        // Or use the referrer-parsed link directly:
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
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Click Tracking</h2>

          <Step n={6} title="Record a click">
            <p>If your app opens Rift links internally, record the click:</p>
            <CodeBlock lang="kotlin">{`val result = rift.click(linkId = "summer-sale")
Log.d("Rift", "Platform: \${result.platform}")
Log.d("Rift", "Deep link: \${result.androidDeepLink}")`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Logout</h2>
          <p className="text-[15px] text-[#a1a1aa]">
            Call <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">clearUserId()</code>{" "}
            when the user signs out. The install ID is preserved.
          </p>
          <CodeBlock lang="kotlin">{`rift.clearUserId()`}</CodeBlock>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">API Reference</h2>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">Constructors</h3>
            <div className="overflow-x-auto">
              <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Constructor</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Description</th>
                  </tr>
                </thead>
                <tbody className="text-[#a1a1aa]">
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">RiftSdk.create(publishableKey, context, conversionSourceUrl?)</td>
                    <td className="px-4 py-2.5">Convenience. Auto-wires SharedPreferences storage + app version. Recommended.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">RiftSdk(config, storage)</td>
                    <td className="px-4 py-2.5">Full control. Pass custom <code>RiftConfig</code> and <code>RiftStorage</code> implementation.</td>
                  </tr>
                </tbody>
              </table>
            </div>
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
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">setUserId(userId)</td>
                    <td className="px-4 py-2.5 font-mono">Unit (suspend @Throws)</td>
                    <td className="px-4 py-2.5">Bind the install to a user. Persists + syncs + retries on next launch.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">trackConversion(type, idempotencyKey, metadata?)</td>
                    <td className="px-4 py-2.5 font-mono">Unit (@Throws)</td>
                    <td className="px-4 py-2.5">Fire a conversion event. Fire-and-forget POST to the source URL.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">checkDeferredDeepLink(clipboardText)</td>
                    <td className="px-4 py-2.5 font-mono">DeferredDeepLinkResult? (suspend @Throws)</td>
                    <td className="px-4 py-2.5">One-call deferred deep link: parse, attribute, fetch link data.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">clearUserId()</td>
                    <td className="px-4 py-2.5 font-mono">Unit (@Throws)</td>
                    <td className="px-4 py-2.5">Remove stored user binding. Call on logout.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">installId()</td>
                    <td className="px-4 py-2.5 font-mono">String (@Throws)</td>
                    <td className="px-4 py-2.5">Persistent install UUID. Generates on first call.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">reportAttributionForLink(linkId)</td>
                    <td className="px-4 py-2.5 font-mono">Boolean (suspend @Throws)</td>
                    <td className="px-4 py-2.5">Simplified attribution — uses internal install_id + app version.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">click(linkId)</td>
                    <td className="px-4 py-2.5 font-mono">ClickResult (suspend @Throws)</td>
                    <td className="px-4 py-2.5">Record a click and return link data.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">getLink(linkId)</td>
                    <td className="px-4 py-2.5 font-mono">GetLinkResult (suspend @Throws)</td>
                    <td className="px-4 py-2.5">Fetch link data without recording a click.</td>
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
