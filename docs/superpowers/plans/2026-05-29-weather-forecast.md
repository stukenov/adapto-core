# Weather Forecast (MOS Ensemble) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** In-house hourly-refreshed weather forecast for all 302 namaz cities, produced by a per-city MOS ensemble model that blends and bias-corrects Open-Meteo NWP feeds.

**Architecture:** New generic `adapto_ml` crate in core (ridge regression + scaler). myqaz adds an Open-Meteo source, a MOS model layer, two scheduler jobs (hourly ingest, nightly train), and `/weather` pages — reusing the existing 302 namaz city slugs/coords and the `Reloadable` cache + `{@html}` render patterns.

**Tech Stack:** Rust, serde, reqwest (rustls), adapto_store, adapto_scheduler, chrono/chrono-tz.

Spec: `docs/superpowers/specs/2026-05-29-weather-forecast-design.md`.

---

## Part A — `adapto_ml` (core crate)

### Task A1: Crate skeleton
**Files:** Create `crates/adapto_ml/Cargo.toml`, `crates/adapto_ml/src/lib.rs`; Modify root `Cargo.toml` members.

- [ ] Add `"crates/adapto_ml"` to workspace `members`.
- [ ] `Cargo.toml`: deps `serde` (workspace, derive), `serde_json` (workspace), `thiserror` (workspace). version.workspace etc.
- [ ] `lib.rs`: `mod scaler; mod ridge;` + `pub use`.
- [ ] Run `cargo build -p adapto_ml`. Expected: compiles (empty).

### Task A2: StandardScaler
**Files:** Create `crates/adapto_ml/src/scaler.rs`.

- [ ] Test: fitting `[[0],[2],[4]]` gives mean 2, std ~1.633; `transform([2])≈0`, `transform([4])≈1.2247`. Zero-variance column → std forced to 1 (transform passes value through as `x-mean`).
- [ ] Impl `StandardScaler { mean: Vec<f64>, std: Vec<f64> }` with `fit(rows: &[Vec<f64>]) -> Self` (population std; std<1e-9 ⇒ 1.0), `transform(&self, x: &[f64]) -> Vec<f64>`, serde derive.
- [ ] `cargo test -p adapto_ml scaler`.

### Task A3: RidgeRegression
**Files:** Create `crates/adapto_ml/src/ridge.rs`.

- [ ] Test `recovers_linear`: data `y = 3*x0 + 2` (x0 = 0..10), `fit(lambda=1e-6)`, `predict([4])≈14` within 0.1.
- [ ] Test `multi_feature`: `y = 1*x0 + 2*x1`, recovers within 0.2.
- [ ] Test `serde_roundtrip`: `from_json(to_json())` predicts equally.
- [ ] Test `singular_or_empty_errors`: empty features ⇒ `Err(MlError::Empty)`.
- [ ] Impl `MlError { Empty, DimMismatch, Singular }` (thiserror). `RidgeRegression { weights, bias, lambda, scaler }`. `fit(features,targets,lambda)`: standardize, build `(XᵀX+λI)` on standardized features (no penalty on bias — handle bias by centering target), solve via Gaussian elimination with partial pivoting; on singular ⇒ `Err`. `predict(x)`: scaler.transform then dot+bias. `to_json/from_json` via serde_json.
- [ ] `cargo test -p adapto_ml`. All pass.
- [ ] Commit `feat(ml): add adapto_ml crate — ridge regression + standard scaler`.

---

## Part B — myqaz weather domain

> myqaz lives at `~/myqaz/myqaz-rs`. It path-patches adapto crates. Add `adapto_ml` to its `[patch.crates-io]` and deps.

### Task B1: Dependencies + city source
**Files:** Modify `Cargo.toml`; Create `src/weather/mod.rs`, `src/weather/cities.rs`.

- [ ] `Cargo.toml`: add `adapto_ml` path dep + patch; `reqwest`, `quick-xml`, `chrono-tz` already present.
- [ ] `cities.rs`: `pub struct GeoCity { slug, name, name_prep, lat, lng }`; `pub fn load_cities(store) -> Vec<GeoCity>` reads the single `namaz` doc (OnceLock) → maps `NamazData.cities`.
- [ ] Test (integration, in-memory store seeded with a 2-city namaz doc) → `load_cities` returns 2 with coords.
- [ ] `src/main.rs`: add `mod weather;`.

### Task B2: Open-Meteo source
**Files:** Create `src/weather/source.rs`.

- [ ] Define typed response structs + `WeatherSource` trait: `async fn fetch(&self, cities: &[GeoCity]) -> Result<Vec<CityFetch>, String>` where `CityFetch { slug, current: ObsPoint, models: Vec<ModelSeries> }`, `ModelSeries { model: String, hourly: Vec<HourPoint>, daily: Vec<DayPoint> }`.
- [ ] `OpenMeteo` impl: build forecast URL `https://api.open-meteo.com/v1/forecast?latitude=..&longitude=..&hourly=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,precipitation,weather_code,cloud_cover&daily=temperature_2m_max,temperature_2m_min,weather_code,precipitation_sum,wind_speed_10m_max&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code&forecast_days=14&models=icon_seamless,gfs_seamless,ecmwf_ifs025,gem_seamless&timezone=auto`. Chunk coords ≤100/request. Parse per-model suffixed arrays (`temperature_2m_icon_seamless`, etc.); when single model requested no suffix.
- [ ] Test: parse a saved JSON fixture `tests/fixtures/openmeteo_2city.json` → correct counts (hourly len, models present, current temp). (Pure parse fn `parse_forecast(json, &cities) -> Vec<CityFetch>` is unit-tested; HTTP not tested.)
- [ ] **Verification step during this task:** `curl`-equivalent (python urllib) one real 2-coord multi-model call; confirm multi-coord+multi-model works; if it 400s, switch `OpenMeteo` to one request per model and concat. Save the real response as the fixture.

### Task B3: Storage schema + types
**Files:** Create `src/weather/store.rs`, `src/weather/types.rs`.

- [ ] `types.rs`: `CityForecast { city, slug, lat, lng, issued_at: String, current: CurrentWx, hourly: Vec<HourWx>, daily: Vec<DayWx> }` (+ serde). `HourWx { t: String, temp, feels, humidity, wind, precip, code: i64, is_estimate: bool }`. `DayWx { date, tmin, tmax, code, precip, wind, tendency: bool }`. `CurrentWx { temp, humidity, wind, code, updated: String }`.
- [ ] `store.rs`: constants `FC="weather_forecast"`, `OBS="weather_obs"`, `TRAIN="weather_train"`, `MODELS="weather_models"`. `ensure_indexes(store)` (slug indexes). `save_forecast(store,&CityForecast)` (delete by slug + insert). `read_forecast(store,slug)->Option<CityForecast>`. `append_obs/append_train/prune(store, cutoff_iso)`.
- [ ] `Reloadable<HashMap<String,Arc<CityForecast>>>` static `FORECASTS` + `refresh_from_store(store)` + `get(slug)`.
- [ ] Tests: save→read roundtrip; prune removes old rows; cache refresh then get.

### Task B4: Features + MOS model
**Files:** Create `src/weather/features.rs`, `src/weather/model.rs`.

- [ ] `features.rs`: `pub fn feature_vector(members: &[f64;4], lead_h: f64, hour: u32, doy: u32) -> Vec<f64>` = `[icon,gfs,ecmwf,gem, lead/336.0, sin(2π h/24), cos, sin(2π doy/365), cos]`. Missing member ⇒ filled with ensemble mean of present members.
- [ ] Test: deterministic vector for fixed input; sin/cos bounds.
- [ ] `model.rs`: `pub struct CityModels { vars: HashMap<String, adapto_ml::RidgeRegression> }`. `predict(var, feat, ensemble_mean) -> f64`: if model present → ridge.predict, else fallback to `ensemble_mean`; clamp (`relative_humidity_2m`→0..100, `precipitation`/`wind_speed_10m`→≥0). `to_json/from_json`.
- [ ] `pub const COLD_START_MIN: usize = 200;` `pub fn train_var(rows:&[(Vec<f64>,f64)]) -> Option<RidgeRegression>` → `None` if `<COLD_START_MIN` else `RidgeRegression::fit(..,1.0).ok()`.
- [ ] Tests: cold-start returns None <200; fallback predict = ensemble mean when no model; clamp negatives.

### Task B5: Ingest job
**Files:** Create `src/jobs/weather_ingest.rs`; Modify `src/jobs/mod.rs`.

- [ ] `weather_ingest_job() -> Job`: `Schedule::Cron("0 0 * * * *")` (top of every hour), `Lane::Light`, `Priority::High`, `catch_up(false)`, `max_attempts(3)`.
- [ ] Body: load cities; `OpenMeteo.fetch(&cities)`; for each city: write `current`→OBS (rounded hour); pair past forecasts whose valid hour == now-hour with OBS → append TRAIN; load `CityModels` from MODELS (or empty); for each hourly lead build features → `predict` per var → assemble `HourWx`/`DayWx`; mark `is_estimate=true` when model used, `tendency=true` for day≥8; build `CityForecast` with `issued_at=now`; `save_forecast`; on per-city fetch failure keep previous doc. After loop: `refresh_from_store`. Log counts.
- [ ] Pure helpers extracted + unit-tested: `pair_training(prev_forecasts, obs_now) -> Vec<TrainRow>`; `assemble_forecast(city, fetch, &CityModels, now) -> CityForecast`.
- [ ] Tests: synthetic fetch+models → `assemble_forecast` produces 48 hourly + 14 daily, estimate flags correct; pairing yields rows only for matching valid hour.
- [ ] `mod.rs`: `pub mod weather_ingest; pub use weather_ingest::weather_ingest_job;`.

### Task B6: Train job
**Files:** Create `src/jobs/weather_train.rs`; Modify `src/jobs/mod.rs`.

- [ ] `weather_train_job() -> Job`: `Schedule::DailyAt{hour:2,min:30}`, `Lane::Heavy`, `Priority::Low`, `catch_up(false)`.
- [ ] Body: for each city: read TRAIN rows grouped by var; `train_var` → if Some store into `CityModels`; `save` MODELS doc per city. Then `prune` OBS+TRAIN older than 60d. Log models trained.
- [ ] Test: seed >200 synthetic rows for one var (y≈icon) → after train, MODELS doc has that var and predicts near icon.

### Task B7: Routes + templates
**Files:** Create `src/routes/weather.rs`, `templates/pages/weather/{index,city,day}.adapto`; Modify `src/main.rs`, `src/views/nav.rs`, `src/routes/mod.rs`, `src/routes/sitemap.rs`, `src/routes/namaz.rs`.

- [ ] `nav.rs`: add `NavSection::Weather` + L2 (None ok) + section entry `("Погода","/weather/",NavSection::Weather)`.
- [ ] `weather.rs`: `pfx(lang)` = `/weather/` | `/kz/weather/`. Loaders (HTML built in Rust + `{@html}`, render_page like namaz):
  - `load_weather_index_state(store,lang)`: cache → all cities grouped, current temp.
  - `load_weather_city_state(store,city,lang)`: current hero + 48h hourly table + 14d table (Rust HTML). 404→empty state if unknown slug.
  - `load_weather_day_state(store,city,date,lang)`: hours of `date`; if `date` outside today..+14 or unknown → empty (→ 404 via auto-route NotFound).
- [ ] Templates: `<route> path:` + `layout: "layouts/base"`; body = `{@html ...}` blocks. RU+KK handled by lang prefix in loader.
- [ ] `namaz.rs`: add a "Погода в {city}" cross-link on the city page (`/weather/{slug}/`).
- [ ] `sitemap.rs`: emit weather index + per-city URLs (both langs).
- [ ] `main.rs`: `register_loader("pages/weather/index"|"city"|"day", ...)`.
- [ ] Tests (`tests/weather_routes.rs`, TestClient): seed forecast doc; index 200 + contains city; city 200 + layout-wrapped + temp; day in-range 200; day out-of-range 404; KK `/kz/weather/{city}/` 200.

### Task B8: Wire scheduler + deploy
**Files:** Modify `src/main.rs`.

- [ ] Add `.job(jobs::weather_ingest_job()).job(jobs::weather_train_job())` to the existing `Scheduler::new(...)` chain; `weather::store::ensure_indexes(&store)` at boot; `weather::store::refresh_from_store(&store)` in cache warm.
- [ ] `cargo test` (myqaz) green; `cargo build`.
- [ ] Cross-compile `cargo zigbuild --release --target x86_64-unknown-linux-gnu`; scp binary; restart systemd; verify `/weather/almaty/` 200 + a current temp; trigger `weather_ingest` via `/admin/jobs`; (optional) `weather_backfill` later.

---

## Self-Review notes
- Spec coverage: A1–A3 = adapto_ml; B1 cities; B2 source; B3 storage/cache; B4 features/MOS+cold-start; B5 ingest (pairing+inference); B6 train+prune; B7 pages/nav/sitemap/cross-link; B8 wiring+deploy. backfill = manual (B8 optional).
- Type consistency: `CityForecast/HourWx/DayWx` defined B3, used B5/B7. `CityModels` defined B4, used B5/B6. `GeoCity` B1 used B2/B5. `CityFetch/ModelSeries` B2 used B5.
- Open question resolved at B2 verification step (multi-coord+multi-model).
