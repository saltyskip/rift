import Link from "next/link";
import type { Competitor, FeatureValue } from "@/lib/competitors";

const SITE_URL = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";

function FeatureCell({ value }: { value: FeatureValue }) {
  switch (value.kind) {
    case "yes":
      return (
        <span className="flex items-center gap-2 text-[#2dd4bf]">
          <span aria-hidden>✓</span>
          {value.note ? <span className="text-[13px] text-[#a1a1aa]">{value.note}</span> : null}
        </span>
      );
    case "no":
      return (
        <span className="flex items-center gap-2 text-[#52525b]">
          <span aria-hidden>—</span>
          {value.note ? <span className="text-[13px] text-[#71717a]">{value.note}</span> : null}
        </span>
      );
    case "partial":
      return (
        <span className="flex items-center gap-2 text-[#f59e0b]">
          <span aria-hidden>◐</span>
          <span className="text-[13px] text-[#a1a1aa]">{value.note}</span>
        </span>
      );
    case "text":
      return <span className="text-[13px] text-[#a1a1aa]">{value.value}</span>;
  }
}

function Schemas({ competitor }: { competitor: Competitor }) {
  const pageUrl = `${SITE_URL}/alternatives/${competitor.slug}`;

  const productJsonLd = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: "Rift",
    applicationCategory: "DeveloperApplication",
    operatingSystem: "Web, iOS, Android",
    url: SITE_URL,
    description: competitor.metaDescription,
    offers: [
      {
        "@type": "Offer",
        name: "Free",
        price: "0",
        priceCurrency: "USD",
        description: "100 links, 1,000 clicks/month",
      },
      {
        "@type": "Offer",
        name: "Pay per request",
        price: "0.01",
        priceCurrency: "USD",
        description: "Per request, unlimited links and clicks",
      },
    ],
    isSimilarTo: {
      "@type": "SoftwareApplication",
      name: competitor.name,
      url: `https://${competitor.domain}`,
    },
  };

  const breadcrumbJsonLd = {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [
      { "@type": "ListItem", position: 1, name: "Home", item: SITE_URL },
      { "@type": "ListItem", position: 2, name: "Alternatives", item: `${SITE_URL}/alternatives` },
      { "@type": "ListItem", position: 3, name: competitor.name, item: pageUrl },
    ],
  };

  const faqJsonLd = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: competitor.faqs.map((f) => ({
      "@type": "Question",
      name: f.q,
      acceptedAnswer: { "@type": "Answer", text: f.a },
    })),
  };

  return (
    <>
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(productJsonLd) }} />
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(breadcrumbJsonLd) }} />
      <script type="application/ld+json" dangerouslySetInnerHTML={{ __html: JSON.stringify(faqJsonLd) }} />
    </>
  );
}

export function ComparisonPage({ competitor }: { competitor: Competitor }) {
  return (
    <article className="mx-auto max-w-4xl px-6 py-14">
      <Schemas competitor={competitor} />

      <nav className="mb-8 text-[13px]">
        <Link href="/alternatives" className="text-[#71717a] transition-colors hover:text-[#2dd4bf]">
          ← All alternatives
        </Link>
      </nav>

      <header className="mb-12">
        <p className="mb-3 text-[13px] font-medium uppercase tracking-widest text-[#2dd4bf]">
          {competitor.name} alternative
        </p>
        <h1 className="mb-5 text-5xl font-bold leading-tight text-[#fafafa]">{competitor.headline}</h1>
        <p className="mb-8 text-lg leading-relaxed text-[#a1a1aa]">{competitor.oneLiner}</p>
        <div className="flex flex-wrap gap-3">
          <Link
            href="/docs"
            className="rounded-lg bg-[#2dd4bf] px-4 py-2 text-[14px] font-medium text-[#042f2e] transition-colors hover:bg-[#5eead4]"
          >
            Get started free
          </Link>
          <Link
            href="#pricing"
            className="rounded-lg border border-[#1e1e22] px-4 py-2 text-[14px] font-medium text-[#fafafa] transition-colors hover:border-[#2dd4bf]"
          >
            See pricing
          </Link>
        </div>
      </header>

      <section className="mb-14">
        <h2 className="mb-5 text-2xl font-bold text-[#fafafa]">
          Rift vs {competitor.name}: side by side
        </h2>
        <div className="overflow-x-auto rounded-xl border border-[#1e1e22]">
          <table className="w-full text-left text-[14px]">
            <thead>
              <tr className="border-b border-[#1e1e22] bg-[#0c0c0e]">
                <th className="px-4 py-3 font-semibold text-[#fafafa]">Feature</th>
                <th className="px-4 py-3 font-semibold text-[#2dd4bf]">Rift</th>
                <th className="px-4 py-3 font-semibold text-[#a1a1aa]">{competitor.name}</th>
              </tr>
            </thead>
            <tbody>
              {competitor.features.map((row, i) => (
                <tr key={i} className="border-b border-[#1e1e22] last:border-b-0">
                  <td className="px-4 py-3 text-[#a1a1aa]">{row.label}</td>
                  <td className="px-4 py-3">
                    <FeatureCell value={row.rift} />
                  </td>
                  <td className="px-4 py-3">
                    <FeatureCell value={row.competitor} />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <section className="mb-14 grid gap-8 md:grid-cols-2">
        <div>
          <h2 className="mb-4 text-xl font-bold text-[#fafafa]">Why teams switch from {competitor.name}</h2>
          <ul className="space-y-3">
            {competitor.whyLeave.map((reason, i) => (
              <li key={i} className="flex gap-3 text-[14px] leading-relaxed text-[#a1a1aa]">
                <span className="mt-[6px] h-1.5 w-1.5 flex-shrink-0 rounded-full bg-[#2dd4bf]" />
                <span>{reason}</span>
              </li>
            ))}
          </ul>
        </div>
        <div>
          <h2 className="mb-4 text-xl font-bold text-[#fafafa]">Where {competitor.name} is still the better pick</h2>
          <ul className="space-y-3">
            {competitor.whereBetter.map((reason, i) => (
              <li key={i} className="flex gap-3 text-[14px] leading-relaxed text-[#a1a1aa]">
                <span className="mt-[6px] h-1.5 w-1.5 flex-shrink-0 rounded-full bg-[#52525b]" />
                <span>{reason}</span>
              </li>
            ))}
          </ul>
        </div>
      </section>

      <section id="pricing" className="mb-14">
        <h2 className="mb-5 text-2xl font-bold text-[#fafafa]">Pricing at three scales</h2>
        <div className="overflow-x-auto rounded-xl border border-[#1e1e22]">
          <table className="w-full text-left text-[14px]">
            <thead>
              <tr className="border-b border-[#1e1e22] bg-[#0c0c0e]">
                <th className="px-4 py-3 font-semibold text-[#fafafa]">Scale</th>
                <th className="px-4 py-3 font-semibold text-[#2dd4bf]">Rift</th>
                <th className="px-4 py-3 font-semibold text-[#a1a1aa]">{competitor.name}</th>
              </tr>
            </thead>
            <tbody>
              {competitor.pricing.map((row, i) => (
                <tr key={i} className="border-b border-[#1e1e22] last:border-b-0">
                  <td className="px-4 py-3 text-[#fafafa]">{row.scale}</td>
                  <td className="px-4 py-3 text-[#a1a1aa]">{row.rift}</td>
                  <td className="px-4 py-3 text-[#a1a1aa]">{row.competitor}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <section className="mb-14">
        <h2 className="mb-5 text-2xl font-bold text-[#fafafa]">Migrating from {competitor.name} to Rift</h2>
        <ol className="space-y-4">
          {competitor.migrationSteps.map((step, i) => (
            <li key={i} className="flex gap-4">
              <span className="flex h-7 w-7 flex-shrink-0 items-center justify-center rounded-full border border-[#2dd4bf]/40 bg-[#2dd4bf]/10 text-[13px] font-semibold text-[#2dd4bf]">
                {i + 1}
              </span>
              <span className="pt-0.5 text-[15px] leading-relaxed text-[#a1a1aa]">{step}</span>
            </li>
          ))}
        </ol>
      </section>

      <section className="mb-14">
        <h2 className="mb-5 text-2xl font-bold text-[#fafafa]">Frequently asked questions</h2>
        <dl className="divide-y divide-[#1e1e22] rounded-xl border border-[#1e1e22] bg-[#0c0c0e]">
          {competitor.faqs.map((item, i) => (
            <div key={i} className="p-5">
              <dt className="text-[15px] font-semibold text-[#fafafa]">{item.q}</dt>
              <dd className="mt-2 text-[14px] leading-relaxed text-[#a1a1aa]">{item.a}</dd>
            </div>
          ))}
        </dl>
      </section>

      {competitor.relatedBlogPosts && competitor.relatedBlogPosts.length > 0 ? (
        <section className="mb-14">
          <h2 className="mb-5 text-xl font-bold text-[#fafafa]">Related reading</h2>
          <ul className="space-y-2">
            {competitor.relatedBlogPosts.map((post) => (
              <li key={post.href}>
                <Link
                  href={post.href}
                  className="text-[15px] text-[#2dd4bf] underline-offset-2 hover:underline"
                >
                  {post.title}
                </Link>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <section className="rounded-2xl border border-[#1e1e22] bg-[#0c0c0e] p-8 text-center">
        <h2 className="mb-3 text-2xl font-bold text-[#fafafa]">
          Ready to ship without the sales cycle?
        </h2>
        <p className="mb-5 text-[15px] leading-relaxed text-[#a1a1aa]">
          Free tier, no credit card. Create your first link in under a minute.
        </p>
        <Link
          href="/docs"
          className="inline-block rounded-lg bg-[#2dd4bf] px-5 py-2.5 text-[14px] font-medium text-[#042f2e] transition-colors hover:bg-[#5eead4]"
        >
          Get your API key
        </Link>
      </section>
    </article>
  );
}
