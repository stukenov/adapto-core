# Adapto VS Code Extension — Design Spec

**Date:** 2026-05-28
**Status:** Approved
**Repo:** `adapto-vscode` (separate repository)

## Overview

Full IDE experience for `.adapto` files: syntax highlighting, LSP-powered language features, CLI integration with sidebar UI, WebView live preview, and snippets. Distributed via VS Code Marketplace and GitHub Releases.

## Architecture

Two components in separate repo `adapto-vscode`:

1. **`adapto-lsp`** — Rust binary. Depends on `adapto_parser` + `adapto_compiler` from `adapto-core` as git dependencies. Uses `tower-lsp` crate. Handles all language intelligence: diagnostics, completions, hover, go-to-definition, symbols, formatting, code actions, semantic tokens.

2. **VS Code extension** — TypeScript client. Thin wrapper that launches LSP binary, provides TextMate grammar, snippets, CLI command integration, WebView preview panel, and sidebar tree view.

Extension bundles pre-built LSP binaries per platform (macOS arm64/x64, Linux x64, Windows x64).

Communication: LSP over stdio.

## LSP Server Features

| Feature | Description |
|---------|-------------|
| Diagnostics | Parse errors from `adapto_parser`, type errors from compiler. Real-time on save/change |
| Autocomplete | Block keywords (`<route>`, `<script>`, `<template>`, `<style>`, `<resource>`, `<layout>`), attribute names, control flow (`{#if}`, `{#each}`, `{#match}`, `{#can}`), state/prop/memo fields within same file, event modifiers (`.prevent`, `.stop`, `.debounce`), filter names |
| Hover | Type info for state fields, docs for keywords/attributes, permission descriptions |
| Go to Definition | State field → declaration in `<script>`, action handler → function, component → file |
| Document Symbols | Outline view: route, state fields, actions, load functions, template structure |
| Formatting | Format `.adapto` files — indent blocks, align attributes |
| Code Actions | Quick fixes for common errors, extract component, wrap in `{#if}` |
| Semantic Tokens | Rich coloring beyond TextMate: distinguish state vs props vs memo, action vs server fn |

## TextMate Grammar

File association: `*.adapto` → language ID `adapto`

### Embedded language scopes

- `<script lang="rust">` → inject `source.rust`
- `<style>` → inject `source.css`
- `<template>` → inject `text.html.basic` + custom adapto template grammar
- `<route>` / `<resource>` / `<layout>` → custom YAML-like grammar

### Template-specific highlights

- Control flow: `{#if}`, `{:else}`, `{/if}`, `{#each}`, `{#match}`, `{#can}` — keyword color
- Expressions: `{variable}`, `{@html raw}` — expression color
- Bindings: `bind:value` — special attribute color
- Event modifiers: `on:click.prevent.stop` — event color
- Components: `<PascalCase />` vs `<lowercase>` — different colors
- Decorators: `#[permission()]`, `#[audit()]` — annotation color

Grammar file: `syntaxes/adapto.tmLanguage.json`

## CLI Integration

### Command Palette

| Command | Action |
|---------|--------|
| `Adapto: New Project` | Run `adapto new` with input prompts for name/template |
| `Adapto: Start Dev Server` | Run `adapto dev`, show output in terminal, status bar indicator |
| `Adapto: Stop Dev Server` | Kill dev server process |
| `Adapto: Build` | Run `adapto build --release` |
| `Adapto: Check` | Run `adapto check`, pipe diagnostics to Problems panel |
| `Adapto: Generate Resource` | Input dialog → `adapto generate resource <Name>` |
| `Adapto: Show Routes` | Run `adapto routes`, display in WebView table |
| `Adapto: Doctor` | Run `adapto doctor`, show results in output channel |

### Status Bar

- Dev server status: `$(server) Adapto: Running :3000` / `$(circle-slash) Adapto: Stopped`
- Click toggles start/stop

### Sidebar Tree View

Activity bar icon opens Adapto sidebar with sections:

- **Project** — `adapto.toml` config summary
- **Routes** — tree of all routes from `.adapto` files
- **Resources** — list of `<resource>` blocks with fields
- **Components** — tree of `.adapto` files organized by directory

Each tree item: click → open file, context menu → generate/delete/rename.

## WebView Preview Panel

### Activation

Command `Adapto: Open Preview` or icon button in editor title bar (like Markdown preview).

### Behavior

- Split panel, right side. Shows rendered HTML output of current `.adapto` file
- Updates on file save (debounced 300ms)
- Uses `adapto_ssr` compiled to WASM for in-extension rendering — no dev server needed
- Falls back to LSP server rendering if WASM too complex for v1

### Features

- Rendered template with sample data (from `<script>` defaults or `preview.json`)
- State toggles — click to simulate state changes (`{#if}` branches)
- Error overlay — parse/compile errors shown inline in preview
- Responsive breakpoint selector (mobile/tablet/desktop)

### Data source priority

1. `preview.json` next to `.adapto` file (explicit mock data)
2. Default values from `state` declarations in `<script>`
3. Empty/placeholder data with type-based generation

## Snippets

| Prefix | Output |
|--------|--------|
| `apage` | Full page scaffold: `<route>` + `<script>` + `<template>` + `<style>` |
| `aroute` | `<route>` block with common attributes |
| `ascript` | `<script lang="rust">` with state/load/action stubs |
| `aresource` | `<resource>` block with fields |
| `alayout` | `<layout>` block |
| `aif` | `{#if condition}...{/if}` |
| `aeach` | `{#each items as item}...{/each}` |
| `amatch` | `{#match expr}{:when pattern}...{/match}` |
| `acan` | `{#can "permission"}...{/can}` |
| `aaction` | Action function with `#[permission]` decorator |
| `aform` | Form struct with validation fields |

## Additional DX Features

- Bracket matching for `{#if}`/`{/if}`, `{#each}`/`{/each}` pairs
- Auto-closing — type `{#if` → auto-insert `{/if}`
- Color decorators in `<style>` blocks
- Emmet support inside `<template>` blocks
- Custom `.adapto` file icon in explorer

## Repo Structure

```
adapto-vscode/
├── crates/
│   └── adapto-lsp/              # Rust LSP server
│       ├── Cargo.toml           # deps: adapto_parser, adapto_compiler, tower-lsp
│       └── src/
│           ├── main.rs          # entry point, stdio transport
│           ├── server.rs        # LanguageServer impl
│           ├── diagnostics.rs   # parse/compile error mapping
│           ├── completion.rs    # autocomplete providers
│           ├── hover.rs         # hover info
│           ├── definition.rs    # go-to-definition
│           ├── symbols.rs       # document symbols
│           ├── formatting.rs    # code formatter
│           ├── actions.rs       # code actions
│           └── semantic.rs      # semantic tokens
├── extension/                   # VS Code extension (TypeScript)
│   ├── package.json             # extension manifest
│   ├── tsconfig.json
│   ├── src/
│   │   ├── extension.ts         # activation, LSP client launch
│   │   ├── preview.ts           # WebView preview provider
│   │   ├── sidebar.ts           # tree view providers
│   │   └── commands.ts          # CLI command wrappers
│   ├── syntaxes/
│   │   └── adapto.tmLanguage.json
│   ├── snippets/
│   │   └── adapto.json
│   ├── icons/
│   │   └── adapto.svg           # file icon
│   └── media/
│       └── preview.css          # WebView styles
├── .github/workflows/
│   ├── ci.yml                   # test LSP + extension on push
│   └── release.yml              # build binaries + publish .vsix
└── README.md
```

## Distribution

### CI Pipeline

1. **On push:** `cargo test` for LSP, `npm test` for extension, build check all platforms
2. **On tag (`v*`):** Cross-compile LSP for 4 targets, bundle into `.vsix`, publish to VS Code Marketplace + GitHub Release

### Platform Binaries (bundled in .vsix)

```
extension/bin/
├── adapto-lsp-darwin-arm64
├── adapto-lsp-darwin-x64
├── adapto-lsp-linux-x64
└── adapto-lsp-win32-x64.exe
```

Extension auto-detects platform at activation, launches correct binary.

### Channels

- **Primary:** VS Code Marketplace as "Adapto"
- **Secondary:** `.vsix` files on GitHub Releases for offline/enterprise installs

## Implementation Phases

### Phase 1: Foundation
- Repo setup, TextMate grammar, basic extension activation
- LSP server skeleton with `tower-lsp`, diagnostics from parser

### Phase 2: Language Intelligence
- Autocomplete, hover, go-to-definition, document symbols
- Semantic tokens

### Phase 3: Developer Tools
- CLI command integration, status bar, sidebar tree views
- Snippets, bracket matching, auto-closing

### Phase 4: Preview & Polish
- WebView preview panel (SSR or WASM-based)
- File icons, Emmet support, formatting
- Code actions, quick fixes

### Phase 5: Distribution
- CI/CD pipeline, cross-compilation
- Marketplace publishing, GitHub releases
