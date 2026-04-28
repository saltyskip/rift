/**
 * Single source of truth for the pricing tiers. Read by the pricing section,
 * the /checkout landing, and any other surface that needs tier labels, prices,
 * or feature lists.
 *
 * Adding a new tier: append an entry here. Both `pricing-section.tsx` and
 * `checkout/page.tsx` (and any future consumers) pick it up automatically.
 */

export type TierAudience = "human" | "agent";

export type PaidTierSlug = "pro" | "business" | "scale";
export type TierSlug = "free" | PaidTierSlug;

export interface TierPrice {
  price: string;
  unit?: string;
}

export interface Tier {
  /** Lowercase slug used in URLs and server APIs (`/checkout?tier=X`). */
  slug: TierSlug;
  /** Display name, Title-cased for headings. */
  name: string;
  human: TierPrice;
  agent: TierPrice;
  desc: string;
  /** Short quantitative limits shown stacked on the card. */
  stats: string[];
  /** Previous tier whose feature list this tier inherits. */
  inherits?: string;
  /** Delta features this tier adds on top of the inherited list. */
  items: string[];
  accent?: boolean;
  enterprise?: boolean;
}

function formatTierPrice(price: TierPrice): string {
  return `${price.price}${price.unit ? ` ${price.unit}` : ""}`;
}

function formatTierFeatures(tier: Tier): string[] {
  return [
    ...(tier.inherits ? [`Everything in ${tier.inherits}`] : []),
    ...tier.items,
  ];
}

export const TIERS: Tier[] = [
  {
    slug: "free",
    name: "Free",
    human: { price: "$0" },
    agent: { price: "$0" },
    desc: "For prototyping",
    stats: ["50 links", "10k events / mo", "1 domain"],
    items: [
      "Full REST API + MCP server",
      "iOS, Android & Web SDKs",
      "Deep links + deferred deep linking",
      "Install attribution + click tracking",
      "Custom styled QR codes with logos",
      "30-day analytics retention",
      "Commercial use allowed",
    ],
  },
  {
    slug: "pro",
    name: "Pro",
    human: { price: "$18", unit: "/ month" },
    agent: { price: "$15", unit: "USDC / 30d" },
    desc: "For shipping",
    stats: ["2,000 links", "100k events / mo", "5 domains"],
    inherits: "Free",
    items: [
      "Conversion tracking",
      "Webhooks on every event",
      "1-year analytics retention",
      "Email support",
    ],
    accent: true,
  },
  {
    slug: "business",
    name: "Business",
    human: { price: "$55", unit: "/ month" },
    agent: { price: "$47", unit: "USDC / 30d" },
    desc: "For scaling teams",
    stats: ["20,000 links", "500k events / mo", "20 domains"],
    inherits: "Pro",
    items: [
      "Unlimited team members",
      "3-year analytics retention",
      "Priority email support",
    ],
  },
  {
    slug: "scale",
    name: "Scale",
    human: { price: "$199", unit: "/ month" },
    agent: { price: "$169", unit: "USDC / 30d" },
    desc: "For serious volume",
    stats: ["100,000 links", "2M events / mo", "Unlimited domains"],
    inherits: "Business",
    items: ["5-year analytics retention", "Dedicated Slack channel"],
    enterprise: true,
  },
];

/** Look up a tier by its URL slug. Used by `/checkout?tier=...`. */
export function getTierBySlug(slug: string): Tier | undefined {
  return TIERS.find((t) => t.slug === slug);
}

/** True when the given string is one of the paid tier slugs. */
export function isPaidTierSlug(value: string): value is PaidTierSlug {
  return value === "pro" || value === "business" || value === "scale";
}

export function buildPricingMarkdown(): string {
  const intro = [
    "# Riftl.ink Pricing",
    "",
    "Machine-readable pricing reference for Riftl.ink. This document is generated from the same tier data used by the website pricing UI.",
    "",
    "## Summary",
    "",
    "- Human checkout uses Stripe and monthly billing.",
    "- Agent checkout is priced in USDC via x402 and is marked coming soon in the UI.",
    "- Same product limits apply across the human and agent lanes.",
    "- No surprise overage billing. Upgrade when you need higher limits.",
    "",
    "## Tiers",
    "",
  ];

  const tierSections = TIERS.flatMap((tier) => [
    `### ${tier.name} (\`${tier.slug}\`)`,
    "",
    `- Description: ${tier.desc}`,
    `- Human price: ${formatTierPrice(tier.human)}`,
    `- Agent price: ${formatTierPrice(tier.agent)}`,
    `- Link limit: ${tier.stats[0]}`,
    `- Event limit: ${tier.stats[1]}`,
    `- Domain limit: ${tier.stats[2]}`,
    `- Recommended: ${tier.accent ? "yes" : "no"}`,
    `- Enterprise-oriented: ${tier.enterprise ? "yes" : "no"}`,
    "",
    "Features:",
    ...formatTierFeatures(tier).map((item) => `- ${item}`),
    "",
  ]);

  const notes = [
    "## Notes",
    "",
    "- The website pricing section is the canonical product surface for plan presentation.",
    "- This markdown file is intended for AI agents and comparison tooling.",
    "- For the live pricing UI, visit https://riftl.ink/#pricing",
    "",
  ];

  return [...intro, ...tierSections, ...notes].join("\n");
}
