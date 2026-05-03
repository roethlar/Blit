//! R30-F4 — wire-roundtrip test for `RemotePullClient::pull_sync_with_spec`.
//!
//! The Phase 1.6 endpoint-isolation unit test only compared two
//! constructed `TransferOperationSpec` values. It did **not** prove
//! that a hand-built spec, when handed to `pull_sync_with_spec`,
//! actually reaches the wire byte-for-byte unchanged. This integration
//! test fixes that: a real tonic gRPC server captures the first
//! `ClientPullMessage::Spec` the client sends and asserts it matches
//! the spec we supplied, including a deliberately divergent
//! `module` / `source_path` (so a future regression that "fixed up"
//! those fields from `self.endpoint.path` would fail loudly).
//!
//! The stub server implements the full `Blit` trait. Methods other
//! than `pull_sync` panic if hit — the test only exercises one RPC.

use std::sync::Arc;
use std::time::Duration;

use blit_core::generated::blit_server::{Blit, BlitServer};
use blit_core::generated::{
    client_pull_message, server_pull_message, ClientPullMessage, ClientPushRequest,
    CompletionRequest, CompletionResponse, DelegatedPullProgress, DelegatedPullRequest,
    DiskUsageEntry, DiskUsageRequest, FileHeader, FilesystemStatsRequest, FilesystemStatsResponse,
    FindEntry, FindRequest, ListModulesRequest, ListModulesResponse, ListRequest, ListResponse,
    PeerCapabilities, PullChunk, PullRequest, PullSyncAck, PurgeRequest, PurgeResponse,
    ServerPullMessage, ServerPushResponse, TransferOperationSpec,
};
use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use blit_core::remote::pull::RemotePullClient;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status, Streaming};

/// Stub `Blit` impl that captures the first incoming
/// `ClientPullMessage::Spec` and immediately ends the response stream
/// after sending a benign `PullSyncAck`. That makes
/// `pull_sync_with_spec` return without doing any data-plane setup or
/// transfer work — the only thing we care about is the spec byte
/// shape arriving on the server side.
struct SpyServer {
    captured: Arc<Mutex<Option<TransferOperationSpec>>>,
}

#[tonic::async_trait]
impl Blit for SpyServer {
    type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
    type PullStream = ReceiverStream<Result<PullChunk, Status>>;
    type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
    type FindStream = ReceiverStream<Result<FindEntry, Status>>;
    type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
    type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;

    async fn push(
        &self,
        _: Request<Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        unimplemented!("test only exercises pull_sync")
    }

    async fn pull(
        &self,
        _: Request<PullRequest>,
    ) -> Result<Response<Self::PullStream>, Status> {
        unimplemented!("test only exercises pull_sync")
    }

    async fn pull_sync(
        &self,
        request: Request<Streaming<ClientPullMessage>>,
    ) -> Result<Response<Self::PullSyncStream>, Status> {
        let captured = Arc::clone(&self.captured);
        let mut stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(8);

        tokio::spawn(async move {
            // The client sends Spec as the very first message in
            // pull_sync_with_spec. Capture it, then close the stream.
            while let Ok(Some(msg)) = stream.message().await {
                if let Some(client_pull_message::Payload::Spec(spec)) = msg.payload {
                    *captured.lock().await = Some(spec);
                    // Send a PullSyncAck so the client can return
                    // cleanly without hitting --checksum mismatch
                    // logic. Immediately drop tx so the stream ends.
                    let _ = tx
                        .send(Ok(ServerPullMessage {
                            payload: Some(server_pull_message::Payload::PullSyncAck(
                                PullSyncAck {
                                    server_checksums_enabled: true,
                                },
                            )),
                        }))
                        .await;
                    break;
                }
            }
            // dropping tx here closes the response stream
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list(
        &self,
        _: Request<ListRequest>,
    ) -> Result<Response<ListResponse>, Status> {
        unimplemented!()
    }

    async fn purge(
        &self,
        _: Request<PurgeRequest>,
    ) -> Result<Response<PurgeResponse>, Status> {
        unimplemented!()
    }

    async fn complete_path(
        &self,
        _: Request<CompletionRequest>,
    ) -> Result<Response<CompletionResponse>, Status> {
        unimplemented!()
    }

    async fn list_modules(
        &self,
        _: Request<ListModulesRequest>,
    ) -> Result<Response<ListModulesResponse>, Status> {
        unimplemented!()
    }

    async fn find(
        &self,
        _: Request<FindRequest>,
    ) -> Result<Response<Self::FindStream>, Status> {
        unimplemented!()
    }

    async fn disk_usage(
        &self,
        _: Request<DiskUsageRequest>,
    ) -> Result<Response<Self::DiskUsageStream>, Status> {
        unimplemented!()
    }

    async fn filesystem_stats(
        &self,
        _: Request<FilesystemStatsRequest>,
    ) -> Result<Response<FilesystemStatsResponse>, Status> {
        unimplemented!()
    }

    async fn delegated_pull(
        &self,
        _: Request<DelegatedPullRequest>,
    ) -> Result<Response<Self::DelegatedPullStream>, Status> {
        unimplemented!()
    }
}

async fn spawn_spy(captured: Arc<Mutex<Option<TransferOperationSpec>>>) -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().expect("local_addr").port();

    tokio::spawn(async move {
        let svc = BlitServer::new(SpyServer { captured });
        Server::builder()
            .add_service(svc)
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .expect("server runs");
    });

    // Tiny grace period so the server is listening before the client
    // attempts to connect. tonic's Server::serve_with_incoming starts
    // the listener synchronously above the await, so this is mostly
    // belt-and-suspenders.
    tokio::time::sleep(Duration::from_millis(50)).await;
    port
}

fn hand_built_spec() -> TransferOperationSpec {
    TransferOperationSpec {
        spec_version: 1,
        // Deliberately different from the values the client's
        // endpoint would imply — that's the load-bearing assertion:
        // the spec wins, the endpoint is transport-only.
        module: "alpha-from-spec".into(),
        source_path: "x/y/from-spec".into(),
        filter: Some(blit_core::generated::FilterSpec {
            include: vec!["*.txt".into()],
            exclude: vec!["tmp/**".into()],
            min_size: Some(1),
            max_size: None,
            min_age_secs: None,
            max_age_secs: None,
            files_from: vec![],
        }),
        compare_mode: blit_core::generated::ComparisonMode::Checksum as i32,
        mirror_mode: blit_core::generated::MirrorMode::FilteredSubset as i32,
        resume: Some(blit_core::generated::ResumeSettings {
            enabled: true,
            block_size: 65536,
        }),
        client_capabilities: Some(PeerCapabilities {
            supports_resume: true,
            supports_tar_shards: true,
            supports_data_plane_tcp: true,
            supports_filter_spec: true,
        }),
        force_grpc: false,
        ignore_existing: false,
    }
}

#[tokio::test]
async fn pull_sync_with_spec_forwards_spec_unchanged_on_wire() {
    let captured: Arc<Mutex<Option<TransferOperationSpec>>> = Arc::new(Mutex::new(None));
    let port = spawn_spy(Arc::clone(&captured)).await;

    // Endpoint deliberately constructed with a DIFFERENT module/path
    // than the spec. This catches any future regression where
    // pull_sync_with_spec re-derives those fields from
    // self.endpoint.path (R25-F1 endpoint-isolation invariant).
    let endpoint = RemoteEndpoint {
        host: "127.0.0.1".to_string(),
        port,
        path: RemotePath::Module {
            module: "beta-from-endpoint".into(),
            rel_path: std::path::PathBuf::from("z"),
        },
    };
    let mut client = RemotePullClient::connect(endpoint)
        .await
        .expect("connect to spy");

    let spec_in = hand_built_spec();
    // Empty manifest, no progress, no track_paths — the spy ends the
    // stream as soon as it sees the spec, so all that matters is the
    // very first message.
    let _result = client
        .pull_sync_with_spec(
            std::path::Path::new("/tmp"),
            Vec::<FileHeader>::new(),
            spec_in.clone(),
            false,
            None,
        )
        .await;

    // Give the spy a moment to write the captured spec into the mutex
    // even after pull_sync_with_spec returned.
    let mut tries = 0;
    let captured_spec = loop {
        if let Some(s) = captured.lock().await.clone() {
            break s;
        }
        if tries > 50 {
            panic!("spy never captured a spec — pull_sync_with_spec didn't send one");
        }
        tries += 1;
        tokio::time::sleep(Duration::from_millis(20)).await;
    };

    // The exact spec we handed in must equal the spec the server
    // received. Including module/source_path — neither was rewritten
    // from the (deliberately divergent) endpoint.
    assert_eq!(
        captured_spec, spec_in,
        "spec on the wire diverged from the spec passed to pull_sync_with_spec"
    );
    assert_eq!(captured_spec.module, "alpha-from-spec");
    assert_eq!(captured_spec.source_path, "x/y/from-spec");
}

#[tokio::test]
async fn pull_sync_wrapper_emits_same_spec_as_build_spec_from_options() {
    // Companion to the test above: the existing CLI entry point
    // (`pull_sync` taking `PullSyncOptions`) must produce the SAME
    // wire bytes as constructing the spec via
    // `build_spec_from_options` and calling `pull_sync_with_spec`
    // directly. That's the R23-F1 wire-equivalence regression guard
    // from the perspective of the actual gRPC stream.
    let captured: Arc<Mutex<Option<TransferOperationSpec>>> = Arc::new(Mutex::new(None));
    let port = spawn_spy(Arc::clone(&captured)).await;

    let endpoint = RemoteEndpoint {
        host: "127.0.0.1".to_string(),
        port,
        path: RemotePath::Module {
            module: "viaopts".into(),
            rel_path: std::path::PathBuf::from("sub/dir"),
        },
    };
    let opts = blit_core::remote::pull::PullSyncOptions {
        force_grpc: true,
        mirror_mode: true,
        delete_all_scope: true,
        filter: Some(blit_core::generated::FilterSpec {
            include: vec!["data/**".into()],
            exclude: vec![],
            min_size: None,
            max_size: None,
            min_age_secs: None,
            max_age_secs: None,
            files_from: vec![],
        }),
        size_only: false,
        ignore_times: false,
        ignore_existing: false,
        force: false,
        checksum: true,
        resume: true,
        block_size: 4096,
    };

    let expected_spec =
        RemotePullClient::build_spec_from_options(&endpoint, &opts).expect("spec builds");

    let mut client = RemotePullClient::connect(endpoint)
        .await
        .expect("connect to spy");
    let _ = client
        .pull_sync(std::path::Path::new("/tmp"), Vec::new(), &opts, false, None)
        .await;

    let mut tries = 0;
    let captured_spec = loop {
        if let Some(s) = captured.lock().await.clone() {
            break s;
        }
        if tries > 50 {
            panic!("spy never captured a spec from pull_sync wrapper");
        }
        tries += 1;
        tokio::time::sleep(Duration::from_millis(20)).await;
    };

    assert_eq!(
        captured_spec, expected_spec,
        "pull_sync wrapper emitted a different spec than build_spec_from_options"
    );
}
