import type { Metadata } from "next";
import { DocsCodeBlock as CodeBlock } from "@/components/docs-code-block";

export const metadata: Metadata = {
  title: "Register Your App — Rift Docs",
  description: "Register iOS and Android apps for branding, association files, and smart landing pages.",
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

export default function AppsPage() {
  return (
    <div className="max-w-3xl">
      <div className="mb-12">
        <p className="text-[13px] font-medium text-[#2dd4bf] uppercase tracking-widest mb-3">Getting Started</p>
        <h1 className="text-4xl font-bold text-[#fafafa] mb-4">Register Your App</h1>
        <p className="text-lg text-[#71717a] leading-relaxed">
          Register your iOS and/or Android app so Rift can serve association files
          (AASA &amp; assetlinks) and display your branding on smart landing pages.
        </p>
      </div>

      <div className="space-y-10">
        <section className="space-y-6">
          <Step n={1} title="Register an iOS app">
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

          <Step n={2} title="Register an Android app">
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
      </div>
    </div>
  );
}
