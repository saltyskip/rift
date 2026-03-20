import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Attribution — Rift Docs",
  description: "Track installs, attribute them to links, and view conversion analytics.",
};

function CodeBlock({ children }: { children: string }) {
  return (
    <pre className="bg-[#0c0c0e] border border-[#1e1e22] rounded-lg p-4 overflow-x-auto text-[13px] leading-relaxed font-mono text-[#a1a1aa]">
      <code>{children}</code>
    </pre>
  );
}

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
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <Step n={1} title="Report an install">
            <p>After the app is installed and opened, report the attribution:</p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/attribution \\
  -H "Content-Type: application/json" \\
  -d '{
    "link_id": "summer-sale",
    "install_id": "device-uuid-here",
    "app_version": "1.0.0"
  }'`}</CodeBlock>
          </Step>

          <Step n={2} title="Link attribution to a user">
            <p>After the user signs up or logs in, connect the attribution to their account:</p>
            <CodeBlock>{`curl -X PUT https://api.riftl.ink/v1/attribution/link \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"install_id": "device-uuid-here"}'`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Analytics</h2>

          <Step n={3} title="View link stats">
            <CodeBlock>{`curl https://api.riftl.ink/v1/links/summer-sale/stats \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <p>Response:</p>
            <CodeBlock>{`{
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
