use color_eyre::Result;
use pgvector::Vector;
use sqlx::PgExecutor;

pub struct KeyVectorStore;

impl KeyVectorStore {
    pub async fn get_vector(executor: impl PgExecutor<'_>, key: &str) -> Result<Option<Vec<f32>>> {
        let row = sqlx::query!(
            r#"
            SELECT vector as "vector: Vector" FROM key_vector_store WHERE key = $1
            "#,
            key
        )
        .fetch_optional(executor)
        .await?;

        Ok(row.and_then(|r| r.vector).map(|v: Vector| v.to_vec()))
    }

    pub async fn set_vector(
        executor: impl PgExecutor<'_>,
        key: &str,
        vector: &[f32],
    ) -> Result<()> {
        let vector = Vector::from(vector.to_vec());
        sqlx::query!(
            r#"
            INSERT INTO key_vector_store (key, vector, updated_at)
            VALUES ($1, $2, now())
            ON CONFLICT (key) DO UPDATE SET
                vector = EXCLUDED.vector,
                updated_at = now()
            "#,
            key,
            vector as Vector
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
