//! Streaming plan foundation (`ue-r2-1d`, REV4 Design §3).
//!
//! Plans transfer payloads from a PARTIAL header stream so first useful
//! work starts without waiting for full enumeration: headers are
//! batched and each batch runs through the unchanged `plan_local_mirror`
//! diff/plan stage, with resulting payloads fed straight into the
//! streaming pipeline. The diff is per-header (destination stat), so
//! batching does not change skip_unchanged / ignore_existing /
//! compare-mode semantics; tar-shard grouping simply never spans a
//! batch.
//!
//! A batch flushes when ANY of these fires:
//! - it reaches [`STREAMING_PLAN_BATCH_HEADERS`] headers;
//! - [`STREAMING_PLAN_FLUSH_AFTER`] elapsed since the batch's first
//!   header (the REV4 "pathological slow enumeration" mitigation — a
//!   slow walker cannot stall first work past the flush window, which
//!   is what keeps the ~1s-start budget for novel workloads);
//! - the header channel closes (scan finished or failed).
//!
//! RELIABLE exceptions (REV4 acceptance criteria): the mirror DELETION
//! pass still requires complete knowledge — the engine runs it only
//! after the scan completed cleanly; only the copy phase streams.
//! Block-resume and checksum-refusal are remote-path handshakes and do
//! not run through this local leg (they converge at `ue-r2-1f`/`1g`).

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use eyre::{Context, Result};
use tokio::sync::mpsc;

use crate::generated::{ComparisonMode, FileHeader};
use crate::remote::transfer::diff_planner::{plan_local_mirror, LocalDiffInputs};
use crate::remote::transfer::payload::TransferPayload;
use crate::transfer_plan::PlanOptions;

/// Maximum headers per planned batch. Large enough that tar-shard
/// grouping inside a batch stays effective; small enough that a batch
/// plans quickly and the pipeline is fed continuously.
pub const STREAMING_PLAN_BATCH_HEADERS: usize = 512;

/// Flush a non-empty batch after this long even if it isn't full, so a
/// slow enumeration cannot delay first work indefinitely.
pub const STREAMING_PLAN_FLUSH_AFTER: Duration = Duration::from_millis(250);

/// How the initial plan was chosen (REV4 Design §3): `Known` when
/// cross-run telemetry existed for this workload shape (the
/// perf-history tuning window produced records, from which plan targets
/// were derived), `Novel` otherwise (conservative defaults, tune live).
/// Both start immediately; neither is a probe phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialPlanStrategy {
    Novel,
    Known { window_records: usize },
}

/// The plan the engine starts a transfer with, produced BEFORE
/// enumeration completes. Refinement arrives as [`PlanUpdate`]s.
#[derive(Clone, Debug)]
pub struct InitialPlan {
    pub strategy: InitialPlanStrategy,
    pub plan_options: PlanOptions,
}

/// One planned batch: the payloads produced from a partial slice of the
/// header stream. Consumed by the engine, which forwards the payloads
/// to the streaming pipeline.
#[derive(Debug)]
pub struct PlanUpdate {
    pub payloads: Vec<TransferPayload>,
    pub headers_planned: usize,
    pub bytes_planned: u64,
}

/// Inputs the per-batch diff/plan stage needs. Owned clones so each
/// batch can run on `spawn_blocking`.
pub(super) struct StreamingPlanInputs {
    pub src_root: PathBuf,
    pub dest_root: PathBuf,
    pub compare_mode: ComparisonMode,
    pub ignore_existing: bool,
    pub skip_unchanged: bool,
    pub initial: InitialPlan,
    /// Accumulate every scanned relative path (mirror needs the full
    /// source set for its post-scan deletion pass).
    pub collect_source_paths: bool,
    /// When the destination sits INSIDE the source tree, its
    /// source-relative prefix. Headers under it are dropped before
    /// planning: with writes concurrent to the walk, freshly written
    /// destination files can be re-enumerated and would otherwise be
    /// copied again recursively (codex ue-r2-1d F1). `None` for the
    /// normal disjoint-roots case.
    pub exclude_dest_subtree: Option<PathBuf>,
}

/// What the planner learned by the end of the stream. Scan totals are
/// the same numbers the collect-all implementation produced; the
/// engine's predictor query and history record consume them unchanged
/// (R44-F1 feature alignment).
#[derive(Debug, Default)]
pub(super) struct StreamingPlanOutcome {
    pub scanned_files: usize,
    pub scanned_bytes: u64,
    pub source_paths: HashSet<String>,
    /// Elapsed from planning start until the first payload was handed
    /// to the pipeline. `None` when the plan produced no payloads
    /// (nothing to copy). This is the streaming redefinition of
    /// "planner duration": the serial latency before first work.
    pub first_payload_elapsed: Option<Duration>,
    pub plan_updates: usize,
}

/// Drain `header_rx`, plan in batches, and feed `payload_tx`.
///
/// Returns `Ok` with partial totals if the pipeline hangs up
/// (`payload_tx` send failure) — the pipeline's own error is
/// authoritative in that case and the engine reports it instead.
/// Returns `Err` only for a genuine diff/plan failure.
pub(super) async fn run_streaming_plan(
    mut header_rx: mpsc::Receiver<FileHeader>,
    inputs: StreamingPlanInputs,
    payload_tx: mpsc::Sender<TransferPayload>,
    planning_start: Instant,
) -> Result<StreamingPlanOutcome> {
    let mut outcome = StreamingPlanOutcome::default();
    let mut batch: Vec<FileHeader> = Vec::with_capacity(STREAMING_PLAN_BATCH_HEADERS);
    let mut deadline = tokio::time::Instant::now();
    let mut closed = false;

    while !closed || !batch.is_empty() {
        // Fill until the batch is full, the flush deadline passes, or
        // the channel closes.
        while !closed && batch.len() < STREAMING_PLAN_BATCH_HEADERS {
            if batch.is_empty() {
                match header_rx.recv().await {
                    Some(header) => {
                        deadline = tokio::time::Instant::now() + STREAMING_PLAN_FLUSH_AFTER;
                        accumulate(&mut outcome, &inputs, &mut batch, header);
                    }
                    None => closed = true,
                }
            } else {
                tokio::select! {
                    maybe_header = header_rx.recv() => match maybe_header {
                        Some(header) => accumulate(&mut outcome, &inputs, &mut batch, header),
                        None => closed = true,
                    },
                    _ = tokio::time::sleep_until(deadline) => break,
                }
            }
        }

        if batch.is_empty() {
            continue; // only possible when `closed` just flipped — loop exits.
        }

        let update = plan_batch(std::mem::take(&mut batch), &inputs).await?;
        outcome.plan_updates += 1;
        for payload in update.payloads {
            if payload_tx.send(payload).await.is_err() {
                // Pipeline hung up (its error is authoritative). Stop
                // planning; dropping header_rx aborts the walker.
                return Ok(outcome);
            }
            if outcome.first_payload_elapsed.is_none() {
                outcome.first_payload_elapsed = Some(planning_start.elapsed());
            }
        }
    }

    Ok(outcome)
}

fn accumulate(
    outcome: &mut StreamingPlanOutcome,
    inputs: &StreamingPlanInputs,
    batch: &mut Vec<FileHeader>,
    header: FileHeader,
) {
    if let Some(excluded) = &inputs.exclude_dest_subtree {
        if std::path::Path::new(&header.relative_path).starts_with(excluded) {
            log::debug!(
                "streaming plan: skipping {} (inside the nested destination subtree)",
                header.relative_path
            );
            return;
        }
    }
    outcome.scanned_files += 1;
    outcome.scanned_bytes += header.size;
    if inputs.collect_source_paths {
        outcome.source_paths.insert(header.relative_path.clone());
    }
    batch.push(header);
}

/// Diff + plan one batch via the unchanged shared stage, off the async
/// runtime (destination stats and payload planning are blocking work —
/// same `spawn_blocking` treatment the collect-all implementation gave
/// the single big plan).
async fn plan_batch(batch: Vec<FileHeader>, inputs: &StreamingPlanInputs) -> Result<PlanUpdate> {
    let headers_planned = batch.len();
    let bytes_planned: u64 = batch.iter().map(|h| h.size).sum();
    let src = inputs.src_root.clone();
    let dst = inputs.dest_root.clone();
    let plan_options = inputs.initial.plan_options;
    let compare_mode = inputs.compare_mode;
    let ignore_existing = inputs.ignore_existing;
    let skip_unchanged = inputs.skip_unchanged;
    let planned = tokio::task::spawn_blocking(move || {
        plan_local_mirror(
            batch,
            LocalDiffInputs {
                src_root: &src,
                dst_root: &dst,
                compare_mode,
                ignore_existing,
                plan_options,
                skip_unchanged,
            },
        )
    })
    .await
    .context("diff_planner task panicked")??;
    Ok(PlanUpdate {
        payloads: planned,
        headers_planned,
        bytes_planned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    fn write_file(path: &std::path::Path, body: &[u8]) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    fn header_for(root: &std::path::Path, rel: &str, body: &[u8]) -> FileHeader {
        write_file(&root.join(rel), body);
        FileHeader {
            relative_path: rel.to_string(),
            size: body.len() as u64,
            mtime_seconds: 0,
            permissions: 0o644,
            checksum: vec![],
        }
    }

    fn inputs(src: &std::path::Path, dst: &std::path::Path) -> StreamingPlanInputs {
        StreamingPlanInputs {
            src_root: src.to_path_buf(),
            dest_root: dst.to_path_buf(),
            compare_mode: ComparisonMode::SizeMtime,
            ignore_existing: false,
            // Plan everything — no destination stats in these tests.
            skip_unchanged: false,
            initial: InitialPlan {
                strategy: InitialPlanStrategy::Novel,
                plan_options: PlanOptions::default(),
            },
            collect_source_paths: true,
            exclude_dest_subtree: None,
        }
    }

    /// The time-based flush is the 1s-start mitigation: a non-full
    /// batch must flush after STREAMING_PLAN_FLUSH_AFTER even though
    /// the header channel stays open (a stalled walker must not stall
    /// first work). The header channel is deliberately never closed
    /// until after the first payload arrives.
    #[tokio::test]
    async fn partial_batch_flushes_on_timer_while_scan_still_open() {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("s"), tmp.path().join("d"));
        std::fs::create_dir_all(&src).unwrap();

        let (header_tx, header_rx) = mpsc::channel(64);
        let (payload_tx, mut payload_rx) = mpsc::channel(64);
        let h = header_for(&src, "a.txt", b"hello");
        let planner = tokio::spawn(run_streaming_plan(
            header_rx,
            inputs(&src, &dst),
            payload_tx,
            Instant::now(),
        ));

        header_tx.send(h).await.unwrap();
        // Channel stays open — only the timer can flush. Bound the
        // wait so a regression fails fast instead of hanging.
        let first = tokio::time::timeout(Duration::from_secs(10), payload_rx.recv())
            .await
            .expect("timer flush must produce the first payload while the scan is still open")
            .expect("payload expected");
        drop(first);

        drop(header_tx);
        let outcome = planner.await.unwrap().unwrap();
        assert_eq!(outcome.scanned_files, 1);
        assert_eq!(outcome.plan_updates, 1);
        assert!(outcome.first_payload_elapsed.is_some());
        assert!(outcome.source_paths.contains("a.txt"));
    }

    /// A full batch flushes immediately (no timer wait), and the
    /// remainder flushes on channel close — payload/file counts must
    /// be exact across the boundary.
    #[tokio::test]
    async fn full_batch_flushes_without_close_and_remainder_on_close() {
        let tmp = tempdir().unwrap();
        let (src, dst) = (tmp.path().join("s"), tmp.path().join("d"));
        std::fs::create_dir_all(&src).unwrap();

        let (header_tx, header_rx) = mpsc::channel(1024);
        let (payload_tx, mut payload_rx) = mpsc::channel(1024);
        let total = STREAMING_PLAN_BATCH_HEADERS + 3;
        for idx in 0..total {
            let h = header_for(&src, &format!("f{idx}.bin"), b"x");
            header_tx.send(h).await.unwrap();
        }
        let planner = tokio::spawn(run_streaming_plan(
            header_rx,
            inputs(&src, &dst),
            payload_tx,
            Instant::now(),
        ));

        // First payload must arrive while the channel is still open
        // (the 512-batch flushed on size).
        let first = tokio::time::timeout(Duration::from_secs(10), payload_rx.recv())
            .await
            .expect("size flush must fire with the channel open")
            .expect("payload expected");
        drop(first);

        drop(header_tx);
        let collected = Arc::new(Mutex::new(1usize)); // first already taken
        while let Some(_p) = payload_rx.recv().await {
            *collected.lock().unwrap() += 1;
        }
        let outcome = planner.await.unwrap().unwrap();
        assert_eq!(outcome.scanned_files, total);
        assert_eq!(outcome.plan_updates, 2, "512-batch + remainder");
        // Every header planned into some payload (tiny files group
        // into tar shards, so payload count < file count — assert via
        // the outcome's scanned totals rather than payload count).
        assert!(*collected.lock().unwrap() >= 2);
    }
}
