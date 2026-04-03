import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

export const metadata: Metadata = {
  title: "Quick Start — Rift Docs",
  description: "Get your API key and start creating deep links in under 5 minutes.",
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

export default function QuickStartPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Documentation</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Quick Start</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Go from zero to mobile deep links in under 10 minutes.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">1. Get your API key</h2>

          <Step n={1} title="Sign up">
            <p>
              Send a POST request with your email. You&apos;ll receive a verification email.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/signup \\
  -H "Content-Type: application/json" \\
  -d '{"email": "you@example.com"}'`}</CodeBlock>
          </Step>

          <Step n={2} title="Verify your email and get your secret key">
            <p>
              Click the verification link in your inbox. The response will contain your secret API key
              (starts with <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rl_live_</code>).
              Save it immediately — it&apos;s shown only once and never sent via email.
              This is your server-side key for managing links, domains, and keys. Never expose it in client code.
            </p>
          </Step>

          <Step n={3} title="Create a publishable key">
            <p>
              Publishable keys (<code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">pk_live_</code>) are client-safe keys
              used by the web and mobile SDKs for click tracking and attribution. Create one after setting up a{" "}
              <a href="/docs/domains" className="text-[#2dd4bf] hover:underline">custom domain</a>:
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/publishable-keys \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
            <p>
              Save the returned key — it won&apos;t be shown again.
              See <a href="/docs/publishable-keys" className="text-[#2dd4bf] hover:underline">Publishable Keys</a> for details.
            </p>
          </Step>

          <Step n={4} title="Add click tracking to your website">
            <p>
              Install the SDK via npm or load it via script tag. Pass your publishable key with your custom domain.
              The SDK auto-tracks clicks on any link pointing to your domain — no attributes or event handlers needed.
            </p>
            <CodeBlock lang="bash">{`npm install @riftlinks/sdk`}</CodeBlock>
            <CodeBlock lang="typescript">{`import { Rift } from '@riftlinks/sdk';
Rift.init("pk_live_YOUR_KEY", { domain: "go.yourcompany.com" });`}</CodeBlock>
            <p className="text-[13px] text-[#52525b]">
              Or via script tag:
            </p>
            <CodeBlock lang="html">{`<script src="https://api.riftl.ink/sdk/rift.js"></script>
<script>Rift.init("pk_live_YOUR_KEY", { domain: "go.yourcompany.com" });</script>`}</CodeBlock>
            <p>
              All links to your domain are auto-tracked. See <a href="/docs/web-sdk" className="text-[#2dd4bf] hover:underline">Web SDK</a> for
              framework-specific examples (Next.js, Svelte, Vue) and the full API reference.
            </p>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Next steps</h2>
          <div className="grid gap-3">
            {[
              { title: "Register Your App", desc: "Configure iOS & Android app details for branding and association files.", href: "/docs/apps" },
              { title: "Custom Domains", desc: "Required for vanity slugs. Use your own brand: go.yourcompany.com/summer-sale.", href: "/docs/domains" },
              { title: "Publishable Keys", desc: "Client-safe keys for SDK click tracking and attribution.", href: "/docs/publishable-keys" },
              { title: "Create Links", desc: "Create deep links with per-platform destinations and metadata.", href: "/docs/links" },
              { title: "Web SDK", desc: "Add download buttons to your website with rift.js.", href: "/docs/web-sdk" },
              { title: "iOS SDK", desc: "Native Swift SDK for click tracking and attribution.", href: "/docs/ios-sdk" },
              { title: "Android SDK", desc: "Native Kotlin SDK for click tracking and attribution.", href: "/docs/android-sdk" },
              { title: "Webhooks", desc: "Real-time notifications for click and attribution events.", href: "/docs/webhooks" },
            ].map((item) => (
              <a
                key={item.title}
                href={item.href}
                className="group flex items-center justify-between p-4 rounded-xl bg-[#111113] border border-[#1e1e22] hover:border-[#2dd4bf]/30 transition-colors"
              >
                <div>
                  <p className="text-[15px] font-medium text-[#fafafa] group-hover:text-[#2dd4bf] transition-colors">{item.title}</p>
                  <p className="text-[13px] text-[#52525b]">{item.desc}</p>
                </div>
                <span className="text-[#3f3f46] group-hover:text-[#2dd4bf] transition-colors">&rarr;</span>
              </a>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
