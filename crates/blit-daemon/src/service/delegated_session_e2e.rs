//! otp-9b loopback e2e: a delegated transfer rides the unified
//! `Transfer` session. Two in-process daemons — the CLI-facing dst
//! daemon (delegation-enabled) and the source daemon — with the
//! `DelegatedPull` RPC driven exactly as the CLI drives it. The dst
//! daemon validates through the gate, then initiates the session as
//! DESTINATION against the source daemon; this file pins that the
//! bytes land byte-identically, that mirror deletions run LOCALLY via
//! the session's one delete rule (no source-attested delete list),
//! and that `force_grpc` maps onto the session's in-stream carrier
//! (surfacing as the wire-compat `tcp_fallback_used` summary bit).
//!
//! The CLI-side byte-path isolation pin (no payload bytes through the
//! CLI) lives in `crates/blit-cli/tests/remote_remote.rs` and keeps
//! guarding the outer contract.

use std::collections::HashMap;
use std::path::PathBuf;

use blit_core::generated::blit_client::BlitClient;
use blit_core::generated::{
    delegated_pull_progress::Payload as ProgressPayload, DelegatedPullProgress,
    DelegatedPullRequest, DelegatedPullSummary, MirrorMode, RemoteSourceLocator,
    TransferOperationSpec,
};
use tokio::sync::oneshot;

use super::transfer_session_e2e::{assert_trees_identical, write_tree, FileSpec};
use crate::delegation_gate::DelegationConfig;
use crate::metrics::TransferMetrics;
use crate::runtime::ModuleConfig;
use crate::service::BlitService;

/// One in-process daemon over a single module, with delegation
/// optionally enabled (the dst daemon needs it; the source does not).
struct Daemon {
    port: u16,
    shutdown: Option<oneshot::Sender<()>>,
    server: Option<tokio::task::JoinHandle<()>>,
    _root: tempfile::TempDir,
    root: PathBuf,
}

impl Daemon {
    async fn start(module: &str, delegation_enabled: bool) -> Self {
        let dir = tempfile::tempdir().expect("module dir");
        let canonical = dir.path().canonicalize().expect("canonical root");
        let mut modules = HashMap::new();
        modules.insert(
            module.to_string(),
            ModuleConfig {
                name: module.into(),
                path: canonical.clone(),
                canonical_root: canonical.clone(),
                read_only: false,
                _comment: None,
                delegation_allowed: true,
            },
        );
        let delegation = DelegationConfig {
            allow_delegated_pull: delegation_enabled,
            // Empty allowlist + master switch on = any host; the
            // operator-facing allowlist mechanics are pinned in
            // delegation_gate.rs.
            allowed_source_hosts: Vec::new(),
        };
        let service = BlitService::from_runtime(
            modules,
            None,
            false,
            true,
            TransferMetrics::disabled(),
            delegation,
        );
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind loopback listener");
        let port = listener.local_addr().expect("listener addr").port();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let server = tokio::spawn(async move {
            blit_core::remote::grpc_server::production_server_builder()
                .add_service(blit_core::generated::blit_server::BlitServer::new(service))
                .serve_with_incoming_shutdown(
                    tokio_stream::wrappers::TcpListenerStream::new(listener),
                    async {
                        let _ = shutdown_rx.await;
                    },
                )
                .await
                .expect("in-process daemon serves");
        });
        Daemon {
            port,
            shutdown: Some(shutdown_tx),
            server: Some(server),
            _root: dir,
            root: canonical,
        }
    }

    async fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(server) = self.server.take() {
            server.await.expect("server task joins");
        }
    }
}

fn spec(source_module: &str) -> TransferOperationSpec {
    TransferOperationSpec {
        spec_version: blit_core::remote::transfer::operation_spec::SUPPORTED_SPEC_VERSION,
        module: source_module.into(),
        source_path: String::new(),
        ..Default::default()
    }
}

/// Drive `DelegatedPull` on the dst daemon as the CLI does and collect
/// every progress event until the stream ends.
async fn run_delegated(
    dst_port: u16,
    src_port: u16,
    dst_module: &str,
    spec: TransferOperationSpec,
) -> Vec<DelegatedPullProgress> {
    let mut client = BlitClient::connect(format!("http://127.0.0.1:{dst_port}"))
        .await
        .expect("connect dst daemon");
    let mut stream = client
        .delegated_pull(DelegatedPullRequest {
            dst_module: dst_module.into(),
            dst_destination_path: String::new(),
            src: Some(RemoteSourceLocator {
                host: "127.0.0.1".into(),
                port: src_port as u32,
            }),
            spec: Some(spec),
            trace_data_plane: false,
            detach: false,
        })
        .await
        .expect("delegated_pull opens")
        .into_inner();
    let mut events = Vec::new();
    while let Some(msg) = stream.message().await.expect("progress stream") {
        events.push(msg);
    }
    events
}

fn summary_of(events: &[DelegatedPullProgress]) -> &DelegatedPullSummary {
    events
        .iter()
        .find_map(|e| match &e.payload {
            Some(ProgressPayload::Summary(s)) => Some(s),
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "no Summary event; events: {:?}",
                events.iter().map(|e| &e.payload).collect::<Vec<_>>()
            )
        })
}

fn assert_no_error(events: &[DelegatedPullProgress]) {
    if let Some(ProgressPayload::Error(e)) = events.iter().find_map(|e| match &e.payload {
        err @ Some(ProgressPayload::Error(_)) => err.as_ref(),
        _ => None,
    }) {
        panic!("delegated transfer errored: {e:?}");
    }
}

const SRC_TREE: &[FileSpec] = &[
    ("a.txt", b"alpha", 1_600_000_001),
    ("dir one/b.log", b"beta beta beta", 1_600_000_002),
    ("dir one/deeper/c.dat", b"gamma-content", 1_600_000_003),
];

/// otp-9b: the delegated transfer rides the unified session — bytes
/// land byte-identically on the dst module over the session's TCP data
/// plane, Started precedes Summary, and the summary counts what this
/// destination actually applied.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delegated_transfer_rides_the_session_and_lands_bytes() {
    let src = Daemon::start("srcmod", false).await;
    let dst = Daemon::start("dstmod", true).await;
    write_tree(&src.root, SRC_TREE);

    let events = run_delegated(dst.port, src.port, "dstmod", spec("srcmod")).await;
    assert_no_error(&events);
    assert!(
        matches!(
            events.first().and_then(|e| e.payload.as_ref()),
            Some(ProgressPayload::Started(_))
        ),
        "Started leads the stream"
    );
    let summary = summary_of(&events);
    assert_eq!(summary.files_transferred, SRC_TREE.len() as u64);
    assert_eq!(
        summary.bytes_transferred,
        SRC_TREE.iter().map(|(_, c, _)| c.len() as u64).sum::<u64>()
    );
    assert!(
        !summary.tcp_fallback_used,
        "default delegated carrier is the session's TCP data plane"
    );
    assert_trees_identical(&src.root, &dst.root);

    src.stop().await;
    dst.stop().await;
}

/// otp-9b: mirror deletions run LOCALLY on the dst daemon via the
/// session's one delete rule — there is no source-attested delete list
/// anymore, and a plain copy (mirror off) must not delete anything.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delegated_mirror_purges_extraneous_locally() {
    let src = Daemon::start("srcmod", false).await;
    let dst = Daemon::start("dstmod", true).await;
    write_tree(&src.root, SRC_TREE);
    std::fs::write(dst.root.join("stale.bin"), b"extraneous").unwrap();

    // Plain copy first: the extraneous file must survive.
    let events = run_delegated(dst.port, src.port, "dstmod", spec("srcmod")).await;
    assert_no_error(&events);
    assert!(
        dst.root.join("stale.bin").exists(),
        "a plain delegated copy never deletes"
    );
    assert_eq!(summary_of(&events).entries_deleted, 0);

    // Mirror: the session's destination-local delete pass purges it.
    let events = run_delegated(
        dst.port,
        src.port,
        "dstmod",
        TransferOperationSpec {
            mirror_mode: MirrorMode::All as i32,
            ..spec("srcmod")
        },
    )
    .await;
    assert_no_error(&events);
    assert!(
        !dst.root.join("stale.bin").exists(),
        "delegated mirror purges extraneous dst entries locally"
    );
    assert_eq!(summary_of(&events).entries_deleted, 1);
    assert_trees_identical(&src.root, &dst.root);

    src.stop().await;
    dst.stop().await;
}

/// otp-9b: `force_grpc` on the spec maps onto the session's in-stream
/// carrier, surfacing on the wire-compat `tcp_fallback_used` summary
/// bit (its historical meaning: "the gRPC byte fallback carried the
/// payload").
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delegated_force_grpc_rides_the_in_stream_carrier() {
    let src = Daemon::start("srcmod", false).await;
    let dst = Daemon::start("dstmod", true).await;
    write_tree(&src.root, SRC_TREE);

    let events = run_delegated(
        dst.port,
        src.port,
        "dstmod",
        TransferOperationSpec {
            force_grpc: true,
            ..spec("srcmod")
        },
    )
    .await;
    assert_no_error(&events);
    assert!(
        summary_of(&events).tcp_fallback_used,
        "force_grpc rides the in-stream carrier and reports on the wire-compat bit"
    );
    assert_trees_identical(&src.root, &dst.root);

    src.stop().await;
    dst.stop().await;
}
