use crate::db::Result;
use sqlx::{PgConnection, Postgres, QueryBuilder};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    sqlx::Type,
    strum::EnumString,
    strum::Display,
    strum::FromRepr,
)]
#[repr(i32)]
pub enum Category {
    ArtAndDesign = 0,
    BookAndReference = 1,
    Development = 2,
    DevicesAndIot = 3,
    Education = 4,
    Entertainment = 5,
    Featured = 6,
    Finance = 7,
    Games = 8,
    HealthAndFitness = 9,
    MusicAndAudio = 10,
    NewsAndWeather = 11,
    Personalisation = 12,
    PhotoAndVideo = 13,
    Productivity = 14,
    Science = 15,
    Security = 16,
    ServerAndCloud = 17,
    Social = 18,
    Utilities = 19,
}

pub async fn snap_has_categories(snap_id: &str, conn: &mut PgConnection) -> Result<bool> {
    let (n_rows,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM snap_categories WHERE snap_id = $1;")
            .bind(snap_id)
            .fetch_one(conn)
            .await?;

    Ok(n_rows > 0)
}

pub async fn set_categories_for_snap(
    snap_id: &str,
    categories: Vec<Category>,
    conn: &mut PgConnection,
) -> Result<()> {
    let mut query_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("INSERT INTO snap_categories(snap_id, category) ");

    query_builder.push_values(categories, |mut b, category| {
        b.push_bind(snap_id).push_bind(category);
    });

    query_builder.build().execute(conn).await?;

    Ok(())
}
