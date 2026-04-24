import type { Metadata } from "next";
import { MagicLinkForm } from "@/components/magic-link-form";

export const metadata: Metadata = {
  title: "Manage your Rift subscription",
  description:
    "Update your card, download invoices, or cancel your subscription. We'll email a secure link to open your billing portal.",
};

export default async function ManagePage({
  searchParams,
}: {
  searchParams: Promise<{ done?: string; error?: string }>;
}) {
  const { done, error } = await searchParams;

  return (
    <main className="pt-24 pb-20 px-6 min-h-[60vh]">
      <div className="mx-auto max-w-xl">
          <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
            Billing
          </p>
          <h1 className="text-3xl font-semibold tracking-[-0.03em] mb-4">
            Manage your Rift subscription.
          </h1>
          <p className="text-[14px] text-[#71717a] mb-8 leading-relaxed">
            Update your card, download invoices, or cancel — all through
            Stripe&rsquo;s hosted portal. Enter your email below and we&rsquo;ll send a
            single-use link.
          </p>

          {done === "1" && (
            <div className="mb-6 rounded-xl border border-[#2dd4bf]/30 bg-[#2dd4bf]/[0.05] p-5">
              <p className="text-[12px] font-mono text-[#2dd4bf] uppercase tracking-widest mb-2">
                Changes saved
              </p>
              <p className="text-[14px] text-[#fafafa]">
                Your subscription has been updated. Any changes take effect
                according to your billing cycle.
              </p>
            </div>
          )}

          {error === "no_subscription" && (
            <div className="mb-6 rounded-xl border border-amber-500/30 bg-amber-500/[0.05] p-5">
              <p className="text-[12px] font-mono text-amber-400 uppercase tracking-widest mb-2">
                No subscription found
              </p>
              <p className="text-[14px] text-[#fafafa]">
                We couldn&rsquo;t find a paid subscription for that email. If you
                meant to start one, head to pricing.
              </p>
            </div>
          )}

          <div className="rounded-xl border border-[#222225] bg-[#111113] p-6">
            <MagicLinkForm
              intent="portal"
              label="Your billing email"
              submitLabel="Send secure link"
              note="The link expires in 15 minutes and can only be used once. It opens Stripe's billing portal directly."
            />
          </div>

          <section className="mt-10 text-[13px] text-[#52525b] leading-relaxed">
            <p>
              Prefer the CLI? Run <code className="text-[#a1a1aa]">rift cancel</code> to
              schedule a cancellation, or <code className="text-[#a1a1aa]">rift billing</code>{" "}
              to see your current plan and renewal date.
            </p>
        </section>
      </div>
    </main>
  );
}
