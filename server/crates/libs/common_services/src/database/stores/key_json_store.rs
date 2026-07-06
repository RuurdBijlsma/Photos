use color_eyre::Result;
use sqlx::PgExecutor;

pub struct KeyJsonStore;

impl KeyJsonStore {
    pub async fn get_value(
        executor: impl PgExecutor<'_>,
        key: &str,
        user_id: Option<i32>,
    ) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query!(
            r#"
            SELECT value
            FROM key_json_store
            WHERE key = $1 AND user_id IS NOT DISTINCT FROM $2
            "#,
            key,
            user_id
        )
        .fetch_optional(executor)
        .await?;

        Ok(row.and_then(|r| r.value))
    }

    pub async fn set_value(
        executor: impl PgExecutor<'_>,
        key: &str,
        value: &serde_json::Value,
        user_id: Option<i32>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO key_json_store (key, value, user_id, updated_at)
            VALUES ($1, $2, $3, now())
            ON CONFLICT (user_id, key) DO UPDATE SET
                value = EXCLUDED.value,
                updated_at = now()
            "#,
            key,
            value,
            user_id
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
