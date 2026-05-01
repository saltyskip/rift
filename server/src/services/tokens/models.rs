//! Shape primitives for the tokens service — no I/O.

use mongodb::bson;
use serde::{Deserialize, Serialize};

/// Which flow a token gates. Stored on the doc and returned by `consume_hash`
/// so the caller can switch on it without looking up by purpose up front —
/// this is what lets `/v1/billing/go` take a single opaque token and
/// dispatch to Checkout vs Portal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenPurpose {
    EmailVerify,
    KeyRotation,
    BillingSubscribe,
    BillingPortal,
}

/// How the token is looked up on consume.
#[derive(Debug, Clone, Copy)]
pub enum TokenKind {
    /// Long opaque token (64-char hex). Looked up by `_id = sha256(raw)`.
    /// Single-use — attempts counter unused. Used by email-verify + billing.
    HashKeyed,
    /// Short code (6-char alphanumeric). Looked up by `(purpose, email)`.
    /// Every consume attempt increments `attempts`; when > `max_attempts`
    /// the doc is deleted and `AttemptsExhausted` is returned. Used by
    /// API key rotation — the small code space (36^6) requires this cap.
    TupleKeyed { max_attempts: i32 },
}

/// What the caller asks for when issuing a token.
pub struct TokenSpec {
    pub purpose: TokenPurpose,
    pub kind: TokenKind,
    pub ttl_secs: i64,
    /// Email that will receive the link/code. Used both for auditing and
    /// as the lookup key for tuple-keyed tokens.
    pub email: String,
    /// Per-purpose payload carried through to consume. Examples: `{user_id}`
    /// for email-verify, `{tenant_id, user_id}` for key-rotation, `{tier}`
    /// for billing subscribe.
    pub metadata: bson::Document,
}

/// Outcome of a consume attempt.
#[derive(Debug)]
pub enum ConsumeOutcome {
    Ok {
        purpose: TokenPurpose,
        email: String,
        metadata: bson::Document,
    },
    /// Token doesn't exist, has expired, was already consumed, or (for
    /// tuple-keyed) the code didn't match.
    NotFound,
    /// Tuple-keyed only: caller exceeded `max_attempts`. The doc was deleted.
    AttemptsExhausted,
}

// ── DB Document ──

/// One row in the `tokens` collection.
///
/// - HashKeyed tokens use `_id = sha256(raw)` so lookup is O(1) via the
///   default `_id` index.
/// - TupleKeyed tokens use a random ObjectId hex as `_id`; lookup is by
///   the `(purpose, email)` index and then verified against `token_hash`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDoc {
    #[serde(rename = "_id")]
    pub id: String,
    pub purpose: TokenPurpose,
    pub email: String,
    /// sha256(raw) hex. Stored even on hash-keyed tokens (where it equals
    /// `_id`) so tuple-keyed consume logic works uniformly.
    pub token_hash: String,
    pub metadata: bson::Document,
    pub attempts: i32,
    pub max_attempts: i32,
    pub expires_at: bson::DateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumed_at: Option<bson::DateTime>,
    pub created_at: bson::DateTime,
}
