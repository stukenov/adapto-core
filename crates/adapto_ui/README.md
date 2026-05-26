# adapto_ui

CSS component library and HTML utilities for Rust web applications.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **html_escape()** — safe HTML escaping for user-generated content
- **CSS components** — pre-built styles for common UI patterns
- **Zero dependencies** — lightweight, no external crates

## Quick Start

```toml
[dependencies]
adapto_ui = "0.1"
```

```rust
use adapto_ui::html_escape;

let safe = html_escape("<script>alert('xss')</script>");
// &lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
