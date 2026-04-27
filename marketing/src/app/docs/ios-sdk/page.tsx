import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "iOS SDK — Rift Docs",
  description: "Integrate Rift deep linking into your iOS app with the native Swift SDK.",
  alternates: { canonical: "/docs/ios-sdk" },
};

export default function IosSdkPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Mobile SDK</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">iOS SDK</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Native Swift SDK for deep linking, attribution, user binding, and conversion tracking.
          Built with Rust and compiled to a Swift Package via UniFFI. Install ID persists across
          app reinstalls via the iOS Keychain.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Quick Start</h2>

          <Step n={1} title="Add the Swift Package">
            <p>
              Download the latest <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rift-ios-sdk-*.tar.gz</code> from{" "}
              <a href="https://github.com/saltyskip/rift/releases" target="_blank" rel="noopener noreferrer" className="text-[#2dd4bf] hover:underline">GitHub Releases</a>.
              Extract it and add as a local Swift package in Xcode:
            </p>
            <p>
              <strong className="text-[#fafafa]">File &rarr; Add Package Dependencies &rarr; Add Local</strong> and select the extracted <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">ios/</code> directory.
            </p>
          </Step>

          <Step n={2} title="Initialize (one line)">
            <p>
              You need a <a href="/docs/publishable-keys" className="text-[#2dd4bf] hover:underline">publishable key</a>.
              The convenience constructor auto-wires Keychain storage and reads the app version from{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">Bundle.main</code>.
            </p>
            <CodeBlock lang="swift">{`import RiftSDK

// One line — Keychain storage, app version, all defaults.
let rift = RiftSdk.create(publishableKey: "pk_live_YOUR_KEY")`}</CodeBlock>
            <Callout type="info">
              The SDK generates a persistent <code>install_id</code> (UUID) on first launch
              and stores it in the Keychain. It survives app deletion and reinstallation.
            </Callout>
          </Step>

          <Step n={3} title="Bind the user (one line)">
            <p>
              Call{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">setUserId</code>{" "}
              wherever you handle your user session — after signup, login, or session restore.
              Safe to call on every launch. The SDK handles persistence, sync, and retry.
            </p>
            <CodeBlock lang="swift">{`// Wherever you know the user is signed in:
Task {
    try? await rift.setUserId(userId: currentUser.id)
}`}</CodeBlock>
          </Step>

          <Step n={4} title="Track conversions (one line)">
            <p>
              Fire a conversion event whenever a user does something worth counting.
              The SDK reads the bound <code>user_id</code> and POSTs to the Rift API
              using your publishable key.
            </p>
            <CodeBlock lang="swift">{`// On trade completion, purchase, signup — whatever you're measuring:
try await rift.trackConversion(
    conversionType: "trade",
    idempotencyKey: orderId,
    metadata: ["asset": "ETH", "side": "buy"]
)`}</CodeBlock>
            <p className="text-[13px] text-[#71717a]">
              The server dedupes via <code>idempotencyKey</code>, so retries are safe.
            </p>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Deferred Deep Linking</h2>

          <Step n={5} title="One-call deferred deep link (3 lines)">
            <p>
              On first launch, check the clipboard for a Rift link. The SDK parses it,
              reports attribution, and returns the link data for navigation — all in one call.
            </p>
            <CodeBlock lang="swift">{`// On first launch:
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
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Click Tracking</h2>

          <Step n={6} title="Record a click">
            <p>If your app opens Rift links internally (e.g., share sheets), record the click:</p>
            <CodeBlock lang="swift">{`let result = try await rift.click(linkId: "summer-sale")
print("Platform: \\(result.platform)")
print("Deep link: \\(result.iosDeepLink ?? "none")")`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Logout</h2>
          <p className="text-[15px] text-[#a1a1aa]">
            Call <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">clearUserId()</code>{" "}
            when the user signs out. The install ID is preserved — only the user binding is removed.
          </p>
          <CodeBlock lang="swift">{`try rift.clearUserId()`}</CodeBlock>
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
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">RiftSdk.create(publishableKey:)</td>
                    <td className="px-4 py-2.5">Convenience. Auto-wires Keychain storage + app version. Recommended.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">RiftSdk(config:, storage:)</td>
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
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">setUserId(userId:)</td>
                    <td className="px-4 py-2.5 font-mono">Void (async throws)</td>
                    <td className="px-4 py-2.5">Bind the install to a user. Persists + syncs + retries on next launch.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">trackConversion(conversionType:, idempotencyKey:, metadata:?)</td>
                    <td className="px-4 py-2.5 font-mono">Void (async throws)</td>
                    <td className="px-4 py-2.5">Fire a conversion event. POSTs to the Rift API via publishable key.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">checkDeferredDeepLink(clipboardText:)</td>
                    <td className="px-4 py-2.5 font-mono">DeferredDeepLinkResult? (async throws)</td>
                    <td className="px-4 py-2.5">One-call deferred deep link: parse, attribute, fetch link data.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">clearUserId()</td>
                    <td className="px-4 py-2.5 font-mono">Void (throws)</td>
                    <td className="px-4 py-2.5">Remove stored user binding. Call on logout.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">installId()</td>
                    <td className="px-4 py-2.5 font-mono">String (throws)</td>
                    <td className="px-4 py-2.5">Persistent install UUID. Generates on first call.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">reportAttributionForLink(linkId:)</td>
                    <td className="px-4 py-2.5 font-mono">Bool (async throws)</td>
                    <td className="px-4 py-2.5">Simplified attribution — uses internal install_id + app version.</td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">click(linkId:)</td>
                    <td className="px-4 py-2.5 font-mono">ClickResult (async throws)</td>
                    <td className="px-4 py-2.5">Record a click and return link data.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">getLink(linkId:)</td>
                    <td className="px-4 py-2.5 font-mono">GetLinkResult (async throws)</td>
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
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">parseClipboardLink(text:)</td>
                    <td className="px-4 py-2.5">Low-level: extracts link ID from a URL. Used internally by <code>checkDeferredDeepLink</code>.</td>
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
