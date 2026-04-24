//! TokenService — the one place that owns token lifecycle (issue + consume).

use std::sync::Arc;

use crate::services::auth::keys;
use crate::services::tokens::model::{ConsumeOutcome, TokenKind, TokenPurpose, TokenSpec};
use crate::services::tokens::repo::{new_token_doc, random_doc_id, TokensRepository};

pub struct TokenService {
    repo: Arc<dyn TokensRepository>,
}

impl TokenService {
    pub fn new(repo: Arc<dyn TokensRepository>) -> Self {
        Self { repo }
    }

    /// Mint a fresh token. Returns the raw token (64-char hex for hash-keyed,
    /// 6-char alphanumeric for tuple-keyed) — caller embeds in email/URL.
    /// The hashed form is what's stored.
    pub async fn issue(&self, spec: TokenSpec) -> Result<String, String> {
        match spec.kind {
            TokenKind::HashKeyed => {
                let raw = keys::generate_verify_token();
                let hash = keys::hash_key(&raw);
                let doc = new_token_doc(
                    hash.clone(),
                    spec.purpose,
                    spec.email,
                    hash,
                    spec.metadata,
                    1, // single-use
                    spec.ttl_secs,
                );
                self.repo.insert(&doc).await?;
                Ok(raw)
            }
            TokenKind::TupleKeyed { max_attempts } => {
                // Supersede any pending request for this (purpose, email).
                // Matches the old `secret_key_create_requests` behavior where
                // re-requesting dropped the previous code.
                self.repo
                    .delete_pending_tuple(spec.purpose, &spec.email)
                    .await?;
                let raw = keys::generate_key_create_code();
                let hash = keys::hash_key(&raw);
                let doc = new_token_doc(
                    random_doc_id(),
                    spec.purpose,
                    spec.email,
                    hash,
                    spec.metadata,
                    max_attempts,
                    spec.ttl_secs,
                );
                self.repo.insert(&doc).await?;
                Ok(raw)
            }
        }
    }

    /// Consume a HashKeyed token. The raw token is hashed, looked up by `_id`,
    /// and atomically marked consumed. Returns the doc's purpose + metadata
    /// so the caller can dispatch (e.g. Stripe Checkout vs Portal).
    pub async fn consume_hash(&self, raw: &str) -> Result<ConsumeOutcome, String> {
        let hash = keys::hash_key(raw);
        match self.repo.consume_by_hash(&hash).await? {
            Some(doc) => Ok(ConsumeOutcome::Ok {
                purpose: doc.purpose,
                email: doc.email,
                metadata: doc.metadata,
            }),
            None => Ok(ConsumeOutcome::NotFound),
        }
    }

    /// Consume a TupleKeyed code. Looks up by `(purpose, email)`, increments
    /// `attempts`, and then checks the hash. Caps attempts: once `attempts >
    /// max_attempts` the doc is deleted and further calls return `NotFound`
    /// (the doc is gone). Normalizes casing on the raw code — callers don't
    /// need to uppercase.
    pub async fn consume_tuple(
        &self,
        raw: &str,
        purpose: TokenPurpose,
        email: &str,
    ) -> Result<ConsumeOutcome, String> {
        let normalized = raw.trim().to_uppercase();
        let hash = keys::hash_key(&normalized);

        let doc = match self.repo.increment_tuple_attempts(purpose, email).await? {
            Some(d) => d,
            None => return Ok(ConsumeOutcome::NotFound),
        };

        if doc.attempts > doc.max_attempts {
            // Attempt cap exceeded — clean up so further calls see nothing
            // and the user is forced to request a new code.
            self.repo.delete_tuple(purpose, email).await?;
            return Ok(ConsumeOutcome::AttemptsExhausted);
        }

        if doc.token_hash != hash {
            return Ok(ConsumeOutcome::NotFound);
        }

        // Valid code — consume by deleting.
        match self.repo.delete_tuple(purpose, email).await? {
            Some(d) => Ok(ConsumeOutcome::Ok {
                purpose: d.purpose,
                email: d.email,
                metadata: d.metadata,
            }),
            None => Ok(ConsumeOutcome::NotFound),
        }
    }

    /// How many tokens of `purpose` were created for `email` in the last
    /// `window_secs`. Used for per-email rate limiting on public endpoints.
    pub async fn count_recent(
        &self,
        purpose: TokenPurpose,
        email: &str,
        window_secs: i64,
    ) -> Result<u64, String> {
        self.repo
            .count_recent_for_email(purpose, email, window_secs)
            .await
    }

    /// True if there's an active (not expired, not consumed) token for
    /// `(purpose, email)`. Used for cooldown enforcement.
    pub async fn pending_exists(&self, purpose: TokenPurpose, email: &str) -> Result<bool, String> {
        self.repo.pending_exists(purpose, email).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tokens::model::{TokenKind, TokenPurpose, TokenSpec};
    use crate::services::tokens::repo::TokenDoc;
    use async_trait::async_trait;
    use mongodb::bson::{self, doc};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemStore {
        docs: Mutex<Vec<TokenDoc>>,
    }

    #[async_trait]
    impl TokensRepository for MemStore {
        async fn insert(&self, doc: &TokenDoc) -> Result<(), String> {
            self.docs.lock().unwrap().push(doc.clone());
            Ok(())
        }

        async fn delete_pending_tuple(
            &self,
            purpose: TokenPurpose,
            email: &str,
        ) -> Result<u64, String> {
            let mut g = self.docs.lock().unwrap();
            let before = g.len();
            g.retain(|d| !(d.purpose == purpose && d.email == email && d.consumed_at.is_none()));
            Ok((before - g.len()) as u64)
        }

        async fn consume_by_hash(&self, token_hash: &str) -> Result<Option<TokenDoc>, String> {
            let mut g = self.docs.lock().unwrap();
            let now = bson::DateTime::now();
            if let Some(pos) = g
                .iter()
                .position(|d| d.id == token_hash && d.consumed_at.is_none() && d.expires_at > now)
            {
                let mut d = g.remove(pos);
                d.consumed_at = Some(now);
                Ok(Some(d))
            } else {
                Ok(None)
            }
        }

        async fn increment_tuple_attempts(
            &self,
            purpose: TokenPurpose,
            email: &str,
        ) -> Result<Option<TokenDoc>, String> {
            let mut g = self.docs.lock().unwrap();
            let now = bson::DateTime::now();
            for d in g.iter_mut() {
                if d.purpose == purpose
                    && d.email == email
                    && d.consumed_at.is_none()
                    && d.expires_at > now
                {
                    d.attempts += 1;
                    return Ok(Some(d.clone()));
                }
            }
            Ok(None)
        }

        async fn delete_tuple(
            &self,
            purpose: TokenPurpose,
            email: &str,
        ) -> Result<Option<TokenDoc>, String> {
            let mut g = self.docs.lock().unwrap();
            if let Some(pos) = g
                .iter()
                .position(|d| d.purpose == purpose && d.email == email)
            {
                Ok(Some(g.remove(pos)))
            } else {
                Ok(None)
            }
        }

        async fn count_recent_for_email(
            &self,
            purpose: TokenPurpose,
            email: &str,
            _window_secs: i64,
        ) -> Result<u64, String> {
            let g = self.docs.lock().unwrap();
            Ok(g.iter()
                .filter(|d| d.purpose == purpose && d.email == email)
                .count() as u64)
        }

        async fn pending_exists(&self, purpose: TokenPurpose, email: &str) -> Result<bool, String> {
            let now = bson::DateTime::now();
            let g = self.docs.lock().unwrap();
            Ok(g.iter().any(|d| {
                d.purpose == purpose
                    && d.email == email
                    && d.consumed_at.is_none()
                    && d.expires_at > now
            }))
        }
    }

    fn service() -> TokenService {
        TokenService::new(Arc::new(MemStore::default()))
    }

    #[tokio::test]
    async fn hash_keyed_round_trip_carries_metadata() {
        let svc = service();
        let raw = svc
            .issue(TokenSpec {
                purpose: TokenPurpose::EmailVerify,
                kind: TokenKind::HashKeyed,
                ttl_secs: 3600,
                email: "u@x.com".into(),
                metadata: doc! { "user_id": "abc" },
            })
            .await
            .unwrap();

        match svc.consume_hash(&raw).await.unwrap() {
            ConsumeOutcome::Ok {
                purpose,
                email,
                metadata,
            } => {
                assert!(matches!(purpose, TokenPurpose::EmailVerify));
                assert_eq!(email, "u@x.com");
                assert_eq!(metadata.get_str("user_id").unwrap(), "abc");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn hash_keyed_is_single_use() {
        let svc = service();
        let raw = svc
            .issue(TokenSpec {
                purpose: TokenPurpose::EmailVerify,
                kind: TokenKind::HashKeyed,
                ttl_secs: 3600,
                email: "u@x.com".into(),
                metadata: doc! {},
            })
            .await
            .unwrap();
        assert!(matches!(
            svc.consume_hash(&raw).await.unwrap(),
            ConsumeOutcome::Ok { .. }
        ));
        assert!(matches!(
            svc.consume_hash(&raw).await.unwrap(),
            ConsumeOutcome::NotFound
        ));
    }

    #[tokio::test]
    async fn tuple_keyed_wrong_code_increments_then_caps() {
        let svc = service();
        let _real = svc
            .issue(TokenSpec {
                purpose: TokenPurpose::KeyRotation,
                kind: TokenKind::TupleKeyed { max_attempts: 5 },
                ttl_secs: 900,
                email: "u@x.com".into(),
                metadata: doc! {},
            })
            .await
            .unwrap();

        for _ in 0..5 {
            assert!(matches!(
                svc.consume_tuple("WRONG1", TokenPurpose::KeyRotation, "u@x.com")
                    .await
                    .unwrap(),
                ConsumeOutcome::NotFound
            ));
        }
        // 6th attempt → AttemptsExhausted (attempts becomes 6 > max 5).
        assert!(matches!(
            svc.consume_tuple("WRONG1", TokenPurpose::KeyRotation, "u@x.com")
                .await
                .unwrap(),
            ConsumeOutcome::AttemptsExhausted
        ));
        // Subsequent calls: doc is gone → NotFound.
        assert!(matches!(
            svc.consume_tuple("WRONG1", TokenPurpose::KeyRotation, "u@x.com")
                .await
                .unwrap(),
            ConsumeOutcome::NotFound
        ));
    }

    #[tokio::test]
    async fn tuple_keyed_correct_code_wins() {
        let svc = service();
        let raw = svc
            .issue(TokenSpec {
                purpose: TokenPurpose::KeyRotation,
                kind: TokenKind::TupleKeyed { max_attempts: 5 },
                ttl_secs: 900,
                email: "u@x.com".into(),
                metadata: doc! { "tenant_id": "t1" },
            })
            .await
            .unwrap();

        match svc
            .consume_tuple(&raw, TokenPurpose::KeyRotation, "u@x.com")
            .await
            .unwrap()
        {
            ConsumeOutcome::Ok { metadata, .. } => {
                assert_eq!(metadata.get_str("tenant_id").unwrap(), "t1");
            }
            other => panic!("expected Ok, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn issuing_tuple_keyed_supersedes_previous() {
        let svc = service();
        let raw1 = svc
            .issue(TokenSpec {
                purpose: TokenPurpose::KeyRotation,
                kind: TokenKind::TupleKeyed { max_attempts: 5 },
                ttl_secs: 900,
                email: "u@x.com".into(),
                metadata: doc! {},
            })
            .await
            .unwrap();
        let _raw2 = svc
            .issue(TokenSpec {
                purpose: TokenPurpose::KeyRotation,
                kind: TokenKind::TupleKeyed { max_attempts: 5 },
                ttl_secs: 900,
                email: "u@x.com".into(),
                metadata: doc! {},
            })
            .await
            .unwrap();
        // First raw is dead; second one wins.
        assert!(matches!(
            svc.consume_tuple(&raw1, TokenPurpose::KeyRotation, "u@x.com")
                .await
                .unwrap(),
            ConsumeOutcome::NotFound
        ));
    }
}
