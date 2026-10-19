//! Round-trips the package_types migration on a scratch database, so reverting
//! never races the other db tests sharing the `ratings` database.

use super::*;
use sqlx::{
    migrate::Migrator,
    postgres::{PgConnectOptions, PgConnection},
    ConnectOptions, Executor,
};
use std::{borrow::Cow, str::FromStr};

const SCRATCH_DB: &str = "ratings_migration_test";
const PACKAGE_TYPES_VERSION: i64 = 20261019000000;
const PRE_PACKAGE_TYPES_VERSION: i64 = 20240208071037;

fn migrator_before_package_types() -> Migrator {
    let mut migrator = sqlx::migrate!("sql/migrations");
    migrator.migrations = Cow::Owned(
        migrator
            .iter()
            .filter(|m| m.version < PACKAGE_TYPES_VERSION)
            .cloned()
            .collect(),
    );
    migrator
}

async fn recreate_scratch_db() -> Result<PgConnectOptions> {
    let opts = PgConnectOptions::from_str(&Config::load()?.postgres_uri)?;
    let mut admin = opts.clone().database("postgres").connect().await?;
    admin
        .execute(format!("DROP DATABASE IF EXISTS {SCRATCH_DB} WITH (FORCE)").as_str())
        .await?;
    admin
        .execute(format!("CREATE DATABASE {SCRATCH_DB}").as_str())
        .await?;

    Ok(opts.database(SCRATCH_DB))
}

async fn drop_scratch_db(opts: &PgConnectOptions) -> Result<()> {
    let mut admin = opts.clone().database("postgres").connect().await?;
    admin
        .execute(format!("DROP DATABASE IF EXISTS {SCRATCH_DB} WITH (FORCE)").as_str())
        .await?;

    Ok(())
}

/// Every column, index and constraint in the public schema, one line each.
async fn schema(conn: &mut PgConnection) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        r#"
        SELECT format('column %s.%s %s(%s) nullable=%s default=%s',
                      table_name, column_name, data_type, character_maximum_length,
                      is_nullable, column_default)
        FROM information_schema.columns
        WHERE table_schema = 'public'
        UNION ALL
        SELECT format('index %s', indexdef)
        FROM pg_indexes
        WHERE schemaname = 'public'
        UNION ALL
        SELECT format('constraint %s.%s %s', conrelid::regclass, conname, pg_get_constraintdef(oid))
        FROM pg_constraint
        WHERE connamespace = 'public'::regnamespace
        ORDER BY 1
        "#,
    )
    .fetch_all(&mut *conn)
    .await?;

    Ok(rows.into_iter().map(|(line,)| line).collect())
}

/// (snap id, total votes, positive votes) per snap, read through the given id column.
async fn snap_tallies(conn: &mut PgConnection, id_column: &str) -> Result<Vec<(String, i64, i64)>> {
    let tallies = sqlx::query_as(&format!(
        "SELECT {id_column}::TEXT, COUNT(*), COUNT(*) FILTER (WHERE vote_up)
         FROM votes GROUP BY 1 ORDER BY 1"
    ))
    .fetch_all(&mut *conn)
    .await?;

    Ok(tallies)
}

#[cfg_attr(not(feature = "db_tests"), ignore)]
#[tokio::test]
async fn package_types_migration_round_trips_snap_votes() -> Result<()> {
    let opts = recreate_scratch_db().await?;
    let mut conn = opts.connect().await?;
    let full = sqlx::migrate!("sql/migrations");

    migrator_before_package_types().run(&mut conn).await?;
    let schema_before = schema(&mut conn).await?;

    // 40 users voting on 25 snaps across up to 3 revisions each, at the old schema.
    conn.execute(
        r#"
        INSERT INTO users (client_hash, last_seen)
        SELECT lpad(to_hex(u), 64, '0'), NOW() FROM generate_series(1, 40) u;

        INSERT INTO votes (user_id_fk, snap_id, snap_revision, vote_up)
        SELECT u.id, lpad(to_hex(s), 32, 'a'), r, (u.id + s + r) % 3 <> 0
        FROM users u, generate_series(1, 25) s, generate_series(1, 3) r
        WHERE (u.id * s + r) % 4 <> 0;
        "#,
    )
    .await?;
    let tallies_before = snap_tallies(&mut conn, "snap_id").await?;
    assert!(!tallies_before.is_empty());

    full.run(&mut conn).await?;

    assert_eq!(snap_tallies(&mut conn, "app_id").await?, tallies_before);
    let non_snap: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM votes WHERE package_type <> 0")
        .fetch_one(&mut conn)
        .await?;
    assert_eq!(non_snap, 0, "existing votes must become snap votes");

    // A re-vote on a migrated row updates it in place through the new upsert.
    let (client_hash, snap_id, revision, vote_up): (String, String, i32, bool) = sqlx::query_as(
        "SELECT users.client_hash, votes.app_id, votes.snap_revision, votes.vote_up
         FROM votes JOIN users ON users.id = votes.user_id_fk LIMIT 1",
    )
    .fetch_one(&mut conn)
    .await?;
    Vote {
        client_hash,
        app_id: snap_id.clone(),
        package_type: PackageType::Snap,
        snap_revision: revision as u32,
        package_version: None,
        vote_up: !vote_up,
        timestamp: sqlx::types::time::OffsetDateTime::now_utc(),
    }
    .save_to_db(&mut conn)
    .await?;
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM votes")
        .fetch_one(&mut conn)
        .await?;
    let expected_total: i64 = tallies_before.iter().map(|(_, total, _)| total).sum();
    assert_eq!(total, expected_total, "re-vote must not add a row");

    // Rows the old schema cannot hold; the down migration must delete them.
    conn.execute(
        r#"
        INSERT INTO votes (user_id_fk, app_id, package_type, package_version, vote_up)
        VALUES (1, 'org.gnome.Calculator', 1, '46.1', true);

        INSERT INTO votes (user_id_fk, app_id, snap_revision, vote_up)
        VALUES (1, repeat('b', 33), 1, true);
        "#,
    )
    .await?;
    let tallies_after_revote = snap_tallies(&mut conn, "app_id").await?;

    full.undo(&mut conn, PRE_PACKAGE_TYPES_VERSION).await?;

    assert_eq!(schema(&mut conn).await?, schema_before);
    let mut expected_after_down = tallies_after_revote;
    expected_after_down.retain(|(id, _, _)| id.len() == 32);
    assert_eq!(
        snap_tallies(&mut conn, "snap_id").await?,
        expected_after_down
    );

    full.run(&mut conn).await?;
    assert_eq!(
        snap_tallies(&mut conn, "app_id").await?,
        expected_after_down
    );

    drop(conn);
    drop_scratch_db(&opts).await?;

    Ok(())
}
