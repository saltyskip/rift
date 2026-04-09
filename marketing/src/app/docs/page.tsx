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
          <CodeBlock lang="bash">{`cargo install --git https://github.com/saltyskip/rift.git rift-cli`}</CodeBlock>
        </Callout>

        <Callout eyebrow="Advanced" title="Doing this manually?">
          <p>
            If you prefer raw API calls and manual Cloudflare setup, use the{" "}
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
              Rift then guides you through your primary branded domain, tests the Worker setup, and
              can continue straight into your alternate domain for stronger Open in App behavior.
            </p>
            <CodeBlock lang="bash">{`rift domains setup`}</CodeBlock>
            <p>
              If you want the Cloudflare details behind this step, read{" "}
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
            You now have a working setup on Rift&apos;s shared domain. Pick your next goal — most
            developers start with custom domains or an SDK integration.
          </p>
          <div className="grid gap-3">
            {[
              {
                title: "Custom Domains",
                desc: "Set up branded links on your own domain.",
                href: "/docs/domains",
              },
              {
                title: "Register Your App",
                desc: "Add iOS/Android for universal links and branded landing pages.",
                href: "/docs/apps",
              },
              {
                title: "Publishable Keys",
                desc: "Create client-safe keys for the web and mobile SDKs.",
                href: "/docs/publishable-keys",
              },
              {
                title: "Create Links",
                desc: "Create deep links with per-platform routing and metadata.",
                href: "/docs/links",
              },
              {
                title: "Web SDK",
                desc: "Add click tracking and attribution to your website.",
                href: "/docs/web-sdk",
              },
              {
                title: "iOS SDK",
                desc: "Integrate deep linking into your iOS app.",
                href: "/docs/ios-sdk",
              },
              {
                title: "Android SDK",
                desc: "Integrate deep linking into your Android app.",
                href: "/docs/android-sdk",
              },
            ].map((item) => (
              <a
                key={item.title}
                href={item.href}
                className="group flex items-center justify-between rounded-xl border border-[#1e1e22] bg-[#111113] p-4 transition-colors hover:border-[#2dd4bf]/30"
              >
                <div>
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
