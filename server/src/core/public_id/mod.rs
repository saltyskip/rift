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
use mongodb::bson::Bson;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod models;
pub use models::{
    AffiliateId, AppId, AppUserId, ConversionEventId, DomainId, InstallEventId, LinkInternalId,
    ParseIdError, PublishableKeyId, SecretKeyId, SourceId, TenantId, UserId, WebhookId,
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
/// Typed prefixed identifier wrapping a MongoDB `ObjectId`. The wire format
/// adds the resource prefix (`<prefix>_<24-char-lowercase-hex>`) at serialize
/// time; BSON serialization emits the native `ObjectId` so a single struct
/// works as both a MongoDB document and an HTTP response.
///
/// Construction is via `Id::from_object_id` (repo / middleware boundary) or
/// `Id::parse` (wire-format strings). There is intentionally **no** blanket
/// `From<ObjectId>` — every `ObjectId` → `Id<P>` conversion must name the
/// target type so cross-resource ID mixups are caught at review time.
pub struct Id<P: IdPrefix> {
    inner: ObjectId,
    _marker: PhantomData<fn() -> P>,
}

impl<P: IdPrefix> Id<P> {
    /// Generate a fresh `Id` backed by a new `ObjectId`. Use this when creating
    /// a new resource that needs an ID assigned in application code (i.e. not
    /// letting MongoDB generate `_id` on insert).
    ///
    /// Intentionally NOT `Default::default()` — defaults should be cheap and
    /// deterministic; this mints a fresh ObjectId (clock + counter).
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self::from_object_id(ObjectId::new())
    }

    /// Construct from a MongoDB ObjectId. Repo / middleware layer only.
    pub fn from_object_id(oid: ObjectId) -> Self {
        Self {
            inner: oid,
            _marker: PhantomData,
        }
    }

    /// Borrow the underlying `ObjectId`. Infallible — there's no parsing.
    pub fn as_object_id(&self) -> &ObjectId {
        &self.inner
    }

    /// Convert to an owned `ObjectId` for storage queries. Infallible — the
    /// `Id<P>` stores a parsed `ObjectId` directly.
    #[allow(clippy::wrong_self_convention)] // `Id<P>` is Copy; `&self` matches the `&id.to_object_id()` call sites.
    pub fn to_object_id(&self) -> ObjectId {
        self.inner
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
        let oid = ObjectId::parse_str(body).map_err(|_| ParseIdError::InvalidHex)?;
        Ok(Self {
            inner: oid,
            _marker: PhantomData,
        })
    }

    /// The raw 24-char lowercase hex of the underlying ObjectId (no prefix).
    pub fn as_hex(&self) -> String {
        self.inner.to_hex()
    }
}

// Direct conversion to `Bson` so `doc! { "_id": id }` produces a native ObjectId
// without round-tripping through serde. The `From<&T>` reference variant comes
// from bson's blanket `impl<T: Clone + Into<Bson>> From<&T> for Bson`.
impl<P: IdPrefix> From<Id<P>> for Bson {
    fn from(id: Id<P>) -> Self {
        Bson::ObjectId(id.inner)
    }
}

impl<P: IdPrefix> Copy for Id<P> {}
impl<P: IdPrefix> Clone for Id<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: IdPrefix> PartialEq for Id<P> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl<P: IdPrefix> Eq for Id<P> {}

impl<P: IdPrefix> std::hash::Hash for Id<P> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl<P: IdPrefix> PartialOrd for Id<P> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<P: IdPrefix> Ord for Id<P> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<P: IdPrefix> fmt::Display for Id<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}_{}", P::PREFIX, self.inner.to_hex())
    }
}

impl<P: IdPrefix> fmt::Debug for Id<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\"{}_{}\"", P::PREFIX, self.inner.to_hex())
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
        if serializer.is_human_readable() {
            // JSON / OpenAPI / MCP wire format: prefixed string.
            serializer.collect_str(&format_args!("{}_{}", P::PREFIX, self.inner.to_hex()))
        } else {
            // BSON (raw, non-human-readable — what the mongodb driver uses):
            // serialize as a native ObjectId so MongoDB stores `_id` in its
            // canonical form. This is the bridge that lets a single struct
            // serve both as a BSON document and an HTTP response.
            self.inner.serialize(serializer)
        }
    }
}

impl<'de, P: IdPrefix> Deserialize<'de> for Id<P> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        if deserializer.is_human_readable() {
            // JSON / path params / MCP inputs: prefixed string.
            let s = String::deserialize(deserializer)?;
            Self::parse(&s).map_err(serde::de::Error::custom)
        } else {
            // BSON: native ObjectId.
            let oid = ObjectId::deserialize(deserializer)?;
            Ok(Self::from_object_id(oid))
        }
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

crate::impl_container!(AppUserIdMarker);
pub struct AppUserIdMarker;
impl IdPrefix for AppUserIdMarker {
    const PREFIX: &'static str = "appusr";
    const SCHEMA_NAME: &'static str = "AppUserId";
}

crate::impl_container!(InstallEventIdMarker);
pub struct InstallEventIdMarker;
impl IdPrefix for InstallEventIdMarker {
    const PREFIX: &'static str = "iev";
    const SCHEMA_NAME: &'static str = "InstallEventId";
}

crate::impl_container!(LinkInternalIdMarker);
pub struct LinkInternalIdMarker;
impl IdPrefix for LinkInternalIdMarker {
    // Distinct from the public `link_id` vanity slug (which stays as a String).
    // This is the internal `_id: ObjectId` of stored Link documents.
    const PREFIX: &'static str = "lnk";
    const SCHEMA_NAME: &'static str = "LinkInternalId";
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
    // `skid_` rather than `sk_` to avoid muscle-memory collision with Stripe's
    // `sk_live_…` / `sk_test_…` secret-key value format.
    const PREFIX: &'static str = "skid";
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
