use adapto_store::AdaptoStore;
use crate::html;
use crate::lang::Lang;
use crate::types::WasteData;
use adapto_store::Query as StoreQuery;

const EXTRA_STYLE: &str = r#"<style>
td.code { font-family: monospace; font-weight: bold; }
td.count { text-align: right; color: #666; }
td.hazard { text-align: center; }
tr:hover { background: #f9f9f9; }
.card p { margin: 8px 0; }
.label { color: #666; font-size: 14px; }
.hazardous { color: #c00; font-weight: bold; }
</style>"#;

fn load_data(store: &AdaptoStore) -> Option<WasteData> {
    let col = store.collection("waste_codes");
    let doc = col.find(StoreQuery::new()).next()?;
    serde_json::from_value(doc.data).ok()
}

fn url_prefix(lang: Lang) -> &'static str {
    match lang {
        Lang::Ru => "/reference/waste-codes/",
        Lang::Kk => "/kz/reference/waste-codes/",
    }
}

fn code_slug(code: &str) -> String {
    code.replace(' ', "-")
}

pub fn render_index(store: &AdaptoStore, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();

    let mut body = format!(
        "<table>\n<tr><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        ui.code, ui.group, ui.count,
    );
    for g in &data.groups {
        let slug = code_slug(&g.code);
        let total: usize = g.subgroups.iter().map(|sg| sg.items.len()).sum();
        body.push_str(&format!(
            "<tr><td class=\"code\"><a href=\"{pfx}{slug}/\">{code}</a></td><td>{name}</td><td class=\"count\">{total}</td></tr>\n",
            code = g.code,
            name = g.name_ru,
        ));
    }
    body.push_str("</table>\n");
    body.push_str(&format!(
        "<p class=\"note\">{}: {}.</p>\n",
        ui.source, data.source
    ));

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.waste_codes, ""),
    ], lang, pfx);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.waste_codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
    ];

    html::page(
        lang,
        &format!("{} — myqaz.kz", data.index_title),
        &data.index_description,
        pfx,
        &nav,
        &format!("<h1>{}</h1>\n{body}", data.index_title),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_group(store: &AdaptoStore, group_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let groups = &data.groups;
    let idx = match groups.iter().position(|g| code_slug(&g.code) == group_slug) {
        Some(i) => i,
        None => return "<h1>Group not found</h1>".to_string(),
    };
    let g = &groups[idx];
    let slug = code_slug(&g.code);
    let path = format!("{pfx}{slug}/");

    let mut body = format!(
        "<table>\n<tr><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        ui.code, ui.name, ui.count,
    );
    for sg in &g.subgroups {
        let sg_slug = code_slug(&sg.code);
        body.push_str(&format!(
            "<tr><td class=\"code\"><a href=\"{path}{sg_slug}/\">{code}</a></td><td>{name}</td><td class=\"count\">{count}</td></tr>\n",
            code = sg.code,
            name = sg.name_ru,
            count = sg.items.len(),
        ));
    }
    body.push_str("</table>\n");

    let prev = if idx > 0 { Some(&groups[idx - 1]) } else { None };
    let next = if idx < groups.len() - 1 { Some(&groups[idx + 1]) } else { None };

    let left = match prev {
        Some(p) => format!(r#"<a href="{pfx}{slug}/">&larr; {code}</a>"#, slug = code_slug(&p.code), code = p.code),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{pfx}{slug}/">{code} &rarr;</a>"#, slug = code_slug(&n.code), code = n.code),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.waste_codes, pfx),
        (&g.code, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.waste_codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (g.code.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("{} — {}", g.code, g.name_ru);
    let desc = format!("Код отхода {} — {}. {} подгрупп.", g.code, g.name_ru, g.subgroups.len());

    html::page(
        lang,
        &format!("{title} — myqaz.kz"),
        &desc,
        &path,
        &nav,
        &format!("<h1>{title}</h1>\n{body}\n{nav_bot}"),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_subgroup(store: &AdaptoStore, group_slug: &str, subgroup_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let group = match data.groups.iter().find(|g| code_slug(&g.code) == group_slug) {
        Some(g) => g,
        None => return "<h1>Group not found</h1>".to_string(),
    };
    let sg_idx = match group.subgroups.iter().position(|sg| code_slug(&sg.code) == subgroup_slug) {
        Some(i) => i,
        None => return "<h1>Subgroup not found</h1>".to_string(),
    };
    let sg = &group.subgroups[sg_idx];
    let g_slug = code_slug(&group.code);
    let sg_slug_str = code_slug(&sg.code);
    let path = format!("{pfx}{g_slug}/{sg_slug_str}/");
    let group_path = format!("{pfx}{g_slug}/");

    let mut body = format!(
        "<table>\n<tr><th>{}</th><th>{}</th><th>{}</th></tr>\n",
        ui.code, ui.name, ui.hazardous,
    );
    for item in &sg.items {
        let item_slug = code_slug(&item.code);
        let hazard = if item.hazardous { "<span class=\"hazardous\">*</span>" } else { "" };
        body.push_str(&format!(
            "<tr><td class=\"code\"><a href=\"{path}{item_slug}/\">{code}</a></td><td>{name}</td><td class=\"hazard\">{hazard}</td></tr>\n",
            code = item.code,
            name = item.name_ru,
        ));
    }
    body.push_str("</table>\n");

    let prev = if sg_idx > 0 { Some(&group.subgroups[sg_idx - 1]) } else { None };
    let next = if sg_idx < group.subgroups.len() - 1 { Some(&group.subgroups[sg_idx + 1]) } else { None };

    let left = match prev {
        Some(p) => format!(r#"<a href="{group_path}{slug}/">&larr; {code}</a>"#, slug = code_slug(&p.code), code = p.code),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some(n) => format!(r#"<a href="{group_path}{slug}/">{code} &rarr;</a>"#, slug = code_slug(&n.code), code = n.code),
        None => "<span></span>".to_string(),
    };
    let nav_bot = format!(r#"<div class="nav-bottom">{left}{right}</div>"#);

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.waste_codes, pfx),
        (&group.code, &group_path),
        (&sg.code, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.waste_codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (group.code.clone(), format!("{}{}", html::DOMAIN, group_path)),
        (sg.code.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("{} — {}", sg.code, sg.name_ru);
    let desc = format!("Код отхода {} — {}. {} позиций.", sg.code, sg.name_ru, sg.items.len());

    html::page(
        lang,
        &format!("{title} — myqaz.kz"),
        &desc,
        &path,
        &nav,
        &format!("<h1>{title}</h1>\n{body}\n{nav_bot}"),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}

pub fn render_item(store: &AdaptoStore, group_slug: &str, subgroup_slug: &str, item_slug: &str, lang: Lang) -> String {
    let data = match load_data(store) {
        Some(d) => d,
        None => return "<h1>Data not found</h1>".to_string(),
    };
    let pfx = url_prefix(lang);
    let ui = lang.ui();
    let group = match data.groups.iter().find(|g| code_slug(&g.code) == group_slug) {
        Some(g) => g,
        None => return "<h1>Group not found</h1>".to_string(),
    };
    let sg = match group.subgroups.iter().find(|sg| code_slug(&sg.code) == subgroup_slug) {
        Some(sg) => sg,
        None => return "<h1>Subgroup not found</h1>".to_string(),
    };
    let item = match sg.items.iter().find(|it| code_slug(&it.code) == item_slug) {
        Some(it) => it,
        None => return "<h1>Item not found</h1>".to_string(),
    };
    let g_slug = code_slug(&group.code);
    let sg_slug_str = code_slug(&sg.code);
    let it_slug = code_slug(&item.code);
    let path = format!("{pfx}{g_slug}/{sg_slug_str}/{it_slug}/");
    let group_path = format!("{pfx}{g_slug}/");
    let sg_path = format!("{pfx}{g_slug}/{sg_slug_str}/");

    let mut body = String::from("<div class=\"card\">\n");
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> <span class=\"val\">{}</span></p>\n",
        ui.code, item.code
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> {}</p>\n",
        ui.name, item.name_ru
    ));
    if !item.name_kz.is_empty() {
        body.push_str(&format!(
            "<p><span class=\"label\">{} (қаз.):</span> {}</p>\n",
            ui.name, item.name_kz
        ));
    }
    if item.hazardous {
        body.push_str(&format!(
            "<p><span class=\"hazardous\">{}: Да</span></p>\n",
            ui.hazardous
        ));
    }
    body.push_str(&format!(
        "<p><span class=\"label\">Подгруппа:</span> <a href=\"{sg_path}\">{}</a></p>\n",
        sg.code
    ));
    body.push_str(&format!(
        "<p><span class=\"label\">{}:</span> <a href=\"{group_path}\">{}</a></p>\n",
        ui.group, group.code
    ));
    body.push_str("</div>\n");

    let ref_path = format!("{}reference/", lang.path_prefix());
    let nav = html::nav_breadcrumb(&[
        ("myqaz.kz", "/"),
        (ui.reference, &ref_path),
        (ui.waste_codes, pfx),
        (&group.code, &group_path),
        (&sg.code, &sg_path),
        (&item.code, ""),
    ], lang, &path);

    let bc = vec![
        ("myqaz.kz".to_string(), format!("{}/", html::DOMAIN)),
        (ui.reference.to_string(), format!("{}{}", html::DOMAIN, ref_path)),
        (ui.waste_codes.to_string(), format!("{}{}", html::DOMAIN, pfx)),
        (group.code.clone(), format!("{}{}", html::DOMAIN, group_path)),
        (sg.code.clone(), format!("{}{}", html::DOMAIN, sg_path)),
        (item.code.clone(), format!("{}{}", html::DOMAIN, path)),
    ];

    let title = format!("{} — {}", item.code, item.name_ru);
    let hazard_str = if item.hazardous { " Опасный отход." } else { "" };
    let desc = format!("Код отхода {} — {}.{}", item.code, item.name_ru, hazard_str);

    html::page(
        lang,
        &format!("{title} — myqaz.kz"),
        &desc,
        &path,
        &nav,
        &format!("<h1>{title}</h1>\n{body}"),
        None,
        Some(&bc),
        EXTRA_STYLE,
    )
}
