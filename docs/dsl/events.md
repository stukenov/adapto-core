# Event binding

```html
<button on:click="increment">+</button>
<input bind:value="query" on:input="search" />
<form on:submit="save">...</form>
```

## События

```txt
on:click
on:input
on:change
on:submit
on:keydown
on:keyup
on:blur
on:focus
```

## Modifiers

```html
<form on:submit.prevent="save">
<button on:click.debounce.300="search">
<input on:input.throttle.500="search" />
```
