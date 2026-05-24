# DSL: обзор

## Цели

DSL должен быть:

* достаточно простым для бизнес-приложений;
* строгим и проверяемым;
* server-first;
* удобным для форм, таблиц, CRUD, workflow и AI-actions;
* безопасным по умолчанию;
* пригодным для генерации кода;
* расширяемым через Rust crates.

## Расширение файла

Основной формат:

```txt
*.adapto
```

Примеры:

```txt
app/page.adapto
app/dashboard/page.adapto
app/customers/[id]/page.adapto
app/customers/layout.adapto
components/Button.adapto
resources/Customer.adapto
```

## Структура `.adapto` файла

Базовая структура:

```html
<route>
  path: "/counter"
  layout: "main"
  auth: required
</route>

<script lang="rust">
  state count: i32 = 0

  action increment() {
    count += 1
  }
</script>

<template>
  <button on:click="increment">
    Count: {count}
  </button>
</template>

<style scoped>
  button {
    padding: 12px;
  }
</style>
```
