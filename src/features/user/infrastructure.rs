//! Infrastructure for user handling
use serde::{de::DeserializeOwned, Deserialize};
use sqlx::{Acquire, Executor, Row};
use tracing::error;

use crate::{
    app::AppContext,
    features::{
        pb::chart::Category,
        user::{
            entities::{User, Vote},
            errors::UserError,
        },
    },
};

/// Create a [`User`] entry, or note that the user has recently been seen, within the current
/// [`AppContext`].
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

/// Deletes a [`User`] with the given [`ClientHash`]
///
/// [`ClientHash`]: crate::features::user::entities::ClientHash
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

/// Gets votes for a snap with the given ID from a given [`ClientHash`]
///
/// [`ClientHash`]: crate::features::user::entities::ClientHash
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

/// Saves a [`Vote`] to the database, if possible.
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

async fn get_json<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: reqwest::Url,
    query: &[(&str, &str)],
) -> Result<T, UserError> {
    let s = client
        .get(url)
        .header("User-Agent", "ratings-service")
        .header("Snap-Device-Series", 16)
        .query(query)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Ok(serde_json::from_str(&s)?)
}

/// Pull snap categories by for a given snapd_id from the snapcraft.io rest API
async fn get_snap_categories(
    snap_id: &str,
    base: &str,
    client: &reqwest::Client,
) -> Result<Vec<Category>, UserError> {
    let base_url = reqwest::Url::parse(base).map_err(|_| UserError::Unknown)?;

    let assertions_url = base_url
        .join(&format!("assertions/snap-declaration/16/{snap_id}"))
        .map_err(|_| UserError::Unknown)?;
    let AssertionsResp {
        headers: Headers { snap_name },
    } = get_json(client, assertions_url, &[]).await?;

    let info_url = base_url
        .join(&format!("snaps/info/{snap_name}"))
        .map_err(|_| UserError::Unknown)?;
    let FindResp {
        snap: SnapInfo { categories },
    } = get_json(client, info_url, &[("fields", "categories")]).await?;

    let res: Result<Vec<Category>, UserError> = categories
        .into_iter()
        .map(|c| Category::try_from(c.name.as_str()).map_err(|_| UserError::Unknown))
        .collect();

    return res;

    // serde structs

    #[derive(Debug, Deserialize)]
    struct AssertionsResp {
        headers: Headers,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    struct Headers {
        snap_name: String,
    }

    #[derive(Debug, Deserialize)]
    struct FindResp {
        snap: SnapInfo,
    }

    #[derive(Debug, Deserialize)]
    struct SnapInfo {
        categories: Vec<RawCategory>,
    }

    #[derive(Debug, Deserialize)]
    struct RawCategory {
        name: String,
    }
}

/// Update the category (we do this every time we get a vote for the time being)
pub(crate) async fn update_category(app_ctx: &AppContext, snap_id: &str) -> Result<(), UserError> {
    let mut pool = app_ctx
        .infrastructure()
        .repository()
        .await
        .map_err(|error| {
            error!("{error:?}");
            UserError::Unknown
        })?;

    let client = app_ctx.http_client();
    let base = &app_ctx.config().snapcraft_io_uri;
    let categories = get_snap_categories(snap_id, base, client).await?;

    // Do a transaction because bulk querying doesn't seem to work cleanly
    let mut tx = pool.begin().await?;

    // Reset the categories since we're refreshing all of them
    tx.execute(
        sqlx::query("DELETE FROM snap_categories WHERE snap_categories.snap_id = $1;")
            .bind(snap_id),
    )
    .await?;

    for category in categories.iter() {
        tx.execute(
            sqlx::query("INSERT INTO snap_categories (snap_id, category) VALUES ($1, $2); ")
                .bind(snap_id)
                .bind(category),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Retrieve all votes for a given [`User`], within the current [`AppContext`].
///
/// May be filtered for a given snap ID.
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

#[cfg(test)]
mod tests {
    use super::*;

    // Can be run explicitly to validate the behaviour of the API calls we make against
    // snapcraft.io but we don't want to do this in local testing or CI by default.
    #[ignore = "hits snapcraft.io"]
    #[tokio::test]
    async fn get_snap_categories_works() {
        let client = reqwest::Client::new();
        let base = "https://api.snapcraft.io/v2/";
        let snap_id = "NeoQngJVBf2wKC48bxnF2xqmfEFGdVnx"; // steam
        let categories = get_snap_categories(snap_id, base, &client).await.unwrap();

        assert_eq!(categories, vec![Category::Games]);
    }
}
