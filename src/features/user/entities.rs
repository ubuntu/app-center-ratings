use sqlx::FromRow;
use time::OffsetDateTime;

pub type UserId = String;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i32,
    pub user_id: UserId,
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

#[derive(Debug, Clone)]
pub struct Vote {
    pub user_id: UserId,
    pub snap_id: String,
    pub snap_revision: u32,
    pub vote_up: bool,
}
