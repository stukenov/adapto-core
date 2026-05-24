# `<style>`

Блок стилей для компонента или страницы.

```html
<style scoped>
  .page {
    max-width: 960px;
    margin: 0 auto;
    padding: 24px;
  }

  h1 {
    font-size: 1.5rem;
    font-weight: 600;
  }

  button {
    padding: 8px 16px;
    border-radius: 4px;
    cursor: pointer;
  }
</style>
```

## Атрибуты

```txt
scoped        стили ограничены текущим компонентом (по умолчанию)
global        стили применяются глобально
```

## Scoped стили

По умолчанию `<style scoped>`. Компилятор добавляет уникальный selector prefix, чтобы стили не пересекались между компонентами.

```html
<style scoped>
  .card {
    border: 1px solid #e0e0e0;
  }
</style>
```

Компилятор генерирует:

```css
[data-adapto-abc123] .card {
  border: 1px solid #e0e0e0;
}
```

## Global стили

Для общих стилей (reset, typography, variables):

```html
<style global>
  :root {
    --color-primary: #2563eb;
    --color-danger: #dc2626;
    --radius: 6px;
  }
</style>
```

## CSS variables

Рекомендуется использовать CSS custom properties для theme:

```html
<style scoped>
  button {
    background: var(--color-primary);
    border-radius: var(--radius);
    color: white;
  }
</style>
```

## Ограничения

* `@import` запрещен внутри scoped style;
* селекторы по id (`#`) не рекомендуются;
* стили не могут обращаться к state напрямую -- для динамических стилей используется `class:` или `style:` binding в template.

## Динамические классы

Динамические стили задаются в template через class binding:

```html
<template>
  <div class:active={is_active} class:error={has_error}>
    ...
  </div>
</template>

<style scoped>
  .active {
    border-color: var(--color-primary);
  }
  .error {
    border-color: var(--color-danger);
  }
</style>
```
