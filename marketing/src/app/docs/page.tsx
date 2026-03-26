import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

export const metadata: Metadata = {
  title: "Quick Start — Rift Docs",
  description: "Get your API key and start creating deep links in under 10 minutes.",
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

function Callout({ type, children }: { type: "info" | "warning"; children: React.ReactNode }) {
  const styles = {
    info: "border-[#60a5fa]/30 bg-[#60a5fa]/5 text-[#93bbfd]",
    warning: "border-[#f59e0b]/30 bg-[#f59e0b]/5 text-[#fbbf24]",
  };
  const labels = { info: "Note", warning: "Important" };
  return (
    <div className={`rounded-lg border px-4 py-3 text-[13px] leading-relaxed ${styles[type]}`}>
      <strong>{labels[type]}:</strong> {children}
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
          Go from zero to cross-platform deep links in under 10 minutes.
        </p>
      </div>

      <div className="space-y-10">
        {/* Section 1: Setup */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">1. Set up your account</h2>

          <Step n={1} title="Sign up">
            <p>
              Send a POST request with your email to get an API key.
              You&apos;ll receive a verification email.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/signup \\
  -H "Content-Type: application/json" \\
  -d '{"email": "you@example.com"}'`}</CodeBlock>
            <p>
              The full key is in the verification email (starts with <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rl_live_</code>).
              Save it — it won&apos;t be shown again.
            </p>
          </Step>

          <Step n={2} title="Verify your email">
            <p>Click the verification link in your inbox. Your key is now active.</p>
          </Step>

          <Step n={3} title="Register your app">
            <p>
              Register your iOS or Android app so Rift can serve the association files
              that make Universal Links and App Links work.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/apps \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"platform": "ios", "bundle_id": "com.example.myapp", "team_id": "ABCDE12345"}'`}</CodeBlock>
            <p>
              See <a href="/docs/apps" className="text-[#2dd4bf] hover:underline">Register Your App</a> for
              Android setup and optional fields like app name and icon.
            </p>
          </Step>

          <Step n={4} title="Add a custom domain">
            <p>
              Deep links resolve through your own domain (e.g. <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">go.yourcompany.com</code>).
              Rift automatically serves the AASA and assetlinks.json files on this domain.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
            <p>
              Point a CNAME record to the address shown in the response, then{" "}
              <a href="/docs/domains" className="text-[#2dd4bf] hover:underline">verify the domain</a>.
            </p>
          </Step>

          <Step n={5} title="Create a publishable key">
            <p>
              Publishable keys (<code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">pk_live_</code>) are
              client-safe keys for the web and mobile SDKs. They&apos;re scoped to a verified domain.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/publishable-keys \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
            <p>
              Save the returned <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">pk_live_</code> key — it won&apos;t be shown again.
            </p>
          </Step>
        </section>

        <div className="gradient-line" />

        {/* Section 2: Create a link */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">2. Create a link</h2>

          <Step n={6} title="Create a deep link with all destinations">
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/links \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "custom_id": "get-app",
    "web_url": "https://yourcompany.com",
    "ios_store_url": "https://apps.apple.com/app/id123456789",
    "android_store_url": "https://play.google.com/store/apps/details?id=com.example.myapp"
  }'`}</CodeBlock>
            <p>
              Your link is now live at{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">
                https://go.yourcompany.com/get-app
              </code>
            </p>
          </Step>
        </section>

        <div className="gradient-line" />

        {/* Section 3: Add to your website */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">3. Add the download button</h2>

          <Step n={7} title="Load rift.js and add a link">
            <p>
              The download button is a plain <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">&lt;a&gt;</code> tag
              pointing to your deep link domain. Universal Links open the app directly when it&apos;s installed.{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">Rift.click()</code> records
              the click via sendBeacon without blocking navigation.
            </p>
            <CodeBlock lang="html">{`<script src="https://api.riftl.ink/sdk/rift.js"></script>
<script>Rift.init("pk_live_YOUR_KEY");</script>

<a href="https://go.yourcompany.com/get-app?redirect=1"
   onclick="Rift.click('get-app', { domain: 'go.yourcompany.com' })">
  Download the App
</a>`}</CodeBlock>
            <Callout type="info">
              The <code>?redirect=1</code> parameter tells the landing page to skip its UI and redirect
              straight to the App Store or Play Store. Without it, users see a full branded landing page —
              useful when the link is shared externally (email, social media, etc.).
            </Callout>
          </Step>
        </section>

        <div className="gradient-line" />

        {/* Section 4: What happens */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">4. What happens when a user clicks</h2>

          <div className="space-y-4 text-[15px] text-[#a1a1aa]">
            <p><strong className="text-[#fafafa]">On your website</strong> (with <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">?redirect=1</code>):</p>
            <ul className="list-disc pl-5 space-y-1.5">
              <li><strong className="text-[#fafafa]">App installed</strong> — iOS/Android intercepts the tap via Universal Links. The app opens directly. Click tracked via sendBeacon.</li>
              <li><strong className="text-[#fafafa]">App not installed</strong> — Redirects straight to the App Store or Play Store. Click tracked server-side.</li>
              <li><strong className="text-[#fafafa]">Desktop</strong> — Redirects to your <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">web_url</code>.</li>
            </ul>

            <p className="pt-2"><strong className="text-[#fafafa]">Shared externally</strong> (email, social, Messages — no <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">?redirect=1</code>):</p>
            <ul className="list-disc pl-5 space-y-1.5">
              <li><strong className="text-[#fafafa]">App installed</strong> — Universal Links open the app directly.</li>
              <li><strong className="text-[#fafafa]">App not installed</strong> — A branded landing page with a download button, your app description, and machine-readable context for AI agents.</li>
            </ul>
          </div>
        </section>

        <div className="gradient-line" />

        {/* Next steps */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Next steps</h2>
          <div className="grid gap-3">
            {[
              { title: "Create Links", desc: "Per-platform destinations, metadata, and agent context.", href: "/docs/links" },
              { title: "Web SDK", desc: "Framework guides for Next.js, Svelte, Vue, and plain HTML.", href: "/docs/web-sdk" },
              { title: "iOS SDK", desc: "Native Swift SDK for click tracking and attribution.", href: "/docs/ios-sdk" },
              { title: "Android SDK", desc: "Native Kotlin SDK for click tracking and attribution.", href: "/docs/android-sdk" },
              { title: "Attribution", desc: "Track installs and report conversions back to your links.", href: "/docs/attribution" },
              { title: "Webhooks", desc: "Real-time Slack or HTTP notifications for clicks and attributions.", href: "/docs/webhooks" },
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
