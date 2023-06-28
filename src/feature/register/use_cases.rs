use super::errors::RegisterError;
use crate::app::infrastructure::Infrastructure;
use rand::{distributions::Alphanumeric, Rng};

type Token = String;
type UserId = str;

#[tracing::instrument]
pub async fn create_user(uid: &UserId, infra: &Infrastructure) -> Result<Token, RegisterError> {
    tracing::info!("");

    if !validate_uid(&uid) {
        return Err(RegisterError::InvalidUid);
    }

    let token = create_token(&uid);
    create_user_in_db(token, infra).await
}

async fn create_user_in_db(token: Token, infra: &Infrastructure) -> Result<Token, RegisterError> {
    let mut pool = match infra.postgres.acquire().await {
        Ok(p) => p,
        Err(_) => return Err(RegisterError::FailedToCreateUserRecord),
    };

    sqlx::query("INSERT INTO users (token) VALUES ($1)")
        .bind(&token)
        .execute(&mut pool)
        .await
        .map_err(|err| {
            tracing::error!("Failed to insert user into the db: {err:?}");
            RegisterError::FailedToCreateUserRecord
        })
        .and_then(|pg_result| {
            let rows_affected = pg_result.rows_affected();

            if rows_affected == 1 {
                Ok(token)
            } else {
                tracing::error!("user insert changed {rows_affected} row(s) but 1 expected");
                Err(RegisterError::FailedToCreateUserRecord)
            }
        })
}

pub const EXPECTED_UID_LENGTH: usize = 64;
pub const TOKEN_LENGTH: usize = 32;

fn validate_uid(uid: &UserId) -> bool {
    uid.len() == EXPECTED_UID_LENGTH
}

fn create_token(_: &UserId) -> Token {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(TOKEN_LENGTH)
        .map(char::from)
        .collect()
}
