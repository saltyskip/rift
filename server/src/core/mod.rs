pub mod cdp;
pub mod config;
pub mod db;
pub mod email;
pub mod http;
pub mod models;
pub mod origin;
// Phase 1 foundation for issue #156. No consumers yet — follow-up commits wire
// each resource. The bin target compiles this module without any reaching
// reference from `main.rs`, hence the blanket dead_code allow.
#[allow(dead_code, unused_imports)]
pub mod public_id;
pub mod rate_limit;
pub mod threat_feed;
pub mod validation;
pub mod webhook_dispatcher;
