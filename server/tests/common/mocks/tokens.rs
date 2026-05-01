use async_trait::async_trait;
use mongodb::bson;
use std::sync::Mutex;

use rift::services::tokens::models::TokenDoc;
use rift::services::tokens::repo::TokensRepository;
use rift::services::tokens::TokenPurpose;

/// In-memory TokensRepository for integration tests. Mirrors the
/// semantics of `TokensRepoMongo` closely enough that the TokenService
/// behaves identically — wrong hashes return None, tuple-keyed docs are
/// deleted on successful consume, etc.
#[derive(Default)]
pub struct MockTokensRepo {
    pub docs: Mutex<Vec<TokenDoc>>,
}

#[async_trait]
impl TokensRepository for MockTokensRepo {
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
