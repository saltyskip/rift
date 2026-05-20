//! Service-layer authorization: permissions, scopes, and the unified
//! `AuthContext` that every authenticated service method takes as its first
//! non-`self` argument.
//!
//! - `models.rs` — `pub` data (`Permission`, `Scopes`, `Principal`,
//!   `ResourceScope`, `AuthContext`, `AuthzError`).
//! - `context.rs` — `impl AuthContext` (constructors from middleware,
//!   `require` / `require_any` checks) and `impl Scopes`.
//!
//! Companion proc-macros live in the `rift-macros` crate
//! (`#[requires(...)]`, `#[requires_any(...)]`, `#[requires_public]`).

pub mod context;
pub mod models;

// `Principal`/`ResourceScope`/`Scopes` are re-exported for future consumers
// (services that introspect the principal kind or pin to an affiliate scope)
// even though PR1 doesn't reference them yet. Allow the lint until the
// migration backlog drains and consumers appear.
#[allow(unused_imports)]
pub use models::{AuthContext, AuthzError, Permission, Principal, ResourceScope, Scopes};
