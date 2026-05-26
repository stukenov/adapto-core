use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use adapto_store::Query as StoreQuery;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ShezhireData {
    eras: Vec<Era>,
}

#[derive(Debug, Deserialize)]
struct Era {
    id: String,
    name: String,
    #[serde(default)]
    name_ru: String,
    #[serde(default)]
    period: String,
    #[serde(default)]
    description: String,
    tribes: Vec<Tribe>,
}

#[derive(Debug, Deserialize)]
struct Tribe {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    subtribes: Vec<String>,
    #[serde(default)]
    territory: String,
    #[serde(default)]
    tamga: String,
    #[serde(default)]
    uran: String,
}

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.era-card { background: #f9f9f9; padding: 16px; border-radius: 8px; margin: 16px 0; }
.era-card h2 { margin: 0 0 8px; font-size: 18px; }
.era-card .period { color: #666; font-size: 14px; margin-bottom: 8px; }
.era-card .desc { font-size: 14px; margin-bottom: 10px; }
.era-card .tribes { font-size: 14px; line-height: 1.8; }
.tribe-detail { margin: 16px 0; }
.tribe-detail dt { font-weight: bold; margin-top: 12px; }
.tribe-detail dd { margin: 4px 0 0 20px; }
.subtribes { display: flex; flex-wrap: wrap; gap: 8px; }
.subtribes span { background: #f0f0f0; padding: 4px 10px; border-radius: 4px; font-size: 14px; }
.subtitle { color: #666; font-size: 15px; margin-bottom: 28px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<ShezhireData> {
    let col = store.collection("shezhire");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/shezhire/",
        Lang::Kk => "/kz/shezhire/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);

    let total_tribes: usize = data.eras.iter().map(|e| e.tribes.len()).sum();
    let mut body = String::new();

    for era in &data.eras {
        body.push_str(&format!(
            "<div class=\"era-card\"><h2><a href=\"{pfx}{id}/\">{name}</a></h2>\n",
            id = era.id, name = era.name,
        ));
        if !era.period.is_empty() {
            let period_with_name = if !era.name_ru.is_empty() {
                format!("{} — {}", era.period, era.name_ru)
            } else {
                era.period.clone()
            };
            body.push_str(&format!("<div class=\"period\">{}</div>\n", period_with_name));
        }
        if !era.description.is_empty() {
            body.push_str(&format!("<div class=\"desc\">{}</div>\n", era.description));
        }
        if !era.tribes.is_empty() {
            body.push_str("<div class=\"tribes\">\n");
            let tribe_links: Vec<String> = era.tribes.iter().map(|t| {
                format!("<a href=\"{pfx}{}/{}/\">{}</a>", era.id, t.id, t.name)
            }).collect();
            body.push_str(&tribe_links.join(" &middot; "));
            body.push_str("\n</div>\n");
        }
        body.push_str("</div>\n");
    }

    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Шежіре", ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Шежіре".to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(lang,
        "Шежіре — история народов Казахстана — myqaz.kz",
        &format!("Шежіре — генеалогия казахского народа. {} эпох, {} племён.", data.eras.len(), total_tribes),
        pfx, &nav,
        &format!("<h1>Шежіре</h1>\n<p>{} эпох, {} племён</p>\n{body}", data.eras.len(), total_tribes),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_era(store: &AdaptoStore, era_id: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let era = match data.eras.iter().find(|e| e.id == era_id) {
        Some(e) => e,
        None => return "<h1>Era not found</h1>".to_string(),
    };
    let path = format!("{pfx}{}/", era.id);

    let mut body = String::new();
    if !era.description.is_empty() {
        body.push_str(&format!("<p>{}</p>\n", era.description));
    }
    if !era.period.is_empty() {
        body.push_str(&format!("<p class=\"period\">{}</p>\n", era.period));
    }

    body.push_str("<table>\n<tr><th>Племя</th><th>Подплемён</th></tr>\n");
    for tribe in &era.tribes {
        body.push_str(&format!(
            "<tr><td><a href=\"{path}{id}/\">{name}</a></td><td>{count}</td></tr>\n",
            id = tribe.id, name = tribe.name, count = tribe.subtribes.len(),
        ));
    }
    body.push_str("</table>\n");

    let title_with_period = if era.period.is_empty() {
        era.name.clone()
    } else {
        format!("{} ({}) ", era.name, era.period)
    };

    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Шежіре", pfx),
        (&era.name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Шежіре".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (era.name.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    html::page(lang,
        &format!("{title_with_period}— Шежіре — myqaz.kz"),
        &format!("{} — {} племён.", era.name, era.tribes.len()),
        &path, &nav,
        &format!("<h1>{}</h1>\n{body}", era.name),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_tribe(store: &AdaptoStore, era_id: &str, tribe_id: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let era = match data.eras.iter().find(|e| e.id == era_id) {
        Some(e) => e,
        None => return "<h1>Era not found</h1>".to_string(),
    };
    let tribe = match era.tribes.iter().find(|t| t.id == tribe_id) {
        Some(t) => t,
        None => return "<h1>Tribe not found</h1>".to_string(),
    };
    let era_path = format!("{pfx}{}/", era.id);
    let path = format!("{pfx}{}/{}/", era.id, tribe.id);

    let mut body = String::new();
    if !tribe.description.is_empty() {
        body.push_str(&format!("<p>{}</p>\n", tribe.description));
    }

    body.push_str("<dl class=\"tribe-detail\">\n");
    if !tribe.territory.is_empty() {
        body.push_str(&format!("<dt>Территория</dt><dd>{}</dd>\n", tribe.territory));
    }
    if !tribe.tamga.is_empty() {
        body.push_str(&format!("<dt>Тамга</dt><dd>{}</dd>\n", tribe.tamga));
    }
    if !tribe.uran.is_empty() {
        body.push_str(&format!("<dt>Ұран</dt><dd>{}</dd>\n", tribe.uran));
    }
    body.push_str("</dl>\n");

    if !tribe.subtribes.is_empty() {
        body.push_str(&format!("<h2>Подплемена ({})</h2>\n<div class=\"subtribes\">\n", tribe.subtribes.len()));
        for st in &tribe.subtribes {
            body.push_str(&format!("<span>{st}</span>\n"));
        }
        body.push_str("</div>\n");
    }

    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Шежіре", pfx),
        (&era.name, &era_path),
        (&tribe.name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Шежіре".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (era.name.clone(), format!("{}{}", html::DOMAIN, era_path)),
        (tribe.name.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let era_label = if era.name.ends_with("Заманы") {
        era.name.clone()
    } else {
        format!("{} заманы", era.name)
    };

    html::page(lang,
        &format!("{} — {} — myqaz.kz", tribe.name, era_label),
        &format!("{} — {} подплемён. {}", tribe.name, tribe.subtribes.len(), {
            let d = &tribe.description;
            if d.chars().count() > 150 {
                let end = d.char_indices().nth(150).map(|(i, _)| i).unwrap_or(d.len());
                &d[..end]
            } else { d }
        }),
        &path, &nav,
        &format!("<h1>{}</h1>\n{body}", tribe.name),
        None, Some(&bc), EXTRA_STYLE)
}
