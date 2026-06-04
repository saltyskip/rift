import type { Metadata } from "next";
import { OauthButtons } from "@/components/oauth-buttons";
import { SignInForm } from "@/components/sign-in-form";

export const metadata: Metadata = {
  title: "Sign in to Rift — Deep links for humans and agents",
  description:
    "Sign in to your Rift account. No password — we'll email you a one-time link. First time? Your account is created on first sign-in.",
};

const INSTALL_CMD =
  "curl -fsSL https://raw.githubusercontent.com/saltyskip/rift/main/client/cli/install.sh | sh";

export default async function SignInPage({
  searchParams,
}: {
  searchParams: Promise<{ next?: string }>;
}) {
  const { next: nextRaw } = await searchParams;
  // Only forward `next` if it's a same-origin path. Server re-validates
  // against the matched origin / marketing_url, but a client-side guard
  // keeps obvious garbage out of the POST body.
  const next = nextRaw && nextRaw.startsWith("/") ? nextRaw : undefined;

  return (
    <main className="pt-24 pb-20 px-6 min-h-[70vh]">
      <div className="mx-auto max-w-md">
        <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
          Get started
        </p>
        <h1 className="text-3xl font-semibold tracking-[-0.03em] mb-4">
          Sign in to Rift.
        </h1>
        <p className="text-[14px] text-[#71717a] mb-8 leading-relaxed">
          No password. We&rsquo;ll email you a link. First time?
          Your account is created on first sign-in.
        </p>

        <div className="rounded-xl border border-[#222225] bg-[#111113] p-6">
          <OauthButtons next={next} />
          <SignInForm next={next} />
        </div>

        <section className="mt-14">
          <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-3">
            Prefer the CLI?
          </p>
          <pre className="rounded-xl border border-[#222225] bg-[#0a0a0b] p-4 text-[12px] text-[#e4e4e7] overflow-x-auto font-mono">
            <code>{INSTALL_CMD}</code>
          </pre>
          <p className="text-[12px] text-[#52525b] mt-3">
            macOS and Linux. Windows users can build from source with{" "}
            <code className="text-[#a1a1aa]">
              cargo install --git https://github.com/saltyskip/rift rift-cli
            </code>
. After installing, run <code className="text-[#a1a1aa]">rift login</code>{" "}
            to sign in with your browser (or{" "}
            <code className="text-[#a1a1aa]">rift login --api-key</code> to paste a
            key you mint from <code className="text-[#a1a1aa]">/account</code>).
          </p>
        </section>
      </div>
    </main>
  );
}
