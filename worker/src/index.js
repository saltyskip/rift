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

    // Forward the request with the original Host and Cloudflare geo data.
    const headers = new Headers(request.headers);
    headers.set("X-Rift-Host", host);

    // Cloudflare provides geo data on every request — forward it to the API.
    const cf = request.cf || {};
    if (cf.country) headers.set("X-Rift-Country", cf.country);
    if (cf.city) headers.set("X-Rift-City", cf.city);
    if (cf.region) headers.set("X-Rift-Region", cf.region);

    const response = await fetch(upstream.toString(), {
      method: request.method,
      headers,
      body: request.method !== "GET" && request.method !== "HEAD" ? request.body : undefined,
      redirect: "manual",
    });

    return response;
  },
};
