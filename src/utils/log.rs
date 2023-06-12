use env_logger::{Builder, Env};

use super::env;

pub fn init() {
    let level = env::get_log_level();

    let env = Env::default().default_filter_or(level);

    let mut builder = Builder::from_env(env);

    let exclude_timestamps = env::get_env_name() != env::EnvName::Dev;
    if exclude_timestamps {
        builder.format_timestamp(None);
    }

    builder.init();
}
