import { TIERS } from "@/lib/tiers";
import { PricingSectionClient } from "@/components/pricing-section-client";

export function PricingSection() {
  return (
    <section id="pricing" className="py-24 px-6 content-auto-section">
      <div className="mx-auto max-w-6xl">
        <div className="reveal reveal-visible">
          <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
            Pricing
          </p>
          <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-3">
            One product. Pay in dollars or USDC.
          </h2>
          <p className="text-[#71717a]">
            Same limits on both lanes. Full API, SDKs, and deep links on every tier
            — free included. No credit card required.
          </p>
        </div>

        <PricingSectionClient tiers={TIERS} />
      </div>
    </section>
  );
}
