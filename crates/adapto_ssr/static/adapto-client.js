(function () {
  "use strict";

  // --- Bootstrap ---
  var el = document.getElementById("__ADAPTO_BOOTSTRAP__");
  if (!el) return;

  var config = JSON.parse(el.textContent);
  var sessionId = config.session_id;
  var csrfToken = config.csrf_token;
  var seq = 0;

  // --- WebSocket with exponential backoff ---
  var ws = null;
  var reconnectAttempt = 0;
  var maxReconnectDelay = 30000;
  var heartbeatInterval = null;
  var heartbeatMs = 25000;
  var connected = false;

  function nextSeq() {
    return ++seq;
  }

  function wsUrl() {
    var proto = location.protocol === "https:" ? "wss://" : "ws://";
    return proto + location.host + "/ws";
  }

  function connect() {
    if (ws && (ws.readyState === WebSocket.CONNECTING || ws.readyState === WebSocket.OPEN)) {
      return;
    }

    ws = new WebSocket(wsUrl());

    ws.onopen = function () {
      connected = true;
      reconnectAttempt = 0;

      // Send session init as first message
      ws.send(JSON.stringify({
        v: 1,
        type: "heartbeat",
        session: sessionId,
        seq: nextSeq()
      }));

      startHeartbeat();
    };

    ws.onmessage = function (e) {
      var msg;
      try {
        msg = JSON.parse(e.data);
      } catch (_) {
        return;
      }
      handleServerMessage(msg);
    };

    ws.onclose = function () {
      connected = false;
      stopHeartbeat();
      scheduleReconnect();
    };

    ws.onerror = function () {
      // onclose fires after onerror
    };
  }

  function scheduleReconnect() {
    var delay = Math.min(1000 * Math.pow(2, reconnectAttempt), maxReconnectDelay);
    delay += Math.random() * 1000; // jitter
    reconnectAttempt++;
    setTimeout(connect, delay);
  }

  function startHeartbeat() {
    stopHeartbeat();
    heartbeatInterval = setInterval(function () {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
          v: 1,
          type: "heartbeat",
          session: sessionId,
          seq: nextSeq()
        }));
      }
    }, heartbeatMs);
  }

  function stopHeartbeat() {
    if (heartbeatInterval) {
      clearInterval(heartbeatInterval);
      heartbeatInterval = null;
    }
  }

  // --- Send helpers ---
  function sendEvent(component, eventType, handler, payload) {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify({
      v: 1,
      type: "event",
      session: sessionId,
      component: component,
      event: eventType,
      handler: handler,
      payload: payload || {},
      seq: nextSeq()
    }));
  }

  function sendFormSubmit(component, handler, formData) {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify({
      v: 1,
      type: "form_submit",
      session: sessionId,
      component: component,
      handler: handler,
      form: formData,
      seq: nextSeq()
    }));
  }

  function sendNavigate(path) {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    ws.send(JSON.stringify({
      v: 1,
      type: "navigate",
      session: sessionId,
      path: path,
      seq: nextSeq()
    }));
  }

  // --- Server message handler ---
  function handleServerMessage(msg) {
    if (!msg || !msg.type) return;

    switch (msg.type) {
      case "patch":
        if (msg.ops && Array.isArray(msg.ops)) {
          for (var i = 0; i < msg.ops.length; i++) {
            applyPatch(msg.ops[i]);
          }
        }
        break;
      case "error":
        console.error("[adapto] server error:", msg.code, msg.message);
        break;
      case "redirect":
        if (msg.flash) {
          sessionStorage.setItem("__adapto_flash", JSON.stringify(msg.flash));
        }
        window.location.href = msg.url;
        break;
      case "heartbeat_ack":
        break;
    }
  }

  // --- All 15 PatchOp handlers ---
  function applyPatch(op) {
    switch (op.op) {
      case "replace_text":
        applyReplaceText(op.target, op.value);
        break;
      case "replace_html":
        applyReplaceHtml(op.target, op.html);
        break;
      case "set_attr":
        applySetAttr(op.target, op.name, op.value);
        break;
      case "remove_attr":
        applyRemoveAttr(op.target, op.name);
        break;
      case "add_class":
        applyAddClass(op.target, op.class);
        break;
      case "remove_class":
        applyRemoveClass(op.target, op.class);
        break;
      case "insert_before":
        applyInsertBefore(op.target, op.html);
        break;
      case "insert_after":
        applyInsertAfter(op.target, op.html);
        break;
      case "remove_node":
        applyRemoveNode(op.target);
        break;
      case "focus":
        applyFocus(op.target);
        break;
      case "scroll_to":
        applyScrollTo(op.target);
        break;
      case "redirect":
        window.location.href = op.url;
        break;
      case "flash":
        applyFlash(op.level, op.message);
        break;
      case "modal_open":
        applyModalOpen(op.id, op.html);
        break;
      case "modal_close":
        applyModalClose(op.id);
        break;
    }
  }

  function queryTarget(target) {
    return document.querySelector('[data-ar-dyn="' + target + '"]')
      || document.querySelector('[data-ar-el="' + target + '"]')
      || document.getElementById(target);
  }

  function applyReplaceText(target, value) {
    var node = queryTarget(target);
    if (node) node.textContent = value;
  }

  function applyReplaceHtml(target, html) {
    var node = queryTarget(target);
    if (node) node.innerHTML = html;
  }

  function applySetAttr(target, name, value) {
    var node = queryTarget(target);
    if (node) node.setAttribute(name, value);
  }

  function applyRemoveAttr(target, name) {
    var node = queryTarget(target);
    if (node) node.removeAttribute(name);
  }

  function applyAddClass(target, cls) {
    var node = queryTarget(target);
    if (node) node.classList.add(cls);
  }

  function applyRemoveClass(target, cls) {
    var node = queryTarget(target);
    if (node) node.classList.remove(cls);
  }

  function applyInsertBefore(target, html) {
    var node = queryTarget(target);
    if (!node) return;
    var tmp = document.createElement("div");
    tmp.innerHTML = html;
    while (tmp.firstChild) {
      node.parentNode.insertBefore(tmp.firstChild, node);
    }
  }

  function applyInsertAfter(target, html) {
    var node = queryTarget(target);
    if (!node) return;
    var tmp = document.createElement("div");
    tmp.innerHTML = html;
    var ref = node.nextSibling;
    while (tmp.firstChild) {
      node.parentNode.insertBefore(tmp.firstChild, ref);
    }
  }

  function applyRemoveNode(target) {
    var node = queryTarget(target);
    if (node && node.parentNode) node.parentNode.removeChild(node);
  }

  function applyFocus(target) {
    var node = queryTarget(target);
    if (node && typeof node.focus === "function") node.focus();
  }

  function applyScrollTo(target) {
    var node = queryTarget(target);
    if (node && typeof node.scrollIntoView === "function") {
      node.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }

  function applyFlash(level, message) {
    var container = document.getElementById("adapto-flash") || createFlashContainer();
    var flash = document.createElement("div");
    flash.className = "adapto-flash adapto-flash--" + level;
    flash.textContent = message;
    flash.setAttribute("role", "alert");

    var close = document.createElement("button");
    close.className = "adapto-flash__close";
    close.textContent = "×";
    close.setAttribute("aria-label", "Dismiss");
    close.onclick = function () {
      dismissFlash(flash);
    };
    flash.appendChild(close);
    container.appendChild(flash);

    setTimeout(function () {
      dismissFlash(flash);
    }, 5000);
  }

  function createFlashContainer() {
    var c = document.createElement("div");
    c.id = "adapto-flash";
    c.setAttribute("aria-live", "polite");
    c.style.cssText = "position:fixed;top:16px;right:16px;z-index:10000;display:flex;flex-direction:column;gap:8px;max-width:400px;";
    document.body.appendChild(c);
    return c;
  }

  function dismissFlash(node) {
    if (!node || !node.parentNode) return;
    node.style.opacity = "0";
    node.style.transition = "opacity 0.3s";
    setTimeout(function () {
      if (node.parentNode) node.parentNode.removeChild(node);
    }, 300);
  }

  function applyModalOpen(id, html) {
    var existing = document.getElementById("adapto-modal-" + id);
    if (existing) existing.parentNode.removeChild(existing);

    var overlay = document.createElement("div");
    overlay.id = "adapto-modal-" + id;
    overlay.className = "adapto-modal-overlay";
    overlay.setAttribute("role", "dialog");
    overlay.setAttribute("aria-modal", "true");
    overlay.style.cssText = "position:fixed;inset:0;background:rgba(0,0,0,0.5);display:flex;align-items:center;justify-content:center;z-index:10001;";

    var content = document.createElement("div");
    content.className = "adapto-modal-content";
    content.innerHTML = html;
    overlay.appendChild(content);

    overlay.addEventListener("click", function (e) {
      if (e.target === overlay) applyModalClose(id);
    });

    document.addEventListener("keydown", function handler(e) {
      if (e.key === "Escape") {
        applyModalClose(id);
        document.removeEventListener("keydown", handler);
      }
    });

    document.body.appendChild(overlay);
    var focusable = content.querySelector("button, [href], input, select, textarea, [tabindex]:not([tabindex=\"-1\"])");
    if (focusable) focusable.focus();
  }

  function applyModalClose(id) {
    var overlay = document.getElementById("adapto-modal-" + id);
    if (overlay && overlay.parentNode) {
      overlay.parentNode.removeChild(overlay);
    }
  }

  // --- Event delegation ---
  var eventTypes = ["click", "input", "change", "submit", "keydown", "keyup", "focus", "blur"];
  var debounceTimers = {};
  var throttleTimers = {};

  function findComponent(node) {
    var el = node;
    while (el) {
      if (el.hasAttribute && el.hasAttribute("data-ar-root")) {
        return el.getAttribute("data-ar-root");
      }
      el = el.parentElement;
    }
    return config.component_tree && config.component_tree.length > 0
      ? config.component_tree[0].id
      : "";
  }

  function parseModifiers(handler) {
    var parts = handler.split("|");
    var name = parts[0].trim();
    var mods = {};
    for (var i = 1; i < parts.length; i++) {
      var mod = parts[i].trim();
      var eqIdx = mod.indexOf("=");
      if (eqIdx > -1) {
        mods[mod.substring(0, eqIdx)] = mod.substring(eqIdx + 1);
      } else {
        mods[mod] = true;
      }
    }
    return { name: name, modifiers: mods };
  }

  function handleDelegatedEvent(e) {
    var type = e.type;
    var target = e.target;

    // Walk up from target to find data-ar-{type} attribute
    var node = target;
    while (node && node !== document) {
      var attrName = "data-ar-" + type;
      if (node.hasAttribute && node.hasAttribute(attrName)) {
        var rawHandler = node.getAttribute(attrName);
        var parsed = parseModifiers(rawHandler);
        var handler = parsed.name;
        var mods = parsed.modifiers;

        if (mods.prevent) e.preventDefault();
        if (mods.stop) e.stopPropagation();

        var component = findComponent(node);
        var payload = buildEventPayload(e, node, type);

        if (mods.debounce) {
          var delay = parseInt(mods.debounce, 10) || 300;
          var key = component + ":" + handler;
          clearTimeout(debounceTimers[key]);
          debounceTimers[key] = setTimeout(function () {
            sendEvent(component, type, handler, payload);
          }, delay);
          return;
        }

        if (mods.throttle) {
          var interval = parseInt(mods.throttle, 10) || 300;
          var tkey = component + ":" + handler;
          if (throttleTimers[tkey]) return;
          throttleTimers[tkey] = setTimeout(function () {
            throttleTimers[tkey] = null;
          }, interval);
        }

        // Handle form submit specially
        if (type === "submit") {
          e.preventDefault();
          var form = node.tagName === "FORM" ? node : node.closest("form");
          if (form) {
            var formData = serializeForm(form);
            sendFormSubmit(component, handler, formData);
            return;
          }
        }

        sendEvent(component, type, handler, payload);
        return;
      }
      node = node.parentElement;
    }
  }

  function buildEventPayload(e, node, type) {
    var payload = {};
    payload.target_id = node.getAttribute("data-ar-el") || node.id || "";

    if (type === "input" || type === "change") {
      if (node.type === "checkbox") {
        payload.value = node.checked;
      } else if (node.type === "radio") {
        payload.value = node.value;
        payload.checked = node.checked;
      } else if (node.tagName === "SELECT") {
        payload.value = node.value;
      } else {
        payload.value = node.value || "";
      }
    }

    if (type === "keydown" || type === "keyup") {
      payload.key = e.key;
      payload.code = e.code;
      payload.shift = e.shiftKey;
      payload.ctrl = e.ctrlKey;
      payload.alt = e.altKey;
      payload.meta = e.metaKey;
    }

    return payload;
  }

  // --- Form serialization ---
  function serializeForm(form) {
    var data = {};
    var elements = form.elements;
    for (var i = 0; i < elements.length; i++) {
      var field = elements[i];
      if (!field.name || field.disabled) continue;
      if (field.type === "file") continue;

      if (field.type === "checkbox") {
        data[field.name] = field.checked;
      } else if (field.type === "radio") {
        if (field.checked) data[field.name] = field.value;
      } else if (field.tagName === "SELECT" && field.multiple) {
        var selected = [];
        for (var j = 0; j < field.options.length; j++) {
          if (field.options[j].selected) selected.push(field.options[j].value);
        }
        data[field.name] = selected;
      } else {
        data[field.name] = field.value;
      }
    }
    return data;
  }

  // --- Register event delegation ---
  for (var i = 0; i < eventTypes.length; i++) {
    document.addEventListener(eventTypes[i], handleDelegatedEvent, true);
  }

  // --- Show persisted flash (after redirect) ---
  try {
    var flash = sessionStorage.getItem("__adapto_flash");
    if (flash) {
      sessionStorage.removeItem("__adapto_flash");
      var parsed = JSON.parse(flash);
      if (parsed && parsed.length === 2) {
        applyFlash(parsed[0], parsed[1]);
      }
    }
  } catch (_) {}

  // --- Connect ---
  connect();

  // --- Public API ---
  window.__adapto = {
    ws: null,
    config: config,
    connected: function () { return connected; },
    send: function (type, data) {
      if (type === "event") sendEvent(data.component, data.event, data.handler, data.payload);
      else if (type === "form") sendFormSubmit(data.component, data.handler, data.form);
      else if (type === "navigate") sendNavigate(data.path);
    }
  };

  Object.defineProperty(window.__adapto, "ws", {
    get: function () { return ws; }
  });

})();
