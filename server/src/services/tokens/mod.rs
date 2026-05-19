//! Unified token service for every "issue → email → redeem" flow.
//!
//! Consolidates what used to live in two collections:
//! - `users.verify_token` + `verify_token_expires_at` (email verification)
//! - `secret_key_create_requests` collection (API key rotation codes)
//!
//! Domain services (UsersService, SecretKeysService, SessionsService,
//! OauthService) call `TokenService::issue` / `consume_hash` /
//! `consume_tuple` and own their own email templates + post-consume actions.
//! Nothing about tier flags, Stripe sessions, or key minting belongs here.

pub mod models;
pub mod repo;
pub mod service;

pub use models::{ConsumeOutcome, TokenKind, TokenPurpose, TokenSpec};
pub use repo::TokensRepository;
pub use service::TokenService;
