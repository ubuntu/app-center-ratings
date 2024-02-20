use ratings::{
    app::AppContext,
    features::pb::chart::Category,
    utils::{Config, Infrastructure},
};
use sqlx::Connection;
use tokio::sync::OnceCell;

const TESTING_SNAP_ID: &str = "3Iwi803Tk3KQwyD6jFiAJdlq8MLgBIoD";
const TESTING_SNAP_CATEGORIES: [Category; 2] = [Category::Utilities, Category::Development];

/// Call [`clear_test_snap`] with this at the start of category tests to clear the Test snap info,
/// this prevents it from polluting other integration tests by repeated runs eventually outstripping
/// the random data.
static CLEAR_TEST_SNAP: OnceCell<()> = OnceCell::const_new();

async fn clear_test_snap() {
    let config = Config::load().unwrap();
    let infra = Infrastructure::new(&config).await.unwrap();
    let app_ctx = AppContext::new(&config, infra);
    let mut conn = app_ctx.infrastructure().repository().await.unwrap();

    let mut tx = conn.begin().await.unwrap();

    sqlx::query("DELETE FROM votes WHERE snap_id = $1;")
        .bind(TESTING_SNAP_ID)
        .execute(&mut *tx)
        .await
        .unwrap();

    sqlx::query("DELETE FROM snap_categories WHERE snap_id = $1;")
        .bind(TESTING_SNAP_ID)
        .execute(&mut *tx)
        .await
        .unwrap();

    tx.commit().await.unwrap();
}

mod user_tests {
    mod category;
    mod double_authenticate_test;
    mod get_votes_lifecycle_test;
    mod reject_invalid_register_test;
    mod simple_lifecycle_test;
}

mod app_tests {
    mod lifecycle_test;
}

mod chart_tests {
    mod category;
    mod lifecycle_test;
}

mod helpers;
