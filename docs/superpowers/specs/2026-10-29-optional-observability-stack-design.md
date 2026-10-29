# Optional observability stack (Grafana + Loki) — design

Date: 2026-10-29
Status: approved pending review
Owner: Ruurd

## Intent

The app currently logs to stdout only, so the sole way to inspect a production
instance is `docker logs`. That has two costs: logs vanish when a container is
recreated, and there is no way to search or chart them. Meanwhile the app already
records a lot of operational truth in Postgres (`jobs`, `media_item`,
`analysis_error`) that is only visible inside its own admin UI.

The goal is a first-class, *optional* observability stack that ships in this
repository: JSON logs written by the app, shipped by Grafana Alloy into Loki,
viewed in Grafana, alongside dashboards built directly from the Postgres data the
app already writes. It must be off by default, add no burden to a plain
`docker compose up`, and be usable by anyone who self-hosts this project — not
just by the author.

## Goals

- Logs survive container replacement and are searchable by level, target and
  time, with the message body queryable as a field.
- Log files are bounded: daily rotation with a configurable number of files kept.
- Enabling monitoring for a self-hosted instance is one compose profile flag on
  top of the existing install, with working datasources and at least one
  dashboard on first boot.
- The default install (`docker compose up -d`) is unchanged: same three
  containers, no new mounts, no required env vars, no errors about files it does
  not have.
- Works on Docker, rootless Podman and NAS-style hosts alike: no Docker socket
  mounted into any container.

## Non-goals

- Metrics/Prometheus, OpenTelemetry traces, alerting, or log-based alert rules.
- Request IDs / trace correlation across handler → job. (Noted as a follow-up; the
  access-log span added here is the prerequisite.)
- Authentication in front of Grafana beyond its own login, and a reverse-proxy
  path (`/grafana`) through the bundled nginx.
- Changing the web client, or surfacing logs in the app's own UI.
- Shipping the monitoring stack in a way that changes what existing users run.

## Anti-goals

- Adding a `/var/run/docker.sock` mount to a publicly documented compose file.
- Making the app's console output verbose by default as a side effect of this.
- Unbounded disk growth (no retention) on a machine that also hosts the media
  library.

## Constraints

- This repo is shipped as two release files (`compose.yml`, `example.env`) that a
  self-hoster downloads into an empty directory; anything else must be an
  additional, clearly documented download.
- `compose.yml` exists twice (repo root for local builds, `cloud-compose/` for
  releases) and the personal deployment lives in a third repo
  (`compose-services`). The services block will be duplicated, consistent with
  how the app services are already duplicated.
- Config is serde-deserialized from `config/settings.yaml` with `RP__*` env
  overrides; a new required field would break anyone with a custom settings file,
  so new fields need `#[serde(default)]` plus a `Default` impl.
- Rust CI runs `cargo fmt --check`, strict `clippy` (pedantic + nursery denied) and
  `SQLX_OFFLINE=true` builds. No SQL changes here, so no `.sqlx` churn expected.
- Loki and Grafana need config files on disk; Grafana cannot be provisioned
  through environment variables alone.

## Approaches considered

**A. Alloy tails Docker's json-file logs via `/var/run/docker.sock`.** Since the
app already writes to stdout, this needs no application change: about an hour of
compose and Alloy config, container labels for free. Rejected for a shipped stack:
it requires root-equivalent socket access in a public compose file, and it breaks
on rootless Podman (`/run/user/<uid>/podman/podman.sock`), on NAS Docker variants,
and on Docker Desktop hosts where `/var/lib/docker` is not visible from inside a
container. That is a steady source of setup failures for exactly the audience this
feature targets.

**B. The app writes JSON lines to `<app_data>/logs/`, Alloy tails that directory.**
Chosen. The log sink is a plain volume the stack already mounts for thumbnails
and hosted assets, so no new privilege and no assumption about the container
runtime. It also gives log persistence for free, which fixes the "logs vanish on
recreate" complaint even for users who never enable Grafana. Costs roughly two
hours of Rust and one new dependency.

**C. Grafana with a Postgres datasource only, no Loki.** Cheapest useful step, and
it is included as a side effect of B (Grafana can talk to the `database` service
in the same compose project by name), but it cannot answer "what did it say before
it died", which is the original complaint. Not sufficient alone.

## The chosen shape

### Application side

A second tracing layer writes to a rolling file sink, independent of the console
layer:

- Console layer: unchanged behaviour and text format; keeps the dev-friendly
  filters already in `settings.yaml`, overridable with `RUST_LOG`.
- File layer: JSON (`flatten_event`, current span name, no span list, no ANSI),
  written through `tracing_appender`'s non-blocking writer to
  `<app_data>/logs/`, daily rotation, `max_files` kept.
- The two layers carry **separate** filters. The file defaults to `info` so that
  the console's `api=debug` / `tower_http=debug` chatter does not get archived,
  and Loki does not pay for it.
- Settings gain `logging.file { enabled, filters, max_files }`, all serde-defaulted
  so an older settings file still loads. Default: enabled, `info`, 7 files. A
  compile-time constant names the `logs` folder, matching how `thumbnails_root`
  and friends are derived from `app_data_root`.
- Because the file layer needs `app_data_root`, settings are loaded before the
  subscriber is installed (today it is the reverse, with a reload workaround).
  Failure to load settings still fails loudly through color-eyre.
- The non-blocking writer's `WorkerGuard` must outlive the process, so
  `setup_binary` grows a third return value and each of the five binaries holds
  it in `main`.
- Two `println!` calls that bypass tracing (`jobs/management.rs` when logging a
  retry's error, `auth/error.rs` for internal auth errors) move to `warn!` / `error!`.
- The HTTP layer gains an INFO-level access log: `TraceLayer` keeps its
  no-op `on_request`, but the span and `on_response` move from DEBUG to INFO with
  status and latency, so the file sink records requests without request IDs yet.

### Stack side

Compose profile `monitoring`, three services appended to the existing file:

- `loki` — single binary, filesystem storage + TSDB schema, retention from an env
  var with the compactor enabled; no published port.
- `alloy` — mounts `${APP_DATA_LOCATION:-photos_app_data}:/app_data:ro` and tails
  `logs/*.log*`, parses each line as JSON, promotes `level` and `target` to labels
  (both bounded cardinals), adds static `service="photos"` / `job="photos-backend"`,
  pushes to `loki:3100`. Positions file on a named volume so a restart does not
  re-ship.
- `grafana` — published on `${GRAFANA_BIND:-127.0.0.1}:${GRAFANA_PORT:-9476}`,
  provisioned datasources (Loki, and Postgres at `database:5432` using the
  existing `DB_*` env values) and one dashboard that works on first boot: job
  queue depth and failure rate, ingest throughput, and error volume from Loki.

Supporting files live in `cloud-compose/monitoring/` (Loki config, Alloy config,
Grafana provisioning + dashboards, a README), mounted with relative paths.

**Why there, and not a top-level `monitoring/`:** the released artifact is
`cloud-compose/compose.yml`, and relative mounts resolve against the compose
file's own directory. A top-level `monitoring/` would make the released file
wrong (it would look for `cloud-compose/monitoring/`), and a copy under
`cloud-compose/` would mean two copies to keep in sync. Putting the single copy
where the release already lives makes both files correct: the released file says
`./monitoring/...` and the zip unpacks exactly that beside it; the root
`compose.yml` references `./cloud-compose/monitoring/...`. Consequence: the
release zip is assembled from `cloud-compose/monitoring/` with the directory name
preserved.

### Packaging and docs

- `cloud-compose/monitoring/` is published as one extra release asset
  (`monitoring.zip`), a small change to `docker-publish.yml`. The two-file install
  flow is untouched; the observability section tells users to fetch the zip and
  unzip it beside `compose.yml`.
- `example.env` gains `GRAFANA_BIND`, `GRAFANA_PORT`, `GRAFANA_ADMIN_PASSWORD` and
  `LOKI_RETENTION_DAYS`, all with working defaults. Profile-gated services still
  get env interpolation evaluated, so no `${VAR:?}` may be used for them.
- Root `README.md` gains an "Optional: monitoring" section;
  `cloud-compose/monitoring/README.md` carries the detail (ports, exposure,
  retention, how to query, how to disable, the Podman caveat).
- Verification of the shipped artifact is explicit: stage a temp directory with
  `compose.yml`, `example.env` as `.env`, and `monitoring/` unzipped, then
  `docker compose --profile monitoring up -d` and inspect Grafana — not just a
  `docker compose config` check against the checkout.

## Decisions taken during the conversation

- Approach B over A, for portability of the shipped stack (user's call).
- Stack is a compose profile inside this repo rather than only in the personal
  `compose-services` repo (user's request).
- Bounded file logging is on by default rather than opt-in, so that enabling the
  profile is a single flag and log persistence benefits every install. This is my
  call — the alternative is an extra env var on the backend service.

## My calls (check these while reading)

1. File sink defaults: enabled, `info`, 7 daily files. Disable with
   `RP__LOGGING__FILE__ENABLED=false`.
2. Separate filters per sink rather than one global filter; console behaviour
   stays as it is today apart from the `RUST_LOG` fix below.
3. The file sink takes the **same filter shape as the console** (`main` + `targets`
   map), not a flat level. The console deliberately quiets `ort`, `hf_hub`,
   `xet*`, `sqlx`, `open_clip_inference`, `object_detector` and `face_id` to
   warn/error; a flat `info` file filter would archive *more* from those targets
   than the console shows, which is the opposite of the intent. Shipped defaults
   duplicate that quiet list with `main: info`.
4. `RUST_LOG` is made to win over `logging.filters` for the console sink. Today
   `binary_setup` reads it and then unconditionally replaces the filter with the
   settings value, so it is effectively dead; making it win is one line and is
   what anyone typing `RUST_LOG` expects. The file sink ignores it and follows
   settings only, so a debug session cannot silently inflate the archive.
5. Access log at INFO is in scope; request IDs are not.
6. Grafana is published on all interfaces at `${GRAFANA_PORT:-9476}`, with
   `GF_SECURITY_ADMIN_PASSWORD` taken from `GRAFANA_ADMIN_PASSWORD` in `.env`
   (documented default `admin`, README says to change it). Chosen by the user over
   a loopback default: LAN access out of the box matters more here than closing
   the window between enabling the profile and setting a password. Loki's 3100 and
   Alloy's 12345 stay unpublished — unlike Grafana they have no login at all.
7. No host paths for Loki/Grafana data: named volumes, matching the existing
   compose style. The author's personal stack may bind-mount to SSD instead.
8. Retention is expressed in days via `LOKI_RETENTION_DAYS` (default 30), applied
   in both `limits_config` and the compactor.
9. `max_files` (days) is the only size bound; no byte budget or gzip. At `info`
   with the quiet list, a day is single-digit MB even with the per-request access
   log.
10. The `off by default` guarantee is scoped to Docker Compose, which is what the
    documented install uses. `podman compose` silently ignoring `profiles` is
    accepted as a documented limitation in `monitoring/README.md` (the failure is
    loud — Grafana crash-loops because the config dir is absent — not silent).

## What is left to the builder

- Exact Alloy/Loki/Grafana config syntax and pinned image tags (verify against
  current docs at implementation time; pin, do not use `latest`).
- Dashboard panels and queries.
- Whether a `monitoring` healthcheck is worth it for Loki/Alloy (they have no
  cheap endpoint; a `depends_on` start order is probably enough).
- Test coverage: no test suite covers log output, so verification is
  `cargo fmt`/`clippy` plus a manual run that inspects the produced JSON file and
  a compose up that inspects Grafana. A unit test for the per-sink filter
  resolution and the log-folder path is cheap enough to add.
