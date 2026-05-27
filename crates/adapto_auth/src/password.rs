use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::AuthError;

type HmacSha256 = Hmac<Sha256>;

const ITERATIONS: u32 = 100_000;
const SALT_LEN: usize = 16;
const HASH_LEN: usize = 32;

fn random_salt() -> [u8; SALT_LEN] {
    let mut salt = [0u8; SALT_LEN];
    getrandom::getrandom(&mut salt).expect("OS RNG failure");
    salt
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], iterations: u32) -> [u8; HASH_LEN] {
    let mut mac = HmacSha256::new_from_slice(password).expect("HMAC accepts any key length");
    mac.update(salt);
    mac.update(&1u32.to_be_bytes());
    let u1 = mac.finalize().into_bytes();

    let mut result = [0u8; HASH_LEN];
    result.copy_from_slice(&u1);
    let mut prev = u1;

    for _ in 1..iterations {
        let mut mac =
            HmacSha256::new_from_slice(password).expect("HMAC accepts any key length");
        mac.update(&prev);
        let ui = mac.finalize().into_bytes();
        for (r, u) in result.iter_mut().zip(ui.iter()) {
            *r ^= *u;
        }
        prev = ui;
    }

    result
}

pub fn hash_password(password: &str) -> String {
    let salt = random_salt();
    let hash = pbkdf2_hmac_sha256(password.as_bytes(), &salt, ITERATIONS);
    format!(
        "pbkdf2-sha256${}${}${}",
        ITERATIONS,
        URL_SAFE_NO_PAD.encode(salt),
        URL_SAFE_NO_PAD.encode(hash)
    )
}

pub fn hash_password_with_salt(password: &str, salt: &[u8]) -> String {
    let hash = pbkdf2_hmac_sha256(password.as_bytes(), salt, ITERATIONS);
    format!(
        "pbkdf2-sha256${}${}${}",
        ITERATIONS,
        URL_SAFE_NO_PAD.encode(salt),
        URL_SAFE_NO_PAD.encode(hash)
    )
}

pub fn verify_password(password: &str, encoded: &str) -> Result<(), AuthError> {
    let parts: Vec<&str> = encoded.splitn(4, '$').collect();
    if parts.len() != 4 || parts[0] != "pbkdf2-sha256" {
        return Err(AuthError::InvalidPasswordHash);
    }

    let iterations: u32 = parts[1]
        .parse()
        .map_err(|_| AuthError::InvalidPasswordHash)?;

    let salt = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|_| AuthError::InvalidPasswordHash)?;

    let stored_hash = URL_SAFE_NO_PAD
        .decode(parts[3])
        .map_err(|_| AuthError::InvalidPasswordHash)?;

    let computed = pbkdf2_hmac_sha256(password.as_bytes(), &salt, iterations);

    if constant_time_eq(&computed, &stored_hash) {
        Ok(())
    } else {
        Err(AuthError::PasswordMismatch)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

pub fn validate_password_strength(password: &str) -> Vec<&'static str> {
    let mut issues = Vec::new();
    if password.len() < 8 {
        issues.push("must be at least 8 characters");
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        issues.push("must contain an uppercase letter");
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        issues.push("must contain a lowercase letter");
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        issues.push("must contain a digit");
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        issues.push("must contain a special character");
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify() {
        let hash = hash_password("MyP@ssw0rd!");
        assert!(verify_password("MyP@ssw0rd!", &hash).is_ok());
    }

    #[test]
    fn wrong_password_fails() {
        let hash = hash_password("correct");
        assert!(verify_password("wrong", &hash).is_err());
    }

    #[test]
    fn deterministic_with_same_salt() {
        let salt = [1u8; SALT_LEN];
        let h1 = hash_password_with_salt("test", &salt);
        let h2 = hash_password_with_salt("test", &salt);
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_salts_different_hashes() {
        let h1 = hash_password("same");
        let h2 = hash_password("same");
        assert_ne!(h1, h2);
    }

    #[test]
    fn malformed_hash_rejected() {
        assert!(matches!(
            verify_password("x", "garbage"),
            Err(AuthError::InvalidPasswordHash)
        ));
    }

    #[test]
    fn password_strength_weak() {
        let issues = validate_password_strength("abc");
        assert!(issues.contains(&"must be at least 8 characters"));
        assert!(issues.contains(&"must contain an uppercase letter"));
        assert!(issues.contains(&"must contain a digit"));
    }

    #[test]
    fn password_strength_strong() {
        let issues = validate_password_strength("MyStr0ng!Pass");
        assert!(issues.is_empty());
    }
}
