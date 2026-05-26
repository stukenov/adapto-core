use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::Law;
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.point { margin: 15px 0; padding: 10px 0; border-bottom: 1px solid #eee; }
.point-num { font-weight: bold; }
.subpoints { margin: 5px 0 0 20px; line-height: 1.8; }
.alpha-nav { display: flex; flex-wrap: wrap; gap: 8px; margin: 20px 0; }
.alpha-nav a { padding: 4px 8px; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
</style>"#;

fn load_all_laws(store: &AdaptoStore) -> Vec<Law> {
    let col = store.collection("laws");
    col.find(StoreQuery::new())
        .filter_map(|doc| serde_json::from_value(doc.data).ok())
        .collect()
}

fn law_slug(law: &Law) -> String {
    law.doc_id.to_lowercase()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/law/laws/",
        Lang::Kk => "/kz/law/laws/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let mut laws = load_all_laws(store);
    laws.sort_by(|a, b| a.title.cmp(&b.title));
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let mut letters: Vec<char> = laws.iter()
        .filter_map(|l| l.title.chars().next())
        .map(|c| c.to_uppercase().next().unwrap_or(c))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    letters.sort();

    let mut body = String::from("<div class=\"alpha-nav\">\n");
    for &letter in &letters {
        body.push_str(&format!("<a href=\"#{letter}\">{letter}</a>\n"));
    }
    body.push_str("</div>\n");

    let mut current_letter = ' ';
    body.push_str("<ul>\n");
    for law in &laws {
        let first = law.title.chars().next().map(|c| c.to_uppercase().next().unwrap_or(c)).unwrap_or(' ');
        if first != current_letter {
            if current_letter != ' ' {
                body.push_str("</ul>\n");
            }
            current_letter = first;
            body.push_str(&format!("<h2 id=\"{current_letter}\">{current_letter}</h2>\n<ul>\n"));
        }
        body.push_str(&format!(
            "<li><a href=\"{pfx}{slug}/\">{title}</a></li>\n",
            slug = law_slug(law), title = law.title,
        ));
    }
    body.push_str("</ul>\n");

    let law_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_path),
        (ui.laws, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_path)),
        (ui.laws.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    let (page_title, page_h1) = match lang {
        Lang::Ru => (
            "Законы Республики Казахстан — myqaz.kz",
            "Законы Республики Казахстан",
        ),
        Lang::Kk => (
            "Заңдар Қазақстан Республикасы — myqaz.kz",
            "Заңдар Қазақстан Республикасы",
        ),
    };

    html::page(lang,
        page_title,
        &format!("{page_h1} — {} законов.", laws.len()),
        pfx, &nav,
        &format!("<h1>{page_h1}</h1>\n<p>Всего: <b>{}</b> законов</p>\n{body}", laws.len()),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_law(store: &AdaptoStore, slug_param: &str, lang: Lang) -> String {
    let laws = load_all_laws(store);
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let law = match laws.iter().find(|l| law_slug(l) == slug_param) {
        Some(l) => l,
        None => return "<h1>Law not found</h1>".to_string(),
    };
    let slug = law_slug(law);
    let path = format!("{pfx}{slug}/");

    let mut body = String::new();
    if !law.preamble.is_empty() {
        body.push_str(&format!("<p>{}</p>\n", law.preamble));
    }

    body.push_str("<ul>\n");
    for art in &law.articles {
        body.push_str(&format!(
            "<li><a href=\"{path}{art_slug}/\">{} {}. {}</a></li>\n",
            ui.article, art.number, art.title, art_slug = art.slug,
        ));
    }
    body.push_str("</ul>\n");

    let law_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_path),
        (ui.laws, pfx),
        (&law.title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_path)),
        (ui.laws.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (law.title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    html::page(lang,
        &format!("{} — myqaz.kz", law.title),
        &format!("{} — {} статей.", law.title, law.articles.len()),
        &path, &nav,
        &format!("<h1>{}</h1>\n{body}", law.title),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_article(store: &AdaptoStore, slug_param: &str, article_slug: &str, lang: Lang) -> String {
    let laws = load_all_laws(store);
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let law = match laws.iter().find(|l| law_slug(l) == slug_param) {
        Some(l) => l,
        None => return "<h1>Law not found</h1>".to_string(),
    };
    let slug = law_slug(law);
    let law_path = format!("{pfx}{slug}/");

    let art_idx = match law.articles.iter().position(|a| a.slug == article_slug) {
        Some(i) => i,
        None => return "<h1>Article not found</h1>".to_string(),
    };
    let art = &law.articles[art_idx];
    let path = format!("{pfx}{slug}/{}/", art.slug);

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

    let prev = if art_idx > 0 { Some(&law.articles[art_idx - 1]) } else { None };
    let next = if art_idx < law.articles.len() - 1 { Some(&law.articles[art_idx + 1]) } else { None };
    let left = match prev {
        Some(p) => format!(r#"<a href="{law_path}{slug}/">&larr; {} {}</a>"#, ui.article, p.number, slug = p.slug),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{law_path}{slug}/">{} {} &rarr;</a>"#, ui.article, n.number, slug = n.slug),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let art_title = format!("{} {}. {}", ui.article, art.number, art.title);
    let law_prefix_path = format!("{}law/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.law, &law_prefix_path),
        (ui.laws, pfx),
        (&law.title, &law_path),
        (&art_title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.law.to_string(), format!("{}{}", html::DOMAIN, law_prefix_path)),
        (ui.laws.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (law.title.clone(), format!("{}{}", html::DOMAIN, law_path)),
        (art_title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let first_text = art.points.first().map(|p| &p.text as &str).unwrap_or("");
    let desc_trunc = if first_text.chars().count() > 200 {
        let end = first_text.char_indices().nth(200).map(|(i, _)| i).unwrap_or(first_text.len());
        &first_text[..end]
    } else { first_text };

    html::page(lang,
        &format!("{art_title} — {} — myqaz.kz", law.title),
        desc_trunc,
        &path, &nav,
        &format!("<h1>{art_title}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}
