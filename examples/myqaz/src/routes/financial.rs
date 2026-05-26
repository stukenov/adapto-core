use adapto_store::{AdaptoStore, Query as StoreQuery};
use crate::html;
use crate::lang::Lang;
use crate::types::{FinancialIndicatorsData, FinancialYear};
use serde_json::Value;

const EXTRA_STYLE: &str = r#"<style>
td.value { text-align: right; font-weight: bold; }
tr:hover { background: #f9f9f9; }
.current { background: #f0f8ff; font-weight: bold; }
.label { color: #666; font-size: 14px; }
.note { color: #666; font-size: 14px; margin-top: 20px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<FinancialIndicatorsData> {
    let col = store.collection("financial");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn get_field<'a>(yr: &'a FinancialYear, field: &str) -> &'a Option<Value> {
    match field {
        "mrp" => &yr.mrp,
        "mzp" => &yr.mzp,
        "subsistence" => &yr.subsistence,
        _ => &None,
    }
}

fn fmt(v: &Option<Value>) -> String {
    match v {
        Some(Value::Number(n)) => {
            if let Some(i) = n.as_i64() {
                format_int(i)
            } else if let Some(f) = n.as_f64() {
                if f == (f as i64) as f64 {
                    format_int(f as i64)
                } else {
                    let s = format!("{:.2}", f);
                    let parts: Vec<&str> = s.split('.').collect();
                    let int_part = parts[0].parse::<i64>().unwrap_or(0);
                    let dec_part = parts.get(1).unwrap_or(&"00");
                    format!("{},{}", format_int(int_part), dec_part)
                }
            } else {
                "—".to_string()
            }
        }
        _ => "—".to_string(),
    }
}

fn format_int(n: i64) -> String {
    let s = n.abs().to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('\u{00a0}');
        }
        result.push(ch);
    }
    if n < 0 {
        result.push('-');
    }
    result.chars().rev().collect()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/reference/financial-indicators/",
        Lang::Kk => "/kz/reference/financial-indicators/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let c = &data.currency;
    let latest = &data.years[0];
    let indicators = &data.indicators;

    let mut body = String::from("<div class=\"card\">\n");
    for ind in indicators {
        let val = fmt(get_field(latest, &ind.field));
        body.push_str(&format!(
            "<p><span class=\"label\"><a href=\"{pfx}{slug}/\">{short}</a> в {} году:</span> <span class=\"val\">{val} {c}</span></p>\n",
            latest.year, slug = ind.slug, short = ind.title_short
        ));
    }
    body.push_str("</div>\n");

    body.push_str("<table>\n<tr><th>Год</th>");
    for ind in indicators {
        body.push_str(&format!(
            r#"<th><a href="{pfx}{slug}/">{short}, {c}</a></th>"#,
            slug = ind.slug, short = ind.title_short
        ));
    }
    body.push_str("</tr>\n");

    for yr in &data.years {
        let cls = if yr.year == latest.year { " class=\"current\"" } else { "" };
        body.push_str(&format!(
            "<tr{cls}><td><a href=\"{pfx}{y}/\">{y}</a></td>",
            y = yr.year
        ));
        for ind in indicators {
            body.push_str(&format!(
                "<td class=\"value\">{}</td>",
                fmt(get_field(yr, &ind.field))
            ));
        }
        body.push_str("</tr>\n");
    }
    body.push_str("</table>\n");

    let ui = lang.ui();
    body.push_str(&format!(
        "<p class=\"note\">{}: Закон Республики Казахстан «О республиканском бюджете» на соответствующий год.</p>\n",
        ui.source
    ));

    let ref_path = format!("{}reference/", lang.path_prefix());
    let bc_label = ui.financial_indicators;
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (bc_label, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}{}", html::DOMAIN, "/")),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (bc_label.to_string(), format!("{}{}", html::DOMAIN, pfx)),
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

pub fn render_indicator(store: &AdaptoStore, slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let c = &data.currency;
    let latest = &data.years[0];
    let indicators = &data.indicators;
    let idx = match indicators.iter().position(|i| i.slug == slug) {
        Some(i) => i,
        None => return "<h1>Indicator not found</h1>".to_string(),
    };
    let ind = &indicators[idx];
    let field = &ind.field;
    let path = format!("{pfx}{}/", ind.slug);

    let prev = if idx > 0 { Some(&indicators[idx - 1]) } else { None };
    let next = if idx < indicators.len() - 1 { Some(&indicators[idx + 1]) } else { None };

    let mut body = String::from("<div class=\"card\">\n");
    body.push_str(&format!(
        "<p><span class=\"label\">{} в {} году:</span> <span class=\"val\">{} {c}</span></p>\n",
        ind.title_short, latest.year, fmt(get_field(latest, field))
    ));
    body.push_str("</div>\n");
    body.push_str(&format!("<p>{}</p>\n", ind.description));

    body.push_str(&format!(
        "<table>\n<tr><th>Год</th><th>{}, {c}</th><th>Закон</th></tr>\n",
        ind.title_short
    ));
    for yr in &data.years {
        let cls = if yr.year == latest.year { " class=\"current\"" } else { "" };
        let budget = yr.budget_law.as_deref().unwrap_or("");
        body.push_str(&format!(
            "<tr{cls}><td><a href=\"{pfx}{y}/\">{y}</a></td><td class=\"value\">{}</td><td style=\"font-size:13px\">{budget}</td></tr>\n",
            fmt(get_field(yr, field)), y = yr.year
        ));
    }
    body.push_str("</table>\n");

    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {short}</a>"#, slug = p.slug, short = p.title_short),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{short} &rarr;</a>"#, slug = n.slug, short = n.title_short),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let ui = lang.ui();
    let ref_path = format!("{}reference/", lang.path_prefix());
    let bc_label = ui.financial_indicators;
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (bc_label, pfx),
        (&ind.title_short, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (bc_label.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (ind.title_short.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let desc = format!(
        "{} в {} году — {} {}. {}",
        ind.title_short, latest.year, fmt(get_field(latest, field)), ind.unit, ind.description
    );

    html::page(
        lang,
        &format!("{} — myqaz.kz", ind.title),
        &desc,
        &path,
        &nav,
        &format!("<h1>{}</h1>\n{body}\n{nav_bot}", ind.title),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_year(store: &AdaptoStore, year_str: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let year: u32 = match year_str.parse() {
        Ok(y) => y,
        Err(_) => return "<h1>Invalid year</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let c = &data.currency;
    let years = &data.years;
    let indicators = &data.indicators;
    let idx = match years.iter().position(|y| y.year == year) {
        Some(i) => i,
        None => return "<h1>Year not found</h1>".to_string(),
    };
    let yr = &years[idx];

    let prev_yr = if idx < years.len() - 1 { Some(&years[idx + 1]) } else { None };
    let next_yr = if idx > 0 { Some(&years[idx - 1]) } else { None };

    let mut body = String::from("<div class=\"card\">\n");
    for ind in indicators {
        body.push_str(&format!(
            "<p><span class=\"label\"><a href=\"{pfx}{slug}/\">{short}</a>:</span> <span class=\"val\">{} {c}</span></p>\n",
            fmt(get_field(yr, &ind.field)), slug = ind.slug, short = ind.title_short
        ));
    }
    body.push_str("</div>\n");
    let budget = yr.budget_law.as_deref().unwrap_or("");
    body.push_str(&format!(
        "<p class=\"note\">Установлены {}.</p>\n",
        budget
    ));

    let left = match prev_yr {
        Some(p) => format!(r#"<a href="{pfx}{y}/">&larr; {y}</a>"#, y = p.year),
        None => "<span></span>".to_string(),
    };
    let right = match next_yr {
        Some(n) => format!(r#"<a href="{pfx}{y}/">{y} &rarr;</a>"#, y = n.year),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let ui = lang.ui();
    let ref_path = format!("{}reference/", lang.path_prefix());
    let bc_label = ui.financial_indicators;
    let y_str = year.to_string();
    let path = format!("{pfx}{year}/");
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (bc_label, pfx),
        (&y_str, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (bc_label.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (y_str.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("МРП, МЗП, прожиточный минимум в {year} году");
    let desc = format!(
        "МРП в {year} году — {} тенге. МЗП — {} тенге. Прожиточный минимум — {} тенге.",
        fmt(&yr.mrp), fmt(&yr.mzp), fmt(&yr.subsistence)
    );

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

