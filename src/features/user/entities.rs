use sqlx::FromRow;
use time::format_description::modifier::UnixTimestamp;

#[derive(Debug, FromRow)]
pub struct User {
    id: i32,
    instance_id: String,
    last_seen: UnixTimestamp,
    first_seen: UnixTimestamp,
}
