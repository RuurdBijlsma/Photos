use sqlx::PgTransaction;

pub async fn get_or_create_remote_user(
    tx: &mut PgTransaction<'_>,
    local_user_id: i32,
    remote_identity: &str,
) -> Result<i32, sqlx::Error> {
    let remote_user_id = sqlx::query_scalar!(
        "SELECT id FROM remote_user WHERE identity = $1 AND user_id = $2",
        remote_identity,
        local_user_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(id) = remote_user_id {
        return Ok(id);
    }

    // Not found, so create it
    let new_id = sqlx::query_scalar!(
        "INSERT INTO remote_user (identity, user_id) VALUES ($1, $2) RETURNING id",
        remote_identity,
        local_user_id
    )
    .fetch_one(&mut **tx)
    .await?;

    Ok(new_id)
}
