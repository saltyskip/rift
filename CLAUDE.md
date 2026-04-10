# Rift

Deep links for humans and agents.

**Stack:** Rust, Axum, MongoDB, Sentry, x402

## Architecture

The server separates **domain logic** from **transport layers**:

- **`services/`** — Transport-agnostic business logic. Each domain (links, auth, domains, apps, webhooks) has its own directory with `models.rs`, `repo.rs`, and optionally `service.rs`
- **`api/`** — HTTP transport. Each slice has `mod.rs` (router) and `routes.rs` (handlers). Route handlers must be **thin wrappers**: extract HTTP params, call a service method, format the response. No business logic, validation, or database calls in route handlers — all of that belongs in `services/`
- **`mcp/`** — MCP protocol transport. Same rule: thin wrappers around service methods. Imports from `services/`, not from `api/`
- **`app.rs`** — `AppState` struct, shared across transports
- **`core/`** — Shared infra only (db connection, config, rate limiting) — no business logic
- **Transport layers must not import from each other** — both `api/` and `mcp/` import from `services/`
- **Domains own their models and repositories** in `services/<domain>/`
- **Auth sub-slices** — `services/auth/` contains `tenants/` (billing entity), `users/` (team members, email verification), `secret_keys/` (signup/verify/CRUD, `rl_live_` keys with `service.rs`), `publishable_keys/` (SDK keys, `pk_live_` prefix), and `usage/` (request tracking). Transport routes live in `api/auth/`

### Cargo Features

- `api` — HTTP API routes (enabled by default)
- `mcp` — MCP protocol server, pulls in `rmcp` and `schemars` (enabled by default)
- Both can be independently disabled: `cargo build --no-default-features --features mcp`
- **CI runs with default features (both enabled).** Individual feature subsets may produce dead-code warnings since `services/` is shared infrastructure

## Multi-Tenancy

All resources (links, domains, apps, webhooks, keys) are scoped by `tenant_id`. The data model
is Tenant → User → SecretKey: a tenant owns all resources, users are team members who authenticate
via email, and secret keys (`rl_live_`) are the API credentials. The auth middleware looks up
the secret key hash in the `secret_keys` collection, resolves the `tenant_id`, and injects
`TenantId` + `AuthKeyId` extensions. Route handlers extract these via `Extension<TenantId>`.

Team members invited via `POST /v1/auth/users` share the same tenant and all its resources.
Creating new secret keys requires email confirmation (6-char code sent to a verified team member).

Public endpoints (landing page, attribution reporting) resolve the tenant from the link_id itself.

## Custom Domains (Primary + Alternate)

Each tenant registers two custom domains: a **primary** domain for landing pages and link resolution, and an **alternate** domain used solely as a Universal Link trampoline. iOS/Android don't trigger Universal Links for same-domain taps, so the landing page "Open in App" button must point to a different domain.

- **Primary** (`go.example.com`) — serves landing pages, resolves links, records clicks
- **Alternate** (`open.example.com`) — ONLY handles the "Open in App" tap. No landing pages, no click recording, no analytics. If the app is installed, iOS intercepts the tap. If not, Rift redirects to the store.

Both domains use the same AASA serving and verification flow. Users point a CNAME (or A+AAAA records) at the Fly app, and TLS certificates are auto-provisioned via Let's Encrypt.

## Conversion Tracking

Post-install events (signups, purchases, deposits) flow through a **sources** abstraction. A source is a webhook receiver with a type-specific parser; v1 ships a `custom` source type that accepts a documented JSON shape from the customer's backend. Every tenant gets an auto-provisioned default custom source on first request (via `get_or_create_default_custom_source` in `services/conversions/repo.rs`).

- **Storage** — per-event rows in the `conversion_events` MongoDB time series collection (same pattern as `click_events`). Counts/sums computed on read via aggregation pipelines in `ConversionsRepo::get_conversion_counts_for_link`. No separate counter cache.
- **Dedup** — a standard `conversion_dedup` collection with a unique index on `(tenant_id, idempotency_key)` and 30-day TTL. Mutex-collection pattern because time series collections can't enforce unique indexes reliably at insert time.
- **Attribution lookup** — events carry `user_id`; `ConversionsService::ingest_parsed` resolves `user_id → Attribution → link_id` via `LinksRepository::find_attribution_by_user` before inserting the event. Events with no matching attribution are logged and dropped.
- **Hard line** — the API answers link-scoped questions only. User-scoped queries (cohorts, funnels, retention) are permanently out of scope. Metadata is stored verbatim but not indexed or queried in v1.
- **Extensibility** — new integrations (RevenueCat, Stripe, etc.) are drop-in parser additions: implement `ConversionParser`, add a `SourceType` variant, add one line to `parser_for`. No schema migration required — `Source.signing_secret` and `Source.config` already exist for integration parsers to use.
- **Outbound webhook** — on successful ingestion, the service fires a `Conversion` webhook event with a stable `event_id` (the MongoDB ObjectId of the stored event) for customer-side dedup on retry. The webhook dispatcher's `find_active_for_event` query is wrapped in a 60-second `cached` layer to kill the per-event DB query hot path.

## Adding a New Domain

1. Create `services/<name>/mod.rs`, `models.rs` for types, `repo.rs` for database access
2. Create `api/<name>/mod.rs` with `pub fn router() -> Router<Arc<AppState>>`
3. Create `api/<name>/routes.rs` for HTTP handlers — import models/repos from `crate::services::<name>`
4. Merge the router in `api/mod.rs` and register paths in the OpenAPI derive
5. Add `#[tracing::instrument]` to all route handlers (skip large args like state, body)
6. Add `#[schema(example = "...")]` to all `ToSchema` struct fields for good OpenAPI documentation

## Style Guidelines

- Prefer iterator chains over imperative loops
- Use `filter_map` to combine filtering and transformation
- Flatten with `?` operator, `.ok()`, `.and_then()` chains
- Use `let-else` for early returns
- All route handlers must have `#[tracing::instrument]` for Sentry visibility
- `ErrorResponse` lives in `error.rs` and is shared across all slices

## Caching Pattern

When using `#[cached(result = true)]` for database lookups that return `Option<T>`:
- **Never cache `None` results** — they cause stale misses after creation
- Return `Err("not_found")` instead of `Ok(None)` inside the cached function
- The `#[cached(result = true)]` macro only caches `Ok` values, so `Err` is always re-executed
- The caller converts `Err("not_found")` back to `Ok(None)`

```rust
#[cached(result = true)]
async fn cached_find(id: &str) -> Result<Item, String> {
    db.find(id).await?.ok_or_else(|| "not_found".to_string())
}

// Caller:
match cached_find(id).await {
    Ok(item) => Ok(Some(item)),
    Err(e) if e == "not_found" => Ok(None),
    Err(e) => Err(e),
}
```

## Migrations

Migrations live in `src/migrations/` and implement the `Migration` trait. Run via CLI:

```sh
cargo run -- migrate --list                        # Show available migrations
cargo run -- migrate --name m001_auth_split        # Dry run (default, no writes)
cargo run -- migrate --name m001_auth_split --apply  # Actually execute
```

- **Dry run is the default** — migrations must always accept a `dry_run: bool` parameter and perform **zero writes** when `dry_run` is true
- In dry run mode, log what *would* happen (e.g. "Would migrate: alice@example.com")
- Migrations should be idempotent — skip documents that are already migrated
- Each migration is a separate file: `m001_description.rs`, `m002_description.rs`, etc.

## Setup

After cloning, enable the shared git hooks:

```sh
git config core.hooksPath .githooks
```

This enables:
- **Web SDK auto-build** — when you commit changes to `sdk/web/src/`, the pre-commit hook rebuilds the IIFE and stages `server/src/sdk/rift.js` automatically
- **Mobile SDK UniFFI check** — prevents UniFFI annotations from leaking into the core crate

## CI Checks

Before pushing, always run all three checks that CI enforces:

```sh
cargo fmt -- --check   # Formatting
cargo clippy -- -D warnings   # Lints (warnings = errors)
cargo test   # All tests pass
```

- **Never suppress warnings** with `#[allow(...)]` — fix the root cause instead
- If clippy complains about too many arguments, use a struct or builder pattern
- If clippy complains about redundant closures, pass the function directly
- If an import is unused, remove it — don't `#[allow(unused_imports)]`
- Run `cargo fmt` before committing to avoid formatting failures in CI

## Mobile SDK (`client/mobile/`)

Rust library compiled to Swift/Kotlin via UniFFI. Three-crate workspace:

- `core/` — Pure Rust. HTTP client, models, parsers. **No UniFFI dependency.**
- `ffi/` — UniFFI boundary. Wraps core types with `#[uniffi::Object]`, `#[uniffi::Record]`, etc.
- `mobile/` — Thin re-export crate. Build target for `staticlib`/`cdylib`.

### Conventions
- **Core must not import UniFFI** — enforced by architecture test + pre-commit hook
- `metadata` fields are `Option<String>` (JSON string) at the FFI boundary
- SDK owns its own models — no shared types with the server
- All errors go through `RiftError` enum

### Building
```sh
cd client/mobile
cargo test                    # Run all tests including architecture tests
./build_xcframework.sh        # Build iOS XCFramework
./build_android.sh            # Build Android libraries
```

### CI/CD
- `sdk-ci.yml` — runs on every push/PR touching `client/mobile/`
- `sdk-release.yml` — triggered by `sdk-v*` tags or manual dispatch

## Web SDK (`sdk/web/`)

TypeScript package built with tsup. Single source produces three outputs:

- `dist/index.mjs` — ESM (`import { Rift } from '@riftl/sdk'`)
- `dist/index.cjs` — CJS (`const { Rift } = require('@riftl/sdk')`)
- `dist/index.global.js` — IIFE (copied to `server/src/sdk/rift.js`, served at `/sdk/rift.js`)

### Conventions
- **The TypeScript source is the single source of truth** — `server/src/sdk/rift.js` is a build artifact
- **The pre-commit hook keeps them in sync** — no manual build step needed (requires `git config core.hooksPath .githooks`)
- **CI verifies sync** — `web-sdk-ci.yml` fails if the IIFE is out of date

### Building manually (if needed)
```sh
cd sdk/web
npm ci
npm run build
cp dist/index.global.js ../../server/src/sdk/rift.js
```

## MCP with Claude Code

The server exposes an MCP endpoint at `/mcp` (streamable HTTP transport). To enable it in Claude Code, add a `.mcp.json` in the project root:

```json
{
  "mcpServers": {
    "rift": {
      "type": "http",
      "url": "http://localhost:3000/mcp",
      "headers": {
        "x-api-key": "rl_live_YOUR_API_KEY_HERE"
      }
    }
  }
}
```

This gives Claude access to `create_link`, `get_link`, `list_links`, `update_link`, and `delete_link` tools. The API key authenticates each MCP session — use the same `rl_live_` key you'd use with the REST API.

## Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `HOST` / `PORT` | No | Server bind (default `0.0.0.0:3000`) |
| `MONGO_URI` / `MONGO_DB` | No | MongoDB (server boots without it, auth disabled) |
| `SENTRY_DSN` | No | Sentry error tracking (empty = disabled) |
| `RESEND_API_KEY` | No | Email verification via Resend |
| `RESEND_FROM_EMAIL` | No | Sender address for verification emails (default `Rift <noreply@updates.riftl.ink>`) |
| `PUBLIC_URL` | No | Base URL for email verification links and landing pages |
| `FREE_DAILY_LIMIT` | No | Anonymous requests per IP per day (default 5) |
| `X402_ENABLED` | No | Enable x402 payments (`true`/`false`) |
| `X402_RECIPIENT` | No | USDC recipient wallet address |
| `X402_PRICE` | No | Price per request in USDC (default `0.01`) |
| `X402_DESCRIPTION` | No | Resource description shown to payers |
| `CDP_API_KEY_ID` / `CDP_API_KEY_SECRET` | No | Coinbase Developer Platform keys for x402 |
| `PRIMARY_DOMAIN` | No | Primary domain for link resolution (default `riftl.ink`) |
