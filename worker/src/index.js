/**
 * Relay Edge Worker
 *
 * Routes custom domain traffic to the Relay API origin,
 * forwarding the original Host header as X-Forwarded-Host
 * so the API can resolve links for the correct tenant.
 */

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    const host = url.hostname;

    // Build the upstream URL, preserving path and query string.
    const origin = env.API_ORIGIN || "https://api.riftl.ink";
    const upstream = new URL(url.pathname + url.search, origin);

    // Forward the request with the original Host as X-Forwarded-Host.
    const headers = new Headers(request.headers);
    headers.set("X-Forwarded-Host", host);

    const response = await fetch(upstream.toString(), {
      method: request.method,
      headers,
      body: request.method !== "GET" && request.method !== "HEAD" ? request.body : undefined,
      redirect: "manual",
    });

    return response;
  },
};
