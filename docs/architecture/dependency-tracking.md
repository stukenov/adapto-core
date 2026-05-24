# Dependency tracking

Компилятор должен разбивать template на static и dynamic fragments.

Исходный template:

```html
<h1>{customer.name}</h1>
<p>{customer.email}</p>
<span>{counter}</span>
```

IR:

```json
{
  "static": ["<h1>", "</h1><p>", "</p><span>", "</span>"],
  "dynamic": [
    {"id": "dyn_0", "expr": "customer.name", "deps": ["customer.name"]},
    {"id": "dyn_1", "expr": "customer.email", "deps": ["customer.email"]},
    {"id": "dyn_2", "expr": "counter", "deps": ["counter"]}
  ]
}
```

Если изменился только `counter`, runtime отправляет patch только для `dyn_2`.
