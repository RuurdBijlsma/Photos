use color_eyre::Result;
use colored::*;
use std::future::Future;
use std::time::Instant;
use tracing::Level;
use tracing_subscriber::{EnvFilter, fmt};

#[macro_export]
macro_rules! run_test {
    ($call:expr) => {
        // We access run_test_impl via $crate::... so it works from anywhere
        $crate::runner::orchestration_utils::run_test_impl(stringify!($call), $call)
    };
}

/// A macro to run a list of tests and print a summary.
#[macro_export]
macro_rules! execute_suite {
        ($context:expr, [ $($test_fn:ident),* $(,)? ]) => {
            {
                // 1. Count the tests by tricking the macro expansion
                let total_tests = 0 $( + { let _ = stringify!($test_fn); 1 } )*;
                let mut passed_tests = 0;
                let suite_start = Instant::now();
                println!();

                // 2. Run each test sequentially
                $(
                    // To run ALL tests even if one fails, remove the '?'
                    // but we'll need to handle the Err manually.
                    run_test!($test_fn($context)).await?;
                    passed_tests += 1;
                )*

                // 3. Print Summary
                println!("{}", "─".repeat(60).truecolor(80, 80, 80));
                println!(
                    "{} {}/{} tests passed successfully in {:.2?}.",
                    " SUMMARY ".on_purple().black().bold(),
                    passed_tests,
                    total_tests,
                    suite_start.elapsed()
                );
                println!("{}", "─".repeat(60).truecolor(80, 80, 80));
                println!();
            }
        };
    }

/// A helper to make test output distinct and readable
pub async fn run_test_impl<Fut>(raw_name: &str, test: Fut) -> Result<()>
where
    Fut: Future<Output = Result<()>>,
{
    let name_no_args = raw_name.split('(').next().unwrap_or(raw_name);
    let pretty_name = name_no_args
        .split("::")
        .last()
        .unwrap_or(name_no_args)
        .trim();

    println!("{}", "─".repeat(60).truecolor(80, 80, 80));
    println!(
        "{} {}",
        " RUNNING ".on_cyan().black().bold(),
        pretty_name.cyan().bold()
    );

    let start_time = Instant::now();
    let result = test.await;
    let elapsed = start_time.elapsed();

    match result {
        Ok(()) => {
            println!(
                "{} {} ({:.2?})",
                " PASSED ".on_green().black().bold(),
                pretty_name.green(),
                elapsed
            );
        }
        Err(ref e) => {
            println!(
                "{} {} ({:.2?})",
                " FAILED ".on_red().black().bold(),
                pretty_name.red(),
                elapsed
            );
            println!("\n{e:?}");
        }
    }

    result
}

pub fn setup_tracing_and_panic_handling() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info,sqlx=warn,api=debug,hyper=error,reqwest=error,ort=warn".into());

    let subscriber = fmt::Subscriber::builder()
        .with_max_level(Level::INFO)
        .with_env_filter(filter)
        .compact()
        .with_target(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed");

    color_eyre::install().expect("Failed to install color_eyre");
}
