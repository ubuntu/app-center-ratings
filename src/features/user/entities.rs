use sqlx::FromRow;
use time::OffsetDateTime;

use crate::features::pb::user;

pub type ClientHash = String;

#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i32,
    pub client_hash: ClientHash,
    pub created: OffsetDateTime,
    pub last_seen: OffsetDateTime,
}

impl User {
    pub fn new(client_hash: &str) -> Self {
        let now = OffsetDateTime::now_utc();
        Self {
            id: -1,
            client_hash: client_hash.to_string(),
            last_seen: now,
            created: now,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Vote {
    pub client_hash: ClientHash,
    pub snap_id: String,
    pub snap_revision: u32,
    pub vote_up: bool,
    pub timestamp: OffsetDateTime,
}

impl Vote {
    pub(crate) fn into_dto(self) -> user::Vote {
        let timestamp = Some(prost_types::Timestamp {
            seconds: self.timestamp.unix_timestamp(),
            nanos: 0,
        });

        user::Vote {
            snap_id: self.snap_id,
            snap_revision: self.snap_revision as i32,
            vote_up: self.vote_up,
            timestamp,
        }
    }
}
