#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub base_url: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.riftl.ink".to_string(),
        }
    }
}
