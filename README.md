# Rift

**Deep links for humans and agents.**

Rift is a deep linking platform that creates smart, cross-platform links with structured context for AI agents. One link works across iOS, Android, and web — with built-in click tracking, attribution, webhooks, and machine-readable metadata.

## Why Rift

- **Cross-platform deep links** — one URL routes to the right destination on iOS, Android, or web
- **Agent-readable** — structured `agent_context` and JSON-LD so AI agents understand what a link does and can act on it
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

The server uses a **vertical slice architecture** — each domain (auth, links, webhooks, etc.) has its own directory under `api/` with routes, models, and repository.

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

Returns link destinations, `agent_context`, and `_rift_meta` with trust signals.

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
| `MONGO_URI` / `MONGO_DB` | No | MongoDB connection (server boots without it) |
| `RESEND_API_KEY` | No | Email verification via Resend |
| `PUBLIC_URL` | No | Base URL for links and verification emails |
| `SENTRY_DSN` | No | Error tracking |
| `X402_ENABLED` | No | Enable x402 crypto payments |
| `PRIMARY_DOMAIN` | No | Primary domain for link resolution (default `riftl.ink`) |

See `.env.example` for the full list.

## License

The server (`server/`) is licensed under the [Business Source License 1.1](LICENSE). You can use, modify, and self-host it freely — you just can't use it to run a competing commercial deep linking service. The license converts to Apache 2.0 on 2030-03-25.

The SDKs (`sdk/`) are licensed under [Apache 2.0](sdk/LICENSE) — use them freely in your apps with no restrictions.
