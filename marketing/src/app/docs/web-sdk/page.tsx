"use client";

import { useState } from "react";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

const FRAMEWORKS = [
  {
    id: "html",
    label: "HTML",
    lang: "html",
    code: `<script src="https://cdn.riftl.ink/rift.js"></script>
<script>
  Rift.init("pk_live_YOUR_KEY");
</script>

<a
  href="https://apps.apple.com/app/id123456789"
  target="_blank"
  rel="noopener noreferrer"
  onclick="if(window.Rift){event.preventDefault();try{Rift.open('summer-sale',{domain:'go.yourcompany.com'})}catch(e){window.open(this.href,'_blank')}}"
>
  Get the App
</a>`,
    notes: "The link works as a normal App Store link without JS. When the Rift SDK loads, it intercepts the click for tracking and deferred deep linking. Users can still right-click to copy the link.",
  },
  {
    id: "nextjs",
    label: "Next.js",
    lang: "jsx",
    code: `// app/layout.tsx — load the SDK once in your root layout
import Script from "next/script";

export default function RootLayout({ children }) {
  return (
    <html lang="en">
      <body>
        <Script
          src="https://cdn.riftl.ink/rift.js"
          strategy="lazyOnload"
          onLoad={() => window.Rift.init("pk_live_YOUR_KEY")}
        />
        {children}
      </body>
    </html>
  );
}

// components/download-button.tsx — progressive enhancement
"use client";

const STORE_URL = "https://apps.apple.com/app/id123456789";

export function DownloadButton({ linkId, domain }) {
  return (
    <a
      href={STORE_URL}
      target="_blank"
      rel="noopener noreferrer"
      onClick={(e) => {
        if (window.Rift) {
          e.preventDefault();
          try {
            window.Rift.open(linkId, { domain });
          } catch {
            window.open(STORE_URL, "_blank", "noopener,noreferrer");
          }
        }
      }}
    >
      Get the App
    </a>
  );
}`,
    notes: 'Load the script once in your root layout, not per component. Call Rift.init() with your publishable key before using Rift.open(). The download button is a real <a> tag that works without JS (right-clickable, accessible). Must be a Client Component ("use client") for the onClick handler.',
  },
  {
    id: "svelte",
    label: "Svelte",
    lang: "svelte",
    code: `<svelte:head>
  <script src="https://cdn.riftl.ink/rift.js" on:load={() => Rift.init('pk_live_YOUR_KEY')}></script>
</svelte:head>

<a
  href="https://apps.apple.com/app/id123456789"
  target="_blank"
  rel="noopener noreferrer"
  on:click={(e) => {
    if (window.Rift) {
      e.preventDefault();
      try {
        window.Rift.open('summer-sale', { domain: 'go.yourcompany.com' });
      } catch {
        window.open(e.currentTarget.href, '_blank');
      }
    }
  }}
>
  Get the App
</a>`,
    notes: "Uses `<svelte:head>` to load the script and initialize with your publishable key. The link falls back to opening the href directly if the SDK hasn't loaded.",
  },
  {
    id: "vue",
    label: "Vue / Nuxt",
    lang: "vue",
    code: `<script setup>
import { onMounted } from "vue";

const storeUrl = "https://apps.apple.com/app/id123456789";

onMounted(() => {
  const s = document.createElement("script");
  s.src = "https://cdn.riftl.ink/rift.js";
  s.onload = () => window.Rift.init("pk_live_YOUR_KEY");
  document.head.appendChild(s);
});

function handleClick(e) {
  if (window.Rift) {
    e.preventDefault();
    try {
      window.Rift.open('summer-sale', { domain: 'go.yourcompany.com' });
    } catch {
      window.open(storeUrl, '_blank');
    }
  }
}
</script>

<template>
  <a :href="storeUrl" target="_blank" rel="noopener noreferrer" @click="handleClick">
    Get the App
  </a>
</template>`,
    notes: "Loads the script dynamically in `onMounted` and calls Rift.init() with your publishable key on load. The link works as a normal App Store link before the SDK loads.",
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
          Add &ldquo;Download&rdquo; or &ldquo;Open in App&rdquo; buttons to your website.
          The SDK handles click tracking, deferred deep linking, and platform-aware navigation
          &mdash; no redirect through a landing page.
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
              Initializes the SDK with your publishable key. Must be called before <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">Rift.open()</code>.
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
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-2 py-1 rounded text-[15px]">Rift.open(linkId, opts?)</code>
            </h3>
            <p className="text-[15px] text-[#a1a1aa]">
              Records a click via <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">POST /v1/attribution/click</code>,
              copies the link URL to clipboard (iOS) for deferred deep linking, and navigates the user
              to the deep link, store, or web URL based on their platform.
            </p>
            <div className="overflow-x-auto">
              <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Option</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Type</th>
                    <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Description</th>
                  </tr>
                </thead>
                <tbody className="text-[#a1a1aa]">
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">domain</td>
                    <td className="px-4 py-2.5 font-mono">string</td>
                    <td className="px-4 py-2.5">
                      Custom domain for the clipboard URL on iOS (e.g. <code className="text-[#71717a]">go.yourcompany.com</code>).
                      Defaults to <code className="text-[#71717a]">location.hostname</code>.
                    </td>
                  </tr>
                  <tr className="border-b border-[#1e1e22]">
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">onComplete</td>
                    <td className="px-4 py-2.5 font-mono">function</td>
                    <td className="px-4 py-2.5">Called with the link data after the click is recorded.</td>
                  </tr>
                  <tr>
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">onError</td>
                    <td className="px-4 py-2.5 font-mono">function</td>
                    <td className="px-4 py-2.5">Called if the API request fails.</td>
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
              Fetches link data without navigating. Returns a Promise with the link metadata.
              Useful for building custom UI based on link data.
            </p>
            <CodeBlock lang="javascript">{`const link = await Rift.getLink("summer-sale");
document.getElementById("title").textContent = link.metadata.title;`}</CodeBlock>
          </div>
        </section>

        <div className="gradient-line" />

        {/* Advanced */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Advanced usage</h2>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">Explicit domain</h3>
            <p className="text-[15px] text-[#a1a1aa]">
              On iOS, the SDK copies the full link URL to the clipboard for deferred deep linking.
              By default it uses <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">location.hostname</code>.
              If the SDK is embedded on a page that isn&apos;t your custom domain (e.g. a third-party site),
              pass the domain explicitly so the clipboard URL uses your domain:
            </p>
            <CodeBlock lang="javascript">{`Rift.open("summer-sale", {
  domain: "go.yourcompany.com"
});`}</CodeBlock>
          </div>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">Self-hosted API</h3>
            <CodeBlock lang="javascript">{`Rift.init("pk_live_YOUR_KEY", {
  baseUrl: "https://api.yourcompany.com"
});`}</CodeBlock>
          </div>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">Custom callback</h3>
            <CodeBlock lang="javascript">{`Rift.open("summer-sale", {
  onComplete: function(data) {
    console.log("Platform:", data.platform);
    analytics.track("app_download_click", {
      link_id: "summer-sale",
      platform: data.platform
    });
  },
  onError: function(err) {
    console.error("Rift error:", err);
  }
});`}</CodeBlock>
          </div>
        </section>

        <div className="gradient-line" />

        {/* How it works */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">How it works</h2>
          <ol className="list-decimal pl-5 space-y-2 text-[15px] text-[#a1a1aa]">
            <li>
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">Rift.open()</code>{" "}
              sends a <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">POST /v1/attribution/click</code> (authenticated
              with the publishable key) to record the click and get the link data.
            </li>
            <li>
              <strong className="text-[#fafafa]">iOS:</strong> the full link URL is copied to the clipboard
              (e.g. <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">https://go.yourcompany.com/summer-sale</code>)
              for deferred deep linking after install.
            </li>
            <li>
              <strong className="text-[#fafafa]">Android:</strong> the link ID is appended to the Play Store URL
              as an install referrer parameter (<code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">rift_link=summer-sale</code>).
            </li>
            <li>
              The SDK navigates to the deep link URI with a 1.5s timeout fallback to the store URL.
              Desktop users go directly to the web URL.
            </li>
          </ol>
        </section>
      </div>
    </div>
  );
}
