"use client";

import { useEffect, useState } from "react";
import { motion } from "motion/react";
import dynamic from "next/dynamic";
import { TerminalDemo } from "@/components/terminal-demo";

const WarpTunnel = dynamic(
  () => import("@/components/warp-tunnel").then((m) => m.WarpTunnel),
  { ssr: false }
);

const fade = (delay: number) => ({
  initial: { opacity: 0, y: 20 },
  whileInView: { opacity: 1, y: 0 },
  viewport: { once: true },
  transition: { duration: 0.5, delay, ease: "easeOut" as const },
});

const heroPhrases = ["Built for humans.", "Ready for agents."];

export default function Home() {
  const [phraseIndex, setPhraseIndex] = useState(0);
  const [visibleText, setVisibleText] = useState("");
  const [isDeleting, setIsDeleting] = useState(false);

  useEffect(() => {
    const phrase = heroPhrases[phraseIndex];
    const atFullPhrase = visibleText === phrase;
    const atEmptyPhrase = visibleText.length === 0;

    const timeout = window.setTimeout(() => {
      if (!isDeleting) {
        if (atFullPhrase) {
          setIsDeleting(true);
          return;
        }

        setVisibleText(phrase.slice(0, visibleText.length + 1));
        return;
      }

      if (atEmptyPhrase) {
        setIsDeleting(false);
        setPhraseIndex((current) => (current + 1) % heroPhrases.length);
        return;
      }

      setVisibleText(phrase.slice(0, visibleText.length - 1));
    }, atFullPhrase ? 1400 : isDeleting ? 45 : 75);

    return () => window.clearTimeout(timeout);
  }, [isDeleting, phraseIndex, visibleText]);

  return (
    <>
      {/* ─── HERO ─── */}
      <section className="relative min-h-screen flex flex-col justify-center overflow-hidden">
        <WarpTunnel />
        <div className="absolute inset-0 grid-bg grid-bg-fade pointer-events-none" style={{ opacity: 0.3 }} />

        <div className="relative z-10 mx-auto max-w-6xl px-6 w-full pt-32 pb-20">
          <motion.div {...fade(0.2)} className="mb-6">
            <span className="inline-flex items-center gap-2 text-[12px] text-[#52525b] font-mono tracking-wide">
              <span className="size-1.5 rounded-full bg-[#2dd4bf] dot-pulse" />
              API-first link infrastructure
            </span>
          </motion.div>

          <motion.h1
            {...fade(0.35)}
            className="text-[clamp(2.5rem,6vw,5rem)] font-semibold leading-[1.05] tracking-[-0.04em] max-w-3xl"
          >
            <span className="sr-only">
              Deep linking and attribution API. Built for humans. Ready for agents.
            </span>
            <span aria-hidden="true">
              Deep linking and attribution API
              <br />
              <span className="text-[#2dd4bf] inline-flex min-h-[1.2em] items-center">
                {visibleText}
                <span className="ml-1 cursor-blink text-[#5eead4]">|</span>
              </span>
            </span>
          </motion.h1>

          <motion.p
            {...fade(0.5)}
            className="mt-6 text-lg text-[#71717a] leading-relaxed max-w-xl"
          >
            A lighter, cheaper way to power links for both user journeys and
            agent-driven actions.
          </motion.p>

          <motion.div {...fade(0.65)} className="mt-8 flex items-center gap-4">
            <a
              href="#"
              className="inline-flex items-center gap-2 bg-[#2dd4bf] text-[#042f2e] text-sm font-medium px-5 py-2.5 rounded-lg hover:bg-[#5eead4] transition-colors glow-teal-sm"
            >
              Create your first link
            </a>
            <a
              href="/docs"
              className="inline-flex items-center gap-2 text-sm text-[#71717a] hover:text-[#fafafa] transition-colors"
            >
              Read quickstart
              <span className="text-[#3f3f46]">&rarr;</span>
            </a>
          </motion.div>

          {/* Quick curl example */}
          <motion.div {...fade(0.8)} className="mt-12">
            <div className="inline-flex items-center gap-3 bg-[#111113]/80 backdrop-blur border border-[#222225] rounded-lg px-4 py-2.5 font-mono text-[13px]">
              <span className="text-[#52525b]">$</span>
              <span className="text-[#71717a]">curl</span>
              <span className="text-[#fafafa]">-X POST</span>
              <span className="text-[#2dd4bf]">/v1/links</span>
              <span className="text-[#71717a]">-d</span>
              <span className="text-[#a78bfa]">&apos;{`{"destination":"myapp://promo"}`}&apos;</span>
            </div>
          </motion.div>
        </div>
      </section>

      {/* ─── DEMO ─── */}
      <section id="how-it-works" className="relative py-24 px-6">
        <div className="mx-auto max-w-6xl">
          <motion.div {...fade(0)}>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">How it works</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-3">The same link works in two ways</h2>
            <p className="text-[#71717a] mb-10 max-w-lg">
              For users, the link behaves like a normal deep link. For agents, the
              same URL resolves into structured metadata and actions instead of a blind redirect.
            </p>
          </motion.div>
          <motion.div {...fade(0.2)}>
            <TerminalDemo />
          </motion.div>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      {/* ─── USE CASES ─── */}
      <section className="py-24 px-6">
        <div className="mx-auto max-w-6xl">
          <motion.div {...fade(0)}>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">Why it matters</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-4">Keep customers moving through your product in the age of agents</h2>
            <p className="text-[#71717a] mb-12 max-w-2xl">
              As more journeys start in chat, assistants, and AI-driven interfaces,
              your links still need to route people to the right place, preserve intent,
              and create a path to action.
            </p>
          </motion.div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <motion.div {...fade(0.05)} className="bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors flex flex-col">
              <div className="flex items-center gap-2 mb-5">
                <div className="size-8 rounded-lg bg-[#2dd4bf]/10 flex items-center justify-center">
                  <span className="text-[#2dd4bf] text-sm">1</span>
                </div>
                <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">Journey</span>
              </div>
              <h3 className="text-lg font-medium mb-3">Keep the journey intact</h3>
              <p className="text-sm text-[#71717a] leading-relaxed mb-5">
                Route users into the right app screen or webpage instead of losing
                them in dead-end links and broken handoffs.
              </p>
              <div className="mt-auto bg-[#0c0c0e] border border-[#1e1e22] rounded-lg p-4 font-mono text-[12px] space-y-1">
                <div className="text-[#3f3f46]">{`// one care link`}</div>
                <div><span className="syn-key">surface</span><span className="text-[#52525b]">: </span><span className="syn-str">&quot;patient portal, chat, sms&quot;</span></div>
                <div><span className="syn-key">destination</span><span className="text-[#52525b]">: </span><span className="syn-str">&quot;app or scheduling page&quot;</span></div>
                <div><span className="syn-key">handoff</span><span className="text-[#52525b]">: </span><span className="syn-str">&quot;follow_up_visit&quot;</span></div>
              </div>
            </motion.div>

            <motion.div {...fade(0.1)} className="bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors flex flex-col">
              <div className="flex items-center gap-2 mb-5">
                <div className="size-8 rounded-lg bg-[#a78bfa]/10 flex items-center justify-center">
                  <span className="text-[#a78bfa] text-sm">2</span>
                </div>
                <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">Intent</span>
              </div>
              <h3 className="text-lg font-medium mb-3">Preserve intent across interfaces</h3>
              <p className="text-sm text-[#71717a] leading-relaxed mb-5">
                The same link can carry enough context for agents to understand what
                the user is trying to do, not just where to redirect.
              </p>
              <div className="mt-auto bg-[#0c0c0e] border border-[#1e1e22] rounded-lg p-4 font-mono text-[12px] space-y-1">
                <div className="text-[#3f3f46]">{`// agent-readable context`}</div>
                <div><span className="syn-key">action</span><span className="text-[#52525b]">: </span><span className="syn-str">&quot;schedule_appointment&quot;</span></div>
                <div><span className="syn-key">visit</span><span className="text-[#52525b]">: </span><span className="syn-str">&quot;annual_checkup&quot;</span></div>
                <div><span className="syn-key">provider</span><span className="text-[#52525b]">: </span><span className="syn-str">&quot;cityhealth_primary_care&quot;</span></div>
              </div>
            </motion.div>

            <motion.div {...fade(0.15)} className="bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors flex flex-col">
              <div className="flex items-center gap-2 mb-5">
                <div className="size-8 rounded-lg bg-[#f59e0b]/10 flex items-center justify-center">
                  <span className="text-[#f59e0b] text-sm">3</span>
                </div>
                <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">Action</span>
              </div>
              <h3 className="text-lg font-medium mb-3">Turn resolution into action</h3>
              <p className="text-sm text-[#71717a] leading-relaxed mb-5">
                When agents can resolve structured actions, your links do more than
                send traffic. They help complete the task.
              </p>
              <div className="mt-auto bg-[#0c0c0e] border border-[#1e1e22] rounded-lg p-4 font-mono text-[12px] space-y-1">
                <div className="text-[#3f3f46]">{`// outcome, not just traffic`}</div>
                <div><span className="syn-key">resolved</span><span className="text-[#52525b]">: </span><span className="syn-num">1</span></div>
                <div><span className="syn-key">completed</span><span className="text-[#52525b]">: </span><span className="syn-num">1</span></div>
                <div><span className="syn-key">status</span><span className="text-[#52525b]">: </span><span className="syn-str">&quot;scheduled&quot;</span></div>
              </div>
            </motion.div>
          </div>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      {/* ─── COMPARISON ─── */}
      <section className="py-24 px-6">
        <div className="mx-auto max-w-6xl">
          <motion.div {...fade(0)}>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">Why Rift</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-4">Most link platforms are expensive, bloated, and painful to build on</h2>
            <p className="text-[#71717a] mb-12 max-w-lg">
              If you&apos;ve fought heavyweight link tooling before, Rift is the alternative:
              simpler to integrate, easier to reason about, and ready for both human
              traffic today and agent-driven workflows over time.
            </p>
          </motion.div>

          <motion.div {...fade(0.1)}>
            <div className="overflow-hidden rounded-xl border border-[#222225]">
              <table className="w-full text-[13px]">
                <thead>
                  <tr className="bg-[#0c0c0e]">
                    <th className="text-left font-mono font-medium text-[#52525b] px-5 py-3.5 border-b border-[#222225]" />
                    <th className="text-center font-mono font-medium text-[#52525b] px-5 py-3.5 border-b border-[#222225]">Bitly</th>
                    <th className="text-center font-mono font-medium text-[#52525b] px-5 py-3.5 border-b border-[#222225]">Branch</th>
                    <th className="text-center font-mono font-medium text-[#2dd4bf] px-5 py-3.5 border-b border-[#222225]">Rift</th>
                  </tr>
                </thead>
                <tbody>
                  {[
                    { feature: "Short links", bitly: true, branch: true, rift: true },
                    { feature: "Deep links", bitly: false, branch: true, rift: true },
                    { feature: "Install attribution", bitly: false, branch: true, rift: true },
                    { feature: "Agent-readable", bitly: false, branch: false, rift: true },
                    { feature: "Self-serve API", bitly: false, branch: false, rift: true },
                    { feature: "Lightweight SDK", bitly: null, branch: false, rift: true },
                    { feature: "Pay per request", bitly: false, branch: false, rift: true },
                  ].map((row, i) => (
                    <tr key={row.feature} className={i % 2 === 0 ? "bg-[#111113]" : "bg-[#0e0e10]"}>
                      <td className="px-5 py-3 text-[#a1a1aa] border-b border-[#222225]/50">{row.feature}</td>
                      {[row.bitly, row.branch, row.rift].map((val, j) => (
                        <td key={j} className="text-center px-5 py-3 border-b border-[#222225]/50">
                          {val === true ? (
                            <span className={j === 2 ? "text-[#2dd4bf]" : "text-[#52525b]"}>&#10003;</span>
                          ) : val === false ? (
                            <span className="text-[#2a2a2d]">&#10005;</span>
                          ) : (
                            <span className="text-[#2a2a2d]">—</span>
                          )}
                        </td>
                      ))}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </motion.div>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      {/* ─── FEATURES ─── */}
      <section className="py-24 px-6">
        <div className="mx-auto max-w-6xl">
          <motion.div {...fade(0)}>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">Under the hood</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-12">API-first link infrastructure</h2>
          </motion.div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            {/* Wide card */}
            <motion.div {...fade(0.05)} className="md:col-span-2 bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors">
              <div className="flex items-center gap-2 mb-4">
                <div className="size-2 rounded-full bg-[#2dd4bf]" />
                <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">One link, every platform</span>
              </div>
              <h3 className="text-lg font-medium mb-2">Vanity slugs or auto-generated IDs</h3>
              <p className="text-sm text-[#71717a] leading-relaxed max-w-md">
                Create <code className="text-[#a1a1aa] bg-[#18181b] px-1 rounded">/r/summer-launch</code> or let us
                generate <code className="text-[#a1a1aa] bg-[#18181b] px-1 rounded">/r/A7F3B2C1</code>.
                Attach a destination URL, arbitrary JSON metadata, and campaign tracking — all in one POST.
                Works on iOS, Android, and web.
              </p>
            </motion.div>

            {/* Tall card */}
            <motion.div {...fade(0.1)} className="md:row-span-2 bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors flex flex-col">
              <div className="flex items-center gap-2 mb-4">
                <div className="size-2 rounded-full bg-[#f59e0b]" />
                <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">API key, you&apos;re live</span>
              </div>
              <h3 className="text-lg font-medium mb-2">No sales call. No contract.</h3>
              <p className="text-sm text-[#71717a] leading-relaxed mb-6">
                Sign up, get a key, start creating links. Data is fully isolated per tenant — your links, clicks,
                and attributions are never mixed with anyone else&apos;s.
              </p>
              <div className="mt-auto bg-[#0c0c0e] border border-[#1e1e22] rounded-lg p-4 font-mono text-[12px] space-y-1.5">
                <div><span className="syn-key">key</span><span className="text-[#52525b]"> → </span><span className="syn-str">rift_live_...</span></div>
                <div><span className="syn-key">links</span><span className="text-[#52525b]"> → </span><span className="syn-num">isolated</span></div>
                <div><span className="syn-key">clicks</span><span className="text-[#52525b]"> → </span><span className="syn-num">isolated</span></div>
                <div><span className="syn-key">attribution</span><span className="text-[#52525b]"> → </span><span className="syn-num">isolated</span></div>
              </div>
            </motion.div>

            {/* Regular cards */}
            <motion.div {...fade(0.15)} className="bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors">
              <div className="flex items-center gap-2 mb-4">
                <div className="size-2 rounded-full bg-[#3b82f6]" />
                <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">Full funnel</span>
              </div>
              <h3 className="text-lg font-medium mb-2">Not just clicks</h3>
              <p className="text-sm text-[#71717a] leading-relaxed">
                Click → install → user signup → conversion. Deferred deep linking works even if the app
                wasn&apos;t installed when the link was clicked. Idempotent attribution, no double-counting.
              </p>
            </motion.div>

            <motion.div {...fade(0.2)} className="bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors">
              <div className="flex items-center gap-2 mb-4">
                <div className="size-2 rounded-full bg-[#a78bfa]" />
                <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">Human vs Agent</span>
              </div>
              <h3 className="text-lg font-medium mb-2">See who&apos;s resolving your links</h3>
              <p className="text-sm text-[#71717a] leading-relaxed">
                Separate analytics for human clicks and agent resolutions. Know when
                AI traffic overtakes human traffic — and what that means for your funnel.
              </p>
            </motion.div>
          </div>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      {/* ─── API WALKTHROUGH ─── */}
      <section className="py-24 px-6">
        <div className="mx-auto max-w-6xl">
          <motion.div {...fade(0)}>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">API</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-12">Five endpoints. That&apos;s the whole API.</h2>
          </motion.div>

          <div className="space-y-2">
            {[
              { method: "POST", path: "/v1/links", desc: "Create a deep link with metadata", auth: true },
              { method: "GET", path: "/v1/links", desc: "List your links", auth: true },
              { method: "GET", path: "/r/{id}", desc: "Resolve — redirect or JSON", auth: false },
              { method: "POST", path: "/v1/attribution", desc: "Report an install from the SDK", auth: false },
              { method: "GET", path: "/v1/links/{id}/stats", desc: "Click, install & conversion stats", auth: true },
            ].map((ep, i) => (
              <motion.div
                key={ep.path + ep.method}
                {...fade(i * 0.05)}
                className="flex items-center gap-4 bg-[#111113] border border-[#222225] rounded-lg px-5 py-3.5 hover:border-[#2dd4bf]/20 transition-colors group"
              >
                <span className={`font-mono text-[12px] font-medium w-12 ${
                  ep.method === "POST" ? "text-[#34d399]" :
                  ep.method === "GET" ? "text-[#60a5fa]" :
                  "text-[#f59e0b]"
                }`}>
                  {ep.method}
                </span>
                <span className="font-mono text-[13px] text-[#fafafa] min-w-[200px]">{ep.path}</span>
                <span className="text-[13px] text-[#52525b] flex-1">{ep.desc}</span>
                {ep.auth ? (
                  <span className="text-[11px] font-mono text-[#2dd4bf]/60 border border-[#2dd4bf]/20 rounded px-2 py-0.5">auth</span>
                ) : (
                  <span className="text-[11px] font-mono text-[#52525b] border border-[#222225] rounded px-2 py-0.5">public</span>
                )}
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      {/* ─── PRICING ─── */}
      <section id="pricing" className="py-24 px-6">
        <div className="mx-auto max-w-6xl">
          <motion.div {...fade(0)}>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">Pricing</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-3">Start free. Scale with usage.</h2>
            <p className="text-[#71717a] mb-12">No credit card. No trial period. No sales demo.</p>
          </motion.div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            {[
              {
                name: "Free",
                price: "$0",
                desc: "For prototyping",
                items: ["100 links", "1,000 clicks/month", "Full API access", "Click & install tracking", "7-day analytics retention"],
                accent: false,
              },
              {
                name: "Pay per request",
                price: "$0.01",
                unit: "/ request",
                desc: "For production",
                items: ["Unlimited links & clicks", "Pay with USDC — no API key needed", "Agents can pay autonomously", "Full attribution & analytics", "Unlimited retention"],
                accent: true,
              },
              {
                name: "Volume",
                price: "Custom",
                desc: "For scale",
                items: ["Volume discounts", "Custom domains", "Webhooks on events", "SLA & priority support", "Dedicated onboarding"],
                accent: false,
              },
            ].map((tier, i) => (
              <motion.div
                key={tier.name}
                {...fade(i * 0.08)}
                className={`rounded-xl border p-7 flex flex-col ${
                  tier.accent
                    ? "border-[#2dd4bf]/30 bg-[#2dd4bf]/[0.03] glow-teal"
                    : "border-[#222225] bg-[#111113] hover:border-[#2dd4bf]/15"
                } transition-colors`}
              >
                {tier.accent && (
                  <span className="text-[11px] font-mono text-[#2dd4bf] mb-4">Recommended</span>
                )}
                <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-3">{tier.name}</p>
                <div className="flex items-baseline gap-1 mb-1">
                  <span className="text-3xl font-semibold tracking-tight">{tier.price}</span>
                  {tier.unit && <span className="text-sm text-[#52525b]">{tier.unit}</span>}
                </div>
                <p className="text-[13px] text-[#52525b] mb-6">{tier.desc}</p>
                <div className="h-px bg-[#222225] mb-5" />
                <ul className="space-y-2.5 flex-1 mb-6">
                  {tier.items.map((item) => (
                    <li key={item} className="flex items-start gap-2.5 text-[13px] text-[#71717a]">
                      <span className="mt-1.5 size-1 rounded-full bg-[#2dd4bf] shrink-0" />
                      {item}
                    </li>
                  ))}
                </ul>
                <a
                  href="#"
                  className={`text-center text-[13px] font-medium px-4 py-2 rounded-lg transition-colors ${
                    tier.accent
                      ? "bg-[#2dd4bf] text-[#042f2e] hover:bg-[#5eead4]"
                      : "border border-[#222225] text-[#a1a1aa] hover:border-[#2dd4bf]/30 hover:text-[#fafafa]"
                  }`}
                >
                  {tier.accent ? "Create your first link" : "Read quickstart"}
                </a>
              </motion.div>
            ))}
          </div>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      {/* ─── CTA ─── */}
      <section className="py-24 px-6">
        <div className="mx-auto max-w-6xl">
          <motion.div {...fade(0)} className="text-center">
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-4">
              Create your first link in <span className="text-[#2dd4bf]">30 seconds</span>
            </h2>
            <p className="text-[#71717a] mb-8 max-w-md mx-auto">
              Sign up, get an API key, and POST to /v1/links. That&apos;s it.
            </p>
            <div className="flex items-center justify-center gap-4">
              <a
                href="#"
                className="inline-flex items-center gap-2 bg-[#2dd4bf] text-[#042f2e] text-sm font-medium px-6 py-2.5 rounded-lg hover:bg-[#5eead4] transition-colors glow-teal-sm"
              >
                Create your first link
              </a>
              <a
                href="/docs"
                className="inline-flex items-center gap-2 text-sm text-[#71717a] hover:text-[#fafafa] transition-colors"
              >
                Read quickstart <span className="text-[#3f3f46]">&rarr;</span>
              </a>
            </div>
          </motion.div>
        </div>
      </section>
    </>
  );
}
