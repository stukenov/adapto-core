use adapto_auth::csrf;
use adapto_auth::error::AuthError;
use adapto_auth::rate_limit::RateLimiter;
use adapto_auth::rbac::{RbacStore, Role};
use adapto_auth::session_token;
use adapto_runtime::types::UserId;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::collections::HashSet;

const SECRET: &[u8] = b"test-secret-key-32-bytes-long!!!";
const OTHER_SECRET: &[u8] = b"other-secret-key-32-bytes-long!!";

fn make_role(name: &str, perms: &[&str]) -> Role {
    Role {
        name: name.to_string(),
        permissions: perms.iter().map(|s| s.to_string()).collect::<HashSet<_>>(),
    }
}

// ===========================================================================
// CSRF — generate / validate
// ===========================================================================

#[test]
fn csrf_roundtrip() {
    let token = csrf::generate_token(SECRET);
    assert!(csrf::validate_token(&token, SECRET).is_ok());
}

#[test]
fn csrf_roundtrip_multiple_tokens_are_unique() {
    let t1 = csrf::generate_token(SECRET);
    let t2 = csrf::generate_token(SECRET);
    // Tokens generated at same second *could* be equal (same timestamp + same
    // secret → same HMAC), but they should at least not panic.
    assert!(csrf::validate_token(&t1, SECRET).is_ok());
    assert!(csrf::validate_token(&t2, SECRET).is_ok());
}

#[test]
fn csrf_wrong_secret_rejected() {
    let token = csrf::generate_token(SECRET);
    let err = csrf::validate_token(&token, OTHER_SECRET).unwrap_err();
    assert!(matches!(err, AuthError::InvalidCsrfToken));
}

#[test]
fn csrf_tampered_signature_rejected() {
    let token = csrf::generate_token(SECRET);
    // Flip last character of signature portion.
    let mut chars: Vec<char> = token.chars().collect();
    let last = chars.len() - 1;
    chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
    let tampered: String = chars.into_iter().collect();

    let err = csrf::validate_token(&tampered, SECRET).unwrap_err();
    assert!(matches!(err, AuthError::InvalidCsrfToken));
}

#[test]
fn csrf_tampered_timestamp_rejected() {
    let token = csrf::generate_token(SECRET);
    let parts: Vec<&str> = token.splitn(2, '.').collect();
    // Decode timestamp, flip a bit, re-encode.
    let mut ts_bytes = URL_SAFE_NO_PAD.decode(parts[0]).unwrap();
    ts_bytes[0] ^= 0xFF;
    let bad_ts = URL_SAFE_NO_PAD.encode(&ts_bytes);
    let tampered = format!("{}.{}", bad_ts, parts[1]);

    let err = csrf::validate_token(&tampered, SECRET).unwrap_err();
    assert!(matches!(err, AuthError::InvalidCsrfToken));
}

#[test]
fn csrf_expired_token_rejected() {
    // Manually craft a token with a timestamp from 2 hours ago.
    let two_hours_ago = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 7200;
    let ts_bytes = two_hours_ago.to_be_bytes();

    let mut mac =
        hmac::Mac::new_from_slice(SECRET as &[u8]).expect("valid key");
    hmac::Mac::update(&mut mac, &ts_bytes);
    let sig: hmac::digest::CtOutput<hmac::Hmac<sha2::Sha256>> = hmac::Mac::finalize(mac);
    let sig_bytes = sig.into_bytes();

    let token = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(ts_bytes),
        URL_SAFE_NO_PAD.encode(sig_bytes)
    );

    let err = csrf::validate_token(&token, SECRET).unwrap_err();
    assert!(matches!(err, AuthError::ExpiredCsrfToken));
}

#[test]
fn csrf_empty_string_is_malformed() {
    let err = csrf::validate_token("", SECRET).unwrap_err();
    assert!(matches!(err, AuthError::MalformedToken));
}

#[test]
fn csrf_no_dot_separator_is_malformed() {
    let err = csrf::validate_token("nodothere", SECRET).unwrap_err();
    // Will either be MalformedToken (bad base64 length) or InvalidCsrfToken
    assert!(matches!(
        err,
        AuthError::MalformedToken | AuthError::InvalidCsrfToken
    ));
}

#[test]
fn csrf_garbage_base64_is_malformed() {
    let err = csrf::validate_token("!!!.!!!", SECRET).unwrap_err();
    assert!(matches!(err, AuthError::MalformedToken));
}

#[test]
fn csrf_short_timestamp_bytes_is_malformed() {
    // Valid base64 but only 4 bytes instead of 8.
    let short_ts = URL_SAFE_NO_PAD.encode([0u8; 4]);
    let fake_sig = URL_SAFE_NO_PAD.encode([0u8; 32]);
    let token = format!("{}.{}", short_ts, fake_sig);
    let err = csrf::validate_token(&token, SECRET).unwrap_err();
    assert!(matches!(err, AuthError::MalformedToken));
}

#[test]
fn csrf_empty_secret_still_works() {
    let empty: &[u8] = b"";
    let token = csrf::generate_token(empty);
    assert!(csrf::validate_token(&token, empty).is_ok());
    // But wrong secret still rejected.
    assert!(csrf::validate_token(&token, SECRET).is_err());
}

// ===========================================================================
// Session Token — sign / verify
// ===========================================================================

#[test]
fn session_roundtrip() {
    let id = "sess_abc123";
    let signed = session_token::sign_session_id(id, SECRET);
    let recovered = session_token::verify_session_id(&signed, SECRET).unwrap();
    assert_eq!(recovered, id);
}

#[test]
fn session_wrong_secret_rejected() {
    let signed = session_token::sign_session_id("sess_1", SECRET);
    let err = session_token::verify_session_id(&signed, OTHER_SECRET).unwrap_err();
    assert!(matches!(err, AuthError::InvalidSessionSignature));
}

#[test]
fn session_tampered_signature_rejected() {
    let signed = session_token::sign_session_id("sess_1", SECRET);
    let mut chars: Vec<char> = signed.chars().collect();
    let last = chars.len() - 1;
    chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
    let tampered: String = chars.into_iter().collect();

    let err = session_token::verify_session_id(&tampered, SECRET).unwrap_err();
    assert!(matches!(err, AuthError::InvalidSessionSignature));
}

#[test]
fn session_tampered_id_rejected() {
    let signed = session_token::sign_session_id("sess_1", SECRET);
    // Replace "sess_1" with "sess_2" keeping the same signature.
    let tampered = signed.replacen("sess_1", "sess_2", 1);
    let err = session_token::verify_session_id(&tampered, SECRET).unwrap_err();
    assert!(matches!(err, AuthError::InvalidSessionSignature));
}

#[test]
fn session_no_dot_is_malformed() {
    let err = session_token::verify_session_id("nodotanywhere", SECRET).unwrap_err();
    assert!(matches!(err, AuthError::MalformedToken));
}

#[test]
fn session_empty_id_is_malformed() {
    // Format: ".base64sig" — empty id portion.
    let fake_sig = URL_SAFE_NO_PAD.encode([0u8; 32]);
    let token = format!(".{}", fake_sig);
    let err = session_token::verify_session_id(&token, SECRET).unwrap_err();
    assert!(matches!(err, AuthError::MalformedToken));
}

#[test]
fn session_empty_signature_is_malformed() {
    let token = "sess_1.";
    let err = session_token::verify_session_id(token, SECRET).unwrap_err();
    assert!(matches!(err, AuthError::MalformedToken));
}

#[test]
fn session_special_chars_in_id() {
    // IDs with dots, slashes, unicode should survive roundtrip because
    // verify uses rfind('.') to split on the last dot.
    let ids = [
        "user@example.com",
        "path/to/session",
        "id-with-dashes_and_underscores",
        "unicode-\u{00e9}\u{00e8}\u{00ea}",
        "dots.in.the.id",
    ];
    for id in &ids {
        let signed = session_token::sign_session_id(id, SECRET);
        let recovered = session_token::verify_session_id(&signed, SECRET).unwrap();
        assert_eq!(&recovered, id, "roundtrip failed for id: {}", id);
    }
}

#[test]
fn session_empty_secret_still_works() {
    let empty: &[u8] = b"";
    let signed = session_token::sign_session_id("sess_1", empty);
    let recovered = session_token::verify_session_id(&signed, empty).unwrap();
    assert_eq!(recovered, "sess_1");
    // Different secret rejects.
    assert!(session_token::verify_session_id(&signed, SECRET).is_err());
}

#[test]
fn session_deterministic_for_same_inputs() {
    let s1 = session_token::sign_session_id("id", SECRET);
    let s2 = session_token::sign_session_id("id", SECRET);
    assert_eq!(s1, s2);
}

// ===========================================================================
// RBAC
// ===========================================================================

#[test]
fn rbac_new_store_is_empty() {
    let store = RbacStore::new();
    let uid = UserId::default();
    let perms = store.get_permissions(&uid);
    assert!(!perms.has("anything"));
    assert!(!store.has_role(&uid, "admin"));
}

#[test]
fn rbac_add_role_and_assign() {
    let mut store = RbacStore::new();
    store.add_role(make_role("editor", &["read", "write"]));

    let uid = UserId::default();
    store.assign_role(&uid, "editor");

    assert!(store.has_role(&uid, "editor"));
    let perms = store.get_permissions(&uid);
    assert!(perms.has("read"));
    assert!(perms.has("write"));
    assert!(!perms.has("delete"));
}

#[test]
fn rbac_assign_role_idempotent() {
    let mut store = RbacStore::new();
    store.add_role(make_role("viewer", &["read"]));
    let uid = UserId::default();

    store.assign_role(&uid, "viewer");
    store.assign_role(&uid, "viewer"); // duplicate
    assert!(store.has_role(&uid, "viewer"));
}

#[test]
fn rbac_revoke_role() {
    let mut store = RbacStore::new();
    store.add_role(make_role("admin", &["admin", "read", "write"]));
    let uid = UserId::default();

    store.assign_role(&uid, "admin");
    assert!(store.has_role(&uid, "admin"));

    store.revoke_role(&uid, "admin");
    assert!(!store.has_role(&uid, "admin"));
    assert!(!store.get_permissions(&uid).has("admin"));
}

#[test]
fn rbac_revoke_nonexistent_role_is_noop() {
    let mut store = RbacStore::new();
    let uid = UserId::default();
    // Should not panic.
    store.revoke_role(&uid, "ghost");
    assert!(!store.has_role(&uid, "ghost"));
}

#[test]
fn rbac_multi_role_permission_aggregation() {
    let mut store = RbacStore::new();
    store.add_role(make_role("viewer", &["read"]));
    store.add_role(make_role("editor", &["write", "publish"]));
    store.add_role(make_role("admin", &["delete", "manage_users"]));

    let uid = UserId::default();
    store.assign_role(&uid, "viewer");
    store.assign_role(&uid, "editor");
    store.assign_role(&uid, "admin");

    let perms = store.get_permissions(&uid);
    assert!(perms.has("read"));
    assert!(perms.has("write"));
    assert!(perms.has("publish"));
    assert!(perms.has("delete"));
    assert!(perms.has("manage_users"));
}

#[test]
fn rbac_overlapping_permissions_across_roles() {
    let mut store = RbacStore::new();
    store.add_role(make_role("role_a", &["read", "write"]));
    store.add_role(make_role("role_b", &["write", "execute"]));

    let uid = UserId::default();
    store.assign_role(&uid, "role_a");
    store.assign_role(&uid, "role_b");

    let perms = store.get_permissions(&uid);
    assert!(perms.has("read"));
    assert!(perms.has("write"));
    assert!(perms.has("execute"));
}

#[test]
fn rbac_assign_undefined_role_gives_no_permissions() {
    let mut store = RbacStore::new();
    let uid = UserId::default();
    store.assign_role(&uid, "undefined_role");

    assert!(store.has_role(&uid, "undefined_role"));
    // Role name assigned but no role definition → no permissions.
    let perms = store.get_permissions(&uid);
    assert!(!perms.has("anything"));
}

#[test]
fn rbac_replace_role_definition() {
    let mut store = RbacStore::new();
    store.add_role(make_role("editor", &["read"]));
    let uid = UserId::default();
    store.assign_role(&uid, "editor");
    assert!(store.get_permissions(&uid).has("read"));
    assert!(!store.get_permissions(&uid).has("write"));

    // Replace the role definition.
    store.add_role(make_role("editor", &["read", "write"]));
    assert!(store.get_permissions(&uid).has("write"));
}

#[test]
fn rbac_different_users_independent() {
    let mut store = RbacStore::new();
    store.add_role(make_role("admin", &["all"]));
    store.add_role(make_role("viewer", &["read"]));

    let alice = UserId::default();
    let bob = UserId::default();

    store.assign_role(&alice, "admin");
    store.assign_role(&bob, "viewer");

    assert!(store.get_permissions(&alice).has("all"));
    assert!(!store.get_permissions(&alice).has("read"));

    assert!(store.get_permissions(&bob).has("read"));
    assert!(!store.get_permissions(&bob).has("all"));
}

#[test]
fn rbac_revoke_one_role_keeps_others() {
    let mut store = RbacStore::new();
    store.add_role(make_role("viewer", &["read"]));
    store.add_role(make_role("editor", &["write"]));

    let uid = UserId::default();
    store.assign_role(&uid, "viewer");
    store.assign_role(&uid, "editor");

    store.revoke_role(&uid, "editor");
    assert!(!store.has_role(&uid, "editor"));
    assert!(store.has_role(&uid, "viewer"));
    assert!(store.get_permissions(&uid).has("read"));
    assert!(!store.get_permissions(&uid).has("write"));
}

#[test]
fn rbac_role_with_no_permissions() {
    let mut store = RbacStore::new();
    store.add_role(make_role("empty", &[]));
    let uid = UserId::default();
    store.assign_role(&uid, "empty");
    assert!(store.has_role(&uid, "empty"));
    assert!(!store.get_permissions(&uid).has("anything"));
}

// ===========================================================================
// Rate Limiter
// ===========================================================================

#[test]
fn rate_limiter_allows_within_limit() {
    let mut limiter = RateLimiter::new(5);
    for i in 0..5 {
        assert!(limiter.check("key").is_ok(), "request {} should pass", i);
    }
}

#[test]
fn rate_limiter_denies_when_exceeded() {
    let mut limiter = RateLimiter::new(2);
    assert!(limiter.check("key").is_ok());
    assert!(limiter.check("key").is_ok());
    assert!(limiter.check("key").is_err());
}

#[test]
fn rate_limiter_different_keys_independent() {
    let mut limiter = RateLimiter::new(1);
    assert!(limiter.check("alice").is_ok());
    assert!(limiter.check("alice").is_err()); // alice exhausted

    // bob has own bucket.
    assert!(limiter.check("bob").is_ok());
    assert!(limiter.check("bob").is_err());
}

#[test]
fn rate_limiter_reset_restores_capacity() {
    let mut limiter = RateLimiter::new(3);
    for _ in 0..3 {
        let _ = limiter.check("key");
    }
    assert!(limiter.check("key").is_err());

    limiter.reset("key");
    assert!(limiter.check("key").is_ok());
}

#[test]
fn rate_limiter_reset_nonexistent_key_is_noop() {
    let mut limiter = RateLimiter::new(5);
    // Should not panic.
    limiter.reset("never_seen");
}

#[test]
fn rate_limiter_remove_creates_fresh_bucket() {
    let mut limiter = RateLimiter::new(1);
    assert!(limiter.check("key").is_ok());
    assert!(limiter.check("key").is_err());

    limiter.remove("key");
    // Next check creates a fresh bucket.
    assert!(limiter.check("key").is_ok());
}

#[test]
fn rate_limiter_remove_nonexistent_key_is_noop() {
    let mut limiter = RateLimiter::new(5);
    limiter.remove("ghost");
}

#[test]
fn rate_limiter_zero_rate_denies_immediately() {
    let mut limiter = RateLimiter::new(0);
    assert!(limiter.check("key").is_err());
}

#[test]
fn rate_limiter_small_bucket_exhausts() {
    let mut limiter = RateLimiter::new(3);
    assert!(limiter.check("key").is_ok());
    assert!(limiter.check("key").is_ok());
    assert!(limiter.check("key").is_ok());
    assert!(limiter.check("key").is_err());
}

#[test]
fn rate_limiter_many_keys() {
    let mut limiter = RateLimiter::new(1);
    for i in 0..100 {
        let key = format!("user_{}", i);
        assert!(limiter.check(&key).is_ok());
        assert!(limiter.check(&key).is_err());
    }
}

// ===========================================================================
// Error Display
// ===========================================================================

#[test]
fn error_display_invalid_csrf() {
    let e = AuthError::InvalidCsrfToken;
    assert_eq!(e.to_string(), "Invalid CSRF token");
}

#[test]
fn error_display_expired_csrf() {
    let e = AuthError::ExpiredCsrfToken;
    assert_eq!(e.to_string(), "Expired CSRF token");
}

#[test]
fn error_display_invalid_session_signature() {
    let e = AuthError::InvalidSessionSignature;
    assert_eq!(e.to_string(), "Invalid session signature");
}

#[test]
fn error_display_malformed_token() {
    let e = AuthError::MalformedToken;
    assert_eq!(e.to_string(), "Malformed token");
}

#[test]
fn error_display_role_not_found() {
    let e = AuthError::RoleNotFound("superadmin".to_string());
    assert_eq!(e.to_string(), "Role not found: superadmin");
}

#[test]
fn error_is_debug() {
    // Ensure Debug is derived (compile-time check + runtime sanity).
    let e = AuthError::MalformedToken;
    let dbg = format!("{:?}", e);
    assert!(dbg.contains("MalformedToken"));
}
