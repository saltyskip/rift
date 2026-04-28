import { DeferredWarpTunnel } from "@/components/deferred-warp-tunnel";
import { HeroTypewriter } from "@/components/hero-typewriter";
import { PricingSection } from "@/components/pricing-section";
import { RevealOnView } from "@/components/reveal-on-view";
import { TerminalDemo } from "@/components/terminal-demo";

const COMPARISON_ROWS = [
  { feature: "Short links", bitly: true, branch: true, rift: true },
  { feature: "Deep links", bitly: false, branch: true, rift: true },
  { feature: "Install attribution", bitly: false, branch: true, rift: true },
  { feature: "Agent-readable", bitly: false, branch: false, rift: true },
  { feature: "Self-serve API", bitly: false, branch: false, rift: true },
  { feature: "Lightweight SDK", bitly: null, branch: false, rift: true },
  { feature: "Pay per request", bitly: false, branch: false, rift: true },
] as const;

const FEATURE_CARDS = [
  {
    title: "One link, every platform",
    tone: "bg-[#2dd4bf]",
    layout: "md:col-span-2",
    heading: "Vanity slugs or auto-generated IDs",
    copy:
      "Create /r/summer-launch or let us generate /r/A7F3B2C1. Attach a destination URL, arbitrary JSON metadata, and campaign tracking — all in one POST. Works on iOS, Android, and web.",
    wide: true,
  },
  {
    title: "API key, you're live",
    tone: "bg-[#f59e0b]",
    layout: "md:row-span-2",
    heading: "No sales call. No contract.",
    copy:
      "Sign up, get a key, start creating links. Data is fully isolated per tenant — your links, clicks, and attributions are never mixed with anyone else's.",
    tall: true,
  },
  {
    title: "Full funnel",
    tone: "bg-[#3b82f6]",
    heading: "Not just clicks",
    copy:
      "Click → install → user signup → conversion. Deferred deep linking works even if the app wasn't installed when the link was clicked. Idempotent attribution, no double-counting.",
  },
  {
    title: "Human vs Agent",
    tone: "bg-[#a78bfa]",
    heading: "See who's resolving your links",
    copy:
      "Separate analytics for human clicks and agent resolutions. Know when AI traffic overtakes human traffic — and what that means for your funnel.",
  },
] as const;

const API_STEPS = [
  { method: "POST", path: "/v1/auth/signup", desc: "Sign up and get an API key", auth: false },
  { method: "POST", path: "/v1/links", desc: "Create a deep link with metadata", auth: true },
  { method: "GET", path: "/r/{id}", desc: "Resolve — redirect or JSON", auth: false },
  { method: "POST", path: "/v1/attribution/install", desc: "Report an install from the SDK", auth: true },
  { method: "GET", path: "/v1/links/{id}/stats", desc: "Click, install & conversion stats", auth: true },
] as const;

export default function Home() {
  return (
    <>
      <section className="relative min-h-screen flex flex-col justify-center overflow-hidden">
        <div className="absolute inset-0 z-0 hero-gradient noise pointer-events-none" />
        <DeferredWarpTunnel />
        <div className="absolute inset-0 grid-bg grid-bg-fade pointer-events-none" style={{ opacity: 0.3 }} />

        <div className="relative z-10 mx-auto max-w-6xl px-6 w-full pt-32 pb-20">
          <div className="mb-6 animate-page-enter" style={{ animationDelay: "0.2s" }}>
            <span className="inline-flex items-center gap-2 text-[12px] text-[#52525b] font-mono tracking-wide">
              <span className="size-1.5 rounded-full bg-[#2dd4bf] dot-pulse" />
              API-first link infrastructure
            </span>
          </div>

          <h1
            className="text-[clamp(2.5rem,6vw,5rem)] font-semibold leading-[1.05] tracking-[-0.04em] max-w-3xl animate-page-enter"
            style={{ animationDelay: "0.35s" }}
          >
            <span className="sr-only">
              Deep linking and attribution API. Built for humans. Ready for agents.
            </span>
            <span aria-hidden="true">
              Deep linking and attribution API
              <br />
              <HeroTypewriter />
            </span>
          </h1>

          <p
            className="mt-6 text-lg text-[#71717a] leading-relaxed max-w-xl animate-page-enter"
            style={{ animationDelay: "0.5s" }}
          >
            A lighter, cheaper way to power links for both user journeys and
            agent-driven actions.
          </p>

          <div
            className="mt-8 flex items-center gap-4 animate-page-enter"
            style={{ animationDelay: "0.65s" }}
          >
            <a
              href="/docs"
              className="inline-flex items-center gap-2 bg-[#2dd4bf] text-[#042f2e] text-sm font-medium px-5 py-2.5 rounded-lg hover:bg-[#5eead4] transition-colors glow-teal-sm"
            >
              Create your first link
            </a>
            <a
              href="/docs"
              className="inline-flex items-center gap-2 text-sm text-[#71717a] hover:text-[#fafafa] transition-colors"
            >
              Read quickstart
              <span className="text-[#3f3f46]">→</span>
            </a>
          </div>

          <div className="mt-12 animate-page-enter" style={{ animationDelay: "0.8s" }}>
            <div className="inline-flex items-center gap-3 bg-[#111113]/80 backdrop-blur border border-[#222225] rounded-lg px-4 py-2.5 font-mono text-[13px]">
              <span className="text-[#52525b]">$</span>
              <span className="text-[#71717a]">curl</span>
              <span className="text-[#fafafa]">-X POST</span>
              <span className="text-[#2dd4bf]">/v1/links</span>
              <span className="text-[#71717a]">-d</span>
              <span className="text-[#a78bfa]">&apos;{`{"web_url":"https://example.com","ios_deep_link":"myapp://promo"}`}&apos;</span>
            </div>
          </div>
        </div>
      </section>

      <section id="how-it-works" className="relative py-24 px-6">
        <div className="mx-auto max-w-6xl">
          <RevealOnView>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">How it works</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-3">The same link works in two ways</h2>
            <p className="text-[#71717a] mb-10 max-w-lg">
              For users, the link behaves like a normal deep link. For agents, the
              same URL resolves into structured metadata and actions instead of a blind redirect.
            </p>
          </RevealOnView>
          <RevealOnView delay={0.2}>
            <TerminalDemo />
          </RevealOnView>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      <section className="py-24 px-6 content-auto-section">
        <div className="mx-auto max-w-6xl">
          <RevealOnView>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">Why it matters</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-4">Keep customers moving through your product in the age of agents</h2>
            <p className="text-[#71717a] mb-12 max-w-2xl">
              As more journeys start in chat, assistants, and AI-driven interfaces,
              your links still need to route people to the right place, preserve intent,
              and create a path to action.
            </p>
          </RevealOnView>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            {[
              {
                step: "1",
                label: "Journey",
                tone: "bg-[#2dd4bf]/10 text-[#2dd4bf]",
                heading: "Keep the journey intact",
                copy:
                  "Route users into the right app screen or webpage instead of losing them in dead-end links and broken handoffs.",
                code: [
                  ["surface", "\"patient portal, chat, sms\""],
                  ["destination", "\"app or scheduling page\""],
                  ["handoff", "\"follow_up_visit\""],
                ],
                comment: "// one care link",
              },
              {
                step: "2",
                label: "Intent",
                tone: "bg-[#a78bfa]/10 text-[#a78bfa]",
                heading: "Preserve intent across interfaces",
                copy:
                  "The same link can carry enough context for agents to understand what the user is trying to do, not just where to redirect.",
                code: [
                  ["action", "\"book\""],
                  ["cta", "\"Schedule your annual checkup\""],
                  ["description", "\"CityHealth primary care visit\""],
                ],
                comment: "// agent-readable context",
              },
              {
                step: "3",
                label: "Action",
                tone: "bg-[#f59e0b]/10 text-[#f59e0b]",
                heading: "Turn resolution into action",
                copy:
                  "When agents can resolve structured actions, your links do more than send traffic. They help complete the task.",
                code: [
                  ["resolved", "1"],
                  ["completed", "1"],
                  ["status", "\"scheduled\""],
                ],
                comment: "// outcome, not just traffic",
              },
            ].map((card, index) => (
              <RevealOnView key={card.heading} delay={0.05 * (index + 1)}>
                <div className="bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors flex flex-col">
                  <div className="flex items-center gap-2 mb-5">
                    <div className={`size-8 rounded-lg flex items-center justify-center ${card.tone}`}>
                      <span className="text-sm">{card.step}</span>
                    </div>
                    <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">{card.label}</span>
                  </div>
                  <h3 className="text-lg font-medium mb-3">{card.heading}</h3>
                  <p className="text-sm text-[#71717a] leading-relaxed mb-5">{card.copy}</p>
                  <div className="mt-auto bg-[#0c0c0e] border border-[#1e1e22] rounded-lg p-4 font-mono text-[12px] space-y-1">
                    <div className="text-[#3f3f46]">{card.comment}</div>
                    {card.code.map(([key, value]) => (
                      <div key={key}>
                        <span className="syn-key">{key}</span>
                        <span className="text-[#52525b]">: </span>
                        <span className={value.startsWith("\"") ? "syn-str" : "syn-num"}>{value}</span>
                      </div>
                    ))}
                  </div>
                </div>
              </RevealOnView>
            ))}
          </div>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      <section className="py-24 px-6 content-auto-section">
        <div className="mx-auto max-w-6xl">
          <RevealOnView>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">Why Rift</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-4">Most link platforms are expensive, bloated, and painful to build on</h2>
            <p className="text-[#71717a] mb-12 max-w-lg">
              If you&apos;ve fought heavyweight link tooling before, Rift is the alternative:
              simpler to integrate, easier to reason about, and ready for both human
              traffic today and agent-driven workflows over time.
            </p>
          </RevealOnView>

          <RevealOnView delay={0.1}>
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
                  {COMPARISON_ROWS.map((row, index) => (
                    <tr key={row.feature} className={index % 2 === 0 ? "bg-[#111113]" : "bg-[#0e0e10]"}>
                      <td className="px-5 py-3 text-[#a1a1aa] border-b border-[#222225]/50">{row.feature}</td>
                      {[row.bitly, row.branch, row.rift].map((value, valueIndex) => (
                        <td key={`${row.feature}-${valueIndex}`} className="text-center px-5 py-3 border-b border-[#222225]/50">
                          {value === true ? (
                            <span className={valueIndex === 2 ? "text-[#2dd4bf]" : "text-[#52525b]"}>✓</span>
                          ) : value === false ? (
                            <span className="text-[#2a2a2d]">✕</span>
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
          </RevealOnView>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      <section className="py-24 px-6 content-auto-section">
        <div className="mx-auto max-w-6xl">
          <RevealOnView>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">Under the hood</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-12">API-first link infrastructure</h2>
          </RevealOnView>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
            <RevealOnView delay={0.05}>
              <div className={`bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors ${FEATURE_CARDS[0].layout}`}>
                <div className="flex items-center gap-2 mb-4">
                  <div className={`size-2 rounded-full ${FEATURE_CARDS[0].tone}`} />
                  <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">{FEATURE_CARDS[0].title}</span>
                </div>
                <h3 className="text-lg font-medium mb-2">{FEATURE_CARDS[0].heading}</h3>
                <p className="text-sm text-[#71717a] leading-relaxed max-w-md">
                  Create <code className="text-[#a1a1aa] bg-[#18181b] px-1 rounded">/r/summer-launch</code> or let us
                  generate <code className="text-[#a1a1aa] bg-[#18181b] px-1 rounded">/r/A7F3B2C1</code>.
                  Attach a destination URL, arbitrary JSON metadata, and campaign tracking — all in one POST.
                  Works on iOS, Android, and web.
                </p>
              </div>
            </RevealOnView>

            <RevealOnView delay={0.1}>
              <div className={`bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors flex flex-col ${FEATURE_CARDS[1].layout}`}>
                <div className="flex items-center gap-2 mb-4">
                  <div className={`size-2 rounded-full ${FEATURE_CARDS[1].tone}`} />
                  <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">{FEATURE_CARDS[1].title}</span>
                </div>
                <h3 className="text-lg font-medium mb-2">{FEATURE_CARDS[1].heading}</h3>
                <p className="text-sm text-[#71717a] leading-relaxed mb-6">{FEATURE_CARDS[1].copy}</p>
                <div className="mt-auto bg-[#0c0c0e] border border-[#1e1e22] rounded-lg p-4 font-mono text-[12px] space-y-1.5">
                  <div><span className="syn-key">key</span><span className="text-[#52525b]"> → </span><span className="syn-str">rl_live_...</span></div>
                  <div><span className="syn-key">links</span><span className="text-[#52525b]"> → </span><span className="syn-num">isolated</span></div>
                  <div><span className="syn-key">clicks</span><span className="text-[#52525b]"> → </span><span className="syn-num">isolated</span></div>
                  <div><span className="syn-key">attribution</span><span className="text-[#52525b]"> → </span><span className="syn-num">isolated</span></div>
                </div>
              </div>
            </RevealOnView>

            {FEATURE_CARDS.slice(2).map((card, index) => (
              <RevealOnView key={card.heading} delay={0.15 + index * 0.05}>
                <div className="bg-[#111113] border border-[#222225] rounded-xl p-7 hover:border-[#2dd4bf]/20 transition-colors">
                  <div className="flex items-center gap-2 mb-4">
                    <div className={`size-2 rounded-full ${card.tone}`} />
                    <span className="text-[12px] font-mono text-[#52525b] uppercase tracking-wider">{card.title}</span>
                  </div>
                  <h3 className="text-lg font-medium mb-2">{card.heading}</h3>
                  <p className="text-sm text-[#71717a] leading-relaxed">{card.copy}</p>
                </div>
              </RevealOnView>
            ))}
          </div>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      <section className="py-24 px-6 content-auto-section">
        <div className="mx-auto max-w-6xl">
          <RevealOnView>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">API</p>
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-12">Five steps to a working link.</h2>
          </RevealOnView>

          <div className="space-y-2">
            {API_STEPS.map((endpoint, index) => (
              <RevealOnView key={`${endpoint.method}-${endpoint.path}`} delay={index * 0.05}>
                <div className="flex items-center gap-4 bg-[#111113] border border-[#222225] rounded-lg px-5 py-3.5 hover:border-[#2dd4bf]/20 transition-colors group">
                  <span
                    className={`font-mono text-[12px] font-medium w-12 ${
                      endpoint.method === "POST"
                        ? "text-[#34d399]"
                        : endpoint.method === "GET"
                          ? "text-[#60a5fa]"
                          : "text-[#f59e0b]"
                    }`}
                  >
                    {endpoint.method}
                  </span>
                  <span className="font-mono text-[13px] text-[#fafafa] min-w-[200px]">{endpoint.path}</span>
                  <span className="text-[13px] text-[#52525b] flex-1">{endpoint.desc}</span>
                  {endpoint.auth ? (
                    <span className="text-[11px] font-mono text-[#2dd4bf]/60 border border-[#2dd4bf]/20 rounded px-2 py-0.5">auth</span>
                  ) : (
                    <span className="text-[11px] font-mono text-[#52525b] border border-[#222225] rounded px-2 py-0.5">public</span>
                  )}
                </div>
              </RevealOnView>
            ))}
          </div>
        </div>
      </section>

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      <PricingSection />

      <div className="mx-auto max-w-6xl px-6"><div className="gradient-line" /></div>

      <section className="py-24 px-6 content-auto-section">
        <div className="mx-auto max-w-6xl">
          <RevealOnView className="text-center">
            <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-4">
              Create your first link in <span className="text-[#2dd4bf]">30 seconds</span>
            </h2>
            <p className="text-[#71717a] mb-8 max-w-md mx-auto">
              Sign up, get an API key, and POST to /v1/links. That&apos;s it.
            </p>
            <div className="flex items-center justify-center gap-4">
              <a
                href="/docs"
                className="inline-flex items-center gap-2 bg-[#2dd4bf] text-[#042f2e] text-sm font-medium px-6 py-2.5 rounded-lg hover:bg-[#5eead4] transition-colors glow-teal-sm"
              >
                Create your first link
              </a>
              <a
                href="/docs"
                className="inline-flex items-center gap-2 text-sm text-[#71717a] hover:text-[#fafafa] transition-colors"
              >
                Read quickstart <span className="text-[#3f3f46]">→</span>
              </a>
            </div>
          </RevealOnView>
        </div>
      </section>
    </>
  );
}
