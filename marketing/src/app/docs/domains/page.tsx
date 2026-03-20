import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

export const metadata: Metadata = {
  title: "Custom Domains — Rift Docs",
  description: "Use your own brand domain for deep links with Relay custom domains.",
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

export default function DomainsPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Getting Started</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Custom Domains</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Use your own brand for links: <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">go.yourcompany.com/summer-sale</code>{" "}
          instead of <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">riftl.ink/r/summer-sale</code>.
          Custom domains also enable universal links and app links.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <Step n={1} title="Register your domain">
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
            <p>Response:</p>
            <CodeBlock lang="json">{`{
  "domain": "go.yourcompany.com",
  "verified": false,
  "verification_token": "a1b2c3d4e5f6...",
  "txt_record": "_rift-verify.go.yourcompany.com",
  "cname_target": "riftl.ink"
}`}</CodeBlock>
          </Step>

          <Step n={2} title="Add DNS records">
            <p>In your DNS provider, create two records:</p>
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
          </Step>

          <Step n={3} title="Deploy the edge worker">
            <p>
              Relay needs a lightweight Cloudflare Worker on your domain to forward requests
              to the API. Create a new Worker in your Cloudflare dashboard with this code:
            </p>
            <CodeBlock lang="javascript">{`export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const host = url.hostname;
    const origin = "https://api.riftl.ink";
    const upstream = new URL(url.pathname + url.search, origin);
    const headers = new Headers(request.headers);
    headers.set("X-Relay-Host", host);
    const response = await fetch(upstream.toString(), {
      method: request.method,
      headers,
      body: request.method !== "GET" && request.method !== "HEAD"
        ? request.body : undefined,
      redirect: "manual",
    });
    return response;
  },
};`}</CodeBlock>
            <p>
              Then add a <strong className="text-[#fafafa]">Worker Route</strong> on your zone:
            </p>
            <div className="overflow-x-auto">
              <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Setting</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Value</th>
                  </tr>
                </thead>
                <tbody className="text-[#a1a1aa]">
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5">Route pattern</td>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">go.yourcompany.com/*</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5">Worker</td>
                    <td className="px-4 py-2.5 font-mono">your-relay-worker</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>
              Make sure the CNAME record for your subdomain is set to{" "}
              <strong className="text-[#fafafa]">Proxied</strong> (orange cloud) in Cloudflare.
            </p>
          </Step>

          <Step n={4} title="Verify ownership">
            <p>Once DNS has propagated:</p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains/go.yourcompany.com/verify \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
          </Step>
        </section>
      </div>
    </div>
  );
}
