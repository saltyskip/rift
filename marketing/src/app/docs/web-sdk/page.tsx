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
  Rift.init("pk_live_YOUR_KEY", { domain: "go.yourcompany.com" });
</script>

<!-- Just a normal link. Click tracking happens automatically. -->
<a href="https://go.yourcompany.com/summer-sale">Get the App</a>`,
    notes: "Pass your custom domain to Rift.init(). The SDK auto-tracks clicks on any link pointing to that domain — no attributes or event handlers needed.",
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
          window.Rift.init(process.env.NEXT_PUBLIC_RIFT_PK, {
            domain: "go.yourcompany.com",
          });
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

// Any component — just use normal links
export function DownloadButton({ linkId }: { linkId: string }) {
  return (
    <a href={\`https://go.yourcompany.com/\${linkId}\`}>
      Get the App
    </a>
  );
}`,
    notes: "The script must be loaded in a Client Component. Once initialized with your domain, all links to that domain are auto-tracked. No onClick, no data attributes — just plain links anywhere in your app.",
  },
  {
    id: "svelte",
    label: "Svelte",
    lang: "svelte",
    code: `<svelte:head>
  <script
    src="https://api.riftl.ink/sdk/rift.js"
    on:load={() => Rift.init('pk_live_YOUR_KEY', { domain: 'go.yourcompany.com' })}
  ></script>
</svelte:head>

<a href="https://go.yourcompany.com/summer-sale">Get the App</a>`,
    notes: "Initialize with your domain in the on:load handler. All matching links are auto-tracked — no per-link setup needed.",
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
  s.onload = () => window.Rift.init("pk_live_YOUR_KEY", {
    domain: "go.yourcompany.com",
  });
  document.head.appendChild(s);
});
</script>

<template>
  <a href="https://go.yourcompany.com/summer-sale">Get the App</a>
</template>`,
    notes: "Pass domain in the init call. All links to your domain are auto-tracked — no @click or custom attributes needed.",
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
          with Universal Links support and automatic click tracking.
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
              Initializes the SDK with your publishable key. Pass{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">domain</code> to enable
              automatic click tracking on all links pointing to that domain.
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
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">opts.domain</td>
                    <td className="px-4 py-2.5 font-mono">string</td>
                    <td className="px-4 py-2.5">Your custom link domain (e.g., <code className="text-[#71717a]">go.yourcompany.com</code>). Enables auto-tracking.</td>
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
              Manually record a click event for programmatic use cases. Not needed when using domain-based auto-tracking.
              Fire-and-forget via{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">sendBeacon</code> &mdash;
              does not block navigation.
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
                    <td className="px-4 py-2.5">Custom domain for the clipboard URL. Defaults to the init domain.</td>
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
              When the user clicks the link, the SDK detects it matches your domain and auto-fires a{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">sendBeacon</code> to{" "}
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
