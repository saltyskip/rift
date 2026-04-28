import { buildApiLlmsText } from "@/lib/agent-docs";

export function GET() {
  return new Response(buildApiLlmsText(), {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
