# Weather URL Nesting & SEO Expansion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand the myqaz weather section into a deep, fully-indexable URL surface (periods, variable hubs, period×variable combos, hourly, per-hour for next 48h, slash-dates) for all 302 cities in RU+KK, with a redesigned no-emoji tiered index, an L2 menu, full SEO + sharded sitemaps, then deploy to prod and verify end-to-end.

**Architecture:** Every page is a pure view over the existing hourly-refreshed `CityForecast` cache (+ a new `weather_climate` collection for the month tail). New data fields (pressure/clouds/uv/sunrise) are added to the ingest. Multi-segment routes are dispatched by `App::page` handlers (returning `PageResponse::{Ok,Redirect}`) that render through the template engine; single-segment city + index stay as template routes. Renderers live in a new `src/weather/render.rs`; slice helpers in `src/weather/slice.rs`.

**Tech Stack:** Rust, adapto_app (template engine + PageResponse), adapto_store, adapto_scheduler, reqwest (rustls), chrono/chrono-tz, serde.

**Spec:** `docs/superpowers/specs/2026-05-29-weather-url-nesting-design.md`.

**Repos:** Implementation in `~/myqaz/myqaz-rs`. Framework helpers (if needed) in `~/adapto-core/crates/adapto_app`.

---

## File Structure

**myqaz (`~/myqaz/myqaz-rs/`):**
- `src/weather/types.rs` — extend `CurrentWx`/`HourWx`/`DayWx` (modify).
- `src/weather/slice.rs` — NEW: pure view/slice helpers over `CityForecast`.
- `src/weather/render.rs` — NEW: per-page-type HTML builders (extract from `routes/weather.rs`).
- `src/weather/grammar.rs` — NEW: `PERIODS`, `VARIABLES` sets, `Period`/`Variable` parse + labels.
- `src/weather/climate.rs` — NEW: `weather_climate` read/write + day-of-year normals type.
- `src/weather/source.rs` — extend Open-Meteo fields (modify).
- `src/weather/store.rs` — add climate collection constant + `ensure_indexes` (modify).
- `src/routes/weather.rs` — thin loaders/dispatchers calling `render.rs` (modify).
- `src/jobs/weather_ingest.rs` — populate new fields (modify).
- `src/jobs/weather_climate.rs` — NEW: manual ERA5 normals backfill job.
- `src/jobs/mod.rs` — export new job (modify).
- `src/views/nav.rs` — add `NavSection::Weather` L2 menu (modify).
- `src/routes/sitemap.rs` — weather sitemap sharding (modify).
- `src/main.rs` — register weather `App::page` dispatchers + climate job (modify).
- `templates/pages/weather/*.adapto` — keep index + city; new templates only if a page type renders via loader (modify/create).
- `static/…/weather.css` — design (create/modify; confirm static path).
- `tests/weather_*.rs` — slice, grammar, routing, climate, index, seo (create).

**adapto-core (`~/adapto-core/`):** only if Phase 3 spike shows a render/redirect helper is missing — add to `crates/adapto_app`.

---

## Phase 0 — Baseline & guardrails

### Task 0.1: Capture green baseline

- [ ] **Step 1:** Run the existing suite, record pass count.

Run: `cd ~/myqaz/myqaz-rs && cargo test 2>&1 | tail -20`
Expected: builds; note the passing test count (baseline).

- [ ] **Step 2:** Confirm dev run boots and serves an existing weather page.

Run: `cd ~/myqaz/myqaz-rs && (MYQAZ_TEMPLATES=./templates cargo run &) ; sleep 8 ; curl -s -o /dev/null -w '%{http_code}\n' localhost:8082/weather ; curl -s -o /dev/null -w '%{http_code}\n' localhost:8082/weather/astana ; pkill -f 'target/debug/myqaz' || true`
Expected: `200` and `200`.

- [ ] **Step 3:** Diagnose the "not all cities" issue (data, not layout).

Run: add a one-off `eprintln!("cities={} cached={}", load_cities(&store).len(), store::all().map(|m| m.len()).unwrap_or(0));` after warmup in `main.rs`, `cargo run`, read the line, then revert the print.
Expected: record both numbers. If `cities < 302` → root cause is `load_cities`/namaz import (note for Task 5.3). If `cached < cities` → ingest coverage (note for Task 4.x).

---

## Phase 1 — Data model + ingest + climate

### Task 1.1: Extend forecast types

**Files:** Modify `src/weather/types.rs`.

- [ ] **Step 1: Write failing test** (append to `types.rs` `#[cfg(test)]`):

```rust
#[test]
fn new_fields_default_and_roundtrip() {
    let h = HourWx { t: "2026-05-29T10:00".into(), temp: 1.0, feels: 0.0, humidity: 50.0,
        wind: 2.0, precip: 0.0, code: 1, is_estimate: true, pressure: 1012.0, clouds: 40.0, uv: 3.0 };
    let s = serde_json::to_string(&h).unwrap();
    let back: HourWx = serde_json::from_str(&s).unwrap();
    assert_eq!(back.pressure, 1012.0);
    // old docs without the new fields still parse (serde default)
    let old: HourWx = serde_json::from_str(r#"{"t":"x","temp":1,"feels":0,"humidity":5,"wind":2,"precip":0,"code":1,"is_estimate":false}"#).unwrap();
    assert_eq!(old.pressure, 0.0);
}
```

- [ ] **Step 2: Run, verify fail.** `cargo test -p myqaz new_fields_default` → FAIL (missing fields).

- [ ] **Step 3: Implement.** Add fields with `#[serde(default)]`:
  - `CurrentWx` += `#[serde(default)] pub feels: f64, pressure: f64, clouds: f64, uv: f64` (one attr per field as needed).
  - `HourWx` += `#[serde(default)] pub pressure: f64`, `clouds: f64`, `uv: f64`.
  - `DayWx` += `#[serde(default)] pub sunrise: String, sunset: String, uv_max: f64, pressure: f64, clouds: f64, humidity: f64`.

- [ ] **Step 4: Run.** `cargo test -p myqaz weather::types` → PASS. Fix all construction sites (`store.rs` test `sample()`, ingest) to include new fields or use `..Default::default()` — derive `Default` on the structs if not present.

- [ ] **Step 5: Commit.** `git commit -am "feat(weather): extend Hour/Day/Current with pressure, clouds, uv, sunrise"`

### Task 1.2: Open-Meteo source — fetch new fields

**Files:** Modify `src/weather/source.rs`.

- [ ] **Step 1: Failing test** — extend the fixture-parse test to assert `pressure`/`clouds`/`uv` and daily `sunrise`/`sunset` parse. Add a tiny fixture with those keys.

- [ ] **Step 2: Run, fail.**

- [ ] **Step 3: Implement.** Extend the const query strings:
  - `HOURLY` += `,surface_pressure,uv_index` (`cloud_cover` already present).
  - `DAILY` += `,sunrise,sunset,uv_index_max`.
  - `CURRENT` += `,surface_pressure,cloud_cover,uv_index`.
  Extend `HourPoint`/`DayPoint`/`ObsPoint` structs + the parse fn to read the new suffixed arrays. `sunrise`/`sunset` are strings; pressure/clouds/uv are best-model raw (read from the first model that has them).

- [ ] **Step 4: Run.** `cargo test -p myqaz weather::source` → PASS.
- [ ] **Step 5:** **Verification:** one real call — `python3 -c "import urllib.request,json; ..."` against the forecast URL for Almaty incl. the new params; confirm 200 + keys present. If a param 400s, drop it and note in spec.
- [ ] **Step 6: Commit.** `git commit -am "feat(weather): fetch surface_pressure, uv_index, sunrise/sunset from Open-Meteo"`

### Task 1.3: Climate normals collection + type

**Files:** Create `src/weather/climate.rs`; Modify `src/weather/store.rs`, `src/weather/mod.rs`.

- [ ] **Step 1: Failing test** (`climate.rs` tests): build a `CityClimate` with 366 day-of-year entries, save+read roundtrip via in-memory store; `normal(doy)` returns the entry; mean computation from synthetic rows.

```rust
// normal_for_doy_returns_mean
let rows = vec![(150u32, 20.0, 8.0, 0.0, 4.0, 40.0), (150, 22.0, 10.0, 2.0, 6.0, 50.0)];
let c = CityClimate::from_daily("almaty", &rows); // averages by doy
let n = c.normal(150).unwrap();
assert!((n.tmax - 21.0).abs() < 1e-9);
```

- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement.** `CityClimate { slug, by_doy: HashMap<u32, ClimNormal> }`, `ClimNormal { tmax, tmin, precip, wind, humidity }`, `from_daily(slug, &[(doy,tmax,tmin,precip,wind,humidity)])` averaging by doy, `normal(doy)`, `to_json/from_json`. Store: add `pub const CLIMATE: &str = "weather_climate";`, index in `ensure_indexes`, `read_climate(store, slug) -> Option<CityClimate>`, `save_climate`.
- [ ] **Step 4: Run.** `cargo test -p myqaz weather::climate` → PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(weather): weather_climate collection + day-of-year normals"`

### Task 1.4: Ingest populates new fields

**Files:** Modify `src/jobs/weather_ingest.rs`.

- [ ] **Step 1: Failing test** — extend `assemble_forecast` test: synthetic fetch with pressure/clouds/uv/sunrise → assert produced `HourWx.pressure`, `DayWx.sunrise` non-empty.
- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement.** In `assemble_forecast`: set hourly `pressure/clouds/uv` from best raw model; daily `sunrise/sunset/uv_max/pressure/clouds/humidity`; current `feels/pressure/clouds/uv`. MOS unchanged for the 5 corrected vars.
- [ ] **Step 4: Run.** `cargo test -p myqaz weather_ingest` → PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(weather): ingest fills pressure/clouds/uv/sunrise fields"`

### Task 1.5: Climate backfill job (manual)

**Files:** Create `src/jobs/weather_climate.rs`; Modify `src/jobs/mod.rs`.

- [ ] **Step 1: Failing test** — pure helper `climate_from_archive(slug, rows) -> CityClimate` (reuse `CityClimate::from_daily`); test produces 366-ish keys from synthetic multi-year rows.
- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement.** `weather_climate_job() -> Job`: `Lane::Heavy`, `Priority::Low`, `catch_up(false)`, NOT scheduled (manual trigger). Body: for each city, fetch ERA5 archive (`https://archive-api.open-meteo.com/v1/archive?...&start_date=<5y ago>&end_date=<yesterday>&daily=temperature_2m_max,temperature_2m_min,precipitation_sum,wind_speed_10m_max,relative_humidity_2m_mean`), batched coords; compute normals; `save_climate`. Log counts. Reuse `source.rs` HTTP plumbing.
- [ ] **Step 4: Run.** `cargo test -p myqaz weather_climate` → PASS.
- [ ] **Step 5:** `mod.rs`: `pub mod weather_climate; pub use weather_climate::weather_climate_job;`
- [ ] **Step 6: Commit.** `git commit -am "feat(weather): manual ERA5 climate backfill job"`

---

## Phase 2 — Grammar + slice helpers

### Task 2.1: Grammar sets + parsing

**Files:** Create `src/weather/grammar.rs`; Modify `src/weather/mod.rs`.

- [ ] **Step 1: Failing test:**

```rust
#[test]
fn classify_segments() {
    assert!(matches!(Period::parse("week"), Some(Period::Week)));
    assert!(matches!(Period::parse("hourly"), Some(Period::Hourly)));
    assert_eq!(Period::parse("nope"), None);
    assert!(matches!(Variable::parse("temperature"), Some(Variable::Temperature)));
    assert_eq!(Variable::parse("week"), None);
    assert_eq!(parse_hour("18"), Some(18));
    assert_eq!(parse_hour("24"), None);
}
```

- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement.** `enum Period { Hourly, Today, Tomorrow, ThreeDays, Week, FourteenDays, Month }` with `parse(&str)->Option`, `slug()`, `label(lang)`, `horizon_days()`. `enum Variable { Temperature, FeelsLike, Wind, Precipitation, Humidity, Pressure, Clouds, UvIndex, SunriseSunset }` with `parse`, `slug`, `label(lang)`, `unit(lang)`. `parse_hour(&str)->Option<u32>` (00..23). `parse_slash_date(y,m,d)->Option<NaiveDate>`.
- [ ] **Step 4: Run.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(weather): URL grammar — Period/Variable/hour/date parsing"`

### Task 2.2: Slice helpers

**Files:** Create `src/weather/slice.rs`; Modify `src/weather/mod.rs`.

- [ ] **Step 1: Failing test** with a fixture `CityForecast` (use `store.rs` `sample()` extended to many hours/days):

```rust
#[test]
fn slices() {
    let fc = sample_multi(); // 48 hourly from 2026-05-29T00:00, 14 daily
    let now = chrono::NaiveDate::from_ymd_opt(2026,5,29).unwrap().and_hms_opt(10,0,0).unwrap();
    assert_eq!(hours_from_now(&fc, now, 48).len(), 48);
    assert_eq!(period_days(&fc, Period::Week).len(), 7);
    assert_eq!(period_days(&fc, Period::FourteenDays).len(), 14);
    assert!(hour_at(&fc, /*day_offset*/0, 18).is_some());
    let series = variable_series(&fc, Variable::Temperature, Period::Week);
    assert_eq!(series.len(), 7);
}
```

- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement** pure fns: `hours_from_now(&fc, now, n) -> Vec<&HourWx>`; `period_hours(&fc, now, period)`; `period_days(&fc, period) -> &[DayWx]` (Week=7, ThreeDays=3, FourteenDays=14, Month=14 here, Today/Tomorrow=1); `day(&fc, &NaiveDate) -> Option<&DayWx>`; `hour_at(&fc, day_offset, hh) -> Option<&HourWx>`; `variable_series(&fc, var, period) -> Vec<(String,f64)>`; `month_with_climate(&fc, &CityClimate, now) -> Vec<DayCell>` (days 1–14 real, 15–30 from climate normals, flagged).
- [ ] **Step 4: Run.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(weather): pure slice helpers over CityForecast"`

---

## Phase 3 — Routing dispatch (the load-bearing phase)

### Task 3.0: SPIKE — confirm render/redirect capability

- [ ] **Step 1:** Inspect `adapto_app` for a public way to render a named template (loader→template→layout) to a `String`, and `PageResponse::Redirect`.

Run: `grep -rn 'Redirect\|pub fn render\|render_page\|render_named\|render_to_string' ~/adapto-core/crates/adapto_app/src ~/myqaz/myqaz-rs/src/template/mod.rs`

- [ ] **Step 2: Decide & record in this task:**
  - If a public render helper exists → use it from `App::page` handlers.
  - If not → add `pub fn render_named(template_name: &str, store: &AdaptoStore, lang: Lang, params: &[(&str,&str)]) -> Option<String>` to `src/template/mod.rs` (myqaz) wrapping the existing `loader → template → layout` pipeline (lines ~300-315), and re-run loaders with a synthetic `LoaderCtx`. Confirm `PageResponse::Redirect(String)` exists in `adapto_app` (it does per CLAUDE.md). Commit the helper: `git commit -am "feat(template): public render_named helper for dynamic dispatch"`.

### Task 3.1: Weather dispatch module

**Files:** Modify `src/routes/weather.rs` (add dispatch fns); they return `PageResponse`.

- [ ] **Step 1: Failing test** (`tests/weather_routes.rs`, TestClient; seed a forecast via in-memory store as existing weather tests do):

```rust
#[tokio::test]
async fn dispatch_matrix() {
    let app = test_app_with_forecast("astana");
    let c = app.test_client();
    assert_eq!(c.get("/weather/astana/week").await.status(), 200);
    assert_eq!(c.get("/weather/astana/temperature").await.status(), 200);
    assert_eq!(c.get("/weather/astana/week/temperature").await.status(), 200);
    assert_eq!(c.get("/weather/astana/hourly").await.status(), 200);
    assert_eq!(c.get("/weather/astana/today/18").await.status(), 200);
    assert_eq!(c.get("/weather/astana/2026/05/29").await.status(), 200);
    // redirect: variable/period -> period/variable
    let r = c.get("/weather/astana/temperature/week").await;
    assert_eq!(r.status(), 301);
    assert_eq!(r.header("location").unwrap(), "/weather/astana/week/temperature");
    // 404-ish: week/18 -> soft-404 noindex (200 with noindex) OR 404 per spike
    assert!(c.get("/weather/astana/week/18").await.text().contains("noindex"));
    // KK
    assert_eq!(c.get("/kz/weather/astana/week").await.status(), 200);
}
```

- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement** in `routes/weather.rs`:
  - `dispatch_l3(store, lang, city, seg) -> PageResponse`: `Period::parse` → render period; `Variable::parse` → render variable hub; dash-date regex → `Redirect` to slash; else soft-404.
  - `dispatch_l4(store, lang, city, a, b) -> PageResponse`: `(today|tomorrow)+hour` → hour page; `Period+Variable` → combo; `Variable+Period` → `Redirect("/.../{period}/{variable}")`; `hourly+Variable` → hourly-var; else soft-404.
  - `dispatch_day(store, lang, city, y, m, d) -> PageResponse`: valid in-horizon → day; today/tomorrow → `Redirect` to `/today`,`/tomorrow`; out-of-range → soft-404.
  - Each render path builds HTML via `render.rs` and wraps via `render_named`/helper from Task 3.0; returns `PageResponse::Ok`.
- [ ] **Step 4: Run.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(weather): segment-arity dispatch with redirects"`

### Task 3.2: Register dispatch routes in main.rs

**Files:** Modify `src/main.rs`.

- [ ] **Step 1:** Register (both langs; specific before generic per adapto_app order):

```rust
.page("/weather/:city/:y/:m/:d", |ctx| routes::weather::dispatch_day(ctx.store(), Lang::Ru, ctx.param("city"), ctx.param("y"), ctx.param("m"), ctx.param("d")))
.page("/weather/:city/:a/:b",    |ctx| routes::weather::dispatch_l4(ctx.store(), Lang::Ru, ctx.param("city"), ctx.param("a"), ctx.param("b")))
.page("/weather/:city/:seg",     |ctx| routes::weather::dispatch_l3(ctx.store(), Lang::Ru, ctx.param("city"), ctx.param("seg")))
// + /kz/weather/... mirrors with Lang::Kk
```
(Keep existing `/weather/:city` city template route and `/weather` index. Ensure the dispatch `:seg` does not shadow `/weather/:city` — different arity.)

- [ ] **Step 2: Run** the dispatch test from 3.1 against the full app. PASS.
- [ ] **Step 3: Commit.** `git commit -am "feat(weather): wire dispatch routes (RU+KK)"`

---

## Phase 4 — Renderers + design

### Task 4.1: render.rs scaffolding + shared chrome

**Files:** Create `src/weather/render.rs`; Modify `src/routes/weather.rs` (move helpers `wmo`, `temp_str`, etc. here or re-export).

- [ ] **Step 1: Failing test:** `render_period(&fc, Period::Week, lang)` returns HTML containing the H1, a stats line (min/max/avg), and a `<table`.
- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement** shared: `page_chrome(state, title, desc, h1, canonical, hreflang)`, `stats_line(series)`, `svg_sparkline(series)`, `faq_block(...)`, breadcrumbs via `components::breadcrumbs`. Each renderer composes these. **No emoji in index renderer; `wmo()` emoji retained in city/day/hour/period.**
- [ ] **Step 4: Run.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(weather): render.rs shared chrome + stats/sparkline"`

### Task 4.2–4.8: One renderer per page type (repeat the TDD cycle)

For EACH of: `render_hourly_list`, `render_hour`, `render_period`, `render_variable_hub`, `render_combo`, `render_day`, (city overview refactor):

- [ ] Write a test asserting: status content present, unique H1, stats line, data table/chart, FAQ, breadcrumbs, canonical + hreflang tags, `noindex` only on soft-404.
- [ ] Run → fail.
- [ ] Implement the renderer (pure `&CityForecast (+climate) → String`), using slice helpers + shared chrome. Hour page: prev/next hour links. Day: prev/next day. Month: `month_with_climate`, label days 15–30 "норма".
- [ ] Run → pass.
- [ ] Commit `feat(weather): <page-type> renderer`.

Representative (combo):

```rust
pub fn render_combo(fc: &CityForecast, p: Period, v: Variable, lang: Lang, base: &str) -> TemplateState {
    let series = slice::variable_series(fc, v, p);
    let h1 = format!("{} в {} — {}", v.label(lang), fc.city, p.label(lang));
    let body = format!("{}{}{}{}",
        intro_prose(fc, p, v, lang),
        stats_line(&series, v, lang),
        svg_bars(&series, v),
        faq_block(fc, p, v, lang));
    page_chrome(lang, &h1, &meta_desc(fc,p,v,lang), &h1, &canonical(base, fc, p, v), &body)
}
```

### Task 4.9: weather.css design + integrity

**Files:** Create/modify the weather CSS (confirm path via `static_dir` registration in main.rs; likely `static/css/weather.css` linked from `base.adapto`).

- [ ] **Step 1:** Add styles: card grid, hourly horizontal strip, day table, SVG chart sizing, responsive breakpoints, L2 menu already styled by `.gn-l2`. Ensure tables scroll on mobile.
- [ ] **Step 2:** Manual: run dev, eyeball index/city/hourly/hour/period/combo/day at 1280px + 390px. No overflow/overlap.
- [ ] **Step 3: Commit.** `git commit -am "style(weather): responsive design for all page types"`

---

## Phase 5 — Index redesign + L2 menu + city fix

### Task 5.1: L2 menu for Weather

**Files:** Modify `src/views/nav.rs`.

- [ ] **Step 1: Failing test:** `NavSection::Weather.section_l2(Lang::Ru).is_some()` and contains a link to `/weather/`.
- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement** the `NavSection::Weather => Some(NavL2 { title: "Погода"/"Ауа райы", title_url: Some("{pfx}weather/"), links: vec![ (Сейчас/hourly), (Сегодня/today via city context), (Неделя), (Месяц) ... ], entity_mode:false })`. City-aware variant: when a city path is active, build links to that city's periods (a helper `weather_l2(lang, city: Option<&str>)`), called from the dispatchers so `l2_current` highlights.
- [ ] **Step 4: Run.** PASS.
- [ ] **Step 5: Manual:** every weather page shows a non-empty L2 menu; active item highlighted.
- [ ] **Step 6: Commit.** `git commit -am "feat(nav): weather L2 menu (was missing)"`

### Task 5.2: Tiered, no-emoji index

**Files:** Modify `src/routes/weather.rs` (`load_weather_index_state`) + `src/weather/render.rs`.

- [ ] **Step 1: Failing test:** index HTML contains three tier headings (Крупнейшие / Областные центры / Остальные), lists all seeded cities, and contains NO emoji char in the index body.
- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement.** Const `MAJOR=["astana","almaty","shymkent"]`; `OBLAST=[17 slugs]` (resolve list — Task 5.4); render three labeled sections; remaining alphabetical. Card = name + temp + RU/KK condition text (no emoji). Placeholder "—" if no cached forecast.
- [ ] **Step 4: Run.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(weather): tiered no-emoji index"`

### Task 5.3: Fix "not all cities"

- [ ] **Step 1:** Using Task 0.3 numbers: if `load_cities < 302`, fix the namaz import / `load_cities` (the namaz doc on prod). If `cached < cities`, ensure the ingest loop iterates the full `load_cities()` and doesn't silently skip on per-chunk failures (log skipped slugs).
- [ ] **Step 2: Test:** seed a 2-city namaz doc → index lists both; integration asserts count.
- [ ] **Step 3: Commit.** `git commit -am "fix(weather): index lists all cities"`

### Task 5.4: Resolve OBLAST slug list

- [ ] **Step 1:** From the namaz city list, pick the 17 oblast-center slugs; hardcode the const. Verify each slug exists in `load_cities()`.
- [ ] **Step 2: Commit** (folded into 5.2 if done together).

---

## Phase 6 — SEO + sitemap sharding

### Task 6.1: Canonical + hreflang + JSON-LD per page

**Files:** Modify `src/weather/render.rs` (shared chrome).

- [ ] **Step 1: Failing test:** each renderer's output contains `<link rel="canonical"`, an `hreflang="ru"` + `hreflang="kk"` pair, and a `BreadcrumbList` JSON-LD script; index also has `ItemList`.
- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement** in `page_chrome`: set `extra_head` with canonical (self; today/tomorrow date → canonical to /today,/tomorrow), hreflang RU↔KK + x-default→RU, and JSON-LD. Confirm `base.adapto` emits `{@html extra_head}`.
- [ ] **Step 4: Run.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(weather): canonical, hreflang, JSON-LD on all pages"`

### Task 6.2: Sharded weather sitemaps

**Files:** Modify `src/routes/sitemap.rs`.

- [ ] **Step 1: Failing test:** `weather_urls(store)` length ≈ expected; `render_weather_shard(store, 0)` ≤ 50k `<url>`; `render_index` lists `sitemap-weather-{i}.xml` for the right shard count.
- [ ] **Step 2: Run, fail.**
- [ ] **Step 3: Implement.** Add `fn weather_urls(store) -> Vec<String>` enumerating the full grammar per city (periods, variable hubs, combos, hourly, today/tomorrow hours, 14 dates). Replace single `sitemap-weather.xml` with `render_weather_shard(store, n)` + sharding in `render_index` (mirror the company-shard code, `WEATHER_SHARD_SIZE = 50_000`). Route match: `/sitemap-weather-{n}.xml`.
- [ ] **Step 4: Run.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(seo): sharded weather sitemaps"`

### Task 6.3: Cross-linking

- [ ] **Step 1:** In renderers, add an internal-link block: city→periods→variables→combos→hours; day↔day; hour↔hour; namaz↔weather (already one-way namaz→weather; keep weather NOT linking namaz per [[feedback_weather_no_namaz_link]]).
- [ ] **Step 2: Test:** a combo page links its sibling periods + the city overview.
- [ ] **Step 3: Commit.** `git commit -am "feat(weather): dense internal linking"`

---

## Phase 7 — Wire, full test, build

### Task 7.1: Full suite + clippy

- [ ] Run: `cargo test 2>&1 | tail -20` → all green (≥ baseline + new). 
- [ ] Run: `cargo clippy --all-targets 2>&1 | tail -20` → no new warnings on touched files.
- [ ] Commit any fixups.

### Task 7.2: CHANGELOG (framework rule)

- [ ] If any `adapto-core` crate changed (e.g. render helper), update `~/adapto-core/CHANGELOG.md` (Added/Changed) with file:line refs and bump workspace version. Commit in adapto-core.

### Task 7.3: Cross-compile

- [ ] Run: `cd ~/myqaz/myqaz-rs && cargo zigbuild --release --target x86_64-unknown-linux-gnu 2>&1 | tail -20` → builds clean.

---

## Phase 8 — Deploy + prod verification (definition of done)

> Prod: `deploy@91.224.74.233:2222`, app at `/opt/myqaz`, port 8082 ([[reference_prod_server]]). Read the deploy config before acting; never guess params ([[feedback_verify_connection]]). Confirm with the user before the irreversible deploy step.

### Task 8.1: Deploy

- [ ] **Step 1:** Read `~/myqaz/myqaz-rs/scripts/` deploy script; confirm binary path, templates path, static path, service name.
- [ ] **Step 2:** Ship binary + templates + static via the existing deploy script; restart the service.
- [ ] **Step 3:** `curl -s -o /dev/null -w '%{http_code}' https://myqaz.kz/health` → 200.

### Task 8.2: Trigger climate backfill (once)

- [ ] Trigger `weather_climate` from `/admin/jobs` (token-guarded). Wait for completion; confirm `weather_climate` populated (spot a city). Month tail now has normals.

### Task 8.3: Automatic verification (prod URLs)

- [ ] Script-check, RU + KK, status + key content for: `/weather`, `/weather/astana`, `/weather/astana/hourly`, `/weather/astana/today/18`, `/weather/astana/week`, `/weather/astana/temperature`, `/weather/astana/week/temperature`, `/weather/astana/2026/05/29`. Redirects: `/weather/astana/temperature/week`→301, dash-date→301, `/weather/astana/week/18`→noindex. Record a table of results.

### Task 8.4: Data presence

- [ ] Confirm `/weather` lists all 302 cities; spot-check 5 cities incl. a small one show real temps (not "—").

### Task 8.5: Speed

- [ ] `curl -s -o /dev/null -w 'ttfb=%{time_starttransfer}s total=%{time_total}s\n'` for `/weather` and a deep combo page. Record. Cache-served pages should be well under ~150ms TTFB server-side.

### Task 8.6: Design integrity (Playwright)

- [ ] Use the playwright MCP: screenshot index/city/hourly/hour/period/variable/combo/day at 1280px and 390px (RU + a KK page). Confirm L2 menu present everywhere, no broken layout, charts/tables render, emoji rules respected. Save screenshots; report any breakage and fix.

### Task 8.7: Sitemap

- [ ] `curl https://myqaz.kz/sitemap.xml` lists `sitemap-weather-*.xml`; fetch a shard, confirm `<url>` count ≤ 50k and sample URLs 200.

### Task 8.8: Finish

- [ ] Update `~/myqaz/myqaz-rs` CHANGELOG/notes. Use superpowers:verification-before-completion before claiming done. Report the full verification table to the user.

---

## Self-Review

**Spec coverage:** URL grammar→Task 2.1/3.1; periods/variables/hubs/combos/hourly/hours/dates→3.1/4.2-4.8; month+climate→1.3/1.5/2.2; index-everything sitemap→6.2; emoji rule→4.1/5.2; tiers→5.2/5.4; L2 menu→5.1; data fields→1.1/1.2/1.4; jobs→1.4/1.5; SEO→6.1/6.3; "not all cities"→0.3/5.3; design→4.9/8.6; deploy+verify→Phase 8. All spec sections mapped.

**Placeholder scan:** OBLAST slug list deferred to Task 5.4 (explicit resolution task, not a silent TODO). Renderer code is representative (combo shown) with an explicit per-type TDD loop in 4.2–4.8 — acceptable for 7 structurally-identical renderers; each step still requires real code at execution.

**Type consistency:** `Period`/`Variable` (2.1) used in slice (2.2), dispatch (3.1), renderers (4.x), sitemap (6.2). `CityForecast`/`HourWx`/`DayWx` extended (1.1) used everywhere. `CityClimate`/`ClimNormal` (1.3) used in 2.2/1.5. `render_named` (3.0) used in 3.1. Names consistent.

**Risk:** Phase 3.0 spike is load-bearing — if `adapto_app` lacks a render helper, Task 3.0 adds one (we own the framework). Soft-404 (HTTP 200 + noindex) is used where a route matches but is invalid, matching the existing weather pattern; true 404 only for unmatched paths via fallback.
