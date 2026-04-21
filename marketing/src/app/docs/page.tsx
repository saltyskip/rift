import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsCalloutCard as Callout } from "@/components/docs-callout";
import { DocsStep as Step } from "@/components/docs-step";
import { QuickstartOutcomeDiagram } from "@/components/quickstart-outcome-diagram";

export const metadata: Metadata = {
  title: "Quick Start — Rift Docs",
  description: "Get your first Rift link live with the CLI in a few minutes.",
};

export default function QuickStartPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="mb-3 text-[13px] font-medium uppercase tracking-widest text-[#2dd4bf]">
          Documentation
        </p>
        <h1 className="mb-4 text-4xl font-bold text-[#fafafa]">Quick Start</h1>
        <p className="text-lg leading-relaxed text-[#71717a]">
          The fastest way to get Rift working is through the CLI. It walks you from account creation
          to branded domains, health checks, and your first real link.
        </p>
      </div>

      <div className="space-y-10">
        <QuickstartOutcomeDiagram />

        <div className="gradient-line" />

        <Callout eyebrow="Recommended" title="Use the CLI">
          <p>
            This is the best path if you want the smoothest setup. Rift guides you through signup,
            custom domains, diagnostics, and first success without making you remember the whole API.
          </p>
          <CodeBlock lang="bash">{`curl -fsSL https://raw.githubusercontent.com/saltyskip/rift/main/client/cli/install.sh | sh`}</CodeBlock>
          <p className="mt-2 text-[13px] text-[#52525b]">
            Or install from source: <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[12px]">cargo install --git https://github.com/saltyskip/rift.git rift-cli</code>
          </p>
        </Callout>

        <Callout eyebrow="Advanced" title="Doing this manually?">
          <p>
            If you prefer raw API calls and manual DNS setup, use the{" "}
            <a href="/docs/manual-setup" className="text-[#2dd4bf] hover:underline">
              manual setup guide
            </a>
            . That path is better for operators who want to script everything or understand every
            infrastructure step.
          </p>
        </Callout>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">CLI flow</h2>

          <Step n={1} title="Create your account and local config">
            <p>
              Start with{" "}
              <code className="rounded bg-[#2dd4bf]/10 px-1.5 py-0.5 text-[13px] text-[#2dd4bf]">
                rift init
              </code>
              . It verifies your email, saves your secret key locally, and helps you create a first
              starter link.
            </p>
            <CodeBlock lang="bash">{`rift init`}</CodeBlock>
          </Step>

          <Step n={2} title="Set up your branded domain">
            <p>
              Rift guides you through your primary branded domain, verifies DNS, and
              can continue straight into your alternate domain for stronger Open in App behavior.
            </p>
            <CodeBlock lang="bash">{`rift domains setup`}</CodeBlock>
            <p>
              If you want the DNS details behind this step, read{" "}
              <a href="/docs/domains" className="text-[#2dd4bf] hover:underline">
                Custom Domains
              </a>
              .
            </p>
          </Step>

          <Step n={3} title="Check what is ready">
            <p>
              Once the domain flow is done,{" "}
              <code className="rounded bg-[#2dd4bf]/10 px-1.5 py-0.5 text-[13px] text-[#2dd4bf]">
                rift doctor
              </code>{" "}
              tells you what you can already do and what is still worth finishing before production.
            </p>
            <CodeBlock lang="bash">{`rift doctor`}</CodeBlock>
          </Step>

          <Step n={4} title="Create and inspect links">
            <p>
              After onboarding, create a real link and test how it resolves across web, iOS, and
              Android.
            </p>
            <CodeBlock lang="bash">{`rift links create
rift links test LINK_ID`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Next steps</h2>
          <p className="text-[15px] text-[#71717a]">
            You now have a working setup on Rift&apos;s shared domain. The rest of the docs
            follow the value chain: set up your infrastructure, create links, acquire users,
            then measure conversions.
          </p>
          <div className="grid gap-3">
            {[
              {
                title: "Custom Domains",
                desc: "Brand your links on your own domain.",
                href: "/docs/domains",
                eyebrow: "Setup",
              },
              {
                title: "Register Your App",
                desc: "Add iOS/Android for universal links and landing pages.",
                href: "/docs/apps",
                eyebrow: "Setup",
              },
              {
                title: "Create Links",
                desc: "Deep links with per-platform routing and metadata.",
                href: "/docs/links",
                eyebrow: "Create",
              },
              {
                title: "Web SDK",
                desc: "Track clicks and copy link IDs to clipboard for deferred deep linking.",
                href: "/docs/web-sdk",
                eyebrow: "Acquire",
              },
              {
                title: "iOS SDK",
                desc: "User binding, conversion tracking, and deferred deep links.",
                href: "/docs/ios-sdk",
                eyebrow: "Convert",
              },
              {
                title: "Android SDK",
                desc: "User binding, conversion tracking, and deferred deep links.",
                href: "/docs/android-sdk",
                eyebrow: "Convert",
              },
              {
                title: "Conversions",
                desc: "Measure signups, purchases, and revenue — from the SDK or your backend.",
                href: "/docs/conversions",
                eyebrow: "Convert",
              },
            ].map((item) => (
              <a
                key={item.title}
                href={item.href}
                className="group flex items-center justify-between rounded-xl border border-[#1e1e22] bg-[#111113] p-4 transition-colors hover:border-[#2dd4bf]/30"
              >
                <div>
                  {"eyebrow" in item && (
                    <p className="text-[10px] font-semibold uppercase tracking-[0.2em] text-[#52525b] mb-1">
                      {(item as { eyebrow: string }).eyebrow}
                    </p>
                  )}
                  <p className="text-[15px] font-medium text-[#fafafa] transition-colors group-hover:text-[#2dd4bf]">
                    {item.title}
                  </p>
                  <p className="text-[13px] text-[#52525b]">{item.desc}</p>
                </div>
                <span className="text-[#3f3f46] transition-colors group-hover:text-[#2dd4bf]">
                  &rarr;
                </span>
              </a>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
