use crate::context::WorkerContext;
use crate::handlers::JobResult;
use color_eyre::Result;
use common_services::database::jobs::Job;
use common_services::database::key_vector_store::KeyVectorStore;
use pgvector::Vector;
use sqlx::PgPool;
use tracing::info;

pub async fn handle(context: &WorkerContext, _job: &Job) -> Result<JobResult> {
    info!("🔄 Updating global centroid...");

    let pool = &context.pool;

    // 1. Calculate the average of 1000 random items
    let centroid = calculate_global_centroid(pool).await?;

    if let Some(vector) = centroid {
        // 2. Save to key_vector_store
        KeyVectorStore::set_vector(pool, "global_centroid", &vector).await?;
        info!("✅ Global centroid updated successfully.");
    } else {
        info!("⚠️ No embeddings found, skipping global centroid update.");
    }

    Ok(JobResult::Done)
}

async fn calculate_global_centroid(pool: &PgPool) -> Result<Option<Vec<f32>>> {
    let result = sqlx::query!(
        r#"
        WITH global_sample AS (
            SELECT embedding
            FROM visual_analysis
            WHERE deleted = false
            ORDER BY random()
            LIMIT 10000
        )
        SELECT avg(embedding)::vector as "center: Vector" FROM global_sample
        "#
    )
    .fetch_one(pool)
    .await?;

    Ok(result.center.map(|v: Vector| v.to_vec()))
}
