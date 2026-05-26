use adapto_store::AdaptoStore;
use serde_json::Value;
use std::path::Path;

const MYQAZ_DATA: &str = "/Users/sakentukenov/myqaz/mining";

pub fn import_all(store: &AdaptoStore) {
    import_json_file(store, "financial", &format!("{MYQAZ_DATA}/financial-indicators/output/indicators.json"));
    import_json_file(store, "measurement", &format!("{MYQAZ_DATA}/measurement-units/output/measurement-units.json"));
    import_json_file(store, "notaries", &format!("{MYQAZ_DATA}/notaries/output/notaries.json"));
    import_json_file(store, "bailiffs", &format!("{MYQAZ_DATA}/bailiffs/output/bailiffs.json"));
    import_json_file(store, "namaz", &format!("{MYQAZ_DATA}/namaz/output/namaz.json"));
    import_json_file(store, "quran", &format!("{MYQAZ_DATA}/quran/output/quran.json"));
    import_json_file(store, "bible", &format!("{MYQAZ_DATA}/bible/output/bible.json"));
    import_json_file(store, "hadith", &format!("{MYQAZ_DATA}/hadith/output/hadiths.json"));
    import_json_file(store, "phone_codes", &format!("{MYQAZ_DATA}/phone-codes/output/phone-codes.json"));
    import_json_file(store, "postal_codes", &format!("{MYQAZ_DATA}/postal-codes/output/postal-codes.json"));
    import_json_file(store, "waste_codes", &format!("{MYQAZ_DATA}/waste-codes/output/waste-codes.json"));
    import_json_file(store, "tax_rates", &format!("{MYQAZ_DATA}/tax-rates/output/tax-rates.json"));
    import_json_file(store, "shezhire", &format!("{MYQAZ_DATA}/shezhire/output/shezhire.json"));

    import_codes(store);
    import_laws(store);

    import_payment_codes(store);

    import_json_file(store, "constitution", &format!("{MYQAZ_DATA}/constitution/output/constitution.json"));
    import_json_file(store, "gov_orgs", &format!("{MYQAZ_DATA}/gov-kz/output/all_orgs.json"));
    import_json_file(store, "decrees", &format!("{MYQAZ_DATA}/decrees/output/clean_index.json"));
    import_json_file(store, "drugs", &format!("{MYQAZ_DATA}/ndda/output/registry_ls_active.json"));

    import_companies(store);

    eprintln!("  Data import complete.");
}

fn import_json_file(store: &AdaptoStore, collection: &str, path: &str) {
    if !Path::new(path).exists() {
        eprintln!("  SKIP {collection}: {path} not found");
        return;
    }
    let col = store.collection(collection);
    if col.count_all() > 0 {
        return;
    }
    let raw = std::fs::read_to_string(path).unwrap();
    let val: Value = serde_json::from_str(&raw).unwrap();
    col.insert(val).unwrap();
    eprintln!("  Imported {collection}");
}

fn import_codes(store: &AdaptoStore) {
    let col = store.collection("codes");
    if col.count_all() > 0 {
        return;
    }
    let codes_dir = format!("{MYQAZ_DATA}/codes/output");
    let dir = match std::fs::read_dir(&codes_dir) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("  SKIP codes: {codes_dir} not found");
            return;
        }
    };
    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).unwrap();
        let val: Value = serde_json::from_str(&raw).unwrap();
        col.insert(val).unwrap();
    }
    eprintln!("  Imported codes ({} documents)", col.count_all());
}

fn import_laws(store: &AdaptoStore) {
    let col = store.collection("laws");
    if col.count_all() > 0 {
        return;
    }
    let laws_dir = format!("{MYQAZ_DATA}/laws/output/laws");
    let dir = match std::fs::read_dir(&laws_dir) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("  SKIP laws: {laws_dir} not found");
            return;
        }
    };
    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).unwrap();
        let val: Value = serde_json::from_str(&raw).unwrap();
        col.insert(val).unwrap();
    }
    eprintln!("  Imported laws ({} documents)", col.count_all());

    let classified_path = format!("{MYQAZ_DATA}/laws/output/laws_classified.json");
    if Path::new(&classified_path).exists() {
        let raw = std::fs::read_to_string(&classified_path).unwrap();
        let val: Value = serde_json::from_str(&raw).unwrap();
        store.collection("laws_classified").insert(val).unwrap();
        eprintln!("  Imported laws_classified");
    }
}

fn import_companies(store: &AdaptoStore) {
    use std::io::{BufRead, BufReader};
    let col = store.collection("companies");
    if col.count_all() > 0 {
        return;
    }
    let path = format!("{MYQAZ_DATA}/egov-ul/output/gbd_ul.jsonl");
    if !Path::new(&path).exists() {
        eprintln!("  SKIP companies: {path} not found");
        return;
    }
    let file = std::fs::File::open(&path).unwrap();
    let reader = BufReader::new(file);
    let mut records: Vec<Value> = Vec::new();
    for line in reader.lines().take(50_000) {
        let line = line.unwrap();
        if line.trim().is_empty() { continue; }
        match serde_json::from_str::<Value>(&line) {
            Ok(val) => records.push(val),
            Err(e) => { if records.len() < 3 { eprintln!("  companies parse error: {e}"); } }
        }
    }
    let count = records.len();
    col.insert(Value::Array(records)).unwrap();
    eprintln!("  Imported companies ({count} of 977K records)");
}

fn import_payment_codes(store: &AdaptoStore) {
    let col = store.collection("payment_codes");
    if col.count_all() > 0 {
        return;
    }
    let path = format!("{MYQAZ_DATA}/payment-codes/output/classifiers_full.json");
    if Path::new(&path).exists() {
        let raw = std::fs::read_to_string(&path).unwrap();
        let val: Value = serde_json::from_str(&raw).unwrap();
        col.insert(val).unwrap();
        eprintln!("  Imported payment_codes");
    }
}
