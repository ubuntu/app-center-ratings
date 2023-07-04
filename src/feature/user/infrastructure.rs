use crate::app::INFRA;

use super::{errors::RegisterError, use_cases::UserId};

pub(crate) async fn create_user_in_db(uid: &UserId) -> Result<UserId, RegisterError> {
    let infra = INFRA.get().expect("Infrastructure should be initialised");
    let mut pool = match infra.postgres.acquire().await {
        Ok(p) => p,
        Err(_) => return Err(RegisterError::FailedToCreateUserRecord),
    };

    sqlx::query("INSERT INTO users (token) VALUES ($1)")
        .bind(&uid)
        .execute(&mut pool)
        .await
        .map_err(|err| {
            tracing::error!("Failed to insert user into the db: {err:?}");
            RegisterError::FailedToCreateUserRecord
        })
        .and_then(|pg_result| {
            let rows_affected = pg_result.rows_affected();

            if rows_affected == 1 {
                Ok(uid.to_string())
            } else {
                tracing::error!("user insert changed {rows_affected} row(s) but 1 expected");
                Err(RegisterError::FailedToCreateUserRecord)
            }
        })
}
