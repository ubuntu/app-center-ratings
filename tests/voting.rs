mod common;

use common::TestHelper;
use ratings::features::common::entities::RatingsBand::{self, *};
use simple_test_case::test_case;

/*
Feature: User voting
    Background:
        Given a Snap named "chu-chu-garden" has already accumulated 5 votes and 3 upvotes

    Scenario: Amy upvotes a snap she hasn't voted for in the past
        When Amy casts an upvote
        Then the total number of votes strictly increases
        And the ratings band monotonically increases

    Rule: Votes that a user updates do not change the total vote count

        Scenario Outline: Sonic changes his vote between downvote and upvote because "chu-chu-garden" got better/worse
            Given Sonic originally voted <original>
            When Sonic changes his vote to <after>
            Then the ratings band <direction>
            But the total number of votes stays constant

            Examples:
                | original | after    | direction               |
                | upvote   | downvote | monotonically increases |
                | downvote | upvote   | monotonically decreases |
*/

#[test_case(true; "up vote")]
#[test_case(false; "down vote")]
#[tokio::test]
async fn voting_increases_vote_count(vote_up: bool) -> anyhow::Result<()> {
    let t = TestHelper::new();

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_revision = 1;
    let snap_id = t.test_snap_with_initial_votes(snap_revision, 3, 2).await?;

    let initial_rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(initial_rating.total_votes, 5, "initial total votes");

    // Vote with a user who has not previously voted for this snap
    t.vote(&snap_id, snap_revision, vote_up, &user_token)
        .await?;

    let rating = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(rating.total_votes, 6, "total votes: vote_up={vote_up}");

    Ok(())
}

/*
// The ratings bands details are found in ../src/features/common/entities.rs and the following
// test expects the break points for each band (upvote%) to be as follows:
//
//   0.80 < r          - VeryGood
//   0.55 < r <= 0.80  - Good
//   0.45 < r <= 0.55  - Neutral
//   0.20 < r <= 0.45  - Poor
//          r <= 0.20  - VeryPoor
//
// NOTE: In order to generate a rating we need to have at least 25 votes
#[test_case(70, 20, 30, true, Good, VeryGood; "good to very good")]
#[tokio::test]
async fn voting_updates_ratings_band(
    initial_up: u64,
    initial_down: u64,
    new: u64,
    vote_up: bool,
    initial_band: RatingsBand,
    new_band: RatingsBand,
) -> anyhow::Result<()> {
    let t = TestHelper::new();

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_revision = 1;
    let snap_id = t
        .test_snap_with_initial_votes(snap_revision, initial_up, initial_down)
        .await?;

    let r = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(r.ratings_band, initial_band, "initial band");

    t.generate_votes(&snap_id, snap_revision, vote_up, new)
        .await?;

    let r = t.get_rating(&snap_id, &user_token).await?;
    assert_eq!(r.ratings_band, new_band, "new band");

    Ok(())
}
*/
