use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::HadithData;
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.hadith-card { border: 1px solid #eee; padding: 16px; border-radius: 4px; margin: 16px 0; }
.hadith-card:hover { border-color: #ccc; }
.hadith-preview { color: #666; font-size: 14px; margin-top: 8px; }
.hadith-meta { font-size: 13px; color: #888; margin-top: 8px; }
.grade { display: inline-block; padding: 2px 8px; border-radius: 3px; font-size: 12px; }
.grade-sahih { background: #e8f5e9; color: #2e7d32; }
.grade-hasan { background: #e3f2fd; color: #1565c0; }
.grade-other { background: #f5f5f5; color: #616161; }
.arabic-block { font-size: 1.4em; line-height: 2; direction: rtl; text-align: right; font-family: 'Traditional Arabic', serif; margin: 24px 0; padding: 20px; background: #fafafa; }
.hadith-text { line-height: 1.8; margin: 24px 0; font-size: 1.05em; }
.explanation { margin: 24px 0; padding: 16px 20px; background: #f9f9f9; border-left: 3px solid #ddd; }
.hints { margin: 16px 0; }
.hints li { margin: 8px 0; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<HadithData> {
    let col = store.collection("hadith");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/islam/hadith/",
        Lang::Kk => "/kz/islam/hadith/",
    }
}

fn grade_class(grade: &str) -> &'static str {
    if grade.contains("Достоверный") || grade.contains("صحيح") {
        "grade-sahih"
    } else if grade.contains("Хороший") {
        "grade-hasan"
    } else {
        "grade-other"
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max { return s.to_string(); }
    let end = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
    format!("{}…", &s[..end])
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let total: usize = data.categories.iter().map(|c| c.hadiths.len()).sum();

    let mut body = format!(
        "<p>Всего хадисов: <b>{total}</b> в {} категориях</p>\n",
        data.categories.len()
    );
    body.push_str("<table>\n<tr><th>Категория</th><th>Хадисов</th></tr>\n");
    for cat in &data.categories {
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}{slug}/\">{title}</a></td><td>{count}</td></tr>\n",
            slug = cat.slug, title = cat.title, count = cat.hadiths.len(),
        ));
    }
    body.push_str("</table>\n");

    let islam_path = format!("{}islam/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Ислам", &islam_path),
        ("Хадисы", ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Ислам".to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        ("Хадисы".to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(lang,
        "Хадисы на русском языке — myqaz.kz",
        &format!("Сборник хадисов — {total} хадисов в {} категориях.", data.categories.len()),
        pfx, &nav,
        &format!("<h1>Хадисы на русском языке</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_category(store: &AdaptoStore, cat_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let cats = &data.categories;
    let idx = match cats.iter().position(|c| c.slug == cat_slug) {
        Some(i) => i,
        None => return "<h1>Category not found</h1>".to_string(),
    };
    let cat = &cats[idx];
    let path = format!("{pfx}{}/", cat.slug);

    let mut body = String::new();
    for h in &cat.hadiths {
        let title = if h.title.is_empty() { "Хадис" } else { &h.title };
        let preview = truncate(&h.hadeeth, 150);
        let mut meta = String::new();
        if !h.attribution.is_empty() {
            meta.push_str(&h.attribution);
        }
        if !h.grade.is_empty() {
            let cls = grade_class(&h.grade);
            meta.push_str(&format!(r#" <span class="grade {cls}">{}</span>"#, h.grade));
        }
        body.push_str(&format!(
            r#"<div class="hadith-card"><h3><a href="{path}{id}/">{title}</a></h3><p class="hadith-preview">{preview}</p><p class="hadith-meta">{meta}</p></div>
"#,
            id = h.id, title = truncate(title, 100),
        ));
    }

    let prev = if idx > 0 { Some(&cats[idx - 1]) } else { None };
    let next = if idx < cats.len() - 1 { Some(&cats[idx + 1]) } else { None };
    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {title}</a>"#, slug = p.slug, title = truncate(&p.title, 30)),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{title} &rarr;</a>"#, slug = n.slug, title = truncate(&n.title, 30)),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let islam_path = format!("{}islam/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Ислам", &islam_path),
        ("Хадисы", pfx),
        (&cat.title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Ислам".to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        ("Хадисы".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (cat.title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("{} — Хадисы — myqaz.kz", cat.title);
    let desc = format!("{}. {} хадисов.", cat.title, cat.hadiths.len());

    html::page(lang, &title, &desc, &path, &nav,
        &format!("<h1>{}</h1>\n<p>{} хадисов</p>\n{body}\n{nav_bot}", cat.title, cat.hadiths.len()),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_hadith(store: &AdaptoStore, cat_slug: &str, hadith_id: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let cat = match data.categories.iter().find(|c| c.slug == cat_slug) {
        Some(c) => c,
        None => return "<h1>Category not found</h1>".to_string(),
    };
    let h_idx = match cat.hadiths.iter().position(|h| h.id == hadith_id) {
        Some(i) => i,
        None => return "<h1>Hadith not found</h1>".to_string(),
    };
    let h = &cat.hadiths[h_idx];
    let cat_path = format!("{pfx}{}/", cat.slug);
    let path = format!("{pfx}{}/{}/", cat.slug, h.id);
    let title_display = if h.title.is_empty() { "Хадис" } else { &h.title };

    let mut body = String::new();
    if !h.grade.is_empty() {
        let cls = grade_class(&h.grade);
        body.push_str(&format!(r#"<p><span class="grade {cls}">{}</span></p>"#, h.grade));
    }
    if !h.attribution.is_empty() {
        body.push_str(&format!("<p class=\"label\">{}</p>\n", h.attribution));
    }
    if !h.hadeeth_ar.is_empty() {
        body.push_str(&format!("<div class=\"arabic-block\">{}</div>\n", h.hadeeth_ar));
    }
    if !h.hadeeth.is_empty() {
        body.push_str(&format!("<div class=\"hadith-text\">{}</div>\n", h.hadeeth));
    }
    if !h.explanation.is_empty() {
        body.push_str(&format!("<div class=\"explanation\"><h3>Комментарий</h3><p>{}</p></div>\n", h.explanation));
    }
    if !h.hints.is_empty() {
        body.push_str("<div class=\"hints\"><h3>Уроки и назидания</h3>\n<ul>\n");
        for hint in &h.hints {
            let trimmed = hint.trim();
            if !trimmed.is_empty() {
                body.push_str(&format!("<li>{trimmed}</li>\n"));
            }
        }
        body.push_str("</ul>\n</div>\n");
    }

    let prev = if h_idx > 0 { Some(&cat.hadiths[h_idx - 1]) } else { None };
    let next = if h_idx < cat.hadiths.len() - 1 { Some(&cat.hadiths[h_idx + 1]) } else { None };
    let left = match prev {
        Some(p) => {
            let t = if p.title.is_empty() { "Хадис" } else { &p.title };
            format!(r#"<a href="{cat_path}{id}/">&larr; {}</a>"#, truncate(t, 40), id = p.id)
        },
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => {
            let t = if n.title.is_empty() { "Хадис" } else { &n.title };
            format!(r#"<a href="{cat_path}{id}/">{} &rarr;</a>"#, truncate(t, 40), id = n.id)
        },
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let islam_path = format!("{}islam/", lang.path_prefix());
    let short_title = truncate(title_display, 40);
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Ислам", &islam_path),
        ("Хадисы", pfx),
        (&cat.title, &cat_path),
        (&short_title, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Ислам".to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        ("Хадисы".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (cat.title.clone(), format!("{}{}", html::DOMAIN, cat_path)),
        (short_title.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let page_title = format!("{} — myqaz.kz", title_display);
    let desc = truncate(&h.hadeeth, 200);

    html::page(lang, &page_title, &desc, &path, &nav,
        &format!("<h1>{title_display}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}
