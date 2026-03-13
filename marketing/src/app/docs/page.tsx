import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Docs — Rift",
  description: "Get started with Rift deep links and custom domains.",
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

function SectionHeading({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h2 id={id} className="text-2xl font-bold text-[#fafafa] scroll-mt-24">
      {children}
    </h2>
  );
}

export default function DocsPage() {
  return (
    <div className="min-h-screen pt-24 pb-20">
      <div className="mx-auto max-w-3xl px-6">
        {/* Header */}
        <div className="mb-16">
          <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Documentation</p>
          <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Quick Setup</h1>
          <p className="text-lg text-[#71717a] leading-relaxed">
            Go from zero to branded short links in under 10 minutes.
            This guide walks you through getting an API key, creating links,
            and setting up a custom domain.
          </p>
        </div>

        {/* Table of contents */}
        <div className="mb-16 p-5 rounded-xl bg-[#111113] border border-[#1e1e22]">
          <p className="text-[11px] font-medium text-[#52525b] uppercase tracking-widest mb-3">On this page</p>
          <nav className="space-y-1.5">
            {[
              ["#get-api-key", "1. Get your API key"],
              ["#create-link", "2. Create your first link"],
              ["#resolve-link", "3. Resolve a link"],
              ["#custom-domain", "4. Set up a custom domain"],
              ["#verify-domain", "5. Verify and go live"],
            ].map(([href, label]) => (
              <a
                key={href}
                href={href}
                className="block text-[14px] text-[#71717a] hover:text-[#2dd4bf] transition-colors"
              >
                {label}
              </a>
            ))}
          </nav>
        </div>

        <div className="space-y-16">
          {/* ── 1. Get API key ── */}
          <section className="space-y-6">
            <SectionHeading id="get-api-key">1. Get your API key</SectionHeading>
            <Step n={1} title="Sign up">
              <p>
                Send a POST request with your email to get an API key.
                You&apos;ll receive a verification email.
              </p>
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/signup \\
  -H "Content-Type: application/json" \\
  -d '{"email": "you@example.com"}'`}</CodeBlock>
              <p>
                The response contains your API key (starts with <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rl_live_</code>).
                Save it — it won&apos;t be shown again.
              </p>
            </Step>

            <Step n={2} title="Verify your email">
              <p>
                Click the verification link in your inbox. Your key is now active.
              </p>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 2. Create a link ── */}
          <section className="space-y-6">
            <SectionHeading id="create-link">2. Create your first link</SectionHeading>
            <Step n={3} title="Create a deep link">
              <p>
                Use your API key to create a link. You can optionally set a custom slug
                and attach metadata.
              </p>
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/links \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "custom_id": "book-downtown",
    "destination": "https://yourapp.com/book?location=downtown",
    "metadata": {
      "campaign": "spring-launch",
      "source": "docs"
    }
  }'`}</CodeBlock>
              <p>Response:</p>
              <CodeBlock>{`{
  "link_id": "book-downtown",
  "url": "https://api.riftl.ink/r/book-downtown"
}`}</CodeBlock>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 3. Resolve ── */}
          <section className="space-y-6">
            <SectionHeading id="resolve-link">3. Resolve a link</SectionHeading>
            <Step n={4} title="As a human (browser)">
              <p>
                Visiting the link in a browser redirects to the destination URL:
              </p>
              <CodeBlock>{`curl -v https://api.riftl.ink/r/book-downtown
# → 302 redirect to https://yourapp.com/book?location=downtown`}</CodeBlock>
            </Step>

            <Step n={5} title="As an agent (JSON)">
              <p>
                Agents that send <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">Accept: application/json</code> get
                structured metadata instead of a redirect:
              </p>
              <CodeBlock>{`curl https://api.riftl.ink/r/book-downtown \\
  -H "Accept: application/json"

{
  "link_id": "book-downtown",
  "destination": "https://yourapp.com/book?location=downtown",
  "metadata": {
    "campaign": "spring-launch",
    "source": "docs"
  }
}`}</CodeBlock>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 4. Custom domain ── */}
          <section className="space-y-6">
            <SectionHeading id="custom-domain">4. Set up a custom domain</SectionHeading>
            <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
              Instead of <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">riftl.ink/r/book-downtown</code>,
              your links can use your own brand: <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">go.yourcompany.com/book-downtown</code>.
            </p>

            <Step n={6} title="Register your domain">
              <p>Tell Rift which domain you want to use:</p>
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
              <p>Response:</p>
              <CodeBlock>{`{
  "domain": "go.yourcompany.com",
  "verified": false,
  "verification_token": "a1b2c3d4e5f6...",
  "txt_record": "_rift-verify.go.yourcompany.com",
  "cname_target": "riftl.ink"
}`}</CodeBlock>
            </Step>

            <Step n={7} title="Add DNS records">
              <p>
                In your DNS provider (Cloudflare, Route 53, Namecheap, etc.), create two records:
              </p>
              <div className="overflow-x-auto">
                <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                  <thead>
                    <tr className="bg-[#0c0c0e]">
                      <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Type</th>
                      <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Name</th>
                      <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Value</th>
                    </tr>
                  </thead>
                  <tbody className="text-[#a1a1aa]">
                    <tr className="border-b border-[#1e1e22]">
                      <td className="px-4 py-2.5 font-mono text-[#60a5fa]">CNAME</td>
                      <td className="px-4 py-2.5 font-mono">go</td>
                      <td className="px-4 py-2.5 font-mono">riftl.ink</td>
                    </tr>
                    <tr>
                      <td className="px-4 py-2.5 font-mono text-[#f59e0b]">TXT</td>
                      <td className="px-4 py-2.5 font-mono">_rift-verify.go</td>
                      <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">a1b2c3d4e5f6...</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <p>
                The CNAME routes traffic to Rift. The TXT record proves you own the domain.
                DNS propagation usually takes a few minutes.
              </p>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 5. Verify ── */}
          <section className="space-y-6">
            <SectionHeading id="verify-domain">5. Verify and go live</SectionHeading>

            <Step n={8} title="Verify ownership">
              <p>Once DNS has propagated, verify your domain:</p>
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains/go.yourcompany.com/verify \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
              <p>Response:</p>
              <CodeBlock>{`{
  "domain": "go.yourcompany.com",
  "verified": true
}`}</CodeBlock>
            </Step>

            <Step n={9} title="Use your branded links">
              <p>
                Your links now work on your custom domain. Any link you&apos;ve already created
                is automatically accessible:
              </p>
              <CodeBlock>{`# Browser → redirect
curl -v https://go.yourcompany.com/book-downtown
# → 302 redirect to https://yourapp.com/book?location=downtown

# Agent → JSON
curl https://go.yourcompany.com/book-downtown \\
  -H "Accept: application/json"
# → { "link_id": "book-downtown", "destination": "...", "metadata": {...} }`}</CodeBlock>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── Next steps ── */}
          <section className="space-y-4">
            <h2 className="text-2xl font-bold text-[#fafafa]">Next steps</h2>
            <div className="grid gap-3">
              {[
                {
                  title: "API Reference",
                  desc: "Full endpoint documentation with try-it-out.",
                  href: "/api-reference",
                },
                {
                  title: "Attribution tracking",
                  desc: "Track installs and conversions from your links.",
                  href: "/api-reference",
                },
                {
                  title: "Manage domains",
                  desc: "List, verify, or remove custom domains.",
                  href: "/api-reference",
                },
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
                  <span className="text-[#3f3f46] group-hover:text-[#2dd4bf] transition-colors">→</span>
                </a>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
