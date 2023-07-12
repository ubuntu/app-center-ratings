use std::sync::Arc;

use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use ratings::utils::env;

use crate::helpers::infrastructure;

static INITIALIZATION_FLAG: Lazy<Arc<Mutex<bool>>> = Lazy::new(|| Arc::new(Mutex::new(false)));

pub async fn before_all() {
    let mutex = Arc::clone(&*INITIALIZATION_FLAG);
    let mut initialised = mutex.lock().await;

    if !*initialised {
        *initialised = true;

        tracing_subscriber::fmt().init();
        env::init();
        infrastructure::init().await;
    }
}

pub async fn after_all() {}
