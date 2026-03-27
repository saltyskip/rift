import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

export const metadata: Metadata = {
  title: "Quick Start — Rift Docs",
  description: "Set up cross-platform deep links with Universal Links, click tracking, and AI-readable context in under 10 minutes.",
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
          One link that opens your app on iOS, routes to the Play Store on Android, and shows a branded
          landing page on desktop — with click tracking, attribution, and machine-readable context for AI agents.
        </p>
      </div>

      <div className="space-y-10">

        {/* ── SET UP ── */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Set up your account</h2>
          <p className="text-[15px] text-[#a1a1aa]">
            Five steps to go from nothing to a working deep link. Each one builds on the last.
          </p>

          <Step n={1} title="Get your API key">
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/signup \\
  -H "Content-Type: application/json" \\
  -d '{"email": "you@example.com"}'`}</CodeBlock>
            <p>
              Check your inbox for the verification email. It contains your full API key
              (starts with <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rl_live_</code>).
              Save it — it won&apos;t be shown again. Click the verification link to activate it.
            </p>
          </Step>

          <Step n={2} title="Register your app">
            <p>
              Tell Rift about your iOS or Android app. This lets Rift serve the association files
              (AASA / assetlinks.json) that make Universal Links and App Links work — so tapping a link
              opens your app directly instead of a webpage.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/apps \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "platform": "ios",
    "bundle_id": "com.example.myapp",
    "team_id": "ABCDE12345"
  }'`}</CodeBlock>
          </Step>

          <Step n={3} title="Add your domain">
            <p>
              Deep links live on your own domain — something
              like <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">go.yourcompany.com</code>.
              Rift serves the AASA and assetlinks.json files here automatically.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
            <p>
              Add the CNAME record shown in the response, then verify:
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains/go.yourcompany.com/verify \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
          </Step>

          <Step n={4} title="Create a publishable key">
            <p>
              Publishable keys (<code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">pk_live_</code>)
              are client-safe — they go in your website and mobile app. They can only record clicks and attributions,
              never manage links.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/publishable-keys \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
            <p>
              Save the returned <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">pk_live_</code> key.
              It won&apos;t be shown again.
            </p>
          </Step>

          <Step n={5} title="Create your first link">
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
              Your link is live
              at <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">https://go.yourcompany.com/get-app</code>.
            </p>
          </Step>
        </section>

        <div className="gradient-line" />

        {/* ── ADD TO YOUR SITE ── */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Add the download button</h2>
          <p className="text-[15px] text-[#a1a1aa]">
            The download button is a plain <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">&lt;a&gt;</code> tag.
            Universal Links handle the heavy lifting — if the app is installed, iOS opens it directly from
            the tap. No JavaScript redirect, no custom URL scheme, no timeout hack.
          </p>

          <CodeBlock lang="html">{`<script src="https://api.riftl.ink/sdk/rift.js"></script>
<script>Rift.init("pk_live_YOUR_KEY");</script>

<a href="https://go.yourcompany.com/get-app?redirect=1"
   onclick="Rift.click('get-app', { domain: 'go.yourcompany.com' })">
  Download the App
</a>`}</CodeBlock>

          <Callout type="info">
            <code>Rift.click()</code> fires a beacon to record the click — it doesn&apos;t block navigation
            or interfere with Universal Links. The <code>?redirect=1</code> parameter tells the landing page to skip
            its UI and go straight to the store when the app isn&apos;t installed.
          </Callout>
        </section>

        <div className="gradient-line" />

        {/* ── WHAT HAPPENS ── */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">What happens when someone clicks</h2>

          <div className="space-y-5 text-[15px] text-[#a1a1aa]">
            <div>
              <p className="font-medium text-[#fafafa] mb-2">From your website</p>
              <p>
                The user taps your download button. <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">Rift.click()</code> records
                the click and copies the link URL to the clipboard (for deferred deep linking after install).
                Then the browser follows the link:
              </p>
              <ul className="list-disc pl-5 space-y-1.5 mt-2">
                <li><strong className="text-[#fafafa]">App installed</strong> — Universal Links intercept the tap. The app opens directly. No webpage loads.</li>
                <li><strong className="text-[#fafafa]">App not installed</strong> — The <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">?redirect=1</code> page
                  redirects straight to the App Store or Play Store.</li>
                <li><strong className="text-[#fafafa]">Desktop</strong> — Redirects to your <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">web_url</code>.</li>
              </ul>
            </div>

            <div>
              <p className="font-medium text-[#fafafa] mb-2">From email, social, or a text message</p>
              <p>
                When the link is shared outside your site, there&apos;s
                no <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">?redirect=1</code> and
                no rift.js. The behavior changes:
              </p>
              <ul className="list-disc pl-5 space-y-1.5 mt-2">
                <li><strong className="text-[#fafafa]">App installed</strong> — Universal Links still open the app directly.</li>
                <li><strong className="text-[#fafafa]">App not installed</strong> — A branded landing page loads with your app name,
                  description, and a download button. The page also includes structured data for AI agents.</li>
              </ul>
            </div>
          </div>
        </section>

        <div className="gradient-line" />

        {/* ── FOR AI AGENTS ── */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">For AI agents</h2>
          <p className="text-[15px] text-[#a1a1aa]">
            Every Rift link is machine-readable. When an agent resolves your link
            with <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">Accept: application/json</code>,
            it gets structured context about what the link does and who created it:
          </p>

          <CodeBlock lang="json">{`{
  "link_id": "get-app",
  "web_url": "https://yourcompany.com",
  "ios_store_url": "https://apps.apple.com/app/id123456789",
  "agent_context": {
    "action": "download",
    "cta": "Get the App",
    "description": "Your app description here"
  },
  "_rift_meta": {
    "source": "tenant_asserted",
    "status": "active",
    "tenant_domain": "go.yourcompany.com",
    "tenant_verified": true
  }
}`}</CodeBlock>
          <p className="text-[15px] text-[#a1a1aa]">
            The landing page includes this same data as JSON-LD for crawlers that parse HTML.
            Add <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">agent_context</code> when{" "}
            <a href="/docs/links" className="text-[#2dd4bf] hover:underline">creating your link</a> to
            control how AI agents describe it.
          </p>
        </section>

        <div className="gradient-line" />

        {/* ── NEXT STEPS ── */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Keep going</h2>
          <div className="grid gap-3">
            {[
              { title: "Create Links", desc: "Per-platform destinations, vanity slugs, metadata, and agent context.", href: "/docs/links" },
              { title: "Web SDK", desc: "Framework-specific guides for Next.js, Svelte, Vue, and plain HTML.", href: "/docs/web-sdk" },
              { title: "iOS SDK", desc: "Native Swift SDK for click tracking and post-install attribution.", href: "/docs/ios-sdk" },
              { title: "Android SDK", desc: "Native Kotlin SDK for click tracking and post-install attribution.", href: "/docs/android-sdk" },
              { title: "Attribution", desc: "Track installs and close the loop on which links drive conversions.", href: "/docs/attribution" },
              { title: "Webhooks", desc: "Real-time notifications for clicks and attributions — Slack, HTTP, or anything.", href: "/docs/webhooks" },
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
