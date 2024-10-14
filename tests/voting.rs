mod common;

use common::TestHelper;

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

#[tokio::test]
async fn upvoting_increases_vote_count() -> anyhow::Result<()> {
    let t = TestHelper::new();
    let user_token = t.authenticate(t.random_sha_256()).await?;
    let snap_id = t.random_id();
    let snap_revision = 1;

    // Initial state: 3 upvote & 2 downvotes
    t.generate_votes(&snap_id, snap_revision, true, 3).await?;
    t.generate_votes(&snap_id, snap_revision, false, 2).await?;

    let initial_rating = t
        .get_rating(&snap_id, &user_token)
        .await?
        .expect("to have an initial rating");
    assert_eq!(initial_rating.total_votes, 5, "initial total votes");

    // Vote with a user who has not previously voted for this snap
    t.vote(&snap_id, snap_revision, true, &user_token).await?;

    let rating = t
        .get_rating(&snap_id, &user_token)
        .await?
        .expect("to have an updated rating");

    assert_eq!(rating.total_votes, 6, "total votes");

    Ok(())
}
