#[derive(Debug, Clone)]
pub enum ClientCredentials {
    SecretKey(String),
    PublishableKey(String),
    /// Browser-login session token (no `rl_`/`pk_` prefix). Sent as a bearer;
    /// the server routes a prefix-less bearer to its session credential.
    SessionToken(String),
}
