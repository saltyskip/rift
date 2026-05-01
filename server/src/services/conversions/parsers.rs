use mongodb::bson::{to_bson, Document};
use serde::Deserialize;

use super::models::{ParseError, ParsedConversion, Source, SourceType};

// ── Parser trait + dispatch ──

pub trait ConversionParser: Send + Sync {
    fn parse(&self, body: &[u8], source: &Source) -> Result<Vec<ParsedConversion>, ParseError>;
}

/// Parser dispatch table. To add a new source type (e.g. RevenueCat, Stripe):
/// 1. Add a variant to `SourceType`
/// 2. Implement `ConversionParser` for the new parser struct
/// 3. Add one line here
///
/// No schema migration required — `Source.signing_secret` and `Source.config`
/// already exist on the struct for integration parsers to use.
pub fn parser_for(source_type: SourceType) -> Box<dyn ConversionParser> {
    match source_type {
        SourceType::Custom => Box::new(CustomParser),
    }
}

// ── CustomParser ──

const MAX_METADATA_BYTES: usize = 1024;
const MAX_IDEMPOTENCY_KEY_LEN: usize = 256;

/// The documented JSON shape for the custom source. This is the contract customers
/// POST against when they wire Rift into their own backend.
#[derive(Debug, Deserialize)]
struct CustomPayload {
    user_id: String,
    /// The conversion type, free-form (e.g. "deposit", "signup", "share").
    #[serde(rename = "type")]
    conversion_type: String,
    #[serde(default)]
    idempotency_key: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

crate::impl_container!(CustomParser);
pub struct CustomParser;

impl ConversionParser for CustomParser {
    fn parse(&self, body: &[u8], _source: &Source) -> Result<Vec<ParsedConversion>, ParseError> {
        let payload: CustomPayload =
            serde_json::from_slice(body).map_err(|e| ParseError::InvalidPayload(e.to_string()))?;

        if payload.user_id.trim().is_empty() {
            return Err(ParseError::MissingField("user_id"));
        }
        if payload.conversion_type.trim().is_empty() {
            return Err(ParseError::MissingField("type"));
        }

        if let Some(key) = &payload.idempotency_key {
            if key.len() > MAX_IDEMPOTENCY_KEY_LEN {
                return Err(ParseError::IdempotencyKeyTooLong(key.len()));
            }
        }

        // Convert metadata to BSON Document and enforce size cap.
        let metadata = match payload.metadata {
            Some(value) => {
                let bson = to_bson(&value)
                    .map_err(|e| ParseError::InvalidPayload(format!("metadata: {e}")))?;
                match bson {
                    mongodb::bson::Bson::Document(doc) => {
                        let size = doc_size_estimate(&doc);
                        if size > MAX_METADATA_BYTES {
                            return Err(ParseError::MetadataTooLarge(size));
                        }
                        Some(doc)
                    }
                    _ => {
                        return Err(ParseError::InvalidPayload(
                            "metadata must be a JSON object".to_string(),
                        ));
                    }
                }
            }
            None => None,
        };

        Ok(vec![ParsedConversion {
            user_id: Some(payload.user_id),
            conversion_type: payload.conversion_type,
            idempotency_key: payload.idempotency_key,
            metadata,
            occurred_at: None,
        }])
    }
}

/// Rough size estimate for a BSON document to enforce the 1KB metadata cap.
/// Not exact — uses the serialized JSON length as a proxy, which is fine for
/// a soft cap. If it matters, we can use `bson::to_vec` instead.
fn doc_size_estimate(doc: &Document) -> usize {
    serde_json::to_vec(doc).map(|v| v.len()).unwrap_or(0)
}

#[cfg(test)]
#[path = "parsers_tests.rs"]
mod tests;
