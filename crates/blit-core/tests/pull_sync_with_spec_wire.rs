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
    PeerCapabilities, PullSyncAck, PurgeRequest, PurgeResponse, ServerPullMessage,
    ServerPushResponse, TransferOperationSpec,
};
use blit_core::remote::endpoint::{RemoteEndpoint, RemotePath};
use blit_core::remote::pull::{PullSyncError, RemotePullClient};
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
    reject_pull_sync: Option<(tonic::Code, &'static str)>,
}

#[tonic::async_trait]
impl Blit for SpyServer {
    type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
    type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
    type FindStream = ReceiverStream<Result<FindEntry, Status>>;
    type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
    type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;
    type SubscribeStream = std::pin::Pin<
        Box<
            dyn tokio_stream::Stream<Item = Result<blit_core::generated::DaemonEvent, Status>>
                + Send,
        >,
    >;

    async fn subscribe(
        &self,
        _: Request<blit_core::generated::SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        unimplemented!("test only exercises pull_sync")
    }

    async fn push(
        &self,
        _: Request<Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        unimplemented!("test only exercises pull_sync")
    }

    async fn pull_sync(
        &self,
        request: Request<Streaming<ClientPullMessage>>,
    ) -> Result<Response<Self::PullSyncStream>, Status> {
        if let Some((code, message)) = self.reject_pull_sync {
            return Err(Status::new(code, message));
        }
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
                            payload: Some(server_pull_message::Payload::PullSyncAck(PullSyncAck {
                                server_checksums_enabled: true,
                            })),
                        }))
                        .await;
                    break;
                }
            }
            // dropping tx here closes the response stream
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn list(&self, _: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        unimplemented!()
    }

    async fn purge(&self, _: Request<PurgeRequest>) -> Result<Response<PurgeResponse>, Status> {
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

    async fn find(&self, _: Request<FindRequest>) -> Result<Response<Self::FindStream>, Status> {
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

    async fn get_state(
        &self,
        _: Request<blit_core::generated::GetStateRequest>,
    ) -> Result<Response<blit_core::generated::DaemonState>, Status> {
        unimplemented!()
    }

    async fn cancel_job(
        &self,
        _: Request<blit_core::generated::CancelJobRequest>,
    ) -> Result<Response<blit_core::generated::CancelJobResponse>, Status> {
        unimplemented!()
    }

    async fn clear_recent(
        &self,
        _: Request<blit_core::generated::ClearRecentRequest>,
    ) -> Result<Response<blit_core::generated::ClearRecentResponse>, Status> {
        unimplemented!()
    }
}

async fn spawn_spy(captured: Arc<Mutex<Option<TransferOperationSpec>>>) -> u16 {
    spawn_spy_with_rejection(captured, None).await
}

async fn spawn_spy_with_rejection(
    captured: Arc<Mutex<Option<TransferOperationSpec>>>,
    reject_pull_sync: Option<(tonic::Code, &'static str)>,
) -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().expect("local_addr").port();

    tokio::spawn(async move {
        let svc = BlitServer::new(SpyServer {
            captured,
            reject_pull_sync,
        });
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
        spec_version: 2,
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
            supports_stream_resize: true,
        }),
        force_grpc: false,
        ignore_existing: false,
        require_complete_scan: false,
        // ue-r2-1h: set so the byte-identical assertion also
        // regression-guards the metadata_only wire shape.
        metadata_only: true,
        // ue-r2-1b: populated with every field set so the byte-identical
        // assertion also regression-guards the new profile's wire shape.
        receiver_capacity: Some(blit_core::generated::CapacityProfile {
            cpu_cores: 16,
            drain_class: blit_core::generated::DrainClass::SsdNvme as i32,
            load_percent: 35,
            max_streams: 8,
            drain_rate_bytes_per_sec: 2_000_000_000,
            max_chunk_bytes: 8 * 1024 * 1024,
            max_inflight_bytes: 256 * 1024 * 1024,
        }),
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
        require_complete_scan: false,
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

#[tokio::test]
async fn pull_sync_with_spec_classifies_initial_rpc_rejection_as_negotiation() {
    let captured: Arc<Mutex<Option<TransferOperationSpec>>> = Arc::new(Mutex::new(None));
    let port = spawn_spy_with_rejection(
        Arc::clone(&captured),
        Some((
            tonic::Code::PermissionDenied,
            "source ACL rejected delegated peer",
        )),
    )
    .await;

    let endpoint = RemoteEndpoint {
        host: "127.0.0.1".to_string(),
        port,
        path: RemotePath::Discovery,
    };
    let mut client = RemotePullClient::connect(endpoint)
        .await
        .expect("connect to rejecting spy");

    let err = client
        .pull_sync_with_spec(
            std::path::Path::new("/tmp"),
            Vec::<FileHeader>::new(),
            hand_built_spec(),
            false,
            None,
            None,
        )
        .await
        .unwrap_err();

    let pull_err = err
        .downcast_ref::<PullSyncError>()
        .expect("initial pull_sync RPC rejection should preserve PullSyncError");
    assert!(
        pull_err.is_negotiation(),
        "initial RPC rejection must be classified as negotiation: {err}"
    );
    assert!(
        err.to_string()
            .contains("source ACL rejected delegated peer"),
        "source rejection reason should survive, got: {err}"
    );
}

// ─── ue-r2-1h: relay session wire tests ──────────────────────────────
//
// `scan_remote_files` and `open_remote_file` (the remote→remote
// relay's primitives) rode the deprecated Pull RPC until ue-r2-1h
// deleted it; they now open their own PullSync sessions. These tests
// pin the client half of that port against a daemon-shaped frame
// script: the spec each session sends, the frames it consumes, and
// the mixed-version degradation the proto comment promises
// (an old daemon ignoring `metadata_only` streams data — the scan
// must still return exactly the headers).

/// `Blit` impl that captures the pull_sync spec and then plays back a
/// fixed frame script. Unlike `SpyServer` it never inspects the
/// client's manifest phase — the relay sessions send an empty
/// manifest and the script is unconditional.
struct CannedFramesServer {
    captured: Arc<Mutex<Option<TransferOperationSpec>>>,
    frames: Vec<server_pull_message::Payload>,
}

#[tonic::async_trait]
impl Blit for CannedFramesServer {
    type PushStream = ReceiverStream<Result<ServerPushResponse, Status>>;
    type PullSyncStream = ReceiverStream<Result<ServerPullMessage, Status>>;
    type FindStream = ReceiverStream<Result<FindEntry, Status>>;
    type DiskUsageStream = ReceiverStream<Result<DiskUsageEntry, Status>>;
    type DelegatedPullStream = ReceiverStream<Result<DelegatedPullProgress, Status>>;
    type SubscribeStream = std::pin::Pin<
        Box<
            dyn tokio_stream::Stream<Item = Result<blit_core::generated::DaemonEvent, Status>>
                + Send,
        >,
    >;

    async fn pull_sync(
        &self,
        request: Request<Streaming<ClientPullMessage>>,
    ) -> Result<Response<Self::PullSyncStream>, Status> {
        let captured = Arc::clone(&self.captured);
        let frames = self.frames.clone();
        let mut stream = request.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(16);

        tokio::spawn(async move {
            // Capture the spec, then play the script and close.
            while let Ok(Some(msg)) = stream.message().await {
                if let Some(client_pull_message::Payload::Spec(spec)) = msg.payload {
                    *captured.lock().await = Some(spec);
                    break;
                }
            }
            for payload in frames {
                if tx
                    .send(Ok(ServerPullMessage {
                        payload: Some(payload),
                    }))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn push(
        &self,
        _: Request<Streaming<ClientPushRequest>>,
    ) -> Result<Response<Self::PushStream>, Status> {
        unimplemented!("test only exercises pull_sync")
    }
    async fn subscribe(
        &self,
        _: Request<blit_core::generated::SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        unimplemented!()
    }
    async fn list(&self, _: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        unimplemented!()
    }
    async fn purge(&self, _: Request<PurgeRequest>) -> Result<Response<PurgeResponse>, Status> {
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
    async fn find(&self, _: Request<FindRequest>) -> Result<Response<Self::FindStream>, Status> {
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
    async fn get_state(
        &self,
        _: Request<blit_core::generated::GetStateRequest>,
    ) -> Result<Response<blit_core::generated::DaemonState>, Status> {
        unimplemented!()
    }
    async fn cancel_job(
        &self,
        _: Request<blit_core::generated::CancelJobRequest>,
    ) -> Result<Response<blit_core::generated::CancelJobResponse>, Status> {
        unimplemented!()
    }
    async fn clear_recent(
        &self,
        _: Request<blit_core::generated::ClearRecentRequest>,
    ) -> Result<Response<blit_core::generated::ClearRecentResponse>, Status> {
        unimplemented!()
    }
}

async fn spawn_canned(
    captured: Arc<Mutex<Option<TransferOperationSpec>>>,
    frames: Vec<server_pull_message::Payload>,
) -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral port");
    let port = listener.local_addr().expect("local_addr").port();

    tokio::spawn(async move {
        let svc = BlitServer::new(CannedFramesServer { captured, frames });
        Server::builder()
            .add_service(svc)
            .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(listener))
            .await
            .expect("server runs");
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    port
}

fn relay_endpoint(port: u16) -> RemoteEndpoint {
    RemoteEndpoint {
        host: "127.0.0.1".to_string(),
        port,
        path: RemotePath::Module {
            module: "relaymod".into(),
            rel_path: std::path::PathBuf::from("sub"),
        },
    }
}

fn wire_header(rel: &str, size: u64) -> FileHeader {
    FileHeader {
        relative_path: rel.into(),
        size,
        mtime_seconds: 1_700_000_000,
        permissions: 0o644,
        checksum: vec![],
    }
}

fn benign_summary() -> server_pull_message::Payload {
    server_pull_message::Payload::Summary(blit_core::generated::PullSummary {
        files_transferred: 0,
        bytes_transferred: 0,
        bytes_zero_copy: 0,
        tcp_fallback_used: true,
        entries_deleted: 0,
    })
}

#[tokio::test]
async fn scan_remote_files_collects_bare_headers_and_sends_metadata_only_spec() {
    // The new-daemon shape: ack, manifest progress, one bare
    // file_header per entry, summary. No data frames.
    let captured: Arc<Mutex<Option<TransferOperationSpec>>> = Arc::new(Mutex::new(None));
    let frames = vec![
        server_pull_message::Payload::PullSyncAck(PullSyncAck {
            server_checksums_enabled: true,
        }),
        server_pull_message::Payload::ManifestBatch(blit_core::generated::ManifestBatch {
            file_count: 2,
            total_bytes: 15,
        }),
        server_pull_message::Payload::FileHeader(wire_header("a.txt", 5)),
        server_pull_message::Payload::FileHeader(wire_header("nested/b.bin", 10)),
        benign_summary(),
    ];
    let port = spawn_canned(Arc::clone(&captured), frames).await;

    let mut client = RemotePullClient::connect(relay_endpoint(port))
        .await
        .expect("connect");
    let headers = client
        .scan_remote_files(std::path::Path::new("dir"))
        .await
        .expect("scan succeeds");

    assert_eq!(
        headers
            .iter()
            .map(|h| h.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec!["a.txt", "nested/b.bin"]
    );
    assert_eq!(headers[1].size, 10);

    let spec = captured.lock().await.clone().expect("spec captured");
    assert!(
        spec.metadata_only,
        "scan must request a metadata-only session"
    );
    assert!(
        spec.force_grpc,
        "scan must pin the control-stream transport"
    );
    assert_eq!(spec.module, "relaymod");
    // endpoint rel_path "sub" joined with the scan path "dir"
    assert_eq!(spec.source_path, "sub/dir");
    assert!(!spec.require_complete_scan);
    assert_eq!(
        spec.mirror_mode,
        blit_core::generated::MirrorMode::Unspecified as i32
    );
}

#[tokio::test]
async fn scan_remote_files_survives_old_daemon_streaming_data() {
    // Mixed-version pin for the proto contract: an old daemon ignores
    // `metadata_only` and runs the full force_grpc fallback — headers
    // interleaved with file bytes and tar shards. The scan must return
    // exactly the headers (including the shard's) and discard bytes.
    let captured: Arc<Mutex<Option<TransferOperationSpec>>> = Arc::new(Mutex::new(None));
    let frames = vec![
        server_pull_message::Payload::PullSyncAck(PullSyncAck {
            server_checksums_enabled: false,
        }),
        server_pull_message::Payload::FilesToDownload(blit_core::generated::FileList {
            relative_paths: vec!["big.bin".into(), "s1.txt".into(), "s2.txt".into()],
        }),
        server_pull_message::Payload::FileHeader(wire_header("big.bin", 8)),
        server_pull_message::Payload::FileData(blit_core::generated::FileData {
            content: b"12345678".to_vec(),
        }),
        server_pull_message::Payload::TarShardHeader(blit_core::generated::TarShardHeader {
            files: vec![wire_header("s1.txt", 2), wire_header("s2.txt", 3)],
            archive_size: 1024,
        }),
        server_pull_message::Payload::TarShardChunk(blit_core::generated::TarShardChunk {
            content: vec![0u8; 1024],
        }),
        server_pull_message::Payload::TarShardComplete(blit_core::generated::TarShardComplete {}),
        benign_summary(),
    ];
    let port = spawn_canned(Arc::clone(&captured), frames).await;

    let mut client = RemotePullClient::connect(relay_endpoint(port))
        .await
        .expect("connect");
    let headers = client
        .scan_remote_files(std::path::Path::new("."))
        .await
        .expect("scan tolerates an old daemon's data frames");

    assert_eq!(
        headers
            .iter()
            .map(|h| h.relative_path.as_str())
            .collect::<Vec<_>>(),
        vec!["big.bin", "s1.txt", "s2.txt"]
    );
}

#[tokio::test]
async fn scan_remote_files_rejects_real_data_plane_negotiation() {
    // force_grpc is set, so a daemon steering the session onto a TCP
    // data plane the scan will never dial must fail fast (not stall
    // until the daemon's accept timeout).
    let captured: Arc<Mutex<Option<TransferOperationSpec>>> = Arc::new(Mutex::new(None));
    let frames = vec![server_pull_message::Payload::Negotiation(
        blit_core::generated::DataTransferNegotiation {
            tcp_port: 9,
            one_time_token: "tok".into(),
            tcp_fallback: false,
            stream_count: 1,
            receiver_capacity: None,
            resize_enabled: false,
            epoch0_sub_token: Vec::new(),
        },
    )];
    let port = spawn_canned(Arc::clone(&captured), frames).await;

    let mut client = RemotePullClient::connect(relay_endpoint(port))
        .await
        .expect("connect");
    let err = client
        .scan_remote_files(std::path::Path::new("."))
        .await
        .expect_err("negotiation during a metadata-only scan must error");
    assert!(
        err.to_string().contains("data-plane negotiation"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn open_remote_file_yields_bytes_and_eofs_at_summary() {
    use tokio::io::AsyncReadExt;

    let captured: Arc<Mutex<Option<TransferOperationSpec>>> = Arc::new(Mutex::new(None));
    let frames = vec![
        server_pull_message::Payload::PullSyncAck(PullSyncAck {
            server_checksums_enabled: true,
        }),
        server_pull_message::Payload::FileHeader(wire_header("", 11)),
        server_pull_message::Payload::FileData(blit_core::generated::FileData {
            content: b"hello ".to_vec(),
        }),
        server_pull_message::Payload::FileData(blit_core::generated::FileData {
            content: b"world".to_vec(),
        }),
        benign_summary(),
    ];
    let port = spawn_canned(Arc::clone(&captured), frames).await;

    let client = RemotePullClient::connect(relay_endpoint(port))
        .await
        .expect("connect");
    let mut reader = client
        .open_remote_file(std::path::Path::new("file.txt"))
        .await
        .expect("open succeeds");

    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .await
        .expect("read to summary EOF");
    assert_eq!(bytes, b"hello world");

    let spec = captured.lock().await.clone().expect("spec captured");
    assert!(!spec.metadata_only, "single-file streaming wants the bytes");
    assert!(spec.force_grpc);
    assert_eq!(spec.source_path, "sub/file.txt");
    let caps = spec.client_capabilities.expect("caps advertised");
    assert!(
        !caps.supports_tar_shards,
        "the single-file reader cannot parse tar archives and must say so"
    );
}

#[tokio::test]
async fn open_remote_file_rejects_tar_shard_frames() {
    use tokio::io::AsyncReadExt;

    let captured: Arc<Mutex<Option<TransferOperationSpec>>> = Arc::new(Mutex::new(None));
    let frames = vec![
        server_pull_message::Payload::TarShardHeader(blit_core::generated::TarShardHeader {
            files: vec![wire_header("s1.txt", 2)],
            archive_size: 512,
        }),
        server_pull_message::Payload::TarShardChunk(blit_core::generated::TarShardChunk {
            content: vec![0u8; 512],
        }),
    ];
    let port = spawn_canned(Arc::clone(&captured), frames).await;

    let client = RemotePullClient::connect(relay_endpoint(port))
        .await
        .expect("connect");
    let mut reader = client
        .open_remote_file(std::path::Path::new("s1.txt"))
        .await
        .expect("open itself succeeds — the error surfaces on read");

    let mut bytes = Vec::new();
    let err = reader
        .read_to_end(&mut bytes)
        .await
        .expect_err("tar frames must fail the read, not decode as file bytes");
    assert!(
        err.to_string().contains("tar-shard"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn open_remote_file_rejects_second_file_header() {
    use tokio::io::AsyncReadExt;

    // ue-r2-1h review (panel F3): one file per session — a second
    // header would silently splice the next file's bytes into the
    // current read. Must be a protocol error, like the tar arm.
    let captured: Arc<Mutex<Option<TransferOperationSpec>>> = Arc::new(Mutex::new(None));
    let frames = vec![
        server_pull_message::Payload::FileHeader(wire_header("", 2)),
        server_pull_message::Payload::FileData(blit_core::generated::FileData {
            content: b"aa".to_vec(),
        }),
        server_pull_message::Payload::FileHeader(wire_header("other.txt", 2)),
        server_pull_message::Payload::FileData(blit_core::generated::FileData {
            content: b"bb".to_vec(),
        }),
        benign_summary(),
    ];
    let port = spawn_canned(Arc::clone(&captured), frames).await;

    let client = RemotePullClient::connect(relay_endpoint(port))
        .await
        .expect("connect");
    let mut reader = client
        .open_remote_file(std::path::Path::new("s1.txt"))
        .await
        .expect("open itself succeeds");

    let mut bytes = Vec::new();
    let err = reader
        .read_to_end(&mut bytes)
        .await
        .expect_err("a second file_header must fail the read");
    assert!(
        err.to_string().contains("second file_header"),
        "unexpected error: {err}"
    );
}
