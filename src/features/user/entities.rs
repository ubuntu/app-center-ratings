use sqlx::FromRow;
use time::OffsetDateTime;

pub type UserId = i32;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: UserId,
    pub instance_id: String,
    pub created: OffsetDateTime,
    pub last_seen: OffsetDateTime,
}

impl User {
    pub fn new(instance_id: &str) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: -1,
            instance_id: instance_id.to_string(),
            last_seen: now,
            created: now,
        }
    }
}
