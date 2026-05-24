use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::AuthError;

type HmacSha256 = Hmac<Sha256>;

/// Maximum age of a CSRF token before it is considered expired (1 hour).
const MAX_TOKEN_AGE_SECS: u64 = 3600;

/// Generate a CSRF token using HMAC-SHA256.
///
/// Token format: `base64(timestamp_bytes) . base64(hmac(timestamp_bytes, secret))`
///
/// The timestamp is embedded so the server can reject stale tokens without
/// storing any state.
pub fn generate_token(secret: &[u8]) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();

    let timestamp_bytes = timestamp.to_be_bytes();

    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(&timestamp_bytes);
    let signature = mac.finalize().into_bytes();

    let ts_encoded = URL_SAFE_NO_PAD.encode(timestamp_bytes);
    let sig_encoded = URL_SAFE_NO_PAD.encode(signature);

    format!("{}.{}", ts_encoded, sig_encoded)
}

/// Validate a CSRF token against the shared secret.
///
/// Checks both the HMAC signature and the token age.
pub fn validate_token(token: &str, secret: &[u8]) -> Result<(), AuthError> {
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    if parts.len() != 2 {
        return Err(AuthError::MalformedToken);
    }

    let timestamp_bytes = URL_SAFE_NO_PAD
        .decode(parts[0])
        .map_err(|_| AuthError::MalformedToken)?;

    if timestamp_bytes.len() != 8 {
        return Err(AuthError::MalformedToken);
    }

    let signature_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| AuthError::MalformedToken)?;

    // Verify HMAC signature.
    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(&timestamp_bytes);
    mac.verify_slice(&signature_bytes)
        .map_err(|_| AuthError::InvalidCsrfToken)?;

    // Verify token age.
    let mut ts_arr = [0u8; 8];
    ts_arr.copy_from_slice(&timestamp_bytes);
    let token_time = u64::from_be_bytes(ts_arr);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();

    if now.saturating_sub(token_time) > MAX_TOKEN_AGE_SECS {
        return Err(AuthError::ExpiredCsrfToken);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let secret = b"test-secret-key-32-bytes-long!!!";
        let token = generate_token(secret);
        assert!(validate_token(&token, secret).is_ok());
    }
}
