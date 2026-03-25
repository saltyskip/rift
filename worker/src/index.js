/**
 * Rift Edge Worker
 *
 * Routes custom domain traffic to the Rift API origin,
 * forwarding the original Host as X-Rift-Host (custom header
 * to avoid Cloudflare overwriting X-Forwarded-Host on outbound fetches).
 */

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const host = url.hostname;

    // Build the upstream URL, preserving path and query string.
    const origin = env.API_ORIGIN || "https://api.riftl.ink";
    const upstream = new URL(url.pathname + url.search, origin);

    // Forward the request with the original Host as X-Rift-Host.
    const headers = new Headers(request.headers);
    headers.set("X-Rift-Host", host);

    const response = await fetch(upstream.toString(), {
      method: request.method,
      headers,
      body: request.method !== "GET" && request.method !== "HEAD" ? request.body : undefined,
      redirect: "manual",
    });

    return response;
  },
};
