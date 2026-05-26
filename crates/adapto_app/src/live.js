(function() {
  var ws, seq = 0;
  var proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  var url = proto + '//' + location.host + '/ws';

  function connect() {
    ws = new WebSocket(url);
    ws.onopen = function() { console.log('[Adapto] Connected'); };
    ws.onmessage = function(e) {
      try {
        var msg = JSON.parse(e.data);
        if (msg.type === 'patch' && msg.ops) {
          msg.ops.forEach(function(op) {
            if (op.op === 'replace_html') {
              var el = document.getElementById(op.target);
              if (el) {
                el.textContent = '';
                var tpl = document.createElement('template');
                tpl.innerHTML = op.html;
                el.appendChild(tpl.content);
              }
            } else if (op.op === 'replace_text') {
              var el2 = document.querySelector('[data-ar-dyn="' + op.target + '"]');
              if (el2) el2.textContent = op.value;
            }
          });
        }
      } catch(err) { console.error('[Adapto]', err); }
    };
    var reconnecting = false;
    ws.onclose = function() {
      if (!reconnecting) {
        reconnecting = true;
        setTimeout(function() { reconnecting = false; connect(); }, 2000);
      }
    };
    ws.onerror = function() {};
  }

  function send(handler, payload) {
    if (ws && ws.readyState === 1) {
      ws.send(JSON.stringify({
        v: 1, type: 'event', session: 'adapto-live',
        component: 'app', event: 'click',
        handler: handler, payload: payload || {}, seq: ++seq
      }));
    }
  }

  // URL routing via pushState
  window.__adapto_navigate = function(path) {
    if (path.indexOf('://') !== -1 || path.indexOf('//') === 0) {
      location.href = path;
      return;
    }
    history.pushState(null, '', path);
    send('navigate', { path: path });
  };

  window.addEventListener('popstate', function() {
    send('navigate', { path: location.pathname });
  });

  // Delegated click handler
  document.addEventListener('click', function(e) {
    // Route links
    var link = e.target.closest('[data-route]');
    if (link) {
      e.preventDefault();
      var path = link.getAttribute('data-route') || link.getAttribute('href');
      window.__adapto_navigate(path);
      return;
    }
    // Action buttons
    var btn = e.target.closest('[data-action]');
    if (!btn) return;
    e.preventDefault();
    var action = btn.getAttribute('data-action');
    var payload = {};
    var id = btn.getAttribute('data-id');
    if (id) payload.id = id;
    // Form field collection
    if (action.indexOf('create') >= 0 || action.indexOf('update') >= 0 || action.indexOf('add') >= 0) {
      document.querySelectorAll('[data-field]').forEach(function(el) {
        payload[el.getAttribute('data-field')] = el.value || '';
      });
    }
    send(action, payload);
    // Update URL for common actions
    if (action === 'show_list' || action.indexOf('delete') >= 0) {
      history.pushState(null, '', location.pathname.split('/').slice(0, 2).join('/'));
    }
  });

  // Debounced search
  var searchTimer = null;
  document.addEventListener('input', function(e) {
    var el = e.target;
    if (el.getAttribute('data-field') === 'search') {
      clearTimeout(searchTimer);
      searchTimer = setTimeout(function() {
        send('search', { query: el.value });
      }, 200);
    }
  });

  connect();
})();
