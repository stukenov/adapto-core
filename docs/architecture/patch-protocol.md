# Patch protocol

## Client event

```json
{
  "v": 1,
  "type": "event",
  "session": "signed_session_id",
  "component": "counter_1",
  "event": "click",
  "handler": "increment",
  "payload": {},
  "seq": 12
}
```

## Form event

```json
{
  "v": 1,
  "type": "form_submit",
  "session": "signed_session_id",
  "component": "customer_form_1",
  "handler": "save",
  "form": {
    "name": "Acme",
    "email": "info@acme.kz"
  },
  "seq": 19
}
```

## Server patch

```json
{
  "v": 1,
  "type": "patch",
  "seq": 12,
  "ops": [
    {
      "op": "replace_text",
      "target": "dyn_42",
      "value": "43"
    },
    {
      "op": "replace_html",
      "target": "frag_91",
      "html": "<span>Saved</span>"
    }
  ]
}
```

## Supported patch operations

```txt
replace_text
replace_html
set_attr
remove_attr
add_class
remove_class
insert_before
insert_after
remove_node
focus
scroll_to
redirect
flash
modal_open
modal_close
```
