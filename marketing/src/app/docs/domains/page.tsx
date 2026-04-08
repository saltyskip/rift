import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsSetupTabs } from "@/components/docs-setup-tabs";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "Custom Domains — Rift Docs",
  description: "Use your own brand domain for deep links with Rift custom domains.",
};

export default function DomainsPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Setup</p>
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
            when <a href="/docs/links" className="underline">creating links</a>. Without a custom
            domain, links use auto-generated IDs on the shared <code>riftl.ink</code> domain.
          </Callout>
        </section>

        <div className="gradient-line" />

        {/* Primary & Alternate */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Primary &amp; alternate domains</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Each tenant registers two custom domains: a <strong className="text-[#fafafa]">primary</strong> domain
            and an <strong className="text-[#fafafa]">alternate</strong> domain.
          </p>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="rounded-xl border border-[#1e1e22] bg-[#111113] p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">Primary</p>
              <p className="mt-1 font-mono text-[14px] text-[#fafafa]">go.yourcompany.com</p>
              <p className="mt-2 text-[13px] leading-relaxed text-[#71717a]">
                Serves landing pages, resolves links, records clicks. This is the domain your
                users see in URLs.
              </p>
            </div>
            <div className="rounded-xl border border-[#1e1e22] bg-[#111113] p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#f59e0b]">Alternate</p>
              <p className="mt-1 font-mono text-[14px] text-[#fafafa]">open.yourcompany.com</p>
              <p className="mt-2 text-[13px] leading-relaxed text-[#71717a]">
                Used only for the &ldquo;Open in App&rdquo; tap. No landing pages, no click
                recording, no analytics.
              </p>
            </div>
          </div>
          <Callout type="info">
            <strong>Why two domains?</strong> iOS and Android don&apos;t trigger Universal Links /
            App Links when the tap originates from the same domain as the link destination.
            The landing page lives on your primary domain, so the &ldquo;Open in App&rdquo; button
            must point to a <em>different</em> domain for the OS to intercept the tap and open your
            app directly.
          </Callout>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Both domains use the same Cloudflare Worker, the same AASA serving, and the same
            verification flow. The CLI sets up both in sequence.
          </p>
        </section>

        <div className="gradient-line" />

        <DocsSetupTabs
          title="Set it up"
          tabs={[
            {
              id: "cli",
              label: "CLI (Recommended)",
              children: (
                <div className="space-y-4 text-[15px] leading-relaxed text-[#a1a1aa]">
                  <p>
                    The CLI handles domain registration, TXT verification, and testing. It
                    prompts you through both your primary and alternate domain in one flow.
                  </p>
                  <CodeBlock lang="bash">{`rift setup domain`}</CodeBlock>
                  <div className="rounded-xl border border-[#2dd4bf]/20 bg-[#2dd4bf]/5 p-4">
                    <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
                      What the CLI handles
                    </p>
                    <div className="mt-3 space-y-2 text-[14px] text-[#d4d4d8]">
                      <p>1. Recommends a primary domain like <code className="rounded bg-[#18181b] px-1.5 py-0.5 text-[13px]">go.yourcompany.com</code></p>
                      <p>2. Registers the domain with Rift and shows you the TXT record to add</p>
                      <p>3. Waits for DNS propagation and verifies ownership</p>
                      <p>4. Tests your Worker once you finish the Cloudflare side</p>
                      <p>5. Continues into your alternate domain setup</p>
                    </div>
                  </div>
                  <p>
                    You still need to create the Cloudflare Worker yourself — see the setup below.
                  </p>
                </div>
              ),
            },
            {
              id: "manual",
              label: "Manual",
              children: (
                <div className="space-y-4 text-[15px] leading-relaxed text-[#a1a1aa]">
                  <p>
                    If you prefer full control over the API calls, DNS records, and verification
                    steps, follow the{" "}
                    <a href="/docs/manual-setup" className="text-[#2dd4bf] hover:underline">
                      manual setup guide
                    </a>
                    . It walks through every step from account creation to domain verification.
                  </p>
                  <p>
                    Either way, you&apos;ll need to create the Cloudflare Worker below.
                  </p>
                </div>
              ),
            },
          ]}
        />

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Create the Cloudflare Worker</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            This is required regardless of whether you use the CLI or manual setup.
            The same Worker handles both your primary and alternate domains.
          </p>

          <Step n={1} title="Create the Worker">
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

          <Step n={2} title="Attach it to your domains">
            <p>
              On the Worker page → <strong className="text-[#fafafa]">Settings</strong> → <strong className="text-[#fafafa]">Domains &amp; Routes</strong> → <strong className="text-[#fafafa]">Add</strong> → <strong className="text-[#fafafa]">Custom Domain</strong>.
              Add both your primary and alternate domains:
            </p>
            <div className="overflow-x-auto">
              <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Custom Domain</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Route Pattern</th>
                  </tr>
                </thead>
                <tbody className="text-[#a1a1aa]">
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">go.yourcompany.com</td>
                    <td className="px-4 py-2.5 font-mono">go.yourcompany.com/*</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#f59e0b]">open.yourcompany.com</td>
                    <td className="px-4 py-2.5 font-mono">open.yourcompany.com/*</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>
              Cloudflare auto-creates proxied DNS records for each custom domain.
            </p>
            <Callout type="warning">
              Route patterns must include <code>{'/*'}</code> at the end. Without the wildcard,
              only the bare domain will match — paths like <code>/download</code> won&apos;t be proxied.
            </Callout>
          </Step>
        </section>
      </div>
    </div>
  );
}
