use rand::Rng;
use sha2::{Digest, Sha256};

const KEY_PREFIX: &str = "rl_live_";
const SDK_KEY_PREFIX: &str = "pk_live_";
const KEY_RANDOM_BYTES: usize = 24;

/// Generate a new API key. Returns (full_key, sha256_hash, display_prefix).
pub fn generate_api_key() -> (String, String, String) {
    let random_bytes: Vec<u8> = rand::rng()
        .random_iter::<u8>()
        .take(KEY_RANDOM_BYTES)
        .collect();
    let random_hex = hex::encode(&random_bytes);
    let full_key = format!("{KEY_PREFIX}{random_hex}");

    let hash = hex::encode(Sha256::digest(full_key.as_bytes()));
    let prefix = format!("{}{}...", KEY_PREFIX, &random_hex[..8]);

    (full_key, hash, prefix)
}

/// Generate a new SDK key. Returns (full_key, sha256_hash, display_prefix).
pub fn generate_sdk_key() -> (String, String, String) {
    let random_bytes: Vec<u8> = rand::rng()
        .random_iter::<u8>()
        .take(KEY_RANDOM_BYTES)
        .collect();
    let random_hex = hex::encode(&random_bytes);
    let full_key = format!("{SDK_KEY_PREFIX}{random_hex}");

    let hash = hex::encode(Sha256::digest(full_key.as_bytes()));
    let prefix = format!("{}{}...", SDK_KEY_PREFIX, &random_hex[..8]);

    (full_key, hash, prefix)
}

/// Generate a random verification token (URL-safe).
pub fn generate_verify_token() -> String {
    let bytes: Vec<u8> = rand::rng().random_iter::<u8>().take(32).collect();
    hex::encode(&bytes)
}

/// Hash an API key for lookup.
pub fn hash_key(key: &str) -> String {
    hex::encode(Sha256::digest(key.as_bytes()))
}

/// Generate a 6-character uppercase alphanumeric code for email confirmation.
pub fn generate_key_create_code() -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..6)
        .map(|_| {
            let idx = rng.random_range(0..CHARS.len());
            CHARS[idx] as char
        })
        .collect()
}
