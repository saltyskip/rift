//! Coinbase Developer Platform (CDP) authenticated facilitator client.

use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use x402_types::proto::{self, VerifyRequest};

const CDP_HOST: &str = "api.cdp.coinbase.com";

crate::impl_container!(CdpFacilitator);
#[derive(Clone)]
pub struct CdpFacilitator {
    api_key_id: String,
    signing_key: SigningKey,
    base_path: String,
    http: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct JwtHeader<'a> {
    alg: &'a str,
    typ: &'a str,
    kid: &'a str,
    nonce: String,
}

#[derive(Debug, Serialize)]
struct JwtClaims<'a> {
    sub: &'a str,
    iss: &'a str,
    aud: Vec<&'a str>,
    nbf: u64,
    exp: u64,
    uri: String,
}

fn b64url_encode(data: &[u8]) -> String {
    use x402_types::util::Base64Bytes;
    let standard = Base64Bytes::encode(data).to_string();
    standard
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

fn std_b64_decode(input: &str) -> Vec<u8> {
    x402_types::util::Base64Bytes::from(input.as_bytes())
        .decode()
        .expect("Invalid base64")
        .to_vec()
}

fn extract_path(url: &str) -> String {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|rest| rest.find('/').map(|i| &rest[i..]))
        .unwrap_or("/")
        .trim_end_matches('/')
        .to_string()
}

impl CdpFacilitator {
    pub fn new(api_key_id: &str, api_key_secret: &str, facilitator_url: &str) -> Self {
        let secret_bytes = std_b64_decode(api_key_secret);
        let seed: [u8; 32] = secret_bytes[..32]
            .try_into()
            .expect("CDP secret too short (need 64 bytes base64-encoded)");
        let signing_key = SigningKey::from_bytes(&seed);
        let base_path = extract_path(facilitator_url);

        Self {
            api_key_id: api_key_id.to_string(),
            signing_key,
            base_path,
            http: reqwest::Client::new(),
        }
    }

    fn make_jwt(&self, method: &str, path: &str) -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let nonce = hex::encode(rand::random::<[u8; 16]>());

        let header = JwtHeader {
            alg: "EdDSA",
            typ: "JWT",
            kid: &self.api_key_id,
            nonce,
        };

        let claims = JwtClaims {
            sub: &self.api_key_id,
            iss: "cdp",
            aud: vec!["cdp_service"],
            nbf: now,
            exp: now + 120,
            uri: format!("{method} {CDP_HOST}{path}"),
        };

        let header_b64 = b64url_encode(&serde_json::to_vec(&header).unwrap());
        let claims_b64 = b64url_encode(&serde_json::to_vec(&claims).unwrap());
        let message = format!("{header_b64}.{claims_b64}");

        let signature = self.signing_key.sign(message.as_bytes());
        let sig_b64 = b64url_encode(&signature.to_bytes());

        format!("{message}.{sig_b64}")
    }

    async fn post<T: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
        body: &T,
    ) -> Result<R, String> {
        let path = format!("{}{endpoint}", self.base_path);
        let jwt = self.make_jwt("POST", &path);
        let url = format!("https://{CDP_HOST}{path}");

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {jwt}"))
            .json(body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        if resp.status().is_success() {
            resp.json()
                .await
                .map_err(|e| format!("JSON parse error: {e}"))
        } else {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            Err(format!("CDP {status}: {text}"))
        }
    }

    pub async fn verify(&self, request: &VerifyRequest) -> Result<proto::VerifyResponse, String> {
        self.post("/verify", request).await
    }

    pub async fn settle(
        &self,
        request: &proto::SettleRequest,
    ) -> Result<proto::SettleResponse, String> {
        self.post("/settle", request).await
    }
}
