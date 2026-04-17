import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getAllCompetitorSlugs, getCompetitor } from "@/lib/competitors";
import { ComparisonPage } from "@/components/alternatives/comparison-page";

const SITE_URL = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";

interface Params {
  competitor: string;
}

export async function generateStaticParams(): Promise<Params[]> {
  return getAllCompetitorSlugs().map((competitor) => ({ competitor }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<Params>;
}): Promise<Metadata> {
  const { competitor: slug } = await params;
  const competitor = getCompetitor(slug);
  if (!competitor) return {};
  const url = `${SITE_URL}/alternatives/${competitor.slug}`;
  return {
    title: `${competitor.name} Alternative — Rift`,
    description: competitor.metaDescription,
    alternates: { canonical: `/alternatives/${competitor.slug}` },
    openGraph: {
      type: "website",
      url,
      title: `${competitor.name} Alternative — Rift`,
      description: competitor.metaDescription,
    },
    twitter: {
      card: "summary_large_image",
      title: `${competitor.name} Alternative — Rift`,
      description: competitor.metaDescription,
    },
    keywords: [competitor.targetKeyword, ...competitor.secondaryKeywords],
  };
}

export default async function AlternativePage({
  params,
}: {
  params: Promise<Params>;
}) {
  const { competitor: slug } = await params;
  const competitor = getCompetitor(slug);
  if (!competitor) notFound();
  return <ComparisonPage competitor={competitor} />;
}
