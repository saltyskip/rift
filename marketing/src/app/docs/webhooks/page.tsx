import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

export const metadata: Metadata = {
  title: "Webhooks — Rift Docs",
  description: "Receive real-time notifications for click and attribution events via HTTPS webhooks.",
};

function Step({ n, title, children }: { n: number; title: string; children: React.ReactNode }) {
  return (
    <div className="relative pl-10">
      <div className="absolute left-0 top-0 flex h-7 w-7 items-center justify-center rounded-full bg-[#2dd4bf]/10 text-[#2dd4bf] text-sm font-semibold border border-[#2dd4bf]/20">
        {n}
      </div>
      <h3 className="text-lg font-semibold text-[#fafafa] mb-3">{title}</h3>
      <div className="space-y-3 text-[15px] text-[#a1a1aa] leading-relaxed">{children}</div>
    </div>
  );
}

function Callout({ type, children }: { type: "info" | "warning"; children: React.ReactNode }) {
  const styles = {
    info: "border-[#60a5fa]/30 bg-[#60a5fa]/5 text-[#93bbfd]",
    warning: "border-[#f59e0b]/30 bg-[#f59e0b]/5 text-[#fbbf24]",
  };
  const labels = { info: "Note", warning: "Important" };
  return (
    <div className={`rounded-lg border px-4 py-3 text-[13px] leading-relaxed ${styles[type]}`}>
      <strong>{labels[type]}:</strong> {children}
    </div>
  );
}

export default function WebhooksPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Tracking</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Webhooks</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Receive real-time notifications when users click your links or when installs are attributed.
          Push events to Slack, your CRM, or any analytics pipeline.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Setup</h2>

          <Step n={1} title="Register a webhook">
            <p>Provide an HTTPS URL and the event types you want to receive:</p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/webhooks \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "url": "https://yourserver.com/rift-webhook",
    "events": ["click", "attribution"]
  }'`}</CodeBlock>
            <p>Response:</p>
            <CodeBlock lang="json">{`{
  "id": "6650a1b2c3d4e5f6a7b8c9d0",
  "url": "https://yourserver.com/rift-webhook",
  "events": ["click", "attribution"],
  "secret": "a1b2c3d4...64-char-hex-string",
  "created_at": "2026-03-24T12:00:00Z"
}`}</CodeBlock>
            <Callout type="warning">
              Save the <code>secret</code> immediately — it is only returned once at creation time.
              You&apos;ll use it to verify webhook signatures.
            </Callout>
          </Step>

          <Step n={2} title="List your webhooks">
            <CodeBlock>{`curl https://api.riftl.ink/v1/webhooks \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <p>
              The list response omits the <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">secret</code> field
              for security.
            </p>
          </Step>

          <Step n={3} title="Delete a webhook">
            <CodeBlock>{`curl -X DELETE https://api.riftl.ink/v1/webhooks/WEBHOOK_ID \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Event payloads</h2>

          <Step n={4} title="Click event">
            <p>Sent when a user clicks or resolves one of your links:</p>
            <CodeBlock lang="json">{`{
  "event": "click",
  "timestamp": "2026-03-24T15:00:00Z",
  "data": {
    "tenant_id": "6650a1b2c3d4e5f6a7b8c9d0",
    "link_id": "summer-sale",
    "user_agent": "Mozilla/5.0 ...",
    "referer": "https://twitter.com",
    "platform": "ios",
    "timestamp": "2026-03-24T15:00:00Z"
  }
}`}</CodeBlock>
          </Step>

          <Step n={5} title="Attribution event">
            <p>Sent when an install is attributed to one of your links:</p>
            <CodeBlock lang="json">{`{
  "event": "attribution",
  "timestamp": "2026-03-24T15:05:00Z",
  "data": {
    "tenant_id": "6650a1b2c3d4e5f6a7b8c9d0",
    "link_id": "summer-sale",
    "install_id": "device-uuid-123",
    "app_version": "1.2.0",
    "timestamp": "2026-03-24T15:05:00Z"
  }
}`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Verifying signatures</h2>

          <Step n={6} title="Validate the HMAC signature">
            <p>
              Every webhook request includes an{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">X-Rift-Signature</code>{" "}
              header containing an HMAC-SHA256 hex digest of the raw request body, signed with your webhook secret.
            </p>
            <CodeBlock lang="python">{`import hmac, hashlib

def verify_webhook(body: bytes, signature: str, secret: str) -> bool:
    expected = hmac.new(
        secret.encode(),
        body,
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(expected, signature)`}</CodeBlock>
            <CodeBlock lang="javascript">{`const crypto = require("crypto");

function verifyWebhook(body, signature, secret) {
  const expected = crypto
    .createHmac("sha256", secret)
    .update(body)
    .digest("hex");
  return crypto.timingSafeEqual(
    Buffer.from(expected),
    Buffer.from(signature)
  );
}`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Limits & retry behavior</h2>
          <ul className="list-disc pl-5 space-y-2 text-[15px] text-[#a1a1aa]">
            <li>Maximum <strong className="text-[#fafafa]">2 webhooks</strong> per tenant.</li>
            <li>Webhook URLs must use <strong className="text-[#fafafa]">HTTPS</strong>.</li>
            <li>
              Failed deliveries are retried <strong className="text-[#fafafa]">4 times</strong> with
              exponential backoff (0s, 1s, 5s, 25s).
            </li>
            <li>Delivery timeout is <strong className="text-[#fafafa]">10 seconds</strong> per attempt.</li>
            <li>Delivery is fire-and-forget — it does not block the API response to the original request.</li>
          </ul>
        </section>
      </div>
    </div>
  );
}
