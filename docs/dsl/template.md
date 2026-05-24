# `<template>`

HTML-like template с server-side expressions.

```html
<template>
  <section class="page">
    <h1>{customer.name}</h1>

    {#if customer.is_active}
      <Badge tone="success">Active</Badge>
    {:else}
      <Badge tone="muted">Inactive</Badge>
    {/if}

    <button on:click="save">Save</button>
  </section>
</template>
```

## Поддерживаемые конструкции

```txt
{expr}                  escaped expression
{@html expr}            unsafe raw HTML, requires explicit allow
{#if condition}         conditional rendering
{:else if condition}
{:else}
{/if}
{#each items as item}
{/each}
{#match value}
{/match}
<slot />
<Component prop={value} />
```
