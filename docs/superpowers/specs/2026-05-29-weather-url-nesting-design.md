# Weather URL Nesting & SEO Expansion — Design Spec

**Date:** 2026-05-29
**Status:** Approved (architecture), pending implementation plan
**Author:** Saken + Claude
**Builds on:** `2026-05-29-weather-forecast-design.md` (MOS ensemble, the data layer)
**Implementation target:** `~/myqaz/myqaz-rs` (weather domain). No `adapto-core` crate changes expected.

## Goal

Expand the myqaz weather section from 3 URL levels into a deep, fully-indexable
programmatic-SEO surface: per-period pages, per-variable hubs, period×variable
combinations, hourly view, per-hour pages for the next 48h, and day pages — for
all 302 cities in RU and KK. Redesign the `/weather` index (no emoji, tiered
prioritization, full SEO) and fix the "not all cities shown" issue.

The forecast data already refreshes hourly into a lock-free cache
(`CityForecast{current, hourly[≤336], daily[≤14]}`). Every new URL is a different
**slice/view** over that same cached document (plus a climate-normals collection
for the tail of the month). No new real-time pipeline is required — this is URL
grammar, slice helpers, renderers, design, SEO, and a one-shot climate backfill.

## Locked Decisions

| Axis | Decision |
|------|----------|
| Nesting | Full matrix, maximum depth |
| Segment order | period → variable (`/week/temperature`, not `/temperature/week`) |
| Periods (7) | `hourly, today, tomorrow, 3-days, week, 14-days, month` |
| Variables (9) | `temperature, feels-like, wind, precipitation, humidity, pressure, clouds, uv-index, sunrise-sunset` |
| Variable hubs | Yes — standalone L3 `/weather/{city}/{var}` (default 7-day horizon) |
| Month | Days 1–14 = MOS forecast; days 15–30 = climate normals, explicitly labeled "норма, не прогноз" |
| Hours | Only next 48h (today + tomorrow), each its own URL: `/weather/{city}/today/{hh}` and `/weather/{city}/tomorrow/{hh}`. NOT nested inside arbitrary dates. |
| Dates | Slash-separated: `/weather/{city}/{yyyy}/{mm}/{dd}` (dash form `2026-05-29` → 301). Day pages contain no hour subpages. |
| Indexing | **Index everything** — all page types in index + sitemap. Defense against thin-content penalty = substantively unique content per page (own H1, intro prose, table, stats, FAQ) + dense internal linking. |
| Emoji | Removed on `/weather` index only; kept on city/day/hour/detail pages (`wmo()`). |
| Index priority | Tiers: republican cities (Astana/Almaty/Shymkent) → 17 oblast centers → rest alphabetical. Curated slug lists, no extra data. |
| Geolocation | **Deferred** to a later spec. |
| Localized slugs | English slugs for both languages (consistent with the existing `/weather/` ↔ `/kz/weather/` base). |

## URL Grammar

RU base `/weather/...`, KK base `/kz/weather/...`. `{city}` = existing namaz slug.

```
/weather/{city}                      L2  overview: current + 7-day summary
/weather/{city}/hourly               L3  hourly list, rolling 48h from current hour
/weather/{city}/hourly/{var}         L4  hourly, single variable
/weather/{city}/{period}             L3  period page (today|tomorrow|3-days|week|14-days|month)
/weather/{city}/{period}/{var}       L4  combo (period → variable)
/weather/{city}/{var}                L3  variable hub (default 7-day horizon)
/weather/{city}/today/{hh}           L4  specific hour today  (hh = 00..23)
/weather/{city}/tomorrow/{hh}        L4  specific hour tomorrow
/weather/{city}/{yyyy}/{mm}/{dd}     L3* day detail (3 path segments; no hours inside)
```

`PERIODS = {hourly, today, tomorrow, 3-days, week, 14-days, month}`
`VARIABLES = {temperature, feels-like, wind, precipitation, humidity, pressure, clouds, uv-index, sunrise-sunset}`
(static `&[&str]` slices; routing is a `match` against these.)

### Routing & dispatch (by segment count after `{city}`)

- **1 segment** `/{seg}`:
  - `seg ∈ PERIODS` → period page (`hourly` → hourly list).
  - `seg ∈ VARIABLES` → variable hub.
  - `seg` matches `^\d{4}-\d{2}-\d{2}$` (legacy dash date) → **301** to slash form.
  - else → 404 (noindex page).
- **2 segments** `/{a}/{b}`:
  - `a ∈ {today, tomorrow}` AND `b` matches `^\d{1,2}$` in `00..23` → hour page.
  - `a ∈ PERIODS` AND `b ∈ VARIABLES` → combo page.
  - `a ∈ VARIABLES` AND `b ∈ PERIODS` → **301** to `/{b}/{a}` (canonical period-first).
  - `a == hourly` AND `b ∈ VARIABLES` → hourly single-variable.
  - else (e.g. `week/18`, `3-days/05`) → 404.
- **3 segments** `/{yyyy}/{mm}/{dd}`:
  - Valid date within `[today, today+13]` → day page.
  - Outside horizon / malformed → 404 (or redirect to city overview).

Axum registration (specific → generic, per adapto_app order): `/weather/:city`,
`/weather/:city/:seg`, `/weather/:city/:a/:b`, `/weather/:city/:y/:m/:d`. Each
handler dispatches internally on the sets/regex above. KK variants mirror under
`/kz/weather/...`.

### Canonical / redirect rules

- Every page is self-canonical (index-everything).
- `today`/`tomorrow` are the canonical pages for those two days; the equivalent
  `/{yyyy}/{mm}/{dd}` day page for today/tomorrow sets `<link rel=canonical>` to
  the `/today` resp. `/tomorrow` URL (single dedupe pair).
- `var/period` → 301 `period/var`.
- legacy dash date → 301 slash date.

## Page-Count Scale (×2 languages, 302 cities)

| Page type | Count |
|---|---|
| L2 city | 604 |
| hourly list | 604 |
| period (×7) | 4 228 |
| variable hub (×9) | 5 436 |
| combo period×var (7×9) | 38 052 |
| day (×14) | 8 456 |
| hour (today+tomorrow, ~48) | ~29 000 |
| **Total** | **~86 000** |

All indexable (user decision). Sitemap must be a **sitemap index** sharded into
`sitemap-weather-N.xml` files of ≤50 000 URLs each (~2–3 shards now; generator
chunks generically). Crawl-budget risk accepted; mitigated by unique per-page
content and dense internal linking.

## Data Model

Existing (`src/weather/types.rs`) extended; serde back-compatible via `#[serde(default)]`
on new fields so old cached docs still deserialize until the next ingest tick.

- `CurrentWx` += `feels: f64, pressure: f64, clouds: f64, uv: f64`.
- `HourWx` += `pressure: f64, clouds: f64, uv: f64`.
- `DayWx` += `sunrise: String, sunset: String, uv_max: f64, pressure: f64, clouds: f64, humidity: f64`.
- `CityForecast` unchanged in shape.

Variable → source:
- `temperature, feels-like, wind, precipitation, humidity` — MOS-corrected (existing).
- `pressure, clouds, uv-index` — best-recent-MAE raw model, no regression (same policy as `weather_code`).
- `sunrise-sunset` — daily astronomical fields from Open-Meteo (deterministic; any model).

New collection **`weather_climate`** (id = city slug): day-of-year normals
`{ doy: 1..366 → {tmax, tmin, precip, wind, humidity} }`. Used only to fill month
days 15–30. Computed once from the ERA5 archive (≈5 years), refreshable annually.

## Jobs (adapto_scheduler)

- **`weather_ingest`** (existing, hourly) — extend the Open-Meteo fetch to include
  hourly `surface_pressure, cloud_cover, uv_index` and daily `sunrise, sunset,
  uv_index_max`; populate the new fields. MOS still applies only to the 5 corrected
  variables; pressure/clouds/uv take the best raw model; sunrise/sunset passthrough.
- **`weather_train`** (existing, nightly) — unchanged (trains the 5 corrected vars).
- **`weather_climate`** (new, manual from `/admin/jobs`) — pull ERA5 archive
  (Open-Meteo archive API, batched coords) ~5 years daily per city, compute
  day-of-year means → `weather_climate`. Heavy lane / Low priority / `catch_up(false)`.

## Rendering & Views

Pure functions over the cached `CityForecast` (+ `weather_climate` for the month tail).

- **`src/weather/slice.rs`** (new, pure, unit-tested): `hours_from_now(&fc, n)`,
  `period_days(&fc, period) -> &[DayWx]`, `period_hours(&fc, period)`,
  `day(&fc, date) -> Option<&DayWx>`, `hour_at(&fc, day_offset, hh) -> Option<&HourWx>`,
  `variable_series(&fc, var, period) -> Vec<(label, value)>`, `month_with_climate(&fc, &climate)`.
- **`src/weather/render.rs`** (new): HTML builders per page type — extract from the
  growing `routes/weather.rs`. `routes/weather.rs` keeps thin loaders that call render.
- Page types: index, city overview, hourly list, hour detail, period, variable hub,
  combo, day. Each renders: distinct `<h1>`, language-specific intro prose, a data
  table and/or inline-SVG chart (sparkline/bars, no JS), summary stats (min/max/avg,
  e.g. "температура от −3° до +7°, в среднем +2°"), and a short FAQ block — making
  every URL substantively unique.
- **Design** (`static` weather CSS): card grid, horizontal hourly strip, day table,
  inline-SVG variable charts, responsive. Breadcrumbs on every page; prev/next nav
  (city↔city on index alpha order; hour↔hour; day↔day); cross-links
  city→periods→variables→combos→hours. Emoji via `wmo()` retained on city/day/hour;
  index uses no emoji.

## Navigation (L2 menu — currently missing)

The weather section has no level-2 submenu (other sections do; loaders already set
`l2_current` but no weather L2 menu is defined). Add a weather L2 nav, shown on
every weather page, with the primary entry points:

- Главная погоды (`/weather/`)
- Сейчас / Почасовой (`/weather/{city}/hourly`) — contextual when a city is active
- Today / Tomorrow / Week / Month
- Variables row (temperature, wind, precipitation, …) when a city is active

The L2 menu is city-aware: on `/weather` index it links to the index facets; on a
city page it links to that city's periods/variables. `l2_current` highlights the
active entry. Implemented in the existing nav component / template, keyed by
`section == "weather"`. Acceptance: a visible, non-empty L2 menu on every weather
page, with the active item highlighted.

## Index Redesign (`/weather`)

- **Tiers** (curated const slug lists): `MAJOR = [astana, almaty, shymkent]`;
  `OBLAST = [17 regional centers]`; rest alphabetical. Render three labeled sections.
- No emoji. Each card: city name + temperature + short condition text (RU/KK label,
  not the emoji). Placeholder "—" when a city has no cached forecast (so no city
  silently disappears).
- **"Not all cities" fix:** the loader already iterates every city from
  `load_cities()`; root cause is either `load_cities()` returning < 302 (prod namaz
  doc) or ingest not covering all 302. Plan step: log `load_cities().len()` and
  `store::all().len()` at warmup; if `< 302`, fix the namaz import; ensure ingest
  iterates the full city list. Acceptance: index lists all 302 cities.
- Full SEO: existing title/desc kept; add JSON-LD `ItemList` of cities,
  `BreadcrumbList`, `hreflang` RU↔KK.

## SEO (all page types)

- Unique `<title>`, meta description, `<h1>` per page (templated with city +
  period + variable + value summary).
- `<link rel=canonical>` (self, except the today/tomorrow dedupe above).
- `hreflang` pair RU↔KK on every page; `x-default` → RU.
- `BreadcrumbList` + `WebPage` JSON-LD on every page; index adds `ItemList`.
- **Sitemap index**: replace single `render_weather` with a generated sitemap index
  + sharded `sitemap-weather-N.xml` (≤50k URLs/shard) enumerating the full grammar.
- Dense internal linking across the page graph for crawlability.

## Testing

- **slice** (pure unit): `hours_from_now` count from a fixed "now"; `period_days`
  bounds (week=7, 14-days=14, month=30 with climate tail); `hour_at` lookup;
  `variable_series` shape; `month_with_climate` fills 15–30.
- **routing** (TestClient): 200 + layout-wrapped for each page type; 301 for
  `var/period`→`period/var` and dash→slash date; 404 for `week/18`, bad variable,
  out-of-horizon day.
- **climate**: day-of-year mean computation from synthetic archive rows.
- **index**: contains the three tier sections; renders all seeded cities; no emoji
  in index markup.
- **SEO**: canonical + hreflang present per type; sitemap index lists the expected
  shard count; a shard never exceeds 50k URLs.
- **integration**: in-memory store seeded with a forecast + climate doc; a combo
  page (`/weather/{city}/week/temperature`) contains the value summary.

## Build Order (plan phases)

1. Data model + `weather_ingest` extension (pressure/clouds/uv/sunrise) + `weather_climate` collection & job.
2. `slice.rs` helpers + tests.
3. Routing + dispatch (arity 1–3, redirects, 404s).
4. Renderers per page type + CSS/design.
5. Index redesign (tiers, no emoji) + "all cities" fix.
6. SEO (canonical/hreflang/JSON-LD) + sitemap-index sharding.
7. Wire routes/loaders + deploy.

## Acceptance & Verification (definition of done)

Implementation is complete only when all of the following pass on **prod**
(myqaz.kz, `/opt/myqaz`, port 8082) after deploy:

- **Automatic:** `cargo test` (myqaz + touched crates) green; new slice/routing/
  climate/index/SEO tests green.
- **Build/deploy:** `cargo zigbuild --release --target x86_64-unknown-linux-gnu`,
  ship to `deploy@91.224.74.233:2222`, restart service, health check 200.
- **Manual (prod URLs, RU + KK):** index, city, hourly, hour (`today/18`),
  period, variable hub, combo, day — each returns 200, wrapped in layout, with
  real data (not "—"), correct breadcrumbs, and a visible L2 menu.
- **Redirects/404:** `var/period`→301, dash-date→301, `week/18`→404,
  out-of-horizon day→404.
- **Data presence:** all 302 cities listed on index; spot-check that several
  cities (incl. small ones) have non-placeholder forecasts.
- **Speed:** page TTFB measured (curl `-w`) for index + a deep combo page;
  record numbers; pages served from cache should be fast (target < ~150ms TTFB
  from the server, network aside).
- **Design integrity:** no broken layout on index/city/hourly/hour/period/
  variable/combo/day at desktop + mobile widths; L2 menu present everywhere;
  charts/tables render; emoji rules respected (none on index, present on detail).
  Verified via Playwright screenshots at ≥2 viewport widths.
- **Sitemap:** sitemap index + shards reachable; no shard > 50k URLs; sample URLs
  resolve 200.

## Verification Items (resolve during planning)

- Confirm Open-Meteo returns `surface_pressure, cloud_cover, uv_index` (hourly) and
  `sunrise, sunset, uv_index_max` (daily) in the existing batched request.
- Confirm the ERA5 archive endpoint supports the multi-year daily pull for climate
  normals at acceptable request volume (batched coords, one-shot).
- Confirm `load_cities()` returns 302 on prod (root-cause the missing-cities issue).
- Confirm adapto_app route registration handles the 4 arity patterns without
  collision, and that `:seg` does not shadow the 3-segment date route.
- Decide the 17 oblast-center slugs for the `OBLAST` tier list.
