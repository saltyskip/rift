# Relay

Deep links for humans and agents.

**Stack:** Rust, Axum, MongoDB, Sentry, x402

## Backend API Conventions

The API layer uses a **vertical slice architecture**:

- Each domain (auth, links, etc.) has its own directory under `api/`
- Each slice has: `mod.rs` (router), `routes.rs` (handlers), `models.rs` (domain types)
- Each `mod.rs` exports `pub fn router() -> Router<Arc<AppState>>` merged in `api/mod.rs`
- **Slices own their models** — no shared models module. Types live in the slice that owns them
- **Slices own their repositories** — db queries live in the slice's `repo.rs`, not in a shared db module
- `core/` is for shared infra only (db connection, config) — no business logic
- `AppState` and OpenAPI spec live in `api/mod.rs`
- **Slices should not import from other slices** — cross-slice data goes through AppState

## Multi-Tenancy

All link data is scoped by `tenant_id` (the API key's ObjectId). The auth middleware injects
a `TenantId` extension into the request on successful API key validation. Route handlers
extract it via `Extension<TenantId>`.

Public endpoints (landing page, attribution reporting) resolve the tenant from the link_id itself.

## Adding a New Slice

1. Create `api/<name>/mod.rs` with `pub fn router() -> Router<Arc<AppState>>`
2. Create `routes.rs` for handlers, `models.rs` for types
3. If the slice needs db: create `repo.rs` with a repo struct initialized from `Database`
4. Merge the router in `api/mod.rs` and register paths in the OpenAPI derive
5. Add `#[tracing::instrument]` to all route handlers (skip large args like state, body)

## Style Guidelines

- Prefer iterator chains over imperative loops
- Use `filter_map` to combine filtering and transformation
- Flatten with `?` operator, `.ok()`, `.and_then()` chains
- Use `let-else` for early returns
- All route handlers must have `#[tracing::instrument]` for Sentry visibility
- `ErrorResponse` lives in `error.rs` and is shared across all slices

## Environment Variables

| Variable | Required | Purpose |
|----------|----------|---------|
| `HOST` / `PORT` | No | Server bind (default `0.0.0.0:3000`) |
| `MONGO_URI` / `MONGO_DB` | No | MongoDB (server boots without it, auth disabled) |
| `SENTRY_DSN` | No | Sentry error tracking (empty = disabled) |
| `RESEND_API_KEY` | No | Email verification via Resend |
| `PUBLIC_URL` | No | Base URL for email verification links and landing pages |
| `FREE_DAILY_LIMIT` | No | Anonymous requests per IP per day (default 5) |
| `X402_ENABLED` | No | Enable x402 payments (`true`/`false`) |
| `X402_RECIPIENT` | No | USDC recipient wallet address |
| `X402_PRICE` | No | Price per request in USDC (default `0.01`) |
| `X402_DESCRIPTION` | No | Resource description shown to payers |
| `CDP_API_KEY_ID` / `CDP_API_KEY_SECRET` | No | Coinbase Developer Platform keys for x402 |
| `PRIMARY_DOMAIN` | No | Primary domain for link resolution (default `riftl.ink`) |
