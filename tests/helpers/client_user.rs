use tonic::metadata::MetadataValue;
use tonic::transport::Endpoint;
use tonic::{Request, Response, Status};

use crate::pb::user::user_client as pb;
use crate::pb::user::{
    AuthenticateRequest, AuthenticateResponse, GetSnapVotesRequest, GetSnapVotesResponse,
    VoteRequest,
};

#[derive(Debug, Clone)]
pub struct UserClient {
    url: String,
}

impl UserClient {
    pub fn new(socket: &str) -> Self {
        Self {
            url: format!("http://{socket}/"),
        }
    }

    #[allow(dead_code)]
    pub async fn authenticate(&self, id: &str) -> Result<Response<AuthenticateResponse>, Status> {
        let mut client = pb::UserClient::connect(self.url.clone()).await.unwrap();
        client
            .authenticate(AuthenticateRequest { id: id.to_string() })
            .await
    }

    #[allow(dead_code)]
    pub async fn vote(&self, token: &str, ballet: VoteRequest) -> Result<Response<()>, Status> {
        let channel = Endpoint::from_shared(self.url.clone())
            .unwrap()
            .connect()
            .await
            .unwrap();
        let mut client = pb::UserClient::with_interceptor(channel, move |mut req: Request<()>| {
            let header: MetadataValue<_> = format!("Bearer {token}").parse().unwrap();
            req.metadata_mut().insert("authorization", header);
            Ok(req)
        });
        client.vote(ballet).await
    }

    #[allow(dead_code)]
    pub async fn get_snap_votes(
        &self,
        token: &str,
        request: GetSnapVotesRequest,
    ) -> Result<Response<GetSnapVotesResponse>, Status> {
        let channel = Endpoint::from_shared(self.url.clone())
            .unwrap()
            .connect()
            .await
            .unwrap();
        let mut client = pb::UserClient::with_interceptor(channel, move |mut req: Request<()>| {
            let header: MetadataValue<_> = format!("Bearer {token}").parse().unwrap();
            req.metadata_mut().insert("authorization", header);
            Ok(req)
        });
        client.get_snap_votes(request).await
    }

    #[allow(dead_code)]
    pub async fn delete(&self, token: &str) -> Result<Response<()>, Status> {
        let channel = Endpoint::from_shared(self.url.clone())
            .unwrap()
            .connect()
            .await
            .unwrap();
        let mut client = pb::UserClient::with_interceptor(channel, move |mut req: Request<()>| {
            let header: MetadataValue<_> = format!("Bearer {token}").parse().unwrap();
            req.metadata_mut().insert("authorization", header);
            Ok(req)
        });

        client.delete(()).await
    }
}
