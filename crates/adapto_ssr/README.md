# adapto_ssr

Server-side rendering for Adapto — renders ComponentIR to HTML with state interpolation, conditional/loop evaluation, event binding, and page wrapping.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **Component rendering** — render ComponentIR to HTML strings
- **State interpolation** — resolve `{expressions}` with server-side data
- **Conditionals and loops** — evaluate `{#if}` and `{#each}` at render time
- **Event binding** — generate client-side event attributes
- **Page wrapping** — full HTML page with bootstrap data for hydration
- **AdaptoServer** — integrated server with SSR pipeline

## Quick Start

```toml
[dependencies]
adapto_ssr = "0.2"
```

```rust
use adapto_ssr::{Renderer, PageRenderer};
use adapto_compiler::Compiler;
use adapto_parser::parse_file;

let ast = parse_file(source)?;
let ir = Compiler::new().compile(&ast)?;

// Render component
let html = Renderer::render_component(&ir, &state)?;

// Full page render
let page = PageRenderer::new()
    .title("My Page")
    .component(&ir)
    .state(&state)
    .render()?;
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
