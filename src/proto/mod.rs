#[expect(
    clippy::result_large_err,
    reason = "tonic autogenerates large Status enum, but it's idiomatic usage"
)]
pub mod app {
    include!("ratings.features.app.rs");
}
#[expect(
    clippy::result_large_err,
    reason = "tonic autogenerates large Status enum, but it's idiomatic usage"
)]
pub mod user {
    include!("ratings.features.user.rs");
}
pub mod common {
    include!("ratings.features.common.rs");
}
#[expect(
    clippy::result_large_err,
    reason = "tonic autogenerates large Status enum, but it's idiomatic usage"
)]
pub mod chart {
    include!("ratings.features.chart.rs");
}
