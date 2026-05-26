use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use adapto_store::Query as StoreQuery;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Company {
    bin: String,
    #[serde(default)]
    nameru: Option<String>,
    #[serde(default)]
    namekz: Option<String>,
    #[serde(default)]
    addressru: Option<String>,
    #[serde(default)]
    addresskz: Option<String>,
    #[serde(default)]
    okedru: Option<String>,
    #[serde(default)]
    director: Option<String>,
    #[serde(default)]
    statusru: Option<String>,
    #[serde(default)]
    datereg: Option<String>,
}

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f5f9ff; }
.count { color: #888; font-size: 13px; }
.company-info dt { font-weight: bold; margin-top: 12px; }
.company-info dd { margin: 4px 0 0 20px; }
.bin { font-family: monospace; }
.status { font-size: 12px; padding: 2px 8px; border-radius: 3px; }
table { width: 100%; border-collapse: collapse; margin: 16px 0; }
th, td { text-align: left; padding: 8px 12px; border-bottom: 1px solid #e0e0e0; font-size: 14px; }
th { background: #f8f8f8; font-weight: 600; }
</style>"#;

fn load_companies(store: &AdaptoStore) -> Vec<Company> {
    let col = store.collection("companies");
    let doc = match col.find(StoreQuery::new()).next() {
        Some(d) => d,
        None => return Vec::new(),
    };
    match serde_json::from_value::<Vec<Company>>(doc.data) {
        Ok(mut v) => {
            v.sort_by(|a, b| {
                a.nameru.as_deref().unwrap_or("").cmp(b.nameru.as_deref().unwrap_or(""))
            });
            v
        }
        Err(e) => {
            eprintln!("  companies deser error: {e}");
            Vec::new()
        }
    }
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/directory/companies",
        Lang::Kk => "/kz/directory/companies",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let companies = load_companies(store);
    let pfx = url_prefix(lang);

    let mut body = format!(
        "<p class=\"search-note\">Показано {} компаний из реестра юридических лиц РК.</p>\n",
        companies.len()
    );

    let mut first_chars: Vec<char> = companies.iter()
        .filter_map(|c| c.nameru.as_ref().and_then(|n| n.chars().next()))
        .collect();
    first_chars.sort();
    first_chars.dedup();

    body.push_str("<div class=\"alpha-nav\">\n");
    for ch in &first_chars {
        body.push_str(&format!("<a href=\"#letter-{ch}\">{ch}</a>\n"));
    }
    body.push_str("</div>\n");

    body.push_str("<table>\n<tr><th>БИН</th><th>Наименование</th><th>Статус</th></tr>\n");
    let mut current_letter = ' ';
    for c in &companies {
        let name = c.nameru.as_deref().unwrap_or("");
        let first = name.chars().next().unwrap_or(' ');
        if first != current_letter {
            current_letter = first;
            body.push_str(&format!(
                "<tr id=\"letter-{current_letter}\"><td colspan=\"3\" style=\"background:#f0f0f0;font-weight:bold;padding:8px\">{current_letter}</td></tr>\n"
            ));
        }
        let status = c.statusru.as_deref().unwrap_or("");
        body.push_str(&format!(
            "<tr><td class=\"bin\"><a href=\"{pfx}/{bin}\">{bin}</a></td><td>{name}</td><td>{status}</td></tr>\n",
            bin = c.bin,
        ));
    }
    body.push_str("</table>\n");

    let dir_path = format!("{}directory/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Справочник", &dir_path),
        ("Компании", ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Справочник".to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        ("Компании".to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(lang,
        "Реестр юридических лиц РК — myqaz.kz",
        &format!("Реестр юридических лиц Республики Казахстан — {} компаний.", companies.len()),
        pfx, &nav,
        &format!("<h1>Реестр юридических лиц</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}

fn render_month(store: &AdaptoStore, ym: &str, lang: Lang) -> String {
    let companies = load_companies(store);
    let pfx = url_prefix(lang);
    let month_num: usize = ym[5..].parse().unwrap_or(0);
    let year = &ym[..4];
    let month_name = MONTH_NAMES.get(month_num).copied().unwrap_or("?");

    let filtered: Vec<&Company> = companies.iter()
        .filter(|c| c.datereg.as_deref().unwrap_or("").starts_with(ym))
        .collect();

    let path = format!("{pfx}/{ym}");

    let mut body = format!(
        "<p class=\"count\">{} юридических лиц</p>\n",
        filtered.len()
    );

    body.push_str("<table>\n<tr><th>БИН</th><th>Наименование</th><th>Руководитель</th><th>Статус</th></tr>\n");
    for c in &filtered {
        let name = c.nameru.as_deref().unwrap_or("");
        let director = c.director.as_deref().unwrap_or("");
        let status = c.statusru.as_deref().unwrap_or("");
        let status_style = if status.contains("Зарегистрирован") {
            "background:#2e7d3222;color:#2e7d32"
        } else {
            "background:#e0e0e0;color:#666"
        };
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}/{bin}\">{bin}</a></td><td><a href=\"{pfx}/{bin}\">{name}</a></td><td>{director}</td><td><span class=\"status\" style=\"{status_style}\">{status}</span></td></tr>\n",
            bin = c.bin,
        ));
    }
    body.push_str("</table>\n");

    let dir_path = format!("{}directory/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Справочник", &dir_path),
        ("Компании", pfx),
        (&format!("{month_name} {year}"), ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Справочник".to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        ("Компании".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (format!("{month_name} {year}"), format!("{}{}", html::DOMAIN, path)),
    ];

    let month_lower = month_name.to_lowercase();
    html::page(lang,
        &format!("Компании за {month_lower} {year} — myqaz.kz"),
        &format!("Юридические лица Казахстана, зарегистрированные в {month_lower} {year} года. {} компаний.", filtered.len()),
        &path, &nav,
        &format!("<h1>{month_name} {year}</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}

fn is_year_month(s: &str) -> bool {
    s.len() == 7 && s.as_bytes()[4] == b'-'
        && s[..4].chars().all(|c| c.is_ascii_digit())
        && s[5..].chars().all(|c| c.is_ascii_digit())
}

static MONTH_NAMES: &[&str] = &[
    "", "Январь", "Февраль", "Март", "Апрель", "Май", "Июнь",
    "Июль", "Август", "Сентябрь", "Октябрь", "Ноябрь", "Декабрь",
];

pub fn render_company(store: &AdaptoStore, bin: &str, lang: Lang) -> String {
    if is_year_month(bin) {
        return render_month(store, bin, lang);
    }
    let companies = load_companies(store);
    let pfx = url_prefix(lang);
    let company = match companies.iter().find(|c| c.bin == bin) {
        Some(c) => c,
        None => return "<h1>Компания не найдена</h1>".to_string(),
    };
    let path = format!("{pfx}/{bin}");
    let name = company.nameru.as_deref().unwrap_or("Компания");

    let mut body = String::from("<dl class=\"company-info\">\n");
    body.push_str(&format!("<dt>БИН</dt><dd class=\"bin\">{bin}</dd>\n"));
    if let Some(ref n) = company.namekz {
        if !n.is_empty() {
            body.push_str(&format!("<dt>Наименование (каз.)</dt><dd>{n}</dd>\n"));
        }
    }
    if let Some(ref addr) = company.addressru {
        if !addr.is_empty() {
            body.push_str(&format!("<dt>Адрес</dt><dd>{addr}</dd>\n"));
        }
    }
    if let Some(ref oked) = company.okedru {
        if !oked.is_empty() {
            body.push_str(&format!("<dt>ОКЭД</dt><dd>{oked}</dd>\n"));
        }
    }
    if let Some(ref dir) = company.director {
        if !dir.is_empty() {
            body.push_str(&format!("<dt>Руководитель</dt><dd>{dir}</dd>\n"));
        }
    }
    if let Some(ref status) = company.statusru {
        if !status.is_empty() {
            body.push_str(&format!("<dt>Статус</dt><dd>{status}</dd>\n"));
        }
    }
    if let Some(ref date) = company.datereg {
        if !date.is_empty() {
            body.push_str(&format!("<dt>Дата регистрации</dt><dd>{date}</dd>\n"));
        }
    }
    body.push_str("</dl>\n");

    let dir_path = format!("{}directory/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Справочник", &dir_path),
        ("Компании", pfx),
        (name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Справочник".to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        ("Компании".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (name.to_string(), format!("{}{}", html::DOMAIN, path)),
    ];

    html::page(lang,
        &format!("{name} — БИН {bin} — myqaz.kz"),
        &format!("{name} — {} — {}", company.okedru.as_deref().unwrap_or(""), company.addressru.as_deref().unwrap_or("")),
        &path, &nav,
        &format!("<h1>{name}</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}
