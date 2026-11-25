pub mod common;

use common::TestHelper;
use tonic::Code;

#[tokio::test]
async fn get_bulk_ratings_success() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;

    let snap_id_1 = t.test_snap_with_initial_votes(1, 30, 10, &[]).await?;
    let snap_id_2 = t.test_snap_with_initial_votes(1, 50, 5, &[]).await?;

    let snap_ids = vec![snap_id_1.clone(), snap_id_2.clone()];
    let ratings = t.get_bulk_ratings(snap_ids, &user_token).await?;

    assert_eq!(ratings.len(), 2);

    let chart_data_1 = ratings
        .iter()
        .find(|cd| cd.rating.as_ref().map_or(false, |r| r.snap_id == snap_id_1))
        .expect("Chart data for snap_id_1 not found");

    let rating_1 = chart_data_1.rating.as_ref().unwrap();
    assert_eq!(rating_1.total_votes, 40);
    assert_eq!(rating_1.snap_id, snap_id_1);

    let chart_data_2 = ratings
        .iter()
        .find(|cd| cd.rating.as_ref().map_or(false, |r| r.snap_id == snap_id_2))
        .expect("Chart data for snap_id_2 not found");

    let rating_2 = chart_data_2.rating.as_ref().unwrap();
    assert_eq!(rating_2.total_votes, 55);
    assert_eq!(rating_2.snap_id, snap_id_2);

    Ok(())
}

#[tokio::test]
async fn get_bulk_ratings_partial_results() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;

    let snap_id_1 = t.test_snap_with_initial_votes(1, 25, 0, &[]).await?;
    let snap_id_2_non_existent = t.random_id();

    let snap_ids = vec![snap_id_1.clone(), snap_id_2_non_existent];
    let ratings = t.get_bulk_ratings(snap_ids, &user_token).await?;

    assert_eq!(ratings.len(), 1);
    let rating_1 = ratings[0].rating.as_ref().unwrap();
    assert_eq!(rating_1.snap_id, snap_id_1);
    assert_eq!(rating_1.total_votes, 25);

    Ok(())
}

#[tokio::test]
async fn get_bulk_ratings_invalid_argument_empty_list() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;

    let result = t.get_bulk_ratings(vec![], &user_token).await;

    assert!(
        result.is_err(),
        "Expected get_bulk_ratings to fail for an empty list"
    );
    let err = result.err().unwrap();

    let status = err
        .downcast_ref::<tonic::Status>()
        .expect("Error should be a tonic::Status");

    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(status.message(), "snap_ids cannot be empty");

    Ok(())
}

#[tokio::test]
async fn get_bulk_ratings_invalid_argument_too_many_ids() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_ids = (0..251).map(|_| t.random_id()).collect();

    let result = t.get_bulk_ratings(snap_ids, &user_token).await;

    assert!(
        result.is_err(),
        "Expected get_bulk_ratings to fail for too many ids"
    );
    let err = result.err().unwrap();

    let status = err
        .downcast_ref::<tonic::Status>()
        .expect("Error should be a tonic::Status");

    assert_eq!(status.code(), Code::InvalidArgument);
    assert_eq!(
        status.message(),
        "Too many snap_ids requested. The maximum is 250"
    );

    Ok(())
}
