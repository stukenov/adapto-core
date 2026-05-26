use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::PostalData;
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
td.code { font-family: monospace; font-weight: bold; }
td.count { text-align: right; color: #666; }
tr:hover { background: #f9f9f9; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<PostalData> {
    let col = store.collection("postal_codes");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/reference/postal-codes/",
        Lang::Kk => "/kz/reference/postal-codes/",
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
        "<table>\n<tr><th>{}</th><th>{}</th></tr>\n",
        ui.region, ui.count,
    );
    for r in &data.regions {
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}{slug}/\">{name}</a></td><td class=\"count\">{count}</td></tr>\n",
            slug = r.slug,
            name = r.name,
            count = r.codes.len(),
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
        (ui.postal_codes, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.postal_codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
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

pub fn render_region(store: &AdaptoStore, region_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let regions = &data.regions;
    let idx = match regions.iter().position(|r| r.slug == region_slug) {
        Some(i) => i,
        None => return "<h1>Region not found</h1>".to_string(),
    };
    let r = &regions[idx];
    let path = format!("{pfx}{}/", r.slug);

    let mut body = format!(
        "<table>\n<tr><th>{}</th><th>Новый</th><th>Старый</th></tr>\n",
        ui.settlement,
    );
    for code in &r.codes {
        let new_idx = if code.index_new.is_empty() { "—" } else { &code.index_new };
        let old_idx = if code.index_old.is_empty() { "—" } else { &code.index_old };
        let name_cell = if !code.index_new.is_empty() {
            format!(r#"<a href="{path}{idx}/">{name}</a>"#, idx = code.index_new, name = code.name)
        } else {
            code.name.clone()
        };
        body.push_str(&format!(
            "<tr><td>{name_cell}</td><td class=\"code\">{new_idx}</td><td class=\"code\">{old_idx}</td></tr>\n",
        ));
    }
    body.push_str("</table>\n");

    let prev = if idx > 0 { Some(&regions[idx - 1]) } else { None };
    let next = if idx < regions.len() - 1 { Some(&regions[idx + 1]) } else { None };

    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {name}</a>"#, slug = p.slug, name = p.name),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{name} &rarr;</a>"#, slug = n.slug, name = n.name),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.postal_codes, pfx),
        (&r.name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.postal_codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (r.name.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("Почтовые индексы — {}", r.name);
    let desc = format!("Почтовые индексы {} — {} индексов.", r.name, r.codes.len());

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

pub fn render_code(store: &AdaptoStore, region_slug: &str, index: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let region = match data.regions.iter().find(|r| r.slug == region_slug) {
        Some(r) => r,
        None => return "<h1>Region not found</h1>".to_string(),
    };
    let code = match region.codes.iter().find(|c| c.index_new == index) {
        Some(c) => c,
        None => return "<h1>Code not found</h1>".to_string(),
    };
    let path = format!("{pfx}{}/{}/", region.slug, code.index_new);
    let region_path = format!("{pfx}{}/", region.slug);

    let mut body = String::from("<div class=\"card\">\n");
    body.push_str(&format!(
        "<p><span class=\"label\">Новый индекс:</span> <span class=\"val\">{}</span></p>\n",
        code.index_new
    ));
    if !code.index_old.is_empty() {
        body.push_str(&format!(
            "<p><span class=\"label\">Старый индекс:</span> {}</p>\n",
            code.index_old
        ));
    }
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.settlement, code.name
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> <a href=\"{region_path}\">{}</a></p>\n",
        ui.region, region.name
    ));
    body.push_str("</div>\n");

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.postal_codes, pfx),
        (&region.name, &region_path),
        (&code.index_new, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.postal_codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (region.name.clone(), format!("{}{}", html::DOMAIN, region_path)),
        (code.index_new.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("Почтовый индекс {} — {}", code.index_new, code.name);
    let desc = format!(
        "Почтовый индекс {} — {}. Область: {}.",
        code.index_new, code.name, region.name
    );

    html::page(
        lang,
        &format!("{title} — myqaz.kz"),
        &desc,
        &path,
        &nav,
        &format!("<h1>{title}</h1>\n{body}"),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}
