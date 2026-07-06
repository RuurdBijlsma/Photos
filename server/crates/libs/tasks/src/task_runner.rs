// Put this in a new file (e.g., src/background_jobs.rs) or at the top of server.rs
use app_state::AppSettings;
use color_eyre::Result;
use common_services::database::jobs::JobType;
use common_services::job_queue::enqueue_job;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};

pub fn init_task_scheduler(pool: &PgPool, settings: &AppSettings) -> Result<()> {
    let schedules = vec![
        TaskSchedule {
            interval: Duration::from_hours(24),
            jobs: vec![
                JobType::Scan,
                JobType::CleanDb,
                JobType::SyncThumbnails,
                JobType::ClusterPhotos,
                JobType::ClusterFaces,
                JobType::UpdateGlobalCentroid,
                JobType::CalcSystemStats,
            ],
        },
        TaskSchedule {
            interval: Duration::from_hours(72),
            jobs: vec![JobType::GenerateDailyCards],
        },
    ];
    run_tasks(pool, settings, schedules)?;

    Ok(())
}

pub struct TaskSchedule {
    pub interval: Duration,
    pub jobs: Vec<JobType>,
}

pub fn run_tasks(
    pool: &PgPool,
    settings: &AppSettings,
    schedules: Vec<TaskSchedule>,
) -> Result<()> {
    for schedule in schedules {
        let pool = pool.clone();
        let ingest_settings = settings.ingest.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(schedule.interval);
            loop {
                interval.tick().await;
                info!(
                    "Running scheduled jobs {:?} for interval {} hours",
                    &schedule.jobs,
                    schedule.interval.as_secs() / 3600
                );

                for job_type in &schedule.jobs {
                    let res = enqueue_job::<()>(&pool, &ingest_settings, *job_type)
                        .call()
                        .await;

                    if let Err(e) = res {
                        error!("Failed to enqueue scheduled job {:?}: {}", job_type, e);
                    }
                }
            }
        });
    }
    Ok(())
}
