use app_state::{CONSTANTS, DATABASE_URL, database_url, load_constants_from_path};
use color_eyre::eyre::Result;
use common_services::api::timeline::interfaces::SortDirection;
use common_services::api::timeline::service::{
    get_photos_by_month, get_timeline_ids, get_timeline_ratios,
};
use common_services::database::app_user::User;
use common_services::database::get_db_pool;
use common_services::database::user_store::UserStore;
use criterion::{Criterion, criterion_group, criterion_main};
use std::path::Path;
use tracing::info;

async fn setup() -> Result<(sqlx::PgPool, User)> {
    let project_root = Path::new("../../../").canonicalize()?;
    let settings_file = project_root.join("config/settings.yaml");
    let constants = load_constants_from_path(&settings_file)?;
    if CONSTANTS.set(constants).is_err() {
        info!("AppConstants were already initialized by another test.");
    }
    let test_db_url = "postgres://photos_user:dev-password@localhost/photos";
    if DATABASE_URL.set(test_db_url.to_owned()).is_err() {
        info!("AppConstants were already initialized by another test.");
    }
    let pool = get_db_pool(database_url(), false).await.expect("db pool");
    let user = UserStore::list_users(&pool)
        .await
        .expect("user")
        .first()
        .expect("user")
        .clone();
    Ok((pool, user))
}

fn bench_timeline(c: &mut Criterion) {
    // Create a Tokio runtime for the benchmark
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    // Run setup ONCE (connect to DB)
    let (pool, user) = rt.block_on(setup()).expect("setup");

    // 1. Benchmark Ratios
    c.bench_function("get_timeline_ratios_desc", |b| {
        b.to_async(&rt).iter(|| async {
            get_timeline_ratios(&user, &pool, SortDirection::Desc)
                .await
                .unwrap();
        });
    });
    c.bench_function("get_timeline_ratios_asc", |b| {
        b.to_async(&rt).iter(|| async {
            get_timeline_ratios(&user, &pool, SortDirection::Asc)
                .await
                .unwrap();
        });
    });

    // 2. Benchmark IDs
    c.bench_function("get_timeline_ids_desc", |b| {
        b.to_async(&rt).iter(|| async {
            get_timeline_ids(&user, &pool, SortDirection::Desc)
                .await
                .unwrap();
        });
    });
    c.bench_function("get_timeline_ids_asc", |b| {
        b.to_async(&rt).iter(|| async {
            get_timeline_ids(&user, &pool, SortDirection::Asc)
                .await
                .unwrap();
        });
    });

    // 3. Benchmark Photos by Month
    let month_ids = vec![
        chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2025, 8, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
    ];

    c.bench_function("get_photos_by_month_desc", |b| {
        b.to_async(&rt).iter(|| async {
            get_photos_by_month(&user, &pool, &month_ids, SortDirection::Desc)
                .await
                .unwrap();
        });
    });
    c.bench_function("get_photos_by_month_asc", |b| {
        b.to_async(&rt).iter(|| async {
            get_photos_by_month(&user, &pool, &month_ids, SortDirection::Asc)
                .await
                .unwrap();
        });
    });
}

criterion_group!(benches, bench_timeline);
criterion_main!(benches);
