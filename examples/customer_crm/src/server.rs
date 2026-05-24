use adapto_client_protocol::patch::*;
use adapto_store::{AdaptoStore, Query, Update};
use adapto_ui;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, WebSocketUpgrade};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;

// ---------------------------------------------------------------------------
// Domain helpers — work with serde_json::Value documents from the store
// ---------------------------------------------------------------------------

fn doc_name(data: &serde_json::Value) -> &str {
    data.get("name").and_then(|v| v.as_str()).unwrap_or("")
}

fn doc_email(data: &serde_json::Value) -> &str {
    data.get("email").and_then(|v| v.as_str()).unwrap_or("")
}

fn doc_company(data: &serde_json::Value) -> &str {
    data.get("company").and_then(|v| v.as_str()).unwrap_or("")
}

fn doc_status(data: &serde_json::Value) -> &str {
    data.get("status").and_then(|v| v.as_str()).unwrap_or("active")
}

fn doc_created(data: &serde_json::Value) -> &str {
    data.get("created_at").and_then(|v| v.as_str()).unwrap_or("")
}

fn doc_initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn avatar_color_class(id: &str) -> &'static str {
    let hash: u32 = id.bytes().map(|b| b as u32).sum();
    match hash % 6 {
        0 => "au-avatar--blue",
        1 => "au-avatar--green",
        2 => "au-avatar--purple",
        3 => "au-avatar--orange",
        4 => "au-avatar--pink",
        _ => "au-avatar--red",
    }
}

fn status_badge_class(status: &str) -> &'static str {
    match status {
        "active" => "au-badge--success",
        "lead" => "au-badge--warning",
        _ => "au-badge--default",
    }
}

fn status_label(status: &str) -> &str {
    match status {
        "active" => "Active",
        "lead" => "Lead",
        "inactive" => "Inactive",
        other => other,
    }
}

// ---------------------------------------------------------------------------
// Shared state — AdaptoStore replaces Vec<Customer>
// ---------------------------------------------------------------------------

struct AppState {
    store: AdaptoStore,
}

impl AppState {
    fn new() -> Self {
        let store = AdaptoStore::open(Some("./data/crm")).unwrap();
        let customers = store.collection("customers");

        // Seed sample data only if collection is empty
        if customers.count_all() == 0 {
            customers.create_index("name", false).unwrap();
            customers.create_index("status", false).unwrap();
            customers.create_index("email", true).unwrap();

            let samples = vec![
                json!({"name": "John Appleseed", "email": "john@apple.com", "company": "Apple Inc.", "status": "active", "created_at": "2025-11-14"}),
                json!({"name": "Sarah Connor", "email": "sarah@cyberdyne.com", "company": "Cyberdyne Systems", "status": "lead", "created_at": "2026-01-08"}),
                json!({"name": "Bruce Wayne", "email": "bruce@wayne.com", "company": "Wayne Enterprises", "status": "active", "created_at": "2025-09-22"}),
                json!({"name": "Ellen Ripley", "email": "ripley@weyland.com", "company": "Weyland Corp", "status": "inactive", "created_at": "2025-06-03"}),
                json!({"name": "Tony Stark", "email": "tony@stark.com", "company": "Stark Industries", "status": "active", "created_at": "2026-03-19"}),
            ];
            customers.insert_many(samples).unwrap();
            println!("  Seeded 5 sample customers");
        } else {
            println!("  Loaded {} customers from disk", customers.count_all());
        }

        Self { store }
    }

    fn customers(&self) -> adapto_store::Collection<'_> {
        self.store.collection("customers")
    }
}

// ---------------------------------------------------------------------------
// HTML escape
// ---------------------------------------------------------------------------

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// ---------------------------------------------------------------------------
// Render functions — read from store
// ---------------------------------------------------------------------------

fn render_stats_bar(state: &AppState) -> String {
    let col = state.customers();
    let total = col.count_all();
    let active = col.count(Query::eq("status", "active")).unwrap_or(0);
    let leads = col.count(Query::eq("status", "lead")).unwrap_or(0);
    let inactive = col.count(Query::eq("status", "inactive")).unwrap_or(0);

    format!(
        r#"<div class="au-grid au-grid--4" style="margin-bottom:var(--au-space-6)">
  <div class="au-card au-card--flat">
    <div class="au-card__body" style="text-align:center">
      <div style="font-size:var(--au-text-3xl);font-weight:var(--au-weight-bold);color:var(--au-color-text)">{total}</div>
      <div style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary);margin-top:var(--au-space-1)">Total Customers</div>
    </div>
  </div>
  <div class="au-card au-card--flat">
    <div class="au-card__body" style="text-align:center">
      <div style="font-size:var(--au-text-3xl);font-weight:var(--au-weight-bold);color:var(--au-color-green)">{active}</div>
      <div style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary);margin-top:var(--au-space-1)">Active</div>
    </div>
  </div>
  <div class="au-card au-card--flat">
    <div class="au-card__body" style="text-align:center">
      <div style="font-size:var(--au-text-3xl);font-weight:var(--au-weight-bold);color:var(--au-color-orange)">{leads}</div>
      <div style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary);margin-top:var(--au-space-1)">Leads</div>
    </div>
  </div>
  <div class="au-card au-card--flat">
    <div class="au-card__body" style="text-align:center">
      <div style="font-size:var(--au-text-3xl);font-weight:var(--au-weight-bold);color:var(--au-color-text-tertiary)">{inactive}</div>
      <div style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary);margin-top:var(--au-space-1)">Inactive</div>
    </div>
  </div>
</div>"#,
    )
}

fn render_customer_row(id: &str, data: &serde_json::Value) -> String {
    let name = esc(doc_name(data));
    let email = esc(doc_email(data));
    let company = esc(doc_company(data));
    let status = doc_status(data);
    let badge_class = status_badge_class(status);
    let sl = esc(status_label(status));
    let initials = esc(&doc_initials(doc_name(data)));
    let avatar_color = avatar_color_class(id);
    let eid = esc(id);

    format!(
        r#"<tr>
  <td>
    <div class="au-flex au-items-center au-gap-3">
      <div class="au-avatar au-avatar--sm {avatar_color}" role="img" aria-label="{name}">
        <span class="au-avatar__initials" aria-hidden="true">{initials}</span>
      </div>
      <span style="font-weight:var(--au-weight-medium)">{name}</span>
    </div>
  </td>
  <td style="color:var(--au-color-text-secondary)">{email}</td>
  <td>{company}</td>
  <td><span class="au-badge {badge_class}">{sl}</span></td>
  <td>
    <div class="au-flex au-gap-2">
      <button type="button" class="au-btn au-btn--ghost au-btn--sm" data-action="show_detail" data-id="{eid}">View</button>
      <button type="button" class="au-btn au-btn--ghost au-btn--sm" data-action="toggle_status" data-id="{eid}">Toggle</button>
      <button type="button" class="au-btn au-btn--ghost au-btn--sm" style="color:var(--au-color-red)" data-action="delete_customer" data-id="{eid}">Delete</button>
    </div>
  </td>
</tr>"#,
    )
}

fn render_customer_table(state: &AppState, search: &str) -> String {
    let col = state.customers();
    let total = col.count_all();

    let docs: Vec<_> = if search.is_empty() {
        col.find(Query::new()).collect()
    } else {
        let q = search.to_lowercase();
        col.find(Query::new())
            .filter(|d| {
                doc_name(&d.data).to_lowercase().contains(&q)
                    || doc_email(&d.data).to_lowercase().contains(&q)
                    || doc_company(&d.data).to_lowercase().contains(&q)
            })
            .collect()
    };

    let shown = docs.len();
    let search_val = esc(search);

    let toolbar = format!(
        r#"<div class="au-flex au-items-center au-justify-between" style="margin-bottom:var(--au-space-5)">
  <div>
    <h2 style="font-size:var(--au-text-2xl);font-weight:var(--au-weight-bold);margin:0">Customers</h2>
    <p style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary);margin:var(--au-space-1) 0 0">{total} total &middot; Showing {shown}</p>
  </div>
  <div class="au-flex au-gap-3 au-items-center">
    <div class="au-input-wrapper au-input-wrapper--prefix">
      <span class="au-input-prefix" aria-hidden="true">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none"><circle cx="7" cy="7" r="5.5" stroke="currentColor" stroke-width="1.5"/><path d="M11 11l3.5 3.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
      </span>
      <input type="search" class="au-input au-input--sm" name="search" placeholder="Search customers..." value="{search_val}" data-field="search" />
    </div>
    <button type="button" class="au-btn au-btn--primary au-btn--sm" data-action="show_form">+ Add Customer</button>
  </div>
</div>"#,
    );

    if docs.is_empty() {
        let empty = r#"<div class="au-table-container">
  <table class="au-table">
    <thead><tr>
      <th>Name</th><th>Email</th><th>Company</th><th>Status</th><th>Actions</th>
    </tr></thead>
    <tbody>
      <tr><td colspan="5" class="au-table__empty">No customers match your search.</td></tr>
    </tbody>
  </table>
</div>"#;
        return format!("{toolbar}{empty}");
    }

    let rows: String = docs
        .iter()
        .map(|d| render_customer_row(&d.id, &d.data))
        .collect();

    format!(
        r#"{toolbar}<div class="au-table-container">
  <table class="au-table au-table--hoverable">
    <thead>
      <tr>
        <th>Name</th>
        <th>Email</th>
        <th>Company</th>
        <th>Status</th>
        <th>Actions</th>
      </tr>
    </thead>
    <tbody>{rows}</tbody>
  </table>
</div>"#,
    )
}

fn render_customer_detail(id: &str, data: &serde_json::Value) -> String {
    let name = esc(doc_name(data));
    let email = esc(doc_email(data));
    let company = esc(doc_company(data));
    let status = doc_status(data);
    let badge_class = status_badge_class(status);
    let sl = esc(status_label(status));
    let initials = esc(&doc_initials(doc_name(data)));
    let avatar_color = avatar_color_class(id);
    let created = esc(doc_created(data));
    let eid = esc(id);

    format!(
        r#"<div style="max-width:640px">
  <button type="button" class="au-btn au-btn--ghost au-btn--sm" data-action="show_list" style="margin-bottom:var(--au-space-4)">
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" style="margin-right:4px"><path d="M10 3L5 8l5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    Back to list
  </button>
  <div class="au-card au-card--elevated">
    <div class="au-card__header">
      <div class="au-flex au-items-center au-gap-4">
        <div class="au-avatar au-avatar--lg {avatar_color}" role="img" aria-label="{name}">
          <span class="au-avatar__initials" aria-hidden="true">{initials}</span>
        </div>
        <div>
          <div style="font-size:var(--au-text-xl);font-weight:var(--au-weight-bold)">{name}</div>
          <div style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary)">{company}</div>
        </div>
        <div style="margin-left:auto">
          <span class="au-badge {badge_class}">{sl}</span>
        </div>
      </div>
    </div>
    <div class="au-card__body">
      <div class="au-stack au-stack--4">
        <div class="au-flex au-justify-between au-items-center" style="padding:var(--au-space-3) 0;border-bottom:1px solid var(--au-color-border)">
          <span style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary)">Email</span>
          <span style="font-size:var(--au-text-sm);font-weight:var(--au-weight-medium)">{email}</span>
        </div>
        <div class="au-flex au-justify-between au-items-center" style="padding:var(--au-space-3) 0;border-bottom:1px solid var(--au-color-border)">
          <span style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary)">Company</span>
          <span style="font-size:var(--au-text-sm);font-weight:var(--au-weight-medium)">{company}</span>
        </div>
        <div class="au-flex au-justify-between au-items-center" style="padding:var(--au-space-3) 0;border-bottom:1px solid var(--au-color-border)">
          <span style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary)">Status</span>
          <span class="au-badge {badge_class}">{sl}</span>
        </div>
        <div class="au-flex au-justify-between au-items-center" style="padding:var(--au-space-3) 0">
          <span style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary)">Added</span>
          <span style="font-size:var(--au-text-sm);font-weight:var(--au-weight-medium)">{created}</span>
        </div>
      </div>
    </div>
    <div class="au-card__footer">
      <div class="au-flex au-gap-3 au-justify-end">
        <button type="button" class="au-btn au-btn--secondary au-btn--sm" data-action="toggle_status" data-id="{eid}">Toggle Status</button>
        <button type="button" class="au-btn au-btn--destructive au-btn--sm" data-action="delete_customer" data-id="{eid}">Delete Customer</button>
      </div>
    </div>
  </div>
</div>"#,
    )
}

fn render_customer_form() -> String {
    r#"<div style="max-width:520px">
  <button type="button" class="au-btn au-btn--ghost au-btn--sm" data-action="show_list" style="margin-bottom:var(--au-space-4)">
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" style="margin-right:4px"><path d="M10 3L5 8l5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>
    Back to list
  </button>
  <div class="au-card au-card--elevated">
    <div class="au-card__header">Add Customer</div>
    <div class="au-card__body">
      <div class="au-form-group">
        <label class="au-form-group__label">Full Name<span class="au-form-group__required" aria-hidden="true">*</span></label>
        <input type="text" class="au-input" name="form_name" placeholder="e.g. Jane Smith" data-field="form_name" required aria-required="true" />
      </div>
      <div class="au-form-group">
        <label class="au-form-group__label">Email<span class="au-form-group__required" aria-hidden="true">*</span></label>
        <input type="email" class="au-input" name="form_email" placeholder="e.g. jane@company.com" data-field="form_email" required aria-required="true" />
      </div>
      <div class="au-form-group">
        <label class="au-form-group__label">Company<span class="au-form-group__required" aria-hidden="true">*</span></label>
        <input type="text" class="au-input" name="form_company" placeholder="e.g. Acme Corp" data-field="form_company" required aria-required="true" />
      </div>
      <div class="au-form-group">
        <label class="au-form-group__label">Status</label>
        <select class="au-select" name="form_status" data-field="form_status">
          <option value="active" selected>Active</option>
          <option value="lead">Lead</option>
          <option value="inactive">Inactive</option>
        </select>
      </div>
    </div>
    <div class="au-card__footer">
      <div class="au-form-actions au-form-actions--end">
        <button type="button" class="au-btn au-btn--secondary" data-action="show_list">Cancel</button>
        <button type="button" class="au-btn au-btn--primary" data-action="add_customer">Add Customer</button>
      </div>
    </div>
  </div>
</div>"#
        .to_string()
}

fn render_route_indicator(route: &str) -> String {
    let parts: Vec<&str> = route.trim_matches('/').split('/').collect();
    let mut html = String::from(r#"<a href="/customers" data-route="/customers">customers</a>"#);
    if parts.len() > 1 {
        match parts[1] {
            "new" => {
                html.push_str(r#" <span class="crm-route-sep">/</span> <span class="crm-route-current">new</span>"#);
            }
            id => {
                html.push_str(&format!(
                    r#" <span class="crm-route-sep">/</span> <span class="crm-route-current">{}</span>"#,
                    esc(id)
                ));
            }
        }
    }
    html
}

// ---------------------------------------------------------------------------
// Full page HTML
// ---------------------------------------------------------------------------

fn render_full_page_with_content(state: &AppState, content: &str, current_route: &str) -> String {
    let css = adapto_ui::bundle_css();
    let stats = render_stats_bar(state);
    let route_indicator = render_route_indicator(current_route);
    let db_stats = state.store.stats();

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Adapto CRM</title>
  <style>{css}</style>
  <style>
    body {{
      background-color: var(--au-color-bg-secondary);
      margin: 0;
    }}
    .crm-main {{
      padding: var(--au-space-6);
    }}
    .crm-routes {{
      display: flex;
      gap: var(--au-space-2);
      align-items: center;
      padding: var(--au-space-2) var(--au-space-4);
      background: var(--au-color-bg);
      border-bottom: 1px solid var(--au-color-border);
      font-size: var(--au-text-sm);
      color: var(--au-color-text-secondary);
      font-family: var(--au-font-family);
    }}
    .crm-routes a {{
      color: var(--au-color-blue);
      text-decoration: none;
      font-weight: var(--au-weight-medium);
    }}
    .crm-routes a:hover {{
      text-decoration: underline;
    }}
    .crm-routes .crm-route-current {{
      color: var(--au-color-text);
      font-weight: var(--au-weight-semibold);
    }}
    .crm-routes .crm-route-sep {{
      color: var(--au-color-text-tertiary);
    }}
    .crm-db-badge {{
      margin-left: auto;
      font-size: var(--au-text-xs);
      color: var(--au-color-text-tertiary);
      font-family: var(--au-font-mono);
    }}
  </style>
</head>
<body>
  <nav class="au-nav au-nav--sticky au-nav--blur">
    <span class="au-nav__brand">Adapto CRM</span>
    <div class="au-nav__items">
      <a href="/customers" class="au-nav__item au-nav__item--active" data-route="/customers">Customers</a>
    </div>
    <div class="au-nav__end">
      <span style="font-size:var(--au-text-xs);color:var(--au-color-text-tertiary);font-family:var(--au-font-mono);margin-right:var(--au-space-3)">adapto_store &middot; {docs} docs &middot; WAL {wal_kb}KB</span>
      <div class="au-avatar au-avatar--sm au-avatar--blue" role="img" aria-label="You">
        <span class="au-avatar__initials" aria-hidden="true">AD</span>
      </div>
    </div>
  </nav>
  <div class="crm-routes" id="crm-breadcrumb">{route_indicator}</div>
  <main class="crm-main">
    <div class="au-container">
      <div id="crm-stats">{stats}</div>
      <div id="crm-content">{content}</div>
    </div>
  </main>
  {client_js}
</body>
</html>"#,
        client_js = inline_client_script(),
        docs = db_stats.total_documents,
        wal_kb = db_stats.wal_size_bytes / 1024,
    )
}

// ---------------------------------------------------------------------------
// Client-side JavaScript
// ---------------------------------------------------------------------------

fn inline_client_script() -> String {
    r##"<script>
(function() {
  var ws, seq = 0;
  var proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
  var url = proto + '//' + location.host + '/ws';

  function connect() {
    ws = new WebSocket(url);
    ws.onopen = function() { console.log('[Adapto CRM] Connected — powered by adapto_store'); };
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
      } catch(err) { console.error('[Adapto CRM]', err); }
    };
    ws.onclose = function() { setTimeout(connect, 2000); };
    ws.onerror = function() { ws.close(); };
  }

  function send(handler, payload) {
    if (ws && ws.readyState === 1) {
      ws.send(JSON.stringify({
        v: 1,
        type: 'event',
        session: 'crm-live',
        component: 'crm',
        event: 'click',
        handler: handler,
        payload: payload || {},
        seq: ++seq
      }));
    }
    if (handler === 'show_list') {
      history.pushState(null, '', '/customers');
    } else if (handler === 'show_detail' && payload && payload.id) {
      history.pushState(null, '', '/customers/' + payload.id);
    } else if (handler === 'show_form') {
      history.pushState(null, '', '/customers/new');
    } else if (handler === 'add_customer') {
      history.pushState(null, '', '/customers');
    }
  }

  window.addEventListener('popstate', function() {
    var path = location.pathname;
    if (path === '/customers' || path === '/') {
      send('show_list', {});
    } else if (path === '/customers/new') {
      send('show_form', {});
    } else if (path.match(/^\/customers\/(.+)$/)) {
      var id = path.split('/')[2];
      send('show_detail', { id: id });
    }
  });

  function collectFields() {
    var data = {};
    document.querySelectorAll('#crm-content [data-field]').forEach(function(el) {
      data[el.getAttribute('data-field')] = el.value || '';
    });
    return data;
  }

  document.addEventListener('click', function(e) {
    var btn = e.target.closest('[data-action]');
    if (!btn) return;
    e.preventDefault();

    var action = btn.getAttribute('data-action');
    var payload = {};

    var id = btn.getAttribute('data-id');
    if (id) payload.id = id;

    if (action === 'add_customer' || action === 'update_customer') {
      var fields = collectFields();
      Object.keys(fields).forEach(function(k) {
        payload[k] = fields[k];
      });
    }

    send(action, payload);
  });

  var searchTimer = null;
  document.addEventListener('input', function(e) {
    var el = e.target;
    if (el.getAttribute('data-field') === 'search') {
      clearTimeout(searchTimer);
      searchTimer = setTimeout(function() {
        send('search_customers', { search: el.value });
      }, 200);
    }
  });

  connect();
})();
</script>"##
        .to_string()
}

// ---------------------------------------------------------------------------
// Axum route handlers
// ---------------------------------------------------------------------------

async fn handle_page(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    let content = render_customer_table(&state, "");
    Html(render_full_page_with_content(&state, &content, "/customers"))
}

async fn handle_customers(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    let content = render_customer_table(&state, "");
    Html(render_full_page_with_content(&state, &content, "/customers"))
}

async fn handle_customer_new(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    let content = render_customer_form();
    Html(render_full_page_with_content(&state, &content, "/customers/new"))
}

async fn handle_customer_detail(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let col = state.customers();
    let content = match col.find_by_id(&id).unwrap() {
        Some(doc) => render_customer_detail(&doc.id, &doc.data),
        None => render_customer_table(&state, ""),
    };
    let route = format!("/customers/{id}");
    Html(render_full_page_with_content(&state, &content, &route))
}

async fn handle_ws(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, state))
}

// ---------------------------------------------------------------------------
// WebSocket event loop
// ---------------------------------------------------------------------------

async fn ws_loop(mut socket: WebSocket, state: Arc<AppState>) {
    let mut search = String::new();

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let val: serde_json::Value = match serde_json::from_str(&text) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let handler = val
                .get("handler")
                .and_then(|h| h.as_str())
                .unwrap_or("");
            let payload = val.get("payload").cloned().unwrap_or(json!({}));
            let client_seq = val.get("seq").and_then(|s| s.as_u64()).unwrap_or(0);

            let (content_html, route) = match handler {
                "show_list" => {
                    search.clear();
                    (render_customer_table(&state, ""), "/customers".to_string())
                }

                "show_detail" => {
                    let id = payload
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let col = state.customers();
                    match col.find_by_id(id).unwrap() {
                        Some(doc) => (
                            render_customer_detail(&doc.id, &doc.data),
                            format!("/customers/{id}"),
                        ),
                        None => {
                            search.clear();
                            (render_customer_table(&state, ""), "/customers".to_string())
                        }
                    }
                }

                "show_form" => (render_customer_form(), "/customers/new".to_string()),

                "add_customer" => {
                    let name = payload
                        .get("form_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let email = payload
                        .get("form_email")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let company = payload
                        .get("form_company")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    let status = payload
                        .get("form_status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("active")
                        .to_string();

                    if name.is_empty() || email.is_empty() || company.is_empty() {
                        (render_customer_form(), "/customers/new".to_string())
                    } else {
                        let col = state.customers();
                        col.insert(json!({
                            "name": name,
                            "email": email,
                            "company": company,
                            "status": status,
                            "created_at": "2026-05-25"
                        }))
                        .unwrap();
                        search.clear();
                        (render_customer_table(&state, ""), "/customers".to_string())
                    }
                }

                "delete_customer" => {
                    let id = payload
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let col = state.customers();
                    let _ = col.delete_by_id(id);
                    (render_customer_table(&state, &search), "/customers".to_string())
                }

                "toggle_status" => {
                    let id = payload
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let col = state.customers();
                    if let Ok(Some(doc)) = col.find_by_id(id) {
                        let new_status = match doc_status(&doc.data) {
                            "active" => "inactive",
                            "inactive" => "lead",
                            "lead" => "active",
                            _ => "active",
                        };
                        let _ = col.update_by_id(
                            id,
                            Update::Set(vec![("status".into(), json!(new_status))]),
                        );
                    }
                    (render_customer_table(&state, &search), "/customers".to_string())
                }

                "search_customers" => {
                    search = payload
                        .get("search")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    (render_customer_table(&state, &search), "/customers".to_string())
                }

                _ => continue,
            };

            let breadcrumb_html = render_route_indicator(&route);
            let stats_html = render_stats_bar(&state);

            let patch = PatchMessage {
                seq: client_seq,
                ops: vec![
                    PatchOp::ReplaceHtml {
                        target: "crm-content".into(),
                        html: content_html,
                    },
                    PatchOp::ReplaceHtml {
                        target: "crm-breadcrumb".into(),
                        html: breadcrumb_html,
                    },
                    PatchOp::ReplaceHtml {
                        target: "crm-stats".into(),
                        html: stats_html,
                    },
                ],
            };

            let server_msg = ServerMessage::new(ServerPayload::Patch(patch));
            if let Ok(json_str) = serde_json::to_string(&server_msg) {
                if socket.send(Message::Text(json_str.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Entrypoint
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState::new());

    let router = Router::new()
        .route("/", get(handle_page))
        .route("/customers", get(handle_customers))
        .route("/customers/new", get(handle_customer_new))
        .route("/customers/:id", get(handle_customer_detail))
        .route("/ws", get(handle_ws))
        .with_state(state);

    let addr = "127.0.0.1:3001";
    println!();
    println!("  Adapto CRM running at http://{addr}");
    println!("  Database: ./data/crm/store.wal");
    println!("  Press Ctrl+C to stop.");
    println!();

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
