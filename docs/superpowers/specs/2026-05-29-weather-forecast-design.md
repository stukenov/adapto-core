# Weather Forecast (MOS Ensemble) — Design Spec

**Date:** 2026-05-29
**Status:** Approved (architecture), pending implementation plan
**Author:** Saken + Claude

## Goal

Build an in-house weather forecast for all 302 cities currently used by the namaz
schedule. Forecasts are produced by **our own model** (MOS ensemble): we continuously
download live weather from Open-Meteo, learn each source's error per city, and emit a
refined, blended forecast that is recomputed every hour.

The reusable machine-learning primitive lives in the framework (new `adapto_ml` crate),
mirroring the `adapto_scheduler` precedent: generic engine in core, domain logic
(ingestion, features, jobs, pages) in myqaz.

## Locked Decisions

| Axis | Decision |
|------|----------|
| Model | MOS ensemble — per-city, per-variable ridge regression that blends Open-Meteo NWP models and corrects bias, learned from observed error over time |
| Placement | New generic crate `adapto_ml` in core; weather domain in myqaz |
| Data source | Open-Meteo only (multi-model + `current` + ERA5 archive), batched coordinates, keyless, behind a `WeatherSource` trait so a second provider can be added later |
| Horizon | Current conditions + 48h hourly + 14-day daily |
| Pages | `/weather/` index + `/weather/{city}/` + `/weather/{city}/{date}/`; RU at `/weather/...`, KK at `/kz/weather/...`; reuse the 302 namaz city slugs |
| Cold start | Below `K = 200` paired samples for a city×variable, fall back to ensemble mean |
| Retention | `weather_obs` and `weather_train` pruned to a rolling 60 days |

## Component Map

### core: `adapto_ml`
Generic, dependency-light (serde only). Pure Rust, no GPU.

- `StandardScaler { mean: Vec<f64>, std: Vec<f64> }` — feature standardization.
- `RidgeRegression { weights: Vec<f64>, bias: f64, lambda: f64, scaler: StandardScaler }`
  - `fit(features: &[Vec<f64>], targets: &[f64], lambda: f64) -> Result<Self, MlError>`
    solves `(XᵀX + λI) w = Xᵀy` via Gaussian elimination (feature count ~9, small).
  - `predict(&self, x: &[f64]) -> f64`
  - `to_json(&self) -> String` / `from_json(s: &str) -> Result<Self, MlError>` (serde).
- `MlError` — singular matrix, dimension mismatch, empty input.

### myqaz: weather domain
- `weather/source.rs` — `WeatherSource` trait + `OpenMeteo` impl: `fetch_forecast` (all
  models, hourly 48h + daily 14d), `fetch_current` (analysis), `fetch_archive` (ERA5).
  reqwest (rustls) + serde. Coordinates batched in chunks (~100/request).
- `weather/features.rs` — build the feature vector per (city, variable, lead).
- `weather/model.rs` — `MosModel`: wraps an `adapto_ml::RidgeRegression` per (city,
  variable); `predict` with cold-start fallback; physical clamping.
- `weather/store.rs` — storage schema, read/write helpers, `Reloadable` cache.
- `jobs/weather_ingest.rs` — hourly orchestration.
- `jobs/weather_train.rs` — nightly refit + prune.
- `routes/weather.rs` — index, city, day loaders (Rust-built HTML + `{@html}`).
- `templates/pages/weather/{index,city,day}.adapto`.

The city list (slug, lat, lng) is read from the existing `namaz` document — single
source of truth, no duplication.

## Data Model (store collections)

- **`weather_forecast`** (id = city slug): the page-facing document.
  `{ city, lat, lng, issued_at, current{…}, hourly:[…×48], daily:[…×14], src_meta }`.
  Read O(1) by slug.
- **`weather_obs`** (rolling, 60d): observed analysis per city per valid-hour.
  `{ slug, valid_at, temp, wind, humidity, precip }`.
- **`weather_train`** (rolling, 60d): training pairs.
  `{ slug, var, lead_h, features:[…], target, valid_at }`.
- **`weather_models`** (id = city slug): per-variable serialized coefficients
  (`adapto_ml` JSON). Loaded by inference, refit nightly. Empty ⇒ fallback.

## MOS Model Structure

One `RidgeRegression` per (city, variable):

```
features = [icon, gfs, ecmwf, gem, lead_norm,
            sin(2π·hour/24), cos(2π·hour/24),
            sin(2π·doy/365), cos(2π·doy/365)]
target   = observed value at the valid hour
```

This single linear model learns optimal source weights, diurnal and seasonal bias, and
lead-dependent correction together.

- Corrected variables: `temperature_2m`, `apparent_temperature`, `wind_speed_10m`,
  `relative_humidity_2m`, `precipitation` (precip clamped ≥ 0).
- Categorical fields (`weather_code`, cloud cover): taken from the model with the best
  recent MAE — no regression.
- Cold start: `< K = 200` paired samples for a city×variable ⇒ ensemble mean; hard switch
  at K.
- Days 8–14: near-zero forecast skill, so we show the best raw model as a "tendency" and
  label it honestly rather than implying precision.

## Jobs (adapto_scheduler)

- **`weather_ingest`** — cron `0 * * * *` (Almaty), Light lane / High priority,
  `catch_up = false`, exponential backoff. Each tick:
  1. Batch-fetch forecast (all models) + `current` for all cities.
  2. Write observed analysis → `weather_obs[slug, now]`.
  3. Pairing: join past forecasts whose `valid_hour == now` with `observed[now]` →
     append to `weather_train`.
  4. Inference: build features from the fresh ensemble, apply the MOS model (or fallback)
     → refined hourly/daily → write `weather_forecast`.
  5. Refresh the `Reloadable` cache.
- **`weather_train`** — `DailyAt 02:30` (Almaty), Heavy lane / Low priority,
  `catch_up = false`. Refit coefficients per city×variable from `weather_train`; prune
  `weather_obs` and `weather_train` older than 60 days.
- **`weather_backfill`** — manual, triggered once from `/admin/jobs`. Pulls ERA5 archive
  (60–90 days hourly) to bootstrap training on first deploy. Not scheduled.

## Inference & Caching

Pages never call Open-Meteo. They read `weather_forecast` through
`Reloadable<HashMap<slug, Arc<CityForecast>>>`, refreshed at the end of each ingest tick
(lock-free, < 1 ms) — the same pattern as `exchange_rates`. Rendering builds HTML in Rust
and injects via `{@html}` to stay under ~8 ms (avoiding the slow `{#each}` path).

## Pages

- `/weather/` (RU) / `/kz/weather/` (KK): index of all 302 cities, grouped by
  region/alphabet, current temperature + icon.
- `/weather/{city}/`: hero current conditions + 48h hourly + 14-day table + JSON-LD +
  cross-link to the namaz city page.
- `/weather/{city}/{date}/`: hourly detail for that date (`today .. +14`; outside range →
  404 / redirect to the city page). `date` format `YYYY-MM-DD`.
- Cross-link namaz ↔ weather (shared slug). Sitemap gains the weather URLs.
- New `NavSection::Weather` variant.

## Error Handling

- Source fetch failure → job retry with backoff.
- Partial per-city failure → **do not overwrite** the last good `weather_forecast`
  document; `issued_at` ages and the page shows "обновлено N ч назад".
- NaN / cold model → fallback + physical clamps (humidity 0–100, precip ≥ 0, wind ≥ 0).
- Missed ingest tick → that hour simply produces no training pair.
- Singular fit matrix → keep the previous model, log a warning.

## Testing

- `adapto_ml`: ridge recovers known coefficients; scaler fit/transform; serde round-trip;
  singular-matrix handling.
- `features`: deterministic vector for a fixed input.
- `source`: parse an Open-Meteo JSON fixture into typed structs.
- `model`: cold-start fallback; clamping; output ranges.
- `ingest`: synthetic forecast + observation → correct training rows.
- `routes`: TestClient — index/city 200 and layout-wrapped; day in-range 200 /
  out-of-range 404; KK variant.
- integration: in-memory store, seed a forecast document, page contains the current
  temperature.

## Build Order (plan phases)

1. `adapto_ml` crate (core, TDD, standalone).
2. weather types + Open-Meteo source (fixture-tested).
3. storage schema + `Reloadable` cache.
4. MOS model + features (cold-start first).
5. `weather_ingest` job (fetch → store → pair → infer → cache).
6. `weather_train` job (refit + prune).
7. routes + templates (index, city, day) RU+KK + nav + sitemap + cross-links.
8. wire into `main.rs` scheduler + admin; deploy; backfill; verify on prod.

## Verification Items (resolve during planning)

- Confirm Open-Meteo serves multi-coordinate **and** multi-model in one request; if not,
  fall back to one request per model (still a handful of calls per hour).
- Confirm Open-Meteo commercial-use terms for myqaz.kz; the `WeatherSource` trait keeps an
  optional API key / base URL configurable for a paid or self-hosted endpoint.
