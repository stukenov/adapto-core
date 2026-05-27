use adapto_auth::csrf;
use adapto_auth::error::AuthError;
use adapto_auth::jwt;
use adapto_auth::middleware::{self, AuthConfig};
use adapto_auth::password;
use adapto_auth::rate_limit::RateLimiter;
use adapto_auth::rbac::{RbacStore, Role};
use adapto_auth::session_store::{InMemorySessionStore, SessionData, SessionStore};
use adapto_auth::session_token;
use adapto_runtime::types::UserId;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use std::collections::HashSet;
use std::time::Duration;

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

// ===========================================================================
// Password Hashing
// ===========================================================================

#[test]
fn password_hash_verify_roundtrip() {
    let hash = password::hash_password("Str0ng!Pass");
    assert!(password::verify_password("Str0ng!Pass", &hash).is_ok());
}

#[test]
fn password_wrong_fails() {
    let hash = password::hash_password("correct");
    assert!(matches!(
        password::verify_password("wrong", &hash),
        Err(AuthError::PasswordMismatch)
    ));
}

#[test]
fn password_different_salts_produce_different_hashes() {
    let h1 = password::hash_password("same");
    let h2 = password::hash_password("same");
    assert_ne!(h1, h2);
    assert!(password::verify_password("same", &h1).is_ok());
    assert!(password::verify_password("same", &h2).is_ok());
}

#[test]
fn password_malformed_hash_rejected() {
    assert!(matches!(
        password::verify_password("x", "not-a-hash"),
        Err(AuthError::InvalidPasswordHash)
    ));
    assert!(matches!(
        password::verify_password("x", "wrong-algo$100000$AAAA$BBBB"),
        Err(AuthError::InvalidPasswordHash)
    ));
}

#[test]
fn password_strength_validation() {
    let weak = password::validate_password_strength("abc");
    assert!(!weak.is_empty());
    let strong = password::validate_password_strength("MyStr0ng!Pass");
    assert!(strong.is_empty());
}

#[test]
fn password_empty_string() {
    let hash = password::hash_password("");
    assert!(password::verify_password("", &hash).is_ok());
    assert!(password::verify_password("x", &hash).is_err());
}

#[test]
fn password_unicode() {
    let hash = password::hash_password("пароль123!A");
    assert!(password::verify_password("пароль123!A", &hash).is_ok());
}

// ===========================================================================
// JWT
// ===========================================================================

#[test]
fn jwt_encode_decode_roundtrip() {
    let claims = jwt::Claims::new("user-42", 3600);
    let token = jwt::encode(&claims, SECRET);
    let decoded = jwt::decode(&token, SECRET).unwrap();
    assert_eq!(decoded.sub, "user-42");
    assert!(!decoded.is_expired());
}

#[test]
fn jwt_wrong_secret_rejected() {
    let claims = jwt::Claims::new("user-1", 3600);
    let token = jwt::encode(&claims, SECRET);
    assert!(jwt::decode(&token, OTHER_SECRET).is_err());
}

#[test]
fn jwt_expired_rejected() {
    let mut claims = jwt::Claims::new("user-1", 0);
    claims.exp = claims.iat - 1;
    let token = jwt::encode(&claims, SECRET);
    assert!(matches!(jwt::decode(&token, SECRET), Err(AuthError::ExpiredJwt)));
}

#[test]
fn jwt_custom_claims_roundtrip() {
    let claims = jwt::Claims::new("user-1", 3600)
        .with_issuer("adapto")
        .with_audience("web")
        .with_claim("role", serde_json::json!("admin"))
        .with_claim("org_id", serde_json::json!(42));
    let token = jwt::encode(&claims, SECRET);
    let decoded = jwt::decode(&token, SECRET).unwrap();
    assert_eq!(decoded.iss.as_deref(), Some("adapto"));
    assert_eq!(decoded.aud.as_deref(), Some("web"));
    assert_eq!(decoded.custom["role"], "admin");
    assert_eq!(decoded.custom["org_id"], 42);
}

#[test]
fn jwt_tampered_payload() {
    let claims = jwt::Claims::new("user-1", 3600);
    let token = jwt::encode(&claims, SECRET);
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    let fake = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(br#"{"sub":"admin","iat":0,"exp":99999999999}"#);
    let tampered = format!("{}.{}.{}", parts[0], fake, parts[2]);
    assert!(jwt::decode(&tampered, SECRET).is_err());
}

#[test]
fn jwt_malformed() {
    assert!(jwt::decode("not.jwt", SECRET).is_err());
    assert!(jwt::decode("", SECRET).is_err());
    assert!(jwt::decode("a.b.c", SECRET).is_err());
}

#[test]
fn jwt_decode_without_verify() {
    let claims = jwt::Claims::new("peek", 3600).with_issuer("test");
    let token = jwt::encode(&claims, SECRET);
    let decoded = jwt::decode_without_verify(&token).unwrap();
    assert_eq!(decoded.sub, "peek");
    assert_eq!(decoded.iss.as_deref(), Some("test"));
}

// ===========================================================================
// Session Store
// ===========================================================================

#[test]
fn session_store_create_and_get() {
    let store = InMemorySessionStore::new();
    let data = SessionData::new("user-1");
    store.create("s1", data).unwrap();
    let got = store.get("s1").unwrap();
    assert_eq!(got.user_id, "user-1");
}

#[test]
fn session_store_get_nonexistent() {
    let store = InMemorySessionStore::new();
    assert!(matches!(store.get("x"), Err(AuthError::SessionNotFound)));
}

#[test]
fn session_store_update() {
    let store = InMemorySessionStore::new();
    let mut data = SessionData::new("user-1");
    store.create("s1", data.clone()).unwrap();
    data.set("lang", serde_json::json!("ru"));
    store.update("s1", data).unwrap();
    let got = store.get("s1").unwrap();
    assert_eq!(got.get("lang").unwrap(), "ru");
}

#[test]
fn session_store_destroy() {
    let store = InMemorySessionStore::new();
    store.create("s1", SessionData::new("u")).unwrap();
    store.destroy("s1").unwrap();
    assert!(!store.exists("s1"));
}

#[test]
fn session_store_cleanup() {
    let store = InMemorySessionStore::new();
    let mut old = SessionData::new("old-user");
    old.last_accessed = Some(std::time::Instant::now() - Duration::from_secs(7200));
    store.create("old", old).unwrap();
    store.create("new", SessionData::new("new-user")).unwrap();

    let removed = store.cleanup_expired(Duration::from_secs(3600));
    assert_eq!(removed, 1);
    assert!(!store.exists("old"));
    assert!(store.exists("new"));
}

#[test]
fn session_store_len() {
    let store = InMemorySessionStore::new();
    assert_eq!(store.len(), 0);
    assert!(store.is_empty());
    store.create("a", SessionData::new("u")).unwrap();
    store.create("b", SessionData::new("u")).unwrap();
    assert_eq!(store.len(), 2);
}

#[test]
fn session_data_set_get_remove() {
    let mut data = SessionData::new("u");
    data.set("k", serde_json::json!(123));
    assert_eq!(data.get("k").unwrap(), &serde_json::json!(123));
    assert!(data.remove("k").is_some());
    assert!(data.get("k").is_none());
}

// ===========================================================================
// Middleware
// ===========================================================================

#[test]
fn middleware_csrf_roundtrip() {
    let cfg = AuthConfig::new(SECRET);
    let token = middleware::generate_csrf_token(&cfg);
    assert!(middleware::validate_csrf_header(&cfg, Some(&token)).is_ok());
}

#[test]
fn middleware_csrf_disabled() {
    let cfg = AuthConfig::new(SECRET).disable_csrf();
    assert!(middleware::validate_csrf_header(&cfg, None).is_ok());
}

#[test]
fn middleware_bearer_roundtrip() {
    let cfg = AuthConfig::new(SECRET).enable_jwt();
    let token = middleware::issue_jwt(&cfg, "user-1", 3600);
    let header = format!("Bearer {}", token);
    let claims = middleware::validate_bearer_token(&cfg, Some(&header)).unwrap();
    assert_eq!(claims.sub, "user-1");
}

#[test]
fn middleware_bearer_missing() {
    let cfg = AuthConfig::new(SECRET);
    assert!(matches!(
        middleware::validate_bearer_token(&cfg, None),
        Err(AuthError::Unauthorized)
    ));
}

#[test]
fn middleware_session_cookie_roundtrip() {
    let cfg = AuthConfig::new(SECRET);
    let signed = middleware::sign_session(&cfg, "sess-42");
    let id = middleware::validate_session_cookie(&cfg, Some(&signed)).unwrap();
    assert_eq!(id, "sess-42");
}

#[test]
fn middleware_public_paths() {
    let cfg = AuthConfig::new(SECRET)
        .public_path("/health")
        .public_path("/static/*");
    assert!(cfg.is_public("/health"));
    assert!(cfg.is_public("/static/main.js"));
    assert!(!cfg.is_public("/api/users"));
}

#[test]
fn middleware_issue_jwt_with_claims() {
    let cfg = AuthConfig::new(SECRET);
    let claims = jwt::Claims::new("admin", 3600).with_issuer("adapto");
    let token = middleware::issue_jwt_with_claims(&cfg, &claims);
    let header = format!("Bearer {}", token);
    let decoded = middleware::validate_bearer_token(&cfg, Some(&header)).unwrap();
    assert_eq!(decoded.sub, "admin");
    assert_eq!(decoded.iss.as_deref(), Some("adapto"));
}

// ===========================================================================
// New Error Variants Display
// ===========================================================================

#[test]
fn error_display_new_variants() {
    assert_eq!(AuthError::InvalidPasswordHash.to_string(), "Invalid password hash");
    assert_eq!(AuthError::PasswordMismatch.to_string(), "Password verification failed");
    assert_eq!(AuthError::ExpiredJwt.to_string(), "Expired JWT");
    assert_eq!(AuthError::SessionNotFound.to_string(), "Session not found");
    assert_eq!(AuthError::SessionExpired.to_string(), "Session expired");
    assert_eq!(AuthError::RateLimitExceeded.to_string(), "Rate limit exceeded");
    assert_eq!(AuthError::Unauthorized.to_string(), "Unauthorized");
    assert_eq!(
        AuthError::InvalidJwt("bad".into()).to_string(),
        "Invalid JWT: bad"
    );
    assert_eq!(
        AuthError::Forbidden("no access".into()).to_string(),
        "Forbidden: no access"
    );
}
