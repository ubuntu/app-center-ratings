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
    .bind(user.last_seen)
    .bind(user.created)
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
        DO UPDATE SET vote_up = EXCLUDED.vote_up;
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

pub(crate) async fn find_user_votes(
    client_hash: String,
    snap_id_filter: Option<String>,
) -> Result<Vec<Vote>, sqlx::Error> {
    let mut pool = get_repository().await?;

    let rows = sqlx::query(
        r#"
                SELECT
                    votes.id,
                    votes.created,
                    votes.snap_id,
                    votes.snap_revision,
                    votes.vote_up
                FROM
                    users
                INNER JOIN
                    votes
                ON
                    users.id = votes.user_id_fk
                WHERE
                    users.client_hash = $1
                AND
                    ($2 IS NULL OR votes.snap_id = $2);
            "#,
    )
    .bind(client_hash.clone())
    .bind(snap_id_filter)
    .fetch_all(&mut *pool)
    .await?;

    let votes: Vec<Vote> = rows
        .into_iter()
        .map(|row| Vote {
            client_hash: client_hash.clone(),
            snap_id: row.get("snap_id"),
            snap_revision: row.get::<i32, _>("snap_revision") as u32,
            vote_up: row.get("vote_up"),
            timestamp: row.get("created"),
        })
        .collect();

    Ok(votes)
}
