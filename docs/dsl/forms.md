# Forms

Формы должны быть first-class entity.

```html
<form name="customerForm" schema="CustomerForm" on:submit="save">
  <Field name="name" label="Name" />
  <Field name="email" label="Email" />
  <Submit>Save</Submit>
</form>
```

## Schema

```rust
form CustomerForm {
  name: String min=2 max=120 required
  email: Email required
  phone: Option<String> max=32
}
```

## Генерация

Компилятор генерирует:

* Rust struct;
* validation logic;
* client hints;
* server-side validation;
* error mapping;
* audit metadata.
