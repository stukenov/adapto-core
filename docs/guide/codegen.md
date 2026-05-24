# Generated Rust code

## Input DSL

```html
<script lang="rust">
  state count: i32 = 0

  action increment() {
    count += 1
  }
</script>

<template>
  <button on:click="increment">Count: {count}</button>
</template>
```

## Generated Rust concept

```rust
pub struct CounterState {
    pub count: i32,
}

impl Component for Counter {
    type State = CounterState;

    fn render(&self, state: &Self::State) -> Rendered {
        Rendered::new()
            .static_part("<button data-ar-click=\"increment\">")
            .dynamic_text("dyn_0", state.count.to_string(), deps!["count"])
            .static_part("</button>")
    }

    fn handle_event(&mut self, event: Event, state: &mut Self::State) -> Result<()> {
        match event.handler.as_str() {
            "increment" => {
                state.count += 1;
                mark_dirty!("count");
                Ok(())
            }
            _ => Err(Error::UnknownHandler)
        }
    }
}
```
