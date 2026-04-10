import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";

export const metadata: Metadata = {
  title: "Manual Setup — Rift Docs",
  description: "Set up Rift by hand with API calls and manual DNS configuration.",
};

export default function ManualSetupPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="mb-3 text-[13px] font-medium uppercase tracking-widest text-[#2dd4bf]">
          Documentation
        </p>
        <h1 className="mb-4 text-4xl font-bold text-[#fafafa]">Manual Setup</h1>
        <p className="text-lg leading-relaxed text-[#71717a]">
          This path is for teams who want to drive Rift with raw API calls and handle DNS
          manually. If you want the guided path, go back to the{" "}
          <a href="/docs" className="text-[#2dd4bf] hover:underline">
            Quick Start
          </a>
          .
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <Step n={1} title="Create your account">
            <p>Sign up with your email to receive a verification link.</p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/signup \\
  -H "Content-Type: application/json" \\
  -d '{"email": "you@example.com"}'`}</CodeBlock>
          </Step>

          <Step n={2} title="Verify your email and save your secret key">
            <p>
              Click the verification link in your inbox. Rift shows your secret key once in the
              browser. Save it immediately. It starts with{" "}
              <code className="rounded bg-[#2dd4bf]/10 px-1.5 py-0.5 text-[13px] text-[#2dd4bf]">
                rl_live_
              </code>
              .
            </p>
          </Step>

          <Step n={3} title="Set up your custom domain">
            <p>
              Before you create a publishable key, you need a verified primary domain
              with a CNAME pointing to Rift.
            </p>
            <p>
              Use the{" "}
              <a href="/docs/domains" className="text-[#2dd4bf] hover:underline">
                Custom Domains guide
              </a>{" "}
              for the full DNS and verification steps.
            </p>
          </Step>

          <Step n={4} title="Create a publishable key">
            <p>
              Publishable keys are safe to use in the web and mobile SDKs for click tracking and
              attribution.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/publishable-keys \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
          </Step>

          <Step n={5} title="Create your first link">
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/links \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "web_url": "https://example.com",
    "ios_deep_link": "myapp://product/123",
    "agent_context": {
      "action": "purchase",
      "cta": "Buy now",
      "description": "Premium widget, 50% off today"
    }
  }'`}</CodeBlock>
          </Step>

          <Step n={6} title="Resolve and inspect the link">
            <CodeBlock>{`curl https://go.yourcompany.com/LINK_ID \\
  -H "Accept: application/json"`}</CodeBlock>
          </Step>
        </section>
      </div>
    </div>
  );
}
