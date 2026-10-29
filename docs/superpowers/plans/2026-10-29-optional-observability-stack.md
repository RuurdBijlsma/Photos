# Optional Observability Stack (Grafana + Loki) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:
> executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the app durable, structured logs and ship an optional Grafana + Loki stack behind a compose profile so a
self-hoster can enable monitoring with one flag.

**Architecture:** The app gains a second tracing layer that writes JSON lines through a daily-rotating, count-bounded
file appender into `<app_data>/logs/`; the console layer keeps its current text format and filters. The stack side adds
`loki`, `alloy` and `grafana` services under the `monitoring` compose profile, with config files in
`cloud-compose/monitoring/`. Alloy tails the log directory from the shared `app_data` volume (no Docker socket),
promotes `level`/`target` to Loki labels, and Grafana is provisioned with a Loki datasource, a Postgres datasource over
the app's own `jobs` table, and a bundled dashboard.

**Tech Stack:** Rust, `tracing` / `tracing-subscriber` / `tracing-appender`, Docker Compose profiles, Grafana 13.3.0,
Loki 3.7.9, Grafana Alloy v1.21.3.

**Spec:** `docs/superpowers/specs/2026-10-29-optional-observability-stack-design.md`

## Global Constraints

- Pinned images: `grafana/grafana:13.3.0`, `grafana/loki:3.7.9`, `grafana/alloy:v1.21.3`. Never `latest`.
- Compose profile name is exactly `monitoring`. Plain `docker compose up -d` must start only `database`, `backend`,
  `web`.
- No `${VAR:?}` in any profile-gated service: compose interpolates the whole file before profile filtering, so it would
  break a plain `up`. Use `${VAR:-default}`.
- No `${VAR:?}` and no Docker socket mounts anywhere in this feature.
- Loki `3100` and Alloy `12345` stay unpublished; only Grafana publishes a port (`${GRAFANA_PORT:-9476}`).
- Relative config paths: `cloud-compose/compose.yml` uses `./monitoring/...`; the repo-root `compose.yml` uses
  `./cloud-compose/monitoring/...`.
- Rust: edition 2024, `rustfmt` `max_width = 100`, `#![deny(clippy::unwrap_used)]` in production crates, clippy
  pedantic + nursery are errors. Run backend commands from `server/`.
- No SQL changes in this plan, so `server/.sqlx/` must not change and `cargo sqlx prepare` must not be run.
- Conventional commit prefixes: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `perf:`.
- Log file naming is fixed: `<app_data>/logs/photos.YYYY-MM-DD.log` (UTC date), matched by Alloy glob `*.log*`.
- **Where verification runs:** this Windows checkout has no `docker` CLI and no reachable Podman machine, so Tasks 1–2
  verify here with `cargo`, and Tasks 3–7's Docker steps run on the laptop server that actually hosts the deployment (
  substitute `podman compose` only if implementing there). Task 7's staged install must run in a clean directory, not in
  the repo.

## Review Focus

Input classes and failure modes the spec implies that no single task's happy path covers, most likely first:

1. `<app_data>/logs` unwritable (read-only volume, wrong owner) — the app must start and keep logging to the console,
   with one warning, never panic. Owned by Task 1.
2. A settings file without the new `logging.file` block (older or hand-edited `settings.yaml`) — must load with defaults
   instead of failing to start. Owned by Task 1.
3. A log message containing quotes, newlines or unicode — must still produce exactly one valid JSON object per line.
   Owned by Task 1.
4. More matching log files on disk than `max_files` — oldest pruned, newest kept. Owned by Task 1.
5. A log file written before monitoring was enabled (old timestamps) — must not be silently dropped at ingest. Owned by
   Task 3 (config) and Task 4 (verification).

---

### Task 1: App writes JSON logs to a rolling file

**Files:**

- Create: `server/crates/libs/common_services/src/logging.rs`
- Modify: `server/crates/libs/common_services/src/lib.rs:13-22` (add `pub mod logging;`)
- Modify: `server/crates/libs/common_services/Cargo.toml:47` (tracing-subscriber features) and add `tracing-appender`
- Modify: `server/crates/libs/common_services/src/binary_setup.rs`
- Modify: `server/crates/libs/app_state/src/constants.rs` (add `LOGS_FOLDER`)
- Modify: `server/crates/libs/app_state/src/raw_settings.rs:125-147`
- Modify: `server/crates/libs/app_state/src/settings.rs:28-70`
- Modify: `server/crates/libs/app_state/src/load_settings.rs:33-42`
- Modify: `server/config/settings.yaml:80-95`
- Modify: `server/crates/apps/{api,worker,watcher,photos_app,worker_scaler}/src/main.rs`
- Test: unit tests inside `server/crates/libs/common_services/src/logging.rs` and
  `server/crates/libs/app_state/src/raw_settings.rs`

**Interfaces:**

- Consumes: nothing (first task).
- Produces:
    - `app_state::constants::LOGS_FOLDER: &str = "logs"`
    - `app_state::IngestSettings { pub logs_root: PathBuf, .. }`
    - `app_state::FileLoggingSettings { pub enabled: bool, pub filters: LoggingFilterSettings, pub max_files: usize }`,
      `impl Default` =
      `{ enabled: true, filters: main "info" + the same quieted targets as the console block, max_files: 7 }`
    -
    `app_state::LoggingSettings { pub filters: LoggingFilterSettings, #[serde(default)] pub file: FileLoggingSettings }`
    - `common_services::logging::LoggingGuard` —
      `#[must_use] pub struct LoggingGuard(Option<tracing_appender::non_blocking::WorkerGuard>)`
    - `common_services::logging::init_logging(settings: &AppSettings) -> LoggingGuard`
    -
    `common_services::logging::file_layer<S>(dir: &Path, filters: EnvFilter, max_files: usize) -> color_eyre::Result<(impl tracing_subscriber::Layer<S> + Send + Sync, tracing_appender::non_blocking::WorkerGuard)>`
    where `S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a> + Send + Sync`
    - `common_services::binary_setup::setup_binary() -> color_eyre::Result<(AppSettings, PgPool, LoggingGuard)>`

- [ ] **Step 1: Add the dependency and the settings types**

In `common_services/Cargo.toml` change line 47 to
`tracing-subscriber = { version = "0.3.23", features = ["env-filter", "json"] }` and add `tracing-appender = "0.2.5"`.

In `raw_settings.rs`, add
`pub struct FileLoggingSettings { pub enabled: bool, pub filters: LoggingFilterSettings, pub max_files: usize }` plus a
`Default` impl (`enabled: true`, `max_files: 7`, `filters` = `main: "info"` with targets
`ort=warn, hf_hub=error, xet_client=warn, xet_data=warn, xet_runtime=warn, xet=warn, open_clip_inference=warn, object_detector=warn, face_id=warn, sqlx=warn`),
and extend `LoggingSettings` with `#[serde(default)] pub file: FileLoggingSettings`. Keep `to_filter_string` as is.

- [ ] **Step 2: Write the failing tests**

In `raw_settings.rs` (a `#[cfg(test)] mod tests`), write:

```rust
// app_state has no serde_yaml dependency; use the `config` crate the loader
// already uses, so the test parses YAML the same way production does.
fn parse_logging(yaml: &str) -> LoggingSettings {
    let cfg = config::Config::builder()
        .add_source(config::File::from_str(yaml, config::FileFormat::Yaml))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    cfg
}

#[test]
fn logging_settings_default_file_block_when_absent() {
    let raw = parse_logging("filters:\n  main: info\n  targets: {}\n");
    assert!(raw.file.enabled);
    assert_eq!(raw.file.max_files, 7);
    assert_eq!(raw.file.filters.main, "info");
}

#[test]
fn file_filter_string_includes_quieted_targets() {
    let s = FileLoggingSettings::default().filters.to_filter_string();
    assert!(s.starts_with("info,"));
    assert!(s.contains("hf_hub=error"));
    assert!(s.contains("ort=warn"));
}
```

In `logging.rs`:

```rust
#[test]
fn writes_one_json_line_per_event() {
    // build file_layer into a tempdir with max_files = 7, wrap in
    // tracing_subscriber::registry(), emit info!(target: "t1", "a \"quote\"\nand newline") via
    // tracing::subscriber::with_default, drop the guard, then read the single file:
    // exactly 1 line, serde_json parses, level == "INFO", target == "t1",
    // message contains a quote and a newline.
}

#[test]
fn prunes_to_max_files_on_start() {
    // create photos.2026-10-21.log .. photos.2026-10-24.log in a tempdir,
    // build file_layer with max_files = 2, then assert 2 files remain and the
    // two newest names survive.
}

#[test]
fn unwritable_directory_is_an_error_not_a_panic() {
    // point file_layer at a path under a file (e.g. tempdir.join("file").join("logs")
    // after writing "file"), assert it returns Err.
}

#[test]
fn file_names_carry_utc_date() {
    // build file_layer in a tempdir, emit one event, assert exactly one file exists
    // and its name matches ^photos\.\d{4}-\d{2}-\d{2}\.log$
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p common_services --lib logging -- --nocapture` (from `server/`)
Expected: FAIL — `file_layer` does not exist.

- [ ] **Step 4: Implement `logging.rs`**

`file_layer` builds
`tracing_appender::rolling::RollingFileAppender::builder().rotation(Rotation::DAILY).filename_prefix("photos").filename_suffix("log").max_log_files(max_files).build(dir)?`,
wraps it in `tracing_appender::non_blocking(...)` (keep the returned guard), and returns
`filters.and_then(fmt::layer().json().flatten_event(true).with_current_span(true).with_span_list(false).with_ansi(false).with_writer(non_blocking))`.

`init_logging(settings)` builds the console `EnvFilter` (Task 2 changes its source; here use
`settings.logging.filters.to_filter_string()`), then:

- if `settings.logging.file.enabled`:
  `file_layer(&settings.ingest.logs_root, EnvFilter::new(settings.logging.file.filters.to_filter_string()), settings.logging.file.max_files)`;
  on `Ok` install `registry().with(console_filter).with(console_layer).with(file)` and return
  `LoggingGuard(Some(guard))`; on `Err` install console-only, `warn!("File logging disabled: {e}")`, return
  `LoggingGuard(None)`.
- else install console-only and return `LoggingGuard(None)`.

`settings.ingest.logs_root` is `app_data_root.join(LOGS_FOLDER)`; add it in `settings.rs` next to `cache_root` and
`create_dir_all` it in `init_app_data_folders`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p common_services --lib logging -- --nocapture` and `cargo test -p app_state --lib`
Expected: PASS (4 tests + 2 tests).

- [ ] **Step 6: Rewire `setup_binary` and the five binaries**

`setup_binary` becomes:
`color_eyre::install()?; let settings = load_app_settings()?; let guard = init_logging(&settings); let pool = get_db_pool(database_url(), true).await?; Ok((settings, pool, guard))` —
settings must load before the subscriber is installed, because the file sink needs `logs_root`. Delete the `reload`
layer machinery; nothing else used it.

In each of `apps/{api,worker,watcher,photos_app,worker_scaler}/src/main.rs` change
`let (settings, pool) = setup_binary().await?;` to `let (settings, pool, _log_guard) = setup_binary().await?;` and keep
the binding alive for the whole of `main` (name it `_log_guard`, never a bare `_`).

- [ ] **Step 7: Add the setting to the shipped config**

In `server/config/settings.yaml` under `logging:` add:

```yaml
  file:
    enabled: true
    max_files: 7
    filters:
      main: info
      targets:
        ort: warn
        hf_hub: error
        xet_client: warn
        xet_data: warn
        xet_runtime: warn
        xet: warn
        open_clip_inference: warn
        object_detector: warn
        face_id: warn
        sqlx: warn
```

- [ ] **Step 8: Verify by hand, then lint**

Run: `cargo run -p api -- --no-scheduler` from `server/`, wait for the "Server listening" line, then
`curl http://127.0.0.1:5272/api/health`, then inspect the newest `<app_data>/logs/photos.*.log`: every line parses as
JSON, `level`/`target`/`timestamp`/`fields.message`-or-`message` present, and the console output is still the old text
format.

Run: `cargo fmt --all -- --check` and `cargo clippy --all-features --tests`
Expected: clean.

- [ ] **Step 9: Commit**

```bash
git add server/crates/libs/common_services server/crates/libs/app_state server/config/settings.yaml server/crates/apps
git commit -m "feat: write JSON logs to a rolling file under app_data"
```

---

### Task 2: Console filter precedence, INFO access log, no stray `println!`

**Files:**

- Modify: `server/crates/libs/common_services/src/logging.rs` (console filter source)
- Modify: `server/crates/apps/api/src/server.rs:49-55`
- Modify: `server/crates/apps/worker/src/jobs/management.rs:253-262`
- Modify: `server/crates/libs/common_services/src/api/auth/error.rs:9,54`
- Test: unit test in `server/crates/libs/common_services/src/logging.rs`

**Interfaces:**

- Consumes: `init_logging` from Task 1.
- Produces: `pub fn console_directives(rust_log: Option<&str>, configured: &str) -> String` in
  `common_services::logging`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn rust_log_overrides_settings_for_console() {
    assert_eq!(console_directives(Some("debug"), "info,api=debug"), "debug");
    assert_eq!(console_directives(None, "info,api=debug"), "info,api=debug");
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p common_services --lib console_directives`
Expected: FAIL — function not found.

- [ ] **Step 3: Implement precedence and use it**

`console_directives` returns `rust_log.unwrap_or(configured).to_string()`. In `init_logging`, build the console filter
with `console_directives(std::env::var("RUST_LOG").ok().as_deref(), &settings.logging.filters.to_filter_string())`. The
file sink keeps using settings only.

- [ ] **Step 4: Raise the access log to INFO**

In `apps/api/src/server.rs` replace `.layer(TraceLayer::new_for_http().on_request(()))` with

```rust
.layer(
TraceLayer::new_for_http()
.make_span_with(DefaultMakeSpan::new().level(Level::INFO))
.on_request(())
.on_response(DefaultOnResponse::new().level(Level::INFO)),
)
```

importing `tower_http::trace::{DefaultMakeSpan, DefaultOnResponse}` and `tracing::Level`. `on_request(())` stays: it
suppresses tower-http's own "started processing request" event so each request is logged once, on response, with status
and latency.

- [ ] **Step 5: Remove the two `println!` calls**

`jobs/management.rs`: drop `println!("{last_error}");` and fold the error into the existing warning —
`warn!("⚠️ Rescheduling job {job_id}. Backoff: {backoff_secs}s: {last_error}");`.

`api/auth/error.rs`: change `AuthError::Internal(e) => println!("Error in /auth: {e:?}")` to
`error!("Error in /auth: {e:?}")` and add `error` to the existing `use tracing::{info, warn};` import.

- [ ] **Step 6: Verify**

Run: `cargo test -p common_services --lib`, then `cargo run -p api -- --no-scheduler` from `server/` and hit
`/api/health` a few times. Expected: console shows one `INFO` line per request with method/uri/status/latency; the JSON
file shows the same request lines only if the file filter is at `info` (it is);
`RUST_LOG=debug cargo run -p api -- --no-scheduler` shows debug lines the settings would have suppressed.

Run: `cargo fmt --all -- --check`, `cargo clippy --all-features --tests`.

- [ ] **Step 7: Commit**

```bash
git add server/crates
git commit -m "feat: log requests at info and let RUST_LOG win over settings"
```

---

### Task 3: Loki service and config

**Files:**

- Create: `cloud-compose/monitoring/loki/config.yaml`
- Modify: `cloud-compose/compose.yml` (new service + volume)
- Modify: `compose.yml` (same service, `./cloud-compose/...` paths)

**Interfaces:**

- Consumes: nothing.
- Produces: service `loki` reachable at `http://loki:3100` from the compose network; volume `photos_loki_data`; env
  `LOKI_RETENTION_DAYS` (default `30`).

- [ ] **Step 1: Write the Loki config**

`cloud-compose/monitoring/loki/config.yaml`:

```yaml
auth_enabled: false

server:
  http_listen_port: 3100
  grpc_listen_port: 9096

common:
  path_prefix: /loki
  replication_factor: 1
  ring:
    kvstore:
      store: inmemory

storage_config:
  tsdb_shipper:
    active_index_directory: /loki/tsdb-index
    cache_location: /loki/tsdb-cache
  filesystem:
    directory: /loki/chunks

schema_config:
  configs:
    - from: "2024-04-01"
      store: tsdb
      object_store: filesystem
      schema: v13
      index:
        prefix: index_
        period: 24h

limits_config:
  retention_period: "${LOKI_RETENTION_DAYS:-30}d"
  reject_old_samples: false

compactor:
  working_directory: /loki/compactor
  compaction_interval: 10m
  retention_enabled: true
  delete_request_store: filesystem
  retention_delete_delay: 2h
  retention_delete_worker_count: 150
```

`retention_period` lives only here; the compactor enforces it and refuses to start without `delete_request_store`.
`reject_old_samples: false` is deliberate: with `tail_from_end = false` in Task 4 a user's pre-existing log files get
shipped, and old timestamps must not be dropped.

- [ ] **Step 2: Add the service to both compose files**

In `cloud-compose/compose.yml` (and identically in the repo-root `compose.yml`, only the config path differs):

```yaml
  loki:
    image: grafana/loki:3.7.9
    container_name: photos_loki
    restart: unless-stopped
    profiles: [ "monitoring" ]
    command:
      - -config.file=/etc/loki/config.yaml
      - -config.expand-env=true
    environment:
      LOKI_RETENTION_DAYS: ${LOKI_RETENTION_DAYS:-30}
    volumes:
      - ./monitoring/loki/config.yaml:/etc/loki/config.yaml:ro
      - photos_loki_data:/loki
```

Root copy uses `./cloud-compose/monitoring/loki/config.yaml:/etc/loki/config.yaml:ro`. Add `photos_loki_data:` to both
`volumes:` blocks. No `ports:`.

- [ ] **Step 3: Verify the config parses**

Run:
`docker run --rm -v "${PWD}/cloud-compose/monitoring/loki/config.yaml:/etc/loki/config.yaml:ro" grafana/loki:3.7.9 -config.file=/etc/loki/config.yaml -config.expand-env=true -verify-config`
Expected: exits 0 with a config summary. Then
`docker compose -f cloud-compose/compose.yml --env-file cloud-compose/.env config >/dev/null` (create
`cloud-compose/.env` as a copy of `example.env`, gitignored) — expected: no output, exit 0.

- [ ] **Step 4: Verify it starts and the plain install is unaffected**

Run from a scratch dir: `docker compose up -d` with only `compose.yml` + `.env` → `docker ps` shows three containers and
no `photos_loki`. Then `docker compose --profile monitoring up -d` → `photos_loki` is up, `docker compose logs loki`
shows no config errors, and `curl -s localhost:3100/ready` is not reachable from the host (port unpublished) — do the
readiness check with `docker exec photos_loki wget -qO- localhost:3100/ready`.

- [ ] **Step 5: Commit**

```bash
git add cloud-compose/compose.yml compose.yml cloud-compose/monitoring/loki/config.yaml
git commit -m "feat: add loki to the optional monitoring profile"
```

---

### Task 4: Alloy ships the JSON logs

**Files:**

- Create: `cloud-compose/monitoring/alloy/config.alloy`
- Modify: `cloud-compose/compose.yml`, `compose.yml` (alloy service + volume)

**Interfaces:**

- Consumes: Task 1's file format (`photos.YYYY-MM-DD.log`, JSON with `level`, `target`, `timestamp`) and Task 3's `loki`
  service.
- Produces: labels `service="photos"`, `job="photos-backend"`, `level`, `target` in Loki; volume `photos_alloy_data`.

- [ ] **Step 1: Write the Alloy config**

```alloy
local.file_match "photos_logs" {
  path_targets = [
    {
      __path__         = "/app_data/logs/*.log*",
      service          = "photos",
      job              = "photos-backend",
    },
  ]
  sync_period = "10s"
}

loki.source.file "photos_logs" {
  targets       = local.file_match.photos_logs.targets
  forward_to    = [loki.process.photos_logs.receiver]
  tail_from_end = false
}

loki.process "photos_logs" {
  forward_to = [loki.write.loki.receiver]

  stage.json {
    expressions = {
      level  = "level",
      target = "target",
    }
  }

  stage.labels {
    values = {
      level  = "",
      target = "",
    }
  }
}

loki.write "loki" {
  endpoint {
    url = "http://loki:3100/loki/api/v1/push"
  }
}
```

`tail_from_end = false` on purpose: the app's rotation creates a *new* file each day rather than renaming, so a fresh
path with no stored position must be read from the beginning; re-shipping on restart is prevented by the positions file,
and the one-time backfill of existing files is desirable.

- [ ] **Step 2: Add the service**

```yaml
  alloy:
    image: grafana/alloy:v1.21.3
    container_name: photos_alloy
    restart: unless-stopped
    profiles: [ "monitoring" ]
    command:
      - run
      - /etc/alloy/config.alloy
      - --storage.path=/var/lib/alloy/data
    volumes:
      - ./monitoring/alloy/config.alloy:/etc/alloy/config.alloy:ro
      - photos_alloy_data:/var/lib/alloy/data
      - ${APP_DATA_LOCATION:-photos_app_data}:/app_data:ro
    depends_on:
      - loki
```

Same in the root file with `./cloud-compose/monitoring/alloy/config.alloy` and `photos_alloy_data` added to `volumes:`.
No `ports:`. Do not override the default `--server.http.listen-addr`.

- [ ] **Step 3: Verify lines arrive**

With the app running under docker (backend writes to the same `photos_app_data` volume), run
`docker compose --profile monitoring up -d`, then `docker logs photos_alloy` for parse errors. Query Loki:

```bash
docker exec photos_loki wget -qO- 'http://localhost:3100/loki/api/v1/query_range?query=%7Bjob%3D%22photos-backend%22%7D&limit=5'
```

Expected: `"status":"success"` and at least one entry whose `stream` labels include `level` and `target`.

- [ ] **Step 4: Verify the position file and old-timestamp backfill (Review Focus 5)**

`docker exec photos_alloy ls -l /var/lib/alloy/data/loki.source.file/positions.yml` — exists. Restart the alloy
container and confirm no duplicate burst (query `count_over_time` over the last minute stays flat).

Then copy a hand-made old file into the log dir: `docker cp` a file named `photos.2026-01-01.log` containing one JSON
line with `"timestamp":"2026-01-01T00:00:00Z"` into the backend container's `/app_data/logs/`, wait ~20s, and query for
it (`{job="photos-backend"} | json | timestamp="2026-01-01T00:00:00Z"`). Expected: the line is present, proving
`reject_old_samples: false` works.

- [ ] **Step 5: Commit**

```bash
git add cloud-compose/compose.yml compose.yml cloud-compose/monitoring/alloy/config.alloy
git commit -m "feat: ship app logs to loki with alloy"
```

---

### Task 5: Grafana service and provisioning

**Files:**

- Create: `cloud-compose/monitoring/grafana/provisioning/datasources/loki.yml`
- Create: `cloud-compose/monitoring/grafana/provisioning/datasources/postgres.yml`
- Create: `cloud-compose/monitoring/grafana/provisioning/dashboards/photos.yml`
- Create: `cloud-compose/monitoring/grafana/dashboards/photos-logs.json`
- Modify: `cloud-compose/compose.yml`, `compose.yml` (grafana service + volume)
- Modify: `example.env` (Grafana vars)

**Interfaces:**

- Consumes: `loki` (Task 3), `database` (existing), `DB_USER`/`DB_PASSWORD`/`DB_NAME`.
- Produces: datasource uids `loki` and `postgres`; dashboard uid `photos-logs`; volume `photos_grafana_data`; env
  `GRAFANA_PORT` (9476), `GRAFANA_ADMIN_PASSWORD` (admin).

- [ ] **Step 1: Write the three provisioning files**

`datasources/loki.yml`: `apiVersion: 1`, one datasource
`name: Loki, uid: loki, type: loki, url: http://loki:3100, isDefault: true`.

`datasources/postgres.yml`:
`name: PostgreSQL, uid: postgres, type: postgres, url: database:5432, user: $__env{POSTGRES_USER}`,
`jsonData: { database: $__env{POSTGRES_DB}, sslmode: disable }`,
`secureJsonData: { password: $__env{POSTGRES_PASSWORD} }`.

`dashboards/photos.yml`: `apiVersion: 1`, provider
`name: photos, folder: Photos, type: file, updateIntervalSeconds: 30, allowUiUpdates: false, options: { path: /var/lib/grafana/dashboards }`.

- [ ] **Step 2: Write a minimal dashboard**

`dashboards/photos-logs.json` — raw dashboard model (no `{"dashboard": ...}` wrapper), `uid: "photos-logs"`,
`title: "Photos backend logs"`, `schemaVersion: 39`, `refresh: "30s"`, one `logs` panel bound with
`"datasource": { "type": "loki", "uid": "loki" }` and query `{service="photos", job="photos-backend"}`. Task 6 adds the
rest.

- [ ] **Step 3: Add the service and env vars**

```yaml
  grafana:
    image: grafana/grafana:13.3.0
    container_name: photos_grafana
    restart: unless-stopped
    profiles: [ "monitoring" ]
    ports:
      - "${GRAFANA_PORT:-9476}:3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_ADMIN_PASSWORD:-admin}
      POSTGRES_USER: ${DB_USER:-photos_user}
      POSTGRES_PASSWORD: ${DB_PASSWORD:-dev-password}
      POSTGRES_DB: ${DB_NAME:-photos}
    volumes:
      - ./monitoring/grafana/provisioning:/etc/grafana/provisioning:ro
      - ./monitoring/grafana/dashboards:/var/lib/grafana/dashboards:ro
      - photos_grafana_data:/var/lib/grafana
    depends_on:
      - loki
      - database
```

Root file uses `./cloud-compose/monitoring/grafana/...`; add `photos_grafana_data`. Append to `example.env`:

```
# Optional monitoring stack (docker compose --profile monitoring up -d)
GRAFANA_PORT=9476
GRAFANA_ADMIN_PASSWORD=admin
LOKI_RETENTION_DAYS=30
```

- [ ] **Step 4: Verify provisioning on first boot**

From a staged dir (compose.yml + .env + monitoring/), `docker compose --profile monitoring up -d`, then:

```bash
curl -s -u admin:admin localhost:9476/api/health
curl -s -u admin:admin localhost:9476/api/search?query= | grep photos-logs
curl -s -u admin:admin localhost:9476/api/datasources/uid/loki/health
```

Expected: `"database":"ok"`, the dashboard uid appears in search, Loki health `"status":"OK"`. Also
`curl -s -u admin:admin localhost:9476/api/datasources` shows `postgres` provisioned with `password` absent from the
response.

- [ ] **Step 5: Verify the plain install is still three containers**

`docker compose down` then a plain `docker compose up -d`: `docker ps` shows only database/backend/web, and
`docker compose config` exits 0 with `example.env` values (proves no `${VAR:?}`).

- [ ] **Step 6: Commit**

```bash
git add cloud-compose/compose.yml compose.yml cloud-compose/monitoring/grafana example.env
git commit -m "feat: provision grafana with loki and postgres datasources"
```

---

### Task 6: Operations dashboard

**Files:**

- Modify: `cloud-compose/monitoring/grafana/dashboards/photos-logs.json` (add panels; rename title to
  `Photos operations`)

**Interfaces:**

- Consumes: datasource uids `loki`, `postgres` from Task 5; tables `jobs` (`status`, `job_type`, `attempts`,
  `created_at`, `finished_at`, `last_error`) and `media_item` (`analysis_status`).
- Produces: nothing consumed by later tasks.

- [ ] **Step 1: Add the Postgres panels**

Five panels, each with `"datasource": { "type": "postgres", "uid": "postgres" }` and `"format": "time_series"` unless
noted:

- Queue depth:
  `SELECT $__timeGroupAlias(created_at,$__interval), count(*) FILTER (WHERE status='queued') AS queued, count(*) FILTER (WHERE status='running') AS running FROM jobs WHERE $__timeFilter(created_at) GROUP BY 1 ORDER BY 1`
- Failed jobs per interval (bars):
  `SELECT $__timeGroupAlias(finished_at,$__interval), count(*) FROM jobs WHERE status='failed' AND $__timeFilter(finished_at) GROUP BY 1 ORDER BY 1`
- Jobs finished per interval by type (stacked):
  `SELECT $__timeGroupAlias(finished_at,$__interval), job_type, count(*) FROM jobs WHERE status='done' AND $__timeFilter(finished_at) GROUP BY 1,2 ORDER BY 1`
- Failure reasons (table, `"format": "table"`):
  `SELECT last_error, count(*) AS n, max(finished_at) AS last_seen FROM jobs WHERE status='failed' AND $__timeFilter(finished_at) GROUP BY last_error ORDER BY n DESC LIMIT 20`
- Failed analyses (stat): `SELECT count(*) FROM media_item WHERE analysis_status='failed'`

- [ ] **Step 2: Add the Loki panels**

- Error rate (time series): `sum(count_over_time({job="photos-backend", level="ERROR"} [$__interval]))`
- Log stream (logs panel): `{job="photos-backend"}` with `| json` added when filtering by field.

- [ ] **Step 3: Verify against a live stack**

`docker compose --profile monitoring up -d`, wait for ingestion to produce rows, open
`http://localhost:9476/d/photos-logs`, and confirm every panel renders without `No data` errors caused by SQL (an empty
interval is fine; a red error is not). Check `/api/ds/query` returns 200 for each panel's refId by reading
`docker logs photos_grafana` — expected: no datasource errors.

- [ ] **Step 4: Commit**

```bash
git add cloud-compose/monitoring/grafana/dashboards/photos-logs.json
git commit -m "feat: add photos operations dashboard"
```

---

### Task 7: Release packaging, docs, end-to-end check

**Files:**

- Modify: `.github/workflows/docker-publish.yml:177-183`
- Modify: `README.md:87` (append section)
- Create: `cloud-compose/monitoring/README.md`
- Modify: `.gitignore` (ignore `monitoring.zip`)

**Interfaces:**

- Consumes: everything above.
- Produces: release asset `monitoring.zip` whose contents are `monitoring/...` at the archive root.

- [ ] **Step 1: Package the stack in CI**

Before the release-upload step add:

```yaml
      - name: Package monitoring stack
        working-directory: cloud-compose
        run: zip -r ../monitoring.zip monitoring
```

and add `monitoring.zip` to the `files:` list of the `softprops/action-gh-release` step list alongside
`cloud-compose/compose.yml` and `example.env`.

- [ ] **Step 2: Write the two READMEs**

`cloud-compose/monitoring/README.md`: what the three containers do, `docker compose --profile monitoring up -d`, Grafana
URL and `GRAFANA_ADMIN_PASSWORD` (including the `$`-in-password caveat for provisioning), `LOKI_RETENTION_DAYS`, that
Loki/Alloy have no published port and no auth, how to disable (
`docker compose stop photos_loki photos_alloy photos_grafana`), where logs live (`<app_data>/logs`, 7 daily files,
`RP__LOGGING__FILE__ENABLED=false` to disable), the `podman compose` caveat, and that Grafana holds DB credentials.

Root `README.md`: after step 9, an "Optional: monitoring" section — download `monitoring.zip` beside `compose.yml`,
unzip, `docker compose --profile monitoring up -d`, open `http://localhost:9476`.

- [ ] **Step 3: Staged end-to-end verification (the real install path)**

In an empty temp dir: copy `cloud-compose/compose.yml`, `example.env` as `.env` (pointing `MEDIA_LOCATION` at a small
real folder, `JWT_SECRET` set), and unzip `monitoring.zip`. Then `docker compose --profile monitoring up -d`. Expected:
six containers up; `curl -s -u admin:admin localhost:9476/api/health` ok; the dashboard lists `Photos operations`; blog
lines from the backend are visible in the Loki log panel.

Then `docker compose down && docker compose up -d` in the same dir: only three containers (proves the profile default),
and the existing `<app_data>` logs are still there.

- [ ] **Step 4: Lint everything one last time**

From `server/`: `cargo fmt --all -- --check`, `cargo clippy --all-features --tests`. From the repo root: `git status`
clean of generated files, `server/.sqlx/` untouched.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/docker-publish.yml README.md cloud-compose/monitoring/README.md .gitignore
git commit -m "docs: document and package the optional monitoring stack"
```
