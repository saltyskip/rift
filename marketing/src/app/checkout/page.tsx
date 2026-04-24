import type { Metadata } from "next";
import Link from "next/link";
import { MagicLinkForm } from "@/components/magic-link-form";
import { Navbar } from "@/components/navbar";
import { Footer } from "@/components/footer";
import { getTierBySlug, isPaidTierSlug, type PaidTierSlug } from "@/lib/tiers";

export const metadata: Metadata = {
  title: "Subscribe to Rift",
  description:
    "Start your Rift paid subscription. We'll email a secure link to complete checkout.",
};

export default async function CheckoutPage({
  searchParams,
}: {
  searchParams: Promise<{ tier?: string }>;
}) {
  const { tier: tierParam } = await searchParams;
  const tier =
    tierParam && isPaidTierSlug(tierParam) ? getTierBySlug(tierParam) : undefined;

  if (!tier) {
    return (
      <>
        <Navbar />
        <main className="pt-32 pb-20 px-6 min-h-[60vh]">
          <div className="mx-auto max-w-xl text-center">
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
              Pick a tier
            </p>
            <h1 className="text-3xl font-semibold tracking-[-0.03em] mb-4">
              No tier selected.
            </h1>
            <p className="text-[14px] text-[#71717a] mb-8">
              Head back to pricing to pick the right plan for your workload.
            </p>
            <Link
              href="/#pricing"
              className="inline-block bg-[#2dd4bf] text-[#042f2e] text-[14px] font-semibold px-5 py-2.5 rounded-lg hover:bg-[#5eead4] transition-colors"
            >
              See pricing
            </Link>
          </div>
        </main>
        <Footer />
      </>
    );
  }

  return (
    <>
      <Navbar />
      <main className="pt-24 pb-20 px-6">
        <div className="mx-auto max-w-3xl grid md:grid-cols-[1.1fr_1fr] gap-10">
          <section>
            <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
              Subscribe
            </p>
            <h1 className="text-3xl font-semibold tracking-[-0.03em] mb-2">
              Start Rift {tier.name}.
            </h1>
            <div className="flex items-baseline gap-1 mb-6">
              <span className="text-3xl font-semibold">{tier.human.price}</span>
              {tier.human.unit && (
                <span className="text-sm text-[#71717a]">{tier.human.unit}</span>
              )}
            </div>

            <ul className="space-y-2 mb-8">
              {tier.stats.map((s) => (
                <li
                  key={s}
                  className="text-[13px] font-mono text-[#2dd4bf]/75 leading-snug"
                >
                  {s}
                </li>
              ))}
            </ul>

            <div className="rounded-xl border border-[#222225] bg-[#111113] p-6">
              <MagicLinkForm
                intent="subscribe"
                tier={tier.slug as PaidTierSlug}
                label="Your email"
                submitLabel="Continue to Stripe"
                note="We'll email a single-use link that takes you to secure Stripe checkout. Existing customers are recognized automatically."
              />
            </div>
          </section>

          <aside className="space-y-6">
            <div className="rounded-xl border border-[#222225] bg-[#111113] p-6">
              <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-3">
                What happens next
              </p>
              <ol className="space-y-3 text-[13px] text-[#a1a1aa] leading-relaxed">
                <li>
                  <span className="text-[#2dd4bf] font-mono mr-2">1.</span>
                  Check your inbox for a secure link.
                </li>
                <li>
                  <span className="text-[#2dd4bf] font-mono mr-2">2.</span>
                  Click it to open Stripe Checkout.
                </li>
                <li>
                  <span className="text-[#2dd4bf] font-mono mr-2">3.</span>
                  Your API key lands in your inbox after payment.
                </li>
              </ol>
            </div>

            <div className="rounded-xl border border-[#222225] bg-[#111113] p-6 space-y-3 text-[13px] text-[#71717a]">
              <div>
                <p className="text-[#fafafa] font-medium mb-1">Secure</p>
                <p>Payments handled end-to-end by Stripe.</p>
              </div>
              <div>
                <p className="text-[#fafafa] font-medium mb-1">Cancel anytime</p>
                <p>
                  Use the{" "}
                  <Link href="/manage" className="text-[#2dd4bf] hover:underline">
                    manage billing
                  </Link>{" "}
                  link or <code className="text-[#a1a1aa]">rift cancel</code> in
                  the CLI.
                </p>
              </div>
            </div>
          </aside>
        </div>
      </main>
      <Footer />
    </>
  );
}
