use crate::csrf;
use crate::error::AuthError;
use crate::jwt;
use crate::session_token;

pub struct AuthConfig {
    secret: Vec<u8>,
    csrf_enabled: bool,
    jwt_enabled: bool,
    session_enabled: bool,
    public_paths: Vec<String>,
}

impl AuthConfig {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            secret: secret.to_vec(),
            csrf_enabled: true,
            jwt_enabled: false,
            session_enabled: true,
            public_paths: vec![],
        }
    }

    pub fn enable_jwt(mut self) -> Self {
        self.jwt_enabled = true;
        self
    }

    pub fn disable_csrf(mut self) -> Self {
        self.csrf_enabled = false;
        self
    }

    pub fn disable_sessions(mut self) -> Self {
        self.session_enabled = false;
        self
    }

    pub fn public_path(mut self, path: &str) -> Self {
        self.public_paths.push(path.into());
        self
    }

    pub fn is_public(&self, path: &str) -> bool {
        self.public_paths.iter().any(|p| {
            if p.ends_with('*') {
                path.starts_with(&p[..p.len() - 1])
            } else {
                path == p
            }
        })
    }

    pub fn secret(&self) -> &[u8] {
        &self.secret
    }

    pub fn csrf_enabled(&self) -> bool {
        self.csrf_enabled
    }

    pub fn jwt_enabled(&self) -> bool {
        self.jwt_enabled
    }

    pub fn session_enabled(&self) -> bool {
        self.session_enabled
    }
}

pub fn validate_csrf_header(config: &AuthConfig, token: Option<&str>) -> Result<(), AuthError> {
    if !config.csrf_enabled {
        return Ok(());
    }
    match token {
        Some(t) => csrf::validate_token(t, &config.secret),
        None => Err(AuthError::InvalidCsrfToken),
    }
}

pub fn validate_bearer_token(config: &AuthConfig, auth_header: Option<&str>) -> Result<jwt::Claims, AuthError> {
    let header = auth_header.ok_or(AuthError::Unauthorized)?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::Unauthorized)?;
    jwt::decode(token, &config.secret)
}

pub fn validate_session_cookie(config: &AuthConfig, cookie_value: Option<&str>) -> Result<String, AuthError> {
    let signed = cookie_value.ok_or(AuthError::SessionNotFound)?;
    session_token::verify_session_id(signed, &config.secret)
}

pub fn generate_csrf_token(config: &AuthConfig) -> String {
    csrf::generate_token(&config.secret)
}

pub fn sign_session(config: &AuthConfig, session_id: &str) -> String {
    session_token::sign_session_id(session_id, &config.secret)
}

pub fn issue_jwt(config: &AuthConfig, subject: &str, ttl_secs: u64) -> String {
    let claims = jwt::Claims::new(subject, ttl_secs);
    jwt::encode(&claims, &config.secret)
}

pub fn issue_jwt_with_claims(config: &AuthConfig, claims: &jwt::Claims) -> String {
    jwt::encode(claims, &config.secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"middleware-test-secret-key-32b!!!";

    fn config() -> AuthConfig {
        AuthConfig::new(SECRET)
    }

    #[test]
    fn csrf_validation_roundtrip() {
        let cfg = config();
        let token = generate_csrf_token(&cfg);
        assert!(validate_csrf_header(&cfg, Some(&token)).is_ok());
    }

    #[test]
    fn csrf_disabled_skips() {
        let cfg = config().disable_csrf();
        assert!(validate_csrf_header(&cfg, None).is_ok());
    }

    #[test]
    fn csrf_missing_fails() {
        let cfg = config();
        assert!(validate_csrf_header(&cfg, None).is_err());
    }

    #[test]
    fn bearer_token_roundtrip() {
        let cfg = config().enable_jwt();
        let token = issue_jwt(&cfg, "user-1", 3600);
        let header = format!("Bearer {}", token);
        let claims = validate_bearer_token(&cfg, Some(&header)).unwrap();
        assert_eq!(claims.sub, "user-1");
    }

    #[test]
    fn bearer_missing_fails() {
        let cfg = config().enable_jwt();
        assert!(matches!(
            validate_bearer_token(&cfg, None),
            Err(AuthError::Unauthorized)
        ));
    }

    #[test]
    fn bearer_no_prefix_fails() {
        let cfg = config().enable_jwt();
        assert!(matches!(
            validate_bearer_token(&cfg, Some("not-bearer")),
            Err(AuthError::Unauthorized)
        ));
    }

    #[test]
    fn session_cookie_roundtrip() {
        let cfg = config();
        let signed = sign_session(&cfg, "sess-abc");
        let id = validate_session_cookie(&cfg, Some(&signed)).unwrap();
        assert_eq!(id, "sess-abc");
    }

    #[test]
    fn session_cookie_missing_fails() {
        let cfg = config();
        assert!(validate_session_cookie(&cfg, None).is_err());
    }

    #[test]
    fn public_paths() {
        let cfg = config()
            .public_path("/health")
            .public_path("/static/*")
            .public_path("/api/public");

        assert!(cfg.is_public("/health"));
        assert!(cfg.is_public("/static/main.js"));
        assert!(cfg.is_public("/static/css/style.css"));
        assert!(cfg.is_public("/api/public"));
        assert!(!cfg.is_public("/api/private"));
        assert!(!cfg.is_public("/dashboard"));
    }

    #[test]
    fn issue_jwt_with_custom_claims() {
        let cfg = config().enable_jwt();
        let claims = jwt::Claims::new("user-1", 3600)
            .with_issuer("adapto")
            .with_claim("role", serde_json::json!("admin"));
        let token = issue_jwt_with_claims(&cfg, &claims);
        let decoded = validate_bearer_token(&cfg, Some(&format!("Bearer {}", token))).unwrap();
        assert_eq!(decoded.iss.as_deref(), Some("adapto"));
    }
}
