use std::sync::Once;

static INIT: Once = Once::new();

pub async fn before_all() {
    INIT.call_once(|| {
        tracing_subscriber::fmt().init();
    });
}

pub async fn after_all() {}
