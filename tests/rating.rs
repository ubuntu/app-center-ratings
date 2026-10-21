pub mod common;

use common::TestHelper;
use ratings::proto::{
    app::{AppRating, RatingTarget},
    common::{PackageType, RatingsBand},
};
use simple_test_case::test_case;
use tonic::Code;

fn target(id: &str, package_type: PackageType) -> RatingTarget {
    RatingTarget {
        id: id.to_string(),
        package_type: package_type.into(),
    }
}

/// Fetches the Rating of a single App through GetRatings
async fn get_one_rating(
    t: &TestHelper,
    target: RatingTarget,
    token: &str,
) -> anyhow::Result<AppRating> {
    let mut ratings = t.get_ratings(vec![target], token).await?;
    assert_eq!(ratings.len(), 1, "{ratings:?}");

    Ok(ratings.remove(0))
}

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

#[tokio::test]
async fn deb_votes_are_excluded_from_snap_ratings() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;

    let snap_id = t.test_snap_with_initial_votes(1, 25, 0, &[]).await?;
    // A deb sharing the snap's App ID must not be counted towards it
    t.generate_deb_votes(&snap_id, "1.0", false, 5).await?;

    let rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(rating.total_votes, 25, "GetRating");

    let ratings = t
        .get_bulk_ratings(vec![snap_id.clone()], &user_token)
        .await?;
    assert_eq!(ratings.len(), 1, "GetBulkRatings: {ratings:?}");
    let rating = ratings[0].rating.as_ref().unwrap();
    assert_eq!(rating.snap_id, snap_id);
    assert_eq!(rating.total_votes, 25, "GetBulkRatings");

    // GetRating must report zero votes for a mock-known ID that only has deb votes
    let deb_only_id = t.test_snap_with_initial_votes(1, 0, 0, &[]).await?;
    t.generate_deb_votes(&deb_only_id, "1.0", true, 5).await?;

    let rating = t.get_rating(&deb_only_id, &user_token).await?;
    assert_eq!(rating.total_votes, 0, "GetRating, deb-only");

    Ok(())
}

#[tokio::test]
async fn deb_vote_increments_get_ratings_total() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let app_id = format!("org.example.{}", t.random_id());
    t.generate_deb_votes(&app_id, "1.0", true, 3).await?;
    let deb = target(&app_id, PackageType::Deb);

    t.vote_deb(&app_id, "1.0", false, &user_token).await?;

    let rating = get_one_rating(&t, deb.clone(), &user_token).await?;
    assert_eq!(rating.target, Some(deb));
    assert_eq!(rating.total_votes, 4);

    Ok(())
}

#[tokio::test]
async fn changing_a_deb_vote_doesnt_alter_get_ratings_total() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let app_id = format!("org.example.{}", t.random_id());
    t.generate_deb_votes(&app_id, "1.0", true, 3).await?;
    let deb = target(&app_id, PackageType::Deb);

    t.vote_deb(&app_id, "1.0", true, &user_token).await?;
    t.vote_deb(&app_id, "1.0", false, &user_token).await?;

    let rating = get_one_rating(&t, deb, &user_token).await?;
    assert_eq!(rating.total_votes, 4);

    Ok(())
}

#[tokio::test]
async fn deb_votes_on_different_versions_aggregate_into_one_rating() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let app_id = format!("org.example.{}", t.random_id());
    t.generate_deb_votes(&app_id, "1.0", true, 20).await?;
    t.generate_deb_votes(&app_id, "2.0", true, 10).await?;
    // The same user voting on a new version counts again
    t.vote_deb(&app_id, "1.0", true, &user_token).await?;
    t.vote_deb(&app_id, "2.0", true, &user_token).await?;

    let rating = get_one_rating(&t, target(&app_id, PackageType::Deb), &user_token).await?;
    assert_eq!(rating.total_votes, 32);
    assert_eq!(rating.ratings_band, RatingsBand::VeryGood as i32);
    assert!(rating.raw_rating > 0.8, "raw_rating: {}", rating.raw_rating);

    Ok(())
}

#[tokio::test]
async fn get_ratings_returns_mixed_targets_in_request_order() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_id = t.test_snap_with_initial_votes(1, 4, 0, &[]).await?;
    let deb_id = format!("org.example.{}", t.random_id());
    t.generate_deb_votes(&deb_id, "1.0", false, 2).await?;
    let unvoted_id = format!("org.example.{}", t.random_id());

    let targets = vec![
        target(&deb_id, PackageType::Deb),
        target(&unvoted_id, PackageType::Deb),
        target(&snap_id, PackageType::Snap),
    ];
    let ratings = t.get_ratings(targets.clone(), &user_token).await?;

    let returned: Vec<_> = ratings.iter().map(|r| r.target.clone().unwrap()).collect();
    assert_eq!(returned, targets);
    let totals: Vec<_> = ratings.iter().map(|r| r.total_votes).collect();
    assert_eq!(totals, vec![2, 0, 4]);

    Ok(())
}

#[tokio::test]
async fn get_ratings_for_an_app_without_votes_is_insufficient() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let app_id = format!("org.example.{}", t.random_id());

    let rating = get_one_rating(&t, target(&app_id, PackageType::Deb), &user_token).await?;
    assert_eq!(rating.total_votes, 0);
    assert_eq!(rating.ratings_band, RatingsBand::InsufficientVotes as i32);
    assert_eq!(rating.raw_rating, 0.0);

    Ok(())
}

#[tokio::test]
async fn snap_and_deb_with_the_same_app_id_get_separate_ratings() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let app_id = t.test_snap_with_initial_votes(1, 3, 0, &[]).await?;
    t.generate_deb_votes(&app_id, "1.0", false, 5).await?;

    let ratings = t
        .get_ratings(
            vec![
                target(&app_id, PackageType::Snap),
                target(&app_id, PackageType::Deb),
            ],
            &user_token,
        )
        .await?;

    let totals: Vec<_> = ratings.iter().map(|r| r.total_votes).collect();
    assert_eq!(totals, vec![3, 5]);

    Ok(())
}

#[tokio::test]
async fn get_ratings_matches_get_rating_for_a_snap() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_id = t.test_snap_with_initial_votes(1, 30, 10, &[]).await?;

    let expected = t.get_rating(&snap_id, &user_token).await?;
    let rating = get_one_rating(&t, target(&snap_id, PackageType::Snap), &user_token).await?;

    assert_eq!(rating.total_votes, expected.total_votes);
    assert_eq!(rating.ratings_band, expected.ratings_band as i32);

    Ok(())
}

#[tokio::test]
async fn get_ratings_returns_duplicate_targets_once() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap = target(&t.random_id(), PackageType::Snap);
    let deb = target(&format!("org.example.{}", t.random_id()), PackageType::Deb);

    let ratings = t
        .get_ratings(
            vec![deb.clone(), snap.clone(), deb.clone(), snap.clone()],
            &user_token,
        )
        .await?;

    let returned: Vec<_> = ratings.iter().map(|r| r.target.clone().unwrap()).collect();
    assert_eq!(returned, vec![deb, snap]);

    Ok(())
}

fn deb_targets(count: usize) -> Vec<RatingTarget> {
    (0..count)
        .map(|i| target(&format!("org.example.app{i}"), PackageType::Deb))
        .collect()
}

#[test_case(vec![]; "empty target list")]
#[test_case(deb_targets(251); "more than 250 targets")]
#[test_case(vec![target("", PackageType::Snap)]; "empty app id")]
#[test_case(vec![target(&"a".repeat(256), PackageType::Deb)]; "app id over 255 bytes")]
#[test_case(vec![RatingTarget { id: "org.example.app".into(), package_type: 2 }]; "unknown package type")]
#[tokio::test]
async fn malformed_get_ratings_is_rejected(targets: Vec<RatingTarget>) -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;

    let err = t
        .get_ratings(targets, &user_token)
        .await
        .expect_err("GetRatings should fail");

    let status = err
        .downcast_ref::<tonic::Status>()
        .expect("Error should be a tonic::Status");
    assert_eq!(status.code(), Code::InvalidArgument, "{status:?}");

    Ok(())
}

#[tokio::test]
async fn get_ratings_accepts_250_targets_and_255_byte_app_ids() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let mut targets = deb_targets(249);
    targets.push(target(&"a".repeat(255), PackageType::Deb));

    let ratings = t.get_ratings(targets, &user_token).await?;
    assert_eq!(ratings.len(), 250);

    Ok(())
}
