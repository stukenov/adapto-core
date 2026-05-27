# adapto_compiler

Template compiler for Adapto — transforms parser AST into intermediate representation with static/dynamic segments, dependency graphs, and route manifests.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **AST to IR** — compile parsed `.adapto` files into ComponentIR
- **Static/dynamic segments** — split templates for optimal rendering
- **Dependency graphs** — track reactive data dependencies
- **Route manifests** — generate route tables from compiled components
- **Validation** — catch template errors at compile time

## Quick Start

```toml
[dependencies]
adapto_compiler = "0.2"
```

```rust
use adapto_compiler::Compiler;
use adapto_parser::parse_file;

let ast = parse_file(source)?;
let compiler = Compiler::new();
let ir = compiler.compile(&ast)?;

// Access compiled output
println!("Component: {}", ir.name);
println!("Static segments: {}", ir.static_segments.len());
println!("Dynamic segments: {}", ir.dynamic_segments.len());
println!("Dependencies: {:?}", ir.dependency_graph);
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
