use sqlx::Row;
use tracing::error;

use crate::{
    app::AppContext,
    features::user::{
        entities::{User, Vote},
        errors::UserError,
    },
};

pub(crate) async fn create_or_seen_user(
    app_ctx: &AppContext,
    user: User,
) -> Result<User, UserError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            UserError::FailedToCreateUserRecord
        })?;

    let result = sqlx::query(
        r#"
        INSERT INTO users (client_hash, created, last_seen)
        VALUES ($1, NOW(), NOW())
        ON CONFLICT (client_hash)
        DO UPDATE SET last_seen = NOW()
        RETURNING id;
        "#,
    )
    .bind(&user.client_hash)
    .fetch_one(&mut *pool)
    .await
    .map_err(|error| {
        error!("{error:?}");
        UserError::FailedToCreateUserRecord
    })?;

    let user_with_id = result
        .try_get("id")
        .map(|id| User { id, ..user })
        .map_err(|error| {
            error!("{error:?}");
            UserError::FailedToCreateUserRecord
        })?;

    Ok(user_with_id)
}

pub(crate) async fn delete_user_by_client_hash(
    app_ctx: &AppContext,
    client_hash: &str,
) -> Result<u64, UserError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            UserError::FailedToDeleteUserRecord
        })?;

    let rows = sqlx::query(
        r#"
        DELETE FROM users
        WHERE client_hash = $1
        "#,
    )
    .bind(client_hash)
    .execute(&mut *pool)
    .await
    .map_err(|error| {
        error!("{error:?}");
        UserError::FailedToDeleteUserRecord
    })?;

    Ok(rows.rows_affected())
}

pub(crate) async fn get_snap_votes_by_client_hash(
    app_ctx: &AppContext,
    snap_id: String,
    client_hash: String,
) -> Result<Vec<Vote>, UserError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            UserError::FailedToGetUserVote
        })?;

    let result = sqlx::query(
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
                    votes.snap_id = $2
        "#,
    )
    .bind(client_hash.clone())
    .bind(snap_id)
    .fetch_all(&mut *pool)
    .await
    .map_err(|error| {
        error!("{error:?}");
        UserError::Unknown
    })?;

    let votes: Vec<Vote> = result
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

pub(crate) async fn save_vote_to_db(app_ctx: &AppContext, vote: Vote) -> Result<u64, UserError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            UserError::FailedToCastVote
        })?;

    let result = sqlx::query(
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
    .await
    .map_err(|error| {
        error!("{error:?}");
        UserError::FailedToCastVote
    })?;

    Ok(result.rows_affected())
}

pub(crate) async fn find_user_votes(
    app_ctx: &AppContext,
    client_hash: String,
    snap_id_filter: Option<String>,
) -> Result<Vec<Vote>, UserError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            UserError::Unknown
        })?;

    let result = sqlx::query(
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
    .await
    .map_err(|error| {
        error!("{error:?}");
        UserError::Unknown
    })?;

    let votes: Vec<Vote> = result
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
