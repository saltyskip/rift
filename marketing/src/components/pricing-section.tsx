"use client";

import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";

type Audience = "human" | "agent";

interface Tier {
  name: string;
  human: { price: string; unit?: string };
  agent: { price: string; unit?: string };
  desc: string;
  items: string[];
  accent?: boolean;
}

const TIERS: Tier[] = [
  {
    name: "Free",
    human: { price: "$0" },
    agent: { price: "$0" },
    desc: "For prototyping",
    items: [
      "50 links",
      "10,000 events / month",
      "1 custom domain",
      "Full API access",
      "Commercial use allowed",
    ],
  },
  {
    name: "Pro",
    human: { price: "$12", unit: "/ month" },
    agent: { price: "$10", unit: "USDC / 30d" },
    desc: "For shipping",
    items: [
      "2,000 links",
      "100,000 events / month",
      "5 custom domains",
      "Full API access",
      "Webhooks & analytics",
    ],
    accent: true,
  },
  {
    name: "Business",
    human: { price: "$39", unit: "/ month" },
    agent: { price: "$33", unit: "USDC / 30d" },
    desc: "For scaling",
    items: [
      "20,000 links",
      "500,000 events / month",
      "20 custom domains",
      "10 team seats",
      "Priority support",
    ],
  },
  {
    name: "Scale",
    human: { price: "$149", unit: "/ month" },
    agent: { price: "$125", unit: "USDC / 30d" },
    desc: "For serious volume",
    items: [
      "100,000 links",
      "2,000,000 events / month",
      "Unlimited domains",
      "Unlimited seats",
      "SLA & dedicated support",
    ],
  },
];

const fade = (delay: number) => ({
  initial: { opacity: 0, y: 20 },
  whileInView: { opacity: 1, y: 0 },
  viewport: { once: true },
  transition: { duration: 0.5, delay, ease: "easeOut" as const },
});

export function PricingSection() {
  const [audience, setAudience] = useState<Audience>("human");

  return (
    <section id="pricing" className="py-24 px-6">
      <div className="mx-auto max-w-6xl">
        <motion.div {...fade(0)}>
          <p className="text-[12px] font-mono text-[#2dd4bf] tracking-wide uppercase mb-3">
            Pricing
          </p>
          <h2 className="text-3xl font-semibold tracking-[-0.03em] mb-3">
            One product. Pay in dollars or USDC.
          </h2>
          <p className="text-[#71717a]">
            Same limits on both lanes. API access on every tier, free tier included. No credit card required.
          </p>
        </motion.div>

        <motion.div {...fade(0.1)} className="mt-8 mb-10 flex items-center gap-3">
          <AudienceToggle audience={audience} setAudience={setAudience} />
          <AnimatePresence mode="wait" initial={false}>
            <motion.span
              key={audience}
              initial={{ opacity: 0, x: -4 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 4 }}
              transition={{ duration: 0.18 }}
              className="text-[12px] font-mono text-[#52525b] tracking-wide"
            >
              {audience === "human"
                ? "Stripe · monthly or annual"
                : "x402 · USDC · no card, no email required"}
            </motion.span>
          </AnimatePresence>
        </motion.div>

        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3">
          {TIERS.map((tier, i) => (
            <TierCard
              key={tier.name}
              tier={tier}
              audience={audience}
              delay={i * 0.06}
            />
          ))}
        </div>

        <motion.p
          {...fade(0.3)}
          className="mt-10 text-[12px] text-[#52525b] text-center"
        >
          Hard limit on every tier — no surprise overage bills. Upgrade any time.
        </motion.p>
      </div>
    </section>
  );
}

function AudienceToggle({
  audience,
  setAudience,
}: {
  audience: Audience;
  setAudience: (a: Audience) => void;
}) {
  return (
    <div
      role="tablist"
      aria-label="Pricing audience"
      className="inline-flex rounded-full border border-[#222225] bg-[#111113] p-1"
    >
      {(["human", "agent"] as const).map((a) => {
        const active = audience === a;
        return (
          <button
            key={a}
            role="tab"
            aria-selected={active}
            onClick={() => setAudience(a)}
            className="relative px-5 py-1.5 text-[13px] font-medium transition-colors"
            style={{ color: active ? "#042f2e" : "#a1a1aa" }}
          >
            {active && (
              <motion.span
                layoutId="pricing-pill"
                className="absolute inset-0 rounded-full bg-[#2dd4bf]"
                transition={{ type: "spring", stiffness: 400, damping: 35 }}
              />
            )}
            <span className="relative z-10 capitalize">{a}s</span>
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
  const isPaid = tier.name !== "Free";
  const cta = !isPaid
    ? "Start free"
    : audience === "agent"
      ? "Pay with wallet"
      : `Get ${tier.name}`;

  return (
    <motion.div
      {...fade(delay)}
      className={`relative rounded-xl border p-7 flex flex-col ${
        tier.accent
          ? "border-[#2dd4bf]/30 bg-[#2dd4bf]/[0.03] glow-teal"
          : "border-[#222225] bg-[#111113] hover:border-[#2dd4bf]/15"
      } transition-colors`}
    >
      {tier.accent && (
        <span className="absolute -top-2.5 left-7 text-[10px] font-mono text-[#042f2e] bg-[#2dd4bf] px-2 py-0.5 rounded-full tracking-wide uppercase">
          Recommended
        </span>
      )}
      <p className="text-[11px] font-mono text-[#52525b] uppercase tracking-widest mb-3">
        {tier.name}
      </p>

      <div className="relative h-[44px] mb-1">
        <AnimatePresence mode="wait" initial={false}>
          <motion.div
            key={`${tier.name}-${audience}`}
            initial={{ opacity: 0, y: -6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 6 }}
            transition={{ duration: 0.18 }}
            className="absolute inset-0 flex items-baseline gap-1"
          >
            <span className="text-3xl font-semibold tracking-tight">
              {price.price}
            </span>
            {price.unit && (
              <span className="text-sm text-[#52525b]">{price.unit}</span>
            )}
          </motion.div>
        </AnimatePresence>
      </div>

      <p className="text-[13px] text-[#52525b] mb-6">{tier.desc}</p>
      <div className="h-px bg-[#222225] mb-5" />

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

      <a
        href="#"
        className={`text-center text-[13px] font-medium px-4 py-2 rounded-lg transition-colors ${
          tier.accent
            ? "bg-[#2dd4bf] text-[#042f2e] hover:bg-[#5eead4]"
            : "border border-[#222225] text-[#a1a1aa] hover:border-[#2dd4bf]/30 hover:text-[#fafafa]"
        }`}
      >
        {cta}
      </a>
    </motion.div>
  );
}
