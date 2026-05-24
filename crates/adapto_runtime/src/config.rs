use serde::{Deserialize, Serialize};

/// Top-level configuration parsed from `adapto.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptoConfig {
    #[serde(default)]
    pub app: AppConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub database: DatabaseConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub live: LiveConfig,
    #[serde(default)]
    pub tenant: TenantConfig,
    #[serde(default)]
    pub ai: AiConfig,
}

impl Default for AdaptoConfig {
    fn default() -> Self {
        Self {
            app: AppConfig::default(),
            server: ServerConfig::default(),
            database: DatabaseConfig::default(),
            security: SecurityConfig::default(),
            live: LiveConfig::default(),
            tenant: TenantConfig::default(),
            ai: AiConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// AppConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_app_name")]
    pub name: String,
    #[serde(default = "default_env")]
    pub env: String,
}

fn default_app_name() -> String {
    "adapto_app".to_string()
}

fn default_env() -> String {
    "development".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            name: default_app_name(),
            env: default_env(),
        }
    }
}

// ---------------------------------------------------------------------------
// ServerConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

// ---------------------------------------------------------------------------
// DatabaseConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    #[serde(default = "default_db_url")]
    pub url: String,
}

fn default_db_url() -> String {
    "postgres://localhost/adapto_dev".to_string()
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: default_db_url(),
        }
    }
}

// ---------------------------------------------------------------------------
// SecurityConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_true")]
    pub csrf: bool,
    #[serde(default = "default_true")]
    pub secure_cookies: bool,
    #[serde(default = "default_csp")]
    pub content_security_policy: String,
}

fn default_true() -> bool {
    true
}

fn default_csp() -> String {
    "strict".to_string()
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            csrf: true,
            secure_cookies: true,
            content_security_policy: default_csp(),
        }
    }
}

// ---------------------------------------------------------------------------
// LiveConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveConfig {
    #[serde(default = "default_ws_path")]
    pub websocket_path: String,
    #[serde(default = "default_max_sessions")]
    pub max_sessions_per_user: u32,
    #[serde(default = "default_rate_limit")]
    pub event_rate_limit_per_second: u32,
}

fn default_ws_path() -> String {
    "/_adapto/live".to_string()
}

fn default_max_sessions() -> u32 {
    10
}

fn default_rate_limit() -> u32 {
    20
}

impl Default for LiveConfig {
    fn default() -> Self {
        Self {
            websocket_path: default_ws_path(),
            max_sessions_per_user: default_max_sessions(),
            event_rate_limit_per_second: default_rate_limit(),
        }
    }
}

// ---------------------------------------------------------------------------
// TenantConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantConfig {
    #[serde(default = "default_tenant_mode")]
    pub mode: String,
    #[serde(default = "default_tenant_strategy")]
    pub strategy: String,
}

fn default_tenant_mode() -> String {
    "required".to_string()
}

fn default_tenant_strategy() -> String {
    "subdomain".to_string()
}

impl Default for TenantConfig {
    fn default() -> Self {
        Self {
            mode: default_tenant_mode(),
            strategy: default_tenant_strategy(),
        }
    }
}

// ---------------------------------------------------------------------------
// AiConfig
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub fallback_model: Option<String>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            default_model: None,
            fallback_model: None,
        }
    }
}
