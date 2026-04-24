use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use url::Url;

const FEED_TIMEOUT: Duration = Duration::from_secs(30);

/// In-memory threat intelligence, populated from free feeds.
/// Checks both exact URLs (malware) and domains (phishing).
#[derive(Clone, Default)]
pub struct ThreatFeed {
    /// Exact malicious URLs (from URLhaus — malware distribution).
    pub urls: Arc<RwLock<HashSet<String>>>,
    /// Known phishing domains (from Phishing.Database).
    pub domains: Arc<RwLock<HashSet<String>>>,
}

impl ThreatFeed {
    pub fn new() -> Self {
        Self {
            urls: Arc::new(RwLock::new(HashSet::new())),
            domains: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Check a URL against both the exact URL set and the domain set.
    /// Returns Some(reason) if matched, None if clean.
    pub async fn check_url(&self, raw_url: &str) -> Option<String> {
        let normalized = normalize_url(raw_url);

        // Check exact URL match (URLhaus — malware).
        if self.urls.read().await.contains(&normalized) {
            return Some(format!("URL matches known malware feed: {raw_url}"));
        }

        // Extract domain and check against phishing domain set.
        if let Ok(parsed) = Url::parse(raw_url) {
            if let Some(host) = parsed.host_str() {
                let host_lower = host.to_lowercase();
                let domains = self.domains.read().await;
                if domains.contains(&host_lower) {
                    return Some(format!("Domain '{host_lower}' is a known phishing domain"));
                }
                // Also check parent domains (e.g., evil.example.com -> example.com).
                let parts: Vec<&str> = host_lower.split('.').collect();
                for i in 1..parts.len().saturating_sub(1) {
                    let parent = parts[i..].join(".");
                    if domains.contains(&parent) {
                        return Some(format!(
                            "Domain '{host_lower}' is a subdomain of known phishing domain '{parent}'"
                        ));
                    }
                }
            }
        }

        None
    }

    /// Refresh all feeds. Only replaces data if at least one source succeeds.
    pub async fn refresh(&self) {
        let client = reqwest::Client::builder()
            .timeout(FEED_TIMEOUT)
            .build()
            .unwrap_or_default();

        // URLhaus — exact malware URLs.
        let mut new_urls = HashSet::new();
        match fetch_urlhaus(&client).await {
            Ok(urls) => {
                tracing::info!(count = urls.len(), "Loaded URLhaus feed");
                new_urls = urls;
            }
            Err(e) => tracing::warn!(error = %e, "Failed to fetch URLhaus feed"),
        }

        if !new_urls.is_empty() {
            *self.urls.write().await = new_urls;
        }

        // Phishing.Database — phishing domains.
        match fetch_phishing_domains(&client).await {
            Ok(domains) => {
                tracing::info!(count = domains.len(), "Loaded Phishing.Database domains");
                if !domains.is_empty() {
                    *self.domains.write().await = domains;
                }
            }
            Err(e) => tracing::warn!(error = %e, "Failed to fetch Phishing.Database"),
        }
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

/// Fetch URLhaus text feed (one URL per line, # comments).
async fn fetch_urlhaus(client: &reqwest::Client) -> Result<HashSet<String>, String> {
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

/// Fetch Phishing.Database active domains (one domain per line).
async fn fetch_phishing_domains(client: &reqwest::Client) -> Result<HashSet<String>, String> {
    let url = "https://raw.githubusercontent.com/Phishing-Database/Phishing.Database/master/phishing-domains-ACTIVE.txt";
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;

    Ok(text
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.trim().to_lowercase())
        .filter(|l| !l.is_empty())
        .collect())
}

#[cfg(test)]
#[path = "threat_feed_tests.rs"]
mod tests;
