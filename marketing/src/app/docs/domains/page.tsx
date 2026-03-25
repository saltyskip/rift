import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

export const metadata: Metadata = {
  title: "Custom Domains — Rift Docs",
  description: "Use your own brand domain for deep links with Rift custom domains.",
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
        {/* Overview */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">How it works</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Custom domains use a <strong className="text-[#fafafa]">Cloudflare Worker</strong> to
            proxy requests from your subdomain to the Rift API. The worker adds
            an <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">X-Rift-Host</code> header
            so Rift knows which domain the request came from.
          </p>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            This requires your domain&apos;s DNS to be managed by Cloudflare. You
            do <strong className="text-[#fafafa]">not</strong> need to transfer your domain —
            just point your registrar&apos;s nameservers to Cloudflare.
          </p>
          <Callout type="info">
            A verified custom domain is <strong>required</strong> to use custom IDs (vanity slugs)
            when <a href="/docs/links" className="underline">creating links</a>. Custom IDs are
            unique per tenant — different tenants can use the same slug on their own domains.
            Without a custom domain, links use auto-generated IDs on the
            primary <code>riftl.ink</code> domain.
          </Callout>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <Step n={1} title="Add your domain to Cloudflare">
            <p>
              In the <a href="https://dash.cloudflare.com" target="_blank" rel="noopener noreferrer" className="text-[#2dd4bf] hover:underline">Cloudflare dashboard</a>,
              click <strong className="text-[#fafafa]">Add a Site</strong> and enter your root domain
              (e.g. <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">yourcompany.com</code>).
              Select the <strong className="text-[#fafafa]">Free</strong> plan.
            </p>
            <p>
              Cloudflare will give you two nameservers. Go to your registrar (GoDaddy, Namecheap, etc.)
              and change the nameservers to Cloudflare&apos;s. For example:
            </p>
            <div className="overflow-x-auto">
              <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Nameserver</th>
                  </tr>
                </thead>
                <tbody className="text-[#a1a1aa]">
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono">ada.ns.cloudflare.com</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono">bob.ns.cloudflare.com</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <Callout type="info">
              This is <strong>not</strong> a domain transfer. Your registrar still owns the domain.
              You&apos;re just telling it to let Cloudflare handle DNS. Cloudflare will auto-import
              your existing DNS records.
            </Callout>
            <p>Wait for the zone to become active in Cloudflare (usually 5–30 minutes).</p>
          </Step>

          <Step n={2} title="Register your domain with Rift">
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
            <p>Save the <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">verification_token</code> — you&apos;ll need it in the next step.</p>
          </Step>

          <Step n={3} title="Add DNS records in Cloudflare">
            <p>In Cloudflare → <strong className="text-[#fafafa]">DNS</strong> → <strong className="text-[#fafafa]">Records</strong>, add a TXT record for domain verification:</p>
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
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#f59e0b]">TXT</td>
                    <td className="px-4 py-2.5 font-mono">_rift-verify.go</td>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">your verification_token</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <Callout type="warning">
              Enter just <code>_rift-verify.go</code> as the name — Cloudflare automatically
              appends your root domain. Entering the full hostname would
              create <code>_rift-verify.go.yourcompany.com.yourcompany.com</code>.
            </Callout>
          </Step>

          <Step n={4} title="Create the Cloudflare Worker">
            <p>
              In Cloudflare → <strong className="text-[#fafafa]">Workers &amp; Pages</strong> → <strong className="text-[#fafafa]">Create</strong>:
            </p>
            <ol className="list-decimal pl-5 space-y-1">
              <li>Name it (e.g. <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">rift-proxy</code>)</li>
              <li>Click <strong className="text-[#fafafa]">Deploy</strong> (deploys the default template)</li>
              <li>Click <strong className="text-[#fafafa]">Edit Code</strong> and replace everything with:</li>
            </ol>
            <CodeBlock lang="javascript">{`export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const host = url.hostname;
    const origin = "https://api.riftl.ink";
    const upstream = new URL(url.pathname + url.search, origin);
    const headers = new Headers(request.headers);
    headers.set("X-Rift-Host", host);
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
            <ol className="list-decimal pl-5 space-y-1" start={4}>
              <li>Click <strong className="text-[#fafafa]">Deploy</strong></li>
            </ol>
          </Step>

          <Step n={5} title="Attach the worker to your subdomain">
            <p>
              On the worker page → <strong className="text-[#fafafa]">Settings</strong> → <strong className="text-[#fafafa]">Domains &amp; Routes</strong> → <strong className="text-[#fafafa]">Add</strong> → <strong className="text-[#fafafa]">Custom Domain</strong>:
            </p>
            <p>
              Enter <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">go.yourcompany.com</code>.
              Cloudflare will auto-create a proxied DNS record for you.
            </p>
            <p>
              Then add a <strong className="text-[#fafafa]">Route</strong> as well:
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
                    <td className="px-4 py-2.5 font-mono">rift-proxy</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <Callout type="warning">
              The route pattern must include <code>/*</code> at the end. Without the wildcard,
              only the bare domain will match — paths like <code>/download</code> won&apos;t be proxied.
            </Callout>
          </Step>

          <Step n={6} title="Verify ownership">
            <p>Wait a few minutes for DNS to propagate, then verify:</p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains/go.yourcompany.com/verify \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <p>
              If verification fails, check that the TXT record has propagated:
            </p>
            <CodeBlock>{`dig +short TXT _rift-verify.go.yourcompany.com`}</CodeBlock>
            <p>
              The output should show your verification token. If it&apos;s empty, wait a few more
              minutes — DNS propagation can take up to 30 minutes.
            </p>
          </Step>

          <Step n={7} title="Test it">
            <CodeBlock>{`# Should return link data as JSON
curl https://go.yourcompany.com/YOUR_LINK_ID \\
  -H "Accept: application/json"

# Should serve the AASA file (if you registered an iOS app)
curl https://go.yourcompany.com/.well-known/apple-app-site-association`}</CodeBlock>
            <Callout type="info">
              If you get an SSL error, your local DNS cache may still point to
              the old IP. Flush it
              with <code>sudo dscacheutil -flushcache &amp;&amp; sudo killall -HUP mDNSResponder</code> on
              macOS, or wait a few minutes.
            </Callout>
          </Step>
        </section>
      </div>
    </div>
  );
}
