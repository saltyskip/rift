import type { Metadata } from "next";
import Link from "next/link";
import { getAllPosts } from "@/lib/blog";

const SITE_URL = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";

export const metadata: Metadata = {
  title: "Blog — Rift",
  description:
    "Deep linking, attribution, and AI-agent infrastructure. Guides, migrations, and engineering notes from the team building Rift.",
  alternates: { canonical: "/blog" },
  openGraph: {
    type: "website",
    url: `${SITE_URL}/blog`,
    title: "Blog — Rift",
    description:
      "Deep linking, attribution, and AI-agent infrastructure. Guides, migrations, and engineering notes.",
  },
  twitter: {
    card: "summary",
    title: "Blog — Rift",
    description:
      "Deep linking, attribution, and AI-agent infrastructure. Guides, migrations, and engineering notes.",
  },
};

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

export default function BlogIndex() {
  const posts = getAllPosts();

  const breadcrumbJsonLd = {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [
      { "@type": "ListItem", position: 1, name: "Home", item: SITE_URL },
      {
        "@type": "ListItem",
        position: 2,
        name: "Blog",
        item: `${SITE_URL}/blog`,
      },
    ],
  };

  const blogJsonLd = {
    "@context": "https://schema.org",
    "@type": "Blog",
    url: `${SITE_URL}/blog`,
    name: "Rift Blog",
    description:
      "Deep linking, attribution, and AI-agent infrastructure — from the team building Rift.",
    blogPost: posts.map((p) => ({
      "@type": "BlogPosting",
      headline: p.frontmatter.title,
      url: `${SITE_URL}/blog/${p.slug}`,
      datePublished: p.frontmatter.publishedAt,
      author: {
        "@type": "Person",
        name: p.frontmatter.author.name,
      },
    })),
  };

  return (
    <div className="mx-auto max-w-3xl px-6 py-14">
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(breadcrumbJsonLd) }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(blogJsonLd) }}
      />

      <header className="mb-14">
        <p className="mb-3 text-[13px] font-medium uppercase tracking-widest text-[#2dd4bf]">
          Writing
        </p>
        <h1 className="mb-4 text-4xl font-bold text-[#fafafa]">Blog</h1>
        <p className="text-lg leading-relaxed text-[#a1a1aa]">
          Guides, migrations, and engineering notes on deep linking, attribution,
          and building infrastructure for humans and AI agents.
        </p>
      </header>

      {posts.length === 0 ? (
        <p className="text-[15px] text-[#71717a]">No posts yet — check back soon.</p>
      ) : (
        <ul className="divide-y divide-[#1e1e22]">
          {posts.map((post) => (
            <li key={post.slug} className="py-8 first:pt-0">
              <Link
                href={`/blog/${post.slug}`}
                className="group block"
              >
                <div className="mb-2 flex items-center gap-3 text-[12px] uppercase tracking-widest text-[#52525b]">
                  <time dateTime={post.frontmatter.publishedAt}>
                    {formatDate(post.frontmatter.publishedAt)}
                  </time>
                  {post.frontmatter.category ? (
                    <>
                      <span className="text-[#1e1e22]">·</span>
                      <span className="text-[#2dd4bf]">
                        {post.frontmatter.category}
                      </span>
                    </>
                  ) : null}
                </div>
                <h2 className="mb-2 text-xl font-semibold text-[#fafafa] transition-colors group-hover:text-[#2dd4bf]">
                  {post.frontmatter.title}
                </h2>
                <p className="text-[15px] leading-relaxed text-[#a1a1aa]">
                  {post.frontmatter.description}
                </p>
              </Link>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
