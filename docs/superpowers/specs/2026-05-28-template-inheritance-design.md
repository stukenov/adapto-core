# Template Inheritance for .adapto

**Date:** 2026-05-28
**Status:** Draft
**Crates affected:** adapto_parser, adapto_compiler, adapto_ssr

## Overview

Add Svelte/Astro-style template inheritance to .adapto files. Layouts are full .adapto files with `<slot/>` placeholders. Pages and child layouts fill slots via `{#fill name}...{/fill}`. Multi-level chains supported (page → layout → parent layout).

## Syntax

### Layout file (`layouts/base.adapto`)

```html
<layout name="base">
  auth: public
</layout>

<script lang="rust">
  prop app_name: String = "Adapto"
  state user: Option<User>
</script>

<template>
  <html>
    <head>
      <slot name="head">
        <title>{app_name}</title>
      </slot>
    </head>
    <body>
      <nav>
        {#if user}
          <span>{user.name}</span>
        {/if}
      </nav>
      <main><slot/></main>
      <aside>
        <slot name="sidebar"/>
      </aside>
    </body>
  </html>
</template>
```

### Child layout inheriting from parent (`layouts/dashboard.adapto`)

```html
<layout name="dashboard">
  layout: "base"
</layout>

<template>
  {#fill head}
    <title>Dashboard</title>
    <link rel="stylesheet" href="/dashboard.css"/>
  {/fill}

  <div class="dashboard-grid">
    <slot name="panel"/>
    <slot/>
  </div>

  {#fill sidebar}
    <nav class="dashboard-nav">...</nav>
  {/fill}
</template>
```

### Page using a layout (`pages/stats.adapto`)

```html
<route>
  path: "/dashboard/stats"
  layout: "dashboard"
</route>

<template>
  {#fill panel}
    <div class="stats-panel">...</div>
  {/fill}

  <h1>Statistics</h1>
  <p>Main content here</p>
</template>
```

### Rendering chain

```
page(stats) → dashboard-layout → base-layout

1. Page renders: default content + named fills
2. Dashboard receives page fills into its slots, renders its own content + fills for base
3. Base receives dashboard fills into its slots, produces final HTML
```

## Slot semantics

- `<slot/>` — default (unnamed) slot. Receives all content not wrapped in `{#fill}`.
- `<slot name="X"/>` — named slot. Receives content from `{#fill X}...{/fill}`.
- `<slot name="X">fallback content</slot>` — fallback rendered when no `{#fill X}` provided.
- Fill completely replaces fallback. No `{#super}` / append mode.
- One default slot per layout. Multiple named slots allowed.

## Grammar changes

### New rule: `fill_block`

```pest
fill_block = { "{#fill" ~ ident ~ "}" ~ template_body ~ "{/fill}" }
```

Added to `template_node`:

```pest
template_node = {
    fill_block
  | if_block | each_block | match_block | can_block
  | unsafe_html | expression | element | component
  | slot_element | text_node
}
```

### Extended `layout_block`

Add `"layout"` to `layout_key`:

```pest
layout_key = { "auth" | "tenant" | "layout" }
```

## AST changes (`adapto_parser::ast`)

### New types

```rust
pub struct FillNode {
    pub slot_name: String,
    pub children: Vec<TemplateNode>,
}
```

### Modified types

```rust
pub enum TemplateNode {
    // ... existing variants
    Fill(FillNode),  // NEW
}

pub struct LayoutBlock {
    pub name: String,
    pub parent_layout: Option<String>,  // NEW
    pub auth: Option<AuthLevel>,
    pub tenant: Option<TenantLevel>,
}
```

## Compiler changes (`adapto_compiler`)

### New IR types

```rust
pub struct SlotPlaceholderIR {
    pub name: Option<String>,
    pub fallback: Option<SegmentBody>,
}

pub struct FillSegmentIR {
    pub slot_name: String,
    pub body: SegmentBody,
}
```

### ComponentIR extension

```rust
pub struct ComponentIR {
    // ... existing fields
    pub slot_placeholders: Vec<SlotPlaceholderIR>,  // NEW — for layouts
    pub fills: Vec<FillSegmentIR>,                   // NEW — for pages/child layouts
}
```

### Compile logic

- `TemplateNode::Slot` → compile fallback children into `SlotPlaceholderIR`
- `TemplateNode::Fill` → compile children into `FillSegmentIR`
- Layout files identified by presence of `<layout>` block (not `<route>`)

## SSR changes (`adapto_ssr`)

### LayoutManager rewrite

```rust
pub struct LayoutManager {
    layouts: HashMap<String, CompiledLayout>,
}

pub struct CompiledLayout {
    pub ir: ComponentIR,
    pub parent: Option<String>,
    pub slots: Vec<SlotDefinition>,
}

pub struct SlotDefinition {
    pub name: Option<String>,
    pub fallback_html: Option<String>,
}
```

### Layout resolution

```rust
impl LayoutManager {
    /// Resolve layout chain bottom-up. Returns [outermost, ..., innermost].
    /// Detects cycles. Max depth: 10.
    pub fn resolve_chain(&self, layout_name: &str) -> Result<Vec<&CompiledLayout>, SsrError>;

    /// Compose page into layout chain.
    /// 1. Render page template → extract fills + default content
    /// 2. For each layout (innermost first):
    ///    a. Replace default <slot/> with incoming default content
    ///    b. Replace named <slot name="X"/> with matching fill or fallback
    ///    c. Resulting HTML becomes "default content" for next (parent) layout
    ///    d. Layout's own {#fill} nodes become fills for parent
    pub fn compose(
        &self,
        chain: &[&CompiledLayout],
        page_fills: &HashMap<String, String>,
        page_default: &str,
        state: &StateStore,
    ) -> Result<String, SsrError>;
}
```

### Renderer integration

`Renderer::render_page()` updated:
1. Check if page has `layout` in route
2. If yes, resolve chain via LayoutManager
3. Render page template, separating fills from default content
4. Call `LayoutManager::compose()` with chain + fills + default
5. Wrap result in HTML document

## Error handling

| Error | When | Severity |
|-------|------|----------|
| Cycle detected | layout A → B → A | Compile error |
| Layout not found | `layout: "nonexistent"` | Compile error |
| Max depth exceeded | Chain > 10 levels | Compile error |
| Duplicate fill | Two `{#fill sidebar}` in same template | Compile error |
| Unknown fill target | `{#fill foo}` but layout has no `<slot name="foo">` | Compile warning |
| Unfilled slot (no fallback) | Named slot without fallback, no matching fill | Renders empty string |
| Fill without layout | `{#fill X}` in page that has no `layout:` | Compile warning, fill content ignored |

## Layout file discovery

Layouts resolved by name from a configured layouts directory:
- `layout: "base"` → looks for `layouts/base.adapto`
- `layout: "admin/dashboard"` → looks for `layouts/admin/dashboard.adapto`

The SSR `Project` struct already tracks source directories. Add `layouts_dir: PathBuf` to project config.

## Testing strategy

### Unit tests (adapto_parser)
- Parse `{#fill name}...{/fill}` → FillNode
- Parse extended `<layout>` with `layout: "parent"`
- Error on `{#fill}` without name
- Error on unclosed `{/fill}`

### Unit tests (adapto_compiler)
- Compile FillNode → FillSegmentIR
- Compile SlotNode with fallback → SlotPlaceholderIR
- Compile layout file (has `<layout>` block, not `<route>`)
- Security: secret state in layout template = error

### Unit tests (adapto_ssr)
- Single layout composition (page → layout)
- Multi-level composition (page → child → parent)
- Named slot fill
- Fallback when no fill provided
- Default slot receives non-fill content
- Cycle detection
- Max depth enforcement
- Missing layout error

### Integration tests
- End-to-end: parse → compile → render with layout chain
- Example: counter app with layout

## Non-goals (explicitly excluded)

- `{#super}` — no appending to parent slot content
- File-system convention (Next.js-style auto-discovery) — explicit `layout:` reference only
- Layout-level middleware/guards — handled by existing route auth/tenant
- Client-side layout transitions — SSR only for now
