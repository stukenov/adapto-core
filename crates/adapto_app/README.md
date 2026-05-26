# adapto_app

HTTP app builder for Rust — declarative routes, path params, multilingual support, WebSocket, built on [axum](https://github.com/tokio-rs/axum).

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **Declarative routing** — `.page("/path/:param", handler)` with automatic path param extraction
- **PageResponse** — return `Ok(html)`, `NotFound` (404), or `Redirect(url)` (301) from handlers
- **Multilingual** — `LangConfig` trait + `localized_page()` registers routes for all languages at once
- **WebSocket** — built-in WebSocket support with action handlers
- **Integrated store** — `adapto_store` available in every handler via `ctx.store()`

## Quick Start

```toml
[dependencies]
adapto_app = "0.1"
adapto_store = "0.1"
```

```rust
use adapto_app::{App, PageResponse};
use adapto_store::AdaptoStore;

#[tokio::main]
async fn main() {
    let store = AdaptoStore::open(None).unwrap();

    App::new("My App")
        .port(3000)
        .store(store)
        .page("/", |_ctx| "<h1>Home</h1>".to_string())
        .page("/users/:id", |ctx| {
            let id = ctx.param("id");
            format!("<h1>User {id}</h1>")
        })
        .run()
        .await
        .unwrap();
}
```

## PageResponse

Return proper HTTP status codes from handlers:

```rust
use adapto_app::PageResponse;

.page("/articles/:slug", |ctx| {
    match find_article(ctx.store(), ctx.param("slug")) {
        Some(html) => PageResponse::Ok(html),
        None => PageResponse::NotFound,       // 404
    }
})

// Redirects
.page("/old-path", |_ctx| {
    PageResponse::Redirect("/new-path".to_string()) // 301
})
```

Handlers returning `String` still work — backward compatible via `From<String>`.

## Multilingual Routes

Register one route for all languages:

```rust
use adapto_app::LangConfig;

#[derive(Clone)]
enum Lang { Ru, Kk }

impl LangConfig for Lang {
    fn code(&self) -> &str { match self { Lang::Ru => "ru", Lang::Kk => "kk" } }
    fn prefix(&self) -> &str { match self { Lang::Ru => "", Lang::Kk => "/kz" } }
}

App::new("My App")
    .store(store)
    .languages(vec![Lang::Ru, Lang::Kk])
    .localized_page("/about", |ctx| {
        format!("<h1>Lang: {}</h1>", ctx.lang_code())
    })
    // Registers both GET /about and GET /kz/about
```

## RequestContext

```rust
ctx.store()       // &AdaptoStore
ctx.param("id")   // &str — path parameter
ctx.path()        // &str — full request path
ctx.query         // String — query string
ctx.lang_code()   // &str — "ru", "kk"
ctx.lang_prefix() // &str — "", "/kz"
```

## Fallback Handler

```rust
use adapto_app::FallbackResponse;

.fallback_fn(|path| {
    if path == "/robots.txt" {
        return FallbackResponse::Raw {
            body: "User-agent: *\nAllow: /".into(),
            content_type: "text/plain; charset=utf-8",
        };
    }
    FallbackResponse::NotFound
})
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
