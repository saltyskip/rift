import { buildPricingMarkdown } from "@/lib/tiers";

export function GET() {
  return new Response(buildPricingMarkdown(), {
    headers: {
      "Content-Type": "text/markdown; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
