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
          Native Swift SDK for click tracking and attribution.
          Built with Rust and compiled to a Swift Package via UniFFI.
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

          <Step n={2} title="Import and initialize">
            <p>
              You need a <a href="/docs/publishable-keys" className="text-[#2dd4bf] hover:underline">publishable key</a> to initialize the SDK.
            </p>
            <CodeBlock lang="swift">{`import RiftSDK

// Initialize once at app launch (e.g., in AppDelegate or @main).
let rift = RiftSdk(config: RiftConfig(publishableKey: "pk_live_YOUR_KEY", baseUrl: nil, logLevel: nil))

// Or point to a self-hosted instance:
let rift = RiftSdk(config: RiftConfig(publishableKey: "pk_live_YOUR_KEY", baseUrl: "https://api.yourcompany.com", logLevel: nil))`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Click Tracking</h2>

          <Step n={3} title="Record a click">
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

          <Step n={4} title="Check clipboard on app launch">
            <p>
              When a user clicks a Rift link on iOS, the landing page copies the full link URL to
              the clipboard. On first launch after install, read the clipboard, extract the link ID,
              fetch the link data, and report attribution:
            </p>
            <CodeBlock lang="swift">{`func handleDeferredDeepLink() async {
    guard let clipboard = UIPasteboard.general.string,
          let linkId = parseClipboardLink(text: clipboard) else {
        return
    }

    // Clear clipboard after reading.
    UIPasteboard.general.string = ""

    // Report attribution.
    do {
        let _ = try await rift.reportAttribution(
            linkId: linkId,
            installId: UIDevice.current.identifierForVendor?.uuidString ?? UUID().uuidString,
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
              The <code>parseClipboardLink(text:)</code> helper handles both full URLs
              (e.g. <code>https://go.yourcompany.com/summer-sale</code>) and the legacy <code>rift:&lt;link_id&gt;</code> format.
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
                    <td className="px-4 py-2.5 font-mono">Bool</td>
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
