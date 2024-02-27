use std::str::FromStr;

use cucumber::{given, then, when, Parameter, World};
use helpers::client::*;
use ratings::{
    features::{
        common::entities::{Rating, RatingsBand, VoteSummary},
        pb::user::VoteRequest,
    },
    utils::Config,
};
mod helpers;

#[derive(Debug, Default)]
struct AuthenticatedUser {
    token: String,
}

#[derive(Debug, Default, Copy, Clone, Parameter, strum::EnumString)]
#[param(name = "vote-type", regex = "upvote|downvote")]
#[strum(ascii_case_insensitive)]
enum VoteType {
    #[default]
    Upvote,
    Downvote,
}
impl From<VoteType> for bool {
    fn from(value: VoteType) -> Self {
        match value {
            VoteType::Upvote => true,
            VoteType::Downvote => false,
        }
    }
}

impl From<VoteType> for u64 {
    fn from(value: VoteType) -> Self {
        bool::from(value) as u64
    }
}

#[derive(Debug, Default, Copy, Clone, Parameter)]
#[param(
    name = "direction",
    regex = "strictly increases|strictly decreases|stays constant|monotonically increases|monotonically decreases"
)]
enum Direction {
    #[default]
    StrictlyIncrease,
    StrictlyDecrease,
    StaysConstant,
    MonotonicallyIncrease,
    MonotonicallyDecrease,
}

impl FromStr for Direction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "strictly increases" => Self::StrictlyIncrease,
            "strictly decreases" => Self::StrictlyDecrease,
            "stays constant" => Self::StaysConstant,
            "monotonically increases" => Self::MonotonicallyIncrease,
            "monotonically decreases" => Self::MonotonicallyDecrease,
            _ => return Err(format!("invalid vote count direction {s}")),
        })
    }
}

impl Direction {
    fn check_and_apply(&self, current: &mut u64, new: u64) {
        match self {
            Direction::StrictlyDecrease => assert_eq!(new, *current - 1),

            Direction::StrictlyIncrease => assert_eq!(new, *current + 1),

            Direction::StaysConstant => assert_eq!(new, *current),
            Direction::MonotonicallyIncrease => assert!(
                new == *current || new == *current + 1,
                "value is not montonically increasing, got {new}; current was {current}"
            ),
            Direction::MonotonicallyDecrease => assert!(
                new == *current || new == *current - 1,
                "value is not montonically decreasing, got {new}; current was {current}"
            ),
        };

        *current = new
    }

    fn check_and_apply_band(&self, current: &mut RatingsBand, new: RatingsBand) {
        let comparison = (*current).partial_cmp(&new);

        if comparison.is_none() {
            *current = new;
            // Unable to conclude anything if there isn't enough information
            return;
        }

        let comparison = comparison.unwrap();

        match (*self, comparison) {
            (Direction::StrictlyIncrease, std::cmp::Ordering::Less) => {}
            (Direction::StrictlyDecrease, std::cmp::Ordering::Greater) => {}
            (Direction::StaysConstant, std::cmp::Ordering::Equal) => {}
            (Direction::MonotonicallyIncrease, std::cmp::Ordering::Equal)
            | (Direction::MonotonicallyIncrease, std::cmp::Ordering::Greater) => {}
            (Direction::MonotonicallyDecrease, std::cmp::Ordering::Less)
            | (Direction::MonotonicallyDecrease, std::cmp::Ordering::Equal) => {}
            _ => {
                panic!("Ratings band did not properly {self:?}; current: {current:?}; new: {new:?}")
            }
        }

        *current = new
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Snap(String);

impl Default for Snap {
    fn default() -> Self {
        Snap("93jv9vhsfbb8f7".to_string())
    }
}

#[derive(Debug, World)]
#[world(init = Self::new)]
struct VotingWorld {
    user: AuthenticatedUser,
    client: TestClient,
    snap: Snap,
    rating: Rating,
}

impl VotingWorld {
    async fn new() -> Self {
        let config = Config::load().expect("Could not load config");
        let client = TestClient::new(config.socket());

        let id = helpers::data_faker::rnd_sha_256();
        tracing::debug!("User ID for this test: {id}");
        let user = AuthenticatedUser {
            token: client
                .authenticate(&id)
                .await
                .expect("could not authenticate user")
                .into_inner()
                .token,
        };

        VotingWorld {
            user,
            client,
            snap: Default::default(),
            rating: Default::default(),
        }
    }
}

#[given(expr = "a Snap named {string} has already accumulated {int} votes and {int} upvotes")]
async fn seed_snap(world: &mut VotingWorld, _snap_name: String, votes: i64, upvotes: i64) {
    world.snap.0 = helpers::data_faker::rnd_id();
    tracing::debug!("Snap ID for this test: {}", world.snap.0);

    helpers::vote_generator::generate_votes(&world.snap.0, 1, true, upvotes as u64, &world.client)
        .await
        .expect("could not generate votes");
    helpers::vote_generator::generate_votes(
        &world.snap.0,
        1,
        false,
        (votes - upvotes) as u64,
        &world.client,
    )
    .await
    .expect("could not generate votes");

    let summary = VoteSummary {
        snap_id: world.snap.0.clone(),
        total_votes: votes,
        positive_votes: upvotes,
    };

    world.rating = Rating::new(summary);
}

#[when(expr = "{word} casts a(n) {vote-type}")]
#[when(expr = "{word} changes his/her/their vote to {vote-type}")]
async fn vote(world: &mut VotingWorld, _user_name: String, vote_type: VoteType) {
    let request = VoteRequest {
        snap_id: world.snap.0.clone(),
        snap_revision: 1,
        vote_up: vote_type.into(),
    };

    world
        .client
        .vote(&world.user.token, request)
        .await
        .expect("could not cast vote");
}

#[given(expr = "{word} originally voted {vote-type}")]
async fn originally_voted(world: &mut VotingWorld, _user_name: String, vote_type: VoteType) {
    vote(world, _user_name, vote_type).await;

    world.rating = world
        .client
        .get_rating(&world.user.token, &world.snap.0)
        .await
        .expect("could not get snap rating")
        .into_inner()
        .rating
        .expect("expected an actual rating")
        .into();
}

#[then(expr = "the total number of votes {direction}")]
async fn check_vote(world: &mut VotingWorld, direction: Direction) {
    let votes = world
        .client
        .get_rating(&world.user.token, &world.snap.0)
        .await
        .expect("could not get snap rating")
        .into_inner()
        .rating
        .expect("Rating response was empty")
        .total_votes;

    direction.check_and_apply(&mut world.rating.total_votes, votes);
}

#[then(expr = "the ratings band {direction}")]
async fn check_upvote(world: &mut VotingWorld, direction: Direction) {
    let band = world
        .client
        .get_rating(&world.user.token, &world.snap.0)
        .await
        .expect("could not get snap rating")
        .into_inner()
        .rating
        .expect("Rating response was empty")
        .ratings_band;

    let band =
        ratings::features::pb::common::RatingsBand::try_from(band).expect("Unknown ratings band");

    direction.check_and_apply_band(&mut world.rating.ratings_band, band.into());
}

#[tokio::main]
async fn main() {
    dotenvy::from_filename(".env_files/test.env").ok();

    VotingWorld::cucumber()
        .repeat_skipped()
        .init_tracing()
        .run_and_exit("tests/features/user/voting.feature")
        .await
}
