import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";
import { DocsSetupTabs } from "@/components/docs-setup-tabs";
import { DocsStep as Step } from "@/components/docs-step";
import { DocsCallout as Callout } from "@/components/docs-callout";
import { LifecycleFlow } from "@/components/lifecycle-flow";

export const metadata: Metadata = {
  title: "Attribution — Rift Docs",
  description:
    "Track clicks, attribute installs to links, and measure conversions across web, iOS, and Android.",
  alternates: { canonical: "/docs/lifecycle" },
};

export default function AttributionPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">
          Tracking
        </p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Attribution</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Rift tracks the full funnel: click → install → identify → convert. The SDKs handle
          most of this automatically. Attribution endpoints use a{" "}
          <a
            href="/docs/publishable-keys"
            className="text-[#2dd4bf] hover:underline"
          >
            publishable key
          </a>{" "}
          (
          <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
            pk_live_
          </code>
          ).
        </p>
      </div>

      <div className="space-y-10">
        {/* How it works */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">How it works</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Four SDK calls map 1:1 to four HTTP endpoints and four webhook
            event types. Call them in funnel order as the user moves
            through your product — Rift records each step, materializes
            the install state, and fires a webhook your backend can react
            to.
          </p>
          <LifecycleFlow />
          <p className="text-[13px] text-[#71717a] leading-relaxed">
            Solid arrows are synchronous: each SDK call hits one endpoint,
            which writes to one (sometimes two) collections and dispatches
            one webhook. Dashed arrows are funnel order — they happen
            later in the user&apos;s journey, often on a different device
            or session. The four verbs are independent: you can call any
            of them at any time, in any order.
          </p>
        </section>

        <div className="gradient-line" />

        {/* Auto tracking */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">What&apos;s tracked automatically</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            Every time a user hits a Rift link — whether on the landing page, via a custom domain,
            or through the shared <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">riftl.ink</code> domain —
            Rift <strong className="text-[#fafafa]">records the click automatically</strong>. No SDK
            call is needed for this. The click captures platform, user agent, and referrer.
          </p>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            The SDK-based click recording (below) is for <em>additional</em> tracking — in-app
            link taps, programmatic navigation, or cases where you want the click response data.
          </p>
        </section>

        <div className="gradient-line" />

        {/* redirect=1 */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Direct redirect mode</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            By default, Rift serves a smart landing page that detects the user&apos;s platform and
            shows &ldquo;Open in App&rdquo; / store buttons. If you want to skip the landing page and
            send the user directly to their platform destination, add{" "}
            <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">?redirect=1</code> to
            the link URL:
          </p>
          <CodeBlock lang="bash">{`https://go.yourcompany.com/summer-sale?redirect=1`}</CodeBlock>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            This still records the click, then immediately redirects:
          </p>
          <div className="overflow-x-auto">
            <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
              <thead>
                <tr className="bg-[#0c0c0e]">
                  <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Platform</th>
                  <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Redirects to</th>
                </tr>
              </thead>
              <tbody className="text-[#a1a1aa]">
                <tr className="border-b border-[#1e1e22]">
                  <td className="px-4 py-2.5 font-medium text-[#7dd3fc]">iOS</td>
                  <td className="px-4 py-2.5">App Store URL</td>
                </tr>
                <tr className="border-b border-[#1e1e22]">
                  <td className="px-4 py-2.5 font-medium text-[#fbbf24]">Android</td>
                  <td className="px-4 py-2.5">
                    Play Store URL with <code className="text-[#71717a] bg-[#18181b] px-1 py-0.5 rounded text-[12px]">referrer=rift_link={"{link_id}"}</code> appended
                    for install attribution
                  </td>
                </tr>
                <tr>
                  <td className="px-4 py-2.5 font-medium text-[#f472b6]">Desktop</td>
                  <td className="px-4 py-2.5">Web URL</td>
                </tr>
              </tbody>
            </table>
          </div>
          <Callout type="info">
            Use <code>redirect=1</code> when you want the fastest path to the destination — for
            example, in email campaigns or QR codes where a landing page adds friction. The click
            is still tracked, but the user never sees the landing page.
          </Callout>
        </section>

        <div className="gradient-line" />

        {/* Click Reporting */}
        <DocsSetupTabs
          title="SDK click reporting"
          tabs={[
            {
              id: "web",
              label: "Web SDK",
              children: (
                <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
                  <p>
                    The Web SDK <strong className="text-[#fafafa]">auto-tracks clicks</strong> on
                    any <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">{"<a>"}</code> tag
                    pointing to your custom domain. No extra code needed after{" "}
                    <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">Rift.init()</code>.
                  </p>
                  <CodeBlock lang="typescript">{`// Auto-tracking is enabled by default after init
Rift.init("pk_live_YOUR_KEY", {
  domain: "go.yourcompany.com",
});

// For programmatic use (buttons, custom UI):
Rift.click("summer-sale");`}</CodeBlock>
                  <p>
                    On click, the SDK fires a{" "}
                    <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">sendBeacon</code> request
                    to <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">POST /v1/lifecycle/click</code> and
                    copies the link URL to the clipboard. The beacon is fire-and-forget — it
                    doesn&apos;t block navigation.
                  </p>
                  <Callout type="info">
                    The clipboard write is how iOS deferred deep linking works. When the user
                    installs the app and opens it, the iOS SDK reads the link ID from the
                    clipboard.
                  </Callout>
                </div>
              ),
            },
            {
              id: "ios",
              label: "iOS SDK",
              children: (
                <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
                  <p>
                    Record a click when your app opens a Rift link internally (e.g. from a
                    push notification or in-app banner):
                  </p>
                  <CodeBlock lang="swift">{`let result = try await rift.click(linkId: "summer-sale")
// result.iosDeepLink — the deep link to navigate to
// result.platform — detected platform`}</CodeBlock>
                  <p>
                    The response includes all link destinations and metadata, so you can
                    navigate the user immediately.
                  </p>
                  <Callout type="info">
                    For links tapped in Safari or other apps, the landing page handles click
                    recording automatically and stages the link ID via clipboard for
                    post-install attribution.
                  </Callout>
                </div>
              ),
            },
            {
              id: "android",
              label: "Android SDK",
              children: (
                <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
                  <p>
                    Record a click when your app opens a Rift link internally:
                  </p>
                  <CodeBlock lang="kotlin">{`val result = rift.click(linkId = "summer-sale")
// result.androidDeepLink — the deep link to navigate to
// result.platform — detected platform`}</CodeBlock>
                  <p>
                    For links tapped in a browser, the landing page handles click recording
                    and passes the link ID to the Play Store via the install referrer
                    parameter.
                  </p>
                </div>
              ),
            },
            {
              id: "http",
              label: "HTTP",
              children: (
                <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
                  <p>Record a click directly via the API:</p>
                  <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/lifecycle/click \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"link_id": "summer-sale"}'`}</CodeBlock>
                  <p>
                    The response includes the full link data (deep links, store URLs,
                    metadata) so you can route the user to the right destination.
                  </p>
                </div>
              ),
            },
          ]}
        />

        <div className="gradient-line" />

        {/* Install Attribution */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">
            Attribute the touch
          </h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
              attribute
            </code>{" "}
            records that an install touched a link — whether it&apos;s a
            fresh install whose first-touch you&apos;re capturing, or an
            already-signed-in user clicking a new campaign link. Every
            call appends an event; first-touch attribution stays sticky
            for stats. On mobile the SDK recovers the link ID after
            install — the mechanism differs by platform.
          </p>

          <div className="grid gap-3 sm:grid-cols-2">
            <div className="rounded-xl border border-[#1e1e22] bg-[#111113] p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#7dd3fc]">
                iOS — Clipboard
              </p>
              <p className="mt-2 text-[13px] leading-relaxed text-[#71717a]">
                The landing page (or Web SDK) writes the link URL to the
                clipboard. On first launch, the iOS SDK reads the clipboard,
                extracts the link ID, reports attribution, and clears the
                clipboard.
              </p>
            </div>
            <div className="rounded-xl border border-[#1e1e22] bg-[#111113] p-4">
              <p className="text-[12px] font-semibold uppercase tracking-[0.18em] text-[#fbbf24]">
                Android — Install Referrer
              </p>
              <p className="mt-2 text-[13px] leading-relaxed text-[#71717a]">
                The landing page appends{" "}
                <code className="text-[#71717a] bg-[#18181b] px-1 py-0.5 rounded text-[12px]">
                  rift_link={"{link_id}"}
                </code>{" "}
                to the Play Store URL. On first launch, the SDK reads the
                install referrer and reports attribution.
              </p>
            </div>
          </div>

          <DocsSetupTabs
            title="Report attribution"
            tabs={[
              {
                id: "ios",
                label: "iOS SDK",
                children: (
                  <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
                    <p>
                      The simplest path is{" "}
                      <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">checkDeferredDeepLink</code> —
                      it parses the clipboard, reports attribution, and returns the link data in one call:
                    </p>
                    <CodeBlock lang="swift">{`// On first app launch
if let result = try await rift.checkDeferredDeepLink(
    clipboardText: UIPasteboard.general.string
) {
    UIPasteboard.general.string = ""
    if let deepLink = result.iosDeepLink {
        handleDeepLink(deepLink)
    }
}`}</CodeBlock>
                    <p>
                      Or use the lower-level method if you need more control:
                    </p>
                    <CodeBlock lang="swift">{`let reported = try await rift.attributeLink(linkId: "summer-sale")`}</CodeBlock>
                  </div>
                ),
              },
              {
                id: "android",
                label: "Android SDK",
                children: (
                  <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
                    <p>
                      On first launch, read the install referrer and report
                      attribution:
                    </p>
                    <CodeBlock lang="kotlin">{`// Using the install referrer
val linkId = parseReferrerLink(referrerString)
if (linkId != null) {
    rift.attributeLink(linkId = linkId)
    val link = rift.getLink(linkId = linkId)
    link.androidDeepLink?.let { handleDeepLink(it) }
}`}</CodeBlock>
                  </div>
                ),
              },
              {
                id: "http",
                label: "HTTP",
                children: (
                  <div className="space-y-3 text-[15px] leading-relaxed text-[#a1a1aa]">
                    <p>Report an install attribution directly:</p>
                    <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/lifecycle/attribute \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "link_id": "summer-sale",
    "install_id": "device-uuid-here",
    "app_version": "1.0.0"
  }'`}</CodeBlock>
                    <Callout type="info">
                      Every <code>attribute</code> call appends an event.
                      First-touch attribution (the campaign that gets
                      credit in stats) is preserved on the install record
                      and not overwritten by subsequent calls — but the
                      webhook fires every time, so receivers can react to
                      re-attribution touchpoints.
                    </Callout>
                  </div>
                ),
              },
            ]}
          />
        </section>

        <div className="gradient-line" />

        {/* Identify */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Identify the user</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
              identify
            </code>{" "}
            is the join between &ldquo;an install touched my link&rdquo;
            and &ldquo;a real user did the valuable thing later.&rdquo; It
            unlocks the conversion-attribution path — every conversion
            you fire afterwards resolves{" "}
            <code className="text-[#71717a] bg-[#18181b] px-1 py-0.5 rounded text-[12px]">
              user_id → first_link_id
            </code>{" "}
            via the install record so credit lands on the right campaign.
            Subscribers receive the full triple{" "}
            <code className="text-[#71717a] bg-[#18181b] px-1 py-0.5 rounded text-[12px]">
              {"{user_id, link_id, link_metadata}"}
            </code>{" "}
            in the webhook, so you can react (grant entitlements, credit
            referrals, send a welcome bonus) without a follow-up lookup.
          </p>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            The mobile SDK persists this binding locally, sends it to the
            server, and silently retries on the next app launch if the
            network call fails. One line, wherever you already handle
            your user session:
          </p>

          <DocsSetupTabs
            title="Bind the user"
            tabs={[
              {
                id: "ios",
                label: "iOS",
                children: (
                  <div className="space-y-3">
                    <CodeBlock lang="swift">{`// Wherever you know the user is signed in
Task {
    try? await rift.setUserId("usr_abc123")
}`}</CodeBlock>
                    <p className="text-[13px] text-[#71717a] leading-relaxed">
                      <code className="text-[#71717a] bg-[#18181b] px-1 py-0.5 rounded text-[12px]">
                        setUserId
                      </code>{" "}
                      is idempotent — safe to call on every launch with the
                      same <code>user_id</code>. On iOS, the SDK persists the
                      binding in the Keychain, so the{" "}
                      <code>install_id</code> survives app reinstalls.
                    </p>
                  </div>
                ),
              },
              {
                id: "android",
                label: "Android",
                children: (
                  <div className="space-y-3">
                    <CodeBlock lang="kotlin">{`// Wherever you know the user is signed in
lifecycleScope.launch {
    runCatching { rift.setUserId("usr_abc123") }
}`}</CodeBlock>
                    <p className="text-[13px] text-[#71717a] leading-relaxed">
                      On Android, the SDK persists the binding in
                      <code className="text-[#71717a] bg-[#18181b] px-1 py-0.5 rounded text-[12px]">
                        SharedPreferences
                      </code>
                      . Android wipes app data on uninstall, so the{" "}
                      <code>install_id</code> does not survive reinstallation —
                      that's the OS contract, not a Rift limitation.
                    </p>
                  </div>
                ),
              },
              {
                id: "http",
                label: "HTTP (advanced)",
                children: (
                  <div className="space-y-3">
                    <p className="text-[13px] text-[#a1a1aa] leading-relaxed">
                      If you already have the <code>install_id</code> somewhere
                      (e.g. your backend received it from the mobile app at
                      signup time), you can also call the endpoint directly
                      with your publishable key:
                    </p>
                    <CodeBlock>{`curl -X PUT https://api.riftl.ink/v1/lifecycle/identify \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "install_id": "device-uuid-here",
    "user_id": "usr_abc123"
  }'`}</CodeBlock>
                    <p className="text-[13px] text-[#71717a] leading-relaxed">
                      Idempotent for the same{" "}
                      <code>(install_id, user_id)</code> pair. Rift refuses
                      to overwrite a previously-bound install with a different
                      user — the first binding wins.
                    </p>
                  </div>
                ),
              },
            ]}
          />

          <Callout type="info">
            <strong className="text-[#fafafa]">On logout:</strong> call{" "}
            <code>rift.clearUserId()</code> to remove the stored user binding.
            The install_id is preserved — only the user link is cleared.
          </Callout>

          <div>
            <p className="text-[13px] text-[#71717a] leading-relaxed mb-2">
              The <code>identify</code> webhook payload your endpoint
              receives:
            </p>
            <CodeBlock lang="json">{`{
  "event": "identify",
  "timestamp": "2026-05-15T12:34:56Z",
  "data": {
    "tenant_id": "65f...",
    "user_id": "usr_abc123",
    "link_id": "summer-sale",
    "install_id": "device-uuid",
    "link_metadata": {
      "bonus_type": "welcome",
      "bonus_amount_usdc": "20"
    }
  }
}`}</CodeBlock>
          </div>

          <Callout type="info">
            Idempotent rebind (same install ↔ same user) is intentionally
            silent — the webhook only fires on a real state transition,
            so subscribers can&apos;t double-fulfill on SDK retries. For
            the existing-install re-attribution case (already-signed-in
            user clicks a new campaign link), credits flow through the{" "}
            <code>attribute</code> webhook instead — its payload carries
            the already-bound <code>user_id</code>.
          </Callout>
        </section>

        <div className="gradient-line" />

        {/* Convert */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Track conversions</h2>
          <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
            <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">
              convert
            </code>{" "}
            is the last verb in the funnel — the valuable action you
            actually care about (first purchase, completed signup,
            subscription start). Fire it from your app with a stable{" "}
            <code className="text-[#71717a] bg-[#18181b] px-1 py-0.5 rounded text-[12px]">
              idempotency_key
            </code>{" "}
            and Rift resolves the user back to the campaign that drove
            their install via the{" "}
            <code className="text-[#71717a] bg-[#18181b] px-1 py-0.5 rounded text-[12px]">
              installs
            </code>{" "}
            record.
          </p>

          <DocsSetupTabs
            title="Fire a conversion"
            tabs={[
              {
                id: "ios",
                label: "iOS",
                children: (
                  <div className="space-y-3">
                    <CodeBlock lang="swift">{`try await rift.trackConversion(
    conversionType: "first_trade",
    idempotencyKey: order.id,
    metadata: ["volume_usdc": "1500"]
)`}</CodeBlock>
                  </div>
                ),
              },
              {
                id: "android",
                label: "Android",
                children: (
                  <div className="space-y-3">
                    <CodeBlock lang="kotlin">{`rift.trackConversion(
    conversionType = "first_trade",
    idempotencyKey = order.id,
    metadata = mapOf("volume_usdc" to "1500")
)`}</CodeBlock>
                  </div>
                ),
              },
              {
                id: "http",
                label: "HTTP",
                children: (
                  <div className="space-y-3">
                    <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/lifecycle/convert \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "user_id": "usr_abc123",
    "type": "first_trade",
    "idempotency_key": "ord_9f8d2",
    "metadata": { "volume_usdc": "1500" }
  }'`}</CodeBlock>
                  </div>
                ),
              },
            ]}
          />

          <Callout type="info">
            Server-to-server conversion sources (Stripe, RevenueCat,
            custom webhooks) configure separately so you can fire
            conversions from billing infrastructure that never touches
            the SDK. See{" "}
            <a
              href="/docs/conversions"
              className="text-[#2dd4bf] hover:underline"
            >
              Conversions
            </a>
            .
          </Callout>
        </section>

        <div className="gradient-line" />

        {/* Analytics */}
        <section className="space-y-6">
          <h2 className="text-2xl font-bold text-[#fafafa]">Analytics</h2>

          <Step n={1} title="Link stats">
            <p>
              Get aggregate click and install counts for a link. The response also
              includes a <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">conversions</code>{" "}
              array populated by <a href="/docs/conversions" className="text-[#2dd4bf] hover:underline">conversion tracking</a>:
            </p>
            <CodeBlock>{`curl https://api.riftl.ink/v1/links/summer-sale/stats \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <CodeBlock lang="json">{`{
  "link_id": "summer-sale",
  "click_count": 1234,
  "install_count": 89,
  "identify_count": 73,
  "convert_count": 61,
  "conversions": [
    { "conversion_type": "deposit", "count": 19 },
    { "conversion_type": "signup", "count": 42 }
  ]
}`}</CodeBlock>
          </Step>

          <Step n={2} title="Time series">
            <p>Get daily click counts for a date range:</p>
            <CodeBlock>{`curl "https://api.riftl.ink/v1/links/summer-sale/timeseries?from=2025-04-01T00:00:00Z&to=2025-04-07T00:00:00Z&granularity=daily" \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            <CodeBlock lang="json">{`{
  "link_id": "summer-sale",
  "granularity": "daily",
  "data": [
    { "date": "2025-04-01", "clicks": 42 },
    { "date": "2025-04-02", "clicks": 67 }
  ]
}`}</CodeBlock>
          </Step>
        </section>

        <div className="gradient-line" />

        {/* Next step */}
        <section className="space-y-4">
          <h2 className="text-2xl font-bold text-[#fafafa]">Next: track conversions</h2>
          <p className="text-[15px] leading-relaxed text-[#a1a1aa]">
            Click and install attribution tell you who came from which link. To measure what they
            do next — signups, purchases, deposits — use{" "}
            <a href="/docs/conversions" className="text-[#2dd4bf] hover:underline">
              Conversions
            </a>
            . Track events from the mobile SDK with one line, or POST from your backend via webhooks.
            Either way, Rift attributes each conversion back to the originating link.
          </p>
        </section>
      </div>
    </div>
  );
}
