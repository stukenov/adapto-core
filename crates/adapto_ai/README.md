# adapto_ai

AI/LLM integration for Adapto — LlmClient trait, prompt templates, response caching, PII detection, budget tracking.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **LlmClient trait** — pluggable LLM backend abstraction
- **Prompt templates** — `{{var}}` substitution with validation
- **Response caching** — TTL and LRU-based cache for LLM responses
- **PII detection** — detect and mask sensitive data before sending to LLMs
- **Budget tracking** — token usage tracking with configurable limits

## Quick Start

```toml
[dependencies]
adapto_ai = "0.2"
```

```rust
use adapto_ai::{MockLlmClient, LlmClient};
use adapto_ai::prompt::PromptTemplate;
use adapto_ai::cache::ResponseCache;

// Prompt template
let tpl = PromptTemplate::new("Summarize this {{language}} code: {{code}}");
let prompt = tpl.render(&[("language", "Rust"), ("code", "fn main() {}")])?;

// Mock client for testing
let client = MockLlmClient::new("mock response");
let response = client.complete(&prompt).await?;

// Response cache
let cache = ResponseCache::new(100, std::time::Duration::from_secs(3600));
cache.put(&prompt, &response);
let cached = cache.get(&prompt);
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
