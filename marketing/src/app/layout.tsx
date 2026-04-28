import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import { GoogleAnalytics } from "@next/third-parties/google";
import { Analytics } from "@vercel/analytics/next";
import "./globals.css";
import { Navbar } from "@/components/navbar";
import { Footer } from "@/components/footer";

const inter = Inter({
  variable: "--font-outfit",
  subsets: ["latin"],
  display: "swap",
});

const jetbrainsMono = JetBrains_Mono({
  variable: "--font-mono",
  subsets: ["latin"],
  display: "swap",
});

const siteUrl = process.env.NEXT_PUBLIC_SITE_URL || "https://riftl.ink";
const xUrl = "https://x.com/riftlinks";
const googleSiteVerification = process.env.NEXT_PUBLIC_GOOGLE_SITE_VERIFICATION;
const gaMeasurementId = process.env.NEXT_PUBLIC_GA_MEASUREMENT_ID;

const title = "Riftl.ink — Deep links for humans and agents";
const description =
  "One link, two audiences. Humans click and get redirected. Agents resolve and get structured JSON. Track every click, install, and conversion.";

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title,
  description,
  // Do NOT set alternates.canonical here — it would cascade to every page
  // and tell Google they are all duplicates of the homepage.  Each page sets
  // its own canonical in its metadata export instead.
  openGraph: {
    type: "website",
    url: siteUrl,
    siteName: "Riftl.ink",
    title,
    description,
  },
  twitter: {
    card: "summary",
    title,
    description,
    site: "@riftlinks",
    creator: "@riftlinks",
  },
  icons: {
    icon: "/logo.svg",
  },
  verification: googleSiteVerification
    ? {
        google: googleSiteVerification,
      }
    : undefined,
};

const organizationJsonLd = {
  "@context": "https://schema.org",
  "@type": "Organization",
  name: "Riftl.ink",
  url: siteUrl,
  logo: `${siteUrl}/logo.svg`,
  description,
  sameAs: ["https://github.com/saltyskip/rift", xUrl],
};

const softwareJsonLd = {
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  name: "Riftl.ink",
  applicationCategory: "DeveloperApplication",
  operatingSystem: "Web, iOS, Android",
  url: siteUrl,
  description,
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
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body
        className={`${inter.variable} ${jetbrainsMono.variable} antialiased`}
        style={{ fontFamily: "var(--font-outfit), system-ui, sans-serif" }}
      >
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(organizationJsonLd) }}
        />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(softwareJsonLd) }}
        />
        <Navbar />
        <main>{children}</main>
        <Footer />
        <Analytics />
      </body>
      {gaMeasurementId ? <GoogleAnalytics gaId={gaMeasurementId} /> : null}
    </html>
  );
}
