use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AuthError;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtHeader {
    alg: String,
    typ: String,
}

impl Default for JwtHeader {
    fn default() -> Self {
        Self {
            alg: "HS256".into(),
            typ: "JWT".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: u64,
    pub exp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Claims {
    pub fn new(subject: &str, ttl_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_secs();

        Self {
            sub: subject.into(),
            iat: now,
            exp: now + ttl_secs,
            iss: None,
            aud: None,
            custom: HashMap::new(),
        }
    }

    pub fn with_issuer(mut self, issuer: &str) -> Self {
        self.iss = Some(issuer.into());
        self
    }

    pub fn with_audience(mut self, audience: &str) -> Self {
        self.aud = Some(audience.into());
        self
    }

    pub fn with_claim(mut self, key: &str, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.exp
    }
}

pub fn encode(claims: &Claims, secret: &[u8]) -> String {
    let header = JwtHeader::default();
    let header_json = serde_json::to_vec(&header).expect("header serialization cannot fail");
    let payload_json = serde_json::to_vec(claims).expect("claims serialization cannot fail");

    let header_b64 = URL_SAFE_NO_PAD.encode(&header_json);
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload_json);

    let signing_input = format!("{}.{}", header_b64, payload_b64);

    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(signing_input.as_bytes());
    let signature = mac.finalize().into_bytes();
    let sig_b64 = URL_SAFE_NO_PAD.encode(signature);

    format!("{}.{}", signing_input, sig_b64)
}

pub fn decode(token: &str, secret: &[u8]) -> Result<Claims, AuthError> {
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidJwt("expected 3 parts".into()));
    }

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let signature = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| AuthError::InvalidJwt("bad signature encoding".into()))?;

    let mut mac = HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(signing_input.as_bytes());
    mac.verify_slice(&signature)
        .map_err(|_| AuthError::InvalidJwt("signature mismatch".into()))?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| AuthError::InvalidJwt("bad payload encoding".into()))?;

    let claims: Claims = serde_json::from_slice(&payload_bytes)
        .map_err(|e| AuthError::InvalidJwt(format!("bad payload: {e}")))?;

    if claims.is_expired() {
        return Err(AuthError::ExpiredJwt);
    }

    Ok(claims)
}

pub fn decode_without_verify(token: &str) -> Result<Claims, AuthError> {
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(AuthError::InvalidJwt("expected 3 parts".into()));
    }

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| AuthError::InvalidJwt("bad payload encoding".into()))?;

    serde_json::from_slice(&payload_bytes)
        .map_err(|e| AuthError::InvalidJwt(format!("bad payload: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"jwt-test-secret-32-bytes-long!!!";

    #[test]
    fn encode_decode_roundtrip() {
        let claims = Claims::new("user-123", 3600);
        let token = encode(&claims, SECRET);
        let decoded = decode(&token, SECRET).unwrap();
        assert_eq!(decoded.sub, "user-123");
    }

    #[test]
    fn wrong_secret_rejected() {
        let claims = Claims::new("user-123", 3600);
        let token = encode(&claims, SECRET);
        assert!(decode(&token, b"wrong-secret-key-32-bytes!!!!!").is_err());
    }

    #[test]
    fn expired_token_rejected() {
        let mut claims = Claims::new("user-123", 0);
        claims.exp = claims.iat - 1;
        let token = encode(&claims, SECRET);
        assert!(matches!(decode(&token, SECRET), Err(AuthError::ExpiredJwt)));
    }

    #[test]
    fn custom_claims() {
        let claims = Claims::new("user-123", 3600)
            .with_issuer("adapto")
            .with_audience("api")
            .with_claim("role", serde_json::json!("admin"));
        let token = encode(&claims, SECRET);
        let decoded = decode(&token, SECRET).unwrap();
        assert_eq!(decoded.iss.as_deref(), Some("adapto"));
        assert_eq!(decoded.aud.as_deref(), Some("api"));
        assert_eq!(decoded.custom.get("role").unwrap(), "admin");
    }

    #[test]
    fn tampered_payload_rejected() {
        let claims = Claims::new("user-123", 3600);
        let token = encode(&claims, SECRET);
        let parts: Vec<&str> = token.splitn(3, '.').collect();
        let fake_payload = URL_SAFE_NO_PAD.encode(b"{\"sub\":\"admin\",\"iat\":0,\"exp\":999999999999}");
        let tampered = format!("{}.{}.{}", parts[0], fake_payload, parts[2]);
        assert!(decode(&tampered, SECRET).is_err());
    }

    #[test]
    fn decode_without_verify_works() {
        let claims = Claims::new("user-123", 3600).with_issuer("test");
        let token = encode(&claims, SECRET);
        let decoded = decode_without_verify(&token).unwrap();
        assert_eq!(decoded.sub, "user-123");
        assert_eq!(decoded.iss.as_deref(), Some("test"));
    }

    #[test]
    fn malformed_token_rejected() {
        assert!(decode("not-a-jwt", SECRET).is_err());
        assert!(decode("a.b", SECRET).is_err());
    }
}
