use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use adapto_store::Query as StoreQuery;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ConstitutionData {
    title: String,
    sections: Vec<Section>,
}

#[derive(Debug, Deserialize)]
struct Section {
    number: u32,
    slug: String,
    title: String,
    articles: Vec<ConArticle>,
}

#[derive(Debug, Deserialize)]
struct ConArticle {
    number: u32,
    slug: String,
    #[serde(default)]
    title: String,
    points: Vec<ConPoint>,
}

#[derive(Debug, Deserialize)]
struct ConPoint {
    number: String,
    text: String,
}

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.point { margin: 15px 0; padding: 10px 0; border-bottom: 1px solid #eee; }
.point-num { font-weight: bold; }
.card p { margin: 8px 0; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<ConstitutionData> {
    let col = store.collection("constitution");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/law/constitution/",
        Lang::Kk => "/kz/law/constitution/",
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let end = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
        format!("{}...", &s[..end])
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let total_articles: usize = data.sections.iter().map(|s| s.articles.len()).sum();
    let mut body = String::new();

    for sec in &data.sections {
        let section_url = format!("{pfx}{}/", sec.slug);
        body.push_str(&format!(
            "<h2><a href=\"{section_url}\">{} {}. {}</a></h2>\n<ul>\n",
            ui.section, sec.number, sec.title
        ));
        for art in &sec.articles {
            let article_url = format!("{section_url}{}/", art.slug);
            let title_part = if !art.title.is_empty() {
                format!(" — {}", art.title)
            } else {
                String::new()
            };
            body.push_str(&format!(
                "<li><a href=\"{article_url}\">{} {}</a>{title_part}</li>\n",
                ui.article, art.number,
            ));
        }
        body.push_str("</ul>\n");
    }

    let law_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_path),
        (ui.constitution, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_path)),
        (ui.constitution.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(lang,
        "Конституция Республики Казахстан — myqaz.kz",
        &format!("Конституция Республики Казахстан — {} разделов, {} статей.", data.sections.len(), total_articles),
        pfx, &nav,
        &format!("<h1>{}</h1>\n<p>{} разделов, {} статей</p>\n{body}", data.title, data.sections.len(), total_articles),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_section(store: &AdaptoStore, section_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let sec_idx = match data.sections.iter().position(|s| s.slug == section_slug) {
        Some(i) => i,
        None => return "<h1>Section not found</h1>".to_string(),
    };
    let sec = &data.sections[sec_idx];
    let section_url = format!("{pfx}{}/", sec.slug);

    let mut body = String::from("<ul>\n");
    for art in &sec.articles {
        let article_url = format!("{section_url}{}/", art.slug);
        let first_text = if !art.points.is_empty() {
            truncate_chars(&art.points[0].text, 120)
        } else {
            String::new()
        };
        body.push_str(&format!(
            "<li><a href=\"{article_url}\">{} {}</a> — {first_text}</li>\n",
            ui.article, art.number,
        ));
    }
    body.push_str("</ul>\n");

    let prev = if sec_idx > 0 { Some(&data.sections[sec_idx - 1]) } else { None };
    let next = if sec_idx < data.sections.len() - 1 { Some(&data.sections[sec_idx + 1]) } else { None };
    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {} {}</a>"#, ui.section, p.number, slug = p.slug),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{} {} &rarr;</a>"#, ui.section, n.number, slug = n.slug),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let rk_short = match lang { Lang::Ru => "РК", Lang::Kk => "ҚР" };
    let title = format!("{} {}. {} — {} {} — myqaz.kz", ui.section, sec.number, sec.title, ui.constitution, rk_short);
    let h1 = format!("{} {} {} {}. {}", ui.section, sec.number, ui.constitution, rk_short, sec.title);

    let law_path = format!("{}law/", lang.path_prefix());
    let crumb_label = format!("{} {}", ui.section, sec.number);
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_path),
        (ui.constitution, pfx),
        (&crumb_label, ""),
    ], lang, &section_url);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_path)),
        (ui.constitution.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (crumb_label.clone(), format!("{}{}", html::DOMAIN, section_url)),
    ];

    let first_anum = sec.articles.first().map(|a| a.number).unwrap_or(0);
    let last_anum = sec.articles.last().map(|a| a.number).unwrap_or(0);
    let desc = format!("{} {} {} {}: {}. {} {}-{}.",
        ui.section, sec.number, ui.constitution, rk_short, sec.title, ui.article, first_anum, last_anum);

    html::page(lang, &title, &desc, &section_url, &nav,
        &format!("<h1>{h1}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_article(store: &AdaptoStore, section_slug: &str, article_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let sec = match data.sections.iter().find(|s| s.slug == section_slug) {
        Some(s) => s,
        None => return "<h1>Section not found</h1>".to_string(),
    };
    let art_idx = match sec.articles.iter().position(|a| a.slug == article_slug) {
        Some(i) => i,
        None => return "<h1>Article not found</h1>".to_string(),
    };
    let art = &sec.articles[art_idx];
    let section_url = format!("{pfx}{}/", sec.slug);
    let path = format!("{section_url}{}/", art.slug);

    let mut body = String::new();
    for pt in &art.points {
        if pt.number == "0" {
            body.push_str(&format!("<p>{}</p>\n", pt.text));
        } else {
            body.push_str(&format!(
                "<div class=\"point\"><p><span class=\"point-num\"><a href=\"{path}#p{num}\">Пункт {num}.</a></span> {text}</p></div>\n",
                num = pt.number, text = pt.text,
            ));
        }
    }

    let prev = if art_idx > 0 { Some(&sec.articles[art_idx - 1]) } else { None };
    let next = if art_idx < sec.articles.len() - 1 { Some(&sec.articles[art_idx + 1]) } else { None };
    let left = match prev {
        Some(p) => format!(r#"<a href="{section_url}{slug}/">&larr; {} {}</a>"#, ui.article, p.number, slug = p.slug),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{section_url}{slug}/">{} {} &rarr;</a>"#, ui.article, n.number, slug = n.slug),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let art_label = format!("{} {}", ui.article, art.number);
    let sec_label = format!("{} {}", ui.section, sec.number);
    let law_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_path),
        (ui.constitution, pfx),
        (&sec_label, &section_url),
        (&art_label, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_path)),
        (ui.constitution.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (sec_label.clone(), format!("{}{}", html::DOMAIN, section_url)),
        (art_label.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let first_text = art.points.first().map(|p| &p.text as &str).unwrap_or("");
    let desc_trunc = if first_text.chars().count() > 200 {
        let end = first_text.char_indices().nth(200).map(|(i, _)| i).unwrap_or(first_text.len());
        &first_text[..end]
    } else { first_text };

    let rk_short = match lang { Lang::Ru => "РК", Lang::Kk => "ҚР" };
    let art_title_suffix = if !art.title.is_empty() {
        format!(". {}", art.title)
    } else {
        String::new()
    };
    let page_title = format!("{} {} {} {}{} — myqaz.kz", ui.article, art.number, ui.constitution, rk_short, art_title_suffix);
    let h1 = format!("{} {} {} {}{}", ui.article, art.number, ui.constitution, rk_short, art_title_suffix);

    html::page(lang,
        &page_title,
        desc_trunc,
        &path, &nav,
        &format!("<h1>{h1}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}
