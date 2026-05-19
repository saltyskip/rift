//! Human session lifecycle.
//!
//! Sits alongside `secret_keys` (machine credentials) and `users` (identity).
//! A session represents "a human is signed into a browser as a particular user
//! in a particular tenant." Sessions are minted by the magic-link signin flow
//! and consumed by `session_auth_gate` middleware on session-protected routes.
//!
//! - Token: opaque random 32-byte hex, hashed in storage.
//! - Lifetime: 30 days, revocable, indexed by `token_hash` and `(user_id, expires_at)`.
//! - Email magic links reuse `TokenService` with `TokenPurpose::Signin`. The
//!   short-lived signin token (15 min) is exchanged for a durable session
//!   on callback.

pub mod models;
pub mod repo;
pub mod service;

pub use models::{SessionError, SessionsConfig};
pub use repo::SessionsRepository;
