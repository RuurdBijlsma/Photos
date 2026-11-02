# Worker pickup log investigation

**Date:** 2026-11-02  
**Status:** Root cause strongly indicated; no permanent code change made.

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

## Scope of what the logs establish

The 21:45 diagnostic establishes that job `96362` was claimed. The later lines showing thumbnail and light workers
starting and exiting do not, by themselves, establish that those workers claimed jobs; the diagnostic output contains no
claim record for them.

The missing pickup event is therefore a logging problem for at least one confirmed claim. It should not be used as the
only signal for whether a job was claimed.

## Possible remediation

The durable fix is likely to change the interaction between SQLx's `log` compatibility checks and the application's
separate per-layer filters, or to use a `tracing-subscriber` version that contains a fix for issue #2519. Any version
update should be checked against the locked dependency version and the upstream fix status first.

Replacing the separate per-layer filters with a single global filter may avoid this class of interaction, but it would
also remove the current ability to keep different console and file filtering policies. Disabling or changing the
log-to-tracing bridge may also affect SQLx log visibility. No remediation has been applied yet.
