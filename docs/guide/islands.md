# Islands

Интерактивные browser-only components должны быть явными.

```html
<Chart island data={sales_data} />
```

Или:

```html
<client:only component="RichTextEditor" props={editor_props} />
```

## Правила

* island получает сериализованные props;
* island не имеет прямого доступа к server state;
* server communication через actions/events;
* island должен быть изолирован security boundary.
