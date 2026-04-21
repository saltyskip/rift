import type { Metadata } from "next";
import { ScalarDocs } from "./scalar-docs";

export const metadata: Metadata = {
  title: "API Reference — Rift",
  description:
    "Interactive API documentation for Rift.",
  alternates: { canonical: "/api-reference" },
};

export default function ApiReferencePage() {
  return <ScalarDocs />;
}
