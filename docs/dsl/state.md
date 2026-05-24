# `state`

State хранится на сервере внутри LiveSession.

```rust
state count: i32 = 0
state query: String = ""
state customers: Vec<Customer> = []
state selected_id: Option<Uuid> = None
```

## Правила

* state доступен в template;
* изменение state помечает зависимые template fragments как dirty;
* state не сериализуется полностью в браузер без явного разрешения;
* sensitive state запрещено отдавать клиенту.

## Sensitive state

```rust
state secret api_key: String
```

Компилятор должен запретить использование `secret` state в template.

## `memo`

Derived state.

```rust
state price: Decimal = 100
state tax: Decimal = 12

memo total: Decimal = price + tax
```

`memo` пересчитывается только при изменении зависимостей.
