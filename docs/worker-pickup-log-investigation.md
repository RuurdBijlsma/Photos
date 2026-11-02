# Worker pickup log investigation

**Date:** 2026-11-02  
**Status:** Root cause strongly indicated; no permanent code change made.

## Introduction

Ruurd Photos places background work—such as scanning media and generating thumbnails—in a database-backed job queue. Worker tasks repeatedly ask the database for an available job, claim one, and then process it. When a worker claims a job, it is supposed to write a `Picked up ... job` message to the application logs. That message is useful for seeing what the workers are doing, but it is not what performs or confirms the claim.

The application sends tracing log events to two destinations: a readable console and, when enabled, daily JSON log files. Each destination can have its own filter settings. This investigation looked at why a job could be claimed while its pickup message was missing from the logs, and whether the configured filters or the worker queue explained the discrepancy.

## Symptom

The worker sometimes claims a queued job, but the expected event is absent:

```text
🐜 Picked up {:?} job: {:?}
```

The event is emitted in `server/crates/apps/worker/src/worker.rs` immediately after `claim_next_job` returns
`Some(job)`.

## Evidence

- In the 21:45 run, temporary instrumentation printed `PICKUP_DIAG branch reached` with `job_id=96362` and
  `job_type=PruneMissingIngestJobs`. This is direct evidence that the claim branch ran, even though the corresponding
  pickup event was not visible.
- The diagnostic reported `event_enabled=true`, `RUST_LOG=None`, and filters whose default directive is `info`. The
  console filter has `sqlx=warn`; the file filter also has `sqlx=warn`. These settings should allow an `INFO` event from
  `worker::worker`.
- In an earlier instrumented run, evaluating `tracing::event_enabled!` before the pickup event was followed by the
  pickup event appearing. In later runs, the diagnostic ran after the pickup event; it reported `event_enabled=true`,
  but the pickup event remained absent. An enabled check after an event cannot restore an event that was already
  dropped.
- Logging setup installs distinct per-layer `EnvFilter`s for the console and JSON file layers in
  `server/crates/libs/common_services/src/logging.rs`.
- `claim_next_job` executes a SQLx query before the pickup event. SQLx uses the `log` crate's `log_enabled!` check while
  deciding whether to produce query log details.

## Best-supported explanation

This closely matches [tokio-rs/tracing issue #2519](https://github.com/tokio-rs/tracing/issues/2519), which describes
stale per-layer filter state after an `enabled` check made through the `log` compatibility path. The next tracing event
can then be filtered out even though the configured filter would normally allow it.

The likely sequence is:

1. The worker claims a job using the SQLx query in `claim_next_job`.
2. SQLx performs a `log_enabled!` check for query logging.
3. The worker emits its pickup `INFO` event.
4. The per-layer filter state left by the compatibility check causes that event to be dropped by the tracing subscriber.

This accounts for the mismatch between the diagnostic's direct `eprintln!` output and the missing tracing event. It also
explains why the filters can show `INFO` while the event is still lost. The behavior is consistent with the upstream
issue, though the exact internal filter state was not instrumented directly in this application.

SQLx has a separate report describing the same missing-log behavior after a query. That report identifies the tracing
issue and documents two workarounds: disable SQL statement logging, or add a no-op layer so callsites are re-evaluated
and stale per-layer filter state is refreshed. See [SQLx issue #3751](https://github.com/transact-rs/sqlx/issues/3751).

## Scope of what the logs establish

The 21:45 diagnostic establishes that job `96362` was claimed. The later lines showing thumbnail and light workers
starting and exiting do not, by themselves, establish that those workers claimed jobs; the diagnostic output contains no
claim record for them.

The missing pickup event is therefore a logging problem for at least one confirmed claim. It should not be used as the
only signal for whether a job was claimed.

## Possible remediation

### Preferred workaround: no-op layer

Add a no-op `Layer` to the registry alongside the console and file layers. It writes nothing, but makes callsite
interest dynamic so the per-layer filters re-check each event and refresh their state. This preserves the separate
console and file filters and avoids adding dummy application log events. SQLx issue #3751 includes a concrete example;
[tracing PR #3572](https://github.com/tokio-rs/tracing/pull/3572) proposes the corresponding upstream fix.

This is still a workaround, and it adds a filter check per event for callsites otherwise accepted by all layers. The
upstream PR describes that trade-off and was still open when checked on 2026-11-02. The project currently locks
`tracing-subscriber` 0.3.23, whose release notes do not include this fix, so a routine dependency update is not yet a
known solution.

### Disable SQL statement logging

SQLx's `ConnectOptions::disable_statement_logging()` prevents its query logger from making the problematic enabled
check. This is the simplest workaround if SQL statement logs are not needed. It also disables slow-statement logs, so
it would remove potentially useful SQLx warnings under the current `sqlx=warn` filters.

### Keep the dummy event temporarily

The current `info!("-")` before the pickup event is a small, local workaround: the dummy event consumes the stale
filter decision, allowing the following pickup event through. It is less robust than a no-op layer because the dash
could appear if a particular iteration has no stale decision to consume, and it adds a meaningless log call to the
worker loop.

The upstream fix is to change `tracing-subscriber` so per-layer-filtered callsites do not cache an `always enabled`
decision that can be invalidated by a later bare `enabled()` check. Track PR #3572 and upgrade once a release containing
that fix is available. Replacing the per-layer filters with one global filter is another possible direction, but it
would remove the current ability to keep different console and file filtering policies.
