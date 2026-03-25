# Relay Edge Worker

Cloudflare Worker that routes custom domain traffic to the Relay API.

## How it works

When a request arrives at a custom domain (e.g. `go.tablefour.com/book-downtown`), the worker:

1. Reads the `Host` header (the custom domain)
2. Forwards the request to the Relay API origin
3. Sets `X-Forwarded-Host` to the original custom domain
4. The API uses this header to look up the domain → tenant mapping and resolve the link

## Setup

### 1. Cloudflare account & domain

1. Create a [Cloudflare account](https://dash.cloudflare.com/sign-up)
2. Add `riftl.ink` as a domain — update your registrar's nameservers to Cloudflare's
3. Wait for DNS propagation (usually a few minutes)

### 2. Deploy the worker

```bash
npm install -g wrangler
wrangler login
cd worker
wrangler deploy
```

### 3. Configure the worker route

In Cloudflare dashboard, add a route for `riftl.ink/*` pointing to the `relay-edge` worker.

Update `API_ORIGIN` in `wrangler.toml` if your API is hosted somewhere other than `https://api.riftl.ink`.

### 4. Custom domain flow (for tenants)

1. Tenant registers their domain via `POST /v1/domains` → gets DNS instructions
2. Tenant creates a CNAME: `go.tablefour.com` → `riftl.ink`
3. Tenant creates a TXT record: `_rift-verify.go.tablefour.com` → `<verification_token>`
4. Tenant calls `POST /v1/domains/go.tablefour.com/verify` to confirm ownership
5. Cloudflare automatically handles SSL for the custom domain (via its edge)
6. Traffic to `go.tablefour.com/book-downtown` hits the worker → API resolves the link

## Local development

```bash
cd worker
wrangler dev
```

Then test with:

```bash
curl -H "Host: go.test.com" http://localhost:8787/SOME-LINK-ID
```
