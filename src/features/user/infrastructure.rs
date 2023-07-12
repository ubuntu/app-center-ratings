use sqlx::{Acquire, Row};

use crate::utils::infrastructure::get_repository;

use super::entities::User;

pub(crate) async fn create_user_in_db(user: User) -> Result<User, sqlx::Error> {
    let mut connection = get_repository().await;
    let mut tx = connection.begin().await?;

    let row = sqlx::query(
        r#"
        INSERT INTO users (instance_id, created, last_seen)
        VALUES ($1, $2, $2)
        RETURNING id
    "#,
    )
    .bind(&user.instance_id)
    .bind(&user.last_seen)
    .bind(&user.created)
    .fetch_one(&mut *tx)
    .await?;

    let id = row.try_get("id")?;

    tx.commit().await.unwrap();

    let user_with_id = User { id, ..user };

    Ok(user_with_id)
}
