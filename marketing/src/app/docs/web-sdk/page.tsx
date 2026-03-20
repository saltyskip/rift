"use client";

import { useState } from "react";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

const FRAMEWORKS = [
  {
    id: "html",
    label: "HTML",
    lang: "html",
    code: `<script src="https://cdn.riftl.ink/rift.js"></script>

<button onclick="Rift.open('summer-sale')">
  Get the App
</button>`,
    notes: "Drop the script tag anywhere in your page. The global `Rift` object is available immediately after load.",
  },
  {
    id: "nextjs",
    label: "Next.js",
    lang: "jsx",
    code: `"use client";
import Script from "next/script";

export function DownloadButton({ linkId }) {
  return (
    <>
      <Script
        src="https://cdn.riftl.ink/rift.js"
        strategy="lazyOnload"
      />
      <button onClick={() => window.Rift?.open(linkId)}>
        Get the App
      </button>
    </>
  );
}`,
    notes: 'Must be a Client Component ("use client") in the App Router. Use `window.Rift?.open()` with optional chaining since the script loads async. `strategy="lazyOnload"` avoids blocking page render.',
  },
  {
    id: "svelte",
    label: "Svelte",
    lang: "svelte",
    code: `<svelte:head>
  <script src="https://cdn.riftl.ink/rift.js"></script>
</svelte:head>

<button on:click={() => window.Rift?.open('summer-sale')}>
  Get the App
</button>`,
    notes: "Use `<svelte:head>` to load the script. Optional chaining handles cases where the script hasn't loaded yet.",
  },
  {
    id: "vue",
    label: "Vue / Nuxt",
    lang: "vue",
    code: `<script setup>
import { onMounted, ref } from "vue";
const ready = ref(false);
onMounted(() => {
  const s = document.createElement("script");
  s.src = "https://cdn.riftl.ink/rift.js";
  s.onload = () => { ready.value = true; };
  document.head.appendChild(s);
});
</script>

<template>
  <button @click="window.Rift?.open('summer-sale')">
    Get the App
  </button>
</template>`,
    notes: "Load the script dynamically in `onMounted` to avoid SSR issues. The `ready` ref can be used to show a loading state.",
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
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-2 py-1 rounded text-[15px]">Rift.open(linkId, opts?)</code>
            </h3>
            <p className="text-[15px] text-[#a1a1aa]">
              Records a click, writes the deferred deep link token, and navigates the user
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
                    <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">baseUrl</td>
                    <td className="px-4 py-2.5 font-mono">string</td>
                    <td className="px-4 py-2.5">API base URL. Default: <code className="text-[#71717a]">https://api.riftl.ink</code></td>
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
            <h3 className="text-lg font-semibold text-[#fafafa]">Self-hosted API</h3>
            <CodeBlock lang="javascript">{`Rift.open("summer-sale", {
  baseUrl: "https://api.yourcompany.com"
});`}</CodeBlock>
          </div>

          <div className="space-y-4">
            <h3 className="text-lg font-semibold text-[#fafafa]">Custom callback</h3>
            <CodeBlock lang="javascript">{`Rift.open("summer-sale", {
  onComplete: function(data) {
    console.log("Platform:", data.platform);
    console.log("Token:", data.token);
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
              sends a <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">POST /v1/sdk/click</code> to
              record the click and get a deferred deep link token.
            </li>
            <li>
              <strong className="text-[#fafafa]">iOS:</strong> the token is written to the clipboard
              as <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">rift:&lt;token&gt;</code> for
              deferred deep linking.
            </li>
            <li>
              <strong className="text-[#fafafa]">Android:</strong> the token is appended to the Play Store URL
              as an install referrer parameter.
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
