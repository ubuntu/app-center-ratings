use sqlx::Row;

use crate::utils::infrastructure::get_repository;

use super::entities::{User, Vote};

pub(crate) async fn create_user_in_db(user: User) -> Result<User, sqlx::Error> {
    let mut pool = get_repository().await?;

    let user_with_id = sqlx::query(
        r#"
        INSERT INTO users (client_hash, created, last_seen)
        VALUES ($1, $2, $2)
        RETURNING id
        "#,
    )
    .bind(&user.client_hash)
    .bind(&user.last_seen)
    .bind(&user.created)
    .fetch_one(&mut *pool)
    .await?
    .try_get("id")
    .map(|id| User { id, ..user })?;

    Ok(user_with_id)
}

pub(crate) async fn user_seen(client_hash: &str) -> Result<bool, sqlx::Error> {
    let mut pool = get_repository().await?;

    let result = sqlx::query(
        r#"
            UPDATE users
            SET last_seen = NOW()
            WHERE client_hash = $1;
        "#,
    )
    .bind(client_hash)
    .execute(&mut *pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

pub(crate) async fn delete_user_by_client_hash(client_hash: &str) -> Result<u64, sqlx::Error> {
    let mut pool = get_repository().await?;

    let rows_deleted = sqlx::query(
        r#"
        DELETE FROM users
        WHERE client_hash = $1
        "#,
    )
    .bind(client_hash)
    .execute(&mut *pool)
    .await?
    .rows_affected();

    Ok(rows_deleted)
}

pub(crate) async fn save_vote_to_db(vote: Vote) -> Result<u64, sqlx::Error> {
    let mut pool = get_repository().await?;

    let rows_affected = sqlx::query(
        r#"
        INSERT INTO votes (user_id_fk, snap_id, snap_revision, vote_up)
        VALUES ((SELECT id FROM users WHERE client_hash = $1), $2, $3, $4)
        ON CONFLICT (user_id_fk, snap_id, snap_revision)
        DO UPDATE SET vote_up = EXCLUDED.vote_up, created = NOW();
        "#,
    )
    .bind(vote.client_hash)
    .bind(vote.snap_id)
    .bind(vote.snap_revision as i32)
    .bind(vote.vote_up)
    .execute(&mut *pool)
    .await?
    .rows_affected();

    Ok(rows_affected)
}
