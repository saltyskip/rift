import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsSetupTabs } from "@/components/docs-setup-tabs";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "Custom Domains — Rift Docs",
  description: "Use your own brand domain for deep links with Rift custom domains.",
  alternates: { canonical: "/docs/domains" },
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
            Custom domains work by pointing a <strong className="text-[#fafafa]">CNAME record</strong> at
            the Rift server. Rift auto-provisions a TLS certificate via Let&apos;s Encrypt
            and uses the <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">Host</code> header
            to route requests to the right tenant.
          </p>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Any DNS provider works — Cloudflare, Route 53, Namecheap, Vercel DNS, etc.
            No proxy or worker is required.
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
            Both domains use the same AASA serving and the same
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
                    The CLI handles domain registration, DNS verification, and connectivity testing. It
                    prompts you through both your primary and alternate domain in one flow.
                  </p>
                  <CodeBlock lang="bash">{`rift domains setup`}</CodeBlock>
                  <div className="rounded-xl border border-[#2dd4bf]/20 bg-[#2dd4bf]/5 p-4">
                    <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
                      What the CLI handles
                    </p>
                    <div className="mt-3 space-y-2 text-[14px] text-[#d4d4d8]">
                      <p>1. Recommends a primary domain like <code className="rounded bg-[#18181b] px-1.5 py-0.5 text-[13px]">go.yourcompany.com</code></p>
                      <p>2. Registers the domain with Rift and shows you the CNAME and TXT records to add</p>
                      <p>3. Waits for DNS propagation and verifies ownership</p>
                      <p>4. Waits for TLS certificate provisioning</p>
                      <p>5. Tests connectivity through your custom domain</p>
                      <p>6. Continues into your alternate domain setup</p>
                    </div>
                  </div>
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
                </div>
              ),
            },
          ]}
        />

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">DNS records</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            For each custom domain, add two DNS records at your DNS provider:
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
                  <td className="px-4 py-2.5 font-mono">CNAME</td>
                  <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">go</td>
                  <td className="px-4 py-2.5 font-mono">returned by the API as <code>cname_target</code></td>
                </tr>
                <tr>
                  <td className="px-4 py-2.5 font-mono">TXT</td>
                  <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">_rift-verify.go</td>
                  <td className="px-4 py-2.5 font-mono">returned by the API as <code>verification_token</code></td>
                </tr>
              </tbody>
            </table>
          </div>

          <Callout type="info">
            Rift auto-provisions a TLS certificate via Let&apos;s Encrypt once the CNAME is pointing
            correctly. No manual certificate setup is needed.
          </Callout>

          <Callout type="warning">
            If you use Cloudflare, make sure the CNAME is set to <strong className="text-[#fafafa]">DNS only</strong> (grey
            cloud), not Proxied. Rift needs to terminate TLS directly.
          </Callout>
        </section>
      </div>
    </div>
  );
}
