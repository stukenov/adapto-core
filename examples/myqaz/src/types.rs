use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ===== Laws =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Law {
    pub doc_id: String,
    pub title: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub preamble: String,
    pub articles: Vec<Article>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub number: String,
    pub slug: String,
    pub title: String,
    pub articles: Vec<Article>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub number: String,
    pub slug: String,
    pub title: String,
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub number: String,
    #[serde(default)]
    pub slug: String,
    pub text: String,
    #[serde(default)]
    pub subpoints: Vec<Subpoint>,
    #[serde(default)]
    pub continuation: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subpoint {
    pub number: String,
    pub text: String,
}

// ===== Codes =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Code {
    pub title: String,
    pub title_genitive: String,
    pub slug: String,
    #[serde(default)]
    pub source: String,
    pub chapters: Vec<Chapter>,
}

// ===== Notaries =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotaryData {
    pub source: String,
    pub regions: Vec<NotaryRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotaryRegion {
    pub chamber_id: u32,
    pub slug: String,
    pub name: String,
    pub notaries: Vec<Notary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notary {
    pub name: String,
    pub slug: String,
    pub license_number: String,
    pub license_date: String,
    pub address: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
}

// ===== Namaz =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamazData {
    pub year: u32,
    pub method: String,
    pub method_description: String,
    pub asr_method: String,
    pub timezone: String,
    #[serde(default)]
    pub breadcrumb_label: String,
    #[serde(default)]
    pub index_title: String,
    #[serde(default)]
    pub index_description: String,
    pub months: Vec<NamazMonthMeta>,
    pub cities: Vec<NamazCity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamazMonthMeta {
    pub number: u32,
    pub slug: String,
    pub name: String,
    pub name_genitive: String,
    pub name_prepositional: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamazCity {
    pub slug: String,
    pub name: String,
    pub name_prep: String,
    pub lat: f64,
    pub lng: f64,
    pub months: HashMap<String, Vec<NamazDay>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamazDay {
    pub day: u32,
    pub fajr: String,
    pub sunrise: String,
    pub dhuhr: String,
    pub asr: String,
    pub maghrib: String,
    pub isha: String,
}

// ===== Quran =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuranData {
    pub surahs: Vec<Surah>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Surah {
    pub number: u32,
    pub slug: String,
    pub name_ar: String,
    pub name_transliterated: String,
    pub name_ru: String,
    pub meaning_ru: String,
    pub revelation: String,
    pub ayah_count: u32,
    pub ayahs: Vec<Ayah>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ayah {
    pub verse: u32,
    pub text_ru: String,
    pub text_ar: String,
}

// ===== Bible =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibleData {
    pub books: Vec<BibleBook>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibleBook {
    pub number: u32,
    pub abbrev: String,
    pub slug: String,
    pub name_en: String,
    pub name_ru: String,
    pub testament: String,
    pub group: String,
    pub chapter_count: u32,
    pub chapters: Vec<BibleChapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BibleChapter {
    pub number: u32,
    pub verses: Vec<String>,
}

// ===== Financial Indicators =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialIndicatorsData {
    pub currency: String,
    pub breadcrumb_label: String,
    pub index_title: String,
    pub index_description: String,
    pub indicators: Vec<FinancialIndicator>,
    pub years: Vec<FinancialYear>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialIndicator {
    pub slug: String,
    pub title: String,
    pub title_short: String,
    pub field: String,
    pub unit: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialYear {
    pub year: u32,
    #[serde(default)]
    pub mrp: Option<serde_json::Value>,
    #[serde(default)]
    pub mzp: Option<serde_json::Value>,
    #[serde(default)]
    pub subsistence: Option<serde_json::Value>,
    #[serde(default)]
    pub budget_law: Option<String>,
}

// ===== Measurement Units =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementData {
    #[serde(default)]
    pub breadcrumb_label: String,
    #[serde(default)]
    pub index_title: String,
    #[serde(default)]
    pub index_description: String,
    pub source: String,
    pub groups: Vec<MeasurementGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementGroup {
    pub slug: String,
    pub title: String,
    pub code_range: String,
    pub items: Vec<MeasurementUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementUnit {
    pub code: String,
    pub name: String,
    pub symbol_ru: String,
    pub symbol_int: String,
}

// ===== Phone Codes =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneData {
    #[serde(default)]
    pub breadcrumb_label: String,
    #[serde(default)]
    pub index_title: String,
    #[serde(default)]
    pub index_description: String,
    #[serde(default)]
    pub country_code: String,
    #[serde(default)]
    pub format: String,
    #[serde(default)]
    pub source: String,
    pub categories: Vec<PhoneCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneCategory {
    pub title: String,
    pub items: Vec<PhoneItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneItem {
    pub slug: String,
    pub city: String,
    pub code: String,
    #[serde(default)]
    pub region: String,
}

// ===== Postal Codes =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostalData {
    #[serde(default)]
    pub breadcrumb_label: String,
    #[serde(default)]
    pub index_title: String,
    #[serde(default)]
    pub index_description: String,
    #[serde(default)]
    pub source: String,
    pub regions: Vec<PostalRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostalRegion {
    pub slug: String,
    pub name: String,
    pub codes: Vec<PostalCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostalCode {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub index_new: String,
    #[serde(default)]
    pub index_old: String,
}

// ===== Waste Codes =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasteData {
    #[serde(default)]
    pub breadcrumb_label: String,
    #[serde(default)]
    pub index_title: String,
    #[serde(default)]
    pub index_description: String,
    #[serde(default)]
    pub source: String,
    pub groups: Vec<WasteGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasteGroup {
    pub code: String,
    pub name_ru: String,
    #[serde(default)]
    pub name_kz: String,
    pub subgroups: Vec<WasteSubgroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasteSubgroup {
    pub code: String,
    pub name_ru: String,
    #[serde(default)]
    pub name_kz: String,
    pub items: Vec<WasteItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasteItem {
    pub code: String,
    pub name_ru: String,
    #[serde(default)]
    pub name_kz: String,
    #[serde(default)]
    pub hazardous: bool,
}

// ===== Tax Rates =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxData {
    pub currency: String,
    pub mrp: Value,
    pub mzp: Value,
    #[serde(default)]
    pub year: u32,
    #[serde(default)]
    pub source: String,
    pub categories: Vec<TaxCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxCategory {
    pub slug: String,
    pub title: String,
    pub items: Vec<TaxItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxItem {
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub title_long: Option<String>,
    #[serde(default)]
    pub rate: Option<String>,
    #[serde(default)]
    pub article: Option<Value>,
    #[serde(default)]
    pub chapter: Option<Value>,
    #[serde(default)]
    pub law_slug: Option<String>,
    #[serde(default)]
    pub law_title: Option<String>,
    #[serde(default)]
    pub payers: Option<String>,
    #[serde(default)]
    pub sub_items: Vec<TaxSubItem>,
    #[serde(default)]
    pub items: Vec<TaxLeaf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxSubItem {
    pub slug: String,
    pub title: String,
    #[serde(default)]
    pub title_long: Option<String>,
    #[serde(default)]
    pub rate: Option<String>,
    #[serde(default)]
    pub rate_formula: Option<String>,
    #[serde(default)]
    pub rate_suffix: Option<String>,
    #[serde(default)]
    pub article: Option<Value>,
    #[serde(default)]
    pub chapter: Option<Value>,
    #[serde(default)]
    pub items: Vec<TaxLeaf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxLeaf {
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub range: Option<String>,
    #[serde(default)]
    pub rate: Option<String>,
    #[serde(default)]
    pub mrp_rate: Option<f64>,
    #[serde(default)]
    pub rate_per_m2: Option<Value>,
    #[serde(default)]
    pub formula: Option<String>,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub article: Option<Value>,
    #[serde(default)]
    pub chapter: Option<Value>,
}

// ===== Payment Codes =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentData {
    #[serde(default)]
    pub knp: Option<KnpData>,
    #[serde(default)]
    pub kse: Option<KseData>,
    #[serde(default)]
    pub oked: Option<OkedData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnpData {
    pub sections: Vec<KnpSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnpSection {
    pub number: String,
    pub title: String,
    pub codes: Vec<KnpCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnpCode {
    pub code: String,
    pub description: String,
    #[serde(default)]
    pub details: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KseData {
    pub sectors: Vec<KseSector>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KseSector {
    pub title: String,
    pub codes: Vec<KseCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KseCode {
    pub code: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkedData {
    pub title: String,
    pub entries: Vec<OkedEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkedEntry {
    pub code: String,
    pub name: String,
}

// ===== Hadith =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HadithData {
    pub categories: Vec<HadithCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HadithCategory {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub hadiths: Vec<HadithItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HadithItem {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub hadeeth: String,
    #[serde(default)]
    pub hadeeth_ar: String,
    #[serde(default)]
    pub attribution: String,
    #[serde(default)]
    pub grade: String,
    #[serde(default)]
    pub explanation: String,
    #[serde(default)]
    pub hints: Vec<String>,
}

// ===== Bailiffs =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BailiffData {
    #[serde(default)]
    pub source: String,
    pub regions: Vec<BailiffRegion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BailiffRegion {
    #[serde(default)]
    pub key_id: u32,
    pub slug: String,
    pub name: String,
    pub bailiffs: Vec<Bailiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bailiff {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub license_number: String,
    #[serde(default)]
    pub license_date: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub phone: String,
}
