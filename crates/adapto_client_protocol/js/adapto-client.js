/**
 * Adapto Client Runtime v1
 * ========================
 * Browser-side runtime for the Adapto Live Runtime framework.
 *
 * Responsibilities:
 *   - WebSocket connection with exponential-backoff reconnection
 *   - Event delegation (click, input, change, submit, keydown, keyup, blur, focus)
 *   - Form serialization
 *   - DOM patch application with focus/scroll/cursor preservation
 *   - Debounce and throttle support on input events
 *   - Heartbeat keep-alive
 *   - Flash messages, modal open/close
 *   - Dev-mode error overlay
 *   - Client-side navigation (pushState)
 *
 * No dependencies. No bundler required. Include via <script> tag.
 *
 * Security note: innerHTML is used only for server-rendered content.
 * The server (our own Rust backend) is the sole source of HTML patches.
 * No user-controlled strings are ever passed to innerHTML.
 *
 * Protocol version: 1
 * Wire format: JSON over WebSocket, matching adapto_client_protocol Rust crate.
 */
(function () {
  "use strict";

  // =========================================================================
  // Configuration
  // =========================================================================

  var PROTOCOL_VERSION = 1;
  var DEFAULT_WS_PATH = "/_adapto/live";
  var HEARTBEAT_INTERVAL = 30000;
  var RECONNECT_MAX_RETRIES = 10;
  var RECONNECT_BASE_DELAY = 1000;

  // =========================================================================
  // State
  // =========================================================================

  var socket = null;
  var wsUrl = null;
  var sessionId = null;
  var csrfToken = null;
  var initialStateHash = null;
  var componentTree = null;
  var seq = 0;
  var connected = false;
  var reconnectAttempts = 0;
  var heartbeatTimer = null;
  var devMode = false;

  // Timers for debounce/throttle keyed by "componentId:handler"
  var debounceTimers = {};
  var throttleTimers = {};

  // =========================================================================
  // Bootstrap
  // =========================================================================

  /**
   * Read the inline bootstrap payload and start the runtime.
   * The server renders a <script id="__ADAPTO_BOOTSTRAP__" type="application/json">
   * element containing session_id, websocket_url, csrf_token, etc.
   */
  function init() {
    var bootstrapEl = document.getElementById("__ADAPTO_BOOTSTRAP__");
    if (!bootstrapEl) {
      console.warn("[Adapto] No bootstrap element found. Skipping initialization.");
      return;
    }

    try {
      var payload = JSON.parse(bootstrapEl.textContent);
      sessionId = payload.session_id;
      csrfToken = payload.csrf_token;
      initialStateHash = payload.initial_state_hash || null;
      componentTree = payload.component_tree || [];
      wsUrl = payload.websocket_url || buildWsUrl();
      devMode = document.documentElement.hasAttribute("data-adapto-dev");

      connect(wsUrl);
      bindEvents();
      checkStoredFlash();
    } catch (e) {
      console.error("[Adapto] Bootstrap failed:", e);
      if (devMode) {
        showErrorOverlay("Bootstrap Error", e.message);
      }
    }
  }

  /**
   * Derive a WebSocket URL from the current page location when none is
   * provided in the bootstrap payload.
   */
  function buildWsUrl() {
    var proto = location.protocol === "https:" ? "wss:" : "ws:";
    return proto + "//" + location.host + DEFAULT_WS_PATH;
  }

  // =========================================================================
  // WebSocket Connection
  // =========================================================================

  function connect(url) {
    if (socket) {
      try { socket.close(); } catch (_) { /* noop */ }
    }

    socket = new WebSocket(url);

    socket.onopen = function () {
      connected = true;
      reconnectAttempts = 0;
      startHeartbeat();
      hideErrorOverlay();
      dispatchAdaptoEvent("adapto:connected");
    };

    socket.onmessage = function (event) {
      handleServerMessage(event.data);
    };

    socket.onclose = function (event) {
      connected = false;
      stopHeartbeat();
      dispatchAdaptoEvent("adapto:disconnected");

      if (!event.wasClean) {
        scheduleReconnect(url);
      }
    };

    socket.onerror = function () {
      // The close handler will fire next; we just log here.
      console.error("[Adapto] WebSocket error");
    };
  }

  function scheduleReconnect(url) {
    if (reconnectAttempts >= RECONNECT_MAX_RETRIES) {
      console.error("[Adapto] Max reconnect attempts reached (" + RECONNECT_MAX_RETRIES + ")");
      if (devMode) {
        showErrorOverlay(
          "Connection Lost",
          "Unable to reconnect after " + RECONNECT_MAX_RETRIES + " attempts. Reload the page."
        );
      }
      dispatchAdaptoEvent("adapto:reconnect_failed");
      return;
    }

    var delay = Math.min(RECONNECT_BASE_DELAY * Math.pow(2, reconnectAttempts), 30000);
    var jitter = Math.floor(Math.random() * 500);
    reconnectAttempts++;

    dispatchAdaptoEvent("adapto:reconnecting", { attempt: reconnectAttempts, delay: delay });
    setTimeout(function () { connect(url); }, delay + jitter);
  }

  /**
   * Send a JSON message over the WebSocket. Silently drops if disconnected.
   */
  function send(msg) {
    if (!connected || !socket || socket.readyState !== WebSocket.OPEN) {
      return false;
    }
    socket.send(JSON.stringify(msg));
    return true;
  }

  // =========================================================================
  // Heartbeat
  // =========================================================================

  function startHeartbeat() {
    stopHeartbeat();
    heartbeatTimer = setInterval(function () {
      send({
        v: PROTOCOL_VERSION,
        type: "heartbeat",
        session: sessionId,
        seq: ++seq
      });
    }, HEARTBEAT_INTERVAL);
  }

  function stopHeartbeat() {
    if (heartbeatTimer !== null) {
      clearInterval(heartbeatTimer);
      heartbeatTimer = null;
    }
  }

  // =========================================================================
  // Event Delegation
  // =========================================================================

  /**
   * Bind all delegated event listeners on the document. Each listener
   * walks up from the event target looking for the corresponding
   * `data-ar-*` attribute.
   */
  function bindEvents() {
    document.addEventListener("click", onClickDelegate, false);
    document.addEventListener("input", onInputDelegate, false);
    document.addEventListener("change", onChangeDelegate, false);
    document.addEventListener("submit", onSubmitDelegate, false);
    document.addEventListener("keydown", onKeydownDelegate, false);
    document.addEventListener("keyup", onKeyupDelegate, false);
    document.addEventListener("focus", onFocusDelegate, true);   // capture phase
    document.addEventListener("blur", onBlurDelegate, true);     // capture phase

    // Client-side navigation: intercept clicks on <a data-ar-nav>
    document.addEventListener("click", onNavLinkDelegate, false);

    // Browser back/forward
    window.addEventListener("popstate", onPopState, false);
  }

  // -- Click ----------------------------------------------------------------

  function onClickDelegate(e) {
    var el = e.target.closest("[data-ar-click]");
    if (!el) return;

    e.preventDefault();
    var handler = el.getAttribute("data-ar-click");
    var componentId = findComponentId(el);
    sendEvent(componentId, "click", handler, {});
  }

  // -- Input ----------------------------------------------------------------

  function onInputDelegate(e) {
    var el = e.target.closest("[data-ar-input]");
    if (!el) return;

    var handler = el.getAttribute("data-ar-input");
    var componentId = findComponentId(el);
    var value = el.value;

    // Debounce
    var debounceMs = parseInt(el.getAttribute("data-ar-debounce") || "0", 10);
    if (debounceMs > 0) {
      var key = componentId + ":" + handler;
      clearTimeout(debounceTimers[key]);
      debounceTimers[key] = setTimeout(function () {
        sendEvent(componentId, "input", handler, { value: value });
      }, debounceMs);
      return;
    }

    // Throttle
    var throttleMs = parseInt(el.getAttribute("data-ar-throttle") || "0", 10);
    if (throttleMs > 0) {
      var tkey = componentId + ":" + handler;
      if (throttleTimers[tkey]) return;
      throttleTimers[tkey] = setTimeout(function () {
        throttleTimers[tkey] = null;
      }, throttleMs);
    }

    sendEvent(componentId, "input", handler, { value: value });
  }

  // -- Change ---------------------------------------------------------------

  function onChangeDelegate(e) {
    var el = e.target.closest("[data-ar-change]");
    if (!el) return;

    var handler = el.getAttribute("data-ar-change");
    var componentId = findComponentId(el);
    var value;

    if (el.type === "checkbox") {
      value = el.checked;
    } else if (el.type === "select-multiple") {
      value = Array.from(el.selectedOptions).map(function (o) { return o.value; });
    } else {
      value = el.value;
    }

    sendEvent(componentId, "change", handler, { value: value });
  }

  // -- Submit ---------------------------------------------------------------

  function onSubmitDelegate(e) {
    var form = e.target.closest("[data-ar-submit]");
    if (!form) return;

    e.preventDefault();
    var handler = form.getAttribute("data-ar-submit");
    var componentId = findComponentId(form);
    var formData = serializeForm(form);

    send({
      v: PROTOCOL_VERSION,
      type: "form_submit",
      session: sessionId,
      component: componentId,
      handler: handler,
      form: formData,
      seq: ++seq
    });
  }

  // -- Keyboard -------------------------------------------------------------

  function onKeydownDelegate(e) {
    var el = e.target.closest("[data-ar-keydown]");
    if (!el) return;
    handleKeyEvent(el, e, "keydown");
  }

  function onKeyupDelegate(e) {
    var el = e.target.closest("[data-ar-keyup]");
    if (!el) return;
    handleKeyEvent(el, e, "keyup");
  }

  function handleKeyEvent(el, e, type) {
    var handler = el.getAttribute("data-ar-" + type);
    var componentId = findComponentId(el);

    // Optional key filter: data-ar-key="Enter" only fires on that key
    var keyFilter = el.getAttribute("data-ar-key");
    if (keyFilter && e.key !== keyFilter) return;

    sendEvent(componentId, type, handler, {
      key: e.key,
      code: e.code,
      shift: e.shiftKey,
      ctrl: e.ctrlKey,
      alt: e.altKey,
      meta: e.metaKey
    });
  }

  // -- Focus / Blur ---------------------------------------------------------

  function onFocusDelegate(e) {
    var el = e.target.closest("[data-ar-focus]");
    if (!el) return;
    var handler = el.getAttribute("data-ar-focus");
    var componentId = findComponentId(el);
    sendEvent(componentId, "focus", handler, {});
  }

  function onBlurDelegate(e) {
    var el = e.target.closest("[data-ar-blur]");
    if (!el) return;
    var handler = el.getAttribute("data-ar-blur");
    var componentId = findComponentId(el);
    sendEvent(componentId, "blur", handler, {});
  }

  // -- Navigation -----------------------------------------------------------

  function onNavLinkDelegate(e) {
    var link = e.target.closest("a[data-ar-nav]");
    if (!link) return;

    var href = link.getAttribute("href");
    if (!href || href.charAt(0) !== "/") return;

    e.preventDefault();
    navigateTo(href);
  }

  function onPopState() {
    sendNavigate(location.pathname);
  }

  function navigateTo(path) {
    history.pushState(null, "", path);
    sendNavigate(path);
  }

  function sendNavigate(path) {
    send({
      v: PROTOCOL_VERSION,
      type: "navigate",
      session: sessionId,
      path: path,
      seq: ++seq
    });
  }

  // =========================================================================
  // Event Envelope
  // =========================================================================

  /**
   * Send a client event message matching the Rust ClientPayload::Event shape.
   *
   * Wire format:
   * {
   *   "v": 1,
   *   "type": "event",
   *   "session": "...",
   *   "component": "...",
   *   "event": "click",
   *   "handler": "increment",
   *   "payload": { ... },
   *   "seq": 42
   * }
   */
  function sendEvent(componentId, eventType, handler, payload) {
    send({
      v: PROTOCOL_VERSION,
      type: "event",
      session: sessionId,
      component: componentId,
      event: eventType,
      handler: handler,
      payload: payload || {},
      seq: ++seq
    });
  }

  // =========================================================================
  // Form Serialization
  // =========================================================================

  /**
   * Serialize all named, enabled form elements into a flat object.
   * Matches the HashMap<String, serde_json::Value> expected by FormSubmitEvent.
   */
  function serializeForm(form) {
    var data = {};
    var elements = form.elements;

    for (var i = 0; i < elements.length; i++) {
      var el = elements[i];
      if (!el.name || el.disabled) continue;

      switch (el.type) {
        case "checkbox":
          data[el.name] = el.checked;
          break;
        case "radio":
          if (el.checked) data[el.name] = el.value;
          break;
        case "select-multiple":
          data[el.name] = Array.from(el.selectedOptions).map(function (o) { return o.value; });
          break;
        case "number":
        case "range":
          data[el.name] = el.value === "" ? null : parseFloat(el.value);
          break;
        case "file":
          // File uploads are out of scope for the WebSocket transport.
          break;
        default:
          data[el.name] = el.value;
      }
    }

    return data;
  }

  // =========================================================================
  // Server Message Handling
  // =========================================================================

  /**
   * Parse and route an incoming server message. The `type` tag is used
   * for dispatch, mirroring the Rust ServerPayload enum.
   */
  function handleServerMessage(raw) {
    var msg;
    try {
      msg = JSON.parse(raw);
    } catch (e) {
      console.error("[Adapto] Malformed server message:", e);
      return;
    }

    switch (msg.type) {
      case "patch":
        applyPatches(msg.ops, msg.seq);
        break;
      case "error":
        handleError(msg);
        break;
      case "redirect":
        handleRedirect(msg);
        break;
      case "heartbeat_ack":
        // Connection is alive. Could measure RTT: Date.now() - sentAt[msg.seq]
        break;
      default:
        console.warn("[Adapto] Unknown server message type:", msg.type);
    }
  }

  // =========================================================================
  // Patch Application
  // =========================================================================

  /**
   * Apply an ordered array of patch operations to the DOM.
   * Focus, scroll position, and text cursor are preserved across the batch.
   */
  function applyPatches(ops, patchSeq) {
    if (!ops || !ops.length) return;

    var focusState = saveFocusState();

    for (var i = 0; i < ops.length; i++) {
      applyPatch(ops[i]);
    }

    restoreFocusState(focusState);
    dispatchAdaptoEvent("adapto:patched", { seq: patchSeq, count: ops.length });
  }

  /**
   * Apply a single patch operation. The `op` field maps to PatchOp variant
   * names in the Rust enum (serde tag = "op").
   *
   * Note: innerHTML is used intentionally for server-authored HTML.
   * The Rust backend is the sole producer of these HTML strings --
   * no user input reaches innerHTML without server-side escaping.
   */
  function applyPatch(op) {
    switch (op.op) {
      case "replace_text":
        patchReplaceText(op.target, op.value);
        break;
      case "replace_html":
        patchReplaceHtml(op.target, op.html);
        break;
      case "set_attr":
        patchSetAttr(op.target, op.name, op.value);
        break;
      case "remove_attr":
        patchRemoveAttr(op.target, op.name);
        break;
      case "add_class":
        patchAddClass(op.target, op["class"]);
        break;
      case "remove_class":
        patchRemoveClass(op.target, op["class"]);
        break;
      case "insert_before":
        patchInsertBefore(op.target, op.html);
        break;
      case "insert_after":
        patchInsertAfter(op.target, op.html);
        break;
      case "remove_node":
        patchRemoveNode(op.target);
        break;
      case "focus":
        patchFocus(op.target);
        break;
      case "scroll_to":
        patchScrollTo(op.target);
        break;
      case "redirect":
        window.location.href = op.url;
        break;
      case "flash":
        showFlash(op.level, op.message);
        break;
      case "modal_open":
        openModal(op.id, op.html);
        break;
      case "modal_close":
        closeModal(op.id);
        break;
      default:
        console.warn("[Adapto] Unknown patch op:", op.op);
    }
  }

  // -- Patch Helpers --------------------------------------------------------

  /**
   * Resolve a target string to a DOM element.
   *
   * Resolution order:
   *   1. data-ar-dyn="<target>"  (dynamic text/html targets)
   *   2. data-ar-id="<target>"   (stable element IDs)
   *   3. Plain CSS selector      (fallback for simple selectors like "#id")
   */
  function resolveTarget(target, preferDyn) {
    if (preferDyn) {
      var dynEl = document.querySelector('[data-ar-dyn="' + cssEscape(target) + '"]');
      if (dynEl) return dynEl;
    }

    var byId = document.querySelector('[data-ar-id="' + cssEscape(target) + '"]');
    if (byId) return byId;

    // Fallback: treat target as CSS selector for convenience
    try {
      return document.querySelector(target);
    } catch (_) {
      return null;
    }
  }

  function patchReplaceText(target, value) {
    var el = resolveTarget(target, true);
    if (el) el.textContent = value;
  }

  function patchReplaceHtml(target, html) {
    var el = resolveTarget(target, true);
    // Server-authored HTML only; see security note at applyPatch
    if (el) el.innerHTML = html; // eslint-disable-line no-unsanitized/property
  }

  function patchSetAttr(target, name, value) {
    var el = resolveTarget(target, false);
    if (!el) return;

    // Special-case: setting "value" on an input should also update the property
    if (name === "value" && ("value" in el)) {
      el.value = value;
    }
    el.setAttribute(name, value);
  }

  function patchRemoveAttr(target, name) {
    var el = resolveTarget(target, false);
    if (el) el.removeAttribute(name);
  }

  function patchAddClass(target, cls) {
    var el = resolveTarget(target, false);
    if (el) el.classList.add(cls);
  }

  function patchRemoveClass(target, cls) {
    var el = resolveTarget(target, false);
    if (el) el.classList.remove(cls);
  }

  function patchInsertBefore(target, html) {
    var el = resolveTarget(target, false);
    // Server-authored HTML only; see security note at applyPatch
    if (el) el.insertAdjacentHTML("beforebegin", html); // eslint-disable-line no-unsanitized/method
  }

  function patchInsertAfter(target, html) {
    var el = resolveTarget(target, false);
    // Server-authored HTML only; see security note at applyPatch
    if (el) el.insertAdjacentHTML("afterend", html); // eslint-disable-line no-unsanitized/method
  }

  function patchRemoveNode(target) {
    var el = resolveTarget(target, false);
    if (el && el.parentNode) el.parentNode.removeChild(el);
  }

  function patchFocus(target) {
    var el = resolveTarget(target, false);
    if (el) el.focus();
  }

  function patchScrollTo(target) {
    var el = resolveTarget(target, false);
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }

  // =========================================================================
  // Focus / Scroll / Cursor Preservation
  // =========================================================================

  /**
   * Capture the currently focused element's state so it can be restored
   * after a DOM patch that might replace or re-render the element.
   */
  function saveFocusState() {
    var active = document.activeElement;
    if (!active || active === document.body || active === document.documentElement) {
      return null;
    }

    var state = {
      arId: active.getAttribute("data-ar-id"),
      domId: active.id || null,
      tag: active.tagName,
      scrollTop: active.scrollTop,
      scrollLeft: active.scrollLeft,
      selectionStart: null,
      selectionEnd: null,
      selectionDirection: null
    };

    // Capture text cursor position for text inputs and textareas
    if (typeof active.selectionStart === "number") {
      try {
        state.selectionStart = active.selectionStart;
        state.selectionEnd = active.selectionEnd;
        state.selectionDirection = active.selectionDirection;
      } catch (_) {
        // Some input types (date, color, etc.) throw on selectionStart access
      }
    }

    return state;
  }

  /**
   * Restore focus, cursor position, and scroll offset captured by saveFocusState.
   */
  function restoreFocusState(state) {
    if (!state) return;

    var el = null;
    if (state.arId) {
      el = document.querySelector('[data-ar-id="' + cssEscape(state.arId) + '"]');
    }
    if (!el && state.domId) {
      el = document.getElementById(state.domId);
    }
    if (!el) return;

    // Only restore focus if the element is still focusable
    el.focus({ preventScroll: true });

    // Restore text cursor
    if (state.selectionStart !== null && typeof el.setSelectionRange === "function") {
      try {
        el.setSelectionRange(state.selectionStart, state.selectionEnd, state.selectionDirection);
      } catch (_) {
        // Not all focused input types support setSelectionRange
      }
    }

    // Restore internal scroll position (e.g., textarea scrolled down)
    if (typeof state.scrollTop === "number") {
      el.scrollTop = state.scrollTop;
      el.scrollLeft = state.scrollLeft;
    }
  }

  // =========================================================================
  // Flash Messages
  // =========================================================================

  /**
   * Display a transient flash notification.
   *
   * The `level` string matches FlashLevel from Rust: "success", "info",
   * "warning", "danger".
   */
  function showFlash(level, message) {
    var container = document.getElementById("adapto-flash");
    if (!container) {
      container = document.createElement("div");
      container.id = "adapto-flash";
      container.setAttribute("role", "status");
      container.setAttribute("aria-live", "polite");
      container.style.cssText =
        "position:fixed;top:1rem;right:1rem;z-index:9999;" +
        "display:flex;flex-direction:column;gap:0.5rem;" +
        "pointer-events:none;max-width:24rem;";
      document.body.appendChild(container);
    }

    var flash = document.createElement("div");
    flash.className = "adapto-flash adapto-flash--" + level;
    flash.setAttribute("role", "alert");
    flash.style.cssText =
      "pointer-events:auto;padding:0.75rem 1rem;border-radius:0.5rem;" +
      "font-size:0.875rem;line-height:1.4;cursor:pointer;" +
      "opacity:0;transform:translateY(-0.5rem);" +
      "transition:opacity 0.2s ease,transform 0.2s ease;" +
      colorForLevel(level);
    flash.textContent = message;

    flash.addEventListener("click", function () {
      dismissFlash(flash);
    });

    container.appendChild(flash);

    // Trigger enter animation on next frame
    requestAnimationFrame(function () {
      flash.style.opacity = "1";
      flash.style.transform = "translateY(0)";
    });

    // Auto-dismiss after 5 seconds
    setTimeout(function () {
      dismissFlash(flash);
    }, 5000);
  }

  function dismissFlash(flash) {
    flash.style.opacity = "0";
    flash.style.transform = "translateY(-0.5rem)";
    setTimeout(function () {
      if (flash.parentNode) flash.parentNode.removeChild(flash);
    }, 200);
  }

  function colorForLevel(level) {
    switch (level) {
      case "success": return "background:#ecfdf5;color:#065f46;border:1px solid #a7f3d0;";
      case "info":    return "background:#eff6ff;color:#1e40af;border:1px solid #bfdbfe;";
      case "warning": return "background:#fffbeb;color:#92400e;border:1px solid #fde68a;";
      case "danger":  return "background:#fef2f2;color:#991b1b;border:1px solid #fecaca;";
      default:        return "background:#f9fafb;color:#374151;border:1px solid #e5e7eb;";
    }
  }

  /**
   * Check sessionStorage for a flash message stored before a redirect.
   */
  function checkStoredFlash() {
    var stored = sessionStorage.getItem("adapto_flash");
    if (!stored) return;

    sessionStorage.removeItem("adapto_flash");
    try {
      var parsed = JSON.parse(stored);
      // Flash stored as a [level, message] tuple (matching Rust's Option<(FlashLevel, String)>)
      if (Array.isArray(parsed) && parsed.length === 2) {
        showFlash(parsed[0], parsed[1]);
      }
    } catch (_) {
      // Corrupt flash data; discard silently.
    }
  }

  // =========================================================================
  // Modal
  // =========================================================================

  function openModal(id, html) {
    closeModal(id);

    var overlay = document.createElement("div");
    overlay.id = "adapto-modal-" + id;
    overlay.className = "adapto-modal-overlay";
    overlay.setAttribute("role", "dialog");
    overlay.setAttribute("aria-modal", "true");
    overlay.style.cssText =
      "position:fixed;inset:0;background:rgba(0,0,0,0.4);" +
      "display:flex;align-items:center;justify-content:center;" +
      "z-index:10000;backdrop-filter:blur(4px);-webkit-backdrop-filter:blur(4px);";

    var content = document.createElement("div");
    content.className = "adapto-modal-content";
    content.style.cssText =
      "background:#fff;border-radius:0.75rem;padding:1.5rem;" +
      "max-width:36rem;width:calc(100% - 2rem);max-height:80vh;" +
      "overflow:auto;box-shadow:0 25px 50px -12px rgba(0,0,0,0.25);";
    // Server-authored HTML only; see security note at top of file
    content.innerHTML = html; // eslint-disable-line no-unsanitized/property

    overlay.appendChild(content);

    // Click outside to close
    overlay.addEventListener("click", function (e) {
      if (e.target === overlay) closeModal(id);
    });

    // Escape to close
    overlay.addEventListener("keydown", function (e) {
      if (e.key === "Escape") closeModal(id);
    });

    document.body.appendChild(overlay);

    // Trap focus inside modal
    var firstFocusable = content.querySelector(
      'a[href],button:not([disabled]),input:not([disabled]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])'
    );
    if (firstFocusable) firstFocusable.focus();
  }

  function closeModal(id) {
    var overlay = document.getElementById("adapto-modal-" + id);
    if (overlay && overlay.parentNode) {
      overlay.parentNode.removeChild(overlay);
    }
  }

  // =========================================================================
  // Error Handling
  // =========================================================================

  function handleError(msg) {
    console.error("[Adapto] Server error [" + msg.code + "]:", msg.message);
    if (devMode) {
      showErrorOverlay("Error: " + msg.code, msg.message);
    }
    dispatchAdaptoEvent("adapto:error", { code: msg.code, message: msg.message, seq: msg.seq });
  }

  function handleRedirect(msg) {
    // Persist flash for after navigation, matching the Rust tuple format
    if (msg.flash) {
      sessionStorage.setItem("adapto_flash", JSON.stringify(msg.flash));
    }
    window.location.href = msg.url;
  }

  // =========================================================================
  // Dev-Mode Error Overlay
  // =========================================================================

  function showErrorOverlay(title, message) {
    hideErrorOverlay();

    var overlay = document.createElement("div");
    overlay.id = "adapto-error-overlay";
    overlay.setAttribute("role", "alert");
    overlay.style.cssText =
      "position:fixed;bottom:0;left:0;right:0;" +
      "background:#1c1c1e;color:#f5f5f7;padding:1rem 1.5rem;" +
      "font-family:-apple-system,BlinkMacSystemFont,'SF Pro Text','Segoe UI',system-ui,sans-serif;" +
      "font-size:0.8125rem;line-height:1.5;z-index:99999;" +
      "border-top:2px solid #ff453a;" +
      "display:flex;align-items:flex-start;gap:1rem;";

    var body = document.createElement("div");
    body.style.cssText = "flex:1;min-width:0;";

    var titleEl = document.createElement("strong");
    titleEl.style.cssText = "color:#ff453a;display:block;margin-bottom:0.25rem;";
    titleEl.textContent = title;

    var messageEl = document.createElement("span");
    messageEl.style.cssText = "color:#a1a1a6;word-break:break-word;";
    messageEl.textContent = message;

    body.appendChild(titleEl);
    body.appendChild(messageEl);

    var closeBtn = document.createElement("button");
    closeBtn.style.cssText =
      "background:none;border:1px solid #48484a;color:#a1a1a6;" +
      "border-radius:0.375rem;padding:0.25rem 0.5rem;cursor:pointer;" +
      "font-size:0.75rem;flex-shrink:0;";
    closeBtn.textContent = "Dismiss";
    closeBtn.addEventListener("click", hideErrorOverlay);

    overlay.appendChild(body);
    overlay.appendChild(closeBtn);

    document.body.appendChild(overlay);
  }

  function hideErrorOverlay() {
    var overlay = document.getElementById("adapto-error-overlay");
    if (overlay && overlay.parentNode) {
      overlay.parentNode.removeChild(overlay);
    }
  }

  // =========================================================================
  // Utilities
  // =========================================================================

  /**
   * Walk up the DOM to find the nearest component root. Components are
   * identified by `data-ar-root` (root component) or `data-ar-component`.
   */
  function findComponentId(el) {
    var comp = el.closest("[data-ar-root]") || el.closest("[data-ar-component]");
    if (!comp) return "root";
    return comp.getAttribute("data-ar-root") || comp.getAttribute("data-ar-component") || "root";
  }

  /**
   * Minimal CSS-escape for attribute selector values. Prevents injection
   * through crafted target strings.
   */
  function cssEscape(str) {
    return str.replace(/["\\]/g, "\\$&");
  }

  /**
   * Dispatch a CustomEvent on document for external code to observe
   * runtime lifecycle events.
   */
  function dispatchAdaptoEvent(name, detail) {
    document.dispatchEvent(new CustomEvent(name, { detail: detail || {} }));
  }

  // =========================================================================
  // Initialization
  // =========================================================================

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }

  // =========================================================================
  // Public API
  // =========================================================================

  window.Adapto = {
    /**
     * Programmatically send an event to the server.
     * Useful for custom components that need to trigger handlers.
     */
    sendEvent: sendEvent,

    /**
     * Trigger a client-side navigation.
     */
    navigate: navigateTo,

    /**
     * Force a reconnection attempt, resetting the backoff counter.
     */
    reconnect: function () {
      reconnectAttempts = 0;
      if (wsUrl) connect(wsUrl);
    },

    /**
     * Gracefully close the WebSocket connection.
     */
    disconnect: function () {
      if (socket) socket.close(1000, "Client disconnect");
    },

    /** Whether the WebSocket is currently open. */
    isConnected: function () { return connected; },

    /** The current session ID from the bootstrap payload. */
    getSessionId: function () { return sessionId; },

    /** Show a flash message programmatically. */
    flash: showFlash,

    /** The protocol version this client speaks. */
    PROTOCOL_VERSION: PROTOCOL_VERSION
  };
})();
