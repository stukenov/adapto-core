use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::{PhoneData, PhoneItem};
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
td.code { font-family: monospace; font-weight: bold; }
tr:hover { background: #f9f9f9; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<PhoneData> {
    let col = store.collection("phone_codes");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/reference/phone-codes/",
        Lang::Kk => "/kz/reference/phone-codes/",
    }
}

fn all_items(data: &PhoneData) -> Vec<&PhoneItem> {
    data.categories.iter().flat_map(|c| c.items.iter()).collect()
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
        body.push_str(&format!(
            "<table>\n<tr><th>{}</th><th>{}</th><th>{}</th></tr>\n",
            ui.name, ui.code, ui.region,
        ));
        for item in &cat.items {
            body.push_str(&format!(
                "<tr><td><a href=\"{pfx}{slug}/\">{city}</a></td><td class=\"code\">{code}</td><td>{region}</td></tr>\n",
                slug = item.slug,
                city = item.city,
                code = item.code,
                region = item.region,
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
        (ui.phone_codes, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.phone_codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
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

pub fn render_city(store: &AdaptoStore, slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let items = all_items(&data);
    let idx = match items.iter().position(|it| it.slug == slug) {
        Some(i) => i,
        None => return "<h1>City not found</h1>".to_string(),
    };
    let item = items[idx];
    let path = format!("{pfx}{}/", item.slug);

    let mut body = String::from("<div class=\"card\">\n");
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> <span class=\"val\">{}</span></p>\n",
        ui.code, item.code
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.country_code, data.country_code
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.number_format, data.format
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.region, item.region
    ));
    body.push_str("</div>\n");

    let prev = if idx > 0 { Some(items[idx - 1]) } else { None };
    let next = if idx < items.len() - 1 { Some(items[idx + 1]) } else { None };

    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {city}</a>"#, slug = p.slug, city = p.city),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{city} &rarr;</a>"#, slug = n.slug, city = n.city),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.phone_codes, pfx),
        (&item.city, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.phone_codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (item.city.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let desc = format!(
        "Телефонные коды {} — {}. Формат номера: +7 ({}) XXX-XX-XX.",
        item.city, item.code, item.code
    );

    html::page(
        lang,
        &format!("Телефонные коды {} — {} — myqaz.kz", item.city, item.code),
        &desc,
        &path,
        &nav,
        &format!("<h1>Телефонные коды {}</h1>\n{body}\n{nav_bot}", item.city),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}
