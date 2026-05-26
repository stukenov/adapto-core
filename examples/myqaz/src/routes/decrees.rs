use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use adapto_store::Query as StoreQuery;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct DecreeIndex {
    doc_id: String,
    title: String,
    #[serde(rename = "type")]
    doc_type: String,
}

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.type-badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 12px; color: #fff; }
.type-U { background: #2196f3; }
.type-P { background: #4caf50; }
.type-V { background: #ff9800; }
.stats { color: #666; font-size: 14px; }
</style>"#;

fn load_decrees(store: &AdaptoStore) -> Vec<DecreeIndex> {
    let col = store.collection("decrees");
    let doc = match col.find(StoreQuery::new()).next() {
        Some(d) => d,
        None => return Vec::new(),
    };
    serde_json::from_value(doc.data).unwrap_or_default()
}

fn type_label(t: &str) -> &str {
    match t {
        "U" => "Указ",
        "P" => "Постановление",
        "V" => "Распоряжение",
        _ => t,
    }
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/law/decrees/",
        Lang::Kk => "/kz/law/decrees/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let decrees = load_decrees(store);
    let pfx = url_prefix(lang);

    let gov_count = decrees.iter().filter(|d| d.doc_type == "P").count();
    let body = format!(
        "<ul><li><a href=\"{pfx}government/\">Правительство</a> — {gov_count}</li></ul>\n"
    );

    let law_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Законодательство", &law_path),
        ("Постановления", ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Законодательство".to_string(), format!("{}{}", html::DOMAIN, law_path)),
        ("Постановления".to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(lang,
        "Постановления — myqaz.kz",
        "Постановления Правительства Республики Казахстан.",
        pfx, &nav,
        &format!("<h1>Постановления</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}
