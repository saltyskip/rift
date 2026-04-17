import type { Metadata } from "next";
import Link from "next/link";
import { getAllCompetitors } from "@/lib/competitors";

const SITE_URL = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";

export const metadata: Metadata = {
  title: "Alternatives — Rift",
  description:
    "Honest head-to-head comparisons between Rift and the major deep-linking, attribution, and URL-shortener platforms. Pick the right tool for your use case.",
  alternates: { canonical: "/alternatives" },
  openGraph: {
    type: "website",
    url: `${SITE_URL}/alternatives`,
    title: "Alternatives — Rift",
    description:
      "Honest head-to-head comparisons between Rift and the major deep-linking, attribution, and URL-shortener platforms.",
  },
  twitter: {
    card: "summary",
    title: "Alternatives — Rift",
    description:
      "Honest head-to-head comparisons between Rift and the major deep-linking, attribution, and URL-shortener platforms.",
  },
};

export default function AlternativesIndex() {
  const competitors = getAllCompetitors();

  const breadcrumbJsonLd = {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [
      { "@type": "ListItem", position: 1, name: "Home", item: SITE_URL },
      {
        "@type": "ListItem",
        position: 2,
        name: "Alternatives",
        item: `${SITE_URL}/alternatives`,
      },
    ],
  };

  return (
    <div className="mx-auto max-w-3xl px-6 py-14">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(breadcrumbJsonLd) }}
      />

      <header className="mb-14">
        <p className="mb-3 text-[13px] font-medium uppercase tracking-widest text-[#2dd4bf]">
          Alternatives
        </p>
        <h1 className="mb-4 text-4xl font-bold text-[#fafafa]">
          Pick the right tool for the job
        </h1>
        <p className="text-lg leading-relaxed text-[#a1a1aa]">
          Head-to-head comparisons between Rift and the major deep-linking,
          attribution, and URL-shortener platforms. Each page has a full feature
          matrix, honest pricing at three scales, and a migration guide if Rift
          turns out to be the right move.
        </p>
      </header>

      <ul className="divide-y divide-[#1e1e22]">
        {competitors.map((c) => (
          <li key={c.slug} className="py-8 first:pt-0">
            <Link href={`/alternatives/${c.slug}`} className="group block">
              <div className="mb-2 flex items-center gap-3 text-[12px] uppercase tracking-widest text-[#52525b]">
                <span className="text-[#2dd4bf]">{c.category}</span>
              </div>
              <h2 className="mb-2 text-xl font-semibold text-[#fafafa] transition-colors group-hover:text-[#2dd4bf]">
                Rift vs {c.name}
              </h2>
              <p className="text-[15px] leading-relaxed text-[#a1a1aa]">
                {c.tagline}
              </p>
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
