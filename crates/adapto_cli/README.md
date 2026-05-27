# adapto_cli

CLI for the Adapto framework — scaffold, develop, build, lint, and generate components.

Part of the [Adapto](https://github.com/stukenov/adapto-core) web framework.

## Features

- **`adapto new`** — scaffold a new Adapto project with sensible defaults
- **`adapto dev`** — start a live development server with hot reload
- **`adapto build`** — compile and validate templates for production
- **`adapto check`** — lint templates and configuration
- **`adapto generate`** — scaffold components, pages, and resources

## Quick Start

```toml
# Install globally
# cargo install adapto_cli
```

```bash
# Create a new project
adapto new myapp
cd myapp

# Start dev server
adapto dev

# Build for production
adapto build

# Lint project
adapto check

# Generate a component
adapto generate component UserProfile
```

## License

MIT — [Saken Tukenov](https://github.com/stukenov)
