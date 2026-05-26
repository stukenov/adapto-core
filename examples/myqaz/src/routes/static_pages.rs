use adapto_store::AdaptoStore;
use std::path::Path;

static STATIC_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/static-pages");

pub fn import_static_pages(store: &AdaptoStore) {
    let col = store.collection("static_pages");
    if col.count_all() > 0 {
        return;
    }
    let base = Path::new(STATIC_DIR);
    if !base.exists() {
        eprintln!("  WARN: static-pages directory not found at {STATIC_DIR}");
        return;
    }
    let mut count = 0usize;
    load_recursive(base, base, store, &mut count);
    eprintln!("  Loaded {count} static pages");
}

fn load_recursive(base: &Path, dir: &Path, store: &AdaptoStore, count: &mut usize) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_recursive(base, &path, store, count);
        } else if path.extension().map_or(false, |e| e == "html") {
            let rel = path.strip_prefix(base).unwrap();
            let mut route = format!("/{}", rel.display());
            route = route.replace("/index.html", "");
            if route.is_empty() {
                route = "/".to_string();
            }
            match std::fs::read_to_string(&path) {
                Ok(html) => {
                    let col = store.collection("static_pages");
                    let doc = serde_json::json!({
                        "path": route,
                        "html": html,
                    });
                    col.insert(doc).unwrap();
                    *count += 1;
                }
                Err(e) => eprintln!("  WARN: failed to read {}: {e}", path.display()),
            }
        }
    }
}

pub fn render_static(store: &AdaptoStore, path: &str) -> String {
    let col = store.collection("static_pages");
    for doc in col.find(adapto_store::Query::new()) {
        if let Some(p) = doc.data.get("path").and_then(|v| v.as_str()) {
            if p == path {
                return doc.data.get("html").and_then(|v| v.as_str()).unwrap_or("").to_string();
            }
        }
    }
    String::new()
}
