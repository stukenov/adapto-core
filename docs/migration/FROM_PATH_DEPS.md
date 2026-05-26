# Migration: Path Dependencies → crates.io

## Cargo.toml

Replace path dependencies with crates.io versions:

```toml
# Before (path deps):
adapto_app = { path = "../../adapto-core/crates/adapto_app" }
adapto_store = { path = "../../adapto-core/crates/adapto_store" }
adapto_ui = { path = "../../adapto-core/crates/adapto_ui" }

# After (crates.io):
adapto_app = "0.1"
adapto_store = "0.1"
adapto_ui = "0.1"
```

Run `cargo update` after changing dependencies.

## slugify()

Replace local slugify implementation with `adapto_store::slugify`:

```rust
// Before:
fn slugify(input: &str) -> String { /* 40 lines of transliteration */ }

// After:
use adapto_store::slugify;
```

Supports Russian + Kazakh Cyrillic, trademark symbols, hyphen normalization.

## localized_page()

Replace duplicate route registration with `localized_page()`:

```rust
// Before: 650 lines of duplicated routes
fn register_routes(app: App) -> App {
    app.page("/reference/drugs", |ctx| render_drugs(ctx, Lang::Ru))
}
fn register_kz_routes(app: App) -> App {
    app.page("/kz/reference/drugs", |ctx| render_drugs(ctx, Lang::Kk))
}

// After: single registration
use adapto_app::LangConfig;

#[derive(Clone)]
enum Lang { Ru, Kk }

impl LangConfig for Lang {
    fn code(&self) -> &str {
        match self { Lang::Ru => "ru", Lang::Kk => "kk" }
    }
    fn prefix(&self) -> &str {
        match self { Lang::Ru => "", Lang::Kk => "/kz" }
    }
}

let app = app
    .languages(vec![Lang::Ru, Lang::Kk])
    .localized_page("/reference/drugs", |ctx| {
        let lang = match ctx.lang_code() { "kk" => Lang::Kk, _ => Lang::Ru };
        render_drugs(ctx.store(), lang)
    });
```

## PageResponse

Return proper HTTP status codes instead of always 200:

```rust
use adapto_app::PageResponse;

// Before: always returns 200
.page("/drugs/:slug", |ctx| {
    match find_drug(ctx.store(), ctx.param("slug")) {
        Some(html) => html,
        None => "<h1>Not found</h1>".to_string(), // 200 OK :(
    }
})

// After: proper 404
.page("/drugs/:slug", |ctx| {
    match find_drug(ctx.store(), ctx.param("slug")) {
        Some(html) => PageResponse::Ok(html),
        None => PageResponse::NotFound,
    }
})

// Redirects (301):
PageResponse::Redirect("/new-url".to_string())
```

Existing handlers returning `String` still work — `From<String> for PageResponse` is implemented.
