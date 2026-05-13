import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";

export const metadata: Metadata = {
  title: "Conversions — Rift Docs",
  description:
    "Track post-install events like signups, purchases, and deposits. Backend-only, attributed to the originating link.",
  alternates: { canonical: "/docs/conversions" },
};

export default function ConversionsPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">
          Tracking
        </p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Conversions</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Measure post-install events — signups, deposits, purchases, shares — and attribute
          them back to the link that drove them. Two paths:{" "}
          <strong className="text-[#fafafa]">SDK tracking</strong> from your mobile app, or{" "}
          <strong className="text-[#fafafa]">backend webhooks</strong> from your server.
          Both are attributed, deduped, and rolled up into your link stats.
        </p>
      </div>

      <div className="space-y-10">
        {/* How it works */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">How it works</h2>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="rounded-xl border border-[#2dd4bf]/20 bg-[#2dd4bf]/5 p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
                SDK tracking
              </p>
              <p className="mt-2 text-[13px] leading-relaxed text-[#71717a]">
                Call <code className="text-[#71717a] bg-[#18181b] px-1 py-0.5 rounded text-[11px]">trackConversion</code>{" "}
                from your iOS or Android app. The SDK handles auth (publishable key),
                user resolution, and the HTTP call. Best for client-side events like
                trades, purchases, or in-app actions.
              </p>
            </div>
            <div className="rounded-xl border border-[#1e1e22] bg-[#111113] p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#f59e0b]">
                Backend webhooks
              </p>
              <p className="mt-2 text-[13px] leading-relaxed text-[#71717a]">
                POST events to a source webhook URL from your server. Best for
                server-side events like Stripe payments, RevenueCat callbacks, or
                admin actions where the SDK isn&apos;t involved.
              </p>
            </div>
          </div>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Both paths resolve{" "}
            <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">user_id</code>{" "}
            → attribution → link, dedupe via idempotency key, and roll counts into the
            link stats endpoint.
          </p>
        </section>

        <div className="gradient-line" />

        {/* SDK tracking */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Option A: SDK tracking</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            The fastest path. The mobile SDK handles auth, user resolution, and the HTTP call.
            One line wherever a conversion happens.
          </p>

          <Step n={1} title="Bind the user (prerequisite)">
            <p>
              Before you can attribute conversions, call{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">setUserId</code>{" "}
              wherever you handle your user session. See the{" "}
              <a href="/docs/attribution" className="text-[#2dd4bf] hover:underline">Attribution</a>{" "}
              docs for the full pattern.
            </p>
            <CodeBlock lang="swift">{`// iOS
try? await rift.setUserId(userId: currentUser.id)`}</CodeBlock>
            <CodeBlock lang="kotlin">{`// Android
rift.setUserId(userId = currentUser.id)`}</CodeBlock>
          </Step>

          <Step n={2} title="Track a conversion">
            <p>
              Call <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">trackConversion</code>{" "}
              whenever a user does something worth counting. The SDK reads the bound user,
              authenticates with your publishable key, and POSTs to{" "}
              <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">POST /v1/attribution/convert</code>.
            </p>
            <CodeBlock lang="swift">{`// iOS — on trade completion, purchase, signup:
try await rift.trackConversion(
    conversionType: "trade",
    idempotencyKey: orderId,
    metadata: ["asset": "ETH", "side": "buy"]
)`}</CodeBlock>
            <CodeBlock lang="kotlin">{`// Android
rift.trackConversion(
    conversionType = "trade",
    idempotencyKey = orderId,
    metadata = mapOf("asset" to "ETH", "side" to "buy")
)`}</CodeBlock>
            <Callout type="info">
              The server dedupes via <code>idempotencyKey</code>, so retries are safe.
              If no <code>user_id</code> is bound, the SDK logs a warning and skips the call.
            </Callout>
          </Step>

          <Step n={3} title="Check your stats">
            <p>
              Conversion counts roll up into the link stats endpoint immediately:
            </p>
            <CodeBlock>{`curl https://api.riftl.ink/v1/links/summer-sale/stats \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        {/* Webhook tracking */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Option B: Backend webhooks</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            For server-side events — Stripe webhooks, RevenueCat callbacks, admin actions —
            POST directly to a source webhook URL. No SDK involved, no client-side code.
          </p>

          <Step n={1} title="Get your webhook URL">
            <p>
              Your tenant&apos;s default custom source is auto-provisioned on first request. List
              your sources to get the URL:
            </p>
            <CodeBlock>{`curl https://api.riftl.ink/v1/sources \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <CodeBlock lang="json">{`{
  "sources": [
    {
      "id": "66a1b2c3d4e5f6a7b8c9d0e1",
      "name": "default",
      "source_type": "custom",
      "webhook_url": "https://api.riftl.ink/w/a1b2c3d4e5f6...",
      "created_at": "2026-04-10T12:00:00Z"
    }
  ]
}`}</CodeBlock>
            <p>
              The <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">webhook_url</code>{" "}
              is the opaque, unguessable URL your backend POSTs events to. Treat it like a secret.
              To rotate it, delete the source and create a new one.
            </p>
          </Step>

          <Step n={2} title="Bind a user to a link (prerequisite)">
            <p>
              Before you can attribute conversions, each user needs an attribution record
              linking them back to the install that originally drove them. The mobile SDK
              handles this in one line — see the{" "}
              <a href="/docs/attribution" className="text-[#2dd4bf] hover:underline">
                Attribution
              </a>{" "}
              doc for the full pattern:
            </p>
            <CodeBlock lang="swift">{`// iOS
try? await rift.setUserId("usr_abc123")`}</CodeBlock>
            <CodeBlock lang="kotlin">{`// Android
rift.setUserId("usr_abc123")`}</CodeBlock>
            <p>
              The SDK persists the binding locally and syncs it to the server, retrying
              automatically on the next app launch if the network call fails.
            </p>
            <Callout type="info">
              Conversions fired with a <code>user_id</code> that has no matching attribution
              record are silently dropped (the webhook still returns 200, but the event is
              not counted toward your link stats). Make sure your app calls{" "}
              <code>setUserId</code> before you start firing conversions for that user.
            </Callout>
          </Step>

          <Step n={3} title="Fire a conversion">
            <p>
              POST to the source&apos;s webhook URL whenever a user does something worth counting.
              The only required fields are{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
                user_id
              </code>{" "}
              and{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
                type
              </code>
              . Idempotency key and metadata are optional.
            </p>
            <CodeBlock>{`curl -X POST https://api.riftl.ink/w/a1b2c3d4e5f6... \\
  -H "Content-Type: application/json" \\
  -d '{
    "user_id": "usr_abc123",
    "type": "deposit",
    "idempotency_key": "tx_550e8400-e29b",
    "metadata": { "tx_hash": "0xabc...", "amount": "100.00", "currency": "usd" }
  }'`}</CodeBlock>
            <p>
              The response tells you what Rift did with the batch:
            </p>
            <CodeBlock lang="json">{`{
  "accepted": 1,
  "deduped": 0,
  "unattributed": 0,
  "failed": 0
}`}</CodeBlock>
          </Step>

          <Step n={4} title="Check your stats">
            <p>
              The link stats endpoint now returns a{" "}
              <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
                conversions
              </code>{" "}
              array with counts grouped by type:
            </p>
            <CodeBlock>{`curl https://api.riftl.ink/v1/links/summer-sale/stats \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <CodeBlock lang="json">{`{
  "link_id": "summer-sale",
  "click_count": 1420,
  "install_count": 340,
  "identify_count": 198,
  "convert_count": 110,
  "conversions": [
    { "conversion_type": "deposit", "count": 19 },
    { "conversion_type": "signup", "count": 91 }
  ]
}`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        {/* Payload shape */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">The event payload</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Every custom-source event follows this shape:
          </p>
          <CodeBlock lang="json">{`{
  "user_id": "usr_abc123",           // required
  "type": "deposit",                 // required — free-form, up to 64 chars
  "idempotency_key": "tx_abc",       // optional, <=256 chars
  "metadata": { "tx_hash": "0x..." } // optional, <=1KB, stored verbatim
}`}</CodeBlock>

          <Callout type="info">
            Need to track revenue or amounts? Put them in <code>metadata</code> — Rift stores
            it verbatim and forwards it on the outbound webhook. Your warehouse handles the
            aggregation.
          </Callout>

          <div className="rounded-xl border border-[#2dd4bf]/20 bg-[#2dd4bf]/5 p-4">
            <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf] mb-2">
              Idempotency key contract
            </p>
            <ul className="list-disc pl-5 space-y-1 text-[13px] text-[#d4d4d8]">
              <li>
                <strong className="text-[#fafafa]">Scoped per tenant</strong> — two tenants can
                use the same key without collision.
              </li>
              <li>
                <strong className="text-[#fafafa]">Unique within a 30-day window</strong> — after
                TTL expiry, keys may be safely reused.
              </li>
              <li>
                <strong className="text-[#fafafa]">Opaque to Rift</strong> — any string up to 256
                characters, not parsed or validated.
              </li>
              <li>
                <strong className="text-[#fafafa]">Collision behavior</strong> — Rift silently
                drops duplicates and returns 200, so your retry logic stays happy. The event is
                not double-counted.
              </li>
              <li>
                <strong className="text-[#fafafa]">Typical values</strong> — on-chain transaction
                hash, order ID, your DB transaction UUID.
              </li>
              <li>
                <strong className="text-[#fafafa]">Optional</strong> — if you omit it, every
                call counts. That&apos;s fine for events where double-counting doesn&apos;t
                matter (e.g. content views), but use a key for anything you need exact counts on.
              </li>
            </ul>
          </div>
        </section>

        <div className="gradient-line" />

        {/* Managing sources */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Managing sources</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            The default custom source handles the common case — one pipe from your backend to
            Rift. If you want to segment events by origin (e.g. &ldquo;backend-deposits&rdquo; vs
            &ldquo;admin-overrides&rdquo;), create additional custom sources explicitly.
          </p>

          <Step n={1} title="Create a source">
            <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/sources \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "name": "backend-deposits",
    "source_type": "custom"
  }'`}</CodeBlock>
          </Step>

          <Step n={2} title="Get one source">
            <CodeBlock>{`curl https://api.riftl.ink/v1/sources/SOURCE_ID \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
          </Step>

          <Step n={3} title="Delete a source">
            <p>
              Historical events for the deleted source remain queryable via the link stats
              endpoint — they still carry the <code>source_id</code> reference even after the
              source document is gone. There is no rotate endpoint; to rotate a webhook URL,
              delete the source and create a new one.
            </p>
            <CodeBlock>{`curl -X DELETE https://api.riftl.ink/v1/sources/SOURCE_ID \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        {/* Webhook delivery */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Outbound webhook delivery</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Every conversion fires an outbound{" "}
            <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
              conversion
            </code>{" "}
            webhook to any registered webhook subscribed to that event type. The payload
            includes a stable{" "}
            <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
              event_id
            </code>{" "}
            — use it as an idempotency key in your handler to safely dedupe delivery retries.
            See the{" "}
            <a href="/docs/webhooks" className="text-[#2dd4bf] hover:underline">
              Webhooks doc
            </a>{" "}
            for the full payload shape and signature verification.
          </p>
          <Callout type="info">
            Webhook delivery is best-effort with 4 retries. For zero-loss reconciliation, poll{" "}
            <code>GET /v1/links/{"{link_id}"}/stats</code> on a schedule — events are the
            durable source of truth inside Rift&apos;s store. The webhook is a push notification
            for convenience, not the canonical data path.
          </Callout>
        </section>

        <div className="gradient-line" />

        {/* Scope */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">What Rift answers</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Rift&apos;s conversion API is deliberately bounded. It answers one class of question
            well and refuses the rest. If a question starts with <em>&ldquo;which link&rdquo;</em>,
            it&apos;s in scope. If it starts with <em>&ldquo;which user&rdquo;</em>, that&apos;s
            your warehouse&apos;s job — pipe events via webhook.
          </p>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="rounded-xl border border-[#2dd4bf]/20 bg-[#2dd4bf]/5 p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
                In scope
              </p>
              <ul className="mt-2 list-disc pl-5 space-y-1 text-[13px] text-[#d4d4d8]">
                <li>Total count per link, per conversion type</li>
                <li>Conversion attribution tied to the originating link</li>
                <li>Idempotent event ingestion with at-least-once delivery</li>
                <li>Outbound webhooks for streaming events to your warehouse</li>
              </ul>
            </div>
            <div className="rounded-xl border border-[#1e1e22] bg-[#111113] p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#71717a]">
                Out of scope
              </p>
              <ul className="mt-2 list-disc pl-5 space-y-1 text-[13px] text-[#71717a]">
                <li>User-level queries (cohorts, funnels, retention)</li>
                <li>Filtering or grouping by metadata fields</li>
                <li>Multi-event behavioral sequences</li>
                <li>Per-event drill-down from the API</li>
              </ul>
            </div>
          </div>
          <p className="text-[13px] text-[#52525b] leading-relaxed">
            Metadata is stored verbatim and forwarded on the outbound webhook, but it&apos;s
            never indexed or queried inside Rift. Use it for your own debugging and warehouse
            joins.
          </p>
        </section>
      </div>
    </div>
  );
}
