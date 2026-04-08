#[derive(Debug, Clone)]
pub enum ClientCredentials {
    SecretKey(String),
    PublishableKey(String),
}
