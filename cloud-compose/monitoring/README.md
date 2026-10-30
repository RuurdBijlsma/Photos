# Optional monitoring stack (Grafana + Loki)

Logs survive container restarts and are searchable, and Grafana charts the job queue
and ingest health straight from the app's own Postgres tables.

The monitoring stack is optional. The default install (`docker compose up -d`) never
starts any of it.

## Enable

```bash
docker compose --profile monitoring up -d
```

That's it — the monitoring config is baked into the images, so no extra files are
needed beside `compose.yml`.

Open <http://localhost:9476>. The login is `admin` and `GRAFANA_ADMIN_PASSWORD` from
your `.env` (default `admin` — change it, the container can read your database
password).

To stop only the monitoring containers:

```bash
docker compose stop photos_loki photos_alloy photos_grafana
```

## What runs

| Service | Image | Purpose | Port |
| --- | --- | --- | --- |
| `loki` | `photos-loki` | Log store, filesystem + TSDB, retention | none published |
| `alloy` | `photos-alloy` | Tails `<app_data>/logs` and pushes to Loki | none published |
| `grafana` | `photos-grafana` | UI, provisioned datasources and dashboard | `${GRAFANA_PORT:-9476}` |

Only Grafana publishes a port, because it is the only one with a login. Loki and
Alloy have no authentication at all.

## How logs get there

The backend writes one JSON file per UTC day to `<app_data>/logs/` — the same
`app_data` volume that already holds thumbnails — and Alloy tails that directory
from a read-only mount. No Docker socket is mounted anywhere, so this also works on
rootless Podman and NAS Docker. Alloy keeps its read positions on its own volume, so
restarting it does not re-ship old lines, and the first run backfills whatever files
are already on disk.

To view them elsewhere: `docker exec photos_backend tail -f /app_data/logs/photos.$(date -u +%F).log`

## Retention and disk use

Two independent limits:

- **Log files on disk**: 7 daily files (`logging.file.max_files` in
  `config/settings.yaml`). Disable entirely with `RP__LOGGING__FILE__ENABLED=false`;
  the console output is unaffected either way.
- **Loki**: `LOKI_RETENTION_DAYS` in `.env` (default 30), enforced by the compactor.

Log files live on the `app_data` volume. On a machine with the app data on an SSD
that is fine; Loki keeps its chunks on its own `photos_loki_data` volume.

## Querying

Useful starting points in Grafana's Explore:

```logql
{job="photos-backend"} | json                      # everything, fields queryable
{job="photos-backend", level="ERROR"}              # errors only
{job="photos-backend"} | json | status >= 500      # failed requests
{job="photos-backend"} | json | target = "api::server"
```

The dashboard is provisioned read-only; edits made in the UI are not saved back
(`allowUiUpdates: false`).

## Telegram alerts

Grafana Unified Alerting is provisioned with a default contact point for Telegram:

1. Create a bot using [@BotFather](https://t.me/BotFather) and copy the token.
2. Message [@userinfobot](https://t.me/userinfobot) or start a chat with your bot to get your chat ID.
3. Set `TELEGRAM_BOT_TOKEN` and `TELEGRAM_CHAT_ID` in your `.env`.

Default provisioned alert rules:
- **`ALERT:` Macro**: Fires immediately if any Rust backend component logs via the `alert!` macro.
- **Background Job Failures**: Fires if background jobs fail within a 5-minute window.

## Caveats

- **Passwords**: Grafana's provisioning loader expands `$__env{...}` and has no
  default syntax, so a `$` inside `DB_PASSWORD` will be mangled. Avoid `$` in that
  password, or override the datasource provisioning by mounting a custom file.
- **Podman**: the "off by default" guarantee assumes a Compose implementation that
  honours `profiles`. If `podman compose up` starts `photos_loki`/`photos_alloy`/
  `photos_grafana` without the flag, your podman-compose version ignores profiles —
  stop them with the `docker compose stop` line above (using `podman compose`).
- **Grafana storage**: dashboards and settings live in `photos_grafana_data`; deleting
  that volume resets Grafana (provisioning re-creates the datasources and dashboard).
- Grafana holds the database credentials in the container environment, so treat port
  `9476` like the database port itself: LAN only, or put it behind your reverse proxy.
