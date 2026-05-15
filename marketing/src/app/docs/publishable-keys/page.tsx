import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "Riftl.ink Publishable Keys — Rift Docs",
  description: "Client-safe Riftl.ink keys for SDK click tracking and attribution endpoints.",
  alternates: { canonical: "/docs/publishable-keys" },
};

export default function PublishableKeysPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Getting Started</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Riftl.ink Publishable Keys</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Client-safe keys for SDK click tracking and attribution. Publishable keys
          (prefix <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">pk_live_</code>)
          are scoped to a verified domain and can only access attribution endpoints &mdash;
          safe to embed in web pages and mobile apps.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">What are publishable keys?</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Your API key (<code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rl_live_</code>)
            is a secret key for server-side use. It can create, update, and delete links. You should never expose it
            in client code.
          </p>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Publishable keys (<code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">pk_live_</code>)
            are designed for client-side use. They can only call the attribution endpoints:
          </p>
          <ul className="list-disc pl-5 space-y-1 text-[15px] text-[#a1a1aa]">
            <li><code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">POST /v1/lifecycle/click</code> &mdash; record a click</li>
            <li><code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">POST /v1/lifecycle/attribute</code> &mdash; report an install attribution</li>
            <li><code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">PUT /v1/lifecycle/identify</code> &mdash; bind a user to an install</li>
            <li><code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">POST /v1/lifecycle/convert</code> &mdash; track a conversion event</li>
          </ul>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Each publishable key is bound to a verified custom domain. This scopes all lookups to your tenant.
          </p>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Prerequisites</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Before creating a publishable key, you need:
          </p>
          <ol className="list-decimal pl-5 space-y-1 text-[15px] text-[#a1a1aa]">
            <li>An API key (<a href="/docs" className="text-[#2dd4bf] hover:underline">Quick Start</a>)</li>
            <li>A <a href="/docs/domains" className="text-[#2dd4bf] hover:underline">verified custom domain</a></li>
          </ol>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Manage publishable keys</h2>

          <Step n={1} title="Create a publishable key">
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/publishable-keys \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
            <p>Response:</p>
            <CodeBlock lang="json">{`{
  "id": "6612...",
  "key": "pk_live_abc123...",
  "domain": "go.yourcompany.com",
  "created_at": "2026-03-26T12:00:00Z"
}`}</CodeBlock>
            <Callout type="warning">
              The full key is only shown once at creation time. Save it immediately.
            </Callout>
          </Step>

          <Step n={2} title="List publishable keys">
            <CodeBlock>{`curl https://api.riftl.ink/v1/auth/publishable-keys \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <p>Response:</p>
            <CodeBlock lang="json">{`{
  "keys": [
    {
      "id": "6612...",
      "key_prefix": "pk_live_abc...",
      "domain": "go.yourcompany.com",
      "created_at": "2026-03-26T12:00:00Z"
    }
  ]
}`}</CodeBlock>
          </Step>

          <Step n={3} title="Revoke a publishable key">
            <CodeBlock>{`curl -X DELETE https://api.riftl.ink/v1/auth/publishable-keys/6612... \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <p>Returns <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">204 No Content</code> on success.</p>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Where to use</h2>
          <div className="space-y-3 text-[15px] text-[#a1a1aa] leading-relaxed">
            <p>Pass your publishable key when initializing the SDKs:</p>
            <ul className="list-disc pl-5 space-y-1">
              <li>
                <strong className="text-[#fafafa]">Web SDK:</strong>{" "}
                <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">Rift.init(&quot;pk_live_YOUR_KEY&quot;)</code>{" "}
                &mdash; see <a href="/docs/web-sdk" className="text-[#2dd4bf] hover:underline">Web SDK docs</a>
              </li>
              <li>
                <strong className="text-[#fafafa]">iOS SDK:</strong>{" "}
                <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">RiftSdk.create(publishableKey: &quot;pk_live_YOUR_KEY&quot;)</code>{" "}
                &mdash; see <a href="/docs/ios-sdk" className="text-[#2dd4bf] hover:underline">iOS SDK docs</a>
              </li>
              <li>
                <strong className="text-[#fafafa]">Android SDK:</strong>{" "}
                <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">RiftSdk.create(&quot;pk_live_YOUR_KEY&quot;, context)</code>{" "}
                &mdash; see <a href="/docs/android-sdk" className="text-[#2dd4bf] hover:underline">Android SDK docs</a>
              </li>
            </ul>
          </div>
          <Callout type="info">
            Publishable keys are safe to embed in client-side code. They can only access the attribution
            endpoints and are scoped to your verified domain.
          </Callout>
        </section>
      </div>
    </div>
  );
}
