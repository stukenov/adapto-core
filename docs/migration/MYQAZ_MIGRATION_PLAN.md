# myqaz.kz -> Adapto Migration Plan

## Current State

| Aspect | myqaz.kz (now) | Hybrid | Full Dynamic |
|--------|----------------|--------|-------------|
| Data store | JSON files in `mining/*/output/` | `adapto_store` | `adapto_store` |
| Admin UI | None (edit JSON manually) | Auto CMS | Auto CMS |
| API | None | REST auto-generated | REST auto-generated |
| Frontend | Static HTML (generators) | SSR + static export | **adapto_ssr** (live) |
| Bot access | Read JSON files | REST API | REST API |
| Search | None | Store indexes | **Live search via WebSocket** |
| Deployment | rsync → Apache | Adapto server + Apache | **Adapto server only** |
| SEO | Manual Schema.org | Auto from metadata | Auto from metadata |
| Interactivity | None | None (static) | **WebSocket live updates** |
| Content update | Re-generate + rsync | Re-export | **Instant (edit → live)** |

---

## Architecture Options

### Option A: Hybrid Model (CMS + Static Export)

Safe, conservative. Admin edits content, static exporter produces HTML, Apache serves.

```
Adapto Server (port 3001, internal)
    ├── Admin CMS (WebSocket CRUD)
    ├── REST API (bot, miners)
    └── Static Exporter → site/ → Apache (public)
```

### Option B: Full Dynamic (RECOMMENDED)

Adapto serves all pages live. No static export. No Apache.

```
┌─────────────────────────────────────────────────────────┐
│                 Adapto Server (port 443)                  │
│                                                          │
│  ┌─────────────────────────────────────────────────┐     │
│  │              adapto_ssr (Renderer)               │     │
│  │  SSR initial paint + data-ar-dyn hydration      │     │
│  │  Bootstrap payload → zero-roundtrip WebSocket   │     │
│  └────────────────────┬────────────────────────────┘     │
│                       │                                   │
│  ┌────────────────────┴────────────────────────────┐     │
│  │              adapto_live (Sessions)              │     │
│  │  WebSocket: PatchOp DOM mutations               │     │
│  │  Live search, navigation, interactions          │     │
│  │  StateStore + DependencyGraph per session        │     │
│  └────────────────────┬────────────────────────────┘     │
│                       │                                   │
│  ┌──────────┐  ┌──────┴─────┐  ┌──────────────────┐     │
│  │ Admin UI │  │ REST API   │  │ Response Cache    │     │
│  │ (.crud)  │  │ (bot, API) │  │ (HTML fragments)  │     │
│  └────┬─────┘  └──────┬─────┘  └────────┬─────────┘     │
│       └───────────────┴─────────────────┘                │
│                       │                                   │
│              ┌────────┴────────┐                         │
│              │  adapto_store   │                          │
│              │  356ns find_by_id                          │
│              │  1.26µs indexed query                      │
│              └─────────────────┘                         │
└─────────────────────────────────────────────────────────┘
```

#### Why Full Dynamic Works for myqaz.kz

**Performance comparison vs current Rust implementation:**

Current myqaz-rs server (`crates/server/`) already does partial dynamic rendering with LRU cache + LazyData JSON loading. Adapto replaces that pipeline.

| Step | Current Rust server | Adapto SSR |
|------|---------------------|------------|
| Data read | `fs::read_to_string()` ~100-500µs | `adapto_store` indexed query: **1.26µs** |
| Deserialization | `serde_json::from_str()` ~50-200µs | Already in memory: **0** |
| HTML building | `format!()` + `.push_str()` ~10-50µs | Same pattern: ~10-50µs |
| **Cache miss total** | **~160-750µs** | **~12-52µs** (5-20x faster) |
| Cache hit | ~1µs (DashMap LRU) | ~1µs (HTTP ETag/CDN) |
| Cold start | Lazy per-file (~100-500µs each) | 87K docs bulk load: **~270ms once** |
| Memory | Lazy + LRU (~50-200MB fluctuating) | All in memory: ~50-100MB (stable) |
| Parallelism | Rayon (namaz only), sync I/O | tokio async per-request |
| Brotli | Pre-compressed `.br` files on disk | Middleware: compress on-the-fly (cached) |

**Why Adapto wins on cache miss**: no file I/O, no deserialization. Data already indexed in B-tree. `format!()` HTML building = same speed both systems.

**Current server already proves dynamic works for myqaz.kz** — financial indicators, phone codes, notaries, measurement units all served dynamically. Adapto extends this to ALL content.

At 100 concurrent users (myqaz.kz peak): 100 × 52µs = 5.2ms total. Trivial.
At 1,000 concurrent: adapto_store concurrent reads = 1.48M ops/sec. No bottleneck.

**Network latency (50-200ms Kazakhstan) dwarfs server render time (< 0.1ms).**

#### Full Dynamic Advantages

1. **Zero deploy lag** — edit content in admin → visible on site instantly
2. **Live search** — WebSocket-driven, no page reload, debounced
3. **No build step** — no `myqaz generate`, no rsync, no `docker restart`
4. **Simpler infrastructure** — one process, no Apache, no Caddy
5. **Interactive features** — bookmark articles, compare laws, prayer time alerts
6. **Real-time analytics** — track page views in adapto_store
7. **Content preview** — admin sees exactly what public sees before publishing

#### SEO with Full Dynamic

Adapto SSR already handles this — `adapto_ssr::Renderer` produces **complete HTML on first request**:

```
Browser: GET /law/laws/education/chapter-1/article-5/
    │
    ▼
adapto_ssr::Renderer::render_page()
    │
    ├── Query: Article where law_slug="education", chapter_slug="chapter-1", slug="article-5"
    │   └── adapto_store indexed lookup: 1.26 µs
    │
    ├── Render full HTML with:
    │   ├── <title>Статья 5. Образование - Закон РК</title>
    │   ├── <meta name="description" content="...">
    │   ├── <link rel="canonical" href="https://myqaz.kz/law/laws/education/chapter-1/article-5/">
    │   ├── Open Graph tags
    │   ├── Schema.org/Legislation JSON-LD
    │   ├── Breadcrumb structured data
    │   └── Full article content (no JS required to read)
    │
    ├── Inject Bootstrap payload (JSON in <script>)
    │   └── WebSocket URL, session ID, CSRF token, dynamic targets
    │
    └── Return complete HTML (Googlebot sees everything)

Browser receives HTML:
    ├── Content visible immediately (SSR)
    ├── WebSocket connects (hydration)
    └── Interactive features activate (search, nav, bookmarks)
```

**Googlebot gets 100% content on first request. No JS execution needed.** Same as static files.

#### Response Cache Layer

For high-traffic pages (MRP calculator, popular laws), add HTTP cache:

```rust
App::new("myqaz.kz")
    .cache_strategy(CacheStrategy::Stale {
        max_age: Duration::from_secs(3600),      // 1 hour fresh
        stale_while_revalidate: Duration::from_secs(86400), // serve stale up to 24h
    })
    // Or per-resource:
    .crud::<Law>().cache(Duration::from_secs(86400))      // laws rarely change
    .crud::<NamazTime>().cache(Duration::from_secs(300))  // prayer times: 5 min
    .crud::<FinancialIndicator>().no_cache()               // always fresh
```

Plus `ETag` / `Last-Modified` headers auto-generated from adapto_store document timestamps. CDN (Cloudflare) caches at edge — same performance as static files, globally.

---

## How Adapto's Existing Code Maps to myqaz.kz

### adapto_ssr (already built)

`crates/adapto_ssr/src/renderer.rs`:
- `render_page()` — full HTML document with layout, bootstrap, client JS
- `render_component()` — wraps in `<div data-ar-root>` for hydration
- `render_segments()` — interleaves static HTML with `<span data-ar-dyn>` dynamic zones
- CSRF token signing via `adapto_auth::csrf`
- Bootstrap payload: session ID, WS URL, component tree metadata

**For myqaz.kz**: Each URL pattern → SSR page. Article page renders full legal text server-side. Dynamic zones only for interactive parts (search, bookmarks, navigation).

### adapto_live (already built)

`crates/adapto_live/src/session.rs`:
- `LiveSession` — per-connection state: user, tenant, route, permissions, StateStore
- `handle_event()` → dispatches to registered ActionHandlers
- `generate_patches()` — dirty tracking + dependency graph → minimal PatchOps

**For myqaz.kz**: Search = `handle_event("search")` → query store → `PatchOp::ReplaceHtml` on results container. Navigation = `handle_event("navigate")` → render new page content → replace `#app-content`.

### adapto_client_protocol (already built)

`crates/adapto_client_protocol/src/patch.rs`:
- 15 PatchOp variants: ReplaceText, ReplaceHtml, SetAttr, AddClass, InsertBefore, RemoveNode, Focus, ScrollTo, Redirect, Flash, ModalOpen, ModalClose...

**For myqaz.kz**: 
- `ReplaceHtml` — search results, page content on navigate
- `ReplaceText` — prayer time updates, financial indicator values
- `Flash` — "Bookmarked!" notifications
- `Redirect` — after form submissions
- `ModalOpen` — article quick-view popup

### adapto_app live.js (already built)

`crates/adapto_app/src/live.js`:
- WebSocket auto-reconnect
- `replace_html` / `replace_text` patch handlers
- Delegated `data-action` click dispatch
- `data-route` pushState navigation
- Debounced `data-field="search"` input

**For myqaz.kz**: Drop-in. Search box, article navigation, breadcrumb updates — all work via existing live.js without changes.

---

## Resource Modeling

### Phase 1: Core Legal Resources (~10 structs)

```rust
#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "laws")]
pub struct Law {
    #[field(required, max_length = 500)]
    pub title: String,

    #[field(required, unique)]
    pub slug: String,

    #[field(required)]
    pub doc_id: String,

    pub preamble: Vec<String>,

    #[field(default = "active", one_of = ["active", "archived", "draft"])]
    pub status: String,

    pub source_url: String,
    pub effective_date: String,
    pub category: String,
}

#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "chapters")]
pub struct Chapter {
    #[field(required)]
    pub law_slug: String,  // belongs_to Law

    #[field(required)]
    pub number: String,

    #[field(required)]
    pub slug: String,

    #[field(required)]
    pub title: String,

    pub sort_order: i32,
}

#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "articles")]
pub struct Article {
    #[field(required)]
    pub law_slug: String,

    #[field(required)]
    pub chapter_slug: String,

    #[field(required)]
    pub number: String,

    #[field(required)]
    pub slug: String,

    #[field(required)]
    pub title: String,

    pub sort_order: i32,
}

#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "points")]
pub struct Point {
    #[field(required)]
    pub article_slug: String,

    pub number: String,
    pub slug: String,
    pub text: String,
    pub subpoints: Vec<String>,  // stored as JSON array
    pub sort_order: i32,
}
```

### Phase 2: Reference Resources (~8 structs)

```rust
#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "notaries")]
pub struct Notary {
    #[field(required)]
    pub name: String,

    #[field(required)]
    pub slug: String,

    pub region_slug: String,
    pub license_number: String,
    pub license_date: String,
    pub address: String,
    pub phone: String,
    pub email: String,
}

#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "financial_indicators")]
pub struct FinancialIndicator {
    #[field(required)]
    pub indicator_type: String,  // mrp, mzp, pm

    #[field(required)]
    pub year: i32,

    pub value: f64,
    pub effective_date: String,
    pub source_url: String,
}

#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "government_orgs")]
pub struct GovernmentOrg {
    #[field(required)]
    pub name: String,

    #[field(required, unique)]
    pub slug: String,

    pub org_type: String,  // ministry, agency, akimat, central
    pub head_name: String,
    pub head_title: String,
    pub photo_url: String,
    pub website: String,
    pub address: String,
    pub vanity_url: String,  // e.g., "/min-health/"
}

// Similarly: Bailiff, TaxRate, PaymentCode, PhoneCode, PostalCode, Drug
```

### Phase 3: Religious & Specialized (~5 structs)

```rust
#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "namaz_cities")]
pub struct NamazCity {
    #[field(required, unique)]
    pub slug: String,

    pub name: String,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "namaz_times")]
pub struct NamazTime {
    #[field(required)]
    pub city_slug: String,

    pub date: String,  // 2026-01-15
    pub fajr: String,
    pub sunrise: String,
    pub dhuhr: String,
    pub asr: String,
    pub maghrib: String,
    pub isha: String,
}

// Similarly: QuranSurah, Hadith, BibleBook
```

---

## Migration Phases

### Phase 0: Importers (1-2 days)
**Goal**: All existing JSON data → adapto_store.

```rust
// src/import/mod.rs
pub mod laws;
pub mod notaries;
pub mod namaz;
pub mod financial;
pub mod government;
// ... one module per data source

// Pattern for each importer:
pub fn import_laws(store: &AdaptoStore) {
    let json: Vec<LawJson> = serde_json::from_str(
        &std::fs::read_to_string("mining/laws/output/laws.json").unwrap()
    ).unwrap();

    let col = store.collection("laws");
    for law in json {
        let resource = Law::from_legacy(law);  // map old shape → new shape
        resource.insert_into(store).unwrap();
    }
}
```

**Data volumes** (for performance planning):
| Collection | Count | Notes |
|-----------|-------|-------|
| laws | ~284 | + 21 codes + constitution |
| chapters | ~3,000 | nested under laws |
| articles | ~15,000 | nested under chapters |
| points | ~50,000 | nested under articles |
| notaries | ~1,000 | by region |
| bailiffs | ~500 | by region |
| government_orgs | ~357 | + ~600 staff |
| financial_indicators | ~50 | historical values |
| namaz_cities | ~20 | Kazakhstan cities |
| namaz_times | ~7,300 | 20 cities x 365 days |
| drugs | ~5,000 | pharmaceutical registry |
| companies | ~5,000 | legal entities |

Total: ~87K documents. At 3.1 µs/insert = ~270ms total import. Trivial.

### Phase 1: Admin CMS (2-3 days)
**Goal**: Edit all content through browser instead of JSON files.

```rust
#[tokio::main]
async fn main() {
    App::new("myqaz.kz Admin")
        .port(3001)
        .store_path("./data/myqaz")

        // Legal
        .crud::<Law>()
        .crud::<Chapter>()
        .crud::<Article>()
        .crud::<Point>()

        // Reference
        .crud::<Notary>()
        .crud::<GovernmentOrg>()
        .crud::<FinancialIndicator>()
        .crud::<TaxRate>()

        // Religious
        .crud::<NamazCity>()
        .crud::<NamazTime>()

        .run()
        .await
        .unwrap();
}
```

This gives: list/detail/create/edit/delete views for every resource type. Auto-generated from field metadata. Immediate productivity gain — no more editing raw JSON.

### Phase 2: Public Page Rendering (2-3 days)
**Goal**: Replace 28 Rust generators with Adapto SSR route handlers.

```rust
// Full dynamic: each URL pattern → axum route → SSR render
App::new("myqaz.kz")
    .store_path("./data/myqaz")

    // Legal content routes (preserve exact URL structure)
    .get_route("/law/laws/", |ctx| render_laws_index(ctx))
    .get_route("/law/laws/:slug/", |ctx| render_law_detail(ctx))
    .get_route("/law/laws/:slug/:chapter/", |ctx| render_chapter(ctx))
    .get_route("/law/laws/:slug/:chapter/:article/", |ctx| render_article(ctx))
    .get_route("/law/codes/:slug/", |ctx| render_code(ctx))
    .get_route("/law/constitution/", |ctx| render_constitution(ctx))

    // Reference
    .get_route("/reference/mrp/", |ctx| render_mrp(ctx))
    .get_route("/reference/tax-rates/", |ctx| render_tax_rates(ctx))

    // Directories
    .get_route("/directory/notaries/", |ctx| render_notaries_index(ctx))
    .get_route("/directory/notaries/:region/", |ctx| render_notaries_region(ctx))
    .get_route("/directory/notaries/:region/:slug/", |ctx| render_notary_detail(ctx))

    // Religious
    .get_route("/namaz/", |ctx| render_namaz_index(ctx))
    .get_route("/namaz/:city/", |ctx| render_namaz_city(ctx))

    // Vanity URLs (government orgs)
    .get_route("/min-:slug/", |ctx| render_gov_org(ctx, "ministry"))
    .get_route("/ag-:slug/", |ctx| render_gov_org(ctx, "agency"))
```

Each handler: query store → render HTML → return with SEO meta. Example:

```rust
fn render_article(ctx: &ActionContext) -> String {
    let slug = ctx.param("article");
    let (_, article) = Article::find_one(&ctx.store, Query::eq("slug", slug)).unwrap();
    let (_, chapter) = Chapter::find_one(&ctx.store, Query::eq("slug", &article.chapter_slug)).unwrap();
    let (_, law) = Law::find_one(&ctx.store, Query::eq("slug", &article.law_slug)).unwrap();

    let points = Point::find_all(&ctx.store,
        Query::eq("article_slug", slug).sort("sort_order", SortDir::Asc));

    html! {
        head {
            title { (format!("Статья {}. {} - {}", article.number, article.title, law.title)) }
            meta name="description" content=(article.summary());
            link rel="canonical" href=(format!("https://myqaz.kz/law/laws/{}/{}/{}/",
                law.slug, chapter.slug, article.slug));
            // OG tags, Schema.org/Legislation JSON-LD auto-injected
        }
        body {
            // Breadcrumb
            nav.breadcrumb {
                a href="/" { "Главная" }
                " → "
                a href="/law/laws/" { "Законы" }
                " → "
                a href=(format!("/law/laws/{}/", law.slug)) { (law.title) }
                " → "
                span { (format!("Статья {}", article.number)) }
            }

            h1 { (format!("Статья {}. {}", article.number, article.title)) }

            @for point in &points {
                div.point {
                    span.point-number { (point.number) "." }
                    " " (point.text)
                    @for sp in &point.subpoints {
                        div.subpoint { (sp) }
                    }
                }
            }

            // Prev/Next navigation
            nav.article-nav data-ar-dyn="article-nav" {
                // Rendered with adjacent article links
            }
        }
    }
}
```

**28 generators → ~15 render functions** (many generators shared similar patterns).

### Phase 3: REST API for Bot (1 day)
**Goal**: Telegram bot queries Adapto API instead of reading JSON files.

```rust
App::new("myqaz.kz")
    .crud::<Law>()       // auto-creates GET /api/laws, GET /api/laws/:id, etc.
    .crud::<Article>()
    .crud::<Notary>()
    // ...
```

Bot changes (Python):
```python
# Before:
with open("mining/laws/output/laws.json") as f:
    laws = json.load(f)

# After:
async def get_law(slug: str):
    resp = await httpx.get(f"http://localhost:3001/api/laws?slug={slug}")
    return resp.json()["data"]
```

### Phase 4: Mining Integration (1 day)
**Goal**: Mining scripts write directly to adapto_store instead of JSON files.

```python
# Python miners → HTTP POST to Adapto API
async def save_notary(notary_data):
    await httpx.post("http://localhost:3001/api/notaries", json=notary_data)
```

Or for Rust miners:
```rust
// Direct store access (same process)
let notary = Notary { ... };
notary.insert_into(&store).unwrap();
```

### Phase 5: SEO Automation (1 day)
**Goal**: Auto-generate sitemap, Schema.org, OG tags from Resource metadata.

```rust
#[derive(Resource, Serialize, Deserialize)]
#[resource(collection = "laws", seo = true)]
pub struct Law {
    #[field(seo_title)]
    pub title: String,

    #[field(seo_slug)]
    pub slug: String,

    #[field(seo_description)]
    pub summary: String,

    // ...
}
```

Auto-generates:
- `GET /sitemap.xml` — dynamic sitemap from all resources with `seo = true`
- `<title>`, `<meta description>`, `<link rel="canonical">` injected by SSR renderer
- Schema.org/Legislation JSON-LD for legal content
- Open Graph tags
- Breadcrumb structured data

Dynamic sitemap advantage: always current. No `myqaz sitemap` build step.

### Phase 6: Live Interactive Features (1-2 days)
**Goal**: Features impossible with static site.

```rust
App::new("myqaz.kz")
    // ...

    // Live search across all legal content
    .on("search", |ctx| {
        let q = ctx.payload["q"].as_str().unwrap_or("");
        let results = Article::search(&ctx.store, q, 20); // full-text search
        ActionResult::replace_html("#search-results", render_search_results(&results))
    })

    // Article bookmarks (per-session or per-user)
    .on("bookmark", |ctx| {
        let article_id = ctx.payload["id"].as_str().unwrap();
        ctx.session.entry("bookmarks").or_insert(vec![]).push(article_id);
        ActionResult::with_ops(vec![
            PatchOp::Flash { level: FlashLevel::Success, message: "Сохранено".into() },
            PatchOp::AddClass { target: format!("#bookmark-{}", article_id), class: "bookmarked".into() },
        ])
    })

    // Compare two law articles side-by-side
    .on("compare", |ctx| {
        let a = Article::find_one(&ctx.store, Query::eq("slug", ctx.payload["a"])).unwrap();
        let b = Article::find_one(&ctx.store, Query::eq("slug", ctx.payload["b"])).unwrap();
        ActionResult::replace_html("#app-content", render_comparison(&a, &b))
    })

    // Prayer time notifications
    .on("set_namaz_alert", |ctx| {
        let city = ctx.payload["city"].as_str().unwrap();
        let prayer = ctx.payload["prayer"].as_str().unwrap();
        ctx.session.insert("namaz_alert", json!({"city": city, "prayer": prayer}));
        ActionResult::replace_html("#alert-status", "Уведомление установлено")
    })
```

### Phase 7: Scheduled Tasks (0.5 days)
**Goal**: Replace systemd timers with Adapto-managed tasks.

```rust
App::new("myqaz.kz")
    // ...
    .schedule("namaz-daily", "0 1 * * *", |ctx| {
        namaz::update_daily(&ctx.store); // calculate + write to store
        // No export needed — next request reads fresh data
    })
    .schedule("indexnow", "0 */6 * * *", |ctx| {
        seo::indexnow_changed(&ctx.store); // notify search engines
    })
    .schedule("mining-sync", "0 3 * * *", |ctx| {
        miners::sync_all(&ctx.store); // pull fresh data from external sources
    })
```

---

## URL Mapping (Critical for SEO)

**Every existing URL must be preserved.**

| Current URL Pattern | Resource | URL Template |
|--------------------|----------|-------------|
| `/law/laws/{slug}/` | Law | `/law/laws/{slug}/` |
| `/law/laws/{slug}/{chapter}/` | Chapter | `/law/laws/{law_slug}/{slug}/` |
| `/law/laws/{slug}/{chapter}/{article}/` | Article | keep nested |
| `/law/codes/{slug}/` | Law (type=code) | same |
| `/law/constitution/` | Law (type=constitution) | same |
| `/directory/notaries/{region}/` | Notary (grouped) | same |
| `/directory/notaries/{region}/{slug}/` | Notary | same |
| `/directory/bailiffs/{region}/` | Bailiff (grouped) | same |
| `/reference/mrp/` | FinancialIndicator | same |
| `/reference/tax-rates/` | TaxRate | same |
| `/namaz/` | NamazCity (list) | same |
| `/namaz/{city}/` | NamazTime (grouped) | same |
| `/min-health/`, `/ag-finmon/` | GovernmentOrg | vanity URLs preserved |
| `/almaty/`, `/astana/` | GovernmentOrg (akimats) | vanity URLs preserved |

---

## What Stays the Same

1. **URL structure** — zero changes to public URLs
2. **HTML output** — same minimal design, same SEO markup (Googlebot sees identical content)
3. **Mining scripts** — still scrape same sources, output to API instead of JSON
4. **Telegram bot** — same features, new data source (API instead of files)

## What Changes

| Before | After |
|--------|-------|
| Apache + Caddy | Adapto server (single binary) |
| rsync deploy | Binary deploy (or Docker) |
| JSON files (scattered) | adapto_store (single embedded DB) |
| 28 Rust generators | ~15 SSR render functions |
| No admin UI | Full CMS via `.crud::<T>()` |
| No search | Live WebSocket search |
| No API | REST auto-generated |
| Edit JSON → re-generate → rsync | Edit in CMS → instantly live |
| systemd timers | `.schedule()` built-in |
| No interactivity | Bookmarks, compare, alerts |
| Content history: git blame | adapto_audit trail |

## What Gets Eliminated

```
❌ deploy/docker-compose.yml     — no Apache container
❌ deploy/httpd-custom.conf      — no Apache config
❌ deploy/myqaz-*.service/timer  — replaced by .schedule()
❌ processing/indexnow.py        — built into scheduler
❌ myqaz-rs/crates/processing/   — 28 generators → 15 render functions
❌ site/ directory               — no static files
❌ mining/*/output/*.json         — data lives in adapto_store
❌ Brotli pre-compression step   — Adapto serves with compression middleware
```

---

## Timeline Estimate (Full Dynamic)

| Phase | Effort | Depends On |
|-------|--------|-----------|
| Resource modeling (23 structs) | 1 day | Adapto Phase 1 (Resource macro) |
| Importers (JSON → store) | 1-2 days | Resource modeling |
| Admin CMS | 1 day | Adapto Phase 3 (.crud()) |
| Public page rendering (SSR) | 3-4 days | Resource modeling + adapto_ssr |
| REST API for bot | 0.5 day | Adapto Phase 4 (REST) |
| SEO automation | 1 day | Public page rendering |
| Live features (search, bookmarks) | 1-2 days | adapto_live |
| Scheduled tasks | 0.5 day | Adapto runtime |
| Mining integration | 1 day | REST API |
| Testing + SEO validation | 1-2 days | All phases |
| **Total** | **~11-14 days** | |

**Prerequisite**: Adapto Phases 1-4 implemented (Resource macro, auto-views, .crud(), REST API).

---

## Deployment (Full Dynamic)

### Production Setup

```
                     ┌──────────┐
                     │Cloudflare│  (CDN + SSL + edge cache)
                     └────┬─────┘
                          │
                     ┌────┴─────┐
                     │  Adapto  │  (single binary, port 3000)
                     │  Server  │
                     └────┬─────┘
                          │
                     ┌────┴─────┐
                     │adapto_   │  (./data/myqaz/)
                     │store     │
                     └──────────┘
```

```bash
# Deploy: single binary
scp target/release/myqaz-server deploy@91.224.74.233:/opt/myqaz/
ssh deploy@91.224.74.233 "systemctl restart myqaz"

# Or Docker (one-liner):
docker run -v /opt/myqaz/data:/data -p 3000:3000 myqaz-server
```

### Caching Headers (replace Apache config)

```rust
App::new("myqaz.kz")
    .middleware(adapto::middleware::Cache {
        default: Duration::from_secs(3600),       // 1h default
        rules: vec![
            ("/law/*", Duration::from_secs(86400)),  // laws: 24h (rarely change)
            ("/namaz/*", Duration::from_secs(300)),   // namaz: 5min
            ("/reference/*", Duration::from_secs(3600)), // reference: 1h
        ],
    })
    .middleware(adapto::middleware::Compression::brotli())
    .middleware(adapto::middleware::ETag)  // auto ETag from store timestamps
```

---

## Risk Mitigation

| Risk | Impact | Mitigation |
|------|--------|-----------|
| URL breakage | High (SEO) | Integration tests: crawl all known URLs, verify 200 status |
| HTML output diff | Medium (SEO) | Snapshot comparison: current HTML vs SSR output for top 100 pages |
| Data loss during import | High | Import is additive, keep original JSON as backup |
| SSR performance | Low | Benchmarked: < 1ms per page. 87K docs fit in memory |
| Server downtime | Medium | systemd auto-restart + Cloudflare "always online" |
| Bot downtime | Low | Run both systems in parallel during transition |
| Googlebot rendering | None | SSR = complete HTML on first request, no JS needed |

## Migration Order (Full Dynamic)

```
1. Model 23 Resource structs
2. Build importers, validate data integrity (count, spot-check)
3. Build public page SSR render functions
4. SEO validation: compare HTML output with current site
5. Launch on staging (port 3001), run crawler, verify all URLs
6. Point Cloudflare to Adapto server
7. Enable admin CMS (internal, auth-gated)
8. Switch bot to API
9. Switch miners to API
10. Decommission: Apache, old generators, JSON files
```

**Parallel runway**: Keep Apache serving current static site until Step 6 passes all checks. Zero-downtime cutover via Cloudflare DNS.

**Rollback**: Point Cloudflare back to Apache. Static files still there. Takes 30 seconds.
