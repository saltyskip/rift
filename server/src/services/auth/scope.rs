//! Scope enforcement helpers.
//!
//! Service methods call these at the top to gate access by `KeyScope`.
//! Two layers of enforcement work together:
//!
//! 1. **Auth middleware** does a coarse path allowlist check for
//!    `KeyScope::Affiliate` keys (only link minting + reads). Hits before
//!    handlers run.
//! 2. **Service-layer checks** (these helpers) are transport-agnostic — they
//!    survive when MCP grows partner support and don't depend on the HTTP
//!    request path. CLAUDE.md's "service is the choke point" rule.
//!
//! `None` is grandfathered to `Full` for the migration window only. A
//! follow-up PR will flip this to deny.

use crate::services::auth::secret_keys::models::ScopeError;
use crate::services::auth::secret_keys::repo::KeyScope;

/// Require the caller to have full tenant access.
///
/// Affiliate-scoped keys are rejected. `None` is grandfathered to `Full`
/// during the migration window — see module docs.
pub fn require_full(scope: Option<&KeyScope>) -> Result<(), ScopeError> {
    match scope {
        Some(KeyScope::Full) | None => Ok(()),
        Some(KeyScope::Affiliate { .. }) => Err(ScopeError::Forbidden),
    }
}

// `require_affiliate` is intentionally not yet defined — there are no v1
// service methods that should require affiliate scope (affiliate-scoped keys
// only call `LinksService::create_link` which branches inline). When the
// dispatcher or partner-side read APIs need it, add it back here.

#[cfg(test)]
#[path = "scope_tests.rs"]
mod tests;
