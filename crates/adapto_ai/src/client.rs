use crate::action::{AiResponse, TokenUsage};
use crate::error::AiError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stop: Vec<String>,
}

impl CompletionRequest {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            stop: Vec::new(),
        }
    }

    pub fn system(mut self, content: &str) -> Self {
        self.messages.push(ChatMessage {
            role: MessageRole::System,
            content: content.into(),
        });
        self
    }

    pub fn user(mut self, content: &str) -> Self {
        self.messages.push(ChatMessage {
            role: MessageRole::User,
            content: content.into(),
        });
        self
    }

    pub fn assistant(mut self, content: &str) -> Self {
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: content.into(),
        });
        self
    }

    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn max_tokens(mut self, tokens: u32) -> Self {
        self.max_tokens = Some(tokens);
        self
    }

    pub fn stop_sequence(mut self, stop: &str) -> Self {
        self.stop.push(stop.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub usage: TokenUsage,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait LlmClient: Send + Sync {
    fn complete(&self, request: CompletionRequest) -> BoxFuture<'_, Result<CompletionResponse, AiError>>;

    fn complete_json(
        &self,
        request: CompletionRequest,
    ) -> BoxFuture<'_, Result<serde_json::Value, AiError>> {
        Box::pin(async move {
            let response = self.complete(request).await?;
            serde_json::from_str(&response.content)
                .map_err(|e| AiError::ExecutionFailed(format!("JSON parse error: {}", e)))
        })
    }
}

pub struct MockLlmClient {
    responses: Arc<Mutex<Vec<CompletionResponse>>>,
    default_response: CompletionResponse,
    call_count: Arc<Mutex<usize>>,
    recorded_requests: Arc<Mutex<Vec<CompletionRequest>>>,
}

impl MockLlmClient {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(Vec::new())),
            default_response: CompletionResponse {
                content: "mock response".into(),
                model: "mock-model".into(),
                usage: TokenUsage::new(10, 20),
                finish_reason: FinishReason::Stop,
            },
            call_count: Arc::new(Mutex::new(0)),
            recorded_requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_response(mut self, content: &str) -> Self {
        self.default_response.content = content.into();
        self
    }

    pub fn with_responses(mut self, responses: Vec<CompletionResponse>) -> Self {
        self.responses = Arc::new(Mutex::new(responses));
        self
    }

    pub fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }

    pub fn recorded_requests(&self) -> Vec<CompletionRequest> {
        self.recorded_requests.lock().unwrap().clone()
    }

    pub fn with_json_response(mut self, value: serde_json::Value) -> Self {
        self.default_response.content = serde_json::to_string(&value).unwrap();
        self
    }
}

impl Default for MockLlmClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmClient for MockLlmClient {
    fn complete(&self, request: CompletionRequest) -> BoxFuture<'_, Result<CompletionResponse, AiError>> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;
        let call_index = *count - 1;

        self.recorded_requests.lock().unwrap().push(request);

        let responses = self.responses.lock().unwrap();
        let response = if call_index < responses.len() {
            responses[call_index].clone()
        } else {
            self.default_response.clone()
        };

        Box::pin(async move { Ok(response) })
    }
}

pub struct MultiProviderClient {
    clients: HashMap<String, Box<dyn LlmClient>>,
}

impl MultiProviderClient {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn add(mut self, name: &str, client: impl LlmClient + 'static) -> Self {
        self.clients.insert(name.into(), Box::new(client));
        self
    }

    pub fn get(&self, name: &str) -> Option<&dyn LlmClient> {
        self.clients.get(name).map(|c| c.as_ref())
    }

    pub fn providers(&self) -> Vec<&str> {
        self.clients.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for MultiProviderClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_client_returns_default() {
        let client = MockLlmClient::new().with_response("hello world");
        let req = CompletionRequest::new("test").user("say hi");
        let resp = client.complete(req).await.unwrap();
        assert_eq!(resp.content, "hello world");
        assert_eq!(client.call_count(), 1);
    }

    #[tokio::test]
    async fn mock_client_records_requests() {
        let client = MockLlmClient::new();
        let req = CompletionRequest::new("test").system("you are helpful").user("hello");
        client.complete(req).await.unwrap();
        let recorded = client.recorded_requests();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].messages.len(), 2);
    }

    #[tokio::test]
    async fn mock_client_sequential_responses() {
        let client = MockLlmClient::new().with_responses(vec![
            CompletionResponse {
                content: "first".into(),
                model: "m".into(),
                usage: TokenUsage::new(1, 1),
                finish_reason: FinishReason::Stop,
            },
            CompletionResponse {
                content: "second".into(),
                model: "m".into(),
                usage: TokenUsage::new(1, 1),
                finish_reason: FinishReason::Stop,
            },
        ]);

        let r1 = client.complete(CompletionRequest::new("m").user("1")).await.unwrap();
        let r2 = client.complete(CompletionRequest::new("m").user("2")).await.unwrap();
        assert_eq!(r1.content, "first");
        assert_eq!(r2.content, "second");
    }

    #[tokio::test]
    async fn mock_client_complete_json() {
        let client = MockLlmClient::new()
            .with_json_response(serde_json::json!({"answer": 42}));
        let resp = client
            .complete_json(CompletionRequest::new("m").user("q"))
            .await
            .unwrap();
        assert_eq!(resp["answer"], 42);
    }

    #[test]
    fn multi_provider_client() {
        let multi = MultiProviderClient::new()
            .add("mock1", MockLlmClient::new().with_response("a"))
            .add("mock2", MockLlmClient::new().with_response("b"));
        assert!(multi.get("mock1").is_some());
        assert!(multi.get("mock2").is_some());
        assert!(multi.get("missing").is_none());
        assert_eq!(multi.providers().len(), 2);
    }

    #[test]
    fn completion_request_builder() {
        let req = CompletionRequest::new("gpt-4")
            .system("be concise")
            .user("hello")
            .temperature(0.5)
            .max_tokens(100)
            .stop_sequence("\n");
        assert_eq!(req.model, "gpt-4");
        assert_eq!(req.messages.len(), 2);
        assert_eq!(req.temperature, Some(0.5));
        assert_eq!(req.max_tokens, Some(100));
        assert_eq!(req.stop.len(), 1);
    }
}
