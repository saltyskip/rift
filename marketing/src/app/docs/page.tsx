import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Docs — Rift",
  description: "Get started with Rift deep links, app configuration, and deferred deep linking.",
};

function CodeBlock({ children }: { children: string }) {
  return (
    <pre className="bg-[#0c0c0e] border border-[#1e1e22] rounded-lg p-4 overflow-x-auto text-[13px] leading-relaxed font-mono text-[#a1a1aa]">
      <code>{children}</code>
    </pre>
  );
}

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

function SectionHeading({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h2 id={id} className="text-2xl font-bold text-[#fafafa] scroll-mt-24">
      {children}
    </h2>
  );
}

export default function DocsPage() {
  return (
    <div className="min-h-screen pt-24 pb-20">
      <div className="mx-auto max-w-3xl px-6">
        {/* Header */}
        <div className="mb-16">
          <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Documentation</p>
          <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Getting Started</h1>
          <p className="text-lg text-[#71717a] leading-relaxed">
            Go from zero to mobile deep links in under 10 minutes.
            This guide walks you through API key setup, app registration,
            per-platform deep links, custom domains, and deferred deep linking.
          </p>
        </div>

        {/* Table of contents */}
        <div className="mb-16 p-5 rounded-xl bg-[#111113] border border-[#1e1e22]">
          <p className="text-[11px] font-medium text-[#52525b] uppercase tracking-widest mb-3">On this page</p>
          <nav className="space-y-1.5">
            {[
              ["#get-api-key", "1. Get your API key"],
              ["#register-app", "2. Register your app"],
              ["#custom-domain", "3. Set up a custom domain"],
              ["#universal-links", "4. Configure universal links / app links"],
              ["#create-link", "5. Create a deep link"],
              ["#handle-links", "6. Handle incoming links"],
              ["#deferred", "7. Deferred deep linking"],
              ["#attribution", "8. Attribution"],
              ["#analytics", "9. Analytics"],
            ].map(([href, label]) => (
              <a
                key={href}
                href={href}
                className="block text-[14px] text-[#71717a] hover:text-[#2dd4bf] transition-colors"
              >
                {label}
              </a>
            ))}
          </nav>
        </div>

        <div className="space-y-16">
          {/* ── 1. Get API key ── */}
          <section className="space-y-6">
            <SectionHeading id="get-api-key">1. Get your API key</SectionHeading>
            <Step n={1} title="Sign up">
              <p>
                Send a POST request with your email to get an API key.
                You&apos;ll receive a verification email.
              </p>
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/auth/signup \\
  -H "Content-Type: application/json" \\
  -d '{"email": "you@example.com"}'`}</CodeBlock>
              <p>
                The response contains your API key prefix (starts with <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">rl_live_</code>).
                The full key is in the verification email — save it, it won&apos;t be shown again.
              </p>
            </Step>

            <Step n={2} title="Verify your email">
              <p>
                Click the verification link in your inbox. Your key is now active.
              </p>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 2. Register your app ── */}
          <section className="space-y-6">
            <SectionHeading id="register-app">2. Register your app</SectionHeading>
            <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
              Register your iOS and/or Android app so Relay can serve association files
              (AASA &amp; assetlinks) and display your branding on smart landing pages.
            </p>

            <Step n={3} title="Register an iOS app">
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/apps \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "platform": "ios",
    "bundle_id": "com.example.myapp",
    "team_id": "ABCDE12345",
    "app_name": "MyApp",
    "icon_url": "https://example.com/icon.png",
    "theme_color": "#FF6B00"
  }'`}</CodeBlock>
              <p>
                <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">bundle_id</code> and{" "}
                <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">team_id</code> are
                required for iOS — they&apos;re used to generate the Apple App Site Association file.
              </p>
            </Step>

            <Step n={4} title="Register an Android app">
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/apps \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "platform": "android",
    "package_name": "com.example.myapp",
    "sha256_fingerprints": ["14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:8A:04:96:B2:3F:CF:44:E5"],
    "app_name": "MyApp",
    "icon_url": "https://example.com/icon.png",
    "theme_color": "#FF6B00"
  }'`}</CodeBlock>
              <p>
                <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">package_name</code> is
                required. The <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">sha256_fingerprints</code> are
                your signing certificate fingerprints for Android App Links verification.
              </p>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 3. Custom domain ── */}
          <section className="space-y-6">
            <SectionHeading id="custom-domain">3. Set up a custom domain</SectionHeading>
            <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
              Use your own brand for links: <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">go.yourcompany.com/summer-sale</code>{" "}
              instead of <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">riftl.ink/r/summer-sale</code>.
              Custom domains also enable universal links and app links.
            </p>

            <Step n={5} title="Register your domain">
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"domain": "go.yourcompany.com"}'`}</CodeBlock>
              <p>Response:</p>
              <CodeBlock>{`{
  "domain": "go.yourcompany.com",
  "verified": false,
  "verification_token": "a1b2c3d4e5f6...",
  "txt_record": "_rift-verify.go.yourcompany.com",
  "cname_target": "riftl.ink"
}`}</CodeBlock>
            </Step>

            <Step n={6} title="Add DNS records">
              <p>
                In your DNS provider, create two records:
              </p>
              <div className="overflow-x-auto">
                <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                  <thead>
                    <tr className="bg-[#0c0c0e]">
                      <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Type</th>
                      <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Name</th>
                      <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Value</th>
                    </tr>
                  </thead>
                  <tbody className="text-[#a1a1aa]">
                    <tr className="border-b border-[#1e1e22]">
                      <td className="px-4 py-2.5 font-mono text-[#60a5fa]">CNAME</td>
                      <td className="px-4 py-2.5 font-mono">go</td>
                      <td className="px-4 py-2.5 font-mono">riftl.ink</td>
                    </tr>
                    <tr>
                      <td className="px-4 py-2.5 font-mono text-[#f59e0b]">TXT</td>
                      <td className="px-4 py-2.5 font-mono">_rift-verify.go</td>
                      <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">a1b2c3d4e5f6...</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </Step>

            <Step n={7} title="Deploy the edge worker">
              <p>
                Relay needs a lightweight Cloudflare Worker on your domain to forward requests
                to the API. Create a new Worker in your Cloudflare dashboard with this code:
              </p>
              <CodeBlock>{`export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const host = url.hostname;
    const origin = "https://api.riftl.ink";
    const upstream = new URL(url.pathname + url.search, origin);
    const headers = new Headers(request.headers);
    headers.set("X-Relay-Host", host);
    const response = await fetch(upstream.toString(), {
      method: request.method,
      headers,
      body: request.method !== "GET" && request.method !== "HEAD"
        ? request.body : undefined,
      redirect: "manual",
    });
    return response;
  },
};`}</CodeBlock>
              <p>
                Then add a <strong className="text-[#fafafa]">Worker Route</strong> on your zone:
              </p>
              <div className="overflow-x-auto">
                <table className="w-full text-[13px] border border-[#1e1e22] rounded-lg overflow-hidden">
                  <thead>
                    <tr className="bg-[#0c0c0e]">
                      <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Setting</th>
                      <th className="text-left text-[#52525b] font-medium px-4 py-2.5 border-b border-[#1e1e22]">Value</th>
                    </tr>
                  </thead>
                  <tbody className="text-[#a1a1aa]">
                    <tr className="border-b border-[#1e1e22]">
                      <td className="px-4 py-2.5">Route pattern</td>
                      <td className="px-4 py-2.5 font-mono text-[#2dd4bf]">go.yourcompany.com/*</td>
                    </tr>
                    <tr>
                      <td className="px-4 py-2.5">Worker</td>
                      <td className="px-4 py-2.5 font-mono">your-relay-worker</td>
                    </tr>
                  </tbody>
                </table>
              </div>
              <p>
                Make sure the CNAME record for your subdomain is set to{" "}
                <strong className="text-[#fafafa]">Proxied</strong> (orange cloud) in Cloudflare.
              </p>
            </Step>

            <Step n={8} title="Verify ownership">
              <p>Once DNS has propagated:</p>
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/domains/go.yourcompany.com/verify \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 4. Universal links / app links ── */}
          <section className="space-y-6">
            <SectionHeading id="universal-links">4. Configure universal links / app links</SectionHeading>
            <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
              Once your app is registered and domain verified, Relay automatically serves
              the association files. You just need to configure your apps to use them.
            </p>

            <Step n={9} title="iOS — Associated Domains">
              <p>
                In Xcode, go to <strong className="text-[#fafafa]">Signing &amp; Capabilities</strong> &rarr;{" "}
                <strong className="text-[#fafafa]">+ Capability</strong> &rarr;{" "}
                <strong className="text-[#fafafa]">Associated Domains</strong>, then add:
              </p>
              <CodeBlock>{`applinks:go.yourcompany.com`}</CodeBlock>
              <p>
                Relay serves the AASA file at{" "}
                <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">
                  https://go.yourcompany.com/.well-known/apple-app-site-association
                </code>
              </p>
            </Step>

            <Step n={10} title="Android — Intent Filters">
              <p>
                Add an intent filter to your <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">AndroidManifest.xml</code>:
              </p>
              <CodeBlock>{`<activity android:name=".MainActivity">
    <intent-filter android:autoVerify="true">
        <action android:name="android.intent.action.VIEW" />
        <category android:name="android.intent.category.DEFAULT" />
        <category android:name="android.intent.category.BROWSABLE" />
        <data android:scheme="https"
              android:host="go.yourcompany.com" />
    </intent-filter>
</activity>`}</CodeBlock>
              <p>
                Relay serves the assetlinks file at{" "}
                <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">
                  https://go.yourcompany.com/.well-known/assetlinks.json
                </code>
              </p>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 5. Create a deep link ── */}
          <section className="space-y-6">
            <SectionHeading id="create-link">5. Create a deep link</SectionHeading>
            <Step n={11} title="Create a link with per-platform destinations">
              <p>
                Specify where each platform should go — deep link URI, store URL, and web fallback:
              </p>
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/links \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "custom_id": "summer-sale",
    "ios_deep_link": "myapp://promo/summer-sale",
    "android_deep_link": "myapp://promo/summer-sale",
    "web_url": "https://example.com/promo/summer-sale",
    "ios_store_url": "https://apps.apple.com/app/id123456789",
    "android_store_url": "https://play.google.com/store/apps/details?id=com.example.myapp",
    "metadata": {
      "title": "Summer Sale — 50% Off",
      "description": "Limited time offer on all products",
      "image": "https://example.com/promo-banner.jpg"
    }
  }'`}</CodeBlock>
              <p>Response:</p>
              <CodeBlock>{`{
  "link_id": "summer-sale",
  "url": "https://api.riftl.ink/r/summer-sale"
}`}</CodeBlock>
            </Step>

            <Step n={12} title="How resolution works">
              <p>When a user clicks the link, Relay detects their platform and serves a smart landing page that:</p>
              <ul className="list-disc pl-5 space-y-1">
                <li><strong className="text-[#fafafa]">iOS</strong> — attempts to open the deep link, falls back to the App Store</li>
                <li><strong className="text-[#fafafa]">Android</strong> — attempts to open the deep link, falls back to the Play Store</li>
                <li><strong className="text-[#fafafa]">Desktop</strong> — redirects to the web URL</li>
              </ul>
              <p>
                The landing page includes your app branding (from step 2) and OG tags from link metadata
                for rich social previews.
              </p>
              <p>
                Agents sending <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">Accept: application/json</code> receive
                all destinations and metadata as JSON:
              </p>
              <CodeBlock>{`curl https://api.riftl.ink/r/summer-sale \\
  -H "Accept: application/json"

{
  "link_id": "summer-sale",
  "ios_deep_link": "myapp://promo/summer-sale",
  "android_deep_link": "myapp://promo/summer-sale",
  "web_url": "https://example.com/promo/summer-sale",
  "ios_store_url": "https://apps.apple.com/app/id123456789",
  "android_store_url": "https://play.google.com/store/apps/details?id=com.example.myapp",
  "metadata": { "title": "Summer Sale — 50% Off", ... }
}`}</CodeBlock>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 6. Handle incoming links ── */}
          <section className="space-y-6">
            <SectionHeading id="handle-links">6. Handle incoming links</SectionHeading>

            <Step n={13} title="iOS — SceneDelegate or AppDelegate">
              <CodeBlock>{`// SceneDelegate.swift
func scene(_ scene: UIScene,
           continue userActivity: NSUserActivity) {
    guard userActivity.activityType ==
              NSUserActivityTypeBrowsingWeb,
          let url = userActivity.webpageURL else { return }

    let linkId = url.path
        .trimmingCharacters(in: CharacterSet(charactersIn: "/"))
    handleDeepLink(linkId: linkId)
}`}</CodeBlock>
            </Step>

            <Step n={14} title="Android — Intent handling">
              <CodeBlock>{`// MainActivity.kt
override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    intent?.data?.let { uri ->
        val linkId = uri.path?.trimStart('/')
        if (linkId != null) handleDeepLink(linkId)
    }
}`}</CodeBlock>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 7. Deferred deep linking ── */}
          <section className="space-y-6">
            <SectionHeading id="deferred">7. Deferred deep linking</SectionHeading>
            <p className="text-[15px] text-[#a1a1aa] leading-relaxed">
              Route users to specific content even if they didn&apos;t have the app installed when they clicked.
              Relay generates a token on click and delivers it to the app after install.
            </p>

            <Step n={15} title="How it works">
              <ol className="list-decimal pl-5 space-y-1">
                <li>User clicks a Relay link on mobile</li>
                <li>Relay generates a token and stores it with the click</li>
                <li><strong className="text-[#fafafa]">iOS:</strong> token is copied to clipboard as <code className="text-[#71717a] bg-[#18181b] px-1.5 py-0.5 rounded text-[13px]">relay:&lt;token&gt;</code></li>
                <li><strong className="text-[#fafafa]">Android:</strong> token is appended to the Play Store URL as an install referrer</li>
                <li>User installs the app and opens it</li>
                <li>App reads the token and sends it to <code className="text-[#2dd4bf] bg-[#2dd4bf]/10 px-1.5 py-0.5 rounded text-[13px]">POST /v1/deferred</code></li>
              </ol>
            </Step>

            <Step n={16} title="iOS — Read from clipboard">
              <CodeBlock>{`func checkDeferredDeepLink() {
    guard let clipboard = UIPasteboard.general.string,
          clipboard.hasPrefix("relay:") else { return }

    let token = String(clipboard.dropFirst("relay:".count))
    UIPasteboard.general.string = ""  // Clear after reading
    resolveDeferred(token: token)
}`}</CodeBlock>
            </Step>

            <Step n={17} title="Android — Read from install referrer">
              <CodeBlock>{`val client = InstallReferrerClient.newBuilder(this).build()
client.startConnection(object : InstallReferrerStateListener {
    override fun onInstallReferrerSetupFinished(code: Int) {
        if (code == InstallReferrerResponse.OK) {
            val referrer = client.installReferrer.installReferrer
            val token = Uri.parse("?\$referrer")
                .getQueryParameter("relay_token")
            if (token != null) resolveDeferred(token)
        }
        client.endConnection()
    }
    override fun onInstallReferrerServiceDisconnected() {}
})`}</CodeBlock>
            </Step>

            <Step n={18} title="Resolve the token">
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/deferred \\
  -H "Content-Type: application/json" \\
  -d '{
    "token": "a1b2c3d4e5f6a7b8",
    "install_id": "device-uuid-here"
  }'`}</CodeBlock>
              <p>Response (matched):</p>
              <CodeBlock>{`{
  "matched": true,
  "link_id": "summer-sale",
  "ios_deep_link": "myapp://promo/summer-sale",
  "android_deep_link": "myapp://promo/summer-sale",
  "metadata": { "title": "Summer Sale — 50% Off" }
}`}</CodeBlock>
              <p>Response (not matched):</p>
              <CodeBlock>{`{ "matched": false }`}</CodeBlock>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 8. Attribution ── */}
          <section className="space-y-6">
            <SectionHeading id="attribution">8. Attribution</SectionHeading>

            <Step n={19} title="Report an install">
              <p>After the app is installed and opened, report the attribution:</p>
              <CodeBlock>{`curl -X POST https://api.riftl.ink/v1/attribution \\
  -H "Content-Type: application/json" \\
  -d '{
    "link_id": "summer-sale",
    "install_id": "device-uuid-here",
    "app_version": "1.0.0"
  }'`}</CodeBlock>
            </Step>

            <Step n={20} title="Link attribution to a user">
              <p>After the user signs up or logs in, connect the attribution to their account:</p>
              <CodeBlock>{`curl -X PUT https://api.riftl.ink/v1/attribution/link \\
  -H "Authorization: Bearer rl_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"install_id": "device-uuid-here"}'`}</CodeBlock>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── 9. Analytics ── */}
          <section className="space-y-6">
            <SectionHeading id="analytics">9. Analytics</SectionHeading>

            <Step n={21} title="View link stats">
              <CodeBlock>{`curl https://api.riftl.ink/v1/links/summer-sale/stats \\
  -H "Authorization: Bearer rl_live_YOUR_KEY"`}</CodeBlock>
              <p>Response:</p>
              <CodeBlock>{`{
  "link_id": "summer-sale",
  "click_count": 1234,
  "install_count": 89,
  "conversion_rate": 0.072
}`}</CodeBlock>
            </Step>
          </section>

          <div className="gradient-line" />

          {/* ── Next steps ── */}
          <section className="space-y-4">
            <h2 className="text-2xl font-bold text-[#fafafa]">Next steps</h2>
            <div className="grid gap-3">
              {[
                {
                  title: "API Reference",
                  desc: "Full endpoint documentation with try-it-out.",
                  href: "/api-reference",
                },
                {
                  title: "Manage apps",
                  desc: "List or remove registered app configurations.",
                  href: "/api-reference",
                },
                {
                  title: "Manage domains",
                  desc: "List, verify, or remove custom domains.",
                  href: "/api-reference",
                },
              ].map((item) => (
                <a
                  key={item.title}
                  href={item.href}
                  className="group flex items-center justify-between p-4 rounded-xl bg-[#111113] border border-[#1e1e22] hover:border-[#2dd4bf]/30 transition-colors"
                >
                  <div>
                    <p className="text-[15px] font-medium text-[#fafafa] group-hover:text-[#2dd4bf] transition-colors">{item.title}</p>
                    <p className="text-[13px] text-[#52525b]">{item.desc}</p>
                  </div>
                  <span className="text-[#3f3f46] group-hover:text-[#2dd4bf] transition-colors">&rarr;</span>
                </a>
              ))}
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
