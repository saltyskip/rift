import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

export const metadata: Metadata = {
  title: "Attribution — Rift Docs",
  description: "Track installs, attribute them to links, and view conversion analytics.",
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

export default function AttributionPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Tracking</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Attribution</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Track installs, attribute them to links, and measure conversion rates.
          Both attribution endpoints require a{" "}
          <a href="/docs/publishable-keys" className="text-[#2dd4bf] hover:underline">publishable key</a>{" "}
          (<code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">pk_live_</code>).
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Attribution flow</h2>

          <Step n={1} title="Record a click">
            <p>
              When a user interacts with a Rift link (via the web SDK, mobile SDK, or your own integration),
              record the click:
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/attribution/click \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"link_id": "summer-sale"}'`}</CodeBlock>
            <p>
              The response includes the full link data (deep links, store URLs, metadata) so you can
              navigate the user to the right destination.
            </p>
          </Step>

          <Step n={2} title="Report an install">
            <p>After the app is installed and opened, report the attribution:</p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/attribution/report \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "link_id": "summer-sale",
    "install_id": "device-uuid-here",
    "app_version": "1.0.0"
  }'`}</CodeBlock>
          </Step>

          <Step n={3} title="Link attribution to a user">
            <p>After the user signs up or logs in, connect the attribution to their account:</p>
            <CodeBlock>{`curl -X PUT https://api.riftl.ink/v1/attribution/link \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "install_id": "device-uuid-here",
    "user_id": "user-123"
  }'`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Analytics</h2>

          <Step n={4} title="View link stats">
            <CodeBlock>{`curl https://api.riftl.ink/v1/links/summer-sale/stats \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <p>Response:</p>
            <CodeBlock lang="json">{`{
  "link_id": "summer-sale",
  "click_count": 1234,
  "install_count": 89,
  "conversion_rate": 0.072
}`}</CodeBlock>
          </Step>
        </section>
      </div>
    </div>
  );
}
