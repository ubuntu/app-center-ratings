use anyhow::anyhow;
use futures::future::join_all;
use rand::{distributions::Alphanumeric, Rng};
use ratings::{
    app::interfaces::authentication::jwt::JwtVerifier,
    features::{
        common::entities::Rating,
        pb::{
            app::{app_client::AppClient, GetRatingRequest},
            chart::{chart_client::ChartClient, ChartData, GetChartRequest, Timeframe},
            user::{
                user_client::UserClient, AuthenticateRequest, GetSnapVotesRequest, Vote,
                VoteRequest,
            },
        },
    },
};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use tonic::{
    metadata::MetadataValue,
    transport::{Channel, Endpoint},
    Request,
};

// re-export to simplify setting up test data in the test files
pub use ratings::features::pb::chart::Category;

// NOTE: these are set by the 'tests' Makefile target
const MOCK_ADMIN_URL: Option<&str> = option_env!("MOCK_ADMIN_URL");
const HOST: Option<&str> = option_env!("HOST");
const PORT: Option<&str> = option_env!("PORT");

macro_rules! client {
    ($client:ident, $channel:expr, $token:expr) => {
        $client::with_interceptor($channel, move |mut req: Request<()>| {
            let header: MetadataValue<_> = format!("Bearer {}", $token).parse().unwrap();
            req.metadata_mut().insert("authorization", header);

            Ok(req)
        })
    };
}

fn rnd_string(len: usize) -> String {
    let rng = rand::thread_rng();
    rng.sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

#[derive(Debug, Default, Clone)]
pub struct TestHelper {
    server_url: String,
    mock_admin_url: &'static str,
    client: Client,
}

impl TestHelper {
    pub fn new() -> Self {
        Self {
            server_url: format!(
                "http://{}:{}/",
                HOST.expect("the integration tests need to be run using make integration-test"),
                PORT.expect("the integration tests need to be run using make integration-test")
            ),
            mock_admin_url: MOCK_ADMIN_URL.unwrap(),
            client: Client::new(),
        }
    }

    pub fn assert_valid_jwt(&self, value: &str) {
        let jwt = JwtVerifier::from_env().expect("unable to init JwtVerifier");
        assert!(jwt.decode(value).is_ok(), "value should be a valid jwt");
    }

    /// NOTE: total needs to be above 25 in order to generate a rating
    pub async fn test_snap_with_initial_votes(
        &self,
        revision: i32,
        upvotes: u64,
        downvotes: u64,
        categories: &[Category],
    ) -> anyhow::Result<String> {
        let snap_id = self.random_id();
        let str_categories: Vec<String> = categories.iter().map(|c| c.to_string()).collect();
        self.client
            .post(format!("{}/{snap_id}", self.mock_admin_url))
            .body(str_categories.join(","))
            .send()
            .await?;

        if upvotes > 0 {
            self.generate_votes(&snap_id, revision, true, upvotes)
                .await?;
        }
        if downvotes > 0 {
            self.generate_votes(&snap_id, revision, false, downvotes)
                .await?;
        }

        Ok(snap_id)
    }

    pub fn random_sha_256(&self) -> String {
        let data = rnd_string(100);
        let mut hasher = Sha256::new();
        hasher.update(data);

        hasher
            .finalize()
            .iter()
            .fold(String::new(), |mut output, b| {
                // This ignores the error without the overhead of unwrap/expect,
                // This is okay because writing to a string can't fail (barring OOM which won't happen)
                let _ = write!(output, "{b:02x}");
                output
            })
    }

    pub fn random_id(&self) -> String {
        rnd_string(32)
    }

    async fn register_and_vote(
        &self,
        snap_id: &str,
        snap_revision: i32,
        vote_up: bool,
    ) -> anyhow::Result<()> {
        let id: String = self.random_sha_256();
        // The first call registers and the second authenticates
        let token = self.authenticate(id.clone()).await?;
        self.authenticate(id).await?;
        self.vote(snap_id, snap_revision, vote_up, &token).await?;

        Ok(())
    }

    pub async fn generate_votes(
        &self,
        snap_id: &str,
        snap_revision: i32,
        vote_up: bool,
        count: u64,
    ) -> anyhow::Result<()> {
        let mut tasks = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let snap_id = snap_id.to_string();
            let client = self.clone();

            tasks.push(tokio::spawn(async move {
                client
                    .register_and_vote(&snap_id, snap_revision, vote_up)
                    .await
            }));
        }

        for res in join_all(tasks).await {
            // Unwrapping twice as the join itself can error as well as the
            // underlying call to register_and_vote.
            // This is here so that tests panic in test generation if there
            // are any issues rather than carrying on with malformed data
            res.unwrap().unwrap();
        }

        Ok(())
    }

    async fn channel(&self) -> Channel {
        Endpoint::from_shared(self.server_url.clone())
            .expect("failed to create Endpoint")
            .connect()
            .await
            .expect("failed to connect")
    }

    pub async fn get_rating(&self, id: &str, token: &str) -> anyhow::Result<Rating> {
        let resp = client!(AppClient, self.channel().await, token)
            .get_rating(GetRatingRequest {
                snap_id: id.to_string(),
            })
            .await?
            .into_inner();

        resp.rating
            .map(Into::into)
            .ok_or(anyhow!("no rating for {id}"))
    }

    pub async fn get_chart(
        &self,
        category: Option<Category>,
        token: &str,
    ) -> anyhow::Result<Vec<ChartData>> {
        let resp = client!(ChartClient, self.channel().await, token)
            .get_chart(GetChartRequest {
                timeframe: Timeframe::Unspecified.into(),
                category: category.map(|v| v.into()),
            })
            .await?
            .into_inner();

        Ok(resp.ordered_chart_data)
    }

    pub async fn vote(
        &self,
        snap_id: &str,
        snap_revision: i32,
        vote_up: bool,
        token: &str,
    ) -> anyhow::Result<()> {
        client!(UserClient, self.channel().await, token)
            .vote(VoteRequest {
                snap_id: snap_id.to_string(),
                snap_revision,
                vote_up,
            })
            .await?;

        Ok(())
    }

    pub async fn get_snap_votes(
        &self,
        token: &str,
        request: GetSnapVotesRequest,
    ) -> anyhow::Result<Vec<Vote>> {
        let resp = client!(UserClient, self.channel().await, token)
            .get_snap_votes(request)
            .await?
            .into_inner();

        Ok(resp.votes)
    }

    pub async fn authenticate(&self, id: String) -> anyhow::Result<String> {
        let resp = UserClient::connect(self.server_url.clone())
            .await?
            .authenticate(AuthenticateRequest { id })
            .await?
            .into_inner();

        Ok(resp.token)
    }
}
