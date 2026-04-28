import type { NextRequest } from "next/server";
import { NextResponse } from "next/server";

const linkHeaders = [
  '</sitemap.xml>; rel="sitemap"',
  '</index.md>; rel="alternate"; type="text/markdown"',
  '</llms.txt>; rel="alternate"; type="text/plain"',
  '</pricing.md>; rel="alternate"; type="text/markdown"',
  '</.well-known/api-catalog>; rel="service-desc"; type="application/linkset+json"; profile="https://www.rfc-editor.org/info/rfc9727"',
  '</.well-known/openapi.json>; rel="service-desc"; type="application/json"',
  '</.well-known/mcp.json>; rel="alternate"; type="application/json"',
];

function attachHeaders(response: NextResponse) {
  linkHeaders.forEach((value) => response.headers.append("Link", value));
  return response;
}

export function proxy(request: NextRequest) {
  if (
    request.nextUrl.pathname === "/" &&
    request.nextUrl.searchParams.get("mode") === "agent"
  ) {
    const url = request.nextUrl.clone();
    url.pathname = "/agent-mode.json";
    url.search = "";
    return attachHeaders(NextResponse.rewrite(url));
  }

  return attachHeaders(NextResponse.next());
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
