//! Affiliates domain — partner records for the affiliate-attribution flow.
//!
//! An advertiser tenant creates `Affiliate` rows for each partner that drives
//! traffic to its app. Each affiliate gets a `partner_key` slug and (via the
//! `/credentials` endpoint) one or more scoped `rl_live_…` keys. Those keys
//! mint links pinned to the affiliate via `Link.affiliate_id`.
//!
//! Mirrors `services/webhooks/` structurally — service layer enforces quota
//! and scope; route handlers stay thin.
//!
//! TODO(dispatcher): Postback delivery is not built in v1. When it lands,
//! `services/affiliates/dispatcher.rs` will load an `Affiliate`, sign with
//! its (future) `signing_secret`, and POST to its (future) `postback_url`.
//! See `orangerock-bcom-rewards-integration.md` Part 1.

pub mod models;
pub mod repo;
pub mod service;
