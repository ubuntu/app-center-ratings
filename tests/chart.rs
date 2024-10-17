// NOTE: this is not at all ideal, but in order to get tests around charts to work we need
//       to be able to control the set of snaps that are in a given category. If tests start
//       failing when you are adding new test cases then double check that you are not
//       making use of any of the Categories that the tests in this file rely on.
pub mod common;

use common::{Category, TestHelper};
use rand::{thread_rng, Rng};

// !! This test expects to be the only one making use of the "Development" category
#[tokio::test]
async fn category_chart_returns_expected_top_snap() -> anyhow::Result<()> {
    let t = TestHelper::new();

    // Generate a random set of snaps within the given category
    for _ in 0..25 {
        let client = t.clone();
        let (upvotes, downvotes) = random_votes(25, 50, 15, 35);
        client
            .test_snap_with_initial_votes(1, upvotes, downvotes, &[Category::Development])
            .await?;
    }

    // A snap that should be returned as the top snap for the category
    let snap_id = t
        .test_snap_with_initial_votes(1, 50, 0, &[Category::Development])
        .await?;

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let mut data = t
        .get_chart(Some(Category::Development), &user_token)
        .await?;

    let top_snap = data[0].rating.take().expect("to have rating for top snap");
    assert_eq!(top_snap.snap_id, snap_id, "{top_snap:?}");

    Ok(())
}

fn random_votes(min_vote: usize, max_vote: usize, min_up: usize, max_up: usize) -> (u64, u64) {
    let mut rng = thread_rng();
    let upvotes = rng.gen_range(min_up..max_up);
    let min_vote = Ord::max(upvotes, min_vote);
    let votes = rng.gen_range(min_vote..=max_vote);

    (upvotes as u64, (votes - upvotes) as u64)
}
