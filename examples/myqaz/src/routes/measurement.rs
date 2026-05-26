use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::{MeasurementData, MeasurementGroup, MeasurementUnit};
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
td.code { font-family: monospace; font-weight: bold; }
td.count { text-align: right; color: #666; }
tr:hover { background: #f9f9f9; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<MeasurementData> {
    let col = store.collection("measurement");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/reference/measurement-units/",
        Lang::Kk => "/kz/reference/measurement-units/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let mut body = format!(
        "<table>\n<tr><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        ui.group, ui.code, ui.count,
    );
    for g in &data.groups {
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}{slug}/\">{title}</a></td><td class=\"code\">{code_range}</td><td class=\"count\">{count}</td></tr>\n",
            slug = g.slug,
            title = g.title,
            code_range = g.code_range,
            count = g.items.len(),
        ));
    }
    body.push_str("</table>\n");
    body.push_str(&format!(
        "<p class=\"note\">{}: {}.</p>\n",
        ui.source, data.source
    ));

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.measurement_units, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.measurement_units.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(
        lang,
        &format!("{} — myqaz.kz", data.index_title),
        &data.index_description,
        pfx,
        &nav,
        &format!("<h1>{}</h1>\n{body}", data.index_title),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_group(store: &AdaptoStore, group_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let groups = &data.groups;
    let idx = match groups.iter().position(|g| g.slug == group_slug) {
        Some(i) => i,
        None => return "<h1>Group not found</h1>".to_string(),
    };
    let g = &groups[idx];
    let path = format!("{pfx}{}/", g.slug);

    let mut body = format!(
        "<table>\n<tr><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        ui.code, ui.name, ui.designation, ui.designation_intl,
    );
    for item in &g.items {
        body.push_str(&format!(
            "<tr><td class=\"code\"><a href=\"{path}{code}/\">{code}</a></td><td>{name}</td><td>{sym_ru}</td><td>{sym_int}</td></tr>\n",
            code = item.code,
            name = item.name,
            sym_ru = item.symbol_ru,
            sym_int = item.symbol_int,
        ));
    }
    body.push_str("</table>\n");

    let left = if idx > 0 {
        let prev = &groups[idx - 1];
        format!(r#"<a href="{pfx}{slug}/">&larr; {title}</a>"#, slug = prev.slug, title = prev.title)
    } else {
        "<span></span>".to_string()
    };
    let right = if idx < groups.len() - 1 {
        let next = &groups[idx + 1];
        format!(r#"<a href="{pfx}{slug}/">{title} &rarr;</a>"#, slug = next.slug, title = next.title)
    } else {
        "<span></span>".to_string()
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.measurement_units, pfx),
        (&g.title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.measurement_units.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (g.title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("{} — МКЕИ", g.title);
    let desc = format!("{}. Коды {}. {}.", g.title, g.code_range, data.source);

    html::page(
        lang,
        &format!("{title} — myqaz.kz"),
        &desc,
        &path,
        &nav,
        &format!("<h1>{title}</h1>\n{body}\n{nav_bot}"),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_item(store: &AdaptoStore, group_slug: &str, item_code: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let group = match data.groups.iter().find(|g| g.slug == group_slug) {
        Some(g) => g,
        None => return "<h1>Group not found</h1>".to_string(),
    };
    let item_idx = match group.items.iter().position(|it| it.code == item_code) {
        Some(i) => i,
        None => return "<h1>Item not found</h1>".to_string(),
    };
    let item = &group.items[item_idx];
    let code = &item.code;
    let path = format!("{pfx}{}/{code}/", group.slug);

    let mut body = String::from("<div class=\"card\">\n");
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> <span class=\"val\">{code}</span></p>\n",
        ui.code
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.name, item.name
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.designation, item.symbol_ru
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.designation_intl, item.symbol_int
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> <a href=\"{pfx}{slug}/\">{title}</a></p>\n",
        ui.group, slug = group.slug, title = group.title
    ));
    body.push_str("</div>\n");

    let parent_path = format!("{pfx}{}/", group.slug);
    let prev = if item_idx > 0 { Some(&group.items[item_idx - 1]) } else { None };
    let next = if item_idx < group.items.len() - 1 { Some(&group.items[item_idx + 1]) } else { None };

    let left = match prev {
        Some(p) => format!(r#"<a href="{parent_path}{code}/">&larr; {code} {name}</a>"#, code = p.code, name = p.name),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{parent_path}{code}/">{code} {name} &rarr;</a>"#, code = n.code, name = n.name),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let crumb_last = format!("{code} {}", item.name);
    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.measurement_units, pfx),
        (&group.title, &parent_path),
        (&crumb_last, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.measurement_units.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (group.title.clone(), format!("{}{}", html::DOMAIN, parent_path)),
        (crumb_last.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("Код {code} — {} ({}) — МКЕИ", item.name, item.symbol_ru);
    let desc = format!("Код единицы измерения {code} — {} ({} / {}). {}.", item.name, item.symbol_ru, item.symbol_int, data.source);
    let h1 = format!("Код {code} — {}", item.name);

    html::page(
        lang,
        &format!("{title} — myqaz.kz"),
        &desc,
        &path,
        &nav,
        &format!("<h1>{h1}</h1>\n{body}\n{nav_bot}"),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}
