use std::future::Future;

use crate::helpers::hooks::{after_all, before_all};
use ratings::utils::Migrator;

pub async fn with_lifecycle<F>(f: F, migrator: Migrator)
where
    F: Future<Output = ()>,
{
    before_all(migrator.clone()).await;
    f.await;
    after_all(migrator.clone()).await;
}
