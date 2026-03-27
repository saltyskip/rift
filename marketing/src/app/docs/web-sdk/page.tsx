"use client";

import { useState } from "react";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

const FRAMEWORKS = [
  {
    id: "html",
    label: "HTML",
    lang: "html",
    code: `<script src="https://api.riftl.ink/sdk/rift.js"></script>
<script>
  Rift.init("pk_live_YOUR_KEY");
</script>

<a
  href="https://go.yourcompany.com/summer-sale"
  onclick="if(window.Rift) Rift.click('summer-sale')"
>
  Get the App
</a>`,
    notes: "The link is a plain <a> tag pointing to your Universal Link domain. Universal Links open the app if installed; the landing page loads if not. Rift.click() fires a beacon to record the click without blocking navigation.",
  },
  {
    id: "nextjs",
    label: "Next.js",
    lang: "jsx",
    code: `// components/rift-init.tsx — Client Component for script loading
"use client";
import Script from "next/script";

export function RiftScript() {
  return (
    <Script
      src="https://api.riftl.ink/sdk/rift.js"
      strategy="afterInteractive"
      onLoad={() => {
        if (window.Rift && process.env.NEXT_PUBLIC_RIFT_PK) {
          window.Rift.init(process.env.NEXT_PUBLIC_RIFT_PK);
        }
      }}
    />
  );
}

// app/layout.tsx — add RiftScript to your root layout
import { RiftScript } from "@/components/rift-init";

export default function RootLayout({ children }) {
  return (
    <html lang="en">
      <body>
        <RiftScript />
        {children}
      </body>
    </html>
  );
}

// components/download-button.tsx — plain <a> tag with click tracking
"use client";

export function DownloadButton({ linkId }: { linkId: string }) {
  const href = \`https://go.yourcompany.com/\${linkId}\`;
  return (
    <a
      href={href}
      onClick={() => window.Rift?.click(linkId)}
    >
      Get the App
    </a>
  );
}`,
    notes: "The script must be loaded in a Client Component (Server Components can't use onLoad). The download button is a plain <a> tag — Universal Links fire on tap, Rift.click() records the click via sendBeacon without blocking.",
  },
  {
    id: "svelte",
    label: "Svelte",
    lang: "svelte",
    code: `<svelte:head>
  <script src="https://api.riftl.ink/sdk/rift.js" on:load={() => Rift.init('pk_live_YOUR_KEY')}></script>
</svelte:head>

<a
  href="https://go.yourcompany.com/summer-sale"
  on:click={() => window.Rift?.click('summer-sale')}
>
  Get the App
</a>`,
    notes: "Uses <svelte:head> to load the script and initialize with your publishable key. The click handler records the event without preventing the default navigation.",
  },
  {
    id: "vue",
    label: "Vue / Nuxt",
    lang: "vue",
    code: `<script setup>
import { onMounted } from "vue";

onMounted(() => {
  const s = document.createElement("script");
  s.src = "https://api.riftl.ink/sdk/rift.js";
  s.onload = () => window.Rift.init("pk_live_YOUR_KEY");
  document.head.appendChild(s);
});
</script>

<template>
  <a
    href="https://go.yourcompany.com/summer-sale"
    @click="window.Rift?.click('summer-sale')"
  >
    Get the App
  </a>
</template>`,
    notes: "Loads the script dynamically in onMounted and calls Rift.init() with your publishable key on load. The link is a plain <a> tag — no preventDefault needed.",
  },
];

export default function WebSdkPage() {
  const [active, setActive] = useState("html");
  const fw = FRAMEWORKS.find((f) => f.id === active)!;

  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Web SDK</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Installation</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Add &ldquo;Download&rdquo; or &ldquo;Open in App&rdquo; buttons to your website
          with Universal Links support and click tracking.
        </p>
      </div>

      <div className="space-y-10">
        {/* Prerequisites */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Prerequisites</h2>
          <p className="text-[15px] text-[#a1a1aa]">
            You need a <a href="/docs/publishable-keys" className="text-[#2dd4bf] hover:underline">publishable key</a>{" "}
            (<code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">pk_live_</code>) to use
            the web SDK. Create one after setting up a{" "}
            <a href="/docs/domains" className="text-[#2dd4bf] hover:underline">custom domain</a>.
          </p>
        </section>

        <div className="gradient-line" />

        {/* Framework tabs */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Quick start</h2>
          <div className="flex gap-1 p-1 rounded-lg bg-[#111113] border border-[#1e1e22] w-fit">
            {FRAMEWORKS.map((f) => (
              <button
                key={f.id}
                onClick={() => setActive(f.id)}
                className={`px-3 py-1.5 rounded-md text-[13px] font-medium transition-colors ${
                  active === f.id
                    ? "bg-[#2dd4bf]/10 text-[#2dd4bf]"
                    : "text-[#71717a] hover:text-[#fafafa]"
                }`}
              >
                {f.label}
              </button>
            ))}
          </div>
          <CodeBlock lang={fw.lang}>{fw.code}</CodeBlock>
          <p className="text-[14px] text-[#a1a1aa]">{fw.notes}</p>
        </section>

        <div className="gradient-line" />

        {/* API */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">API Reference</h2>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-2 py-1 rounded text-[15px]">Rift.init(publishableKey, opts?)</code>
            </h3>
            <p className="text-[15px] text-[#a1a1aa]">
              Initializes the SDK with your publishable key. Must be called before <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">Rift.click()</code>.
            </p>
            <div className="overflow-x-auto">
              <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Param</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Type</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Description</th>
                  </tr>
                </thead>
                <tbody className="text-[#a1a1aa]">
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">publishableKey</td>
                    <td className="px-4 py-2.5 font-mono">string</td>
                    <td className="px-4 py-2.5">Your publishable key (starts with <code className="text-[#71717a]">pk_live_</code>). Required.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">opts.baseUrl</td>
                    <td className="px-4 py-2.5 font-mono">string</td>
                    <td className="px-4 py-2.5">API base URL. Default: <code className="text-[#71717a]">https://api.riftl.ink</code></td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-2 py-1 rounded text-[15px]">Rift.click(linkId, opts?)</code>
            </h3>
            <p className="text-[15px] text-[#a1a1aa]">
              Records a click event via <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">sendBeacon</code>.
              Fire-and-forget &mdash; does not block navigation, does not return data.
              Use in the <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">onClick</code> handler
              of an <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">&lt;a&gt;</code> tag.
              Do not call <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">preventDefault()</code> &mdash;
              the <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">&lt;a&gt;</code> tag handles navigation
              so Universal Links work.
            </p>
            <div className="overflow-x-auto">
              <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Param</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Type</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Description</th>
                  </tr>
                </thead>
                <tbody className="text-[#a1a1aa]">
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">linkId</td>
                    <td className="px-4 py-2.5 font-mono">string</td>
                    <td className="px-4 py-2.5">The link ID to record a click for.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">opts.domain</td>
                    <td className="px-4 py-2.5 font-mono">string</td>
                    <td className="px-4 py-2.5">Custom domain for the clipboard URL written during the click (e.g., <code className="text-[#71717a]">go.yourcompany.com</code>). Defaults to the current page hostname.</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-2 py-1 rounded text-[15px]">Rift.getLink(linkId, opts?)</code>
            </h3>
            <p className="text-[15px] text-[#a1a1aa]">
              Fetches link data without navigating. Returns a Promise with the link metadata,
              destinations, and agent context. Useful for building custom UI.
            </p>
            <CodeBlock lang="javascript">{`const link = await Rift.getLink("summer-sale");
console.log(link.agent_context); // { action, cta, description }
console.log(link._rift_meta);    // { status, tenant_domain, ... }`}</CodeBlock>
          </div>
        </section>

        <div className="gradient-line" />

        {/* How it works */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">How it works</h2>
          <ol className="list-decimal pl-5 space-y-2 text-[15px] text-[#a1a1aa]">
            <li>
              The download button is a plain{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">&lt;a href=&quot;https://go.yourcompany.com/link-id&quot;&gt;</code>{" "}
              tag pointing to your Universal Link domain.
            </li>
            <li>
              When the user taps the link, <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">Rift.click()</code>{" "}
              fires a <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">sendBeacon</code> to{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">POST /v1/attribution/click</code>{" "}
              to record the click. This is fire-and-forget and does not block navigation.
            </li>
            <li>
              <strong className="text-[#fafafa]">App installed:</strong> iOS/Android intercepts the tap via Universal Links / App Links
              and opens the app directly. The landing page never loads.
            </li>
            <li>
              <strong className="text-[#fafafa]">App not installed:</strong> The browser navigates to the landing page,
              which shows a branded &ldquo;Get the App&rdquo; page with an App Store / Play Store button.
              The landing page also copies the link URL to clipboard (iOS) for deferred deep linking.
            </li>
          </ol>
        </section>
      </div>
    </div>
  );
}
