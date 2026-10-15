# Perf regression tests

Numbers must come from your machine (same hardware)

## One-time setup

```bash
cd web
npx playwright install chromium # or chrome channel
```

Backend stays local: Rust + podman postgres running, e.g. `cargo run --release`.

## Run before every MR to main

```bash
cd web
npm run build-only
# terminal A: serve prod build (proxies /api -> 127.0.0.1:5272)
npm run preview # -> http://localhost:4173
# terminal B:
npm run perf
```

This runs 5 measured runs (plus warmup) of:

* `/` LCP, `/explore` LCP, click photo -> viewer LCP
* fast scroll at ~33k px/s: FPS, p95/max frametime, spikes >100ms (LoAF + rAF)
* page-to-page (`/ -> /explore -> /map -> /search`) FPS

Output: `web/tests/perf/results/YYYY-MM-DD_<branch>_<sha>.json`.
Commit this file.

## Tolerances (`check-perf-regression.mjs`)

Regression = worse than baseline + tolerance:

* LCP: +10% or +200ms
* p95 frame: +15% or +5ms
* scroll spikes>100ms: must not increase
* FPS: -5% or -2fps

## Files

* `run.mjs` — local runner (Chrome preferred, Chromium fallback)
* `check-perf-regression.mjs` — `--current <pr.json> --baseline <main.json>`
* `results/` — committed JSON per run
* `.github/workflows/perf-gate.yml` — PR check: requires a new JSON,
  validates schema, compares latest PR file vs newest main file
