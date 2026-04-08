import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";

export const metadata: Metadata = {
  title: "Universal Links — Rift Docs",
  description: "Configure iOS Associated Domains and Android App Links with Rift.",
};

export default function UniversalLinksPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Deep Linking</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Universal Links</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Once your app is registered and domain verified, Rift automatically serves
          the association files. You just need to configure your apps to use them.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Prerequisites</h2>
          <ul className="list-disc pl-5 space-y-1 text-[15px] text-[#a1a1aa]">
            <li><a href="/docs/apps" className="text-[#2dd4bf] hover:underline">Register your app</a> (iOS and/or Android)</li>
            <li><a href="/docs/domains" className="text-[#2dd4bf] hover:underline">Set up a custom domain</a> and verify it</li>
          </ul>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <Step n={1} title="iOS — Associated Domains">
            <p>
              In Xcode, go to <strong className="text-[#fafafa]">Signing &amp; Capabilities</strong> &rarr;{" "}
              <strong className="text-[#fafafa]">+ Capability</strong> &rarr;{" "}
              <strong className="text-[#fafafa]">Associated Domains</strong>, then add:
            </p>
            <CodeBlock>{`applinks:go.yourcompany.com`}</CodeBlock>
            <p>
              Rift serves the AASA file at{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">
                https://go.yourcompany.com/.well-known/apple-app-site-association
              </code>
            </p>
          </Step>

          <Step n={2} title="Android — Intent Filters">
            <p>
              Add an intent filter to your <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">AndroidManifest.xml</code>:
            </p>
            <CodeBlock lang="xml">{`<activity android:name=".MainActivity">
    <intent-filter android:autoVerify="true">
        <action android:name="android.intent.action.VIEW" />
        <category android:name="android.intent.category.DEFAULT" />
        <category android:name="android.intent.category.BROWSABLE" />
        <data android:scheme="https"
              android:host="go.yourcompany.com" />
    </intent-filter>
</activity>`}</CodeBlock>
            <p>
              Rift serves the assetlinks file at{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">
                https://go.yourcompany.com/.well-known/assetlinks.json
              </code>
            </p>
          </Step>
        </section>
      </div>
    </div>
  );
}
