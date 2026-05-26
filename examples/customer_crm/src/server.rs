use adapto_app::handler::{ActionContext, ActionResult, AppState};
use adapto_app::{App, ResourceMeta};
use adapto_macros::Resource;
use adapto_store::{Query, Update};
use adapto_ui::html_escape;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Resource definition — replaces manual struct + helpers
// ---------------------------------------------------------------------------

#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(collection = "customers")]
pub struct Customer {
    #[field(required, max_length = 120)]
    pub name: String,

    #[field(required, unique, format = "email")]
    pub email: String,

    #[field(required)]
    pub company: String,

    #[field(default = "active", one_of = ["active", "lead", "inactive"])]
    pub status: String,

    pub created_at: String,
}

impl ResourceMeta for Customer {
    fn collection_name() -> &'static str { "customers" }
    fn field_names() -> &'static [&'static str] { &["name", "email", "company", "status", "created_at"] }
    fn resource_label() -> &'static str { "Customer" }
    fn resource_label_plural() -> &'static str { "Customers" }
    fn route_prefix() -> &'static str { "/customers" }
    fn ensure_indexes(store: &adapto_store::AdaptoStore) {
        Customer::ensure_indexes(store);
    }
}

// ---------------------------------------------------------------------------
// UI helpers
// ---------------------------------------------------------------------------

fn esc(s: &str) -> String { html_escape(s) }

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

fn avatar_color(id: &str) -> &'static str {
    let h: u32 = id.bytes().map(|b| b as u32).sum();
    match h % 6 {
        0 => "au-avatar--blue",
        1 => "au-avatar--green",
        2 => "au-avatar--purple",
        3 => "au-avatar--orange",
        4 => "au-avatar--pink",
        _ => "au-avatar--red",
    }
}

fn badge(status: &str) -> (&'static str, &'static str) {
    match status {
        "active" => ("au-badge--success", "Active"),
        "lead" => ("au-badge--warning", "Lead"),
        "inactive" => ("au-badge--default", "Inactive"),
        _ => ("au-badge--default", "Other"),
    }
}

// ---------------------------------------------------------------------------
// Render functions — same UI, fewer lines
// ---------------------------------------------------------------------------

fn render_stats(store: &adapto_store::AdaptoStore) -> String {
    let col = store.collection("customers");
    let total = col.count_all();
    let active = col.count(Query::eq("status", "active")).unwrap_or(0);
    let leads = col.count(Query::eq("status", "lead")).unwrap_or(0);
    let inactive = col.count(Query::eq("status", "inactive")).unwrap_or(0);

    fn stat_card(value: u64, label: &str, color: &str) -> String {
        format!(
            r#"<div class="au-card au-card--flat"><div class="au-card__body" style="text-align:center">
  <div style="font-size:var(--au-text-3xl);font-weight:var(--au-weight-bold);color:var(--au-color-{color})">{value}</div>
  <div style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary);margin-top:var(--au-space-1)">{label}</div>
</div></div>"#
        )
    }

    format!(
        r#"<div class="au-grid au-grid--4" style="margin-bottom:var(--au-space-6)">{}{}{}{}</div>"#,
        stat_card(total, "Total Customers", "text"),
        stat_card(active, "Active", "green"),
        stat_card(leads, "Leads", "orange"),
        stat_card(inactive, "Inactive", "text-tertiary"),
    )
}

fn render_row(id: &str, c: &Customer) -> String {
    let (bc, bl) = badge(&c.status);
    format!(
        r#"<tr>
  <td><div class="au-flex au-items-center au-gap-3">
    <div class="au-avatar au-avatar--sm {ac}" role="img" aria-label="{n}"><span class="au-avatar__initials" aria-hidden="true">{ini}</span></div>
    <span style="font-weight:var(--au-weight-medium)">{n}</span>
  </div></td>
  <td style="color:var(--au-color-text-secondary)">{e}</td>
  <td>{co}</td>
  <td><span class="au-badge {bc}">{bl}</span></td>
  <td><div class="au-flex au-gap-2">
    <button type="button" class="au-btn au-btn--ghost au-btn--sm" data-action="show_detail" data-id="{eid}">View</button>
    <button type="button" class="au-btn au-btn--ghost au-btn--sm" data-action="toggle_status" data-id="{eid}">Toggle</button>
    <button type="button" class="au-btn au-btn--ghost au-btn--sm" style="color:var(--au-color-red)" data-action="delete_customer" data-id="{eid}">Delete</button>
  </div></td>
</tr>"#,
        ac = avatar_color(id), n = esc(&c.name), ini = esc(&initials(&c.name)),
        e = esc(&c.email), co = esc(&c.company), eid = esc(id),
    )
}

fn render_table(store: &adapto_store::AdaptoStore, search: &str) -> String {
    let col = store.collection("customers");
    let total = col.count_all();

    let docs: Vec<_> = if search.is_empty() {
        col.find(Query::new()).collect()
    } else {
        let q = search.to_lowercase();
        col.find(Query::new())
            .filter(|d| {
                let data_str = d.data.to_string().to_lowercase();
                data_str.contains(&q)
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
      <input type="search" class="au-input au-input--sm" placeholder="Search customers..." value="{search_val}" data-field="search" />
    </div>
    <button type="button" class="au-btn au-btn--primary au-btn--sm" data-action="show_form">+ Add Customer</button>
  </div>
</div>"#
    );

    if docs.is_empty() {
        return format!(
            r#"{toolbar}<div class="au-table-container"><table class="au-table"><thead><tr>
<th>Name</th><th>Email</th><th>Company</th><th>Status</th><th>Actions</th>
</tr></thead><tbody><tr><td colspan="5" class="au-table__empty">No customers match your search.</td></tr></tbody></table></div>"#
        );
    }

    let rows: String = docs
        .iter()
        .filter_map(|d| Customer::from_document(d).map(|c| render_row(&d.id, &c)))
        .collect();

    format!(
        r#"{toolbar}<div class="au-table-container"><table class="au-table au-table--hoverable"><thead><tr>
<th>Name</th><th>Email</th><th>Company</th><th>Status</th><th>Actions</th>
</tr></thead><tbody>{rows}</tbody></table></div>"#
    )
}

fn render_detail(id: &str, c: &Customer) -> String {
    let (bc, bl) = badge(&c.status);
    format!(
        r#"<div style="max-width:640px">
  <button type="button" class="au-btn au-btn--ghost au-btn--sm" data-action="show_list" style="margin-bottom:var(--au-space-4)">
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" style="margin-right:4px"><path d="M10 3L5 8l5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>Back to list
  </button>
  <div class="au-card au-card--elevated">
    <div class="au-card__header"><div class="au-flex au-items-center au-gap-4">
      <div class="au-avatar au-avatar--lg {ac}" role="img" aria-label="{n}"><span class="au-avatar__initials" aria-hidden="true">{ini}</span></div>
      <div><div style="font-size:var(--au-text-xl);font-weight:var(--au-weight-bold)">{n}</div><div style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary)">{co}</div></div>
      <div style="margin-left:auto"><span class="au-badge {bc}">{bl}</span></div>
    </div></div>
    <div class="au-card__body"><div class="au-stack au-stack--4">
      {rows}
    </div></div>
    <div class="au-card__footer"><div class="au-flex au-gap-3 au-justify-end">
      <button type="button" class="au-btn au-btn--secondary au-btn--sm" data-action="toggle_status" data-id="{eid}">Toggle Status</button>
      <button type="button" class="au-btn au-btn--destructive au-btn--sm" data-action="delete_customer" data-id="{eid}">Delete Customer</button>
    </div></div>
  </div>
</div>"#,
        ac = avatar_color(id), n = esc(&c.name), ini = esc(&initials(&c.name)),
        co = esc(&c.company), eid = esc(id),
        rows = [("Email", esc(&c.email)), ("Company", esc(&c.company)), ("Status", format!(r#"<span class="au-badge {bc}">{bl}</span>"#)), ("Added", esc(&c.created_at))]
            .iter()
            .enumerate()
            .map(|(i, (label, val))| {
                let border = if i < 3 { ";border-bottom:1px solid var(--au-color-border)" } else { "" };
                format!(r#"<div class="au-flex au-justify-between au-items-center" style="padding:var(--au-space-3) 0{border}"><span style="font-size:var(--au-text-sm);color:var(--au-color-text-secondary)">{label}</span><span style="font-size:var(--au-text-sm);font-weight:var(--au-weight-medium)">{val}</span></div>"#)
            })
            .collect::<String>(),
    )
}

fn render_form() -> &'static str {
    r#"<div style="max-width:520px">
  <button type="button" class="au-btn au-btn--ghost au-btn--sm" data-action="show_list" style="margin-bottom:var(--au-space-4)">
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" style="margin-right:4px"><path d="M10 3L5 8l5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>Back to list
  </button>
  <div class="au-card au-card--elevated">
    <div class="au-card__header">Add Customer</div>
    <div class="au-card__body">
      <div class="au-form-group"><label class="au-form-group__label">Full Name<span class="au-form-group__required">*</span></label><input type="text" class="au-input" placeholder="e.g. Jane Smith" data-field="form_name" required /></div>
      <div class="au-form-group"><label class="au-form-group__label">Email<span class="au-form-group__required">*</span></label><input type="email" class="au-input" placeholder="e.g. jane@company.com" data-field="form_email" required /></div>
      <div class="au-form-group"><label class="au-form-group__label">Company<span class="au-form-group__required">*</span></label><input type="text" class="au-input" placeholder="e.g. Acme Corp" data-field="form_company" required /></div>
      <div class="au-form-group"><label class="au-form-group__label">Status</label><select class="au-select" data-field="form_status"><option value="active" selected>Active</option><option value="lead">Lead</option><option value="inactive">Inactive</option></select></div>
    </div>
    <div class="au-card__footer"><div class="au-form-actions au-form-actions--end">
      <button type="button" class="au-btn au-btn--secondary" data-action="show_list">Cancel</button>
      <button type="button" class="au-btn au-btn--primary" data-action="add_customer">Add Customer</button>
    </div></div>
  </div>
</div>"#
}

fn breadcrumb_html(route: &str) -> String {
    let parts: Vec<&str> = route.trim_matches('/').split('/').collect();
    let mut html = String::from(r#"<a href="/customers" data-route="/customers">customers</a>"#);
    if parts.len() > 1 {
        html.push_str(r#" <span class="adapto-breadcrumb-sep">/</span> <span class="adapto-breadcrumb-current">"#);
        html.push_str(&esc(parts[1]));
        html.push_str("</span>");
    }
    html
}

fn full_patch(store: &adapto_store::AdaptoStore, content: &str, route: &str) -> ActionResult {
    use adapto_client_protocol::patch::PatchOp;
    ActionResult::with_ops(vec![
        PatchOp::ReplaceHtml { target: "app-content".into(), html: content.to_string() },
        PatchOp::ReplaceHtml { target: "app-breadcrumb".into(), html: breadcrumb_html(route) },
        PatchOp::ReplaceHtml { target: "app-stats".into(), html: render_stats(store) },
    ])
}

fn get_str<'a>(payload: &'a serde_json::Value, key: &str) -> &'a str {
    payload.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

// ---------------------------------------------------------------------------
// Seed data
// ---------------------------------------------------------------------------

fn seed_if_empty(store: &adapto_store::AdaptoStore) {
    let col = store.collection("customers");
    if col.count_all() > 0 {
        println!("  Loaded {} customers from disk", col.count_all());
        return;
    }

    let samples = vec![
        json!({"name": "John Appleseed", "email": "john@apple.com", "company": "Apple Inc.", "status": "active", "created_at": "2025-11-14"}),
        json!({"name": "Sarah Connor", "email": "sarah@cyberdyne.com", "company": "Cyberdyne Systems", "status": "lead", "created_at": "2026-01-08"}),
        json!({"name": "Bruce Wayne", "email": "bruce@wayne.com", "company": "Wayne Enterprises", "status": "active", "created_at": "2025-09-22"}),
        json!({"name": "Ellen Ripley", "email": "ripley@weyland.com", "company": "Weyland Corp", "status": "inactive", "created_at": "2025-06-03"}),
        json!({"name": "Tony Stark", "email": "tony@stark.com", "company": "Stark Industries", "status": "active", "created_at": "2026-03-19"}),
    ];
    col.insert_many(samples).unwrap();
    println!("  Seeded 5 sample customers");
}

// ---------------------------------------------------------------------------
// Entrypoint — 846 → ~30 lines
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    App::new("Adapto CRM")
        .port(3001)
        .store_path("./data/crm")
        .resource::<Customer>()
        .index_page(|state: Arc<AppState>| {
            seed_if_empty(&state.store);
            let table = render_table(&state.store, "");
            let stats = render_stats(&state.store);
            let db = state.store.stats();
            format!(
                r#"<div id="app-stats">{stats}</div>
<div style="font-size:var(--au-text-xs);color:var(--au-color-text-tertiary);font-family:var(--au-font-mono);margin-bottom:var(--au-space-3)">adapto_store &middot; {} docs &middot; WAL {}KB</div>
{table}"#,
                db.total_documents, db.wal_size_bytes / 1024,
            )
        })
        .on("show_list", |ctx: &mut ActionContext<'_>| {
            ctx.session.remove("search");
            full_patch(&ctx.store, &render_table(ctx.store, ""), "/customers")
        })
        .on("show_detail", |ctx: &mut ActionContext<'_>| {
            let id = get_str(ctx.payload, "id");
            match Customer::find_by_id(ctx.store, id) {
                Some((_id, c)) => full_patch(ctx.store, &render_detail(id, &c), &format!("/customers/{id}")),
                None => full_patch(ctx.store, &render_table(ctx.store, ""), "/customers"),
            }
        })
        .on("show_form", |ctx: &mut ActionContext<'_>| {
            full_patch(ctx.store, render_form(), "/customers/new")
        })
        .on("add_customer", |ctx: &mut ActionContext<'_>| {
            let name = get_str(ctx.payload, "form_name").trim().to_string();
            let email = get_str(ctx.payload, "form_email").trim().to_string();
            let company = get_str(ctx.payload, "form_company").trim().to_string();
            let status = get_str(ctx.payload, "form_status").to_string();
            if name.is_empty() || email.is_empty() || company.is_empty() {
                return full_patch(ctx.store, render_form(), "/customers/new");
            }
            let c = Customer { name, email, company, status: if status.is_empty() { "active".into() } else { status }, created_at: "2026-05-25".into() };
            let _ = c.insert_into(ctx.store);
            ctx.session.remove("search");
            full_patch(ctx.store, &render_table(ctx.store, ""), "/customers")
        })
        .on("delete_customer", |ctx: &mut ActionContext<'_>| {
            let id = get_str(ctx.payload, "id");
            Customer::delete(ctx.store, id);
            let search = ctx.session.get("search").cloned().unwrap_or_default();
            full_patch(ctx.store, &render_table(ctx.store, &search), "/customers")
        })
        .on("toggle_status", |ctx: &mut ActionContext<'_>| {
            let id = get_str(ctx.payload, "id");
            if let Some((_, c)) = Customer::find_by_id(ctx.store, id) {
                let new = match c.status.as_str() { "active" => "inactive", "inactive" => "lead", _ => "active" };
                let col = ctx.store.collection("customers");
                let _ = col.update_by_id(id, Update::Set(vec![("status".into(), json!(new))]));
            }
            let search = ctx.session.get("search").cloned().unwrap_or_default();
            full_patch(ctx.store, &render_table(ctx.store, &search), "/customers")
        })
        .on("search_customers", |ctx: &mut ActionContext<'_>| {
            let search = get_str(ctx.payload, "search").to_string();
            ctx.session.insert("search".into(), search.clone());
            full_patch(ctx.store, &render_table(ctx.store, &search), "/customers")
        })
        .on("search", |ctx: &mut ActionContext<'_>| {
            let search = get_str(ctx.payload, "query").to_string();
            ctx.session.insert("search".into(), search.clone());
            full_patch(ctx.store, &render_table(ctx.store, &search), "/customers")
        })
        .run()
        .await
        .unwrap();
}
