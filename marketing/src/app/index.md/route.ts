import { buildIndexMarkdown } from "@/lib/agent-docs";

export function GET() {
  return new Response(buildIndexMarkdown(), {
    headers: {
      "Content-Type": "text/markdown; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
