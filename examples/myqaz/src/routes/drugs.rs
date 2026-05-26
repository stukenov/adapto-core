use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use adapto_store::Query as StoreQuery;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Drug {
    id: u64,
    #[serde(default)]
    reg_number: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, rename = "producerNameRu")]
    producer: Option<String>,
    #[serde(default, rename = "countryNameRu")]
    country: Option<String>,
    #[serde(default)]
    internationalnames: Option<String>,
    #[serde(default)]
    dosage_form_name: Option<String>,
    #[serde(default)]
    dosage_value: Option<f64>,
    #[serde(default, rename = "dosageMeasure")]
    dosage_measure: Option<String>,
    #[serde(default)]
    reg_date: Option<String>,
    #[serde(default)]
    recipe_sign: Option<bool>,
}

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.stats { color: #666; font-size: 14px; }
td.code { font-family: monospace; font-size: 13px; white-space: nowrap; }
.drug-info dt { font-weight: bold; margin-top: 12px; }
.drug-info dd { margin: 4px 0 0 20px; }
.rx { color: #e53935; font-weight: bold; }
.otc { color: #43a047; }
</style>"#;

fn load_drugs(store: &AdaptoStore) -> Vec<Drug> {
    let col = store.collection("drugs");
    let doc = match col.find(StoreQuery::new()).next() {
        Some(d) => d,
        None => return Vec::new(),
    };
    match serde_json::from_value(doc.data) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("  drugs deser error: {e}");
            Vec::new()
        }
    }
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/reference/drugs/",
        Lang::Kk => "/kz/reference/drugs/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let drugs = load_drugs(store);
    let pfx = url_prefix(lang);

    let mut body = format!("<p class=\"stats\">{} препаратов</p>\n", drugs.len());
    body.push_str("<table>\n<tr><th>Название</th><th>Производитель</th><th>Страна</th></tr>\n");
    for d in &drugs {
        let name = d.name.as_deref().unwrap_or("");
        let producer = d.producer.as_deref().unwrap_or("");
        let country = d.country.as_deref().unwrap_or("");
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}{id}/\">{name}</a></td><td>{producer}</td><td>{country}</td></tr>\n",
            id = d.id,
        ));
    }
    body.push_str("</table>\n");

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Справочник", &ref_path),
        ("Лекарства", ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Справочник".to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        ("Лекарства".to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    let (page_title, page_h1) = match lang {
        Lang::Ru => (
            "Реестр лекарственных средств — myqaz.kz",
            "Реестр лекарственных средств",
        ),
        Lang::Kk => (
            "Дәрілік заттар тізілімі — myqaz.kz",
            "Дәрілік заттар тізілімі",
        ),
    };

    html::page(lang,
        page_title,
        &format!("Реестр зарегистрированных лекарственных средств Казахстана — {} препаратов.", drugs.len()),
        pfx, &nav,
        &format!("<h1>{page_h1}</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_drug(store: &AdaptoStore, drug_id: &str, lang: Lang) -> String {
    let drugs = load_drugs(store);
    let pfx = url_prefix(lang);
    let id: u64 = match drug_id.parse() {
        Ok(n) => n,
        Err(_) => return "<h1>Invalid ID</h1>".to_string(),
    };
    let drug = match drugs.iter().find(|d| d.id == id) {
        Some(d) => d,
        None => return "<h1>Drug not found</h1>".to_string(),
    };
    let path = format!("{pfx}{}/", drug.id);

    let drug_name = drug.name.as_deref().unwrap_or("Препарат");
    let producer = drug.producer.as_deref().unwrap_or("");
    let country = drug.country.as_deref().unwrap_or("");

    let mut body = String::from("<dl class=\"drug-info\">\n");
    if let Some(ref rn) = drug.reg_number {
        body.push_str(&format!("<dt>Регистрационный номер</dt><dd>{rn}</dd>\n"));
    }
    if !producer.is_empty() {
        body.push_str(&format!("<dt>Производитель</dt><dd>{producer}</dd>\n"));
    }
    if !country.is_empty() {
        body.push_str(&format!("<dt>Страна</dt><dd>{country}</dd>\n"));
    }
    if let Some(ref inn) = drug.internationalnames {
        if !inn.is_empty() {
            body.push_str(&format!("<dt>МНН</dt><dd>{inn}</dd>\n"));
        }
    }
    if let Some(ref form) = drug.dosage_form_name {
        if !form.is_empty() {
            body.push_str(&format!("<dt>Лекарственная форма</dt><dd>{form}</dd>\n"));
        }
    }
    if let Some(val) = drug.dosage_value {
        let measure = drug.dosage_measure.as_deref().unwrap_or("");
        body.push_str(&format!("<dt>Дозировка</dt><dd>{val} {measure}</dd>\n"));
    }
    if let Some(ref date) = drug.reg_date {
        if !date.is_empty() {
            let short = &date[..10.min(date.len())];
            body.push_str(&format!("<dt>Дата регистрации</dt><dd>{short}</dd>\n"));
        }
    }
    match drug.recipe_sign {
        Some(true) => body.push_str("<dt>Отпуск</dt><dd class=\"rx\">По рецепту</dd>\n"),
        Some(false) => body.push_str("<dt>Отпуск</dt><dd class=\"otc\">Без рецепта</dd>\n"),
        _ => {}
    }
    body.push_str("</dl>\n");

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Справочник", &ref_path),
        ("Лекарства", pfx),
        (drug_name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Справочник".to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        ("Лекарства".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (drug_name.to_string(), format!("{}{}", html::DOMAIN, path)),
    ];

    html::page(lang,
        &format!("{drug_name} — Лекарства — myqaz.kz"),
        &format!("{drug_name} — {producer} — {country}"),
        &path, &nav,
        &format!("<h1>{drug_name}</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}
