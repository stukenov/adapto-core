use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::{BailiffData, BailiffRegion, Bailiff};
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<BailiffData> {
    let col = store.collection("bailiffs");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/directory/bailiffs/",
        Lang::Kk => "/kz/directory/bailiffs/",
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

    let mut cities: Vec<&BailiffRegion> = Vec::new();
    let mut oblasts: Vec<&BailiffRegion> = Vec::new();
    for r in &data.regions {
        if r.bailiffs.is_empty() { continue; }
        if CITIES.iter().any(|c| r.name.contains(c)) {
            cities.push(r);
        } else {
            oblasts.push(r);
        }
    }
    cities.sort_by(|a, b| a.name.cmp(&b.name));
    oblasts.sort_by(|a, b| a.name.cmp(&b.name));

    let total: usize = data.regions.iter().map(|r| r.bailiffs.len()).sum();
    let mut body = format!("<p>Всего ЧСИ: <b>{total}</b></p>\n");

    body.push_str("<h2>Города</h2>\n<ul>\n");
    for r in &cities {
        body.push_str(&format!(
            "<li><a href=\"{pfx}{slug}/\">{name}</a> ({count})</li>\n",
            slug = r.slug, name = r.name, count = r.bailiffs.len()
        ));
    }
    body.push_str("</ul>\n");

    body.push_str("<h2>Области</h2>\n<ul>\n");
    for r in &oblasts {
        body.push_str(&format!(
            "<li><a href=\"{pfx}{slug}/\">{name}</a> ({count})</li>\n",
            slug = r.slug, name = r.name, count = r.bailiffs.len()
        ));
    }
    body.push_str("</ul>\n");

    let dir_path = format!("{}directory/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.directory, &dir_path),
        (ui.bailiffs, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.directory.to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        (ui.bailiffs.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    let (page_title, page_h1, desc) = match lang {
        Lang::Ru => (
            format!("Частные судебные исполнители Казахстана — все {total} ЧСИ | myqaz.kz"),
            "Частные судебные исполнители Казахстана".to_string(),
            format!("Полный список ЧСИ Казахстана: {total} частных судебных исполнителей в {} регионах. Адреса, телефоны, лицензии.", data.regions.len()),
        ),
        Lang::Kk => (
            format!("Жеке сот орындаушылары Қазақстан Республикасы — барлық {total} ЖСО | myqaz.kz"),
            "Жеке сот орындаушылары Қазақстан Республикасы".to_string(),
            format!("Қазақстан Республикасының жеке сот орындаушыларының толық тізімі: {total} ЖСО."),
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

    let mut sorted = region.bailiffs.clone();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let mut body = format!(
        "<table>\n<tr><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        ui.name, ui.address, ui.phone,
    );
    for b in &sorted {
        let phone = clean_phone(&b.phone);
        body.push_str(&format!(
            "<tr><td><a href=\"{path}{slug}/\">{name}</a></td><td>{addr}</td><td>{phone}</td></tr>\n",
            slug = b.slug,
            name = short_name(&b.name),
            addr = b.address,
        ));
    }
    body.push_str("</table>\n");

    let dir_path = format!("{}directory/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.directory, &dir_path),
        (ui.bailiffs, pfx),
        (&region.name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.directory.to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        (ui.bailiffs.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (region.name.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let count = region.bailiffs.len();
    let desc = format!("Частные судебные исполнители {} — список {} судебных исполнителей.", region.name, count);

    html::page(
        lang,
        &format!("ЧСИ {} — список {} судебных исполнителей | myqaz.kz", region.name, count),
        &desc,
        &path,
        &nav,
        &format!("<h1>ЧСИ: {}</h1>\n{body}", region.name),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_bailiff(store: &AdaptoStore, region_slug: &str, bailiff_slug: &str, lang: Lang) -> String {
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
    let bailiff = match region.bailiffs.iter().find(|b| b.slug == bailiff_slug) {
        Some(b) => b,
        None => return "<h1>Bailiff not found</h1>".to_string(),
    };
    let path = format!("{pfx}{}/{}/", region.slug, bailiff.slug);
    let region_path = format!("{pfx}{}/", region.slug);

    let mut body = String::from("<div class=\"card\">\n");
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.name, bailiff.name
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> <a href=\"{region_path}\">{}</a></p>\n",
        ui.region, region.name
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.address, bailiff.address
    ));
    if !bailiff.phone.is_empty() {
        body.push_str(&format!(
            "<p><span class=\"label\">{}:</span> {}</p>\n",
            ui.phone, clean_phone(&bailiff.phone)
        ));
    }
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> № {} от {}</p>\n",
        ui.license, bailiff.license_number, bailiff.license_date
    ));
    body.push_str("</div>\n");

    let schema = format!(
        r#"{{"@context":"https://schema.org","@type":"LegalService","name":"ЧСИ {}","description":"Частный судебный исполнитель {}. {}, {}","address":{{"@type":"PostalAddress","streetAddress":"{}","addressLocality":"{}"}},"url":"{}{}","areaServed":{{"@type":"AdministrativeArea","name":"{}"}}{}}}"#,
        json_esc(&bailiff.name),
        json_esc(&bailiff.name),
        json_esc(&bailiff.address),
        json_esc(&region.name),
        json_esc(&bailiff.address),
        json_esc(&region.name),
        html::DOMAIN,
        path,
        json_esc(&region.name),
        if bailiff.phone.is_empty() { String::new() } else { format!(r#","telephone":"{}""#, json_esc(&clean_phone(&bailiff.phone))) },
    );

    let dir_path = format!("{}directory/", lang.path_prefix());
    let sn = short_name(&bailiff.name);
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.directory, &dir_path),
        (ui.bailiffs, pfx),
        (&region.name, &region_path),
        (&sn, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.directory.to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        (ui.bailiffs.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (region.name.clone(), format!("{}{}", html::DOMAIN, region_path)),
        (sn.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let (title, desc) = match lang {
        Lang::Ru => (
            format!("ЧСИ {}, {} — адрес, телефон | myqaz.kz", bailiff.name, region.name),
            {
                let mut d = format!("ЧСИ {}, {}.", bailiff.name, region.name);
                if !bailiff.address.is_empty() { d.push_str(&format!(" Адрес: {}.", bailiff.address)); }
                if !bailiff.phone.is_empty() {
                    let pc = clean_phone(&bailiff.phone);
                    if !pc.is_empty() { d.push_str(&format!(" Тел: {pc}.")); }
                }
                d
            },
        ),
        Lang::Kk => (
            format!("ЖСО {}, {} — мекенжай, телефон | myqaz.kz", bailiff.name, region.name),
            {
                let mut d = format!("ЖСО {}, {}.", bailiff.name, region.name);
                if !bailiff.address.is_empty() { d.push_str(&format!(" Мекенжай: {}.", bailiff.address)); }
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
        &format!("<h1>{} {}</h1>\n{body}", match lang { Lang::Ru => "ЧСИ", Lang::Kk => "ЖСО" }, bailiff.name),
        Some(&schema),
        Some(&bc),
        EXTRA_STYLE,
    )
}
