use app_state::{
    AppSettings, CONSTANTS, database_url, load_constants_from_path, load_settings_from_path,
};
use color_eyre::eyre::Result;
use common_services::api::system::service::get_system_stats;
use common_services::database::get_db_pool;
use criterion::{Criterion, criterion_group, criterion_main};
use std::path::Path;
use tracing::info;

async fn setup() -> Result<(sqlx::PgPool, AppSettings)> {
    let project_root = Path::new("../../../").canonicalize()?;
    let settings_file = project_root.join("config/settings.yaml");
    let constants = load_constants_from_path(&settings_file)?;
    if CONSTANTS.set(constants).is_err() {
        info!("AppConstants were already initialized by another test.");
    }
    let settings = load_settings_from_path(&settings_file, None)?;
    let pool = get_db_pool(database_url(), false).await.expect("db pool");
    Ok((pool, settings))
}

fn bench_timeline(c: &mut Criterion) {
    // Create a Tokio runtime for the benchmark
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    // Run setup ONCE (connect to DB)
    let (pool, settings) = rt.block_on(setup()).expect("setup");

    // 1. Benchmark Ratios
    c.bench_function("get_timeline_ratios_desc", |b| {
        b.to_async(&rt).iter(|| async {
            get_system_stats(&pool, &settings.ingest, 1).await.unwrap();
        });
    });
}

criterion_group!(benches, bench_timeline);
criterion_main!(benches);
