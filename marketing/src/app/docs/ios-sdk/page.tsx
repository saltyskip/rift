import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "iOS SDK — Rift Docs",
  description: "Integrate Rift deep linking into your iOS app with the native Swift SDK.",
};

export default function IosSdkPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Mobile SDK</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">iOS SDK</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Native Swift SDK for click tracking, attribution, and user binding.
          Built with Rust and compiled to a Swift Package via UniFFI.
          The SDK persists the install ID in the iOS Keychain, so it survives
          app reinstalls.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Installation</h2>

          <Step n={1} title="Add the Swift Package">
            <p>
              Download the latest <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rift-ios-sdk-*.tar.gz</code> from{" "}
              <a href="https://github.com/saltyskip/rift/releases" target="_blank" rel="noopener noreferrer" className="text-[#2dd4bf] hover:underline">GitHub Releases</a>.
              Extract it and add the directory as a local Swift package in Xcode:
            </p>
            <p>
              <strong className="text-[#fafafa]">File &rarr; Add Package Dependencies &rarr; Add Local</strong> and select the extracted <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">ios/</code> directory.
            </p>
          </Step>

          <Step n={2} title="Initialize">
            <p>
              You need a <a href="/docs/publishable-keys" className="text-[#2dd4bf] hover:underline">publishable key</a> and
              a storage backend. The SDK ships with{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">KeychainStorage</code>{" "}
              which persists data across app reinstalls via the iOS Keychain.
            </p>
            <CodeBlock lang="swift">{`import RiftSDK

// Initialize once at app launch (e.g., in AppDelegate or @main).
let rift = RiftSdk(
    config: RiftConfig(
        publishableKey: "pk_live_YOUR_KEY",
        baseUrl: nil,
        logLevel: nil
    ),
    storage: KeychainStorage()
)`}</CodeBlock>
            <Callout type="info">
              The SDK generates a persistent <code>install_id</code> (UUID) on first launch
              and stores it in the Keychain. Unlike <code>UserDefaults</code>, Keychain entries
              survive app deletion and reinstallation — so the same install keeps its identity
              even if the user re-downloads your app.
            </Callout>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">User Binding</h2>

          <Step n={3} title="Bind the user after signup or login">
            <p>
              Call <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">setUserId</code>{" "}
              wherever you already handle your user session — after signup, login, or session restore.
              The SDK persists the binding locally, syncs it to the server, and retries
              automatically on the next app launch if the network call fails.
            </p>
            <CodeBlock lang="swift">{`// Wherever you know the user is signed in:
Task {
    try? await rift.setUserId(userId: currentUser.id)
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
            <CodeBlock lang="swift">{`try rift.clearUserId()`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Click Tracking</h2>

          <Step n={5} title="Record a click">
            <p>If your app opens Rift links internally (e.g., share sheets), record the click:</p>
            <CodeBlock lang="swift">{`do {
    let result = try await rift.click(linkId: "summer-sale")
    print("Platform: \\(result.platform)")
    print("Deep link: \\(result.iosDeepLink ?? "none")")
} catch {
    print("Click error: \\(error)")
}`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Post-Install Attribution</h2>

          <Step n={6} title="Check clipboard on first launch">
            <p>
              When a user clicks a Rift link on iOS, the landing page copies the full link URL to
              the clipboard. On first launch after install, read the clipboard, extract the link ID,
              and report attribution:
            </p>
            <CodeBlock lang="swift">{`func handleDeferredDeepLink() async {
    guard let clipboard = UIPasteboard.general.string,
          let linkId = parseClipboardLink(text: clipboard) else {
        return
    }

    // Clear clipboard after reading.
    UIPasteboard.general.string = ""

    // Report attribution — the SDK uses its persistent install_id automatically.
    do {
        let _ = try await rift.reportAttribution(
            linkId: linkId,
            installId: try rift.installId(),
            appVersion: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "unknown"
        )
    } catch {
        print("Attribution error: \\(error)")
    }

    // Fetch link data to navigate.
    if let link = try? await rift.getLink(linkId: linkId) {
        if let deepLink = link.iosDeepLink {
            handleDeepLink(deepLink)
        }
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
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-2 py-1 rounded text-[15px]">RiftSdk(config:, storage:)</code>
            </h3>
            <p className="text-[15px] text-[#a1a1aa]">
              Constructor. Takes a <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">RiftConfig</code> (publishable
              key + optional base URL and log level) and a storage backend implementing the{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">RiftStorage</code> protocol.
              Use the bundled <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">KeychainStorage()</code> for
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
                    <td className="px-4 py-2.5 font-mono">String (throws)</td>
                    <td className="px-4 py-2.5">Returns the persistent install UUID, generating on first call.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">setUserId(userId)</td>
                    <td className="px-4 py-2.5 font-mono">Void (async throws)</td>
                    <td className="px-4 py-2.5">Binds the install to a user. Persists + syncs + retries on next launch.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">clearUserId()</td>
                    <td className="px-4 py-2.5 font-mono">Void (throws)</td>
                    <td className="px-4 py-2.5">Removes stored user binding. Call on logout.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">click(linkId)</td>
                    <td className="px-4 py-2.5 font-mono">ClickResult</td>
                    <td className="px-4 py-2.5">Async. Records a click and returns link data.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">reportAttribution(linkId, installId, appVersion)</td>
                    <td className="px-4 py-2.5 font-mono">Bool</td>
                    <td className="px-4 py-2.5">Async. Reports an install attribution to Rift.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">getLink(linkId)</td>
                    <td className="px-4 py-2.5 font-mono">GetLinkResult</td>
                    <td className="px-4 py-2.5">Async. Fetches link data without recording a click.</td>
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
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">parseClipboardLink(text:)</td>
                    <td className="px-4 py-2.5">Extracts link ID from a clipboard URL (e.g. <code>https://go.example.com/summer-sale</code>) or legacy <code>rift:&lt;link_id&gt;</code> format. Returns <code>nil</code> if not found.</td>
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
