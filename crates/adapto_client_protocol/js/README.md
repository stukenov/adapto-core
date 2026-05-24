# Adapto Client Runtime

Browser-side runtime for the Adapto Live Runtime framework. Zero dependencies, no bundler required.

## Installation

Include directly in your HTML. The server typically injects this automatically:

```html
<script src="/_adapto/js/adapto-client.js"></script>
```

For production, use the minified build (generate via `terser adapto-client.js -o adapto-client.min.js -c -m`).

## Bootstrap

The runtime reads its configuration from an inline JSON element rendered by the server:

```html
<script id="__ADAPTO_BOOTSTRAP__" type="application/json">
{
  "session_id": "sess-abc-123",
  "websocket_url": "ws://localhost:3000/_adapto/live",
  "csrf_token": "tok_xyz",
  "initial_state_hash": "sha256:abcdef",
  "component_tree": [
    {
      "id": "c:0",
      "name": "App",
      "dynamic_targets": [{ "id": "title", "deps": ["page_title"] }]
    }
  ]
}
</script>
```

If `websocket_url` is omitted, the client derives it from the current page location (`ws://` or `wss://` + host + `/_adapto/live`).

Enable dev mode by adding `data-adapto-dev` to `<html>`:

```html
<html data-adapto-dev>
```

## Data Attributes

All event binding uses `data-ar-*` attributes. No JavaScript is needed in templates.

### Event Binding

| Attribute | Event | Example |
|---|---|---|
| `data-ar-click` | click | `<button data-ar-click="increment">+1</button>` |
| `data-ar-input` | input | `<input data-ar-input="search">` |
| `data-ar-change` | change | `<select data-ar-change="set_role">` |
| `data-ar-submit` | submit | `<form data-ar-submit="create_user">` |
| `data-ar-keydown` | keydown | `<input data-ar-keydown="handle_key">` |
| `data-ar-keyup` | keyup | `<input data-ar-keyup="handle_key">` |
| `data-ar-focus` | focus | `<input data-ar-focus="on_focus">` |
| `data-ar-blur` | blur | `<input data-ar-blur="on_blur">` |

### Modifiers

| Attribute | Purpose | Example |
|---|---|---|
| `data-ar-debounce` | Debounce input events (ms) | `<input data-ar-input="search" data-ar-debounce="300">` |
| `data-ar-throttle` | Throttle input events (ms) | `<input data-ar-input="resize" data-ar-throttle="100">` |
| `data-ar-key` | Filter keyboard events by key | `<input data-ar-keydown="submit" data-ar-key="Enter">` |

### Component Identity

| Attribute | Purpose |
|---|---|
| `data-ar-root` | Root component container. Value = component ID. |
| `data-ar-component` | Nested component container. Value = component ID. |
| `data-ar-id` | Stable element ID for patch targeting. |
| `data-ar-dyn` | Dynamic text/HTML target for `replace_text` and `replace_html` patches. |

### Navigation

```html
<a href="/dashboard" data-ar-nav>Dashboard</a>
```

Links with `data-ar-nav` use client-side navigation (pushState) instead of full page loads.

## Wire Protocol

### Client to Server

All messages include `v: 1` (protocol version) and `type` (discriminator).

**Event:**
```json
{
  "v": 1,
  "type": "event",
  "session": "sess-abc-123",
  "component": "counter",
  "event": "click",
  "handler": "increment",
  "payload": {},
  "seq": 1
}
```

**Form Submit:**
```json
{
  "v": 1,
  "type": "form_submit",
  "session": "sess-abc-123",
  "component": "signup_form",
  "handler": "submit_signup",
  "form": { "name": "Alice", "email": "alice@example.com" },
  "seq": 5
}
```

**Navigate:**
```json
{
  "v": 1,
  "type": "navigate",
  "session": "sess-abc-123",
  "path": "/dashboard",
  "seq": 10
}
```

**Heartbeat:**
```json
{
  "v": 1,
  "type": "heartbeat",
  "session": "sess-abc-123",
  "seq": 42
}
```

### Server to Client

**Patch (DOM updates):**
```json
{
  "v": 1,
  "type": "patch",
  "seq": 1,
  "ops": [
    { "op": "replace_text", "target": "c:counter#count", "value": "42" },
    { "op": "add_class", "target": "#btn", "class": "active" }
  ]
}
```

**Patch Operations:**

| Op | Fields | Description |
|---|---|---|
| `replace_text` | `target`, `value` | Set element's `textContent` |
| `replace_html` | `target`, `html` | Set element's `innerHTML` |
| `set_attr` | `target`, `name`, `value` | Set an attribute |
| `remove_attr` | `target`, `name` | Remove an attribute |
| `add_class` | `target`, `class` | Add a CSS class |
| `remove_class` | `target`, `class` | Remove a CSS class |
| `insert_before` | `target`, `html` | Insert HTML before element |
| `insert_after` | `target`, `html` | Insert HTML after element |
| `remove_node` | `target` | Remove element from DOM |
| `focus` | `target` | Move focus to element |
| `scroll_to` | `target` | Scroll element into view |
| `redirect` | `url` | Navigate to URL |
| `flash` | `level`, `message` | Show flash notification |
| `modal_open` | `id`, `html` | Open a modal dialog |
| `modal_close` | `id` | Close a modal dialog |

**Error:**
```json
{
  "v": 1,
  "type": "error",
  "seq": 7,
  "code": "INVALID_HANDLER",
  "message": "Handler 'foo' not found on component 'bar'"
}
```

**Redirect:**
```json
{
  "v": 1,
  "type": "redirect",
  "url": "/login",
  "flash": ["warning", "Please sign in"]
}
```

**Heartbeat Ack:**
```json
{
  "v": 1,
  "type": "heartbeat_ack",
  "seq": 42
}
```

## Public API

The runtime exposes `window.Adapto` for programmatic access:

```javascript
// Send a custom event
Adapto.sendEvent("my-component", "click", "my_handler", { key: "value" });

// Client-side navigation
Adapto.navigate("/settings");

// Connection management
Adapto.isConnected();   // boolean
Adapto.reconnect();     // force reconnect
Adapto.disconnect();    // graceful close

// Session
Adapto.getSessionId();  // string

// Flash messages
Adapto.flash("success", "Saved successfully");

// Protocol version
Adapto.PROTOCOL_VERSION; // 1
```

## Lifecycle Events

The runtime dispatches `CustomEvent`s on `document`:

| Event | Detail | When |
|---|---|---|
| `adapto:connected` | `{}` | WebSocket opened |
| `adapto:disconnected` | `{}` | WebSocket closed |
| `adapto:reconnecting` | `{ attempt, delay }` | Reconnect scheduled |
| `adapto:reconnect_failed` | `{}` | Max retries exhausted |
| `adapto:patched` | `{ seq, count }` | DOM patches applied |
| `adapto:error` | `{ code, message, seq }` | Server error received |

```javascript
document.addEventListener("adapto:connected", function() {
  console.log("Connected to server");
});
```

## Focus Preservation

During DOM patching, the runtime automatically preserves:

- Currently focused element (matched by `data-ar-id` or DOM `id`)
- Text cursor position (`selectionStart`, `selectionEnd`, `selectionDirection`)
- Scroll position within the focused element (e.g., scrolled textarea)

## Reconnection

On unexpected disconnect, the client reconnects with exponential backoff:

- Base delay: 1000ms
- Max delay: 30000ms (capped)
- Jitter: 0-500ms random
- Max attempts: 10

After exhausting retries, the dev-mode error overlay shows a "reload" message.
