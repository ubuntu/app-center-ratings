use sqlx::FromRow;
use time::OffsetDateTime;

pub type UserId = i32;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: UserId,
    pub user_id: String,
    pub created: OffsetDateTime,
    pub last_seen: OffsetDateTime,
}

impl User {
    pub fn new(user_id: &str) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: -1,
            user_id: user_id.to_string(),
            last_seen: now,
            created: now,
        }
    }
}
