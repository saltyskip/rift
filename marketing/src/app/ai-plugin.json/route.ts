import { siteUrl } from "@/lib/agent-docs";

const body = {
  schema_version: "v1",
  name_for_human: "Riftl.ink",
  name_for_model: "riftl",
  description_for_human:
    "Deep linking and attribution API for humans and AI agents.",
  description_for_model:
    "Use Riftl.ink to create, inspect, and resolve deep links with attribution metadata. Authenticate with a bearer token using an rl_live_ secret key.",
  auth: {
    type: "service_http",
    authorization_type: "bearer",
  },
  api: {
    type: "openapi",
    url: `${siteUrl}/.well-known/openapi.json`,
    is_user_authenticated: true,
  },
  logo_url: `${siteUrl}/logo.svg`,
  contact_email: "hello@riftl.ink",
  legal_info_url: `${siteUrl}/privacy`,
};

export function GET() {
  return new Response(JSON.stringify(body, null, 2), {
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "public, max-age=3600",
    },
  });
}
