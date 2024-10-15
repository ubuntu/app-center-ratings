use anyhow::anyhow;
use futures::future::join_all;
use rand::{distributions::Alphanumeric, Rng};
use ratings::{
    app::interfaces::authentication::jwt::JwtVerifier,
    features::{
        common::entities::Rating,
        pb::{
            app::{app_client::AppClient, GetRatingRequest},
            chart::{
                chart_client::ChartClient, Category, GetChartRequest, GetChartResponse, Timeframe,
            },
            user::{
                user_client::UserClient, AuthenticateRequest, AuthenticateResponse,
                GetSnapVotesRequest, GetSnapVotesResponse, VoteRequest,
            },
        },
    },
};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use tonic::{
    metadata::MetadataValue,
    transport::{Channel, Endpoint},
    Request, Response, Status,
};

// TODO: read these from the environment rather than hard coding
const HOST: &str = "0.0.0.0";
const PORT: u16 = 8080;

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

// NOTE: Deliberately not including the helpers for testing setting the log level or
//       fetching the API info.

#[derive(Debug, Clone)]
pub struct TestHelper {
    url: String,
}

impl TestHelper {
    pub fn new() -> Self {
        Self {
            url: format!("http://{}:{}/", HOST, PORT),
        }
    }

    // JWT assertions

    pub fn assert_valid_jwt(&self, value: &str) {
        let jwt = JwtVerifier::from_env().expect("unable to init JwtVerifier");
        assert!(jwt.decode(value).is_ok(), "value should be a valid jwt");
    }

    pub fn assert_invalid_jwt(&self, value: &str) {
        let jwt = JwtVerifier::from_env().expect("unable to init JwtVerifier");
        assert!(jwt.decode(value).is_err(), "expected invalid jwt");
    }

    // Data generation

    /// NOTE: total needs to be above 25 in order to generate a rating
    pub async fn test_snap_with_initial_votes(&self, revision: i32, upvotes: u64, downvotes: u64) -> anyhow::Result<String> {
        let snap_id = self.random_id();
        if upvotes > 0 {
            self.generate_votes(&snap_id, revision, true, upvotes).await?;
        }
        if downvotes > 0 {
            self.generate_votes(&snap_id, revision, false, downvotes).await?;
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
            // unwrapping twice as the join itself can error as well as the
            // underlying call to register_and_vote
            _ = res;
            //res.unwrap().unwrap();
        }

        Ok(())
    }

    // GRPC client interactions

    async fn channel(&self) -> Channel {
        Endpoint::from_shared(self.url.clone())
            .expect("failed to create Endpoint")
            .connect()
            .await
            .expect("failed to connect")
    }

    pub async fn get_rating(&self, id: &str, token: &str) -> anyhow::Result<Rating> {
        let rating = client!(AppClient, self.channel().await, token)
            .get_rating(GetRatingRequest {
                snap_id: id.to_string(),
            })
            .await?
            .into_inner()
            .rating
            .ok_or(anyhow!("no rating for {id}"))?
            .into();

        Ok(rating)
    }

    pub async fn get_chart(
        &self,
        timeframe: Timeframe,
        token: &str,
    ) -> Result<Response<GetChartResponse>, Status> {
        client!(ChartClient, self.channel().await, token)
            .get_chart(GetChartRequest {
                timeframe: timeframe.into(),
                category: None,
            })
            .await
    }

    pub async fn get_chart_of_category(
        &self,
        timeframe: Timeframe,
        category: Option<Category>,
        token: &str,
    ) -> Result<Response<GetChartResponse>, Status> {
        client!(ChartClient, self.channel().await, token)
            .get_chart(GetChartRequest {
                timeframe: timeframe.into(),
                category: category.map(|v| v.into()),
            })
            .await
    }

    pub async fn vote(
        &self,
        snap_id: &str,
        snap_revision: i32,
        vote_up: bool,
        token: &str,
    ) -> Result<Response<()>, Status> {
        client!(UserClient, self.channel().await, token)
            .vote(VoteRequest {
                snap_id: snap_id.to_string(),
                snap_revision,
                vote_up,
            })
            .await
    }

    pub async fn get_snap_votes(
        &self,
        token: &str,
        request: GetSnapVotesRequest,
    ) -> Result<Response<GetSnapVotesResponse>, Status> {
        client!(UserClient, self.channel().await, token)
            .get_snap_votes(request)
            .await
    }

    pub async fn delete(&self, token: &str) -> Result<Response<()>, Status> {
        client!(UserClient, self.channel().await, token)
            .delete(())
            .await
    }

    pub async fn authenticate(&self, id: String) -> anyhow::Result<String> {
        let resp: AuthenticateResponse = UserClient::connect(self.url.clone())
            .await
            .unwrap()
            .authenticate(AuthenticateRequest { id })
            .await
            .unwrap()
            .into_inner();

        Ok(resp.token)
    }
}
