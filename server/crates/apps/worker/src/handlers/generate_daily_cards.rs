use crate::context::WorkerContext;
use crate::handlers::JobResult;
use crate::handlers::common::daily_cards::DailyCardGenerator;
use crate::handlers::common::daily_cards::cluster_card::ClusterCardGenerator;
use crate::handlers::common::daily_cards::estimatr_card::LocationEstimatrCardGenerator;
use crate::handlers::common::daily_cards::on_this_day_card::OnThisDayCardGenerator;
use chrono::Utc;
use color_eyre::Result;
use common_services::database::jobs::Job;
use tracing::{info, warn};

pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    info!("Running daily cards generation job");

    let mut tx = context.pool.begin().await?;

    // Cleanup daily cards:
    // - card_date before today
    // - card_date is NULL & shown is true
    let today = Utc::now().naive_utc().date();
    sqlx::query!(
        "DELETE FROM daily_card WHERE card_date < $1 OR (card_date IS NULL AND shown = true)",
        today
    )
    .execute(&mut *tx)
    .await?;

    let users = sqlx::query!("SELECT id FROM app_user")
        .fetch_all(&mut *tx)
        .await?;

    let generators: Vec<Box<dyn DailyCardGenerator + Send + Sync>> = vec![
        Box::new(ClusterCardGenerator),
        Box::new(OnThisDayCardGenerator),
        Box::new(LocationEstimatrCardGenerator),
    ];

    for user in users {
        for generator in &generators {
            info!(
                "Running {} daily card generator for user {}",
                generator.card_type(),
                user.id
            );
            if let Err(e) = generator
                .generate(&mut tx, user.id, &context.settings)
                .await
            {
                warn!(
                    "Failed to generate daily cards of type {} for user {}: {:?}",
                    generator.card_type(),
                    user.id,
                    e
                );
            }
        }
    }

    tx.commit().await?;
    info!("Daily cards generation job completed successfully");

    Ok(JobResult::Done)
}
