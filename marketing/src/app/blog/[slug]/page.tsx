import type { Metadata } from "next";
import { notFound } from "next/navigation";
import Link from "next/link";
import { getAllSlugs, getPostBySlug } from "@/lib/blog";
import { ArticleSchema } from "@/components/blog/article-schema";

const SITE_URL = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";

interface Params {
  slug: string;
}

export async function generateStaticParams(): Promise<Params[]> {
  return getAllSlugs().map((slug) => ({ slug }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<Params>;
}): Promise<Metadata> {
  const { slug } = await params;
  const post = getPostBySlug(slug);
  if (!post) return {};
  const fm = post.frontmatter;
  const url = `${SITE_URL}/blog/${fm.slug}`;
  return {
    title: `${fm.title} — Rift`,
    description: fm.description,
    alternates: { canonical: `/blog/${fm.slug}` },
    openGraph: {
      type: "article",
      url,
      title: fm.title,
      description: fm.description,
      publishedTime: fm.publishedAt,
      modifiedTime: fm.updatedAt || fm.publishedAt,
      authors: [fm.author.name],
      ...(fm.ogImage ? { images: [fm.ogImage] } : {}),
    },
    twitter: {
      card: "summary_large_image",
      title: fm.title,
      description: fm.description,
      ...(fm.ogImage ? { images: [fm.ogImage] } : {}),
    },
    authors: fm.author.url
      ? [{ name: fm.author.name, url: fm.author.url }]
      : [{ name: fm.author.name }],
  };
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

export default async function BlogPostPage({
  params,
}: {
  params: Promise<Params>;
}) {
  const { slug } = await params;
  const post = getPostBySlug(slug);
  if (!post) notFound();

  const { default: MDXContent } = await import(
    `../../../../content/blog/${slug}.mdx`
  );

  const fm = post.frontmatter;

  return (
    <article className="mx-auto max-w-3xl px-6 py-14">
      <ArticleSchema post={fm} />

      <nav className="mb-10 text-[13px]">
        <Link
          href="/blog"
          className="text-[#71717a] transition-colors hover:text-[#2dd4bf]"
        >
          ← All posts
        </Link>
      </nav>

      <header className="mb-12">
        {fm.category ? (
          <p className="mb-3 text-[13px] font-medium uppercase tracking-widest text-[#2dd4bf]">
            {fm.category}
          </p>
        ) : null}
        <h1 className="mb-5 text-4xl font-bold leading-tight text-[#fafafa]">
          {fm.title}
        </h1>
        <p className="mb-6 text-lg leading-relaxed text-[#a1a1aa]">
          {fm.description}
        </p>
        <div className="flex items-center gap-3 text-[13px] text-[#71717a]">
          <span className="text-[#fafafa]">{fm.author.name}</span>
          <span className="text-[#1e1e22]">·</span>
          <time dateTime={fm.publishedAt}>{formatDate(fm.publishedAt)}</time>
          {fm.updatedAt && fm.updatedAt !== fm.publishedAt ? (
            <>
              <span className="text-[#1e1e22]">·</span>
              <span>Updated {formatDate(fm.updatedAt)}</span>
            </>
          ) : null}
        </div>
      </header>

      <div className="article-body">
        <MDXContent />
      </div>

      {fm.author.bio ? (
        <footer className="mt-16 rounded-xl border border-[#1e1e22] bg-[#0c0c0e] p-6">
          <p className="mb-1 text-[12px] font-semibold uppercase tracking-[0.18em] text-[#2dd4bf]">
            About the author
          </p>
          <p className="mb-2 text-[15px] font-semibold text-[#fafafa]">
            {fm.author.name}
          </p>
          <p className="text-[14px] leading-relaxed text-[#a1a1aa]">
            {fm.author.bio}
          </p>
        </footer>
      ) : null}
    </article>
  );
}
