import type { MetadataRoute } from "next";

const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";

const routes = [
  "",
  "/api-reference",
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

const lastModified = new Date();

export default function sitemap(): MetadataRoute.Sitemap {
  return routes.map((route) => ({
    url: `${siteUrl}${route}`,
    lastModified,
  }));
}
