use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use adapto_store::Query as StoreQuery;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct ClassifiersData {
    oked: Classifier,
    kof: Classifier,
    kpved: Classifier,
    kse_full: Classifier,
}

#[derive(Debug, Deserialize)]
struct Classifier {
    title: String,
    entries: Vec<ClassEntry>,
}

#[derive(Debug, Deserialize)]
struct ClassEntry {
    code: String,
    name: String,
}

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
td.code { font-family: monospace; white-space: nowrap; }
.stats { color: #666; font-size: 14px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<ClassifiersData> {
    let col = store.collection("payment_codes");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/reference/classifiers/",
        Lang::Kk => "/kz/reference/classifiers/",
    }
}

struct ClassInfo {
    slug: &'static str,
    short: &'static str,
}

fn classifiers() -> Vec<ClassInfo> {
    vec![
        ClassInfo { slug: "oked", short: "ОКЭД" },
        ClassInfo { slug: "kof", short: "КОФ" },
        ClassInfo { slug: "kpved", short: "КПВЭД" },
        ClassInfo { slug: "kse", short: "КСЭ" },
    ]
}

fn get_classifier<'a>(data: &'a ClassifiersData, slug: &str) -> Option<&'a Classifier> {
    match slug {
        "oked" => Some(&data.oked),
        "kof" => Some(&data.kof),
        "kpved" => Some(&data.kpved),
        "kse" => Some(&data.kse_full),
        _ => None,
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);

    let items = [
        ("oked", "ОКЭД", &data.oked),
        ("kof", "КОФ", &data.kof),
        ("kpved", "КПВЭД", &data.kpved),
        ("kse", "КСЭ", &data.kse_full),
    ];

    let mut body = String::from("<table>\n<tr><th>Код</th><th>Классификатор</th><th>Записей</th></tr>\n");
    for (slug, short, cl) in &items {
        body.push_str(&format!(
            "<tr><td><b>{short}</b></td><td><a href=\"{pfx}{slug}/\">{title}</a></td><td>{count}</td></tr>\n",
            title = cl.title, count = cl.entries.len(),
        ));
    }
    body.push_str("</table>\n");

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Справочник", &ref_path),
        ("Классификаторы", ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Справочник".to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        ("Классификаторы".to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(lang,
        "Классификаторы РК — ОКЭД, КОФ, КПВЭД, КСЭ — myqaz.kz",
        "Классификаторы Республики Казахстан: ОКЭД, КОФ, КПВЭД, КСЭ.",
        pfx, &nav,
        &format!("<h1>Классификаторы РК</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_classifier(store: &AdaptoStore, slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let cl = match get_classifier(&data, slug) {
        Some(c) => c,
        None => return "<h1>Classifier not found</h1>".to_string(),
    };
    let path = format!("{pfx}{slug}/");

    let mut body = format!("<p class=\"stats\">{} записей</p>\n", cl.entries.len());
    body.push_str("<table>\n<tr><th>Код</th><th>Наименование</th></tr>\n");
    for e in &cl.entries {
        body.push_str(&format!(
            "<tr><td class=\"code\">{code}</td><td>{name}</td></tr>\n",
            code = e.code, name = e.name,
        ));
    }
    body.push_str("</table>\n");

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Справочник", &ref_path),
        ("Классификаторы", pfx),
        (&cl.title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Справочник".to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        ("Классификаторы".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (cl.title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    html::page(lang,
        &format!("{} — myqaz.kz", cl.title),
        &format!("{} — {} записей.", cl.title, cl.entries.len()),
        &path, &nav,
        &format!("<h1>{}</h1>\n{body}", cl.title),
        None, Some(&bc), EXTRA_STYLE)
}
