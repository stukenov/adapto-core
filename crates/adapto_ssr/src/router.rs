use adapto_compiler::manifest::RouteManifest;
use std::collections::HashMap;

/// Route matcher that resolves request paths against the compiled
/// route manifest.
///
/// Supports exact paths (`/customers`), dynamic segments
/// (`/customers/:id`), and nested dynamic parameters
/// (`/customers/:id/orders/:order_id`).
pub struct Router {
    manifest: RouteManifest,
}

/// Result of successfully matching a request path to a route.
#[derive(Debug, Clone)]
pub struct RouteMatch {
    /// Route identifier from the manifest.
    pub route_id: String,
    /// The matched path pattern.
    pub path: String,
    /// Extracted dynamic parameters (e.g., `"id" -> "123"`).
    pub params: HashMap<String, String>,
    /// Authentication level required by this route.
    pub auth: String,
    /// Tenant requirement for this route.
    pub tenant: String,
    /// Permission required to access this route, if any.
    pub permission: Option<String>,
    /// Layout to wrap this page in, if any.
    pub layout: Option<String>,
}

impl Router {
    pub fn new(manifest: RouteManifest) -> Self {
        Self { manifest }
    }

    /// Match a request path against the route manifest.
    ///
    /// Tries each route in declaration order. Exact matches take priority
    /// implicitly — if you declare `/customers` before `/customers/:id`,
    /// an exact hit on `/customers` wins. Dynamic segments are extracted
    /// into `RouteMatch::params`.
    pub fn match_route(&self, path: &str) -> Option<RouteMatch> {
        let normalized = normalize_path(path);

        for entry in &self.manifest.routes {
            if let Some(params) = Self::extract_params(&entry.path, &normalized) {
                return Some(RouteMatch {
                    route_id: entry.id.clone(),
                    path: entry.path.clone(),
                    params,
                    auth: entry.auth.clone(),
                    tenant: entry.tenant.clone(),
                    permission: entry.permission.clone(),
                    layout: entry.layout.clone(),
                });
            }
        }

        None
    }

    /// Extract path parameters by comparing a route pattern against a
    /// concrete path.
    ///
    /// Returns `Some(params)` if every segment matches (static segments
    /// must be equal, dynamic `:name` segments capture their value).
    /// Returns `None` on mismatch.
    fn extract_params(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
        let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if pattern_segments.len() != path_segments.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (pat, val) in pattern_segments.iter().zip(path_segments.iter()) {
            if let Some(param_name) = pat.strip_prefix(':') {
                params.insert(param_name.to_string(), val.to_string());
            } else if pat != val {
                return None;
            }
        }

        Some(params)
    }
}

/// Strip trailing slashes and ensure a leading slash for consistent
/// matching.
fn normalize_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{}", trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manifest() -> RouteManifest {
        use adapto_compiler::manifest::RouteEntry;

        RouteManifest {
            routes: vec![
                RouteEntry {
                    id: "customers_list".into(),
                    path: "/customers".into(),
                    file: "customers/list.adapto".into(),
                    method: "GET".into(),
                    auth: "required".into(),
                    tenant: "required".into(),
                    permission: None,
                    layout: Some("main".into()),
                    cache: "no-store".into(),
                },
                RouteEntry {
                    id: "customer_detail".into(),
                    path: "/customers/:id".into(),
                    file: "customers/detail.adapto".into(),
                    method: "GET".into(),
                    auth: "required".into(),
                    tenant: "required".into(),
                    permission: Some("customers.read".into()),
                    layout: Some("main".into()),
                    cache: "no-store".into(),
                },
                RouteEntry {
                    id: "customer_orders".into(),
                    path: "/customers/:id/orders/:order_id".into(),
                    file: "customers/orders.adapto".into(),
                    method: "GET".into(),
                    auth: "required".into(),
                    tenant: "required".into(),
                    permission: Some("orders.read".into()),
                    layout: Some("main".into()),
                    cache: "no-store".into(),
                },
                RouteEntry {
                    id: "home".into(),
                    path: "/".into(),
                    file: "home.adapto".into(),
                    method: "GET".into(),
                    auth: "public".into(),
                    tenant: "none".into(),
                    permission: None,
                    layout: None,
                    cache: "public".into(),
                },
            ],
        }
    }

    #[test]
    fn exact_path_match() {
        let router = Router::new(test_manifest());
        let m = router.match_route("/customers").unwrap();
        assert_eq!(m.route_id, "customers_list");
        assert!(m.params.is_empty());
    }

    #[test]
    fn dynamic_segment_match() {
        let router = Router::new(test_manifest());
        let m = router.match_route("/customers/123").unwrap();
        assert_eq!(m.route_id, "customer_detail");
        assert_eq!(m.params.get("id").unwrap(), "123");
    }

    #[test]
    fn nested_dynamic_segments() {
        let router = Router::new(test_manifest());
        let m = router.match_route("/customers/42/orders/99").unwrap();
        assert_eq!(m.route_id, "customer_orders");
        assert_eq!(m.params.get("id").unwrap(), "42");
        assert_eq!(m.params.get("order_id").unwrap(), "99");
    }

    #[test]
    fn no_match_returns_none() {
        let router = Router::new(test_manifest());
        assert!(router.match_route("/nonexistent").is_none());
    }

    #[test]
    fn extract_params_exact() {
        let params = Router::extract_params("/customers", "/customers").unwrap();
        assert!(params.is_empty());
    }

    #[test]
    fn extract_params_dynamic() {
        let params = Router::extract_params("/users/:id", "/users/abc").unwrap();
        assert_eq!(params.get("id").unwrap(), "abc");
    }

    #[test]
    fn extract_params_mismatch_length() {
        assert!(Router::extract_params("/a/b", "/a").is_none());
    }

    #[test]
    fn extract_params_mismatch_static() {
        assert!(Router::extract_params("/a/b", "/a/c").is_none());
    }

    #[test]
    fn root_path_match() {
        let router = Router::new(test_manifest());
        let m = router.match_route("/").unwrap();
        assert_eq!(m.route_id, "home");
        assert_eq!(m.auth, "public");
    }

    #[test]
    fn trailing_slash_normalized() {
        let router = Router::new(test_manifest());
        let m = router.match_route("/customers/").unwrap();
        assert_eq!(m.route_id, "customers_list");
    }
}
