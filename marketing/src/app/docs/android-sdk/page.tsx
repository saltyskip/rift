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
          Native Kotlin SDK for click tracking, attribution, and user binding.
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
              You need a <a href="/docs/publishable-keys" className="text-[#2dd4bf] hover:underline">publishable key</a> and
              a storage backend. The SDK ships with{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">SharedPrefsStorage</code>{" "}
              which persists data via Android&apos;s <code>SharedPreferences</code>.
            </p>
            <CodeBlock lang="kotlin">{`import ink.riftl.sdk.*

// Initialize once (e.g., in Application.onCreate).
val rift = RiftSdk(
    config = RiftConfig(
        publishableKey = "pk_live_YOUR_KEY",
        baseUrl = null,
        logLevel = null
    ),
    storage = SharedPrefsStorage(applicationContext)
)`}</CodeBlock>
            <Callout type="info">
              The SDK generates a persistent <code>install_id</code> (UUID) on first launch
              and stores it in SharedPreferences. On Android, app data is wiped on uninstall
              by design, so the install ID does not survive reinstallation. For fresh-install
              attribution, use the{" "}
              <a href="https://developer.android.com/google/play/installreferrer" target="_blank" rel="noopener noreferrer" className="text-[#2dd4bf] hover:underline">
                Google Play Install Referrer
              </a>{" "}
              (see below).
            </Callout>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">User Binding</h2>

          <Step n={3} title="Bind the user after signup or login">
            <p>
              Call <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">setUserId</code>{" "}
              wherever you already handle your user session. The SDK persists the binding
              locally, syncs it to the server, and retries automatically on the next app
              launch if the network call fails.
            </p>
            <CodeBlock lang="kotlin">{`// Wherever you know the user is signed in:
lifecycleScope.launch {
    runCatching { rift.setUserId(userId = currentUser.id) }
}`}</CodeBlock>
            <p className="text-[13px] text-[#71717a]">
              Idempotent — safe to call on every launch with the same user ID. A no-op if
              the binding is already synced.
            </p>
          </Step>

          <Step n={4} title="Clear on logout">
            <p>
              Remove the user binding when the user signs out. The install ID is preserved.
            </p>
            <CodeBlock lang="kotlin">{`rift.clearUserId()`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Click Tracking</h2>

          <Step n={5} title="Record a click">
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

          <Step n={6} title="Read link ID from install referrer">
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

          <Step n={7} title="Resolve and report attribution">
            <CodeBlock lang="kotlin">{`suspend fun resolveAndAttribute(linkId: String) {
    // Report attribution — use the SDK's persistent install ID.
    try {
        val success = rift.reportAttribution(
            linkId = linkId,
            installId = rift.installId(),
            appVersion = BuildConfig.VERSION_NAME
        )
        Log.d("Rift", "Attribution reported: \$success")
    } catch (e: RiftError) {
        Log.e("Rift", "Attribution error", e)
    }

    // Fetch link data for navigation.
    try {
        val link = rift.getLink(linkId = linkId)
        link.androidDeepLink?.let { handleDeepLink(it) }
    } catch (e: RiftError) {
        Log.e("Rift", "Link fetch error", e)
    }
}`}</CodeBlock>
            <Callout type="info">
              Use <code>rift.installId()</code> to get the SDK&apos;s persistent install ID
              instead of generating your own. The same UUID is used by <code>setUserId</code>
              for user binding.
            </Callout>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">API Reference</h2>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-2 py-1 rounded text-[15px]">RiftSdk(config, storage)</code>
            </h3>
            <p className="text-[15px] text-[#a1a1aa]">
              Constructor. Takes a <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">RiftConfig</code> (publishable
              key + optional base URL and log level) and a storage backend implementing the{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">RiftStorage</code> interface.
              Use the bundled <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">SharedPrefsStorage(context)</code> for
              production.
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
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">installId()</td>
                    <td className="px-4 py-2.5 font-mono">String (@Throws)</td>
                    <td className="px-4 py-2.5">Returns the persistent install UUID, generating on first call.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">setUserId(userId)</td>
                    <td className="px-4 py-2.5 font-mono">Unit (suspend @Throws)</td>
                    <td className="px-4 py-2.5">Binds the install to a user. Persists + syncs + retries on next launch.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">clearUserId()</td>
                    <td className="px-4 py-2.5 font-mono">Unit (@Throws)</td>
                    <td className="px-4 py-2.5">Removes stored user binding. Call on logout.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">click(linkId)</td>
                    <td className="px-4 py-2.5 font-mono">ClickResult</td>
                    <td className="px-4 py-2.5">Suspend. Records a click and returns link data.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">reportAttribution(linkId, installId, appVersion)</td>
                    <td className="px-4 py-2.5 font-mono">Boolean</td>
                    <td className="px-4 py-2.5">Suspend. Reports an install attribution to Rift.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">getLink(linkId)</td>
                    <td className="px-4 py-2.5 font-mono">GetLinkResult</td>
                    <td className="px-4 py-2.5">Suspend. Fetches link data without recording a click.</td>
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
