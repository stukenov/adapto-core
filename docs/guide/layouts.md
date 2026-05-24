# Layouts

## Layout file

```html
<layout name="dashboard">
  auth: required
  tenant: required
</layout>

<template>
  <AppShell>
    <Sidebar />
    <main>
      <slot />
    </main>
  </AppShell>
</template>
```

## Использование в page

```html
<route>
  path: "/customers"
  layout: "dashboard"
</route>
```
