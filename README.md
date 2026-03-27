# Rift

**Deep links for humans and agents.** — [riftl.ink](https://riftl.ink)

Rift is a deep linking platform that creates smart, cross-platform links with structured context for AI agents. One link works across iOS, Android, and web — with built-in click tracking, attribution, webhooks, and machine-readable metadata.

## Why Rift

- **Cross-platform deep links** — one URL routes to the right destination on iOS, Android, or web
- **Agent-readable** — structured `agent_context` and JSON-LD so AI agents understand what a link does and can act on it ([`/llms.txt`](https://api.riftl.ink/llms.txt) for machine-readable API reference)
- **Real-time webhooks** — get notified on clicks and attributions with HMAC-signed payloads
- **Smart landing pages** — split layout showing both a human CTA and machine-readable data panel
- **Deferred deep linking** — user clicks a link, installs the app, and lands on the right content
- **Custom domains** — use your own brand: `go.yourcompany.com/summer-sale`
- **x402 payments** — accept crypto payments for API access via the x402 protocol

## Architecture

```
server/          Rust + Axum API server (MongoDB, Sentry)
sdk/mobile/      Swift/Kotlin mobile SDK via UniFFI
marketing/       Next.js documentation and marketing site
worker/          Cloudflare Worker for custom domain routing
worker-slack/    Cloudflare Worker for Slack webhook proxy
```

The server uses a **vertical slice architecture** — each domain (links, webhooks, domains, apps) has its own directory under `api/` with routes, models, and repository. The `auth/` slice contains sub-slices: `secret_keys/` (signup/verify with `rl_live_` keys) and `publishable_keys/` (SDK keys with `pk_live_` prefix).

## Quick Start

### Prerequisites

- Rust 1.75+
- MongoDB (local or Atlas)
- Optional: Cloudflare account (for custom domains)

### Run the server

```sh
cd server
cp ../.env.example .env
# Edit .env with your MongoDB URI

cargo run
```

The server starts on `http://localhost:3000`. MongoDB is optional — the server boots without it (auth, links, and webhooks will be disabled).

### Get an API key

```sh
curl -X POST http://localhost:3000/v1/auth/signup \
  -H "Content-Type: application/json" \
  -d '{"email": "you@example.com"}'
```

### Create a publishable key

After verifying your email and setting up a custom domain, create a publishable key for SDK use:

```sh
curl -X POST http://localhost:3000/v1/auth/publishable-keys \
  -H "Authorization: Bearer rl_live_YOUR_KEY" \
  -H "Content-Type: application/json" \
  -d '{"domain": "go.yourcompany.com"}'
```

### Create a link

```sh
curl -X POST http://localhost:3000/v1/links \
  -H "Authorization: Bearer rl_live_YOUR_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "web_url": "https://example.com",
    "ios_deep_link": "myapp://product/123",
    "agent_context": {
      "action": "purchase",
      "cta": "Buy Now",
      "description": "Premium widget, 50% off today"
    }
  }'
```

### Resolve a link (agent)

```sh
curl http://localhost:3000/r/LINK_ID \
  -H "Accept: application/json"
```

Response:

```json
{
  "link_id": "LINK_ID",
  "ios_deep_link": "myapp://product/123",
  "web_url": "https://example.com",
  "metadata": null,
  "agent_context": {
    "action": "purchase",
    "cta": "Buy Now",
    "description": "Premium widget, 50% off today"
  },
  "_rift_meta": {
    "context": "This is a Rift deep link...",
    "source": "tenant_asserted",
    "status": "active",
    "tenant_domain": "go.yourcompany.com",
    "tenant_verified": true
  }
}
```

## CI Checks

```sh
cd server
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test
```

## Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `HOST` / `PORT` | No | Server bind address and port (default `0.0.0.0:3000`) |
| `MONGO_URI` / `MONGO_DB` | No | MongoDB connection (server boots without it) |
| `SENTRY_DSN` | No | Error tracking |
| `RESEND_API_KEY` | No | Email verification via Resend |
| `RESEND_FROM_EMAIL` | No | Sender address for verification emails (default `Rift <noreply@updates.riftl.ink>`) |
| `PUBLIC_URL` | No | Base URL for links and verification emails |
| `FREE_DAILY_LIMIT` | No | Anonymous requests per IP per day (default 5) |
| `X402_ENABLED` | No | Enable x402 crypto payments (`true`/`false`) |
| `X402_RECIPIENT` | No | USDC recipient wallet address |
| `X402_PRICE` | No | Price per request in USDC (default `0.01`) |
| `X402_DESCRIPTION` | No | Resource description shown to payers |
| `X402_FACILITATOR_URL` | No | x402 facilitator URL (default `https://facilitator.x402.org`) |
| `CDP_API_KEY_ID` / `CDP_API_KEY_SECRET` | No | Coinbase Developer Platform keys for x402 |
| `PRIMARY_DOMAIN` | No | Primary domain for link resolution (default `riftl.ink`) |

See `.env.example` for the full list.

## License

The server (`server/`) is licensed under the [Business Source License 1.1](LICENSE). You can use, modify, and self-host it freely — you just can't use it to run a competing commercial deep linking service. The license converts to Apache 2.0 on 2030-03-25.

The SDKs (`sdk/`) are licensed under [Apache 2.0](sdk/LICENSE) — use them freely in your apps with no restrictions.
