use std::future::Future;

use crate::helpers::hooks::{after_all, before_all};

pub async fn with_lifecycle<F>(f: F)
where
    F: Future<Output = ()>,
{
    before_all().await;
    f.await;
    after_all().await;
}
