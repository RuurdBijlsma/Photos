# AGENTS.md

Self-hosted Google Photos alternative ("Ruurd Photos"): a Rust backend (axum, sqlx/Postgres + pgvector, ONNX models)
and a Vue 3 + Vuetify web client. This file tells coding agents how the repo is laid out and how to build, test, and
change it.

There is effectively one developer (Ruurd), so a few tracked files deliberately contain this machine's paths: the
`ORT_DYLIB_PATH` and `DATABASE_URL` exports in `server/justfile` and `server/scripts/all.ps1`, and the media/app-data
folders in `server/config/settings.yaml`. Don't portabilize or "clean up" those — if your setup differs, override with
env vars (`ORT_DYLIB_PATH`, `DATABASE_URL`, `RP__INGEST__MEDIA_FOLDER`, `RP__INGEST__APP_DATA_FOLDER`).

## Repository layout

| Path                        | Contents                                                             |
|-----------------------------|----------------------------------------------------------------------|
| `server/`                   | Rust workspace (13 crates), migrations, settings, SQLx offline cache |
| `web/`                      | Vue 3 + TypeScript SPA (Vite, Pinia, Vuetify)                        |
| `compose.yml`               | Production stack: pgvector DB + backend + nginx web                  |
| `cloud-compose/compose.yml` | Compose file attached to GitHub releases                             |
| `.github/workflows/`        | Rust CI, Web CI, frontend perf gate, release images, GitHub Pages    |
| `scripts/`                  | Windows helpers (start Postgres, copy app data into a Podman volume) |

### Server crates

- `apps/photos_app` — the binary used by Docker and the integration tests: runs API + worker scaler + watcher in one
  process (toggle with `--no-api`, `--no-scaler`, `--no-watcher`, `--no-scheduler`).
- `apps/api` — axum HTTP API. Routers/handlers in `src/routes/<domain>/`, wiring in `src/routes/mod.rs`.
- `apps/worker` — job executor. Dispatch table: `src/handlers/mod.rs` (`JobType` → handler module).
- `apps/worker_scaler` — spawns/retires worker processes from queued job demand and system memory
  (`config/settings.yaml` → `scaler.profiles`).
- `apps/watcher` — `notify`-based media folder watcher; enqueues ingest/removal jobs.
- `libs/common_services` — business logic (`src/api/<domain>/service.rs` + `interfaces.rs`), DB tables/stores, job
  queue, `AppError`.
- `libs/app_state` — settings/constants loading (`config/settings.yaml`, `RP__*` env overrides).
- `libs/common_types` — shared types and protobuf codegen (`proto/timeline.proto` compiled by `build.rs` via prost).
- `libs/model_provider`, `libs/ml_analysis` — lazy ONNX model loading and visual analysis (CLIP embeddings, faces,
  objects, colors).
- `libs/generate_thumbnails` — AVIF thumbnails, video transcodes, panoramas.
- `libs/tasks` — scheduled task runner.
- `test_integration` — end-to-end API/ingest suite.

## Supporting crates

The heavy media and ML work lives in separate crates, checked out beside this repo (`../media_analyzer_rs`,
`../open-clip-rs`, `../face_id-rs`, …) and consumed here as published crates.io versions — there are no `path` or `git`
dependencies. Fix or extend the behavior there, bump the version here, and keep the `crates/libs` wrappers thin.

- `media_analyzer` — metadata/EXIF extraction, resolved timestamps and timezones, offline geolocation, historical
  weather and sun times; shells out to the `exiftool` binary. Used by `common_services` (ingest, `media_item` tables),
  `worker` (ingest/takeout handlers), `worker_scaler`, `test_integration`.
- `exiftool` — ExifTool wrapper for writing tags; orientation and datetime are written through it in
  `common_services/src/utils.rs` and `generate_thumbnails/src/motion_photo.rs`.
- `meteostat` — historic weather from weather stations. Not a direct dependency: `media_analyzer` uses it and the
  result lands in the `weather` table.
- `open_clip_inference` — CLIP image/text embeddings on ONNX Runtime; drives search and photo clustering. Used by
  `model_provider` (lazy model loading), `ml_analysis`, `common_services` (search), `api`, `worker`.
- `face_id` — face detection (boxes, landmarks) plus face embeddings. Used by `ml_analysis`, `common_types` (shared
  types), `common_services` (faces table), `worker` (`cluster_faces`).
- `object_detector` — YOLOE-26-seg detection: a tag, pixel mask and box per object, 4000+ object vocabulary. Used by
  `ml_analysis`, `common_types`, `common_services` (objects table).
- `material-color-utils` — Material You color: prominent colors from an image and theme palettes generated from a seed
  color. Used by `ml_analysis` (`color_data.rs`), `app_state` (theme `variant` setting), `common_services`/`api`
  (theme routes).
- `panorama-tiler` — turns equirectangular/cylindrical panoramas into multi-resolution cubemap tiles
  (`generate_thumbnails/src/pano.rs`).

Production deployment is not built here: the `compose-services` repo (`stacks/photos/compose.yml`) runs the published
`ghcr.io/ruurdbijlsma/photos-server` and `photos-web` images and joins the external `proxy-net`; the root `compose.yml`
in this repo builds from source for local runs.

## Prerequisites

- Rust stable (edition 2024; Docker builds with Rust 1.98)
- Node 20.19+ or 22.12+ with npm (lockfile: `web/package-lock.json`)
- PostgreSQL 18 with pgvector (Podman/Docker is fine)
- `exiftool`, `ffmpeg`, `protoc`, `sqlx-cli`, `nasm`, plus libheif and ONNX Runtime native libs — see `server/README.md`
  for Windows (vcpkg libheif) and Linux installs.

## Setup

Backend (from `server/`):

```powershell
cd server
cp example.env .env          # DATABASE_URL + RP__SECRETS__JWT
./scripts/start_postgres.ps1 # Podman: pgvector on localhost:5432, photos_user/dev-password, db "photos"
cargo sqlx migrate run       # optional; the binaries also run migrations on startup
./scripts/recreate_db.ps1    # or: drop/recreate/migrate the dev db and seed the dev user
```

`config/settings.yaml` is resolved relative to the current directory, so run binaries from `server/`, not the repo
root. Override any setting with `RP__SECTION__KEY` env vars (`__` = nesting), e.g.
`RP__SCALER__PROFILES__LIGHT__MAX_WORKERS=5`; Docker uses this to point media/app data at `/media` and `/app_data`.
ML models are downloaded from Hugging Face on first use into `<app_data>/cache/huggingface`.

Frontend (from `web/`):

```powershell
cd web
npm ci
npm run dev   # http://localhost:5173; proxies /api, /thumbnails, /hosted to 127.0.0.1:5272
```

## Build and run

GENERALLY when running, use release profile, otherwise crates like `image` slow down and tests/running takes minutes
instead of seconds. Also using release profile will take advantage of incremental compilation, since I do all
runs/builds with release.

```powershell
# Backend (all from server/)
cargo run -p photos_app                       # full stack in one process
cargo run -p api -- --no-scheduler            # API only
cargo run -p worker -- --exclude <job_type>   # worker only
cargo run -p watcher                          # watcher only
cargo build --release -p photos_app           # release binary
just setup | just run api | just check        # justfile shortcuts (see server/justfile)

# Frontend (from web/)
npm run dev          # Vite dev server
npm run build        # type-check + build (run-p lint:type-check build-only)
npm run build-only   # build without type-check
npm run preview      # serve dist/ on :4173, proxies to :5272
```

Docker: `docker compose up -d` needs `MEDIA_LOCATION` and `JWT_SECRET` in the root `.env`; the UI is on port 9475.

## Testing

Only tests that really matter are the integration tests.

```powershell
# Fast loop, no database or models needed (from server/):
cargo fmt --all -- --check
cargo clippy --all-features

# End-to-end suite (from server/): creates and drops database "test_db", starts photos_app in-process,
# copies crates/test_integration/assets/media_dir, needs local Postgres, exiftool and model downloads.
cargo test -p test_integration -- --nocapture

# Local full run (release profile + dynamically loaded ONNX Runtime; CI runs no tests at all)
just test    # = cargo test --profile release --features load-dynamic -- --nocapture

# Benchmarks (need a populated local DB; from server/)
cargo bench --bench system_bench --features load-dynamic
cargo bench --bench timeline_benchmark --features load-dynamic
```

Without `DATABASE_URL`, `cargo build`/`clippy` resolve SQL through the committed `.sqlx` cache, so SQL that has no
cache entry fails to compile until you point `DATABASE_URL` at a migrated dev DB.

The web app has no unit-test suite. Frontend performance is guarded by `web/tests/perf` (Playwright):

```powershell
cd web
npm run build-only
npm run preview   # terminal A, proxies /api to a running backend on :5272
npm run perf      # terminal B: 5 measured runs, writes tests/perf/results/<date>_<branch>_<sha>.json
```

Commit that JSON file — CI's perf gate rejects PRs touching `web/**` without a new result file and compares it to the
newest result on `main`. Details: `web/tests/perf/README.md`.

## Linting and formatting

Run linting whenever you want. Never revert auto formatting changes.

```powershell
# Backend (from server/)
just fmt     # cargo fmt --all
just clippy  # --all-features --tests --benches, clippy::pedantic + nursery denied

# Frontend (from web/)
npm run lint
```

`./scripts/all.ps1` is not a plain lint: it ends with `cargo sqlx prepare` (see the SQLx rule below) and also runs
`cargo doc`. Only run it when you want those side effects.

## Code style

- **Rust**: `rustfmt` with `max_width = 100` (`server/.rustfmt.toml`). Clippy `pedantic` + `nursery` are errors. Every
  production crate has `#![deny(clippy::unwrap_used)]` — return `Result`/`Option` and use `?`; each crate's `lib.rs`
  lists deliberate allow-lists (e.g. `large_futures`, lossy casts), but only for lints that are reasonable to ignore
  crate-wide. Avoid panics, but in tests, or some exception cases, we can use `expect("..")`. Never `unwrap()`.
- **API layering**: keep `apps/api/src/routes/**` handlers thin; put logic and request/response DTOs in
  `common_services/src/api/<domain>/`, and SQL in `common_services/src/database/{tables,stores}`. HTTP failures go
  through `AppError`.
- **SQLx**: queries are compile-time checked. Before creating a merge request, regenerate the offline cache against a
  live DB (`./scripts/update_sqlx.ps1`, i.e. `cargo sqlx prepare --workspace`) and commit `server/.sqlx/`; CI builds
  with `SQLX_OFFLINE=true`. Otherwise the CI can't pass. It's not needed to `update_sqlx` more often than that, it'll
  just waste time to run it for every change/commit. As an AI, generally, you never run this, unless explicitely asked,
  or when asked to prepare for a MR.
- **Web**: Prettier — no semicolons, single quotes, 100 columns, 2-space indent. Use the `@/` alias for `src/`. API
  access lives in `src/scripts/services/*` (shared axios client with refresh queue), state in Pinia stores
  (`src/scripts/stores`), hand-written DTOs in `src/scripts/types/api`, components/views in `src/vues`.
- **Protobuf**: `server/crates/libs/common_types/proto/timeline.proto` is compiled into Rust (`common_types::pb`);
  `web/src/proto/timeline.proto` is the same schema for ts-proto. Change both, run `npm run proto:gen`, and commit
  `web/src/scripts/types/generated/timeline.ts`. Never hand-edit generated code.

## Adding features

- **API endpoint**: router + handler in `apps/api/src/routes/<domain>/`, logic in `common_services/src/api/<domain>/`,
  a service function in `web/src/scripts/services/`, and a type in `web/src/scripts/types/api/`. If the endpoint is
  hot (timeline, search, albums, people, camera, explore, storage), consider the protobuf path (`Protobuf<T>` responses
  decoded with ts-proto in `web/src/scripts/services`).
- **Job type**: add the variant to `JobType` in `common_services/src/database/jobs.rs`, add a handler module under
  `apps/worker/src/handlers/`, and wire it in `handlers/mod.rs`. JobType is a postgres enum, so an SQL migration is
  needed. SQL migration files are located at `server/migrations`.
- **Setting**: add the field to `app_state` (`raw_settings.rs` + `settings.rs`) and to `config/settings.yaml`; expose it
  through the frontend settings/admin API if users should change it at runtime.

## CI and PRs

- `Rust CI` (`.github/workflows/server-quality.yml`): `cargo fmt --check`, `cargo clippy --no-deps` with `-D
  clippy::all -D clippy::pedantic -D clippy::nursery`, and `cargo build` with `SQLX_OFFLINE=true`. It runs no tests, so
  a failing test passes CI; and it builds for Linux, where `load-dynamic` isn't needed — every build on this Windows
  machine requires it, so breakage that only appears under `--features load-dynamic` passes CI too. `just test` + `just
  clippy` locally are the real gate.
- `Web CI` (`web-quality.yml`): `npm ci`, `lint:oxlint`, `lint:eslint`, `build`, `prettier:check`.
- `Frontend Performance Regression` (`perf-gate.yml`): every PR touching `web/**` must add a perf result file.
- Releases publish multi-arch Docker images and attach `cloud-compose/compose.yml` + `example.env`; `pages.yml`
  deploys the web client as a demo (built with `GITHUB_PAGES=true`).
- Branch from `develop`; PRs target `main`. History uses conventional prefixes (`feat:`, `fix:`, `chore:`,
  `refactor:`, `docs:`, `perf:`) - follow that. Keep committed artifacts in sync (`.sqlx/`, generated proto, perf
  result) in the same PR as the change that caused them.

## Gotchas

- Run backend commands from `server/` — `config/settings.yaml` and `migrations/` are resolved from the CWD.
- SQL is compile-time checked: `just` exports `DATABASE_URL`, so queries are validated against the dev DB — after a
  migration or a new query, a stale schema means the crate won't compile. Recreate the dev DB freely:
  `./scripts/recreate_db.ps1` drops it, migrates and re-seeds the dev user, and there is never data worth keeping.
  `./scripts/disconnect_postgres.ps1` clears blocking connections.
- The integration suite is destructive to the `test_db` database and slow on first run (model downloads).
- `--features load-dynamic` requires `ORT_DYLIB_PATH` pointing at `onnxruntime`; the default build links it statically.
- In Docker the backend runs only `photos_app`; do not start `api`/`watcher`/`worker` as separate containers.
- Thumbnails and the Hugging Face cache live under `app_data/` (gitignored). The API serves them at `/thumbnails` and
  `/hosted` when `api.serve_static_assets` is true; nginx does it in compose.
- Manual API poking: Bruno collection in `server/bruno/ruurd_photos`. The first `POST /api/auth/register` on an empty
  DB needs no invite and creates the admin user; every later registration needs an invite token. After
  `recreate_db.ps1` a dev user already exists (`common_types::dev_constants` — `ruurd@example.com` / `kibbeling`), so
  `POST /api/auth/login` is the quick path; it sets the auth cookies. `/api/health` is public.
- On Windows, `load-dynamic` is required. Default bundled onnxruntime is not recent enough.
- Supporting crates are pulled from crates.io by version. To try an unpublished change, publish and bump the version
  here (each version is repeated across several `apps/*` and `libs/*` manifests, so update all of them), or add a
  temporary `[patch.crates-io]` entry in `server/Cargo.toml` — remove it (and the `Cargo.lock` churn) before the MR.
- A dependency that links ONNX Runtime must have `load-dynamic` forwarded through its wrapper crate and every app that
  pulls it in (`ml_analysis`, `model_provider`, `common_types`, `common_services`, `tasks`, `api`, `worker`,
  `photos_app`); `just check` runs clippy with `--all-features`, so a missing forward fails linting, not only tests.
