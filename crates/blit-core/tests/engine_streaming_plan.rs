//! ue-r2-1d structural proof: the engine plans from a PARTIAL header
//! stream — first useful work reaches the sink BEFORE enumeration
//! completes (REV4 Design §3 / the ~1s-start acceptance criterion,
//! proven structurally instead of by wall clock).
//!
//! Mechanism: a gated `TransferSource` emits a first wave of headers,
//! then BLOCKS its scan until the sink has observed at least one
//! written payload, then emits the second wave. Under the pre-1d
//! collect-all implementation this deadlocks (planning waited for the
//! scan to finish, the scan waited for first work) and the test fails
//! via timeout; under streaming planning the first wave flushes on the
//! time-based batch flush, bytes land, the gate opens, and the run
//! completes with every file copied.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use blit_core::engine::{EngineRequest, TransferEngine};
use blit_core::fs_enum::FileFilter;
use blit_core::generated::FileHeader;
use blit_core::orchestrator::LocalMirrorOptions;
use blit_core::remote::transfer::payload::{PreparedPayload, TransferPayload};
use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink, SinkOutcome, TransferSink};
use blit_core::remote::transfer::source::{FsTransferSource, TransferSource};
use eyre::Result;
use tokio::sync::{mpsc, Notify};

fn write_file(path: &Path, body: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
}

fn header_for(rel: &str, body: &[u8]) -> FileHeader {
    FileHeader {
        relative_path: rel.to_string(),
        size: body.len() as u64,
        mtime_seconds: 0,
        permissions: 0o644,
        checksum: vec![],
    }
}

/// Emits `first_wave`, then waits on `gate` (fired by the sink when the
/// first payload lands), then emits `second_wave`. Byte reads delegate
/// to a real `FsTransferSource` over the same root.
struct GatedSource {
    inner: FsTransferSource,
    first_wave: Mutex<Vec<FileHeader>>,
    second_wave: Mutex<Vec<FileHeader>>,
    gate: Arc<Notify>,
}

#[async_trait::async_trait]
impl TransferSource for GatedSource {
    fn scan(
        &self,
        _filter: Option<FileFilter>,
        _unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> (
        mpsc::Receiver<FileHeader>,
        tokio::task::JoinHandle<Result<u64>>,
    ) {
        let (tx, rx) = mpsc::channel(64);
        let first: Vec<FileHeader> = std::mem::take(&mut self.first_wave.lock().unwrap());
        let second: Vec<FileHeader> = std::mem::take(&mut self.second_wave.lock().unwrap());
        let gate = self.gate.clone();
        let handle = tokio::spawn(async move {
            let mut emitted = 0u64;
            for h in first {
                if tx.send(h).await.is_err() {
                    eyre::bail!("header channel closed during first wave");
                }
                emitted += 1;
            }
            // The load-bearing wait: enumeration does not finish until
            // the sink saw work. Collect-all planning deadlocks here.
            gate.notified().await;
            for h in second {
                if tx.send(h).await.is_err() {
                    eyre::bail!("header channel closed during second wave");
                }
                emitted += 1;
            }
            Ok(emitted)
        });
        (rx, handle)
    }

    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
        self.inner.prepare_payload(payload).await
    }

    async fn check_availability(
        &self,
        headers: Vec<FileHeader>,
        unreadable_paths: Arc<Mutex<Vec<String>>>,
    ) -> Result<Vec<FileHeader>> {
        self.inner
            .check_availability(headers, unreadable_paths)
            .await
    }

    async fn open_file(
        &self,
        header: &FileHeader,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.inner.open_file(header).await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

/// Wraps `FsTransferSink`; fires `gate` after the first successful
/// payload write.
struct NotifyingSink {
    inner: FsTransferSink,
    gate: Arc<Notify>,
    writes: AtomicUsize,
}

#[async_trait::async_trait]
impl TransferSink for NotifyingSink {
    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
        let outcome = self.inner.write_payload(payload).await?;
        if self.writes.fetch_add(1, Ordering::SeqCst) == 0 {
            self.gate.notify_one();
        }
        Ok(outcome)
    }

    async fn finish(&self) -> Result<()> {
        self.inner.finish().await
    }

    fn root(&self) -> &Path {
        self.inner.root()
    }
}

#[tokio::test]
async fn first_work_lands_before_enumeration_completes() {
    let tmp = tempfile::tempdir().unwrap();
    let src: PathBuf = tmp.path().join("src");
    let dst: PathBuf = tmp.path().join("dst");
    std::fs::create_dir_all(&src).unwrap();

    // Real bytes for both waves exist up front — the gate only delays
    // header EMISSION, mimicking a slow walker.
    let mut first_wave = Vec::new();
    let mut second_wave = Vec::new();
    for idx in 0..3 {
        let rel = format!("first-{idx}.bin");
        let body = format!("wave-one-{idx}").into_bytes();
        write_file(&src.join(&rel), &body);
        first_wave.push(header_for(&rel, &body));
    }
    for idx in 0..3 {
        let rel = format!("second-{idx}.bin");
        let body = format!("wave-two-{idx}").into_bytes();
        write_file(&src.join(&rel), &body);
        second_wave.push(header_for(&rel, &body));
    }

    let gate = Arc::new(Notify::new());
    let source = Arc::new(GatedSource {
        inner: FsTransferSource::new(src.clone()),
        first_wave: Mutex::new(first_wave),
        second_wave: Mutex::new(second_wave),
        gate: gate.clone(),
    });
    let options = LocalMirrorOptions {
        // Everything copies; no destination stats, no fast paths
        // (the engine skips fast-path selection only via its own
        // logic — mirror=false + a dir source would normally probe,
        // but the fast-path prober walks the REAL fs and sees six
        // small files → Tiny. Force the streaming leg the same way
        // the R45 test does: mirror=true disables fast paths; the
        // deletion pass is a no-op because dst only contains what we
        // just wrote.)
        mirror: true,
        skip_unchanged: false,
        perf_history: false,
        progress: false,
        preserve_times: false,
        ..Default::default()
    };
    let sink = Arc::new(NotifyingSink {
        inner: FsTransferSink::new(
            src.clone(),
            dst.clone(),
            FsSinkConfig {
                preserve_times: false,
                dry_run: false,
                checksum: None,
                resume: false,
                compare_mode: blit_core::generated::ComparisonMode::SizeMtime,
            },
        ),
        gate,
        writes: AtomicUsize::new(0),
    });

    let engine = TransferEngine::new();
    let summary = tokio::time::timeout(
        Duration::from_secs(30),
        engine.execute(EngineRequest {
            src_root: src.clone(),
            dest_root: dst.clone(),
            source,
            sink,
            options,
        }),
    )
    .await
    .expect(
        "deadlock: enumeration waited for first work but planning waited for full \
         enumeration — the streaming plan foundation regressed to collect-all",
    )
    .expect("engine run failed");

    assert_eq!(summary.copied_files, 6, "both waves must land: {summary:?}");
    for idx in 0..3 {
        assert_eq!(
            std::fs::read(dst.join(format!("first-{idx}.bin"))).unwrap(),
            format!("wave-one-{idx}").into_bytes()
        );
        assert_eq!(
            std::fs::read(dst.join(format!("second-{idx}.bin"))).unwrap(),
            format!("wave-two-{idx}").into_bytes()
        );
    }
}
