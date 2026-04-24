import type { Metadata } from "next";
import { FreeSignupForm } from "@/components/free-signup-form";

export const metadata: Metadata = {
  title: "Sign up for Rift — Deep links for humans and agents",
  description:
    "Install the Rift CLI and create your first deep link in under two minutes. Free tier forever, no credit card required.",
};

const INSTALL_CMD =
  "curl -fsSL https://raw.githubusercontent.com/saltyskip/rift/main/client/cli/install.sh | sh";

export default function SignupPage() {
  return (
    <main className="pt-24 pb-20 px-6">
      <div className="mx-auto max-w-3xl">
          <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
            Get started
          </p>
          <h1 className="text-4xl font-semibold tracking-[-0.03em] mb-4">
            Rift in 90 seconds.
          </h1>
          <p className="text-[15px] text-[#71717a] max-w-xl leading-relaxed">
            Install the CLI, verify your email, create a link. No credit card, no
            dashboard to log into, no quota anxiety — the Free tier ships production-ready.
          </p>

          <section className="mt-12">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-4">
              1 · Install
            </p>
            <pre className="rounded-xl border border-[#222225] bg-[#0a0a0b] p-5 text-[13px] text-[#e4e4e7] overflow-x-auto font-mono">
              <code>{INSTALL_CMD}</code>
            </pre>
            <p className="text-[13px] text-[#52525b] mt-3">
              macOS and Linux. Windows users can build from source with{" "}
              <code className="text-[#a1a1aa]">cargo install --git https://github.com/saltyskip/rift rift-cli</code>.
            </p>
          </section>

          <section className="mt-12">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-4">
              2 · Create your account
            </p>
            <pre className="rounded-xl border border-[#222225] bg-[#0a0a0b] p-5 text-[13px] text-[#e4e4e7] overflow-x-auto font-mono">
              <code>{`rift init
# Enter your email → verify → paste your rl_live_ key`}</code>
            </pre>
          </section>

          <section className="mt-12">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-4">
              3 · Ship
            </p>
            <pre className="rounded-xl border border-[#222225] bg-[#0a0a0b] p-5 text-[13px] text-[#e4e4e7] overflow-x-auto font-mono">
              <code>{`rift links create --web-url https://yourapp.com/onboard
rift subscribe pro   # when you outgrow Free`}</code>
            </pre>
          </section>

          <section className="mt-16 rounded-xl border border-[#222225] bg-[#111113] p-8">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-2">
              Prefer the web?
            </p>
            <h2 className="text-lg font-semibold mb-2">
              Sign up for free in your browser.
            </h2>
            <p className="text-[14px] text-[#71717a] mb-5">
              Enter your email and we&rsquo;ll send a verification link. Your API key
              appears once after verification.
            </p>
            <FreeSignupForm />
          </section>

          <section className="mt-16">
            <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-4">
              Already have an account?
            </p>
            <pre className="rounded-xl border border-[#222225] bg-[#0a0a0b] p-5 text-[13px] text-[#e4e4e7] overflow-x-auto font-mono">
              <code>{`rift login     # paste your rl_live_ key
rift whoami    # confirm this machine is connected`}</code>
            </pre>
          </section>
      </div>
    </main>
  );
}
