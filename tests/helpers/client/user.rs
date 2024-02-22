use tonic::metadata::MetadataValue;
use tonic::transport::Endpoint;
use tonic::{async_trait, Request, Response, Status};

use ratings::features::pb::user::user_client as pb;
use ratings::features::pb::user::{
    AuthenticateRequest, AuthenticateResponse, GetSnapVotesRequest, GetSnapVotesResponse,
    VoteRequest,
};

use super::Client;

#[async_trait]
pub trait UserClient: Client {
    async fn vote(&self, token: &str, ballet: VoteRequest) -> Result<Response<()>, Status> {
        let channel = Endpoint::from_shared(self.url().to_string())
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

    async fn get_snap_votes(
        &self,
        token: &str,
        request: GetSnapVotesRequest,
    ) -> Result<Response<GetSnapVotesResponse>, Status> {
        let channel = Endpoint::from_shared(self.url().to_string())
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

    async fn delete(&self, token: &str) -> Result<Response<()>, Status> {
        let channel = Endpoint::from_shared(self.url().to_string())
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

    async fn authenticate(&self, id: &str) -> Result<Response<AuthenticateResponse>, Status> {
        let mut client = pb::UserClient::connect(self.url().to_string())
            .await
            .unwrap();
        client
            .authenticate(AuthenticateRequest { id: id.to_string() })
            .await
    }
}
