use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::QuranData;
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
tr:hover { background: #f9f9f9; }
.ayah-block { display: block; text-decoration: none; border: none; color: inherit; }
.ayah-block:hover { background: #f9f9f9; }
.ayah { margin: 24px 0; padding: 16px 0; border-bottom: 1px solid #eee; }
.ayah-ar { font-size: 1.4em; line-height: 2; direction: rtl; text-align: right; font-family: 'Traditional Arabic', serif; }
.ayah-num { display: inline-flex; align-items: center; justify-content: center; width: 28px; height: 28px; border-radius: 50%; background: #f0f0f0; font-size: 13px; color: #666; }
.ayah-ru { line-height: 1.7; margin-top: 8px; }
.ayah-page-ar { font-size: 1.5em; line-height: 2; direction: rtl; text-align: center; font-family: 'Traditional Arabic', serif; margin: 24px 0; padding: 20px; background: #fafafa; }
.ayah-page-ru { font-size: 1.1em; text-align: center; line-height: 1.8; margin: 24px 0; }
.card p { margin: 8px 0; }
.subtitle { color: #666; font-size: 15px; margin-bottom: 28px; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<QuranData> {
    let col = store.collection("quran");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/islam/quran/",
        Lang::Kk => "/kz/islam/quran/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);

    let mut body = String::from(
        "<table>\n<tr><th class=\"num\">№</th><th>Арабское</th><th>Транслитерация</th><th>Перевод</th><th class=\"num\">Аятов</th><th>Место</th></tr>\n"
    );
    for s in &data.surahs {
        body.push_str(&format!(
            "<tr><td class=\"num\">{}</td><td dir=\"rtl\" lang=\"ar\">{name_ar}</td><td><a href=\"{pfx}{slug}/\">{trans}</a></td><td><a href=\"{pfx}{slug}/\">{meaning}</a></td><td class=\"num\">{count}</td><td>{rev}</td></tr>\n",
            s.number, slug = s.slug, name_ar = s.name_ar, trans = s.name_transliterated,
            meaning = s.meaning_ru, count = s.ayah_count, rev = s.revelation,
        ));
    }
    body.push_str("</table>\n");

    let quran_label = match lang { Lang::Ru => "Коран", Lang::Kk => "Құран" };
    let islam_label = match lang { Lang::Ru => "Ислам", Lang::Kk => "Ислам" };
    let islam_path = format!("{}islam/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (islam_label, &islam_path),
        (quran_label, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (islam_label.to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        (quran_label.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    let page_title = match lang {
        Lang::Ru => "Коран на русском языке — перевод Кулиева — myqaz.kz",
        Lang::Kk => "Құран на русском языке — перевод Кулиева — myqaz.kz",
    };
    let page_h1 = "Коран на русском языке — перевод Кулиева";

    html::page(lang,
        page_title,
        "Коран на русском языке — 114 сур. Перевод Эльмира Кулиева.",
        pfx, &nav,
        &format!("<h1>{page_h1}</h1>\n{body}"),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_surah(store: &AdaptoStore, surah_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let surahs = &data.surahs;
    let idx = match surahs.iter().position(|s| s.slug == surah_slug) {
        Some(i) => i,
        None => return "<h1>Surah not found</h1>".to_string(),
    };
    let s = &surahs[idx];
    let path = format!("{pfx}{}/", s.slug);

    let mut body = format!(
        "<p class=\"subtitle\">{} &bull; {} &bull; {} аятов</p>\n",
        s.meaning_ru, if s.revelation == "Meccan" || s.revelation == "Mecca" { "Мекканская" } else { "Мединская" }, s.ayah_count
    );

    for ayah in &s.ayahs {
        body.push_str(&format!(
            r#"<a href="{path}{v}/" class="ayah-block"><div class="ayah"><div class="ayah-ar" dir="rtl" lang="ar"><span class="ayah-num">{v}</span> {ar}</div><div class="ayah-ru"><span class="ayah-num">{v}</span> {ru}</div></div></a>
"#,
            v = ayah.verse, ar = ayah.text_ar, ru = ayah.text_ru,
        ));
    }

    let prev = if idx > 0 { Some(&surahs[idx - 1]) } else { None };
    let next = if idx < surahs.len() - 1 { Some(&surahs[idx + 1]) } else { None };
    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {num}. {name}</a>"#, slug = p.slug, num = p.number, name = p.name_ru),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{num}. {name} &rarr;</a>"#, slug = n.slug, num = n.number, name = n.name_ru),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let quran_label = match lang { Lang::Ru => "Коран", Lang::Kk => "Құран" };
    let islam_label = match lang { Lang::Ru => "Ислам", Lang::Kk => "Ислам" };
    let islam_path = format!("{}islam/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (islam_label, &islam_path),
        (quran_label, pfx),
        (&s.name_ru, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (islam_label.to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        (quran_label.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (s.name_ru.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let h1 = format!("Сура {} ({}) ({})", s.number, s.name_ru, s.name_ar);
    let title = format!("Сура {}. {} ({}) — {} — myqaz.kz", s.number, s.name_ru, s.name_ar, quran_label);
    let desc = format!("Сура {} ({}). {}. {} аятов.", s.name_ru, s.name_ar, s.meaning_ru, s.ayah_count);

    html::page(lang, &title, &desc, &path, &nav,
        &format!("<h1>{h1}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_ayah(store: &AdaptoStore, surah_slug: &str, verse: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let surahs = &data.surahs;
    let s_idx = match surahs.iter().position(|s| s.slug == surah_slug) {
        Some(i) => i,
        None => return "<h1>Surah not found</h1>".to_string(),
    };
    let s = &surahs[s_idx];
    let v_num: u32 = match verse.parse() {
        Ok(n) => n,
        Err(_) => return "<h1>Invalid verse</h1>".to_string(),
    };
    let a_idx = match s.ayahs.iter().position(|a| a.verse == v_num) {
        Some(i) => i,
        None => return "<h1>Ayah not found</h1>".to_string(),
    };
    let ayah = &s.ayahs[a_idx];
    let surah_path = format!("{pfx}{}/", s.slug);
    let path = format!("{pfx}{}/{}/", s.slug, ayah.verse);

    let body = format!(
        "<div class=\"ayah-page-ar\">{}</div>\n<div class=\"ayah-page-ru\">{}</div>\n",
        ayah.text_ar, ayah.text_ru,
    );

    let prev: Option<(String, String)> = if a_idx > 0 {
        Some((format!("← {}:{}", s.number, s.ayahs[a_idx - 1].verse),
              format!("{pfx}{}/{}/", s.slug, s.ayahs[a_idx - 1].verse)))
    } else if s_idx > 0 {
        let ps = &surahs[s_idx - 1];
        let last_v = ps.ayahs.last().map(|a| a.verse).unwrap_or(1);
        Some((format!("← {}:{}", ps.number, last_v),
              format!("{pfx}{}/{}/", ps.slug, last_v)))
    } else { None };
    let next: Option<(String, String)> = if a_idx < s.ayahs.len() - 1 {
        Some((format!("{}:{} →", s.number, s.ayahs[a_idx + 1].verse),
              format!("{pfx}{}/{}/", s.slug, s.ayahs[a_idx + 1].verse)))
    } else if s_idx < surahs.len() - 1 {
        let ns = &surahs[s_idx + 1];
        Some((format!("{}:1 →", ns.number), format!("{pfx}{}/1/", ns.slug)))
    } else { None };

    let left = match &prev { Some((l, u)) => format!(r#"<a href="{u}">{l}</a>"#), None => "<span></span>".to_string() };
    let right = match &next { Some((l, u)) => format!(r#"<a href="{u}">{l}</a>"#), None => "<span></span>".to_string() };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let quran_label = match lang { Lang::Ru => "Коран", Lang::Kk => "Құран" };
    let islam_label = match lang { Lang::Ru => "Ислам", Lang::Kk => "Ислам" };
    let islam_path = format!("{}islam/", lang.path_prefix());
    let v_str = ayah.verse.to_string();
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (islam_label, &islam_path),
        (quran_label, pfx),
        (&s.name_ru, &surah_path),
        (&v_str, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (islam_label.to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        (quran_label.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (s.name_ru.clone(), format!("{}{}", html::DOMAIN, surah_path)),
        (v_str.clone(), format!("{}{}", html::DOMAIN, path)),
    ];
    let h1 = format!("Сура {}, аят {}", s.name_ru, ayah.verse);
    let title = format!("Аят {} суры {} ({}) — {} — myqaz.kz", ayah.verse, s.name_ru, s.name_ar, quran_label);

    html::page(lang, &title, &ayah.text_ru, &path, &nav,
        &format!("<h1>{h1}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}
