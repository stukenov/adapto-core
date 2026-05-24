# Components

## Component file

```html
<script lang="rust">
  prop tone: String = "default"
  prop label: String
</script>

<template>
  <span class="badge badge-{tone}">{label}</span>
</template>
```

## Использование

```html
<Badge tone="success" label="Active" />
```

## Slots

```html
<template>
  <button class="btn">
    <slot />
  </button>
</template>
```

Использование:

```html
<Button>Save</Button>
```
