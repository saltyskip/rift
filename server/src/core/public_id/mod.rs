//! Typed prefixed identifiers — the only ID type allowed outside the storage layer.
//!
//! The wire format is `<prefix>_<24-char-lowercase-hex>` where the body is the raw
//! MongoDB `ObjectId` hex. `Id<P>` stores just the hex; the prefix is added on
//! serialize and validated on deserialize. Marker `P` makes per-resource aliases
//! distinct types — passing an `AffiliateId` where a `WebhookId` is expected fails
//! to build.
//!
//! Background: issue #156. `ObjectId` is the MongoDB storage type and must not
//! appear anywhere except repos and migrations. Architecture test
//! `object_id_confined_to_storage_layer` enforces this; new files inherit the rule.

use std::fmt;
use std::marker::PhantomData;
use std::str::FromStr;

use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod models;
pub use models::{
    AffiliateId, AppId, ConversionEventId, DomainId, ParseIdError, PublishableKeyId, SecretKeyId,
    SourceId, TenantId, UserId, WebhookId,
};

/// Implemented by zero-sized marker types to declare a resource's prefix and schema name.
pub trait IdPrefix {
    /// Wire-format prefix, e.g. `"aff"` → `"aff_665a…"`.
    const PREFIX: &'static str;
    /// Name surfaced in OpenAPI / JSON Schema documents, e.g. `"AffiliateId"`.
    const SCHEMA_NAME: &'static str;
}

/// 24-char lowercase ObjectId hex length. ObjectIds are always 12 bytes → 24 hex chars.
pub const HEX_LEN: usize = 24;

crate::impl_container!(Id);
/// Typed prefixed identifier wrapping a raw ObjectId hex string.
///
/// Construction goes through `from_object_id` (from a repo) or `parse` (from the wire);
/// both validate the body is exactly 24 lowercase hex chars. Serializes as
/// `<prefix>_<hex>`; deserializes by checking the prefix matches `P::PREFIX`.
pub struct Id<P: IdPrefix> {
    /// The raw 24-char ObjectId hex. **No prefix.**
    hex: String,
    _marker: PhantomData<fn() -> P>,
}

impl<P: IdPrefix> Id<P> {
    /// Construct from a MongoDB ObjectId. Repo layer only.
    pub fn from_object_id(oid: ObjectId) -> Self {
        Self {
            hex: oid.to_hex(),
            _marker: PhantomData,
        }
    }

    /// Convert back to an `ObjectId` for storage queries. Repo layer only.
    /// Infallible in practice because construction validates the hex body —
    /// returns `Err` only if the underlying hex was somehow corrupted.
    pub fn to_object_id(&self) -> Result<ObjectId, ParseIdError> {
        ObjectId::parse_str(&self.hex).map_err(|_| ParseIdError::InvalidHex)
    }

    /// Parse `<prefix>_<24-char-hex>`. The body must be lowercase hex (matching
    /// `ObjectId::to_hex()`).
    pub fn parse(s: &str) -> Result<Self, ParseIdError> {
        let (prefix, body) = s.split_once('_').ok_or(ParseIdError::MissingSeparator)?;
        if prefix != P::PREFIX {
            return Err(ParseIdError::WrongPrefix {
                expected: P::PREFIX,
                got: prefix.to_string(),
            });
        }
        if body.len() != HEX_LEN {
            return Err(ParseIdError::InvalidLength {
                expected: HEX_LEN,
                got: body.len(),
            });
        }
        if !body.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(ParseIdError::InvalidHex);
        }
        Ok(Self {
            hex: body.to_string(),
            _marker: PhantomData,
        })
    }

    /// Borrow the raw hex body (no prefix). Repo layer use.
    pub fn as_hex(&self) -> &str {
        &self.hex
    }
}

impl<P: IdPrefix> From<ObjectId> for Id<P> {
    fn from(oid: ObjectId) -> Self {
        Self::from_object_id(oid)
    }
}

impl<P: IdPrefix> Clone for Id<P> {
    fn clone(&self) -> Self {
        Self {
            hex: self.hex.clone(),
            _marker: PhantomData,
        }
    }
}

impl<P: IdPrefix> PartialEq for Id<P> {
    fn eq(&self, other: &Self) -> bool {
        self.hex == other.hex
    }
}
impl<P: IdPrefix> Eq for Id<P> {}

impl<P: IdPrefix> std::hash::Hash for Id<P> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hex.hash(state);
    }
}

impl<P: IdPrefix> PartialOrd for Id<P> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<P: IdPrefix> Ord for Id<P> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hex.cmp(&other.hex)
    }
}

impl<P: IdPrefix> fmt::Display for Id<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", P::PREFIX, self.hex)
    }
}

impl<P: IdPrefix> fmt::Debug for Id<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}_{}\"", P::PREFIX, self.hex)
    }
}

impl<P: IdPrefix> FromStr for Id<P> {
    type Err = ParseIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl<P: IdPrefix> Serialize for Id<P> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("{}_{}", P::PREFIX, self.hex))
    }
}

impl<'de, P: IdPrefix> Deserialize<'de> for Id<P> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl<P: IdPrefix> utoipa::PartialSchema for Id<P> {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        use utoipa::openapi::schema::{Object, SchemaType, Type};
        Object::builder()
            .schema_type(SchemaType::Type(Type::String))
            .pattern(Some(format!("^{}_[0-9a-f]{{{}}}$", P::PREFIX, HEX_LEN)))
            .examples([serde_json::Value::String(format!(
                "{}_{}",
                P::PREFIX,
                "0".repeat(HEX_LEN)
            ))])
            .description(Some(format!(
                "Prefixed public identifier (prefix `{}_`, 24-char lowercase ObjectId hex body).",
                P::PREFIX,
            )))
            .into()
    }
}

impl<P: IdPrefix> utoipa::ToSchema for Id<P> {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed(P::SCHEMA_NAME)
    }
}

#[cfg(feature = "mcp")]
impl<P: IdPrefix> schemars::JsonSchema for Id<P> {
    fn inline_schema() -> bool {
        true
    }
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed(P::SCHEMA_NAME)
    }
    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": format!("^{}_[0-9a-f]{{{}}}$", P::PREFIX, HEX_LEN),
            "description": format!(
                "Prefixed public identifier (prefix `{}_`, 24-char lowercase ObjectId hex body).",
                P::PREFIX,
            ),
        })
    }
}

// ── Per-resource marker types ──
//
// One marker per public-facing resource. Each marker is a zero-sized type whose
// sole purpose is hosting `impl IdPrefix` — hence the `impl_container!` exemption.

crate::impl_container!(AffiliateIdMarker);
pub struct AffiliateIdMarker;
impl IdPrefix for AffiliateIdMarker {
    const PREFIX: &'static str = "aff";
    const SCHEMA_NAME: &'static str = "AffiliateId";
}

crate::impl_container!(AppIdMarker);
pub struct AppIdMarker;
impl IdPrefix for AppIdMarker {
    const PREFIX: &'static str = "app";
    const SCHEMA_NAME: &'static str = "AppId";
}

crate::impl_container!(ConversionEventIdMarker);
pub struct ConversionEventIdMarker;
impl IdPrefix for ConversionEventIdMarker {
    const PREFIX: &'static str = "cev";
    const SCHEMA_NAME: &'static str = "ConversionEventId";
}

crate::impl_container!(DomainIdMarker);
pub struct DomainIdMarker;
impl IdPrefix for DomainIdMarker {
    const PREFIX: &'static str = "dom";
    const SCHEMA_NAME: &'static str = "DomainId";
}

crate::impl_container!(PublishableKeyIdMarker);
pub struct PublishableKeyIdMarker;
impl IdPrefix for PublishableKeyIdMarker {
    const PREFIX: &'static str = "pkid";
    const SCHEMA_NAME: &'static str = "PublishableKeyId";
}

crate::impl_container!(SecretKeyIdMarker);
pub struct SecretKeyIdMarker;
impl IdPrefix for SecretKeyIdMarker {
    const PREFIX: &'static str = "sk";
    const SCHEMA_NAME: &'static str = "SecretKeyId";
}

crate::impl_container!(SourceIdMarker);
pub struct SourceIdMarker;
impl IdPrefix for SourceIdMarker {
    const PREFIX: &'static str = "src";
    const SCHEMA_NAME: &'static str = "SourceId";
}

crate::impl_container!(TenantIdMarker);
pub struct TenantIdMarker;
impl IdPrefix for TenantIdMarker {
    const PREFIX: &'static str = "tnt";
    const SCHEMA_NAME: &'static str = "TenantId";
}

crate::impl_container!(UserIdMarker);
pub struct UserIdMarker;
impl IdPrefix for UserIdMarker {
    const PREFIX: &'static str = "usr";
    const SCHEMA_NAME: &'static str = "UserId";
}

crate::impl_container!(WebhookIdMarker);
pub struct WebhookIdMarker;
impl IdPrefix for WebhookIdMarker {
    const PREFIX: &'static str = "wh";
    const SCHEMA_NAME: &'static str = "WebhookId";
}

#[cfg(test)]
#[path = "public_id_tests.rs"]
mod tests;
