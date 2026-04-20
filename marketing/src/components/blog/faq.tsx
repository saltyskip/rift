export interface FaqItem {
  q: string;
  a: string;
}

export function FAQ({ items }: { items: FaqItem[] }) {
  const jsonLd = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: items.map((item) => ({
      "@type": "Question",
      name: item.q,
      acceptedAnswer: {
        "@type": "Answer",
        text: item.a,
      },
    })),
  };

  return (
    <section className="my-10 space-y-3">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
      />
      <h2 className="text-2xl font-bold text-[#fafafa]">
        Frequently asked questions
      </h2>
      <dl className="divide-y divide-[#1e1e22] rounded-xl border border-[#1e1e22] bg-[#0c0c0e]">
        {items.map((item, i) => (
          <div key={i} className="p-5">
            <dt className="text-[15px] font-semibold text-[#fafafa]">
              {item.q}
            </dt>
            <dd className="mt-2 text-[14px] leading-relaxed text-[#a1a1aa]">
              {item.a}
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
