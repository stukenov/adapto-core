use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::AuthError;

type HmacSha256 = Hmac<Sha256>;

/// Sign a session ID with the shared secret.
///
/// Returns a string in the format `session_id.base64(hmac(session_id, secret))`.
/// The session ID itself is transmitted in the clear; the signature prevents
/// forgery.
pub fn sign_session_id(session_id: &str, secret: &[u8]) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(session_id.as_bytes());
    let signature = mac.finalize().into_bytes();

    let sig_encoded = URL_SAFE_NO_PAD.encode(signature);
    format!("{}.{}", session_id, sig_encoded)
}

/// Verify a signed session ID and return the original session ID on success.
///
/// The signed value must be in the format produced by [`sign_session_id`].
pub fn verify_session_id(signed: &str, secret: &[u8]) -> Result<String, AuthError> {
    let dot_pos = signed.rfind('.').ok_or(AuthError::MalformedToken)?;

    let session_id = &signed[..dot_pos];
    let sig_part = &signed[dot_pos + 1..];

    if session_id.is_empty() || sig_part.is_empty() {
        return Err(AuthError::MalformedToken);
    }

    let signature_bytes = URL_SAFE_NO_PAD
        .decode(sig_part)
        .map_err(|_| AuthError::MalformedToken)?;

    let mut mac =
        HmacSha256::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(session_id.as_bytes());
    mac.verify_slice(&signature_bytes)
        .map_err(|_| AuthError::InvalidSessionSignature)?;

    Ok(session_id.to_string())
}
