use adapto_auth::csrf;
use adapto_auth::rate_limit::RateLimiter;
use adapto_auth::rbac::{RbacStore, Role};
use adapto_auth::session_token;
use adapto_runtime::types::UserId;
use std::collections::HashSet;

const SECRET: &[u8] = b"test-secret-key-32-bytes-long!!!";

// ---------------------------------------------------------------------------
// CSRF
// ---------------------------------------------------------------------------

#[test]
fn csrf_generate_and_validate() {
    let token = csrf::generate_token(SECRET);
    assert!(csrf::validate_token(&token, SECRET).is_ok());
}

#[test]
fn csrf_reject_tampered_token() {
    let token = csrf::generate_token(SECRET);
    let tampered = format!("{}x", token);
    let result = csrf::validate_token(&tampered, SECRET);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Session Token
// ---------------------------------------------------------------------------

#[test]
fn session_sign_and_verify() {
    let session_id = "sess_abc123";
    let signed = session_token::sign_session_id(session_id, SECRET);
    let recovered = session_token::verify_session_id(&signed, SECRET).unwrap();
    assert_eq!(recovered, session_id);
}

#[test]
fn session_reject_tampered_signature() {
    let signed = session_token::sign_session_id("sess_abc123", SECRET);
    // Flip the last character of the signature portion.
    let mut chars: Vec<char> = signed.chars().collect();
    let last = chars.len() - 1;
    chars[last] = if chars[last] == 'A' { 'B' } else { 'A' };
    let tampered: String = chars.into_iter().collect();

    let result = session_token::verify_session_id(&tampered, SECRET);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// RBAC
// ---------------------------------------------------------------------------

fn make_role(name: &str, perms: &[&str]) -> Role {
    Role {
        name: name.to_string(),
        permissions: perms.iter().map(|s| s.to_string()).collect::<HashSet<_>>(),
    }
}

#[test]
fn rbac_add_role_with_permissions() {
    let mut store = RbacStore::new();
    store.add_role(make_role("editor", &["read", "write"]));

    let uid = UserId::default();
    store.assign_role(&uid, "editor");

    let perms = store.get_permissions(&uid);
    assert!(perms.has("read"));
    assert!(perms.has("write"));
}

#[test]
fn rbac_assign_role_to_user() {
    let mut store = RbacStore::new();
    store.add_role(make_role("viewer", &["read"]));

    let uid = UserId::default();
    assert!(!store.has_role(&uid, "viewer"));

    store.assign_role(&uid, "viewer");
    assert!(store.has_role(&uid, "viewer"));
}

#[test]
fn rbac_get_permissions_aggregates_roles() {
    let mut store = RbacStore::new();
    store.add_role(make_role("viewer", &["read"]));
    store.add_role(make_role("editor", &["write"]));

    let uid = UserId::default();
    store.assign_role(&uid, "viewer");
    store.assign_role(&uid, "editor");

    let perms = store.get_permissions(&uid);
    assert!(perms.has("read"));
    assert!(perms.has("write"));
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

    let perms = store.get_permissions(&uid);
    assert!(!perms.has("admin"));
}

// ---------------------------------------------------------------------------
// RateLimiter
// ---------------------------------------------------------------------------

#[test]
fn rate_limiter_allow_within_limit() {
    let mut limiter = RateLimiter::new(5);
    for _ in 0..5 {
        assert!(limiter.check("sess_1").is_ok());
    }
}

#[test]
fn rate_limiter_deny_when_exceeded() {
    let mut limiter = RateLimiter::new(2);
    assert!(limiter.check("sess_1").is_ok());
    assert!(limiter.check("sess_1").is_ok());
    // Third request should be denied.
    assert!(limiter.check("sess_1").is_err());
}

#[test]
fn rate_limiter_refill_over_time() {
    let mut limiter = RateLimiter::new(10);

    // Exhaust all tokens.
    for _ in 0..10 {
        let _ = limiter.check("sess_1");
    }
    assert!(limiter.check("sess_1").is_err());

    // After a reset (simulating time passage), tokens should be available.
    limiter.reset("sess_1");
    assert!(limiter.check("sess_1").is_ok());
}

#[test]
fn rate_limiter_remove_session() {
    let mut limiter = RateLimiter::new(1);
    assert!(limiter.check("sess_1").is_ok());
    assert!(limiter.check("sess_1").is_err());

    // Remove and re-check — a fresh bucket is created.
    limiter.remove("sess_1");
    assert!(limiter.check("sess_1").is_ok());
}
