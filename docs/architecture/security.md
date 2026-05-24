# Модель безопасности

## Escaping

Все `{expr}` escaped по умолчанию.

Raw HTML:

```html
{@html trusted_content}
```

Должно требовать explicit annotation:

```rust
state trusted_content: SafeHtml
```

## CSRF

Все form/actions должны проверять signed token.

## Event authorization

Каждый event проверяется:

```txt
session valid?
user authenticated?
tenant valid?
permission valid?
rate limit ok?
payload schema valid?
```

## Input validation

Все payloads валидируются через сгенерированную schema.

## Output policy

Secret state нельзя использовать в template.

```rust
state secret token: String
```

Компилятор должен выбросить ошибку:

```txt
E0421: secret state `token` cannot be rendered in template
```
