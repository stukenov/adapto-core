# Adapto Live Runtime

## Назначение

Adapto Live Runtime — это server-first web framework для бизнес-приложений, CRM, админок, кабинетов, workflow-систем и AI-enabled enterprise SaaS.

Цель системы — дать разработчику опыт уровня Svelte/Phoenix LiveView, но с глубоким Rust-ядром:

* компоненты описываются декларативным DSL;
* первичный HTML рендерится на сервере;
* состояние страницы живет на сервере;
* браузер отправляет события через WebSocket;
* сервер обновляет state и возвращает только HTML/DOM patches;
* клиентский JS остается минимальным;
* безопасность, роли, tenant, audit и формы встроены в framework по умолчанию.

Система не должна быть клоном React, Next.js или Svelte. Она должна быть узким secure-by-default application framework для enterprise-продуктов.

## Базовая формула

```txt
Phoenix LiveView idea
+ Svelte-like compiler
+ Rust actor/session runtime
+ server-first forms/actions
+ tiny browser client
+ enterprise defaults
```

## Основные принципы

### Server-first by default

По умолчанию весь UI рендерится на сервере. Браузер получает HTML и маленький клиентский runtime для событий и patching.

Client-side JS используется только там, где явно указан interactive/island-компонент.

### State lives on server

Состояние страницы хранится в Rust LiveSession. Браузер не является источником истины.

```txt
Browser event → Rust LiveSession → state update → render diff → browser patch
```

### Compiler-driven UI

Framework должен компилировать `.adapto` DSL в Rust render-функции, route manifest, dependency graph и client-side metadata.

### No Virtual DOM by default

Не использовать React-like Virtual DOM как центральную модель. Вместо этого:

* template AST;
* static/dynamic segmentation;
* dependency tracking;
* dirty state;
* targeted patch generation.

### Secure by default

Встроенные механизмы:

* CSRF protection;
* strict form validation;
* RBAC;
* tenant isolation;
* audit log;
* default escaping;
* safe HTML opt-in only;
* server-side permissions check;
* signed session/channel IDs;
* rate limits на события;
* schema-driven input validation.

## Итоговое позиционирование

Adapto Live Runtime — это не frontend framework.

Это:

```txt
secure server-rendered application runtime
for enterprise SaaS, CRM, admin panels and AI workflows
```

Короткая формула:

```txt
Svelte-like DX
Phoenix LiveView-like interaction model
Rust-native backend/runtime
enterprise defaults
AI-native actions
```

Основная ставка: не победить React/Next.js на массовом рынке, а создать более строгий, безопасный и производительный framework для бизнес-приложений, где сервер должен контролировать state, permissions, audit, tenant и AI-операции.

## Навигация по документации

- [Архитектура](architecture/overview.md) — высокоуровневая архитектура системы
- [Компоненты системы](architecture/crates.md) — описание crates
- [Patch protocol](architecture/patch-protocol.md) — протокол обновлений
- [Dependency tracking](architecture/dependency-tracking.md) — отслеживание зависимостей
- [Безопасность](architecture/security.md) — модель безопасности
- [DSL обзор](dsl/overview.md) — цели и структура DSL
- [DSL блоки](dsl/) — route, script, state, actions, template, events, forms, permissions, tenant, audit, resource, ai-actions
- [Руководство](guide/) — routing, layouts, components, islands, data loading, error boundaries, database, codegen
- [Операции](operations/) — dev server, observability, deployment, configuration
- [Примеры](examples/) — customer page, lesson tracker, compiler errors
- [Проект](project/) — roadmap, non-goals, rust stack, risks
