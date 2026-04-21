import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Deep Link Audit — Rift",
  description: "Audit your deep linking setup with Rift.",
  alternates: { canonical: "/tools/audit" },
};

export default function AuditLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return children;
}
