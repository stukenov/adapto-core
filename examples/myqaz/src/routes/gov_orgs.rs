use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use adapto_store::Query as StoreQuery;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GovOrg {
    #[serde(default)]
    long_name: Option<String>,
    #[serde(default)]
    shortname: Option<String>,
    #[serde(default)]
    slug: Option<String>,
    #[serde(default)]
    phone: Option<String>,
    #[serde(default)]
    about_organization: Option<String>,
    #[serde(default)]
    project_name: Option<String>,
    #[serde(default)]
    project_website: Option<String>,
}

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.org-info dt { font-weight: bold; margin-top: 12px; }
.org-info dd { margin: 4px 0 0 20px; }
.stats { color: #666; font-size: 14px; }
</style>"#;

fn load_orgs(store: &AdaptoStore) -> Vec<GovOrg> {
    let col = store.collection("gov_orgs");
    let doc = match col.find(StoreQuery::new()).next() {
        Some(d) => d,
        None => return Vec::new(),
    };
    match serde_json::from_value(doc.data) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  gov_orgs deser error: {e}");
            Vec::new()
        }
    }
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/directory/government/",
        Lang::Kk => "/kz/directory/government/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let all_orgs = load_orgs(store);
    let orgs: Vec<&GovOrg> = all_orgs.iter()
        .filter(|o| o.long_name.is_some() && o.slug.is_some())
        .collect();
    let pfx = url_prefix(lang);

    let mut body = format!("<p class=\"stats\">{} организаций</p>\n", orgs.len());
    body.push_str("<table>\n<tr><th>Организация</th></tr>\n");
    for org in &orgs {
        let slug = org.slug.as_deref().unwrap_or("");
        let name = org.long_name.as_deref().unwrap_or("");
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}{slug}/\">{name}</a></td></tr>\n",
        ));
    }
    body.push_str("</table>\n");

    let dir_path = format!("{}directory/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Справочник", &dir_path),
        ("Госорганы", ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Справочник".to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        ("Госорганы".to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(lang,
        "Государственные органы Казахстана — myqaz.kz",
        &format!("Государственные органы Республики Казахстан — {} организаций.", orgs.len()),
        pfx, &nav,
        &format!("<h1>Государственные органы</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}

fn strip_html(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_string()
}

pub fn render_org(store: &AdaptoStore, slug: &str, lang: Lang) -> String {
    let orgs = load_orgs(store);
    let pfx = url_prefix(lang);
    let org = match orgs.iter().find(|o| o.slug.as_deref() == Some(slug)) {
        Some(o) => o,
        None => return "<h1>Organization not found</h1>".to_string(),
    };
    let org_name = org.long_name.as_deref().unwrap_or("Организация");
    let path = format!("{pfx}{slug}/");

    let mut body = String::new();
    body.push_str("<dl class=\"org-info\">\n");

    if let Some(ref phone) = org.phone {
        if !phone.is_empty() {
            body.push_str(&format!("<dt>Телефон</dt><dd>{phone}</dd>\n"));
        }
    }
    if let Some(ref website) = org.project_website {
        if !website.is_empty() {
            body.push_str(&format!("<dt>Веб-сайт</dt><dd><a href=\"{website}\">{website}</a></dd>\n"));
        }
    }
    body.push_str("</dl>\n");

    if let Some(ref about) = org.about_organization {
        let clean = strip_html(about);
        if !clean.is_empty() {
            body.push_str(&format!("<h2>Об организации</h2>\n<p>{clean}</p>\n"));
        }
    }

    let dir_path = format!("{}directory/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Справочник", &dir_path),
        ("Госорганы", pfx),
        (org_name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Справочник".to_string(), format!("{}{}", html::DOMAIN, dir_path)),
        ("Госорганы".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (org_name.to_string(), format!("{}{}", html::DOMAIN, path)),
    ];

    let desc = strip_html(org.about_organization.as_deref().unwrap_or(""));
    let desc_trunc = if desc.chars().count() > 200 {
        let end = desc.char_indices().nth(200).map(|(i, _)| i).unwrap_or(desc.len());
        desc[..end].to_string()
    } else { desc };

    html::page(lang,
        &format!("{} — myqaz.kz", org_name),
        &desc_trunc,
        &path, &nav,
        &format!("<h1>{}</h1>\n{body}", org_name),
        None, Some(&bc), EXTRA_STYLE)
}
