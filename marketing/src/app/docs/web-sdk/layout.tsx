import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Web SDK — Rift Docs",
  description:
    "Track clicks and copy link IDs for deferred deep linking with rift.js.",
  alternates: { canonical: "/docs/web-sdk" },
};

export default function WebSdkLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}
