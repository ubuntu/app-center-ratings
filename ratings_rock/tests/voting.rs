pub mod common;

use common::{Category, TestHelper};
use ratings::ratings::RatingsBand::{self, *};
use simple_test_case::test_case;

#[test_case(true; "up vote")]
#[test_case(false; "down vote")]
#[tokio::test]
async fn voting_increases_vote_count(vote_up: bool) -> anyhow::Result<()> {
    let t = TestHelper::new();

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_revision = 1;
    let snap_id = t
        .test_snap_with_initial_votes(snap_revision, 3, 2, &[Category::Social])
        .await?;

    let initial_rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(initial_rating.total_votes, 5, "initial total votes");

    // Vote with a user who has not previously voted for this snap
    t.vote(&snap_id, snap_revision, vote_up, &user_token)
        .await?;

    let rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(rating.total_votes, 6, "total votes: vote_up={vote_up}");

    Ok(())
}

#[test_case(true; "up to down vote")]
#[test_case(false; "down to up vote")]
#[tokio::test]
async fn changing_your_vote_doesnt_alter_total(initial_up: bool) -> anyhow::Result<()> {
    let t = TestHelper::new();

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_revision = 1;
    let snap_id = t
        .test_snap_with_initial_votes(snap_revision, 3, 2, &[Category::Social])
        .await?;

    let initial_rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(initial_rating.total_votes, 5, "initial total votes");

    // Vote with a user who has not previously voted for this snap
    t.vote(&snap_id, snap_revision, initial_up, &user_token)
        .await?;

    let rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(rating.total_votes, 6, "total votes");

    // That user changing their vote shouldn't alter the total
    t.vote(&snap_id, snap_revision, !initial_up, &user_token)
        .await?;

    let rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(rating.total_votes, 6, "total votes");

    Ok(())
}

// The ratings bands details are found in ../src/features/common/entities.rs and the following
// test expects the break points for the value of the confidence interval:
//
//   0.80 < r          - VeryGood
//   0.55 < r <= 0.80  - Good
//   0.45 < r <= 0.55  - Neutral
//   0.20 < r <= 0.45  - Poor
//          r <= 0.20  - VeryPoor
//
// NOTE: In order to generate a rating we need to have at least 25 votes
#[test_case(true, Neutral, Good; "neutral to good")]
#[test_case(false, Neutral, Poor; "neutral to poor")]
#[tokio::test]
async fn voting_updates_ratings_band(
    vote_up: bool,
    initial_band: RatingsBand,
    new_band: RatingsBand,
) -> anyhow::Result<()> {
    let t = TestHelper::new();

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_revision = 1;
    let snap_id = t
        .test_snap_with_initial_votes(snap_revision, 60, 40, &[Category::Games])
        .await?;

    let r = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(r.ratings_band, initial_band, "initial band");

    t.generate_votes(&snap_id, snap_revision, vote_up, 50)
        .await?;

    let r = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(r.ratings_band, new_band, "new band");

    Ok(())
}

#[tokio::test]
async fn voting_on_a_snap_without_categories_works() -> anyhow::Result<()> {
    let t = TestHelper::new();

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_revision = 1;
    let snap_id = t
        .test_snap_with_initial_votes(snap_revision, 3, 2, &[])
        .await?;

    let initial_rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(initial_rating.total_votes, 5, "initial total votes");

    // Vote with a user who has not previously voted for this snap
    t.vote(&snap_id, snap_revision, true, &user_token).await?;

    let rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(rating.total_votes, 6, "total votes: vote_up=true");

    Ok(())
}
