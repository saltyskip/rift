"use client";

import { useState } from "react";
import { type Tier, type TierAudience } from "@/lib/tiers";
import { cn } from "@/lib/utils";

type Audience = TierAudience;

interface PricingSectionClientProps {
  tiers: Tier[];
}

export function PricingSectionClient({ tiers }: PricingSectionClientProps) {
  const [audience, setAudience] = useState<Audience>("human");

  return (
    <>
      <div className="mt-8 mb-10 flex items-center gap-3">
        <AudienceToggle audience={audience} setAudience={setAudience} />
        <span
          key={audience}
          className="text-[12px] font-mono text-[#52525b] tracking-wide transition-all duration-200 animate-page-enter"
        >
          {audience === "human"
            ? "Stripe · cancel anytime"
            : "x402 · USDC · coming soon"}
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
        {tiers.map((tier, i) => (
          <TierCard
            key={tier.slug}
            tier={tier}
            audience={audience}
            delay={i * 0.06}
          />
        ))}
      </div>

      <p className="mt-10 text-[12px] text-[#52525b] text-center">
        Hard limit on every tier — no surprise overage bills. Upgrade any time.
      </p>
    </>
  );
}

function AudienceToggle({
  audience,
  setAudience,
}: {
  audience: Audience;
  setAudience: (audience: Audience) => void;
}) {
  return (
    <div
      role="tablist"
      aria-label="Pricing audience"
      className="inline-flex rounded-full border border-[#222225] bg-[#111113] p-1"
    >
      {(["human", "agent"] as const).map((option) => {
        const active = audience === option;

        return (
          <button
            key={option}
            role="tab"
            aria-selected={active}
            onClick={() => setAudience(option)}
            className={cn(
              "relative rounded-full px-5 py-1.5 text-[13px] font-medium transition-all duration-200",
              active ? "bg-[#2dd4bf] text-[#042f2e]" : "text-[#a1a1aa]"
            )}
          >
            <span className="capitalize">{option}s</span>
          </button>
        );
      })}
    </div>
  );
}

function TierCard({
  tier,
  audience,
  delay,
}: {
  tier: Tier;
  audience: Audience;
  delay: number;
}) {
  const price = tier[audience];
  const isPaid = tier.slug !== "free";
  const agentComingSoon = audience === "agent";
  const cta = agentComingSoon ? "Coming soon" : !isPaid ? "Start free" : `Get ${tier.name}`;
  const ctaHref = !isPaid ? "/signup" : `/checkout?tier=${tier.slug}`;

  return (
    <div
      className={cn(
        "relative rounded-xl border p-7 flex flex-col transition-all duration-300 animate-page-enter",
        tier.accent
          ? "border-[#2dd4bf]/30 bg-[#2dd4bf]/[0.03] glow-teal"
          : "border-[#222225] bg-[#111113] hover:border-[#2dd4bf]/15"
      )}
      style={{ animationDelay: `${delay}s` }}
    >
      {tier.accent && (
        <span className="absolute -top-2.5 left-7 text-[10px] font-mono text-[#042f2e] bg-[#2dd4bf] px-2 py-0.5 rounded-full tracking-wide uppercase">
          Recommended
        </span>
      )}
      <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-3">
        {tier.name}
      </p>

      <div className="mb-1 flex min-h-[44px] items-baseline gap-1 animate-page-enter">
        <span className="text-3xl font-semibold tracking-tight">{price.price}</span>
        {price.unit ? (
          <span className="text-sm text-[#52525b]">{price.unit}</span>
        ) : null}
      </div>

      <p className="text-[13px] text-[#52525b] mb-3">{tier.desc}</p>

      <ul className="space-y-1 mb-5">
        {tier.stats.map((stat) => (
          <li
            key={stat}
            className="text-[12px] font-mono text-[#2dd4bf]/75 leading-snug"
          >
            {stat}
          </li>
        ))}
      </ul>

      <div className="h-px bg-[#222225] mb-5" />

      {tier.inherits ? (
        <p className="text-[11px] text-[#52525b] mb-3 leading-snug">
          Everything in{" "}
          <span className="text-[#a1a1aa] font-medium">{tier.inherits}</span>,
          plus:
        </p>
      ) : null}

      <ul className="space-y-2.5 flex-1 mb-6">
        {tier.items.map((item) => (
          <li
            key={item}
            className="flex items-start gap-2.5 text-[13px] text-[#71717a]"
          >
            <span className="mt-1.5 size-1 rounded-full bg-[#2dd4bf] shrink-0" />
            {item}
          </li>
        ))}
      </ul>

      {agentComingSoon ? (
        <span
          aria-disabled="true"
          className="text-center text-[13px] font-medium px-4 py-2 rounded-lg border border-dashed border-[#222225] text-[#52525b] cursor-not-allowed"
        >
          {cta}
        </span>
      ) : (
        <a
          href={ctaHref}
          className={cn(
            "text-center text-[13px] font-medium px-4 py-2 rounded-lg transition-colors",
            tier.accent
              ? "bg-[#2dd4bf] text-[#042f2e] hover:bg-[#5eead4]"
              : "border border-[#222225] text-[#a1a1aa] hover:border-[#2dd4bf]/30 hover:text-[#fafafa]"
          )}
        >
          {cta}
        </a>
      )}

      {tier.enterprise ? (
        <a
          href="mailto:hello@riftl.ink?subject=Rift%20Scale%20tier%20inquiry"
          className="mt-2 text-center text-[12px] text-[#52525b] hover:text-[#2dd4bf] transition-colors"
        >
          Need SSO or a larger plan? Talk to us →
        </a>
      ) : null}
    </div>
  );
}
