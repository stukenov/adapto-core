use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::BibleData;
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.chapter-grid { display: flex; flex-wrap: wrap; gap: 8px; margin: 16px 0; }
.chapter-grid a { padding: 6px 12px; background: #f0f0f0; border-radius: 4px; }
.chapter-grid a:hover { background: #e0e0e0; }
.verse-block { display: block; padding: 6px 8px; border-radius: 4px; margin: 2px 0; }
.verse-block:hover { background: #f0f8ff; }
.verse-block sup { font-size: 11px; color: #999; margin-right: 4px; }
.stats { color: #666; font-size: 14px; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<BibleData> {
    let col = store.collection("bible");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/christianity/bible/",
        Lang::Kk => "/kz/christianity/bible/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);

    let ot: Vec<&_> = data.books.iter().filter(|b| b.testament == "ВЗ").collect();
    let nt: Vec<&_> = data.books.iter().filter(|b| b.testament == "НЗ").collect();

    let mut body = String::from("<h2>Ветхий Завет</h2>\n");
    body.push_str("<table>\n<tr><th>Книга</th><th>Глав</th></tr>\n");
    for b in &ot {
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}{slug}/\">{name}</a></td><td>{count}</td></tr>\n",
            slug = b.slug, name = b.name_ru, count = b.chapter_count,
        ));
    }
    body.push_str("</table>\n");

    body.push_str("<h2>Новый Завет</h2>\n");
    body.push_str("<table>\n<tr><th>Книга</th><th>Глав</th></tr>\n");
    for b in &nt {
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}{slug}/\">{name}</a></td><td>{count}</td></tr>\n",
            slug = b.slug, name = b.name_ru, count = b.chapter_count,
        ));
    }
    body.push_str("</table>\n");

    let total_chapters: u32 = data.books.iter().map(|b| b.chapter_count).sum();
    body.push_str(&format!(
        "<p class=\"stats\">{} книг, {} глав</p>\n",
        data.books.len(), total_chapters
    ));

    let chr_path = format!("{}christianity/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Христианство", &chr_path),
        ("Библия", ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Христианство".to_string(), format!("{}{}", html::DOMAIN, chr_path)),
        ("Библия".to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(lang,
        "Библия — Синодальный перевод — myqaz.kz",
        &format!("Библия на русском языке. Синодальный перевод. {} книг, {} глав.", data.books.len(), total_chapters),
        pfx, &nav,
        &format!("<h1>Библия &mdash; Синодальный перевод</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_book(store: &AdaptoStore, book_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let books = &data.books;
    let idx = match books.iter().position(|b| b.slug == book_slug) {
        Some(i) => i,
        None => return "<h1>Book not found</h1>".to_string(),
    };
    let book = &books[idx];
    let path = format!("{pfx}{}/", book.slug);

    let testament = if book.testament == "ВЗ" { "Ветхий Завет" } else { "Новый Завет" };
    let mut body = format!("<p class=\"stats\">{} · {} глав</p>\n", testament, book.chapter_count);

    body.push_str("<div class=\"chapter-grid\">\n");
    for ch in &book.chapters {
        body.push_str(&format!(
            "<a href=\"{path}{num}/\">Глава {num}</a>\n",
            num = ch.number,
        ));
    }
    body.push_str("</div>\n");

    let prev = if idx > 0 { Some(&books[idx - 1]) } else { None };
    let next = if idx < books.len() - 1 { Some(&books[idx + 1]) } else { None };
    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {name}</a>"#, slug = p.slug, name = p.name_ru),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{name} &rarr;</a>"#, slug = n.slug, name = n.name_ru),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let chr_path = format!("{}christianity/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Христианство", &chr_path),
        ("Библия", pfx),
        (&book.name_ru, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Христианство".to_string(), format!("{}{}", html::DOMAIN, chr_path)),
        ("Библия".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (book.name_ru.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("{} — Библия — myqaz.kz", book.name_ru);

    html::page(lang, &title, &format!("{} — {} глав", book.name_ru, book.chapter_count),
        &path, &nav,
        &format!("<h1>{}</h1>\n{body}\n{nav_bot}", book.name_ru),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_chapter(store: &AdaptoStore, book_slug: &str, chapter_str: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let books = &data.books;
    let b_idx = match books.iter().position(|b| b.slug == book_slug) {
        Some(i) => i,
        None => return "<h1>Book not found</h1>".to_string(),
    };
    let book = &books[b_idx];
    let ch_num: u32 = match chapter_str.parse() {
        Ok(n) => n,
        Err(_) => return "<h1>Invalid chapter</h1>".to_string(),
    };
    let ch = match book.chapters.iter().find(|c| c.number == ch_num) {
        Some(c) => c,
        None => return "<h1>Chapter not found</h1>".to_string(),
    };
    let book_path = format!("{pfx}{}/", book.slug);
    let path = format!("{pfx}{}/{}/", book.slug, ch.number);

    let mut body = String::from("<div class=\"verses\">\n");
    for (i, verse) in ch.verses.iter().enumerate() {
        let v_num = i + 1;
        body.push_str(&format!(
            "<a href=\"{path}{v_num}/\" class=\"verse-block\" id=\"v{v_num}\"><sup>{v_num}</sup>{verse}</a>\n",
        ));
    }
    body.push_str("</div>\n");

    let ch_idx = book.chapters.iter().position(|c| c.number == ch_num).unwrap_or(0);
    let prev: Option<(String, String)> = if ch_idx > 0 {
        Some((format!("← Глава {}", book.chapters[ch_idx - 1].number),
              format!("{book_path}{}/", book.chapters[ch_idx - 1].number)))
    } else if b_idx > 0 {
        let pb = &books[b_idx - 1];
        let last_ch = pb.chapters.last().map(|c| c.number).unwrap_or(1);
        Some((format!("← {} {}", pb.name_ru, last_ch),
              format!("{pfx}{}/{}/", pb.slug, last_ch)))
    } else { None };
    let next: Option<(String, String)> = if ch_idx < book.chapters.len() - 1 {
        Some((format!("Глава {} →", book.chapters[ch_idx + 1].number),
              format!("{book_path}{}/", book.chapters[ch_idx + 1].number)))
    } else if b_idx < books.len() - 1 {
        let nb = &books[b_idx + 1];
        Some((format!("{} 1 →", nb.name_ru), format!("{pfx}{}/1/", nb.slug)))
    } else { None };

    let left = match &prev { Some((l, u)) => format!(r#"<a href="{u}">{l}</a>"#), None => "<span></span>".to_string() };
    let right = match &next { Some((l, u)) => format!(r#"<a href="{u}">{l}</a>"#), None => "<span></span>".to_string() };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let chr_path = format!("{}christianity/", lang.path_prefix());
    let ch_label = format!("Глава {}", ch.number);
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Христианство", &chr_path),
        ("Библия", pfx),
        (&book.name_ru, &book_path),
        (&ch_label, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Христианство".to_string(), format!("{}{}", html::DOMAIN, chr_path)),
        ("Библия".to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (book.name_ru.clone(), format!("{}{}", html::DOMAIN, book_path)),
        (ch_label.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let h1 = format!("{}, Глава {}", book.name_ru, ch.number);
    let title = format!("{h1} — Библия — myqaz.kz");

    html::page(lang, &title, &format!("{}, глава {}. {} стихов.", book.name_ru, ch.number, ch.verses.len()),
        &path, &nav,
        &format!("<h1>{h1}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}
