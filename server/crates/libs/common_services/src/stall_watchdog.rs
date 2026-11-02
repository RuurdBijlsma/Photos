//! Temporary diagnostics for unexplained multi-second API stalls.
//!
//! Writes one line per second to `<app_data>/logs/stall.log` from two places:
//! a plain OS thread and a tokio task (one per runtime). A gap in the `os` lines
//! means the whole process stopped (paging, antivirus, suspended); a gap or a
//! large `drift_ms` in a `tokio` line means that runtime had no free worker for
//! a trivial task. It writes straight to the file instead of through `tracing`,
//! so a blocked console sink stays visible as a difference between the two.
//!
//! Remove once the stall is found and fixed.

use chrono::Local;
use sqlx::PgPool;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

pub fn spawn_stall_watchdog(runtime_label: &'static str, logs_root: &Path, pool: PgPool) {
    let path = logs_root.join("stall.log");

    std::thread::spawn({
        let path = path.clone();
        move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                write_line(&path, "os alive");
            }
        }
    });

    let metrics = tokio::runtime::Handle::current().metrics();
    tokio::spawn(async move {
        let mut last = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let now = Instant::now();
            let drift = now
                .duration_since(last)
                .saturating_sub(Duration::from_secs(1));
            last = now;
            write_line(
                &path,
                &format!(
                    "tokio rt={runtime_label} drift_ms={} workers={} tasks={} queue={} db_size={} db_idle={}",
                    drift.as_millis(),
                    metrics.num_workers(),
                    metrics.num_alive_tasks(),
                    metrics.global_queue_depth(),
                    pool.size(),
                    pool.num_idle(),
                ),
            );
        }
    });
}

fn write_line(path: &Path, line: &str) {
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    drop(writeln!(
        file,
        "{} {line}",
        Local::now().format("%H:%M:%S%.3f")
    ));
}

#[cfg(test)]
mod tests {
    use super::spawn_stall_watchdog;
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn writes_one_line_per_second_from_both_sources() {
        let dir = tempfile::tempdir().expect("test setup");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://user:pass@localhost/photos")
            .expect("lazy pool");

        spawn_stall_watchdog("test", dir.path(), pool);
        tokio::time::sleep(Duration::from_millis(2500)).await;

        let content =
            std::fs::read_to_string(dir.path().join("stall.log")).expect("stall.log written");
        assert!(content.contains("os alive"), "{content}");
        assert!(content.contains("tokio rt=test drift_ms="), "{content}");
    }
}
