use crate::lang::Lang;

pub const DOMAIN: &str = "https://myqaz.kz";

pub const STYLE: &str = r#"<style>
body { max-width: 800px; margin: 0 auto; padding: 20px; font-family: Arial, sans-serif; }
a { color: #008ace; text-decoration: none; border-bottom: 1px solid #b2ccf0; }
a:hover { color: #C00; border-color: #f0b2b2; }
nav { margin-bottom: 20px; font-size: 14px; }
ul { line-height: 1.8; }
h2 { margin-top: 30px; }
.point { margin: 15px 0; padding: 10px 0; border-bottom: 1px solid #eee; }
.point-num { font-weight: bold; }
.nav-bottom { margin-top: 30px; display: flex; justify-content: space-between; font-size: 14px; }
blockquote { border-left: 3px solid #ccc; margin: 20px 0; padding: 10px 20px; font-size: 18px; line-height: 1.6; }
.subpoints { margin: 5px 0 0 20px; line-height: 1.8; }
.alpha-nav { display: flex; flex-wrap: wrap; gap: 8px; margin: 20px 0; }
.alpha-nav a { padding: 4px 8px; }
body { line-height: 1.6; }
nav { line-height: 1.6; }
table { width: 100%; border-collapse: collapse; margin-top: 10px; }
th, td { text-align: left; padding: 8px 12px; border-bottom: 1px solid #eee; }
th { font-weight: bold; border-bottom: 2px solid #ccc; }
.card { background: #f9f9f9; padding: 20px; margin: 20px 0; }
.card .val { font-weight: bold; font-size: 24px; color: #008ace; }
@media (max-width: 600px) {
  body { padding: 16px 14px; font-size: 16px; }
  h1 { font-size: 22px; line-height: 1.3; }
  h2 { font-size: 18px; margin-top: 24px; }
  nav { font-size: 13px; line-height: 1.5; }
  table { display: block; overflow-x: auto; -webkit-overflow-scrolling: touch; }
  thead, tbody, tr { min-width: max-content; }
  th, td { padding: 10px 8px; font-size: 14px; }
  td a { display: inline-block; padding: 4px 0; min-height: 32px; line-height: 1.5; }
  ul { padding-left: 20px; }
  li { padding: 2px 0; }
  li a { display: inline-block; padding: 4px 0; }
  .nav-bottom { flex-direction: column; gap: 12px; align-items: flex-start; }
  .nav-bottom a { padding: 8px 0; min-height: 44px; display: flex; align-items: center; }
  .card { padding: 16px; margin: 16px 0; }
  .card .val { font-size: 20px; }
  blockquote { padding: 8px 14px; font-size: 16px; margin: 16px 0; }
  input[type="text"] { font-size: 16px; padding: 12px; border-radius: 0; -webkit-appearance: none; }
  .table-wide { padding: 0 14px; }
  .alpha-nav a { padding: 8px 12px; min-height: 44px; display: inline-flex; align-items: center; }
}
</style>"#;

pub fn seo_head(
    title: &str,
    description: &str,
    path: &str,
    schema_json: Option<&str>,
    breadcrumbs: Option<&[(String, String)]>,
) -> String {
    seo_head_lang(title, description, path, schema_json, breadcrumbs, Lang::Ru, false)
}

pub fn seo_head_lang(
    title: &str,
    description: &str,
    path: &str,
    schema_json: Option<&str>,
    breadcrumbs: Option<&[(String, String)]>,
    lang: Lang,
    has_alternate: bool,
) -> String {
    let url = format!("{DOMAIN}{path}");
    let locale = lang.og_locale();
    let desc_truncated = truncate_chars(description, 200);

    let mut parts = vec![format!(
        r#"<link rel="icon" href="/favicon.ico" type="image/x-icon">
<link rel="canonical" href="{url}">
<meta property="og:title" content="{title}">
<meta property="og:description" content="{desc_truncated}">
<meta property="og:url" content="{url}">
<meta property="og:type" content="article">
<meta property="og:site_name" content="myqaz.kz">
<meta property="og:locale" content="{locale}">"#
    )];

    if has_alternate {
        parts.push(crate::lang::hreflang_tags(path, lang));
    }

    let mut schemas = Vec::new();

    if let Some(schema) = schema_json {
        schemas.push(schema.to_string());
    }

    if let Some(crumbs) = breadcrumbs {
        let items: Vec<String> = crumbs
            .iter()
            .enumerate()
            .map(|(i, (name, url))| {
                format!(
                    r#"{{"@type":"ListItem","position":{},"name":"{}","item":"{}"}}"#,
                    i + 1, name, url
                )
            })
            .collect();
        schemas.push(format!(
            r#"{{"@context":"https://schema.org","@type":"BreadcrumbList","itemListElement":[{}]}}"#,
            items.join(",")
        ));
    }

    for schema in &schemas {
        parts.push(format!(
            r#"<script type="application/ld+json">{schema}</script>"#
        ));
    }

    parts.join("\n")
}

const FEEDBACK_WIDGET: &str = r##"<style>
.pg-fb{margin:48px 0 24px;padding:24px 0 0;border-top:1px solid #e5e5e5}
.fb-row{display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:16px}
.fb-left{display:flex;align-items:center;gap:12px;color:#666;font-size:15px;line-height:1;transition:color .3s ease}
.fb-left.fb-done{color:#48a84a}
.fb-check{display:inline-block;width:18px;height:18px;vertical-align:middle;margin-right:2px}
.fb-vote{display:inline-flex;gap:0;border:1px solid #d5d5d5;border-radius:8px;overflow:hidden}
.fb-vote button{padding:8px 20px;border:none;border-right:1px solid #d5d5d5;background:#fff;color:#333;cursor:pointer;font:inherit;font-size:14px;line-height:1;transition:background .15s ease,color .15s ease}
.fb-vote button:last-child{border-right:none}
.fb-vote button:hover{background:#f0f7fc;color:#008ace}
.fb-err{background:none;border:none;color:#999;font:inherit;font-size:13px;cursor:pointer;padding:8px 0;transition:color .15s ease}
.fb-err:hover{color:#008ace}
.fb-form{margin-top:16px;max-height:0;overflow:hidden;opacity:0;transition:max-height .35s ease,opacity .3s ease .05s}
.fb-form.fb-open{max-height:260px;opacity:1}
.fb-form textarea{display:block;width:100%;max-width:480px;padding:12px 14px;border:1px solid #d5d5d5;border-radius:8px;font:inherit;font-size:14px;line-height:1.45;color:#333;resize:vertical;box-sizing:border-box}
.fb-form textarea:focus{border-color:#008ace;box-shadow:0 0 0 3px rgba(0,138,206,.12);outline:none}
.fb-submit{display:inline-block;margin-top:10px;padding:8px 24px;border:none;border-radius:8px;background:#008ace;color:#fff;font:inherit;font-size:14px;cursor:pointer;transition:background .15s ease}
.fb-submit:hover{background:#0077b5}
@media(max-width:600px){
.fb-row{flex-direction:column;align-items:flex-start;gap:14px}
.fb-vote button{padding:12px 28px;font-size:16px;min-height:44px}
.fb-err{font-size:14px;min-height:44px;display:flex;align-items:center}
.fb-form textarea{font-size:16px;padding:14px;max-width:100%}
.fb-submit{padding:12px 32px;font-size:16px;min-height:44px}
}
</style>
<div id="pg-fb" class="pg-fb">
<div class="fb-row">
<div class="fb-left" id="fb-left">
<span id="fb-q">Была ли страница полезной?</span>
<span class="fb-vote" id="fb-vote">
<button type="button" onclick="fbV(1)">Да</button>
<button type="button" onclick="fbV(0)">Нет</button>
</span>
</div>
<button type="button" class="fb-err" id="fb-err" onclick="fbE()">Сообщить об ошибке</button>
</div>
<div class="fb-form" id="fb-form">
<textarea id="fb-ta" maxlength="500" rows="3"></textarea>
<button type="button" class="fb-submit" onclick="fbS()">Отправить</button>
</div>
</div>
<script>
var _fbDone=false;
var _fbSvg='<svg class="fb-check" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg"><circle cx="9" cy="9" r="9" fill="#48a84a"/><path d="M5.5 9.2l2.2 2.2 4.8-4.8" stroke="#fff" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></svg>';
function _fbOk(msg){var el=document.getElementById('fb-left');el.classList.add('fb-done');var q=document.getElementById('fb-q');q.textContent='';q.appendChild(document.createRange().createContextualFragment(_fbSvg));q.appendChild(document.createTextNode(' '+msg));_fbDone=true;document.getElementById('fb-err').style.display='none'}
function fbV(v){if(_fbDone)return;document.getElementById('fb-vote').style.display='none';if(v===1){_fbOk('Спасибо!')}else{document.getElementById('fb-q').textContent='Спасибо за отзыв!';document.getElementById('fb-form').classList.add('fb-open')}}
function fbE(){if(_fbDone)return;document.getElementById('fb-form').classList.add('fb-open');document.getElementById('fb-ta').placeholder='Опишите ошибку';document.getElementById('fb-ta').focus()}
function fbS(){var t=document.getElementById('fb-ta').value.trim();if(!t)return;document.getElementById('fb-form').classList.remove('fb-open');_fbOk('Спасибо!')}
</script>"##;

pub fn page(
    lang: Lang,
    title: &str,
    description: &str,
    path: &str,
    nav_html: &str,
    body_html: &str,
    schema_json: Option<&str>,
    breadcrumbs: Option<&[(String, String)]>,
    extra_style: &str,
) -> String {
    let seo = seo_head_lang(title, description, path, schema_json, breadcrumbs, lang, false);
    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<meta name="description" content="{description}">
{seo}
{STYLE}
{extra_style}
</head>
<body>
{nav_html}
{body_html}
{FEEDBACK_WIDGET}
</body>
</html>"#,
        lang = lang.code(),
    )
}

pub fn nav_breadcrumb(crumbs: &[(&str, &str)], lang: Lang, path: &str) -> String {
    let mut parts = Vec::new();
    for (name, url) in crumbs {
        if url.is_empty() {
            parts.push((*name).to_string());
        } else {
            parts.push(format!(r#"<a href="{url}">{name}</a>"#));
        }
    }
    let switcher = crate::lang::lang_switcher(path, lang);
    format!("<nav>{} {switcher}</nav>", parts.join(" → "))
}

pub fn nav_bottom(prev: Option<(&str, &str)>, next: Option<(&str, &str)>) -> String {
    let left = match prev {
        Some((url, label)) => format!(r#"<a href="{url}">{label}</a>"#),
        None => "<span></span>".to_string(),
    };
    let right = match next {
        Some((url, label)) => format!(r#"<a href="{url}">{label}</a>"#),
        None => "<span></span>".to_string(),
    };
    format!(r#"<div class="nav-bottom">{left}{right}</div>"#)
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let end = s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len());
        s[..end].to_string()
    }
}
