//! sf-2 loopback e2e pin (`docs/plan/SMALL_FILE_CEILING.md`, slice
//! sf-2): a many-tiny-file push must open more than one data-plane
//! connection.
//!
//! The daemon proposes the epoch-0 stream count at its early manifest
//! flush (`FILE_LIST_EARLY_FLUSH_ENTRIES` = 128 entries), so a 10k-file
//! push used to negotiate from a ~128-file prefix — 1 stream — and ride
//! it for the whole transfer (measured on the 10 GbE rig and again by
//! the sf-1 loopback probe; see DIAGNOSIS.md in
//! `docs/bench/10gbe-2026-07-05/`). The client-side shape-correction
//! resize (`maybe_shape_resize` in blit-core's push client) re-runs the
//! shape table over the accumulated need list and corrects upward
//! through the ue-r2-2 resize wire. This test runs the REAL daemon push
//! service in-process and the REAL client against it, then pins the
//! settled stream count above 1.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use blit_core::fs_enum::FileFilter;
use blit_core::generated::blit_server::BlitServer;
use blit_core::generated::MirrorMode;
use blit_core::remote::transfer::source::FsTransferSource;
use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePushClient};

use crate::runtime::ModuleConfig;
use crate::service::BlitService;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn many_tiny_file_push_opens_more_than_one_data_plane_connection() {
    let dest = tempfile::tempdir().expect("dest dir");
    let canonical = dest.path().canonicalize().expect("canonical dest");
    let mut modules = HashMap::new();
    modules.insert(
        "test".to_string(),
        ModuleConfig {
            name: "test".into(),
            path: canonical.clone(),
            canonical_root: canonical.clone(),
            read_only: false,
            _comment: None,
            delegation_allowed: true,
        },
    );
    let service = BlitService::with_modules(modules, false);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind loopback listener");
    let port = listener.local_addr().expect("listener addr").port();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        blit_core::remote::grpc_server::production_server_builder()
            .add_service(BlitServer::new(service))
            .serve_with_incoming_shutdown(
                tokio_stream::wrappers::TcpListenerStream::new(listener),
                async {
                    let _ = shutdown_rx.await;
                },
            )
            .await
            .expect("in-process daemon serves");
    });

    // The plan's small-file cell: 10k tiny files. The shape table
    // assigns 8 streams (file-count tier); the early-flush proposal
    // sees only the first manifest chunk and starts at 1.
    const FILE_COUNT: usize = 10_000;
    let src = tempfile::tempdir().expect("src dir");
    for i in 0..FILE_COUNT {
        std::fs::write(src.path().join(format!("f{i:05}.bin")), b"x").expect("seed source file");
    }

    let endpoint = RemoteEndpoint {
        host: "127.0.0.1".into(),
        port,
        path: RemotePath::Module {
            module: "test".into(),
            rel_path: PathBuf::new(),
        },
    };
    let mut client = RemotePushClient::connect(endpoint)
        .await
        .expect("client connects");
    let source = Arc::new(FsTransferSource::new(src.path().to_path_buf()));
    let report = client
        .push(
            source,
            &FileFilter::default(),
            false,
            MirrorMode::FilteredSubset,
            false,
            false,
            None,
            false,
        )
        .await
        .expect("push succeeds");

    assert!(!report.fallback_used, "must ride the TCP data plane");
    assert_eq!(
        report.summary.files_transferred as usize, FILE_COUNT,
        "every file arrives"
    );
    let streams = report
        .data_plane_streams
        .expect("data plane ran, stream count recorded");
    assert!(
        streams > 1,
        "a {FILE_COUNT}-file push must correct the partial-manifest \
         1-stream proposal upward via shape resize; settled at {streams}"
    );

    let _ = shutdown_tx.send(());
    server.await.expect("server task joins");
}
