use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::NamazData;
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
.months { display: flex; flex-wrap: wrap; gap: 8px; margin: 20px 0; }
.months a { padding: 6px 12px; background: #f0f0f0; border-radius: 4px; }
.months span.cur { padding: 6px 12px; background: #008ace; color: white; border-radius: 4px; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
.prayer-name { display: inline-block; width: 120px; }
tr:hover { background: #f9f9f9; }
</style>"#;

fn day_of_week(year: u32, month: u32, day: u32) -> &'static str {
    // Zeller's formula for day of week
    let (y, m) = if month <= 2 { (year as i32 - 1, month as i32 + 12) } else { (year as i32, month as i32) };
    let q = day as i32;
    let k = y % 100;
    let j = y / 100;
    let h = (q + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    match h {
        0 => "Суббота", 1 => "Воскресенье", 2 => "Понедельник", 3 => "Вторник",
        4 => "Среда", 5 => "Четверг", 6 => "Пятница", _ => "",
    }
}

fn load_data(store: &AdaptoStore) -> Option<NamazData> {
    let col = store.collection("namaz");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/islam/namaz/",
        Lang::Kk => "/kz/islam/namaz/",
    }
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let mut body = format!(
        "<table>\n<tr><th>Город</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        ui.fajr, ui.dhuhr, ui.maghrib, ui.isha,
    );
    for city in &data.cities {
        let sample = city.months.get("1").and_then(|days| days.get(14));
        let (fajr, dhuhr, maghrib, isha) = match sample {
            Some(d) => (d.fajr.as_str(), d.dhuhr.as_str(), d.maghrib.as_str(), d.isha.as_str()),
            None => ("—", "—", "—", "—"),
        };
        body.push_str(&format!(
            "<tr><td><a href=\"{pfx}{slug}/\">{name}</a></td><td>{fajr}</td><td>{dhuhr}</td><td>{maghrib}</td><td>{isha}</td></tr>\n",
            slug = city.slug, name = city.name,
        ));
    }
    body.push_str("</table>\n");
    body.push_str(&format!(
        "<p class=\"note\">Метод расчёта: {}. Часовой пояс: {}.</p>\n",
        data.method_description, data.timezone
    ));

    let islam_path = format!("{}islam/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Ислам", &islam_path),
        (ui.namaz, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Ислам".to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        (ui.namaz.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    let title = format!("Время намаза в Казахстане — расписание на {} год — myqaz.kz", data.year);
    let desc = format!("Расписание намаза для городов Казахстана на {} год.", data.year);

    html::page(lang, &title, &desc, pfx, &nav,
        &format!("<h1>Время намаза в городах Казахстана на {} год</h1>\n{body}", data.year),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_city(store: &AdaptoStore, city_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let cities = &data.cities;
    let idx = match cities.iter().position(|c| c.slug == city_slug) {
        Some(i) => i,
        None => return "<h1>City not found</h1>".to_string(),
    };
    let city = &cities[idx];
    let path = format!("{pfx}{}/", city.slug);

    let mut months_nav = String::from("<div class=\"months\">\n");
    for m in &data.months {
        months_nav.push_str(&format!(
            "<a href=\"{path}{slug}/\">{name}</a>\n",
            slug = m.slug, name = m.name,
        ));
    }
    months_nav.push_str("</div>\n");

    let mut body = months_nav;
    body.push_str(&format!(
        "<table>\n<tr><th>Месяц</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        ui.fajr, ui.sunrise, ui.dhuhr, ui.asr, ui.maghrib, ui.isha,
    ));
    for m in &data.months {
        let key = m.number.to_string();
        let sample = city.months.get(&key).and_then(|days| days.get(14));
        if let Some(d) = sample {
            body.push_str(&format!(
                "<tr><td><a href=\"{path}{slug}/\">{name}</a></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                d.fajr, d.sunrise, d.dhuhr, d.asr, d.maghrib, d.isha,
                slug = m.slug, name = m.name,
            ));
        }
    }
    body.push_str("</table>\n");

    let prev = if idx > 0 { Some(&cities[idx - 1]) } else { None };
    let next = if idx < cities.len() - 1 { Some(&cities[idx + 1]) } else { None };
    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {name}</a>"#, slug = p.slug, name = p.name),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{name} &rarr;</a>"#, slug = n.slug, name = n.name),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let islam_path = format!("{}islam/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Ислам", &islam_path),
        (ui.namaz, pfx),
        (&city.name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Ислам".to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        (ui.namaz.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (city.name.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("Время намаза в {} — расписание на {} год — myqaz.kz", city.name_prep, data.year);
    let desc = format!("Расписание намаза в {} на {} год.", city.name_prep, data.year);

    html::page(lang, &title, &desc, &path, &nav,
        &format!("<h1>Расписание намаза в {} на {} год</h1>\n{body}\n{nav_bot}", city.name_prep, data.year),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_month(store: &AdaptoStore, city_slug: &str, month_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let city = match data.cities.iter().find(|c| c.slug == city_slug) {
        Some(c) => c,
        None => return "<h1>City not found</h1>".to_string(),
    };
    let month = match data.months.iter().find(|m| m.slug == month_slug) {
        Some(m) => m,
        None => return "<h1>Month not found</h1>".to_string(),
    };
    let city_path = format!("{pfx}{}/", city.slug);
    let path = format!("{pfx}{}/{}/", city.slug, month.slug);
    let key = month.number.to_string();
    let days = city.months.get(&key);

    let mut months_nav = String::from("<div class=\"months\">\n");
    for m in &data.months {
        if m.slug == month.slug {
            months_nav.push_str(&format!("<span class=\"cur\">{}</span>\n", m.name));
        } else {
            months_nav.push_str(&format!(
                "<a href=\"{city_path}{slug}/\">{name}</a>\n",
                slug = m.slug, name = m.name,
            ));
        }
    }
    months_nav.push_str("</div>\n");

    let mut body = months_nav;
    body.push_str(&format!(
        "<table>\n<tr><th>День</th><th>Бамдат<br><span class=\"kz\">{}</span></th><th>Күн шығуы<br><span class=\"kz\">{}</span></th><th>Бесін<br><span class=\"kz\">{}</span></th><th>Намаздыгер<br><span class=\"kz\">{}</span></th><th>Ақшам<br><span class=\"kz\">{}</span></th><th>Құптан<br><span class=\"kz\">{}</span></th></tr>\n",
        ui.fajr, ui.sunrise, ui.dhuhr, ui.asr, ui.maghrib, ui.isha,
    ));
    if let Some(days) = days {
        for d in days {
            body.push_str(&format!(
                "<tr><td><a href=\"{path}{day}/\">{day}</a></td><td class=\"time\">{}</td><td class=\"time\">{}</td><td class=\"time\">{}</td><td class=\"time\">{}</td><td class=\"time\">{}</td><td class=\"time\">{}</td></tr>\n",
                d.fajr, d.sunrise, d.dhuhr, d.asr, d.maghrib, d.isha, day = d.day,
            ));
        }
    }
    body.push_str("</table>\n");
    body.push_str(&format!(
        "<p class=\"note\">Метод расчёта: {}. Часовой пояс: {}.</p>\n",
        data.method_description, data.timezone
    ));

    let m_idx = data.months.iter().position(|m| m.slug == month.slug).unwrap_or(0);
    let prev_m = if m_idx > 0 { Some(&data.months[m_idx - 1]) } else { None };
    let next_m = if m_idx < data.months.len() - 1 { Some(&data.months[m_idx + 1]) } else { None };
    let left = match prev_m {
        Some(p) => format!(r#"<a href="{city_path}{slug}/">&larr; {name}</a>"#, slug = p.slug, name = p.name),
        None => "<span></span>".to_string(),
    };
    let right = match next_m {
        Some(n) => format!(r#"<a href="{city_path}{slug}/">{name} &rarr;</a>"#, slug = n.slug, name = n.name),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let islam_path = format!("{}islam/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Ислам", &islam_path),
        (ui.namaz, pfx),
        (&city.name, &city_path),
        (&month.name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Ислам".to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        (ui.namaz.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (city.name.clone(), format!("{}{}", html::DOMAIN, city_path)),
        (month.name.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let mn = month.name.to_lowercase();
    let title = format!("Время намаза в {} на {} {} года — myqaz.kz", city.name_prep, mn, data.year);
    let desc = format!("Время намаза в {} на {} {} года.", city.name_prep, mn, data.year);

    html::page(lang, &title, &desc, &path, &nav,
        &format!("<h1>Расписание намаза в {} — {} {}</h1>\n{body}\n{nav_bot}", city.name_prep, month.name, data.year),
        None, Some(&bc), EXTRA_STYLE)
}

pub fn render_day(store: &AdaptoStore, city_slug: &str, month_slug: &str, day_str: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let city = match data.cities.iter().find(|c| c.slug == city_slug) {
        Some(c) => c,
        None => return "<h1>City not found</h1>".to_string(),
    };
    let month = match data.months.iter().find(|m| m.slug == month_slug) {
        Some(m) => m,
        None => return "<h1>Month not found</h1>".to_string(),
    };
    let day_num: u32 = match day_str.parse() {
        Ok(n) => n,
        Err(_) => return "<h1>Invalid day</h1>".to_string(),
    };
    let key = month.number.to_string();
    let days = city.months.get(&key);
    let day = days.and_then(|ds| ds.iter().find(|d| d.day == day_num));
    let day = match day {
        Some(d) => d,
        None => return "<h1>Day not found</h1>".to_string(),
    };

    let city_path = format!("{pfx}{}/", city.slug);
    let month_path = format!("{pfx}{}/{}/", city.slug, month.slug);
    let path = format!("{pfx}{}/{}/{}/", city.slug, month.slug, day_num);

    let weekday = day_of_week(data.year, month.number, day_num);
    let mut body = format!("<div class=\"card\">\n<p class=\"label\">{}, {} {} {} г.</p>\n", weekday, day_num, month.name_genitive.to_lowercase(), data.year);
    let prayer_descs_ru = [
        ("Бамдат", "bamdat", "утренний намаз", ui.fajr),
        ("Күн шығуы", "kun-shygysy", "восход солнца", ui.sunrise),
        ("Бесін", "besin", "полуденный намаз", ui.dhuhr),
        ("Намаздыгер", "namazdyger", "послеполуденный намаз", ui.asr),
        ("Ақшам", "aqsham", "вечерний намаз", ui.maghrib),
        ("Құптан", "quptan", "ночной намаз", ui.isha),
    ];
    let times = [&day.fajr, &day.sunrise, &day.dhuhr, &day.asr, &day.maghrib, &day.isha];
    for (i, (kk, slug, desc_ru, ru)) in prayer_descs_ru.iter().enumerate() {
        let time = times[i];
        body.push_str(&format!(
            "<p><a href=\"{path}{slug}/\" class=\"prayer-name\">{kk}</a> <span class=\"val\">{time}</span> <span class=\"label\">— {desc_ru} <span class=\"kz\">({ru})</span></span></p>\n"
        ));
    }
    body.push_str("</div>\n");

    body.push_str(&format!(
        "<p><a href=\"{month_path}\">Полное расписание на {} {}</a></p>\n",
        month.name.to_lowercase(), data.year
    ));
    body.push_str("<p class=\"note\">Метод расчёта: Ханафитский мазхаб, MWL (Fajr 18°, Isha 17°, Asr Hanafi).</p>\n");

    let prev_day = if day_num > 1 { Some(day_num - 1) } else { None };
    let max_day = days.map(|ds| ds.len() as u32).unwrap_or(31);
    let next_day = if day_num < max_day { Some(day_num + 1) } else { None };
    let left = match prev_day {
        Some(p) => format!(r#"<a href="{month_path}{p}/">&larr; {p} {}</a>"#, month.name_genitive),
        None => "<span></span>".to_string(),
    };
    let right = match next_day {
        Some(n) => format!(r#"<a href="{month_path}{n}/">{n} {} &rarr;</a>"#, month.name_genitive),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let islam_path = format!("{}islam/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Ислам", &islam_path),
        (ui.namaz, pfx),
        (&city.name, &city_path),
        (&month.name, &month_path),
        (day_str, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Ислам".to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        (ui.namaz.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (city.name.clone(), format!("{}{}", html::DOMAIN, city_path)),
        (month.name.clone(), format!("{}{}", html::DOMAIN, month_path)),
        (day_str.to_string(), format!("{}{}", html::DOMAIN, path)),
    ];

    let mn_gen = month.name_genitive.to_lowercase();
    let title = format!("Время намаза в {}, {} {} {} — myqaz.kz", city.name, day_num, mn_gen, data.year);
    let h1 = format!("Время намаза в {} на {} {} {} года", city.name_prep, day_num, mn_gen, data.year);
    let desc = format!("Время намаза в {}, {} {} {}. {}: {}, {}: {}.",
        city.name, day_num, mn_gen, data.year, ui.fajr, day.fajr, ui.maghrib, day.maghrib);

    html::page(lang, &title, &desc, &path, &nav,
        &format!("<h1>{h1}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}

struct PrayerInfo {
    slug: &'static str,
    kk_name: &'static str,
    ru_name: &'static str,
}

const PRAYERS: [PrayerInfo; 6] = [
    PrayerInfo { slug: "bamdat", kk_name: "Бамдат", ru_name: "Фаджр" },
    PrayerInfo { slug: "kun-shygysy", kk_name: "Күн шығуы", ru_name: "Восход" },
    PrayerInfo { slug: "besin", kk_name: "Бесін", ru_name: "Зухр" },
    PrayerInfo { slug: "namazdyger", kk_name: "Намаздыгер", ru_name: "Аср" },
    PrayerInfo { slug: "aqsham", kk_name: "Ақшам", ru_name: "Магриб" },
    PrayerInfo { slug: "quptan", kk_name: "Құптан", ru_name: "Иша" },
];

fn get_prayer_time(day: &crate::types::NamazDay, slug: &str) -> String {
    match slug {
        "bamdat" => day.fajr.clone(),
        "kun-shygysy" => day.sunrise.clone(),
        "besin" => day.dhuhr.clone(),
        "namazdyger" => day.asr.clone(),
        "aqsham" => day.maghrib.clone(),
        "quptan" => day.isha.clone(),
        _ => "—".to_string(),
    }
}

pub fn render_prayer(store: &AdaptoStore, city_slug: &str, month_slug: &str, day_str: &str, prayer_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let city = match data.cities.iter().find(|c| c.slug == city_slug) {
        Some(c) => c,
        None => return "<h1>City not found</h1>".to_string(),
    };
    let month = match data.months.iter().find(|m| m.slug == month_slug) {
        Some(m) => m,
        None => return "<h1>Month not found</h1>".to_string(),
    };
    let day_num: u32 = match day_str.parse() {
        Ok(n) => n,
        Err(_) => return "<h1>Invalid day</h1>".to_string(),
    };
    let key = month.number.to_string();
    let days = city.months.get(&key);
    let day = match days.and_then(|ds| ds.iter().find(|d| d.day == day_num)) {
        Some(d) => d,
        None => return "<h1>Day not found</h1>".to_string(),
    };
    let prayer = match PRAYERS.iter().find(|p| p.slug == prayer_slug) {
        Some(p) => p,
        None => return "<h1>Prayer not found</h1>".to_string(),
    };

    let time = get_prayer_time(day, prayer_slug);
    let mn_gen = month.name_genitive.to_lowercase();
    let city_path = format!("{pfx}{}/", city.slug);
    let month_path = format!("{pfx}{}/{}/", city.slug, month.slug);
    let day_path = format!("{pfx}{}/{}/{}/", city.slug, month.slug, day_num);
    let path = format!("{day_path}{}/", prayer.slug);

    let title = format!("{} в {}, {} {} {} — {} — myqaz.kz", prayer.kk_name, city.name, day_num, mn_gen, data.year, time);
    let h1 = format!("{} ({}) в {} — {} {} {}", prayer.kk_name, prayer.ru_name, city.name, day_num, mn_gen, data.year);

    let weekday = day_of_week(data.year, month.number, day_num);
    let prayer_descs = [
        ("bamdat", "Утренний намаз (Бамдат/Фаджр) совершается от начала рассвета до восхода солнца."),
        ("kun-shygysy", "Күн шығуы (Восход) — время восхода солнца."),
        ("besin", "Бесін (Зухр) — полуденный намаз, совершается после прохождения солнцем зенита."),
        ("namazdyger", "Намаздыгер (Аср) — послеполуденный намаз."),
        ("aqsham", "Ақшам (Магриб) — вечерний намаз, совершается после захода солнца."),
        ("quptan", "Құптан (Иша) — ночной намаз."),
    ];
    let desc_text = prayer_descs.iter().find(|(s, _)| *s == prayer_slug).map(|(_, d)| *d).unwrap_or("");

    let mut body = format!("<div class=\"card\">\n<p class=\"label\">{weekday}, {day_num} {} {year} г.</p>\n<p><span class=\"val\" style=\"font-size:32px\">{time}</span></p>\n<p>{desc_text}</p>\n</div>\n", mn_gen, year = data.year);
    let mut inline_prayers = Vec::new();
    for p in &PRAYERS {
        let t = get_prayer_time(day, p.slug);
        if p.slug == prayer_slug {
            inline_prayers.push(format!("<strong>{} {}</strong>", p.kk_name, t));
        } else {
            inline_prayers.push(format!("<a href=\"{day_path}{}/\">{} {}</a>", p.slug, p.kk_name, t));
        }
    }
    body.push_str(&format!("<p>{}</p>\n", inline_prayers.join(" &middot; ")));
    body.push_str(&format!("<p><a href=\"{day_path}\">Все намазы на {} {}</a></p>\n", day_num, mn_gen));
    body.push_str(&format!("<p class=\"note\">{} ({}) — {} {} {} г.</p>\n",
        prayer.kk_name, prayer.ru_name, day_num, mn_gen, data.year));

    let max_day = days.map(|ds| ds.len() as u32).unwrap_or(31);
    let prev_day = if day_num > 1 { Some(day_num - 1) } else { None };
    let next_day = if day_num < max_day { Some(day_num + 1) } else { None };
    let left = match prev_day {
        Some(p) => format!(r#"<a href="{pfx}{}/{}/{p}/{prayer_slug}/">&larr; {p} {}</a>"#, city.slug, month.slug, mn_gen),
        None => "<span></span>".to_string(),
    };
    let right_label = match next_day {
        Some(n) => {
            let next_days = city.months.get(&key);
            let next_time = next_days.and_then(|ds| ds.iter().find(|dd| dd.day == n)).map(|dd| get_prayer_time(dd, prayer_slug)).unwrap_or_default();
            format!(r#"<a href="{pfx}{}/{}/{n}/{prayer_slug}/">{n} {}: {} &rarr;</a>"#, city.slug, month.slug, mn_gen, next_time)
        }
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right_label}</div>"#);

    let islam_path = format!("{}islam/", lang.path_prefix());
    let day_label = day_str.to_string();
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        ("Ислам", &islam_path),
        (ui.namaz, pfx),
        (&city.name, &city_path),
        (&month.name, &month_path),
        (&day_label, &day_path),
        (prayer.kk_name, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        ("Ислам".to_string(), format!("{}{}", html::DOMAIN, islam_path)),
        (ui.namaz.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (city.name.clone(), format!("{}{}", html::DOMAIN, city_path)),
        (month.name.clone(), format!("{}{}", html::DOMAIN, month_path)),
        (day_label.clone(), format!("{}{}", html::DOMAIN, day_path)),
        (prayer.kk_name.to_string(), format!("{}{}", html::DOMAIN, path)),
    ];

    let desc = format!("{} ({}) в {}, {} {} {} — {}.",
        prayer.kk_name, prayer.ru_name, city.name, day_num, mn_gen, data.year, time);

    html::page(lang, &title, &desc, &path, &nav,
        &format!("<h1>{h1}</h1>\n{body}\n{nav_bot}"),
        None, Some(&bc), EXTRA_STYLE)
}
