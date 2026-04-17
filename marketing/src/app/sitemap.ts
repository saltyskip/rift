import type { MetadataRoute } from "next";
import { getAllPosts } from "@/lib/blog";
import { getAllCompetitorSlugs } from "@/lib/competitors";

const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";

const staticRoutes = [
  "",
  "/alternatives",
  "/api-reference",
  "/blog",
  "/docs",
  "/docs/android-sdk",
  "/docs/apps",
  "/docs/attribution",
  "/docs/conversions",
  "/docs/deferred",
  "/docs/domains",
  "/docs/ios-sdk",
  "/docs/links",
  "/docs/manual-setup",
  "/docs/publishable-keys",
  "/docs/universal-links",
  "/docs/web-sdk",
  "/docs/webhooks",
  "/tools/audit",
];

export default function sitemap(): MetadataRoute.Sitemap {
  const now = new Date();

  const staticEntries = staticRoutes.map((route) => ({
    url: `${siteUrl}${route}`,
    lastModified: now,
  }));

  const posts = getAllPosts().map((post) => ({
    url: `${siteUrl}/blog/${post.slug}`,
    lastModified: new Date(
      post.frontmatter.updatedAt || post.frontmatter.publishedAt,
    ),
  }));

  const alternatives = getAllCompetitorSlugs().map((slug) => ({
    url: `${siteUrl}/alternatives/${slug}`,
    lastModified: now,
  }));

  return [...staticEntries, ...posts, ...alternatives];
}
