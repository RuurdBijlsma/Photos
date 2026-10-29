//! Subscriber setup: a human-readable console sink and an optional JSON file sink.
//!
//! The file sink exists so that logs survive a container being recreated and can be
//! shipped to Loki. It is deliberately a second layer with its own filter: the
//! console's `debug` targets would otherwise flood the archive.

use app_state::AppSettings;
use std::path::Path;
use tracing::{Subscriber, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, fmt};

/// Keeps the non-blocking file writer alive for the lifetime of the process.
///
/// Dropping it flushes and stops the writer thread, so every binary must hold it
/// until `main` returns.
#[must_use]
pub struct LoggingGuard(#[allow(dead_code)] Option<WorkerGuard>);

/// Builds the JSON file layer and its writer guard.
///
/// One file per UTC day, named `photos.YYYY-MM-DD.log`, pruning the oldest once
/// `max_files` are on disk.
pub fn file_layer<S>(
    dir: &Path,
    filters: EnvFilter,
    max_files: usize,
) -> color_eyre::Result<(impl Layer<S> + Send + Sync, WorkerGuard)>
where
    S: Subscriber + for<'a> LookupSpan<'a> + Send + Sync,
{
    let appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("photos")
        .filename_suffix("log")
        .max_log_files(max_files)
        .build(dir)?;
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let layer = fmt::layer()
        .json()
        .flatten_event(true)
        .with_current_span(true)
        .with_span_list(false)
        .with_ansi(false)
        .with_writer(writer);

    Ok((filters.and_then(layer), guard))
}

/// Installs the global subscriber: console always, JSON file when enabled.
///
/// A file sink that cannot be opened never takes the process down: the console
/// sink is installed first and the failure is reported through it.
pub fn init_logging(settings: &AppSettings) -> LoggingGuard {
    let console_filter = EnvFilter::new(settings.logging.filters.to_filter_string());
    let console_layer = fmt::layer();

    if !settings.logging.file.enabled {
        tracing_subscriber::registry()
            .with(console_filter)
            .with(console_layer)
            .init();
        return LoggingGuard(None);
    }

    let file_filter = EnvFilter::new(settings.logging.file.filters.to_filter_string());
    let Ok((file, guard)) = file_layer(
        &settings.ingest.logs_root,
        file_filter,
        settings.logging.file.max_files,
    ) else {
        tracing_subscriber::registry()
            .with(console_filter)
            .with(console_layer)
            .init();
        warn!(
            path = %settings.ingest.logs_root.display(),
            "File logging is disabled: the log directory could not be opened"
        );
        return LoggingGuard(None);
    };

    tracing_subscriber::registry()
        .with(console_filter)
        .with(console_layer)
        .with(file)
        .init();

    LoggingGuard(Some(guard))
}

#[cfg(test)]
mod tests {
    use super::file_layer;
    use std::fs;
    use std::path::Path;
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::layer::SubscriberExt;

    fn log_file_names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("test setup")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_file())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn writes_one_json_line_per_event() {
        let dir = tempfile::tempdir().expect("test setup");

        let (layer, guard) = file_layer(dir.path(), EnvFilter::new("info"), 7).expect("test setup");
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "t1", "a \"quote\"\nand newline");
        });
        drop(guard);

        let names = log_file_names(dir.path());
        assert_eq!(
            names.len(),
            1,
            "expected exactly one log file, got {names:?}"
        );

        let content = fs::read_to_string(dir.path().join(&names[0])).expect("test setup");
        let lines: Vec<&str> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        assert_eq!(lines.len(), 1, "expected one line, got {lines:?}");

        let parsed: serde_json::Value = serde_json::from_str(lines[0]).expect("test setup");
        assert_eq!(parsed["level"], "INFO");
        assert_eq!(parsed["target"], "t1");
        let message = parsed["message"].as_str().expect("test setup");
        assert!(message.contains("quote"), "message was {message:?}");
        assert!(message.contains('\n'), "message was {message:?}");
    }

    #[test]
    fn prunes_to_max_files_on_start() {
        let dir = tempfile::tempdir().expect("test setup");
        for day in 21..=24 {
            fs::write(dir.path().join(format!("photos.2026-10-{day}.log")), "{}").expect("test setup");
        }

        let (_layer, _guard) =
            file_layer::<tracing_subscriber::Registry>(dir.path(), EnvFilter::new("info"), 2)
                .expect("test setup");

        let names = log_file_names(dir.path());
        assert_eq!(
            names.len(),
            2,
            "expected the oldest files pruned, got {names:?}"
        );
        assert!(!names.iter().any(|name| name.contains("2026-10-21")));
        assert!(!names.iter().any(|name| name.contains("2026-10-22")));
        assert!(names.iter().any(|name| name.contains("2026-10-24")));
    }

    #[test]
    fn unwritable_directory_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().expect("test setup");
        let blocker = dir.path().join("blocker");
        fs::write(&blocker, "not a directory").expect("test setup");

        let result = file_layer::<tracing_subscriber::Registry>(
            &blocker.join("logs"),
            EnvFilter::new("info"),
            7,
        );

        assert!(result.is_err());
    }

    #[test]
    fn file_names_carry_utc_date() {
        let dir = tempfile::tempdir().expect("test setup");

        let (layer, guard) = file_layer(dir.path(), EnvFilter::new("info"), 7).expect("test setup");
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("hello");
        });
        drop(guard);

        let names = log_file_names(dir.path());
        assert_eq!(names.len(), 1);
        let name = &names[0];
        let date = name
            .strip_prefix("photos.")
            .and_then(|rest| rest.strip_suffix(".log"))
            .expect("log file name shape");
        assert_eq!(date.len(), 10, "expected YYYY-MM-DD in {name}");
        assert_eq!(
            date.matches('-').count(),
            2,
            "expected YYYY-MM-DD in {name}"
        );
    }
}
