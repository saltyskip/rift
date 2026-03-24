use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

const FEED_TIMEOUT: Duration = Duration::from_secs(30);

/// In-memory set of known malicious URLs, populated from free threat feeds.
#[derive(Clone, Default)]
pub struct ThreatFeed {
    urls: Arc<RwLock<HashSet<String>>>,
}

impl ThreatFeed {
    pub fn new() -> Self {
        Self {
            urls: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Check if any of the given URLs are in the threat feed.
    pub async fn check_urls(&self, urls: &[&str]) -> Option<String> {
        let feed = self.urls.read().await;
        for url in urls {
            let normalized = normalize_url(url);
            if feed.contains(&normalized) {
                return Some(url.to_string());
            }
        }
        None
    }

    /// Refresh the feed from all sources.
    /// Only replaces the feed if at least one source returned data.
    pub async fn refresh(&self) {
        let client = reqwest::Client::builder()
            .timeout(FEED_TIMEOUT)
            .build()
            .unwrap_or_default();

        let mut new_urls = HashSet::new();

        match fetch_urlhaus(&client).await {
            Ok(urls) => {
                tracing::info!(count = urls.len(), "Loaded URLhaus feed");
                new_urls.extend(urls);
            }
            Err(e) => tracing::warn!(error = %e, "Failed to fetch URLhaus feed"),
        }

        match fetch_openphish(&client).await {
            Ok(urls) => {
                tracing::info!(count = urls.len(), "Loaded OpenPhish feed");
                new_urls.extend(urls);
            }
            Err(e) => tracing::warn!(error = %e, "Failed to fetch OpenPhish feed"),
        }

        if new_urls.is_empty() {
            tracing::warn!("No threat feed data received — keeping existing feed");
            return;
        }

        let count = new_urls.len();
        *self.urls.write().await = new_urls;
        tracing::info!(total = count, "Threat feed updated");
    }

    /// Start a background task that refreshes the feed periodically.
    pub fn start_background_refresh(self, interval_secs: u64) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                self.refresh().await;
            }
        });
    }
}

/// Normalize a URL for matching: lowercase, strip trailing slash, strip fragment.
fn normalize_url(url: &str) -> String {
    let mut s = url.trim().to_lowercase();
    if let Some(idx) = s.find('#') {
        s.truncate(idx);
    }
    while s.ends_with('/') {
        s.pop();
    }
    s
}

async fn fetch_urlhaus(client: &reqwest::Client) -> Result<Vec<String>, String> {
    let url = "https://urlhaus.abuse.ch/downloads/text_online/";
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;

    Ok(text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .map(|l| normalize_url(l.trim()))
        .filter(|l| !l.is_empty())
        .collect())
}

async fn fetch_openphish(client: &reqwest::Client) -> Result<Vec<String>, String> {
    let url = "https://openphish.com/feed.txt";
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;

    Ok(text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| normalize_url(l.trim()))
        .filter(|l| !l.is_empty())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_fragment_and_trailing_slash() {
        assert_eq!(normalize_url("https://example.com/"), "https://example.com");
        assert_eq!(
            normalize_url("https://example.com/path#frag"),
            "https://example.com/path"
        );
        assert_eq!(normalize_url("HTTPS://EXAMPLE.COM"), "https://example.com");
    }

    #[tokio::test]
    async fn check_urls_returns_match() {
        let feed = ThreatFeed::new();
        {
            let mut urls = feed.urls.write().await;
            urls.insert("https://evil.com/phish".to_string());
        }

        assert_eq!(
            feed.check_urls(&["https://example.com", "https://evil.com/phish"])
                .await,
            Some("https://evil.com/phish".to_string())
        );
        assert_eq!(feed.check_urls(&["https://safe.com"]).await, None);
    }
}
