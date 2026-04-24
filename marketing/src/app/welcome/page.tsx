import type { Metadata } from "next";
import Link from "next/link";
import { Navbar } from "@/components/navbar";
import { Footer } from "@/components/footer";

export const metadata: Metadata = {
  title: "Welcome to Rift",
  description: "Your subscription is active. Check your inbox for your API key and install the CLI to ship your first link.",
};

const INSTALL_CMD =
  "curl -fsSL https://raw.githubusercontent.com/saltyskip/rift/main/client/cli/install.sh | sh";

export default function WelcomePage() {
  return (
    <>
      <Navbar />
      <main className="pt-24 pb-20 px-6 min-h-[70vh]">
        <div className="mx-auto max-w-2xl">
          <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
            Payment received
          </p>
          <h1 className="text-4xl font-semibold tracking-[-0.03em] mb-4">
            Welcome to Rift.
          </h1>
          <p className="text-[15px] text-[#71717a] max-w-xl leading-relaxed">
            Your subscription is active. Check your inbox for your{" "}
            <code className="text-[#a1a1aa]">rl_live_</code> API key — we&rsquo;ll only
            show it once, so save it somewhere safe.
          </p>

          <section className="mt-12">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-4">
              1 · Install the CLI
            </p>
            <pre className="rounded-xl border border-[#222225] bg-[#0a0a0b] p-5 text-[13px] text-[#e4e4e7] overflow-x-auto font-mono">
              <code>{INSTALL_CMD}</code>
            </pre>
          </section>

          <section className="mt-10">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-4">
              2 · Log in with your key
            </p>
            <pre className="rounded-xl border border-[#222225] bg-[#0a0a0b] p-5 text-[13px] text-[#e4e4e7] overflow-x-auto font-mono">
              <code>{`rift login
# paste the rl_live_ key from your email

rift whoami
# confirm this machine is connected`}</code>
            </pre>
          </section>

          <section className="mt-10">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-4">
              3 · Create your first link
            </p>
            <pre className="rounded-xl border border-[#222225] bg-[#0a0a0b] p-5 text-[13px] text-[#e4e4e7] overflow-x-auto font-mono">
              <code>{`rift links create --web-url https://yourapp.com/path`}</code>
            </pre>
          </section>

          <section className="mt-14 rounded-xl border border-[#222225] bg-[#111113] p-6">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-3">
              Need to change something?
            </p>
            <p className="text-[14px] text-[#71717a] mb-4">
              Update your card, download invoices, or cancel anytime.
            </p>
            <div className="flex flex-wrap gap-3">
              <Link
                href="/manage"
                className="text-[13px] font-medium border border-[#222225] text-[#fafafa] px-4 py-2 rounded-lg hover:border-[#2dd4bf]/30 transition-colors"
              >
                Manage billing
              </Link>
              <Link
                href="/signup"
                className="text-[13px] font-medium text-[#71717a] px-4 py-2 rounded-lg hover:text-[#fafafa] transition-colors"
              >
                See all commands
              </Link>
            </div>
          </section>
        </div>
      </main>
      <Footer />
    </>
  );
}
