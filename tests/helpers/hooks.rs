use std::sync::{Arc, Once};

use once_cell::sync::Lazy;
use ratings::utils::Migrator;
use tokio::sync::Mutex;

static INIT: Once = Once::new();
static TEST_COUNTER: Lazy<Arc<Mutex<i32>>> = Lazy::new(|| Arc::new(Mutex::new(0)));

pub async fn before_all(migrator: Migrator) {
    INIT.call_once(|| {
        tracing_subscriber::fmt().init();
    });
    if let Err(e) = migrator.run().await {
        panic!("{}", e)
    }

    let counter = Arc::clone(&*TEST_COUNTER);
    let mut test_counter = counter.lock().await;
    *test_counter += 1;
}

pub async fn after_all(migrator: Migrator) {
    let counter = Arc::clone(&*TEST_COUNTER);
    let mut test_counter = counter.lock().await;
    *test_counter -= 1;
    if *test_counter == 0 {
        if let Err(e) = migrator.revert().await {
            panic!("{}", e)
        }
    }
}
