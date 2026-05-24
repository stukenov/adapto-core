# Компоненты системы

## adapto_cli

CLI-инструмент для разработки, сборки и генерации проекта.

Команды:

```bash
adapto new my-app
adapto dev
adapto build
adapto check
adapto generate resource Customer
adapto routes
adapto doctor
```

Функции:

* запуск dev server;
* hot reload;
* компиляция DSL;
* проверка типов;
* генерация route manifest;
* генерация Rust-кода;
* генерация TypeScript client metadata;
* диагностика security/config ошибок.

## adapto_parser

Парсер DSL-файлов `.adapto`.

Вход:

```txt
app/dashboard/page.adapto
```

Выход:

```rust
ComponentAst
```

Парсер должен распознавать:

* frontmatter/config block;
* `<script lang="rust">`;
* `<template>`;
* `<style>`;
* state declarations;
* actions;
* server actions;
* permissions;
* forms;
* event bindings;
* components;
* slots;
* layouts;
* route metadata.

## adapto_compiler

Компилятор превращает AST в intermediate representation.

Выходные артефакты:

```txt
.target/adapto/
  manifest/routes.json
  manifest/components.json
  generated/rust/*.rs
  generated/client/*.json
  assets/client.js
```

Компилятор должен строить:

* component tree;
* dependency graph;
* static/dynamic HTML segments;
* event map;
* action map;
* form schema;
* permission map;
* patch targets;
* hydration/island metadata.

## adapto_ssr

SSR-модуль отвечает за первичный HTTP-render.

Функции:

* route matching;
* layout composition;
* server-side data loading;
* HTML rendering;
* streaming response в будущем;
* injection of client runtime;
* initial state snapshot;
* signed session bootstrap.

Первичный HTML должен содержать:

```html
<div data-ar-root="page_01">
  ...rendered HTML...
</div>
<script type="application/json" id="__ADAPTO_BOOTSTRAP__">
  {...signed bootstrap payload...}
</script>
<script src="/assets/adapto-client.js"></script>
```

## adapto_live

Live runtime управляет активными browser sessions.

Каждая открытая вкладка получает LiveSession:

```rust
struct LiveSession {
    id: SessionId,
    user_id: Option<UserId>,
    tenant_id: Option<TenantId>,
    route: RouteId,
    root_component: ComponentId,
    state: StateStore,
    dirty: DirtySet,
    socket: LiveSocket,
    permissions: PermissionSet,
    audit: AuditSink,
}
```

Функции LiveSession:

* accept event;
* validate event;
* check permission;
* run handler;
* mutate state;
* mark dirty fields;
* render changed fragments;
* send patch;
* write audit event;
* recover on reconnect.

## adapto_client

Минимальный JS-клиент в браузере.

Функции:

* WebSocket connect/reconnect;
* event delegation;
* form serialization;
* patch apply;
* optimistic UI optional;
* file upload optional;
* focus preservation;
* scroll preservation;
* input cursor preservation;
* error overlay в dev mode.

Клиент не должен содержать бизнес-логику.
