use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::Code;
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.point { margin: 15px 0; padding: 10px 0; border-bottom: 1px solid #eee; }
.point-num { font-weight: bold; }
.subpoints { margin: 5px 0 0 20px; line-height: 1.8; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
</style>"#;

fn load_all_codes(store: &AdaptoStore) -> Vec<Code> {
    let col = store.collection("codes");
    col.find(StoreQuery::new())
        .filter_map(|doc| serde_json::from_value(doc.data).ok())
        .collect()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/law/codes/",
        Lang::Kk => "/kz/law/codes/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let codes = load_all_codes(store);
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let mut body = String::from("<table>\n<tr><th>Кодекс</th></tr>\n");
    let mut sorted = codes.clone();
    sorted.sort_by(|a, b| a.title.cmp(&b.title));
    for code in &sorted {
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}{slug}/\">{title}</a></td></tr>\n",
            slug = code.slug, title = code.title,
        ));
    }
    body.push_str("</table>\n");

    let law_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_path),
        (ui.codes, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_path)),
        (ui.codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    let (page_title, page_h1) = match lang {
        Lang::Ru => (
            "Кодексы Республики Казахстан — myqaz.kz",
            "Кодексы Республики Казахстан",
        ),
        Lang::Kk => (
            "Қазақстан Республикасының Кодекстері — myqaz.kz",
            "Қазақстан Республикасының Кодекстері",
        ),
    };

    html::page(lang,
        page_title,
        &format!("{page_h1} — {} кодексов.", sorted.len()),
        pfx, &nav,
        &format!("<h1>{page_h1}</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_code(store: &AdaptoStore, code_slug: &str, lang: Lang) -> String {
    let codes = load_all_codes(store);
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let code = match codes.iter().find(|c| c.slug == code_slug) {
        Some(c) => c,
        None => return "<h1>Code not found</h1>".to_string(),
    };
    let path = format!("{pfx}{}/", code.slug);

    let mut body = String::new();
    for ch in &code.chapters {
        let ch_path = format!("{path}{}/", ch.slug);
        body.push_str(&format!(
            "<h2><a href=\"{ch_path}\">{} {}. {}</a></h2>\n<ul>\n",
            ui.chapter, ch.number, ch.title,
        ));
        for art in &ch.articles {
            body.push_str(&format!(
                "<li><a href=\"{ch_path}{slug}/\">{} {}</a></li>\n",
                ui.article, art.number, slug = art.slug,
            ));
        }
        body.push_str("</ul>\n");
    }

    let law_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_path),
        (ui.codes, pfx),
        (&code.title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_path)),
        (ui.codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (code.title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    html::page(lang,
        &format!("{} — myqaz.kz", code.title),
        &format!("{} — {} глав.", code.title, code.chapters.len()),
        &path, &nav,
        &format!("<h1>{}</h1>\n{body}", code.title),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_chapter(store: &AdaptoStore, code_slug: &str, chapter_slug: &str, lang: Lang) -> String {
    let codes = load_all_codes(store);
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let code = match codes.iter().find(|c| c.slug == code_slug) {
        Some(c) => c,
        None => return "<h1>Code not found</h1>".to_string(),
    };
    let ch_idx = match code.chapters.iter().position(|ch| ch.slug == chapter_slug) {
        Some(i) => i,
        None => return "<h1>Chapter not found</h1>".to_string(),
    };
    let ch = &code.chapters[ch_idx];
    let code_path = format!("{pfx}{}/", code.slug);
    let path = format!("{pfx}{}/{}/", code.slug, ch.slug);

    let mut body = String::from("<ul>\n");
    for art in &ch.articles {
        body.push_str(&format!(
            "<li><a href=\"{path}{slug}/\">{} {}. {}</a></li>\n",
            ui.article, art.number, art.title, slug = art.slug,
        ));
    }
    body.push_str("</ul>\n");

    let prev = if ch_idx > 0 { Some(&code.chapters[ch_idx - 1]) } else { None };
    let next = if ch_idx < code.chapters.len() - 1 { Some(&code.chapters[ch_idx + 1]) } else { None };
    let left = match prev {
        Some(p) => format!(r#"<a href="{code_path}{slug}/">&larr; {} {}</a>"#, ui.chapter, p.number, slug = p.slug),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{code_path}{slug}/">{} {} &rarr;</a>"#, ui.chapter, n.number, slug = n.slug),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let ch_title = format!("{} {}. {}", ui.chapter, ch.number, ch.title);
    let law_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_path),
        (ui.codes, pfx),
        (&code.title, &code_path),
        (&ch_title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_path)),
        (ui.codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (code.title.clone(), format!("{}{}", html::DOMAIN, code_path)),
        (ch_title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let ch_h1 = format!("{} {} {}. {}", ui.chapter, ch.number, code.title_genitive, ch.title);
    html::page(lang,
        &format!("{ch_title} — {} — myqaz.kz", code.title),
        &format!("{} {} {}: {}.", ui.chapter, ch.number, code.title, ch.title),
        &path, &nav,
        &format!("<h1>{ch_h1}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_article(store: &AdaptoStore, code_slug: &str, chapter_slug: &str, article_slug: &str, lang: Lang) -> String {
    let codes = load_all_codes(store);
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let code = match codes.iter().find(|c| c.slug == code_slug) {
        Some(c) => c,
        None => return "<h1>Code not found</h1>".to_string(),
    };
    let ch = match code.chapters.iter().find(|c| c.slug == chapter_slug) {
        Some(c) => c,
        None => return "<h1>Chapter not found</h1>".to_string(),
    };
    let art_idx = match ch.articles.iter().position(|a| a.slug == article_slug) {
        Some(i) => i,
        None => return "<h1>Article not found</h1>".to_string(),
    };
    let art = &ch.articles[art_idx];
    let code_path = format!("{pfx}{}/", code.slug);
    let ch_path = format!("{pfx}{}/{}/", code.slug, ch.slug);
    let path = format!("{pfx}{}/{}/{}/", code.slug, ch.slug, art.slug);

    let mut body = String::new();
    for pt in &art.points {
        if pt.number == "0" {
            body.push_str(&format!("<p>{}</p>\n", pt.text));
            for cont in &pt.continuation {
                body.push_str(&format!("<p>{cont}</p>\n"));
            }
            continue;
        }
        body.push_str(&format!(
            "<div class=\"point\"><span class=\"point-num\">{})</span> {}\n",
            pt.number, pt.text
        ));
        if !pt.subpoints.is_empty() {
            body.push_str("<div class=\"subpoints\">\n");
            for sp in &pt.subpoints {
                body.push_str(&format!("{}) {}<br>\n", sp.number, sp.text));
            }
            body.push_str("</div>\n");
        }
        for cont in &pt.continuation {
            body.push_str(&format!("<p>{cont}</p>\n"));
        }
        body.push_str("</div>\n");
    }

    let prev = if art_idx > 0 { Some(&ch.articles[art_idx - 1]) } else { None };
    let next = if art_idx < ch.articles.len() - 1 { Some(&ch.articles[art_idx + 1]) } else { None };
    let left = match prev {
        Some(p) => format!(r#"<a href="{ch_path}{slug}/">&larr; {} {}</a>"#, ui.article, p.number, slug = p.slug),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{ch_path}{slug}/">{} {} &rarr;</a>"#, ui.article, n.number, slug = n.slug),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let art_title = format!("{} {}. {}", ui.article, art.number, art.title);
    let ch_label = format!("{} {}", ui.chapter, ch.number);
    let law_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_path),
        (ui.codes, pfx),
        (&code.title, &code_path),
        (&ch_label, &ch_path),
        (&art_title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_path)),
        (ui.codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (code.title.clone(), format!("{}{}", html::DOMAIN, code_path)),
        (ch_label.clone(), format!("{}{}", html::DOMAIN, ch_path)),
        (art_title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let first_text = art.points.first().map(|p| &p.text as &str).unwrap_or("");
    let desc_trunc = if first_text.chars().count() > 200 {
        let end = first_text.char_indices().nth(200).map(|(i, _)| i).unwrap_or(first_text.len());
        &first_text[..end]
    } else { first_text };

    html::page(lang,
        &format!("{art_title} — {} — myqaz.kz", code.title_genitive),
        desc_trunc,
        &path, &nav,
        &format!("<h1>{art_title}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}
