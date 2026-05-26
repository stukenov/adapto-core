use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::{NotaryData, NotaryRegion, Notary};
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<NotaryData> {
    let col = store.collection("notaries");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/directory/notaries/",
        Lang::Kk => "/kz/directory/notaries/",
    }
}

fn short_name(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().collect();
    if parts.len() >= 3 {
        format!("{} {}. {}.", parts[0], &parts[1].chars().next().unwrap_or(' '), &parts[2].chars().next().unwrap_or(' '))
    } else if parts.len() == 2 {
        format!("{} {}.", parts[0], &parts[1].chars().next().unwrap_or(' '))
    } else {
        name.to_string()
    }
}

fn json_esc(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn clean_phone(s: &str) -> String {
    s.trim().trim_end_matches(',').trim().to_string()
}

const CITIES: &[&str] = &["Алматы", "Астана", "Шымкент"];

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let mut cities: Vec<&NotaryRegion> = Vec::new();
    let mut oblasts: Vec<&NotaryRegion> = Vec::new();
    for r in &data.regions {
        if r.notaries.is_empty() { continue; }
        if CITIES.iter().any(|c| r.name.contains(c)) {
            cities.push(r);
        } else {
            oblasts.push(r);
        }
    }
    cities.sort_by(|a, b| a.name.cmp(&b.name));
    oblasts.sort_by(|a, b| a.name.cmp(&b.name));

    let total: usize = data.regions.iter().map(|r| r.notaries.len()).sum();
    let mut body = format!("<p>Всего нотариусов: <b>{total}</b></p>\n");

    body.push_str("<h2>Города</h2>\n<ul>\n");
    for r in &cities {
        body.push_str(&format!(
            "<li><a href=\"{pfx}{slug}/\">{name}</a> ({count})</li>\n",
            slug = r.slug, name = r.name, count = r.notaries.len()
        ));
    }
    body.push_str("</ul>\n");

    body.push_str("<h2>Области</h2>\n<ul>\n");
    for r in &oblasts {
        body.push_str(&format!(
            "<li><a href=\"{pfx}{slug}/\">{name}</a> ({count})</li>\n",
            slug = r.slug, name = r.name, count = r.notaries.len()
        ));
    }
    body.push_str("</ul>\n");

    let ref_path = format!("{}directory/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.directory, &ref_path),
        (ui.notaries, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.directory.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.notaries.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    let (page_title, page_h1, desc) = match lang {
        Lang::Ru => (
            format!("Нотариусы Казахстана — все {total} нотариусов по регионам | myqaz.kz"),
            "Нотариусы Казахстана".to_string(),
            format!("Полный список нотариусов Казахстана: {total} нотариусов в {} регионах. Адреса, телефоны, лицензии.", data.regions.len()),
        ),
        Lang::Kk => (
            format!("Нотариустар Қазақстан Республикасы — барлық {total} нотариус | myqaz.kz"),
            "Нотариустар Қазақстан Республикасы".to_string(),
            format!("Қазақстан Республикасының нотариустарының толық тізімі: {total} нотариус.", ),
        ),
    };

    html::page(
        lang,
        &page_title,
        &desc,
        pfx,
        &nav,
        &format!("<h1>{page_h1}</h1>\n{body}"),
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
    let region = match data.regions.iter().find(|r| r.slug == region_slug) {
        Some(r) => r,
        None => return "<h1>Region not found</h1>".to_string(),
    };
    let path = format!("{pfx}{}/", region.slug);

    let mut sorted = region.notaries.clone();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let mut body = format!(
        "<table>\n<tr><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        ui.name, ui.address, ui.phone,
    );
    for n in &sorted {
        let phone = clean_phone(&n.phone);
        body.push_str(&format!(
            "<tr><td><a href=\"{path}{slug}/\">{name}</a></td><td>{addr}</td><td>{phone}</td></tr>\n",
            slug = n.slug,
            name = short_name(&n.name),
            addr = n.address,
        ));
    }
    body.push_str("</table>\n");

    let dir_path = format!("{}directory/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.directory, &dir_path),
        (ui.notaries, pfx),
        (&region.name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.directory.to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        (ui.notaries.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (region.name.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let count = region.notaries.len();
    let desc = format!("Нотариусы {} — список {} нотариусов с адресами и телефонами.", region.name, count);

    html::page(
        lang,
        &format!("Нотариусы {} — список {} нотариусов с адресами | myqaz.kz", region.name, count),
        &desc,
        &path,
        &nav,
        &format!("<h1>Нотариусы: {}</h1>\n{body}", region.name),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_notary(store: &AdaptoStore, region_slug: &str, notary_slug: &str, lang: Lang) -> String {
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
    let notary = match region.notaries.iter().find(|n| n.slug == notary_slug) {
        Some(n) => n,
        None => return "<h1>Notary not found</h1>".to_string(),
    };
    let path = format!("{pfx}{}/{}/", region.slug, notary.slug);
    let region_path = format!("{pfx}{}/", region.slug);

    let mut body = String::from("<div class=\"card\">\n");
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.name, notary.name
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> <a href=\"{region_path}\">{}</a></p>\n",
        ui.region, region.name
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.address, notary.address
    ));
    if !notary.phone.is_empty() {
        body.push_str(&format!(
            "<p><span class=\"label\">{}:</span> {}</p>\n",
            ui.phone, clean_phone(&notary.phone)
        ));
    }
    if !notary.email.is_empty() {
        body.push_str(&format!(
            "<p><span class=\"label\">{}:</span> {}</p>\n",
            ui.email, notary.email
        ));
    }
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> № {} от {}</p>\n",
        ui.license, notary.license_number, notary.license_date
    ));
    body.push_str("</div>\n");

    let schema = format!(
        r#"{{"@context":"https://schema.org","@type":"LegalService","name":"Нотариус {}","description":"Нотариус {}. {}, {}","address":{{"@type":"PostalAddress","streetAddress":"{}","addressLocality":"{}"}},"url":"{}{}","areaServed":{{"@type":"AdministrativeArea","name":"{}"}}{}{}}}"#,
        json_esc(&notary.name),
        json_esc(&notary.name),
        json_esc(&notary.address),
        json_esc(&region.name),
        json_esc(&notary.address),
        json_esc(&region.name),
        html::DOMAIN,
        path,
        json_esc(&region.name),
        if notary.phone.is_empty() { String::new() } else { format!(r#","telephone":"{}""#, json_esc(&clean_phone(&notary.phone))) },
        if notary.email.is_empty() { String::new() } else { format!(r#","email":"{}""#, json_esc(&notary.email)) },
    );

    let dir_path = format!("{}directory/", lang.path_prefix());
    let sn = short_name(&notary.name);
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.directory, &dir_path),
        (ui.notaries, pfx),
        (&region.name, &region_path),
        (&sn, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.directory.to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        (ui.notaries.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (region.name.clone(), format!("{}{}", html::DOMAIN, region_path)),
        (sn.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let (title, desc) = match lang {
        Lang::Ru => (
            format!("Нотариус {}, {} — адрес, телефон | myqaz.kz", notary.name, region.name),
            {
                let mut d = format!("Нотариус {}, {}.", notary.name, region.name);
                if !notary.address.is_empty() { d.push_str(&format!(" Адрес: {}.", notary.address)); }
                if !notary.phone.is_empty() {
                    let pc = clean_phone(&notary.phone);
                    if !pc.is_empty() { d.push_str(&format!(" Тел: {pc}.")); }
                }
                d
            },
        ),
        Lang::Kk => (
            format!("Нотариус {}, {} — мекенжай, телефон | myqaz.kz", notary.name, region.name),
            {
                let mut d = format!("Нотариус {}, {}.", notary.name, region.name);
                if !notary.address.is_empty() { d.push_str(&format!(" Мекенжай: {}.", notary.address)); }
                d
            },
        ),
    };

    html::page(
        lang,
        &title,
        &desc,
        &path,
        &nav,
        &format!("<h1>Нотариус {}</h1>\n{body}", notary.name),
        Some(&schema),
        Some(&bc),
        EXTRA_STYLE,
    )
}
