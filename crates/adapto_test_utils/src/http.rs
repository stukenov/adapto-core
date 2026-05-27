use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TestRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub query_params: Vec<(String, String)>,
}

impl TestRequest {
    pub fn get(path: &str) -> Self {
        Self {
            method: "GET".into(),
            path: path.into(),
            headers: HashMap::new(),
            body: None,
            query_params: Vec::new(),
        }
    }

    pub fn post(path: &str) -> Self {
        Self {
            method: "POST".into(),
            path: path.into(),
            headers: HashMap::new(),
            body: None,
            query_params: Vec::new(),
        }
    }

    pub fn put(path: &str) -> Self {
        Self {
            method: "PUT".into(),
            path: path.into(),
            headers: HashMap::new(),
            body: None,
            query_params: Vec::new(),
        }
    }

    pub fn delete(path: &str) -> Self {
        Self {
            method: "DELETE".into(),
            path: path.into(),
            headers: HashMap::new(),
            body: None,
            query_params: Vec::new(),
        }
    }

    pub fn patch(path: &str) -> Self {
        Self {
            method: "PATCH".into(),
            path: path.into(),
            headers: HashMap::new(),
            body: None,
            query_params: Vec::new(),
        }
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }

    pub fn content_type(self, ct: &str) -> Self {
        self.header("content-type", ct)
    }

    pub fn authorization(self, value: &str) -> Self {
        self.header("authorization", value)
    }

    pub fn bearer(self, token: &str) -> Self {
        self.authorization(&format!("Bearer {}", token))
    }

    pub fn cookie(self, value: &str) -> Self {
        self.header("cookie", value)
    }

    pub fn json_body(mut self, value: &Value) -> Self {
        self.body = Some(serde_json::to_vec(value).unwrap());
        self.headers
            .insert("content-type".into(), "application/json".into());
        self
    }

    pub fn text_body(mut self, text: &str) -> Self {
        self.body = Some(text.as_bytes().to_vec());
        self
    }

    pub fn raw_body(mut self, bytes: Vec<u8>) -> Self {
        self.body = Some(bytes);
        self
    }

    pub fn query(mut self, key: &str, value: &str) -> Self {
        self.query_params
            .push((key.to_string(), value.to_string()));
        self
    }

    pub fn full_path(&self) -> String {
        if self.query_params.is_empty() {
            self.path.clone()
        } else {
            let qs: Vec<String> = self
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            format!("{}?{}", self.path, qs.join("&"))
        }
    }

    pub fn body_str(&self) -> Option<&str> {
        self.body
            .as_ref()
            .and_then(|b| std::str::from_utf8(b).ok())
    }

    pub fn body_json(&self) -> Option<Value> {
        self.body
            .as_ref()
            .and_then(|b| serde_json::from_slice(b).ok())
    }
}

#[derive(Debug, Clone)]
pub struct TestResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl TestResponse {
    pub fn ok(body: &str) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: body.as_bytes().to_vec(),
        }
    }

    pub fn json(status: u16, value: &Value) -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type".into(), "application/json".into());
        Self {
            status,
            headers,
            body: serde_json::to_vec(value).unwrap(),
        }
    }

    pub fn not_found() -> Self {
        Self {
            status: 404,
            headers: HashMap::new(),
            body: b"Not Found".to_vec(),
        }
    }

    pub fn redirect(location: &str) -> Self {
        let mut headers = HashMap::new();
        headers.insert("location".into(), location.into());
        Self {
            status: 302,
            headers,
            body: Vec::new(),
        }
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    pub fn json_body(&self) -> Option<Value> {
        serde_json::from_slice(&self.body).ok()
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status)
    }

    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.status)
    }

    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.status)
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }
}

pub fn assert_status(resp: &TestResponse, expected: u16) {
    assert_eq!(
        resp.status, expected,
        "Expected status {}, got {}. Body: {}",
        expected,
        resp.status,
        resp.text(),
    );
}

pub fn assert_body_contains(resp: &TestResponse, needle: &str) {
    let body = resp.text();
    assert!(
        body.contains(needle),
        "Expected body to contain {:?}, but body was:\n{}",
        needle,
        body,
    );
}

pub fn assert_json_field(resp: &TestResponse, path: &str, expected: &Value) {
    let json = resp
        .json_body()
        .expect("Response body is not valid JSON");
    let actual = json_path(&json, path);
    assert_eq!(
        actual,
        Some(expected),
        "JSON field {:?}: expected {:?}, got {:?}",
        path,
        expected,
        actual,
    );
}

pub fn assert_header(resp: &TestResponse, name: &str, expected: &str) {
    let actual = resp.header(name);
    assert_eq!(
        actual,
        Some(expected),
        "Header {:?}: expected {:?}, got {:?}",
        name,
        expected,
        actual,
    );
}

fn json_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = value;
    for part in path.split('.') {
        match current {
            Value::Object(map) => {
                current = map.get(part)?;
            }
            Value::Array(arr) => {
                let idx: usize = part.parse().ok()?;
                current = arr.get(idx)?;
            }
            _ => return None,
        }
    }
    Some(current)
}
