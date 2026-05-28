# Adapto VS Code Extension — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a full VS Code extension for `.adapto` files with syntax highlighting, Rust-based LSP server (diagnostics, completions, hover, go-to-definition, symbols), CLI integration, WebView preview, and snippets.

**Architecture:** Separate repo `adapto-vscode` with two components: (1) `adapto-lsp` Rust crate using `tower-lsp`, depending on `adapto_parser`/`adapto_compiler` from `adapto-core` as git deps; (2) TypeScript VS Code extension client that launches the LSP binary and provides TextMate grammar, snippets, commands, sidebar, and WebView preview.

**Tech Stack:** Rust (tower-lsp, tokio, serde_json), TypeScript (vscode, vscode-languageclient), Node.js (esbuild for bundling), GitHub Actions (cross-compilation, vsce publish)

**Spec:** `docs/superpowers/specs/2026-05-28-vscode-extension-design.md`

---

## File Structure

```
~/adapto-vscode/
├── crates/
│   └── adapto-lsp/
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs              # Entry point, stdio transport
│           ├── server.rs            # LanguageServer trait impl, dispatch
│           ├── document.rs          # Document state management (open/change/close)
│           ├── diagnostics.rs       # Parse + compile error → LSP Diagnostic mapping
│           ├── completion.rs        # Autocomplete providers
│           ├── hover.rs             # Hover info providers
│           ├── definition.rs        # Go-to-definition
│           ├── symbols.rs           # Document symbol outline
│           ├── semantic_tokens.rs   # Semantic token provider
│           └── formatter.rs         # Document formatting
├── extension/
│   ├── package.json                 # Extension manifest (contributes, activationEvents)
│   ├── tsconfig.json
│   ├── esbuild.mjs                  # Bundle script
│   ├── src/
│   │   ├── extension.ts             # activate/deactivate, LSP client
│   │   ├── commands.ts              # CLI command wrappers
│   │   ├── preview.ts               # WebView preview provider
│   │   └── sidebar.ts              # Tree view providers
│   ├── syntaxes/
│   │   └── adapto.tmLanguage.json   # TextMate grammar
│   ├── language-configuration.json  # Brackets, comments, auto-closing
│   ├── snippets/
│   │   └── adapto.json              # Code snippets
│   └── icons/
│       └── adapto-icon.png          # File icon
├── .github/
│   └── workflows/
│       ├── ci.yml                   # Test on push
│       └── release.yml              # Build + publish on tag
├── .gitignore
├── Cargo.toml                       # Workspace root
└── README.md
```

---

## Task 1: Repository Scaffolding

**Files:**
- Create: `~/adapto-vscode/Cargo.toml` (workspace root)
- Create: `~/adapto-vscode/.gitignore`
- Create: `~/adapto-vscode/crates/adapto-lsp/Cargo.toml`
- Create: `~/adapto-vscode/crates/adapto-lsp/src/main.rs`
- Create: `~/adapto-vscode/extension/package.json`
- Create: `~/adapto-vscode/extension/tsconfig.json`
- Create: `~/adapto-vscode/extension/esbuild.mjs`
- Create: `~/adapto-vscode/extension/src/extension.ts`

- [ ] **Step 1: Create repo and workspace Cargo.toml**

```bash
mkdir -p ~/adapto-vscode/crates/adapto-lsp/src
cd ~/adapto-vscode
git init
```

`~/adapto-vscode/Cargo.toml`:
```toml
[workspace]
members = ["crates/adapto-lsp"]
resolver = "2"
```

- [ ] **Step 2: Create .gitignore**

`~/adapto-vscode/.gitignore`:
```
/target
node_modules/
*.vsix
extension/out/
extension/bin/
.superpowers/
```

- [ ] **Step 3: Create adapto-lsp Cargo.toml**

`~/adapto-vscode/crates/adapto-lsp/Cargo.toml`:
```toml
[package]
name = "adapto-lsp"
version = "0.1.0"
edition = "2021"

[dependencies]
adapto_parser = { git = "https://github.com/nickstukenov/adapto-core.git", branch = "master" }
adapto_compiler = { git = "https://github.com/nickstukenov/adapto-core.git", branch = "master" }
tower-lsp = "0.20"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dashmap = "6"
ropey = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
```

- [ ] **Step 4: Create minimal main.rs (hello-world LSP)**

`~/adapto-vscode/crates/adapto-lsp/src/main.rs`:
```rust
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct AdaptoLsp {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for AdaptoLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Adapto LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| AdaptoLsp { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

- [ ] **Step 5: Create extension package.json**

`~/adapto-vscode/extension/package.json`:
```json
{
  "name": "adapto",
  "displayName": "Adapto",
  "description": "Language support for Adapto (.adapto) files",
  "version": "0.1.0",
  "publisher": "adapto",
  "engines": {
    "vscode": "^1.85.0"
  },
  "categories": ["Programming Languages"],
  "activationEvents": [
    "onLanguage:adapto"
  ],
  "main": "./out/extension.js",
  "contributes": {
    "languages": [
      {
        "id": "adapto",
        "aliases": ["Adapto", "adapto"],
        "extensions": [".adapto"],
        "configuration": "./language-configuration.json"
      }
    ],
    "grammars": [
      {
        "language": "adapto",
        "scopeName": "source.adapto",
        "path": "./syntaxes/adapto.tmLanguage.json",
        "embeddedLanguages": {
          "source.rust": "rust",
          "source.css": "css",
          "text.html.basic": "html"
        }
      }
    ]
  },
  "scripts": {
    "vscode:prepublish": "node esbuild.mjs --production",
    "build": "node esbuild.mjs",
    "watch": "node esbuild.mjs --watch",
    "lint": "eslint src --ext ts"
  },
  "dependencies": {
    "vscode-languageclient": "^9.0.1"
  },
  "devDependencies": {
    "@types/vscode": "^1.85.0",
    "@types/node": "^20.0.0",
    "esbuild": "^0.21.0",
    "typescript": "^5.4.0"
  }
}
```

- [ ] **Step 6: Create tsconfig.json**

`~/adapto-vscode/extension/tsconfig.json`:
```json
{
  "compilerOptions": {
    "module": "commonjs",
    "target": "ES2022",
    "outDir": "out",
    "lib": ["ES2022"],
    "sourceMap": true,
    "rootDir": "src",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true
  },
  "exclude": ["node_modules", "out"]
}
```

- [ ] **Step 7: Create esbuild.mjs**

`~/adapto-vscode/extension/esbuild.mjs`:
```js
import * as esbuild from "esbuild";

const production = process.argv.includes("--production");
const watch = process.argv.includes("--watch");

const ctx = await esbuild.context({
  entryPoints: ["src/extension.ts"],
  bundle: true,
  format: "cjs",
  minify: production,
  sourcemap: !production,
  sourcesContent: false,
  platform: "node",
  outfile: "out/extension.js",
  external: ["vscode"],
  logLevel: "silent",
});

if (watch) {
  await ctx.watch();
  console.log("watching...");
} else {
  await ctx.rebuild();
  await ctx.dispose();
}
```

- [ ] **Step 8: Create minimal extension.ts**

`~/adapto-vscode/extension/src/extension.ts`:
```typescript
import * as vscode from "vscode";

export function activate(context: vscode.ExtensionContext) {
  console.log("Adapto extension activated");
}

export function deactivate() {}
```

- [ ] **Step 9: Verify LSP builds**

```bash
cd ~/adapto-vscode
cargo build -p adapto-lsp
```

Expected: successful build.

- [ ] **Step 10: Verify extension builds**

```bash
cd ~/adapto-vscode/extension
npm install
npm run build
```

Expected: `out/extension.js` created.

- [ ] **Step 11: Commit**

```bash
cd ~/adapto-vscode
git add -A
git commit -m "feat: scaffold adapto-vscode repo with LSP crate and extension"
```

---

## Task 2: TextMate Grammar

**Files:**
- Create: `~/adapto-vscode/extension/syntaxes/adapto.tmLanguage.json`
- Create: `~/adapto-vscode/extension/language-configuration.json`

- [ ] **Step 1: Create language-configuration.json**

`~/adapto-vscode/extension/language-configuration.json`:
```json
{
  "comments": {
    "blockComment": ["<!--", "-->"]
  },
  "brackets": [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"],
    ["<", ">"]
  ],
  "autoClosingPairs": [
    { "open": "{", "close": "}" },
    { "open": "[", "close": "]" },
    { "open": "(", "close": ")" },
    { "open": "\"", "close": "\"", "notIn": ["string"] },
    { "open": "'", "close": "'", "notIn": ["string"] },
    { "open": "<!--", "close": "-->" },
    { "open": "<", "close": ">", "notIn": ["string"] }
  ],
  "surroundingPairs": [
    { "open": "{", "close": "}" },
    { "open": "[", "close": "]" },
    { "open": "(", "close": ")" },
    { "open": "\"", "close": "\"" },
    { "open": "'", "close": "'" },
    { "open": "<", "close": ">" }
  ],
  "folding": {
    "markers": {
      "start": "^\\s*<(route|script|template|style|resource|layout)",
      "end": "^\\s*</(route|script|template|style|resource|layout)>"
    }
  },
  "indentationRules": {
    "increaseIndentPattern": "<(?!area|base|br|col|embed|hr|img|input|link|meta|param|source|track|wbr)([a-zA-Z][a-zA-Z0-9]*)[^/>]*>(?!.*</\\1>)|\\{[^}]*$",
    "decreaseIndentPattern": "^\\s*(<\\/[a-zA-Z][a-zA-Z0-9]*>|\\}|\\{\\/(if|each|match|can)\\})"
  }
}
```

- [ ] **Step 2: Create TextMate grammar**

`~/adapto-vscode/extension/syntaxes/adapto.tmLanguage.json`:
```json
{
  "name": "Adapto",
  "scopeName": "source.adapto",
  "fileTypes": ["adapto"],
  "patterns": [
    { "include": "#route-block" },
    { "include": "#script-block" },
    { "include": "#template-block" },
    { "include": "#style-block" },
    { "include": "#resource-block" },
    { "include": "#layout-block" },
    { "include": "#comment" }
  ],
  "repository": {
    "comment": {
      "name": "comment.block.html.adapto",
      "begin": "<!--",
      "end": "-->"
    },
    "route-block": {
      "name": "meta.block.route.adapto",
      "begin": "(<)(route)(>)",
      "beginCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "end": "(</)(route)(>)",
      "endCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "patterns": [
        { "include": "#route-content" }
      ]
    },
    "route-content": {
      "patterns": [
        {
          "match": "^\\s*(path|method|layout|page_title|auth|role|permission|tenant|cache|error|not_found)\\s*(:)",
          "captures": {
            "1": { "name": "variable.other.property.adapto" },
            "2": { "name": "punctuation.separator.key-value.adapto" }
          }
        },
        {
          "match": "\\b(public|required|verified|none|optional|no_store|private|static|get|post|put|delete|patch)\\b",
          "name": "constant.language.adapto"
        },
        {
          "match": "\"[^\"]*\"",
          "name": "string.quoted.double.adapto"
        }
      ]
    },
    "script-block": {
      "name": "meta.block.script.adapto",
      "begin": "(<)(script)(\\s+lang\\s*=\\s*\"rust\")?(>)",
      "beginCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "entity.other.attribute-name.adapto" },
        "4": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "end": "(</)(script)(>)",
      "endCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "patterns": [
        { "include": "#script-keywords" },
        { "include": "#decorators" },
        { "include": "source.rust" }
      ]
    },
    "script-keywords": {
      "patterns": [
        {
          "match": "\\b(prop|state|memo|load|action|server|form|ai_action)\\b",
          "name": "keyword.other.adapto"
        },
        {
          "match": "\\b(async|fn|use|struct|let|mut)\\b",
          "name": "keyword.other.rust"
        }
      ]
    },
    "decorators": {
      "patterns": [
        {
          "match": "#\\[(permission|audit)\\(\"[^\"]*\"\\)\\]",
          "name": "meta.attribute.adapto",
          "captures": {
            "1": { "name": "entity.name.function.decorator.adapto" }
          }
        }
      ]
    },
    "template-block": {
      "name": "meta.block.template.adapto",
      "begin": "(<)(template)(>)",
      "beginCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "end": "(</)(template)(>)",
      "endCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "patterns": [
        { "include": "#template-content" }
      ]
    },
    "template-content": {
      "patterns": [
        { "include": "#control-flow" },
        { "include": "#expression" },
        { "include": "#unsafe-html" },
        { "include": "#component-tag" },
        { "include": "#html-tag" },
        { "include": "#comment" }
      ]
    },
    "control-flow": {
      "patterns": [
        {
          "match": "\\{#(if|each|match|can)\\b",
          "name": "keyword.control.begin.adapto"
        },
        {
          "match": "\\{:(else\\s+if|else|when)\\b",
          "name": "keyword.control.branch.adapto"
        },
        {
          "match": "\\{/(if|each|match|can)\\}",
          "name": "keyword.control.end.adapto"
        }
      ]
    },
    "expression": {
      "name": "meta.expression.adapto",
      "begin": "\\{(?!#|/|:|@)",
      "end": "\\}",
      "beginCaptures": {
        "0": { "name": "punctuation.definition.expression.begin.adapto" }
      },
      "endCaptures": {
        "0": { "name": "punctuation.definition.expression.end.adapto" }
      },
      "patterns": [
        {
          "match": "[a-zA-Z_][a-zA-Z0-9_.]*",
          "name": "variable.other.adapto"
        }
      ]
    },
    "unsafe-html": {
      "match": "\\{@html\\s+[^}]+\\}",
      "name": "meta.unsafe-html.adapto",
      "captures": {
        "0": { "name": "keyword.operator.unsafe.adapto" }
      }
    },
    "component-tag": {
      "patterns": [
        {
          "match": "(</?)(\\b[A-Z][a-zA-Z0-9]*\\b)",
          "captures": {
            "1": { "name": "punctuation.definition.tag.begin.adapto" },
            "2": { "name": "entity.name.type.component.adapto" }
          }
        }
      ]
    },
    "html-tag": {
      "patterns": [
        {
          "match": "(</?)(\\b[a-z][a-z0-9-]*\\b)",
          "captures": {
            "1": { "name": "punctuation.definition.tag.begin.adapto" },
            "2": { "name": "entity.name.tag.html.adapto" }
          }
        },
        { "include": "#tag-attributes" }
      ]
    },
    "tag-attributes": {
      "patterns": [
        {
          "match": "\\b(on):(\\w+)(?:\\.(prevent|stop|debounce|throttle)(?:\\.(\\d+))?)*",
          "captures": {
            "1": { "name": "keyword.operator.event.adapto" },
            "2": { "name": "entity.name.function.event.adapto" },
            "3": { "name": "support.function.modifier.adapto" },
            "4": { "name": "constant.numeric.adapto" }
          }
        },
        {
          "match": "\\b(bind):(\\w+)",
          "captures": {
            "1": { "name": "keyword.operator.binding.adapto" },
            "2": { "name": "variable.other.binding.adapto" }
          }
        },
        {
          "match": "\\b(island)\\b",
          "name": "keyword.other.island.adapto"
        },
        {
          "match": "\\b(class|id|style|href|src|alt|type|name|value|placeholder|disabled|readonly)\\b(?=\\s*=)",
          "name": "entity.other.attribute-name.html.adapto"
        },
        {
          "match": "\"[^\"]*\"",
          "name": "string.quoted.double.adapto"
        }
      ]
    },
    "style-block": {
      "name": "meta.block.style.adapto",
      "begin": "(<)(style)(\\s+(?:scoped|global))?(>)",
      "beginCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "keyword.other.scope.adapto" },
        "4": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "end": "(</)(style)(>)",
      "endCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "patterns": [
        { "include": "source.css" }
      ]
    },
    "resource-block": {
      "name": "meta.block.resource.adapto",
      "begin": "(<)(resource)(\\s+[^>]*)?(>)",
      "beginCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "entity.other.attribute-name.adapto" },
        "4": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "end": "(</)(resource)(>)",
      "endCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "patterns": [
        { "include": "#resource-content" }
      ]
    },
    "resource-content": {
      "patterns": [
        {
          "match": "\\b(field|permission|tenant|primary_key)\\b",
          "name": "keyword.other.resource.adapto"
        },
        {
          "match": "@(email|required|min|max|unique|optional|searchable|readonly)\\b",
          "name": "support.function.constraint.adapto"
        },
        {
          "match": "\\b(String|Integer|Float|Boolean|DateTime|Uuid|Json|Text)\\b",
          "name": "support.type.adapto"
        },
        {
          "match": "\"[^\"]*\"",
          "name": "string.quoted.double.adapto"
        }
      ]
    },
    "layout-block": {
      "name": "meta.block.layout.adapto",
      "begin": "(<)(layout)(\\s+[^>]*)?(>)",
      "beginCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "entity.other.attribute-name.adapto" },
        "4": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "end": "(</)(layout)(>)",
      "endCaptures": {
        "1": { "name": "punctuation.definition.tag.begin.adapto" },
        "2": { "name": "entity.name.tag.block.adapto" },
        "3": { "name": "punctuation.definition.tag.end.adapto" }
      },
      "patterns": [
        {
          "match": "\\b(name|auth|tenant)\\s*(:)",
          "captures": {
            "1": { "name": "variable.other.property.adapto" },
            "2": { "name": "punctuation.separator.key-value.adapto" }
          }
        },
        {
          "match": "\\b(public|required|verified|none|optional)\\b",
          "name": "constant.language.adapto"
        },
        {
          "match": "\"[^\"]*\"",
          "name": "string.quoted.double.adapto"
        }
      ]
    }
  }
}
```

- [ ] **Step 3: Verify grammar loads in VS Code**

Manually: open VS Code, run "Developer: Inspect Editor Tokens and Scopes" on a `.adapto` file. Verify block tags get `entity.name.tag.block.adapto`, control flow gets `keyword.control.*`, expressions get `variable.other.adapto`.

- [ ] **Step 4: Commit**

```bash
cd ~/adapto-vscode
git add extension/syntaxes/ extension/language-configuration.json
git commit -m "feat: add TextMate grammar and language configuration for .adapto files"
```

---

## Task 3: Document State Management

**Files:**
- Create: `~/adapto-vscode/crates/adapto-lsp/src/document.rs`
- Modify: `~/adapto-vscode/crates/adapto-lsp/src/main.rs`

- [ ] **Step 1: Create document.rs**

`~/adapto-vscode/crates/adapto-lsp/src/document.rs`:
```rust
use adapto_compiler::compiler::{CompileOutput, Compiler};
use adapto_parser::{parse, AdaptoFile, ParseError};
use dashmap::DashMap;
use tower_lsp::lsp_types::Url;

#[derive(Debug)]
pub struct DocumentState {
    pub text: String,
    pub version: i32,
    pub ast: Option<AdaptoFile>,
    pub compiled: Option<CompileOutput>,
    pub parse_errors: Vec<ParseError>,
    pub compile_errors: Vec<adapto_compiler::error::CompileError>,
}

impl DocumentState {
    pub fn new(text: String, version: i32) -> Self {
        let mut state = Self {
            text,
            version,
            ast: None,
            compiled: None,
            parse_errors: Vec::new(),
            compile_errors: Vec::new(),
        };
        state.reparse();
        state
    }

    pub fn update(&mut self, text: String, version: i32) {
        self.text = text;
        self.version = version;
        self.reparse();
    }

    fn reparse(&mut self) {
        self.parse_errors.clear();
        self.compile_errors.clear();

        match parse(&self.text) {
            Ok(ast) => {
                let mut compiler = Compiler::new();
                match compiler.compile_file(&ast, "<editor>") {
                    Ok(output) => {
                        self.compiled = Some(output);
                    }
                    Err(e) => {
                        self.compile_errors.push(e);
                        self.compiled = None;
                    }
                }
                self.ast = Some(ast);
            }
            Err(e) => {
                self.parse_errors.push(e);
                self.ast = None;
                self.compiled = None;
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct DocumentStore {
    documents: DashMap<Url, DocumentState>,
}

impl DocumentStore {
    pub fn open(&self, uri: Url, text: String, version: i32) {
        self.documents.insert(uri, DocumentState::new(text, version));
    }

    pub fn change(&self, uri: &Url, text: String, version: i32) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.update(text, version);
        }
    }

    pub fn close(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<dashmap::mapref::one::Ref<'_, Url, DocumentState>> {
        self.documents.get(uri)
    }
}
```

- [ ] **Step 2: Update main.rs to use DocumentStore**

Replace `~/adapto-vscode/crates/adapto-lsp/src/main.rs`:
```rust
mod document;

use document::DocumentStore;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct AdaptoLsp {
    client: Client,
    documents: DocumentStore,
}

#[tower_lsp::async_trait]
impl LanguageServer for AdaptoLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Adapto LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let version = params.text_document.version;
        self.documents.open(uri.clone(), text, version);
        self.publish_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.change(&uri, change.text, version);
            self.publish_diagnostics(&uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.close(&params.text_document.uri);
    }
}

impl AdaptoLsp {
    async fn publish_diagnostics(&self, uri: &Url) {
        let diagnostics = if let Some(doc) = self.documents.get(uri) {
            let mut diags = Vec::new();
            for err in &doc.parse_errors {
                if let adapto_parser::ParseError::Syntax { line, col, message } = err {
                    diags.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line.saturating_sub(1) as u32,
                                character: col.saturating_sub(1) as u32,
                            },
                            end: Position {
                                line: line.saturating_sub(1) as u32,
                                character: *col as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some("adapto".to_string()),
                        message: message.clone(),
                        ..Default::default()
                    });
                }
            }
            for err in &doc.compile_errors {
                diags.push(Diagnostic {
                    range: Range::default(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("adapto".to_string()),
                    message: err.to_string(),
                    ..Default::default()
                });
            }
            diags
        } else {
            Vec::new()
        };

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| AdaptoLsp {
        client,
        documents: DocumentStore::default(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

- [ ] **Step 3: Verify builds**

```bash
cd ~/adapto-vscode
cargo build -p adapto-lsp
```

- [ ] **Step 4: Commit**

```bash
cd ~/adapto-vscode
git add crates/adapto-lsp/src/
git commit -m "feat: add document state management with parse/compile on change"
```

---

## Task 4: LSP Client in Extension

**Files:**
- Modify: `~/adapto-vscode/extension/src/extension.ts`
- Modify: `~/adapto-vscode/extension/package.json`

- [ ] **Step 1: Update extension.ts with LSP client**

`~/adapto-vscode/extension/src/extension.ts`:
```typescript
import * as path from "path";
import * as os from "os";
import { workspace, ExtensionContext } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient;

function getServerPath(context: ExtensionContext): string {
  const platform = os.platform();
  const arch = os.arch();

  let binaryName: string;
  if (platform === "darwin" && arch === "arm64") {
    binaryName = "adapto-lsp-darwin-arm64";
  } else if (platform === "darwin") {
    binaryName = "adapto-lsp-darwin-x64";
  } else if (platform === "linux") {
    binaryName = "adapto-lsp-linux-x64";
  } else if (platform === "win32") {
    binaryName = "adapto-lsp-win32-x64.exe";
  } else {
    binaryName = "adapto-lsp";
  }

  const bundled = path.join(context.extensionPath, "bin", binaryName);
  try {
    require("fs").accessSync(bundled, require("fs").constants.X_OK);
    return bundled;
  } catch {
    return "adapto-lsp";
  }
}

export function activate(context: ExtensionContext) {
  const serverPath = getServerPath(context);

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: [],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "adapto" }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher("**/*.adapto"),
    },
  };

  client = new LanguageClient(
    "adapto-lsp",
    "Adapto Language Server",
    serverOptions,
    clientOptions,
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
```

- [ ] **Step 2: Rebuild extension**

```bash
cd ~/adapto-vscode/extension
npm run build
```

- [ ] **Step 3: Commit**

```bash
cd ~/adapto-vscode
git add extension/
git commit -m "feat: add LSP client with platform-specific binary detection"
```

---

## Task 5: Autocomplete Provider

**Files:**
- Create: `~/adapto-vscode/crates/adapto-lsp/src/completion.rs`
- Modify: `~/adapto-vscode/crates/adapto-lsp/src/main.rs`

- [ ] **Step 1: Create completion.rs**

`~/adapto-vscode/crates/adapto-lsp/src/completion.rs`:
```rust
use tower_lsp::lsp_types::*;

use crate::document::DocumentState;

pub fn completions(doc: &DocumentState, position: Position) -> Vec<CompletionItem> {
    let line_idx = position.line as usize;
    let lines: Vec<&str> = doc.text.lines().collect();
    let current_line = lines.get(line_idx).copied().unwrap_or("");

    let mut items = Vec::new();
    let context = detect_context(current_line, &lines, line_idx);

    match context {
        Context::TopLevel => {
            items.extend(block_completions());
        }
        Context::Route => {
            items.extend(route_attribute_completions());
        }
        Context::Template => {
            items.extend(template_completions());
            if let Some(ref ast) = doc.ast {
                items.extend(state_field_completions(ast));
            }
        }
        Context::Resource => {
            items.extend(resource_completions());
        }
        Context::Script => {
            items.extend(script_keyword_completions());
        }
        Context::EventModifier => {
            items.extend(event_modifier_completions());
        }
        Context::Unknown => {}
    }

    items
}

#[derive(Debug)]
enum Context {
    TopLevel,
    Route,
    Template,
    Resource,
    Script,
    EventModifier,
    Unknown,
}

fn detect_context(current_line: &str, lines: &[&str], line_idx: usize) -> Context {
    let trimmed = current_line.trim();
    if trimmed.starts_with('<') && !trimmed.contains('>') {
        return Context::TopLevel;
    }

    if trimmed.contains("on:") {
        return Context::EventModifier;
    }

    for i in (0..=line_idx).rev() {
        let l = lines.get(i).copied().unwrap_or("").trim().to_string();
        if l.starts_with("</") {
            continue;
        }
        if l.starts_with("<route") {
            return Context::Route;
        }
        if l.starts_with("<template") {
            return Context::Template;
        }
        if l.starts_with("<resource") {
            return Context::Resource;
        }
        if l.starts_with("<script") {
            return Context::Script;
        }
    }

    Context::Unknown
}

fn block_completions() -> Vec<CompletionItem> {
    vec![
        make_snippet("route", "<route>\n\t$0\n</route>", "Route block"),
        make_snippet(
            "script",
            "<script lang=\"rust\">\n\t$0\n</script>",
            "Script block",
        ),
        make_snippet("template", "<template>\n\t$0\n</template>", "Template block"),
        make_snippet("style", "<style scoped>\n\t$0\n</style>", "Style block"),
        make_snippet(
            "resource",
            "<resource table=\"$1\">\n\t$0\n</resource>",
            "Resource block",
        ),
        make_snippet(
            "layout",
            "<layout name=\"$1\">\n\t$0\n</layout>",
            "Layout block",
        ),
    ]
}

fn route_attribute_completions() -> Vec<CompletionItem> {
    vec![
        make_keyword("path", "Route path pattern"),
        make_keyword("method", "HTTP method (get, post, put, delete)"),
        make_keyword("layout", "Layout name"),
        make_keyword("auth", "Auth level (public, required, verified)"),
        make_keyword("permission", "Required permission"),
        make_keyword("tenant", "Tenant mode (none, required, optional)"),
        make_keyword("cache", "Cache policy"),
    ]
}

fn template_completions() -> Vec<CompletionItem> {
    vec![
        make_snippet("if", "{#if $1}\n\t$0\n{/if}", "Conditional block"),
        make_snippet(
            "if-else",
            "{#if $1}\n\t$2\n{:else}\n\t$0\n{/if}",
            "Conditional with else",
        ),
        make_snippet(
            "each",
            "{#each $1 as $2}\n\t$0\n{/each}",
            "Loop over collection",
        ),
        make_snippet(
            "match",
            "{#match $1}\n\t{:when $2}\n\t\t$0\n{/match}",
            "Pattern match",
        ),
        make_snippet("can", "{#can \"$1\"}\n\t$0\n{/can}", "Permission guard"),
        make_snippet("html", "{@html $0}", "Raw HTML output"),
    ]
}

fn resource_completions() -> Vec<CompletionItem> {
    vec![
        make_snippet(
            "field",
            "field $1: $2 ${3|@required,@optional,@unique,@searchable|}",
            "Resource field",
        ),
        make_keyword("permission", "Resource permission"),
        make_keyword("tenant", "Tenant scope"),
        make_keyword("primary_key", "Primary key field"),
    ]
}

fn script_keyword_completions() -> Vec<CompletionItem> {
    vec![
        make_snippet("state", "state $1: $2 = $0", "State declaration"),
        make_snippet("prop", "prop $1: $2", "Prop declaration"),
        make_snippet("memo", "memo $1: $2 = $0", "Memo (computed) declaration"),
        make_snippet(
            "load",
            "load async fn $1(ctx: Ctx) {\n\t$0\n}",
            "Load function",
        ),
        make_snippet(
            "action",
            "action async fn $1($2) {\n\t$0\n}",
            "Action handler",
        ),
        make_snippet(
            "server",
            "server async fn $1($2) {\n\t$0\n}",
            "Server function",
        ),
    ]
}

fn event_modifier_completions() -> Vec<CompletionItem> {
    vec![
        make_keyword("prevent", "Call preventDefault()"),
        make_keyword("stop", "Call stopPropagation()"),
        make_snippet("debounce", "debounce.${1:300}", "Debounce with ms delay"),
        make_snippet("throttle", "throttle.${1:300}", "Throttle with ms delay"),
    ]
}

fn state_field_completions(ast: &adapto_parser::AdaptoFile) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    if let Some(ref script) = ast.script {
        for s in &script.states {
            items.push(CompletionItem {
                label: s.name.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(format!("state: {}", s.ty)),
                ..Default::default()
            });
        }
        for p in &script.props {
            items.push(CompletionItem {
                label: p.name.clone(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(format!("prop: {}", p.ty)),
                ..Default::default()
            });
        }
        for m in &script.memos {
            items.push(CompletionItem {
                label: m.name.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(format!("memo: {}", m.ty)),
                ..Default::default()
            });
        }
        for a in &script.actions {
            items.push(CompletionItem {
                label: a.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("action".to_string()),
                ..Default::default()
            });
        }
    }
    items
}

fn make_snippet(label: &str, snippet: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(detail.to_string()),
        insert_text: Some(snippet.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}

fn make_keyword(label: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(detail.to_string()),
        ..Default::default()
    }
}
```

- [ ] **Step 2: Register completion in main.rs**

Add `mod completion;` at top of main.rs.

Add to `ServerCapabilities` in `initialize`:
```rust
completion_provider: Some(CompletionOptions {
    trigger_characters: Some(vec![
        "<".to_string(),
        "{".to_string(),
        ":".to_string(),
        ".".to_string(),
        "@".to_string(),
    ]),
    ..Default::default()
}),
```

Add method to `LanguageServer` impl:
```rust
async fn completion(
    &self,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;
    let items = if let Some(doc) = self.documents.get(&uri) {
        completion::completions(&doc, pos)
    } else {
        Vec::new()
    };
    Ok(Some(CompletionResponse::Array(items)))
}
```

- [ ] **Step 3: Build and verify**

```bash
cd ~/adapto-vscode
cargo build -p adapto-lsp
```

- [ ] **Step 4: Commit**

```bash
cd ~/adapto-vscode
git add crates/adapto-lsp/src/
git commit -m "feat: add context-aware autocomplete for all block types"
```

---

## Task 6: Hover Provider

**Files:**
- Create: `~/adapto-vscode/crates/adapto-lsp/src/hover.rs`
- Modify: `~/adapto-vscode/crates/adapto-lsp/src/main.rs`

- [ ] **Step 1: Create hover.rs**

`~/adapto-vscode/crates/adapto-lsp/src/hover.rs`:
```rust
use tower_lsp::lsp_types::*;

use crate::document::DocumentState;

pub fn hover_info(doc: &DocumentState, position: Position) -> Option<Hover> {
    let line_idx = position.line as usize;
    let col = position.character as usize;
    let lines: Vec<&str> = doc.text.lines().collect();
    let current_line = lines.get(line_idx)?;

    let word = extract_word(current_line, col)?;

    if let Some(info) = keyword_hover(&word) {
        return Some(info);
    }

    if let Some(ref ast) = doc.ast {
        if let Some(info) = state_hover(ast, &word) {
            return Some(info);
        }
    }

    None
}

fn extract_word(line: &str, col: usize) -> Option<String> {
    let bytes = line.as_bytes();
    if col >= bytes.len() {
        return None;
    }

    let mut start = col;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }

    let mut end = col;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(line[start..end].to_string())
}

fn keyword_hover(word: &str) -> Option<Hover> {
    let docs = match word {
        "route" => "**`<route>`** — Route metadata block\n\nDefines HTTP route: path, method, auth level, layout, permissions, tenant scope, caching.",
        "script" => "**`<script lang=\"rust\">`** — Rust logic block\n\nDeclare state, props, memos, load functions, actions, server functions, and forms.",
        "template" => "**`<template>`** — HTML template block\n\nReactive HTML with control flow (`{#if}`, `{#each}`, `{#match}`, `{#can}`), expressions (`{expr}`), event bindings (`on:event`), and two-way bindings (`bind:field`).",
        "style" => "**`<style>`** — CSS styles\n\nAdd `scoped` for component-local styles or `global` for app-wide.",
        "resource" => "**`<resource>`** — Data model definition\n\nDefines fields with types and constraints, permissions, tenant scoping. Generates CRUD operations.",
        "layout" => "**`<layout>`** — Layout wrapper\n\nNamed layout with auth and tenant requirements. Referenced by routes.",
        "state" => "**`state`** — Reactive state field\n\n```\nstate name: Type = default\n```\n\nDeclares reactive state. Changes trigger template re-render via dynamic segments.",
        "prop" => "**`prop`** — Component property\n\n```\nprop name: Type\n```\n\nRead-only input from parent component.",
        "memo" => "**`memo`** — Computed value\n\n```\nmemo name: Type = expression\n```\n\nDerived from state, re-computed when dependencies change.",
        "load" => "**`load`** — Data loader\n\n```\nload async fn name(ctx: Ctx) { ... }\n```\n\nRuns on page load. Populates initial state from DB or API.",
        "action" => "**`action`** — Event handler\n\n```\n#[permission(\"...\")]\naction async fn name(params) { ... }\n```\n\nHandles user interactions. Can have permission and audit decorators.",
        "island" => "**`island`** — Component isolation\n\nMarks component for independent hydration. Only this component's JS ships to client.",
        _ => return None,
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: docs.to_string(),
        }),
        range: None,
    })
}

fn state_hover(ast: &adapto_parser::AdaptoFile, word: &str) -> Option<Hover> {
    let script = ast.script.as_ref()?;

    for s in &script.states {
        if s.name == word {
            let mut info = format!("**state** `{}: {}`", s.name, s.ty);
            if let Some(ref d) = s.default {
                info.push_str(&format!(" = `{}`", d));
            }
            if s.secret {
                info.push_str("\n\n⚠️ Secret — cannot be rendered in template");
            }
            return Some(make_hover(&info));
        }
    }

    for p in &script.props {
        if p.name == word {
            let info = format!("**prop** `{}: {}`", p.name, p.ty);
            return Some(make_hover(&info));
        }
    }

    for m in &script.memos {
        if m.name == word {
            let info = format!("**memo** `{}: {}` = `{}`", m.name, m.ty, m.expr);
            return Some(make_hover(&info));
        }
    }

    for a in &script.actions {
        if a.name == word {
            let params: Vec<String> = a.params.iter().map(|p| format!("{}: {}", p.name, p.ty)).collect();
            let mut info = format!("**action** `{}({})`", a.name, params.join(", "));
            if let Some(ref perm) = a.permission {
                info.push_str(&format!("\n\nPermission: `{}`", perm));
            }
            if a.is_async {
                info.push_str("\n\nasync");
            }
            return Some(make_hover(&info));
        }
    }

    None
}

fn make_hover(content: &str) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content.to_string(),
        }),
        range: None,
    }
}
```

- [ ] **Step 2: Register hover in main.rs**

Add `mod hover;` at top.

Add to `ServerCapabilities`:
```rust
hover_provider: Some(HoverProviderCapability::Simple(true)),
```

Add method:
```rust
async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let result = if let Some(doc) = self.documents.get(&uri) {
        hover::hover_info(&doc, pos)
    } else {
        None
    };
    Ok(result)
}
```

- [ ] **Step 3: Build**

```bash
cd ~/adapto-vscode
cargo build -p adapto-lsp
```

- [ ] **Step 4: Commit**

```bash
cd ~/adapto-vscode
git add crates/adapto-lsp/src/
git commit -m "feat: add hover provider for keywords and state fields"
```

---

## Task 7: Document Symbols

**Files:**
- Create: `~/adapto-vscode/crates/adapto-lsp/src/symbols.rs`
- Modify: `~/adapto-vscode/crates/adapto-lsp/src/main.rs`

- [ ] **Step 1: Create symbols.rs**

`~/adapto-vscode/crates/adapto-lsp/src/symbols.rs`:
```rust
use tower_lsp::lsp_types::*;

use crate::document::DocumentState;

#[allow(deprecated)]
pub fn document_symbols(doc: &DocumentState) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    let ast = match &doc.ast {
        Some(a) => a,
        None => return symbols,
    };

    if let Some(ref route) = ast.route {
        let mut children = Vec::new();
        if let Some(ref path) = route.path {
            children.push(make_property("path", path, SymbolKind::STRING));
        }
        if let Some(ref method) = route.method {
            children.push(make_property("method", method, SymbolKind::STRING));
        }
        symbols.push(DocumentSymbol {
            name: "route".to_string(),
            detail: route.path.clone(),
            kind: SymbolKind::MODULE,
            tags: None,
            deprecated: None,
            range: Range::default(),
            selection_range: Range::default(),
            children: Some(children),
        });
    }

    if let Some(ref script) = ast.script {
        let mut children = Vec::new();

        for s in &script.states {
            children.push(make_symbol(
                &s.name,
                Some(s.ty.clone()),
                SymbolKind::VARIABLE,
            ));
        }
        for p in &script.props {
            children.push(make_symbol(
                &p.name,
                Some(p.ty.clone()),
                SymbolKind::PROPERTY,
            ));
        }
        for m in &script.memos {
            children.push(make_symbol(
                &m.name,
                Some(format!("{} (memo)", m.ty)),
                SymbolKind::VARIABLE,
            ));
        }
        for l in &script.loaders {
            children.push(make_symbol(&l.name, Some("loader".to_string()), SymbolKind::FUNCTION));
        }
        for a in &script.actions {
            children.push(make_symbol(
                &a.name,
                Some("action".to_string()),
                SymbolKind::METHOD,
            ));
        }
        for sf in &script.server_fns {
            children.push(make_symbol(
                &sf.name,
                Some("server fn".to_string()),
                SymbolKind::FUNCTION,
            ));
        }
        for f in &script.forms {
            children.push(make_symbol(
                &f.name,
                Some("form".to_string()),
                SymbolKind::STRUCT,
            ));
        }

        symbols.push(DocumentSymbol {
            name: "script".to_string(),
            detail: None,
            kind: SymbolKind::NAMESPACE,
            tags: None,
            deprecated: None,
            range: Range::default(),
            selection_range: Range::default(),
            children: Some(children),
        });
    }

    if ast.template.is_some() {
        symbols.push(make_symbol("template", None, SymbolKind::NAMESPACE));
    }

    if let Some(ref style) = ast.style {
        let scope = if style.scoped { "scoped" } else { "global" };
        symbols.push(make_symbol("style", Some(scope.to_string()), SymbolKind::NAMESPACE));
    }

    if let Some(ref resource) = ast.resource {
        let mut children = Vec::new();
        for f in &resource.fields {
            children.push(make_symbol(
                &f.name,
                Some(f.ty.clone()),
                SymbolKind::FIELD,
            ));
        }
        symbols.push(DocumentSymbol {
            name: format!("resource ({})", resource.table),
            detail: Some(resource.name.clone()),
            kind: SymbolKind::CLASS,
            tags: None,
            deprecated: None,
            range: Range::default(),
            selection_range: Range::default(),
            children: Some(children),
        });
    }

    symbols
}

#[allow(deprecated)]
fn make_symbol(name: &str, detail: Option<String>, kind: SymbolKind) -> DocumentSymbol {
    DocumentSymbol {
        name: name.to_string(),
        detail,
        kind,
        tags: None,
        deprecated: None,
        range: Range::default(),
        selection_range: Range::default(),
        children: None,
    }
}

#[allow(deprecated)]
fn make_property(name: &str, value: &str, kind: SymbolKind) -> DocumentSymbol {
    DocumentSymbol {
        name: name.to_string(),
        detail: Some(value.to_string()),
        kind,
        tags: None,
        deprecated: None,
        range: Range::default(),
        selection_range: Range::default(),
        children: None,
    }
}
```

- [ ] **Step 2: Register in main.rs**

Add `mod symbols;` at top.

Add to `ServerCapabilities`:
```rust
document_symbol_provider: Some(OneOf::Left(true)),
```

Add method:
```rust
async fn document_symbol(
    &self,
    params: DocumentSymbolParams,
) -> Result<Option<DocumentSymbolResponse>> {
    let uri = params.text_document.uri;
    let syms = if let Some(doc) = self.documents.get(&uri) {
        symbols::document_symbols(&doc)
    } else {
        Vec::new()
    };
    Ok(Some(DocumentSymbolResponse::Nested(syms)))
}
```

- [ ] **Step 3: Build**

```bash
cd ~/adapto-vscode
cargo build -p adapto-lsp
```

- [ ] **Step 4: Commit**

```bash
cd ~/adapto-vscode
git add crates/adapto-lsp/src/
git commit -m "feat: add document symbols for outline view"
```

---

## Task 8: Go-to-Definition

**Files:**
- Create: `~/adapto-vscode/crates/adapto-lsp/src/definition.rs`
- Modify: `~/adapto-vscode/crates/adapto-lsp/src/main.rs`

- [ ] **Step 1: Create definition.rs**

`~/adapto-vscode/crates/adapto-lsp/src/definition.rs`:
```rust
use tower_lsp::lsp_types::*;

use crate::document::DocumentState;

pub fn goto_definition(doc: &DocumentState, position: Position, uri: &Url) -> Option<Location> {
    let line_idx = position.line as usize;
    let col = position.character as usize;
    let lines: Vec<&str> = doc.text.lines().collect();
    let current_line = lines.get(line_idx)?;

    let word = extract_word(current_line, col)?;

    let script = doc.ast.as_ref()?.script.as_ref()?;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with(&format!("state {}", word))
            || trimmed.starts_with(&format!("prop {}", word))
            || trimmed.starts_with(&format!("memo {}", word))
        {
            return Some(Location {
                uri: uri.clone(),
                range: Range {
                    start: Position { line: i as u32, character: 0 },
                    end: Position { line: i as u32, character: line.len() as u32 },
                },
            });
        }

        for a in &script.actions {
            if a.name == word && trimmed.contains(&format!("fn {}", word)) {
                return Some(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line: i as u32, character: 0 },
                        end: Position { line: i as u32, character: line.len() as u32 },
                    },
                });
            }
        }

        for l in &script.loaders {
            if l.name == word && trimmed.contains(&format!("fn {}", word)) {
                return Some(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line: i as u32, character: 0 },
                        end: Position { line: i as u32, character: line.len() as u32 },
                    },
                });
            }
        }

        for sf in &script.server_fns {
            if sf.name == word && trimmed.contains(&format!("fn {}", word)) {
                return Some(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position { line: i as u32, character: 0 },
                        end: Position { line: i as u32, character: line.len() as u32 },
                    },
                });
            }
        }
    }

    None
}

fn extract_word(line: &str, col: usize) -> Option<String> {
    let bytes = line.as_bytes();
    if col >= bytes.len() {
        return None;
    }

    let mut start = col;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }

    let mut end = col;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(line[start..end].to_string())
}
```

- [ ] **Step 2: Register in main.rs**

Add `mod definition;` at top.

Add to `ServerCapabilities`:
```rust
definition_provider: Some(OneOf::Left(true)),
```

Add method:
```rust
async fn goto_definition(
    &self,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let result = if let Some(doc) = self.documents.get(&uri) {
        definition::goto_definition(&doc, pos, &uri)
            .map(GotoDefinitionResponse::Scalar)
    } else {
        None
    };
    Ok(result)
}
```

- [ ] **Step 3: Build**

```bash
cd ~/adapto-vscode
cargo build -p adapto-lsp
```

- [ ] **Step 4: Commit**

```bash
cd ~/adapto-vscode
git add crates/adapto-lsp/src/
git commit -m "feat: add go-to-definition for state, props, actions, loaders"
```

---

## Task 9: Semantic Tokens

**Files:**
- Create: `~/adapto-vscode/crates/adapto-lsp/src/semantic_tokens.rs`
- Modify: `~/adapto-vscode/crates/adapto-lsp/src/main.rs`

- [ ] **Step 1: Create semantic_tokens.rs**

`~/adapto-vscode/crates/adapto-lsp/src/semantic_tokens.rs`:
```rust
use tower_lsp::lsp_types::*;

pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::VARIABLE,    // 0 - state
    SemanticTokenType::PROPERTY,    // 1 - prop
    SemanticTokenType::FUNCTION,    // 2 - action
    SemanticTokenType::KEYWORD,     // 3 - control flow
    SemanticTokenType::STRING,      // 4 - string
    SemanticTokenType::DECORATOR,   // 5 - decorator
    SemanticTokenType::TYPE,        // 6 - type name
    SemanticTokenType::NAMESPACE,   // 7 - block name
];

pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::READONLY,
    SemanticTokenModifier::ASYNC,
];

use crate::document::DocumentState;

pub fn semantic_tokens(doc: &DocumentState) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    let ast = match &doc.ast {
        Some(a) => a,
        None => return tokens,
    };

    let lines: Vec<&str> = doc.text.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let i = i as u32;
        let trimmed = line.trim();

        if trimmed.starts_with("state ") {
            if let Some(name_end) = trimmed[6..].find(':') {
                let name_start = line.find("state ").unwrap() as u32 + 6;
                push_token(&mut tokens, &mut prev_line, &mut prev_start, i, name_start, name_end as u32, 0, 0b001);
            }
        }

        if trimmed.starts_with("prop ") {
            if let Some(name_end) = trimmed[5..].find(':') {
                let name_start = line.find("prop ").unwrap() as u32 + 5;
                push_token(&mut tokens, &mut prev_line, &mut prev_start, i, name_start, name_end as u32, 1, 0b010);
            }
        }

        if trimmed.starts_with("memo ") {
            if let Some(name_end) = trimmed[5..].find(':') {
                let name_start = line.find("memo ").unwrap() as u32 + 5;
                push_token(&mut tokens, &mut prev_line, &mut prev_start, i, name_start, name_end as u32, 0, 0b010);
            }
        }

        if trimmed.contains("action ") && trimmed.contains("fn ") {
            if let Some(fn_pos) = trimmed.find("fn ") {
                let after_fn = &trimmed[fn_pos + 3..];
                if let Some(paren) = after_fn.find('(') {
                    let abs_start = line.find("fn ").unwrap() as u32 + 3;
                    push_token(&mut tokens, &mut prev_line, &mut prev_start, i, abs_start, paren as u32, 2, 0);
                }
            }
        }

        if trimmed.starts_with("#[permission") || trimmed.starts_with("#[audit") {
            let start = (line.len() - line.trim_start().len()) as u32;
            push_token(&mut tokens, &mut prev_line, &mut prev_start, i, start, trimmed.len() as u32, 5, 0);
        }
    }

    tokens
}

fn push_token(
    tokens: &mut Vec<SemanticToken>,
    prev_line: &mut u32,
    prev_start: &mut u32,
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
    token_modifiers: u32,
) {
    let delta_line = line - *prev_line;
    let delta_start = if delta_line == 0 {
        start - *prev_start
    } else {
        start
    };

    tokens.push(SemanticToken {
        delta_line,
        delta_start,
        length,
        token_type,
        token_modifiers_bitset: token_modifiers,
    });

    *prev_line = line;
    *prev_start = start;
}
```

- [ ] **Step 2: Register in main.rs**

Add `mod semantic_tokens;` at top.

Add to `ServerCapabilities`:
```rust
semantic_tokens_provider: Some(
    SemanticTokensServerCapabilities::SemanticTokensOptions(
        SemanticTokensOptions {
            legend: SemanticTokensLegend {
                token_types: semantic_tokens::TOKEN_TYPES.to_vec(),
                token_modifiers: semantic_tokens::TOKEN_MODIFIERS.to_vec(),
            },
            full: Some(SemanticTokensFullOptions::Bool(true)),
            range: None,
            ..Default::default()
        },
    ),
),
```

Add method:
```rust
async fn semantic_tokens_full(
    &self,
    params: SemanticTokensParams,
) -> Result<Option<SemanticTokensResult>> {
    let uri = params.text_document.uri;
    let tokens = if let Some(doc) = self.documents.get(&uri) {
        semantic_tokens::semantic_tokens(&doc)
    } else {
        Vec::new()
    };
    Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data: tokens,
    })))
}
```

- [ ] **Step 3: Build**

```bash
cd ~/adapto-vscode
cargo build -p adapto-lsp
```

- [ ] **Step 4: Commit**

```bash
cd ~/adapto-vscode
git add crates/adapto-lsp/src/
git commit -m "feat: add semantic token provider for rich syntax coloring"
```

---

## Task 10: Snippets File

**Files:**
- Create: `~/adapto-vscode/extension/snippets/adapto.json`
- Modify: `~/adapto-vscode/extension/package.json`

- [ ] **Step 1: Create snippets/adapto.json**

`~/adapto-vscode/extension/snippets/adapto.json`:
```json
{
  "Adapto Page": {
    "prefix": "apage",
    "body": [
      "<route>",
      "  path: \"$1\"",
      "  method: get",
      "  auth: ${2|public,required,verified|}",
      "</route>",
      "",
      "<script lang=\"rust\">",
      "  $3",
      "</script>",
      "",
      "<template>",
      "  $0",
      "</template>",
      "",
      "<style scoped>",
      "</style>"
    ],
    "description": "Full Adapto page scaffold"
  },
  "Route Block": {
    "prefix": "aroute",
    "body": [
      "<route>",
      "  path: \"$1\"",
      "  method: ${2|get,post,put,delete|}",
      "  layout: \"${3:main}\"",
      "  auth: ${4|public,required,verified|}",
      "</route>"
    ],
    "description": "Route metadata block"
  },
  "Script Block": {
    "prefix": "ascript",
    "body": [
      "<script lang=\"rust\">",
      "  state ${1:name}: ${2:String} = ${3:\"default\"}",
      "",
      "  load async fn load(ctx: Ctx) {",
      "    $0",
      "  }",
      "</script>"
    ],
    "description": "Script block with state and loader"
  },
  "Resource Block": {
    "prefix": "aresource",
    "body": [
      "<resource table=\"${1:items}\">",
      "  tenant: ${2|none,required,optional|}",
      "  primary_key: id",
      "",
      "  field ${3:name}: ${4:String} @required",
      "  $0",
      "",
      "  permission read: \"${1}.read\"",
      "  permission write: \"${1}.write\"",
      "</resource>"
    ],
    "description": "Resource data model"
  },
  "Layout Block": {
    "prefix": "alayout",
    "body": [
      "<layout name=\"${1:main}\">",
      "  auth: ${2|public,required|}",
      "</layout>"
    ],
    "description": "Layout wrapper"
  },
  "If Block": {
    "prefix": "aif",
    "body": [
      "{#if ${1:condition}}",
      "  $0",
      "{/if}"
    ],
    "description": "Conditional template block"
  },
  "If-Else Block": {
    "prefix": "aifelse",
    "body": [
      "{#if ${1:condition}}",
      "  $2",
      "{:else}",
      "  $0",
      "{/if}"
    ],
    "description": "Conditional with else branch"
  },
  "Each Block": {
    "prefix": "aeach",
    "body": [
      "{#each ${1:items} as ${2:item}}",
      "  $0",
      "{/each}"
    ],
    "description": "Loop over collection"
  },
  "Match Block": {
    "prefix": "amatch",
    "body": [
      "{#match ${1:expr}}",
      "  {:when ${2:pattern}}",
      "    $0",
      "{/match}"
    ],
    "description": "Pattern matching"
  },
  "Can Block": {
    "prefix": "acan",
    "body": [
      "{#can \"${1:permission}\"}",
      "  $0",
      "{/can}"
    ],
    "description": "Permission guard"
  },
  "Action Function": {
    "prefix": "aaction",
    "body": [
      "#[permission(\"${1:resource.action}\")]",
      "action async fn ${2:handle}(${3:params}: ${4:Type}) {",
      "  $0",
      "}"
    ],
    "description": "Action handler with permission"
  },
  "Form Struct": {
    "prefix": "aform",
    "body": [
      "form struct ${1:MyForm} {",
      "  ${2:field}: ${3:String},",
      "  $0",
      "}"
    ],
    "description": "Form validation struct"
  }
}
```

- [ ] **Step 2: Add snippet contribution to package.json**

Add to `contributes` in `package.json`:
```json
"snippets": [
  {
    "language": "adapto",
    "path": "./snippets/adapto.json"
  }
]
```

- [ ] **Step 3: Commit**

```bash
cd ~/adapto-vscode
git add extension/snippets/ extension/package.json
git commit -m "feat: add code snippets for all Adapto block types"
```

---

## Task 11: CLI Command Integration

**Files:**
- Create: `~/adapto-vscode/extension/src/commands.ts`
- Modify: `~/adapto-vscode/extension/src/extension.ts`
- Modify: `~/adapto-vscode/extension/package.json`

- [ ] **Step 1: Create commands.ts**

`~/adapto-vscode/extension/src/commands.ts`:
```typescript
import * as vscode from "vscode";

let devServerTerminal: vscode.Terminal | undefined;
let statusBarItem: vscode.StatusBarItem;

export function registerCommands(context: vscode.ExtensionContext) {
  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Left,
    100,
  );
  statusBarItem.command = "adapto.toggleDevServer";
  updateStatusBar(false);
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  context.subscriptions.push(
    vscode.commands.registerCommand("adapto.newProject", newProject),
    vscode.commands.registerCommand("adapto.startDevServer", () =>
      startDevServer(context),
    ),
    vscode.commands.registerCommand("adapto.stopDevServer", stopDevServer),
    vscode.commands.registerCommand("adapto.toggleDevServer", () =>
      toggleDevServer(context),
    ),
    vscode.commands.registerCommand("adapto.build", build),
    vscode.commands.registerCommand("adapto.check", check),
    vscode.commands.registerCommand("adapto.generateResource", generateResource),
    vscode.commands.registerCommand("adapto.showRoutes", showRoutes),
    vscode.commands.registerCommand("adapto.doctor", doctor),
  );
}

async function newProject() {
  const name = await vscode.window.showInputBox({
    prompt: "Project name",
    placeHolder: "my-app",
  });
  if (!name) return;

  const folder = await vscode.window.showOpenDialog({
    canSelectFolders: true,
    canSelectFiles: false,
    openLabel: "Select parent directory",
  });
  if (!folder?.[0]) return;

  const terminal = vscode.window.createTerminal("Adapto");
  terminal.show();
  terminal.sendText(`cd "${folder[0].fsPath}" && adapto new ${name}`);
}

function startDevServer(context: vscode.ExtensionContext) {
  if (devServerTerminal) {
    devServerTerminal.show();
    return;
  }

  devServerTerminal = vscode.window.createTerminal({
    name: "Adapto Dev",
    iconPath: new vscode.ThemeIcon("server"),
  });
  devServerTerminal.show();
  devServerTerminal.sendText("adapto dev");
  updateStatusBar(true);

  vscode.window.onDidCloseTerminal((t) => {
    if (t === devServerTerminal) {
      devServerTerminal = undefined;
      updateStatusBar(false);
    }
  });
}

function stopDevServer() {
  if (devServerTerminal) {
    devServerTerminal.dispose();
    devServerTerminal = undefined;
    updateStatusBar(false);
  }
}

function toggleDevServer(context: vscode.ExtensionContext) {
  if (devServerTerminal) {
    stopDevServer();
  } else {
    startDevServer(context);
  }
}

function build() {
  const terminal = vscode.window.createTerminal("Adapto Build");
  terminal.show();
  terminal.sendText("adapto build --release");
}

function check() {
  const terminal = vscode.window.createTerminal("Adapto Check");
  terminal.show();
  terminal.sendText("adapto check");
}

async function generateResource() {
  const name = await vscode.window.showInputBox({
    prompt: "Resource name (PascalCase)",
    placeHolder: "Customer",
  });
  if (!name) return;

  const terminal = vscode.window.createTerminal("Adapto Generate");
  terminal.show();
  terminal.sendText(`adapto generate resource ${name}`);
}

function showRoutes() {
  const terminal = vscode.window.createTerminal("Adapto Routes");
  terminal.show();
  terminal.sendText("adapto routes");
}

function doctor() {
  const terminal = vscode.window.createTerminal("Adapto Doctor");
  terminal.show();
  terminal.sendText("adapto doctor");
}

function updateStatusBar(running: boolean) {
  if (running) {
    statusBarItem.text = "$(server) Adapto: Running";
    statusBarItem.tooltip = "Click to stop dev server";
    statusBarItem.backgroundColor = undefined;
  } else {
    statusBarItem.text = "$(circle-slash) Adapto: Stopped";
    statusBarItem.tooltip = "Click to start dev server";
    statusBarItem.backgroundColor = undefined;
  }
}
```

- [ ] **Step 2: Add command contributions to package.json**

Add to `contributes` in `package.json`:
```json
"commands": [
  { "command": "adapto.newProject", "title": "Adapto: New Project" },
  { "command": "adapto.startDevServer", "title": "Adapto: Start Dev Server" },
  { "command": "adapto.stopDevServer", "title": "Adapto: Stop Dev Server" },
  { "command": "adapto.build", "title": "Adapto: Build" },
  { "command": "adapto.check", "title": "Adapto: Check" },
  { "command": "adapto.generateResource", "title": "Adapto: Generate Resource" },
  { "command": "adapto.showRoutes", "title": "Adapto: Show Routes" },
  { "command": "adapto.doctor", "title": "Adapto: Doctor" }
]
```

- [ ] **Step 3: Update extension.ts to register commands**

Add to `activate` in `extension.ts`:
```typescript
import { registerCommands } from "./commands";

// Inside activate():
registerCommands(context);
```

- [ ] **Step 4: Build extension**

```bash
cd ~/adapto-vscode/extension
npm run build
```

- [ ] **Step 5: Commit**

```bash
cd ~/adapto-vscode
git add extension/
git commit -m "feat: add CLI command integration with status bar and command palette"
```

---

## Task 12: Sidebar Tree View

**Files:**
- Create: `~/adapto-vscode/extension/src/sidebar.ts`
- Modify: `~/adapto-vscode/extension/src/extension.ts`
- Modify: `~/adapto-vscode/extension/package.json`

- [ ] **Step 1: Create sidebar.ts**

`~/adapto-vscode/extension/src/sidebar.ts`:
```typescript
import * as vscode from "vscode";
import * as path from "path";

export class AdaptoTreeProvider
  implements vscode.TreeDataProvider<AdaptoTreeItem>
{
  private _onDidChangeTreeData = new vscode.EventEmitter<
    AdaptoTreeItem | undefined
  >();
  readonly onDidChangeTreeData = this._onDidChangeTreeData.event;

  constructor(private workspaceRoot: string | undefined) {}

  refresh(): void {
    this._onDidChangeTreeData.fire(undefined);
  }

  getTreeItem(element: AdaptoTreeItem): vscode.TreeItem {
    return element;
  }

  async getChildren(element?: AdaptoTreeItem): Promise<AdaptoTreeItem[]> {
    if (!this.workspaceRoot) {
      return [];
    }

    if (!element) {
      return [
        new AdaptoTreeItem(
          "Routes",
          vscode.TreeItemCollapsibleState.Collapsed,
          "routes",
        ),
        new AdaptoTreeItem(
          "Components",
          vscode.TreeItemCollapsibleState.Collapsed,
          "components",
        ),
        new AdaptoTreeItem(
          "Resources",
          vscode.TreeItemCollapsibleState.Collapsed,
          "resources",
        ),
      ];
    }

    if (element.category === "components") {
      return this.findAdaptoFiles();
    }

    return [];
  }

  private async findAdaptoFiles(): Promise<AdaptoTreeItem[]> {
    const pattern = new vscode.RelativePattern(this.workspaceRoot!, "**/*.adapto");
    const files = await vscode.workspace.findFiles(pattern);

    return files.map((file) => {
      const rel = path.relative(this.workspaceRoot!, file.fsPath);
      const item = new AdaptoTreeItem(
        rel,
        vscode.TreeItemCollapsibleState.None,
        "file",
      );
      item.command = {
        command: "vscode.open",
        title: "Open",
        arguments: [file],
      };
      item.resourceUri = file;
      item.iconPath = new vscode.ThemeIcon("file-code");
      return item;
    });
  }
}

export class AdaptoTreeItem extends vscode.TreeItem {
  constructor(
    public readonly label: string,
    public readonly collapsibleState: vscode.TreeItemCollapsibleState,
    public readonly category: string,
  ) {
    super(label, collapsibleState);

    switch (category) {
      case "routes":
        this.iconPath = new vscode.ThemeIcon("symbol-interface");
        break;
      case "components":
        this.iconPath = new vscode.ThemeIcon("symbol-class");
        break;
      case "resources":
        this.iconPath = new vscode.ThemeIcon("database");
        break;
    }
  }
}

export function registerSidebar(context: vscode.ExtensionContext) {
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  const treeProvider = new AdaptoTreeProvider(workspaceRoot);

  vscode.window.registerTreeDataProvider("adaptoExplorer", treeProvider);

  context.subscriptions.push(
    vscode.commands.registerCommand("adapto.refreshExplorer", () =>
      treeProvider.refresh(),
    ),
  );

  const watcher = vscode.workspace.createFileSystemWatcher("**/*.adapto");
  watcher.onDidCreate(() => treeProvider.refresh());
  watcher.onDidDelete(() => treeProvider.refresh());
  watcher.onDidChange(() => treeProvider.refresh());
  context.subscriptions.push(watcher);
}
```

- [ ] **Step 2: Add viewsContainers and views to package.json**

Add to `contributes`:
```json
"viewsContainers": {
  "activitybar": [
    {
      "id": "adapto",
      "title": "Adapto",
      "icon": "icons/adapto-icon.png"
    }
  ]
},
"views": {
  "adapto": [
    {
      "id": "adaptoExplorer",
      "name": "Explorer"
    }
  ]
}
```

- [ ] **Step 3: Update extension.ts**

Add to imports and `activate`:
```typescript
import { registerSidebar } from "./sidebar";

// Inside activate():
registerSidebar(context);
```

- [ ] **Step 4: Create placeholder icon**

Create a simple 24x24 PNG icon at `~/adapto-vscode/extension/icons/adapto-icon.png`. For now, use any placeholder.

- [ ] **Step 5: Build and commit**

```bash
cd ~/adapto-vscode/extension
npm run build
cd ~/adapto-vscode
git add extension/
git commit -m "feat: add sidebar tree view with routes, components, resources sections"
```

---

## Task 13: WebView Preview Panel

**Files:**
- Create: `~/adapto-vscode/extension/src/preview.ts`
- Create: `~/adapto-vscode/extension/media/preview.css`
- Modify: `~/adapto-vscode/extension/src/extension.ts`
- Modify: `~/adapto-vscode/extension/package.json`

- [ ] **Step 1: Create preview.css**

`~/adapto-vscode/extension/media/preview.css`:
```css
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  padding: 16px;
  color: var(--vscode-foreground);
  background: var(--vscode-editor-background);
}

.error {
  background: var(--vscode-inputValidation-errorBackground);
  border: 1px solid var(--vscode-inputValidation-errorBorder);
  padding: 12px;
  border-radius: 4px;
  margin: 8px 0;
  font-family: monospace;
  white-space: pre-wrap;
}

.preview-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 0;
  border-bottom: 1px solid var(--vscode-panel-border);
  margin-bottom: 16px;
}

.preview-header h2 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
}

.route-info {
  font-size: 12px;
  color: var(--vscode-descriptionForeground);
  background: var(--vscode-badge-background);
  padding: 2px 8px;
  border-radius: 10px;
}

.preview-content {
  border: 1px solid var(--vscode-panel-border);
  border-radius: 4px;
  padding: 16px;
  min-height: 200px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 300px;
  color: var(--vscode-descriptionForeground);
}
```

- [ ] **Step 2: Create preview.ts**

`~/adapto-vscode/extension/src/preview.ts`:
```typescript
import * as vscode from "vscode";
import * as path from "path";

export class AdaptoPreviewProvider {
  private panel: vscode.WebviewPanel | undefined;
  private context: vscode.ExtensionContext;
  private disposables: vscode.Disposable[] = [];

  constructor(context: vscode.ExtensionContext) {
    this.context = context;
  }

  show() {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "adapto") {
      vscode.window.showWarningMessage("Open an .adapto file first");
      return;
    }

    if (this.panel) {
      this.panel.reveal(vscode.ViewColumn.Beside);
      this.update(editor.document);
      return;
    }

    this.panel = vscode.window.createWebviewPanel(
      "adaptoPreview",
      "Adapto Preview",
      vscode.ViewColumn.Beside,
      {
        enableScripts: true,
        localResourceRoots: [
          vscode.Uri.file(path.join(this.context.extensionPath, "media")),
        ],
      },
    );

    const cssUri = this.panel.webview.asWebviewUri(
      vscode.Uri.file(
        path.join(this.context.extensionPath, "media", "preview.css"),
      ),
    );

    this.panel.webview.html = this.getBaseHtml(cssUri);
    this.update(editor.document);

    this.panel.onDidDispose(() => {
      this.panel = undefined;
      this.disposables.forEach((d) => d.dispose());
      this.disposables = [];
    });

    this.disposables.push(
      vscode.workspace.onDidChangeTextDocument((e) => {
        if (
          e.document.languageId === "adapto" &&
          e.document === vscode.window.activeTextEditor?.document
        ) {
          this.update(e.document);
        }
      }),
    );

    this.disposables.push(
      vscode.window.onDidChangeActiveTextEditor((editor) => {
        if (editor?.document.languageId === "adapto") {
          this.update(editor.document);
        }
      }),
    );
  }

  private update(document: vscode.TextDocument) {
    if (!this.panel) return;

    const text = document.getText();
    const fileName = path.basename(document.fileName);

    const templateMatch = text.match(/<template>([\s\S]*?)<\/template>/);
    const routeMatch = text.match(/path:\s*"([^"]+)"/);
    const stateMatches = [...text.matchAll(/state\s+(\w+):\s+(\w+)\s*=\s*(.+)/g)];

    const templateContent = templateMatch?.[1]?.trim() || "";
    const routePath = routeMatch?.[1] || "/";

    let preview = templateContent;
    for (const [, name, , defaultVal] of stateMatches) {
      const cleaned = defaultVal.replace(/"/g, "").trim();
      preview = preview.replace(new RegExp(`\\{${name}\\}`, "g"), cleaned);
    }

    preview = preview
      .replace(/\{#if\s+[^}]+\}/g, "<!-- if -->")
      .replace(/\{:else(?:\s+if\s+[^}]+)?\}/g, "")
      .replace(/\{\/if\}/g, "<!-- /if -->")
      .replace(/\{#each\s+[^}]+\}/g, "<!-- each -->")
      .replace(/\{\/each\}/g, "<!-- /each -->")
      .replace(/\{#match\s+[^}]+\}/g, "<!-- match -->")
      .replace(/\{:when\s+[^}]+\}/g, "")
      .replace(/\{\/match\}/g, "<!-- /match -->")
      .replace(/\{#can\s+"[^"]+"\}/g, "")
      .replace(/\{\/can\}/g, "")
      .replace(/\{@html\s+([^}]+)\}/g, "$1")
      .replace(/\{[^}]+\}/g, '<span style="opacity:0.5">{...}</span>');

    this.panel.webview.postMessage({
      type: "update",
      fileName,
      routePath,
      content: preview,
    });
  }

  private getBaseHtml(cssUri: vscode.Uri): string {
    return `<!DOCTYPE html>
<html>
<head>
  <link rel="stylesheet" href="${cssUri}">
</head>
<body>
  <div class="preview-header">
    <h2 id="fileName">Preview</h2>
    <span class="route-info" id="routePath">/</span>
  </div>
  <div class="preview-content" id="content">
    <div class="empty-state">
      <p>Open an .adapto file to see preview</p>
    </div>
  </div>
  <script>
    window.addEventListener('message', event => {
      const msg = event.data;
      if (msg.type === 'update') {
        document.getElementById('fileName').textContent = msg.fileName;
        document.getElementById('routePath').textContent = msg.routePath;
        document.getElementById('content').innerHTML = msg.content;
      }
    });
  </script>
</body>
</html>`;
  }

  dispose() {
    this.panel?.dispose();
    this.disposables.forEach((d) => d.dispose());
  }
}

export function registerPreview(context: vscode.ExtensionContext) {
  const provider = new AdaptoPreviewProvider(context);

  context.subscriptions.push(
    vscode.commands.registerCommand("adapto.openPreview", () => provider.show()),
  );
}
```

- [ ] **Step 3: Add preview command to package.json**

Add to `commands` array:
```json
{
  "command": "adapto.openPreview",
  "title": "Adapto: Open Preview",
  "icon": "$(open-preview)"
}
```

Add `menus` to `contributes`:
```json
"menus": {
  "editor/title": [
    {
      "command": "adapto.openPreview",
      "when": "resourceLangId == adapto",
      "group": "navigation"
    }
  ]
}
```

- [ ] **Step 4: Update extension.ts**

Add:
```typescript
import { registerPreview } from "./preview";

// Inside activate():
registerPreview(context);
```

- [ ] **Step 5: Build and commit**

```bash
cd ~/adapto-vscode/extension
npm run build
cd ~/adapto-vscode
git add extension/
git commit -m "feat: add WebView preview panel for .adapto files"
```

---

## Task 14: Formatter

**Files:**
- Create: `~/adapto-vscode/crates/adapto-lsp/src/formatter.rs`
- Modify: `~/adapto-vscode/crates/adapto-lsp/src/main.rs`

- [ ] **Step 1: Create formatter.rs**

`~/adapto-vscode/crates/adapto-lsp/src/formatter.rs`:
```rust
use tower_lsp::lsp_types::*;

pub fn format_document(text: &str) -> Vec<TextEdit> {
    let mut formatted = String::new();
    let mut indent = 0usize;
    let mut in_style = false;
    let mut in_script = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            formatted.push('\n');
            continue;
        }

        if trimmed.starts_with("</style>") {
            in_style = false;
        }
        if trimmed.starts_with("</script>") {
            in_script = false;
        }

        if in_style || in_script {
            formatted.push_str(line);
            formatted.push('\n');
            continue;
        }

        if trimmed.starts_with("</") || trimmed.starts_with("{/") {
            indent = indent.saturating_sub(1);
        }

        if trimmed == "{:else}" || trimmed.starts_with("{:else ") || trimmed.starts_with("{:when ") {
            let temp_indent = indent.saturating_sub(1);
            formatted.push_str(&"  ".repeat(temp_indent));
            formatted.push_str(trimmed);
            formatted.push('\n');
        } else {
            formatted.push_str(&"  ".repeat(indent));
            formatted.push_str(trimmed);
            formatted.push('\n');
        }

        if (trimmed.starts_with('<') && !trimmed.starts_with("</") && !trimmed.ends_with("/>") && !trimmed.ends_with("-->"))
            || trimmed.starts_with("{#")
        {
            indent += 1;
        }

        if trimmed.starts_with("<style") {
            in_style = true;
        }
        if trimmed.starts_with("<script") {
            in_script = true;
        }
    }

    let line_count = text.lines().count() as u32;
    let last_line_len = text.lines().last().map(|l| l.len()).unwrap_or(0) as u32;

    vec![TextEdit {
        range: Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: line_count, character: last_line_len },
        },
        new_text: formatted,
    }]
}
```

- [ ] **Step 2: Register in main.rs**

Add `mod formatter;` at top.

Add to `ServerCapabilities`:
```rust
document_formatting_provider: Some(OneOf::Left(true)),
```

Add method:
```rust
async fn formatting(
    &self,
    params: DocumentFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let uri = params.text_document.uri;
    let edits = if let Some(doc) = self.documents.get(&uri) {
        formatter::format_document(&doc.text)
    } else {
        Vec::new()
    };
    Ok(Some(edits))
}
```

- [ ] **Step 3: Build**

```bash
cd ~/adapto-vscode
cargo build -p adapto-lsp
```

- [ ] **Step 4: Commit**

```bash
cd ~/adapto-vscode
git add crates/adapto-lsp/src/
git commit -m "feat: add document formatter for .adapto files"
```

---

## Task 15: CI/CD Pipelines

**Files:**
- Create: `~/adapto-vscode/.github/workflows/ci.yml`
- Create: `~/adapto-vscode/.github/workflows/release.yml`

- [ ] **Step 1: Create ci.yml**

`~/adapto-vscode/.github/workflows/ci.yml`:
```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  lsp:
    name: Build LSP
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build -p adapto-lsp
      - run: cargo test -p adapto-lsp

  extension:
    name: Build Extension
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
      - run: cd extension && npm ci
      - run: cd extension && npm run build
```

- [ ] **Step 2: Create release.yml**

`~/adapto-vscode/.github/workflows/release.yml`:
```yaml
name: Release
on:
  push:
    tags: ["v*"]

jobs:
  build-lsp:
    name: Build LSP - ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-apple-darwin
            os: macos-latest
            binary: adapto-lsp-darwin-x64
          - target: aarch64-apple-darwin
            os: macos-latest
            binary: adapto-lsp-darwin-arm64
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            binary: adapto-lsp-linux-x64
          - target: x86_64-pc-windows-msvc
            os: windows-latest
            binary: adapto-lsp-win32-x64.exe
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: Swatinem/rust-cache@v2
      - run: cargo build -p adapto-lsp --release --target ${{ matrix.target }}
      - name: Rename binary
        shell: bash
        run: |
          src="target/${{ matrix.target }}/release/adapto-lsp"
          if [[ "${{ matrix.os }}" == "windows-latest" ]]; then
            src="${src}.exe"
          fi
          cp "$src" "${{ matrix.binary }}"
      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.binary }}
          path: ${{ matrix.binary }}

  package:
    name: Package Extension
    needs: build-lsp
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20

      - uses: actions/download-artifact@v4
        with:
          path: extension/bin

      - name: Flatten binaries
        run: |
          cd extension/bin
          for dir in */; do
            mv "$dir"* .
            rmdir "$dir"
          done
          chmod +x adapto-lsp-*
          ls -la

      - run: cd extension && npm ci
      - run: npm install -g @vscode/vsce

      - name: Package .vsix
        run: cd extension && vsce package --no-dependencies

      - uses: actions/upload-artifact@v4
        with:
          name: adapto.vsix
          path: extension/*.vsix

      - name: Publish to Marketplace
        if: github.event_name == 'push'
        env:
          VSCE_PAT: ${{ secrets.VSCE_PAT }}
        run: cd extension && vsce publish --no-dependencies

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          files: |
            extension/*.vsix
            extension/bin/adapto-lsp-*
```

- [ ] **Step 3: Commit**

```bash
cd ~/adapto-vscode
git add .github/
git commit -m "ci: add CI and release workflows for cross-platform builds"
```

---

## Task 16: README and Final Polish

**Files:**
- Create: `~/adapto-vscode/README.md`

- [ ] **Step 1: Create README.md**

`~/adapto-vscode/README.md`:
```markdown
# Adapto for VS Code

Full IDE support for [Adapto](https://github.com/nickstukenov/adapto-core) `.adapto` files.

## Features

- **Syntax Highlighting** — Full TextMate grammar for all 6 block types with embedded Rust, CSS, and HTML
- **Diagnostics** — Real-time parse and compile errors via LSP
- **Autocomplete** — Context-aware completions for keywords, state fields, actions, event modifiers
- **Hover Info** — Documentation for keywords, type info for state/props/actions
- **Go to Definition** — Navigate to state, prop, memo, and action declarations
- **Document Symbols** — Outline view showing routes, state, actions, forms
- **Semantic Highlighting** — Rich coloring for state vs props vs actions vs decorators
- **Code Formatting** — Format .adapto files with proper indentation
- **Snippets** — Quick scaffolds: `apage`, `aroute`, `aif`, `aeach`, `aaction`, and more
- **CLI Integration** — Command palette and status bar for adapto dev/build/check/generate
- **Sidebar** — Project explorer showing routes, components, and resources
- **Live Preview** — WebView panel rendering template output

## Requirements

- VS Code 1.85+
- [Adapto CLI](https://github.com/nickstukenov/adapto-core) (for CLI commands)

## Quick Start

1. Install the extension
2. Open a folder containing `.adapto` files
3. Start editing — syntax highlighting, diagnostics, and completions activate automatically
4. Use `Ctrl+Shift+P` → "Adapto:" to access commands

## Development

```bash
# Build LSP server
cargo build -p adapto-lsp

# Build extension
cd extension && npm install && npm run build

# Run in VS Code
# Press F5 in VS Code to launch Extension Development Host
```
```

- [ ] **Step 2: Add launch.json for debugging**

Create `~/adapto-vscode/.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "name": "Run Extension",
      "type": "extensionHost",
      "request": "launch",
      "args": ["--extensionDevelopmentPath=${workspaceFolder}/extension"],
      "outFiles": ["${workspaceFolder}/extension/out/**/*.js"],
      "preLaunchTask": "npm: build"
    }
  ]
}
```

- [ ] **Step 3: Final commit**

```bash
cd ~/adapto-vscode
git add -A
git commit -m "docs: add README and VS Code debug configuration"
```

---

## Summary

| Task | Component | Commits |
|------|-----------|---------|
| 1 | Repo scaffolding | 1 |
| 2 | TextMate grammar | 1 |
| 3 | Document state management | 1 |
| 4 | LSP client (TypeScript) | 1 |
| 5 | Autocomplete | 1 |
| 6 | Hover provider | 1 |
| 7 | Document symbols | 1 |
| 8 | Go-to-definition | 1 |
| 9 | Semantic tokens | 1 |
| 10 | Snippets | 1 |
| 11 | CLI commands | 1 |
| 12 | Sidebar tree view | 1 |
| 13 | WebView preview | 1 |
| 14 | Formatter | 1 |
| 15 | CI/CD pipelines | 1 |
| 16 | README + polish | 1 |

**Total: 16 tasks, 16 commits**
