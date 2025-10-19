use blit_core::generated::blit_server::{Blit, BlitServer};
use blit_core::generated::{
    ClientPushRequest, CompletionRequest, CompletionResponse, ListModulesRequest,
    ListModulesResponse, ListRequest, ListResponse, PullChunk, PullRequest, PurgeRequest,
    PurgeResponse, ServerPushResponse,
};
use tonic::{transport::Server, Request, Response, Status, Streaming};

use eyre::Result;

#[derive(Default)]
pub struct BlitService;

#[tonic::async_trait]
impl Blit for BlitService {
    type PushStream = tokio_stream::wrappers::ReceiverStream<Result<ServerPushResponse, Status>>;
    type PullStream = tokio_stream::wrappers::ReceiverStream<Result<PullChunk, Status>>;

    async fn push(
        &self,
        _request: Request<Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        Err(Status::unimplemented("Push is not yet implemented"))
    }

    async fn pull(
        &self,
        _request: Request<PullRequest>,
    ) -> Result<Response<Self::PullStream>, Status> {
        Err(Status::unimplemented("Pull is not yet implemented"))
    }

    async fn list(&self, _request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        Err(Status::unimplemented("List is not yet implemented"))
    }

    async fn purge(
        &self,
        _request: Request<PurgeRequest>,
    ) -> Result<Response<PurgeResponse>, Status> {
        Err(Status::unimplemented("Purge is not yet implemented"))
    }

    async fn complete_path(
        &self,
        _request: Request<CompletionRequest>,
    ) -> Result<Response<CompletionResponse>, Status> {
        Err(Status::unimplemented("CompletePath is not yet implemented"))
    }

    async fn list_modules(
        &self,
        _request: Request<ListModulesRequest>,
    ) -> Result<Response<ListModulesResponse>, Status> {
        Err(Status::unimplemented("ListModules is not yet implemented"))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "[::1]:50051".parse()?;
    let service = BlitService::default();

    println!("blitd v2 listening on {}", addr);

    Server::builder()
        .add_service(BlitServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
