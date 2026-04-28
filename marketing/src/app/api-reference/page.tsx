import type { Metadata } from "next";
import { ScalarDocs } from "./scalar-docs";

export const metadata: Metadata = {
  title: "Riftl.ink API Reference — Rift",
  description:
    "Interactive API documentation for Riftl.ink.",
  alternates: { canonical: "/api-reference" },
};

export default function ApiReferencePage() {
  return <ScalarDocs />;
}
