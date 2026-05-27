# adapto_parser

Template parser for the Adapto DSL — parses `.adapto` files into an AST with route, template, script, style, and resource blocks.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **Pest PEG parser** — robust parsing with clear error messages
- **Template blocks** — HTML with `{expressions}`, `{#if}`, `{#each}`, `{#can}` directives
- **Script blocks** — state, actions, computed properties, load functions
- **Route blocks** — path patterns with parameters
- **Style blocks** — scoped CSS
- **Resource blocks** — data source declarations

## Quick Start

```toml
[dependencies]
adapto_parser = "0.2"
```

```rust
use adapto_parser::parse_file;

let source = r#"
<route path="/users/:id" />

<template>
  <h1>{user.name}</h1>
  {#if user.active}
    <span class="badge">Active</span>
  {/if}
</template>

<script>
  state { user: {} }
  load { user = fetch("/api/users/" + params.id) }
</script>
"#;

let ast = parse_file(source)?;
assert_eq!(ast.route.unwrap().path, "/users/:id");
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
