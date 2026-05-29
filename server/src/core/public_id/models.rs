//! Data types for `core::public_id` — the parse error enum and per-resource
//! type aliases. The `Id<P>` struct itself lives in `mod.rs` because it
//! hosts every trait impl in this module.

use super::{
    AffiliateIdMarker, AppIdMarker, AppUserIdMarker, ConversionEventIdMarker, DomainIdMarker, Id,
    InstallEventIdMarker, LinkInternalIdMarker, PublishableKeyIdMarker, SecretKeyIdMarker,
    SourceIdMarker, TenantIdMarker, UserIdMarker, WebhookIdMarker,
};

/// Errors returned by [`Id::parse`] and [`Id::to_object_id`].
#[derive(Debug, thiserror::Error)]
pub enum ParseIdError {
    #[error("missing `_` separator between prefix and body")]
    MissingSeparator,
    #[error("wrong prefix: expected `{expected}`, got `{got}`")]
    WrongPrefix { expected: &'static str, got: String },
    #[error("invalid body length: expected {expected} chars, got {got}")]
    InvalidLength { expected: usize, got: usize },
    #[error("body is not valid 24-char lowercase hex")]
    InvalidHex,
}

pub type AffiliateId = Id<AffiliateIdMarker>;
pub type AppId = Id<AppIdMarker>;
pub type AppUserId = Id<AppUserIdMarker>;
pub type ConversionEventId = Id<ConversionEventIdMarker>;
pub type DomainId = Id<DomainIdMarker>;
pub type InstallEventId = Id<InstallEventIdMarker>;
pub type LinkInternalId = Id<LinkInternalIdMarker>;
pub type PublishableKeyId = Id<PublishableKeyIdMarker>;
pub type SecretKeyId = Id<SecretKeyIdMarker>;
pub type SourceId = Id<SourceIdMarker>;
pub type TenantId = Id<TenantIdMarker>;
pub type UserId = Id<UserIdMarker>;
pub type WebhookId = Id<WebhookIdMarker>;
