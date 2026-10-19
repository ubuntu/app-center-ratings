// NOTE: this is not at all ideal, but in order to get tests around charts to work we need
//       to be able to control the set of snaps that are in a given category. If tests start
//       failing when you are adding new test cases then double check that you are not
//       making use of any of the Categories that the tests in this file rely on.
pub mod common;

use common::{Category, TestHelper};
use rand::{thread_rng, Rng};
use simple_test_case::test_case;

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

#[test_case(&[(0, 25), (10, 15), (25, 0)], &[2,1,0], Category::DevicesAndIot; "Creation order is reverse rating order")]
#[test_case(&[(27, 0), (25, 0), (26, 0)], &[0,2,1], Category::NewsAndWeather; "More positive votes is weighted higher")]
#[tokio::test]
async fn category_chart_returns_expected_order(
    snap_votes: &[(u64, u64)],
    expected_order: &[usize],
    category: Category,
) -> anyhow::Result<()> {
    let t = TestHelper::new();
    let mut ids = Vec::with_capacity(snap_votes.len());

    for &(upvotes, downvotes) in snap_votes.iter() {
        let id = t
            .test_snap_with_initial_votes(1, upvotes, downvotes, &[category])
            .await?;
        ids.push(id);
    }

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let data = t.get_chart(Some(category), &user_token).await?;

    let chart_indicies: Vec<usize> = data
        .into_iter()
        .map(|c| {
            ids.iter()
                .position(|id| id == &c.rating.as_ref().unwrap().snap_id)
                .unwrap()
        })
        .collect();
    assert_eq!(&chart_indicies, expected_order);

    Ok(())
}

// !! This test expects to be the only one making use of the "Science" category
#[tokio::test]
async fn category_chart_excludes_deb_votes() -> anyhow::Result<()> {
    let t = TestHelper::new();

    let snap_id = t
        .test_snap_with_initial_votes(1, 25, 0, &[Category::Science])
        .await?;
    // A deb sharing the snap's App ID must not be counted towards it
    t.generate_deb_votes(&snap_id, "1.0", false, 5).await?;

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let data = t.get_chart(Some(Category::Science), &user_token).await?;

    let ratings: Vec<_> = data
        .iter()
        .filter_map(|c| c.rating.as_ref())
        .filter(|r| r.snap_id == snap_id)
        .collect();
    assert_eq!(ratings.len(), 1, "{data:?}");
    assert_eq!(ratings[0].total_votes, 25);

    Ok(())
}

#[tokio::test]
async fn chart_excludes_deb_votes() -> anyhow::Result<()> {
    let t = TestHelper::new();

    // Rated higher than any snap in the suite, so it would top the chart if counted.
    // Unknown to the snapcraft.io mock, so a snap name lookup for it would fail.
    let app_id = format!("org.example.{}", t.random_id());
    t.generate_deb_votes(&app_id, "1.0", true, 100).await?;

    let user_token = t.authenticate(t.random_sha_256()).await?;
    let data = t.get_chart(None, &user_token).await?;

    assert!(
        data.iter()
            .all(|c| c.rating.as_ref().unwrap().snap_id != app_id),
        "{data:?}"
    );

    Ok(())
}

fn random_votes(min_vote: usize, max_vote: usize, min_up: usize, max_up: usize) -> (u64, u64) {
    let mut rng = thread_rng();
    let upvotes = rng.gen_range(min_up..max_up);
    let min_vote = Ord::max(upvotes, min_vote);
    let votes = rng.gen_range(min_vote..=max_vote);

    (upvotes as u64, (votes - upvotes) as u64)
}
