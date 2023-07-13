use sqlx::{Acquire, Row};

use crate::utils::infrastructure::get_repository;

use super::entities::User;

pub(crate) async fn create_user_in_db(user: User) -> Result<User, sqlx::Error> {
    let mut connection = get_repository().await?;
    let mut tx = connection.begin().await?;

    let row = sqlx::query(
        r#"
        INSERT INTO users (user_id, created, last_seen)
        VALUES ($1, $2, $2)
        RETURNING id
        "#,
    )
    .bind(&user.user_id)
    .bind(&user.last_seen)
    .bind(&user.created)
    .fetch_one(&mut *tx)
    .await?;

    let id = row.try_get("id")?;
    tx.commit().await?;

    let user_with_id = User { id, ..user };

    Ok(user_with_id)
}

pub(crate) async fn delete_user_by_user_id(id: &str) -> Result<u64, sqlx::Error> {
    let mut connection = get_repository().await?;
    let mut tx = connection.begin().await?;

    let id = id.parse::<i32>().expect("should be number");
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let rows_deleted = result.rows_affected();

    Ok(rows_deleted)
}
