use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    Ru,
    Kk,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::Kk => "kk",
        }
    }

    pub fn og_locale(self) -> &'static str {
        match self {
            Lang::Ru => "ru_RU",
            Lang::Kk => "kk_KZ",
        }
    }

    pub fn path_prefix(self) -> &'static str {
        match self {
            Lang::Ru => "/",
            Lang::Kk => "/kz/",
        }
    }

    pub fn alternate(self) -> Lang {
        match self {
            Lang::Ru => Lang::Kk,
            Lang::Kk => Lang::Ru,
        }
    }

    pub fn ui(self) -> &'static UiStrings {
        match self {
            Lang::Ru => &UI_RU,
            Lang::Kk => &UI_KK,
        }
    }
}

impl fmt::Display for Lang {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

pub struct UiStrings {
    pub home: &'static str,
    pub law: &'static str,
    pub laws: &'static str,
    pub codes: &'static str,
    pub constitution: &'static str,
    pub article: &'static str,
    pub chapter: &'static str,
    pub section: &'static str,
    pub point: &'static str,
    pub prev: &'static str,
    pub next: &'static str,
    pub directory: &'static str,
    pub reference: &'static str,
    pub table_of_contents: &'static str,
    pub site_name: &'static str,
    pub lang_switch_label: &'static str,
    pub notaries: &'static str,
    pub bailiffs: &'static str,
    pub financial_indicators: &'static str,
    pub measurement_units: &'static str,
    pub tax_rates: &'static str,
    pub payment_codes: &'static str,
    pub phone_codes: &'static str,
    pub postal_codes: &'static str,
    pub waste_codes: &'static str,
    pub government: &'static str,
    pub namaz: &'static str,
    pub quran: &'static str,
    pub hadith: &'static str,
    pub bible: &'static str,
    pub address: &'static str,
    pub phone: &'static str,
    pub email: &'static str,
    pub license: &'static str,
    pub year: &'static str,
    pub value: &'static str,
    pub source: &'static str,
    pub name: &'static str,
    pub code: &'static str,
    pub group: &'static str,
    pub count: &'static str,
    pub designation: &'static str,
    pub designation_intl: &'static str,
    pub region: &'static str,
    pub country_code: &'static str,
    pub number_format: &'static str,
    pub settlement: &'static str,
    pub hazardous: &'static str,
    pub month_names: &'static [&'static str; 12],
    pub fajr: &'static str,
    pub sunrise: &'static str,
    pub dhuhr: &'static str,
    pub asr: &'static str,
    pub maghrib: &'static str,
    pub isha: &'static str,
}

static MONTHS_RU: [&str; 12] = [
    "Январь", "Февраль", "Март", "Апрель", "Май", "Июнь",
    "Июль", "Август", "Сентябрь", "Октябрь", "Ноябрь", "Декабрь",
];

static MONTHS_KK: [&str; 12] = [
    "Қаңтар", "Ақпан", "Наурыз", "Сәуір", "Мамыр", "Маусым",
    "Шілде", "Тамыз", "Қыркүйек", "Қазан", "Қараша", "Желтоқсан",
];

pub static UI_RU: UiStrings = UiStrings {
    home: "Главная",
    law: "Законодательство",
    laws: "Законы",
    codes: "Кодексы",
    constitution: "Конституция",
    article: "Статья",
    chapter: "Глава",
    section: "Раздел",
    point: "Пункт",
    prev: "← Предыдущая",
    next: "Следующая →",
    directory: "Справочник",
    reference: "Справочные таблицы",
    table_of_contents: "Содержание",
    site_name: "myqaz.kz",
    lang_switch_label: "Қаз",
    notaries: "Нотариусы",
    bailiffs: "Частные судебные исполнители",
    financial_indicators: "Финансовые показатели",
    measurement_units: "Единицы измерений",
    tax_rates: "Налоговые ставки",
    payment_codes: "Коды платежей",
    phone_codes: "Телефонные коды",
    postal_codes: "Почтовые индексы",
    waste_codes: "Коды отходов",
    government: "Госорганы",
    namaz: "Время намаза",
    quran: "Коран",
    hadith: "Хадисы",
    bible: "Библия",
    address: "Адрес",
    phone: "Телефон",
    email: "Email",
    license: "Лицензия",
    year: "Год",
    value: "Значение",
    source: "Источник",
    name: "Наименование",
    code: "Код",
    group: "Группа",
    count: "Кол-во",
    designation: "Обозначение (рус.)",
    designation_intl: "Обозначение (межд.)",
    region: "Область",
    country_code: "Код страны",
    number_format: "Формат номера",
    settlement: "Населённый пункт",
    hazardous: "Опасный",
    month_names: &MONTHS_RU,
    fajr: "Фаджр",
    sunrise: "Восход",
    dhuhr: "Зухр",
    asr: "Аср",
    maghrib: "Магриб",
    isha: "Иша",
};

pub static UI_KK: UiStrings = UiStrings {
    home: "Басты бет",
    law: "Заңнама",
    laws: "Заңдар",
    codes: "Кодекстер",
    constitution: "Конституция",
    article: "Бап",
    chapter: "Тарау",
    section: "Бөлім",
    point: "Тармақ",
    prev: "← Алдыңғы",
    next: "Келесі →",
    directory: "Анықтамалық",
    reference: "Анықтамалық кестелер",
    table_of_contents: "Мазмұны",
    site_name: "myqaz.kz",
    lang_switch_label: "Рус",
    notaries: "Нотариустар",
    bailiffs: "Жеке сот орындаушылары",
    financial_indicators: "Қаржылық көрсеткіштер",
    measurement_units: "Өлшем бірліктері",
    tax_rates: "Салық мөлшерлемелері",
    payment_codes: "Төлем кодтары",
    phone_codes: "Телефон кодтары",
    postal_codes: "Пошта индекстері",
    waste_codes: "Қалдық кодтары",
    government: "Мемлекеттік органдар",
    namaz: "Намаз уақыты",
    quran: "Құран",
    hadith: "Хадис",
    bible: "Інжіл",
    address: "Мекенжай",
    phone: "Телефон",
    email: "Email",
    license: "Лицензия",
    year: "Жыл",
    value: "Мәні",
    source: "Дереккөз",
    name: "Атауы",
    code: "Код",
    group: "Топ",
    count: "Саны",
    designation: "Белгіленуі (қаз.)",
    designation_intl: "Белгіленуі (халықар.)",
    region: "Облыс",
    country_code: "Ел коды",
    number_format: "Нөмір форматы",
    settlement: "Елді мекен",
    hazardous: "Қауіпті",
    month_names: &MONTHS_KK,
    fajr: "Фажр",
    sunrise: "Шұғыла",
    dhuhr: "Зұхр",
    asr: "Аср",
    maghrib: "Мағрип",
    isha: "Ишақ",
};

pub fn hreflang_tags(path: &str, lang: Lang) -> String {
    let domain = crate::html::DOMAIN;
    let alt = lang.alternate();
    let alt_path = match (lang, alt) {
        (Lang::Ru, Lang::Kk) => format!("/kz{path}"),
        (Lang::Kk, Lang::Ru) => path.strip_prefix("/kz").unwrap_or(path).to_string(),
        _ => path.to_string(),
    };
    format!(
        r#"<link rel="alternate" hreflang="{}" href="{domain}{path}">
<link rel="alternate" hreflang="{}" href="{domain}{alt_path}">"#,
        lang.code(),
        alt.code(),
    )
}

pub fn lang_switcher(path: &str, lang: Lang) -> String {
    let alt = lang.alternate();
    let alt_path = match (lang, alt) {
        (Lang::Ru, Lang::Kk) => format!("/kz{path}"),
        (Lang::Kk, Lang::Ru) => path.strip_prefix("/kz").unwrap_or(path).to_string(),
        _ => path.to_string(),
    };
    let label = lang.ui().lang_switch_label;
    format!(r#"<a href="{alt_path}" style="font-size:13px;margin-left:12px">{label}</a>"#)
}
