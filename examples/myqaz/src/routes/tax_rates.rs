use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::{TaxData, TaxItem, TaxSubItem};
use adapto_store::Query as StoreQuery;
use serde_json::Value;

const EXTRA_STYLE: &str = r#"<style>
td.code { font-family: monospace; font-weight: bold; }
.rate { text-align: right; }
tr:hover { background: #f9f9f9; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
.note { color: #666; font-size: 14px; margin-top: 20px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<TaxData> {
    let col = store.collection("tax_rates");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/reference/tax-rates/",
        Lang::Kk => "/kz/reference/tax-rates/",
    }
}

fn article_url(lang: Lang, chapter: &Option<Value>, article: &Option<Value>) -> String {
    let ch = match chapter {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => return String::new(),
    };
    let art = match article {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => return String::new(),
    };
    format!("{}law/codes/tax/chapter-{ch}/article-{art}/", lang.path_prefix())
}

fn article_link(lang: Lang, chapter: &Option<Value>, article: &Option<Value>) -> String {
    let url = article_url(lang, chapter, article);
    if url.is_empty() {
        return String::new();
    }
    let art = match article {
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::String(s)) => s.clone(),
        _ => return String::new(),
    };
    format!(r#"<a href="{url}">ст. {art}</a>"#)
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let mut body = String::new();
    for cat in &data.categories {
        body.push_str(&format!("<h2>{}</h2>\n", cat.title));
        body.push_str("<table>\n<tr><th>Налог</th><th>Ставка</th><th>Статья НК</th></tr>\n");
        for item in &cat.items {
            let rate = item.rate.as_deref().unwrap_or("—");
            let art_link = article_link(lang, &item.chapter, &item.article);
            body.push_str(&format!(
                "<tr><td><a href=\"{pfx}{slug}/\">{title}</a></td><td class=\"rate\">{rate}</td><td>{art_link}</td></tr>\n",
                slug = item.slug,
                title = item.title,
            ));
        }
        body.push_str("</table>\n");
    }
    body.push_str(&format!(
        "<p class=\"note\">{}: {}.</p>\n",
        ui.source, data.source
    ));

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.tax_rates, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.tax_rates.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    let title = "Налоговые ставки в Казахстане — myqaz.kz";
    let desc = format!("Налоговые ставки в Казахстане на {} год. МРП: {} тенге.", data.year, data.mrp);

    html::page(
        lang,
        title,
        &desc,
        pfx,
        &nav,
        &format!("<h1>Налоговые ставки в Казахстане</h1>\n{body}"),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_item(store: &AdaptoStore, item_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let mut found_item: Option<&TaxItem> = None;
    let mut found_idx = 0usize;
    let mut all_items: Vec<&TaxItem> = Vec::new();
    for cat in &data.categories {
        for item in &cat.items {
            all_items.push(item);
        }
    }
    for (i, item) in all_items.iter().enumerate() {
        if item.slug == item_slug {
            found_item = Some(item);
            found_idx = i;
            break;
        }
    }
    let item = match found_item {
        Some(it) => it,
        None => return "<h1>Tax not found</h1>".to_string(),
    };
    let path = format!("{pfx}{}/", item.slug);
    let title_display = item.title_long.as_deref().unwrap_or(&item.title);

    let mut body = String::from("<div class=\"card\">\n");
    if let Some(rate) = &item.rate {
        body.push_str(&format!(
            "<p><span class=\"label\">Ставка:</span> <span class=\"val\">{rate}</span></p>\n"
        ));
    }
    let art_link = article_link(lang, &item.chapter, &item.article);
    if !art_link.is_empty() {
        body.push_str(&format!(
            "<p><span class=\"label\">Статья НК:</span> {art_link}</p>\n"
        ));
    }
    if let Some(payers) = &item.payers {
        body.push_str(&format!(
            "<p><span class=\"label\">Плательщики:</span> {payers}</p>\n"
        ));
    }
    body.push_str("</div>\n");

    if !item.sub_items.is_empty() {
        for sub in &item.sub_items {
            body.push_str(&format!("<h2><a href=\"{path}{slug}/\">{title}</a></h2>\n",
                slug = sub.slug, title = sub.title));
            if !sub.items.is_empty() {
                body.push_str("<table>\n<tr><th>Наименование</th><th>Ставка</th></tr>\n");
                for leaf in &sub.items {
                    let lr = leaf.rate.as_deref().unwrap_or("—");
                    body.push_str(&format!(
                        "<tr><td>{}</td><td class=\"rate\">{lr}</td></tr>\n",
                        leaf.name
                    ));
                }
                body.push_str("</table>\n");
            }
        }
    }

    if !item.items.is_empty() {
        body.push_str("<table>\n<tr><th>Наименование</th><th>Ставка</th></tr>\n");
        for leaf in &item.items {
            let lr = leaf.rate.as_deref().unwrap_or("—");
            body.push_str(&format!(
                "<tr><td>{}</td><td class=\"rate\">{lr}</td></tr>\n",
                leaf.name
            ));
        }
        body.push_str("</table>\n");
    }

    let prev = if found_idx > 0 { Some(all_items[found_idx - 1]) } else { None };
    let next = if found_idx < all_items.len() - 1 { Some(all_items[found_idx + 1]) } else { None };

    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {title}</a>"#, slug = p.slug, title = p.title),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{title} &rarr;</a>"#, slug = n.slug, title = n.title),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.tax_rates, pfx),
        (&item.title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.tax_rates.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (item.title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let desc = format!("{}: {}. {}", item.title, item.rate.as_deref().unwrap_or("—"), title_display);

    html::page(
        lang,
        &format!("{} — myqaz.kz", title_display),
        &desc,
        &path,
        &nav,
        &format!("<h1>{title_display}</h1>\n{body}\n{nav_bot}"),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_sub_item(store: &AdaptoStore, item_slug: &str, sub_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let mut found_item: Option<&TaxItem> = None;
    for cat in &data.categories {
        for item in &cat.items {
            if item.slug == item_slug {
                found_item = Some(item);
                break;
            }
        }
    }
    let item = match found_item {
        Some(it) => it,
        None => return "<h1>Tax not found</h1>".to_string(),
    };
    let sub = match item.sub_items.iter().find(|s| s.slug == sub_slug) {
        Some(s) => s,
        None => return "<h1>Sub-item not found</h1>".to_string(),
    };
    let path = format!("{pfx}{}/{}/", item.slug, sub.slug);
    let item_path = format!("{pfx}{}/", item.slug);
    let title_display = sub.title_long.as_deref().unwrap_or(&sub.title);

    let mut body = String::from("<div class=\"card\">\n");
    if let Some(rate) = &sub.rate {
        body.push_str(&format!(
            "<p><span class=\"label\">Ставка:</span> <span class=\"val\">{rate}</span></p>\n"
        ));
    }
    let art_link = article_link(lang, &sub.chapter, &sub.article);
    if !art_link.is_empty() {
        body.push_str(&format!(
            "<p><span class=\"label\">Статья НК:</span> {art_link}</p>\n"
        ));
    }
    body.push_str("</div>\n");

    if !sub.items.is_empty() {
        body.push_str("<table>\n<tr><th>Наименование</th><th>Ставка</th></tr>\n");
        for leaf in &sub.items {
            let lr = leaf.rate.as_deref().unwrap_or("—");
            body.push_str(&format!(
                "<tr><td>{}</td><td class=\"rate\">{lr}</td></tr>\n",
                leaf.name
            ));
        }
        body.push_str("</table>\n");
    }

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.tax_rates, pfx),
        (&item.title, &item_path),
        (&sub.title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.tax_rates.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (item.title.clone(), format!("{}{}", html::DOMAIN, item_path)),
        (sub.title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let desc = format!("{}: {}. {}", sub.title, sub.rate.as_deref().unwrap_or("—"), title_display);

    html::page(
        lang,
        &format!("{} — myqaz.kz", sub.title),
        &desc,
        &path,
        &nav,
        &format!("<h1>{title_display}</h1>\n{body}"),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}
