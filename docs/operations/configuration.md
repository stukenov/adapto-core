# Configuration

`adapto.toml`:

```toml
[app]
name = "school-ai"
env = "development"

[server]
host = "0.0.0.0"
port = 3000

[database]
url = "postgres://..."

[security]
csrf = true
secure_cookies = true
content_security_policy = "strict"

[live]
websocket_path = "/_adapto/live"
max_sessions_per_user = 10
event_rate_limit_per_second = 20

[tenant]
mode = "required"
strategy = "subdomain"

[ai]
default_model = "soz-kz-600m"
fallback_model = "gpt-5.5-thinking"
```
