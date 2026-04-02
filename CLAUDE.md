# Rift

Deep links for humans and agents.

**Stack:** Rust, Axum, MongoDB, Sentry, x402

## Architecture

The server separates **domain logic** from **transport layers**:

- **`services/`** — Transport-agnostic business logic. Each domain (links, auth, domains, apps, webhooks) has its own directory with `models.rs`, `repo.rs`, and optionally `service.rs`
- **`api/`** — HTTP transport. Each slice has `mod.rs` (router) and `routes.rs` (handlers). Imports models/repos from `services/`
- **`mcp/`** — MCP protocol transport. Imports from `services/`, not from `api/`
- **`app.rs`** — `AppState` struct, shared across transports
- **`core/`** — Shared infra only (db connection, config, rate limiting) — no business logic
- **Transport layers must not import from each other** — both `api/` and `mcp/` import from `services/`
- **Domains own their models and repositories** in `services/<domain>/`
- **Auth sub-slices** — `services/auth/` contains `secret_keys/` (signup/verify, `rl_live_` keys) and `publishable_keys/` (SDK keys, `pk_live_` prefix). Transport routes live in `api/auth/`

### Cargo Features

- `api` — HTTP API routes (enabled by default)
- `mcp` — MCP protocol server, pulls in `rmcp` and `schemars` (enabled by default)
- Both can be independently disabled: `cargo build --no-default-features --features mcp`
- **CI runs with default features (both enabled).** Individual feature subsets may produce dead-code warnings since `services/` is shared infrastructure

## Multi-Tenancy

All link data is scoped by `tenant_id` (the API key's ObjectId). The auth middleware injects
a `TenantId` extension into the request on successful API key validation. Route handlers
extract it via `Extension<TenantId>`.

Public endpoints (landing page, attribution reporting) resolve the tenant from the link_id itself.

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

## Mobile SDK (`sdk/mobile/`)

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
cd sdk/mobile
cargo test                    # Run all tests including architecture tests
./build_xcframework.sh        # Build iOS XCFramework
./build_android.sh            # Build Android libraries
```

### CI/CD
- `sdk-ci.yml` — runs on every push/PR touching `sdk/mobile/`
- `sdk-release.yml` — triggered by `sdk-v*` tags or manual dispatch

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
