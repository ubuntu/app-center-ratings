//! Various struct definitions for handling user data.

use sqlx::FromRow;
use time::OffsetDateTime;

use crate::features::pb::user;

/// A hash of a given user client.
pub type ClientHash = String;

/// Information about a user who may be rating snaps.
#[derive(Debug, Clone, FromRow)]
pub struct User {
    /// The user's ID
    pub id: i32,
    /// A hash of the user's client
    pub client_hash: ClientHash,
    /// The time the user was created
    pub created: OffsetDateTime,
    /// The time the user was last seen
    pub last_seen: OffsetDateTime,
}

impl User {
    /// Creates a new user from the given [`ClientHash`]
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

/// A Vote, as submitted by a user
#[derive(Debug, Clone)]
pub struct Vote {
    /// The hash of the user client
    pub client_hash: ClientHash,
    /// The ID of the snap being voted on
    pub snap_id: String,
    /// The revision of the snap being voted on
    pub snap_revision: u32,
    /// Whether this is a positive or negative vote
    pub vote_up: bool,
    /// The timestamp of the vote
    pub timestamp: OffsetDateTime,
}

impl Vote {
    /// Converts this vote into its wire component for transfer over the network.
    pub fn into_protobuf_vote(self) -> user::Vote {
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
