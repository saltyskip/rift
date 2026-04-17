import type { PostFrontmatter } from "@/lib/blog";

const SITE_URL = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";

export function ArticleSchema({ post }: { post: PostFrontmatter }) {
  const url = `${SITE_URL}/blog/${post.slug}`;

  const author = {
    "@type": "Person",
    name: post.author.name,
    ...(post.author.url ? { url: post.author.url } : {}),
    ...(post.author.github || post.author.twitter
      ? {
          sameAs: [post.author.github, post.author.twitter].filter(
            Boolean,
          ) as string[],
        }
      : {}),
  };

  const articleJsonLd = {
    "@context": "https://schema.org",
    "@type": "TechArticle",
    headline: post.title,
    description: post.description,
    datePublished: post.publishedAt,
    dateModified: post.updatedAt || post.publishedAt,
    author,
    publisher: {
      "@type": "Organization",
      name: "Rift",
      logo: {
        "@type": "ImageObject",
        url: `${SITE_URL}/logo.svg`,
      },
    },
    mainEntityOfPage: {
      "@type": "WebPage",
      "@id": url,
    },
    url,
    keywords: [post.targetKeyword, ...(post.secondaryKeywords || [])].join(", "),
  };

  const breadcrumbJsonLd = {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: [
      {
        "@type": "ListItem",
        position: 1,
        name: "Home",
        item: SITE_URL,
      },
      {
        "@type": "ListItem",
        position: 2,
        name: "Blog",
        item: `${SITE_URL}/blog`,
      },
      {
        "@type": "ListItem",
        position: 3,
        name: post.title,
        item: url,
      },
    ],
  };

  return (
    <>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(articleJsonLd) }}
      />
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{ __html: JSON.stringify(breadcrumbJsonLd) }}
      />
    </>
  );
}
