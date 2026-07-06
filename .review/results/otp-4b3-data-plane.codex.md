Reading additional input from stdin...
OpenAI Codex v0.142.5
--------
workdir: /home/michael/dev/Blit
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f35d5-df00-7212-91fa-7f8c1325040f
--------
user
Review the diff of commit 3ae0a5f (run: git show 3ae0a5f). It implements ONE_TRANSFER_PATH slice otp-4b-3 (deterministic mid-transfer cancel e2e for the unified transfer session; spec: docs/plan/ONE_TRANSFER_PATH.md and the otp-4b-3 section of .review/findings/otp-4b-data-plane.md). The goal: a CancelJob fired while payload bytes flow over the TCP data plane must surface to the client as SessionFault{CANCELLED} (the peer's framed reason), not the data-plane Broken-pipe, and must not hang; the daemon must tear the job down cleanly. Check: correctness regressions, the acceptance criteria, FAST/SIMPLE/RELIABLE, cancellation/StallGuard/byte-identical invariants for transfer code, whether the new select! in source_send_half can drop or reorder legitimate events (needs/summary) or lose the SinkOutcome, whether recv_peer_fault parking on None or prefer_peer_fault's TRANSFER_STALL_TIMEOUT fallback can hang or mislabel a genuine local transport error, and that the test count did not drop (1513 -> 1515). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll inspect the commit diff, the referenced plan/finding sections, and the surrounding transfer code/tests to check the requested invariants. No edits or skills.
exec
/usr/bin/zsh -lc 'git show --stat --oneline --decorate --find-renames 3ae0a5f' in /home/michael/dev/Blit
 succeeded in 0ms:
3ae0a5f (HEAD -> master) otp-4b-3: source surfaces CANCELLED on mid-transfer cancel + e2e
 .review/findings/otp-4b-data-plane.md              |  90 +++++++++++-
 crates/blit-core/src/transfer_session/mod.rs       |  95 ++++++++++++-
 .../src/service/transfer_session_e2e.rs            | 155 +++++++++++++++++++++
 3 files changed, 338 insertions(+), 2 deletions(-)

exec
/usr/bin/zsh -lc 'git show --find-renames --format=fuller --stat --patch 3ae0a5f' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 3ae0a5f4525ccd7bd1c9f4a0910c0ec67a7e3670
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Mon Jul 6 01:10:06 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Mon Jul 6 01:10:06 2026 -0400

    otp-4b-3: source surfaces CANCELLED on mid-transfer cancel + e2e
    
    A CancelJob fired while payload bytes are in flight over the TCP data
    plane now surfaces to the client as SessionFault{CANCELLED} — the peer's
    framed abort reason — instead of the data-plane Broken-pipe (INTERNAL)
    the cancel also causes, and never hangs.
    
    source_send_half races the payload drain (SourceDataPlane::finish, where
    a push spends its byte-transfer wall time) against a peer-framed fault on
    the control lane: recv_peer_fault wins if the fault arrives while the
    drain is still pending (dropping the finish() future aborts the in-flight
    workers via AbortOnDrop, so a worker blocked reading a slow file no
    longer hangs), and prefer_peer_fault prefers the framed reason over a
    data-plane error that surfaces first (bounded by TRANSFER_STALL_TIMEOUT).
    
    Tests (1513 -> 1515):
    - mid_transfer_cancel_surfaces_cancelled_over_the_data_plane (e2e):
      a source stuck mid-payload after one chunk, cancel fired via the row's
      token; asserts client CANCELLED (no hang) + daemon drains active[].
      Guard: reverting the select hangs the client (timeout FAIL).
    - prefer_peer_fault_prefers_a_framed_fault (unit): framed CANCELLED wins
      over a DATA_PLANE_FAILED break. Guard: reverting returns the break.
    
    Finding: .review/findings/otp-4b-data-plane.md (otp-4b-3 section).
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
---
 .review/findings/otp-4b-data-plane.md              |  90 +++++++++++-
 crates/blit-core/src/transfer_session/mod.rs       |  95 ++++++++++++-
 .../src/service/transfer_session_e2e.rs            | 155 +++++++++++++++++++++
 3 files changed, 338 insertions(+), 2 deletions(-)

diff --git a/.review/findings/otp-4b-data-plane.md b/.review/findings/otp-4b-data-plane.md
index dc7732e..179308f 100644
--- a/.review/findings/otp-4b-data-plane.md
+++ b/.review/findings/otp-4b-data-plane.md
@@ -11,10 +11,98 @@ that fix introduced; confirming re-review PASS). Suite 1509 → **1512/0**.
 **PASS** (no findings; the one load-bearing busy-spin bug was caught in
 the author's pre-commit e2e and fixed before the reviewed commit —
 verdict `.review/results/otp-4b2-data-plane.gpt-verdict.md`). Suite 1512
-→ **1513/0**. 4b-3 (cancel e2e) queued.
+→ **1513/0**. 4b-3 (deterministic mid-transfer cancel e2e + source-side
+cancel responsiveness) **implemented** — suite 1513 → **1515/0**; codex
+review pending.
 
 ---
 
+## otp-4b-3 (deterministic mid-transfer cancel e2e) — implemented
+
+### What
+Pin, deterministically, that a `CancelJob` fired while payload bytes are
+in flight over the TCP data plane surfaces to the client as
+`SessionFault{CANCELLED}` (the peer's framed abort reason) — not the
+data-plane transport break the cancel also causes — and that the daemon
+tears the job down cleanly. Building the e2e surfaced that the current
+source could **not** meet that contract, so this slice is a small
+source-side reliability fix plus its guard tests.
+
+### Problem found (empirically, before the fix)
+The daemon side was already correct: on a `CancelJob` the served
+`Transfer` dispatcher (`core.rs::resolve_transfer_session_outcome`,
+otp-4a codex F1) drops the `run_destination` future and frames
+`SessionError{CANCELLED}` on the control lane. But the SOURCE only
+consulted the control lane when it happened to be parked at
+`events.recv()`. During the **payload drain** (`SourceDataPlane::finish`,
+where a push spends its byte-transfer wall time) the send half awaited
+only the data-plane pipeline. So a mid-transfer cancel dropped the
+destination → the source's socket write hit `Broken pipe` first → the
+client surfaced `SessionFault{INTERNAL}` "Broken pipe", and if a worker
+was blocked reading a slow file (never writing) the socket break never
+unblocked it and the client **hung**. (Both observed with a throwaway
+gated-source experiment.)
+
+### Approach (source-side fix, `transfer_session/mod.rs`)
+`source_send_half` now races the data-plane drain against a peer-framed
+fault on the control lane, covering both orderings:
+- `recv_peer_fault(events)` — awaits the next `SourceEvent::Fault` the
+  receive half forwards. In a `biased` `select!` against `dp.finish()`,
+  if the framed fault arrives while the drain is still pending (e.g. a
+  worker blocked reading), it wins; dropping the unfinished `finish()`
+  future drops the `SourceDataPlane`, whose `AbortOnDrop` stops the
+  in-flight workers. This is the fix that makes the blocked-reader case
+  terminate as CANCELLED instead of hanging.
+- `prefer_peer_fault(events, dp_err)` — when the socket break makes
+  `finish()` return `Err` first, prefer the framed reason if the control
+  lane delivers one within `TRANSFER_STALL_TIMEOUT` (the peer runs the
+  same stall guard on its receive workers, so within that window it
+  always frames the real reason); otherwise fall back to the raw
+  data-plane error.
+
+### Files
+- `crates/blit-core/src/transfer_session/mod.rs` — `recv_peer_fault` +
+  `prefer_peer_fault` helpers; `source_send_half`'s finish() drain wrapped
+  in the `select!`; `use …stall_guard::TRANSFER_STALL_TIMEOUT`.
+- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — the harness
+  now retains an `ActiveJobs` clone (to fire the row's cancel token, which
+  is exactly what `cancel_authorized` fires); `StuckAfterFirstChunkSource`;
+  the cancel e2e.
+
+### Tests
+Suite 1513 → **1515** (+2):
+- `mid_transfer_cancel_surfaces_cancelled_over_the_data_plane`
+  (blit-daemon e2e) — a `StuckAfterFirstChunkSource` writes one 64 KiB
+  chunk over the data plane then blocks; the test waits for that chunk
+  (bytes provably flowed), fires the row's cancel token, and asserts the
+  client returns `SessionFault{CANCELLED}` within 10 s (no hang) and the
+  daemon drains the row from `active[]`.
+- `prefer_peer_fault_prefers_a_framed_fault` (blit-core unit) — a framed
+  `CANCELLED` on the events channel wins over a `DATA_PLANE_FAILED`
+  data-plane error.
+
+### Guard proof
+- e2e: reverting the `select!` to `dp.finish().await?` makes the blocked
+  reader hang → the client's 10 s timeout trips → test FAILS
+  ("client must not hang on a mid-transfer cancel: Elapsed"). Restored →
+  passes.
+- unit: reverting `prefer_peer_fault` to return `dp_err` yields
+  `DataPlaneFailed` and the assert FAILS ("framed CANCELLED must win").
+  Restored → passes.
+
+### Known gaps (new)
+- A cancel while an *earlier* `dp.queue()` batch is blocked on pipeline
+  backpressure (multi-file, sustained) still surfaces the data-plane
+  error rather than CANCELLED — `queue()` is not raced (racing it would
+  consume live `Need` events on the happy path). The drain (`finish()`)
+  is where a push spends its transfer wall time, so this is the dominant
+  path; the queue-backpressure edge is a follow-up. The peer's stall
+  guard still bounds it.
+- The RPC-level `CancelJob` mapping (auth via `cancel_authorized`,
+  gRPC outcome codes) is exercised by its own unit tests; this e2e fires
+  the same cancellation token directly to keep the session-propagation
+  assertion deterministic.
+
 ## otp-4b-2 (resize + multi-stream + sf-2 pin) — implemented
 
 ### What
diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
index c4d4322..79c2708 100644
--- a/crates/blit-core/src/transfer_session/mod.rs
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -40,6 +40,7 @@ use crate::remote::transfer::diff_planner;
 use crate::remote::transfer::payload::PreparedPayload;
 use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
 use crate::remote::transfer::source::TransferSource;
+use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
 use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
 use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
 use crate::transfer_plan::PlanOptions;
@@ -932,8 +933,33 @@ async fn source_send_half(
     // Close the data plane BEFORE SourceDone so the destination's receive
     // pipeline sees each socket's END record and completes; SourceDone on
     // the control lane then lets the destination score and summarize.
+    //
+    // The drain is the byte-transfer phase's wall-time sink, so a
+    // mid-transfer cancel almost always lands here. Race it against a
+    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
+    // the served session frames `SessionError{CANCELLED}`, and the source
+    // must surface THAT — not the data-plane transport break it also
+    // causes. Two orderings, both covered:
+    //   * fault arrives while the drain is still pending (e.g. a worker
+    //     blocked reading a slow file, so the socket break never unblocks
+    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
+    //     `finish()` future drops the data plane, and its `AbortOnDrop`
+    //     stops the in-flight workers.
+    //   * the socket break makes `finish()` return `Err` first → prefer
+    //     the framed reason if the control lane delivers one within the
+    //     stall window (`prefer_peer_fault`).
     if let Some(dp) = data_plane.take() {
-        dp.finish().await?;
+        tokio::select! {
+            biased;
+            fault = recv_peer_fault(&mut events) => {
+                return Err(eyre::Report::new(fault));
+            }
+            res = dp.finish() => {
+                if let Err(dp_err) = res {
+                    return Err(prefer_peer_fault(&mut events, dp_err).await);
+                }
+            }
+        }
     }
 
     tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
@@ -1135,6 +1161,39 @@ async fn resolve_in_flight_resize(
     }
 }
 
+/// Await the next peer-framed fault the receive half forwards on the
+/// control lane, ignoring any non-fault event. Used to race the
+/// data-plane drain (otp-4b-3): a mid-transfer `SessionError` (e.g. a
+/// `CancelJob` → `CANCELLED`) must abort the send and surface as the
+/// fault. Parks forever once the channel closes with no fault so the
+/// data-plane future it races decides the outcome instead — during the
+/// drain the receive half only ever forwards a fault (SourceDone has not
+/// gone out, so no summary; the resize was already resolved).
+async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
+    loop {
+        match events.recv().await {
+            Some(SourceEvent::Fault(fault)) => return fault,
+            Some(_) => continue,
+            None => std::future::pending().await,
+        }
+    }
+}
+
+/// A data-plane operation failed mid-transfer. The break is usually the
+/// *symptom* of a peer abort — within `TRANSFER_STALL_TIMEOUT` the peer
+/// (which runs the same stall guard on its receive workers) always frames
+/// the real reason on the control lane. Prefer that framed fault; fall
+/// back to the raw data-plane error if none arrives in that window.
+async fn prefer_peer_fault(
+    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
+    dp_err: eyre::Report,
+) -> eyre::Report {
+    match tokio::time::timeout(TRANSFER_STALL_TIMEOUT, recv_peer_fault(events)).await {
+        Ok(fault) => eyre::Report::new(fault),
+        Err(_) => dp_err,
+    }
+}
+
 /// Plan one batch of needed headers with the engine planner and emit
 /// the resulting payload records per the in-stream grammar.
 async fn send_payload_records(
@@ -1852,6 +1911,40 @@ mod tests {
         assert!(!git.is_empty(), "git component must be non-empty");
     }
 
+    /// otp-4b-3: a data-plane break during the drain prefers the peer's
+    /// framed reason. When the receive half has forwarded a
+    /// `SessionError{CANCELLED}` on the control lane, `prefer_peer_fault`
+    /// returns THAT fault, not the raw data-plane transport error — the
+    /// non-timeout half of the mid-transfer-cancel guard (the e2e in
+    /// `blit-daemon` guards the still-pending-drain half).
+    #[tokio::test]
+    async fn prefer_peer_fault_prefers_a_framed_fault() {
+        let (tx, mut rx) = mpsc::unbounded_channel::<SourceEvent>();
+        // The peer framed CANCELLED on the control lane before we ask.
+        tx.send(SourceEvent::Fault(SessionFault {
+            code: session_error::Code::Cancelled,
+            message: "transfer cancelled via CancelJob".into(),
+            local_build_id: String::new(),
+            peer_build_id: String::new(),
+            peer_notified: true,
+        }))
+        .expect("send fault");
+
+        let dp_err = eyre::Report::new(SessionFault::refusal(
+            session_error::Code::DataPlaneFailed,
+            "Broken pipe (os error 32)",
+        ));
+        let chosen = prefer_peer_fault(&mut rx, dp_err).await;
+        let fault = chosen
+            .downcast_ref::<SessionFault>()
+            .expect("a SessionFault");
+        assert_eq!(
+            fault.code,
+            session_error::Code::Cancelled,
+            "the framed CANCELLED must win over the data-plane break"
+        );
+    }
+
     #[test]
     fn fault_round_trips_the_wire_shape() {
         let fault = SessionFault {
diff --git a/crates/blit-daemon/src/service/transfer_session_e2e.rs b/crates/blit-daemon/src/service/transfer_session_e2e.rs
index 6ac93c3..e86eef5 100644
--- a/crates/blit-daemon/src/service/transfer_session_e2e.rs
+++ b/crates/blit-daemon/src/service/transfer_session_e2e.rs
@@ -50,6 +50,7 @@ struct Daemon {
     server: Option<tokio::task::JoinHandle<()>>,
     _dest: tempfile::TempDir,
     dest_root: PathBuf,
+    active_jobs: crate::active_jobs::ActiveJobs,
 }
 
 impl Daemon {
@@ -69,6 +70,7 @@ impl Daemon {
             },
         );
         let service = BlitService::with_modules(modules, false);
+        let active_jobs = service.active_jobs.clone();
         let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
             .await
             .expect("bind loopback listener");
@@ -100,6 +102,7 @@ impl Daemon {
             server: Some(server),
             _dest: dest,
             dest_root: canonical,
+            active_jobs,
         }
     }
 
@@ -192,6 +195,158 @@ fn fault_of(err: &eyre::Report) -> &SessionFault {
         .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
 }
 
+// --- otp-4b-3: deterministic mid-transfer cancel over the data plane ---
+
+/// A `TransferSource` that puts a transfer into a provably-stuck
+/// mid-payload state: `open_file` writes exactly one 64 KiB chunk over
+/// the data plane (so bytes have demonstrably flowed), signals `started`,
+/// then blocks forever without emitting the rest of the file. The
+/// transfer therefore cannot complete on its own — the only exits are the
+/// cancel under test or the reader being dropped when the session aborts.
+/// Everything else delegates to the real filesystem source.
+struct StuckAfterFirstChunkSource {
+    inner: FsTransferSource,
+    started: Arc<tokio::sync::Notify>,
+}
+
+#[async_trait::async_trait]
+impl blit_core::remote::transfer::source::TransferSource for StuckAfterFirstChunkSource {
+    fn scan(
+        &self,
+        filter: Option<FileFilter>,
+        unreadable: Arc<std::sync::Mutex<Vec<String>>>,
+    ) -> (
+        tokio::sync::mpsc::Receiver<blit_core::generated::FileHeader>,
+        tokio::task::JoinHandle<eyre::Result<u64>>,
+    ) {
+        self.inner.scan(filter, unreadable)
+    }
+
+    async fn prepare_payload(
+        &self,
+        payload: blit_core::remote::transfer::payload::TransferPayload,
+    ) -> eyre::Result<blit_core::remote::transfer::payload::PreparedPayload> {
+        self.inner.prepare_payload(payload).await
+    }
+
+    async fn check_availability(
+        &self,
+        headers: Vec<blit_core::generated::FileHeader>,
+        unreadable: Arc<std::sync::Mutex<Vec<String>>>,
+    ) -> eyre::Result<Vec<blit_core::generated::FileHeader>> {
+        self.inner.check_availability(headers, unreadable).await
+    }
+
+    async fn open_file(
+        &self,
+        header: &blit_core::generated::FileHeader,
+    ) -> eyre::Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
+        let mut inner = self.inner.open_file(header).await?;
+        // A generous duplex buffer so the one chunk lands without the
+        // writer parking on backpressure.
+        let (mut w, r) = tokio::io::duplex(256 * 1024);
+        let started = Arc::clone(&self.started);
+        tokio::spawn(async move {
+            use tokio::io::{AsyncReadExt, AsyncWriteExt};
+            let mut buf = vec![0u8; 64 * 1024];
+            if let Ok(n) = inner.read(&mut buf).await {
+                if n > 0 && w.write_all(&buf[..n]).await.is_ok() {
+                    started.notify_one();
+                }
+            }
+            // Hold the write half open (no EOF) and never write again:
+            // the transfer is now stuck mid-payload until the session is
+            // aborted (which drops this task) or cancelled.
+            std::future::pending::<()>().await;
+            drop(w);
+        });
+        Ok(Box::new(r))
+    }
+
+    fn root(&self) -> &Path {
+        self.inner.root()
+    }
+}
+
+/// otp-4b-3: fire a `CancelJob`-equivalent (the row's cancellation token,
+/// exactly what the RPC handler fires) while a payload is stuck mid-flight
+/// over the TCP data plane. The client must surface
+/// `SessionFault{CANCELLED}` — the peer's framed abort reason — rather
+/// than the data-plane transport break it also causes, and it must not
+/// hang. The daemon must then tear the job down cleanly (the active row
+/// drains).
+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+async fn mid_transfer_cancel_surfaces_cancelled_over_the_data_plane() {
+    let daemon = Daemon::start(false).await;
+    let src = tempfile::tempdir().unwrap();
+    // One file larger than a single chunk, so the stuck reader keeps the
+    // transfer provably incomplete after its first 64 KiB.
+    std::fs::write(src.path().join("big.bin"), vec![0xABu8; 4 * 1024 * 1024]).unwrap();
+
+    let started = Arc::new(tokio::sync::Notify::new());
+    let source = Arc::new(StuckAfterFirstChunkSource {
+        inner: FsTransferSource::new(src.path().to_path_buf()),
+        started: Arc::clone(&started),
+    });
+
+    let ep = daemon.endpoint.clone();
+    let client =
+        tokio::spawn(
+            async move { run_push_session(&ep, source, PushSessionOptions::default()).await },
+        );
+
+    // Bytes have flowed over the data plane and the transfer is now stuck
+    // mid-payload — a deterministic mid-transfer point.
+    tokio::time::timeout(std::time::Duration::from_secs(10), started.notified())
+        .await
+        .expect("payload bytes should flow over the data plane before cancel");
+
+    // Fire the row's cancellation token — exactly what the `CancelJob` RPC
+    // handler does via `cancel_authorized` (audit-9). The RPC-level
+    // mapping (auth, outcome codes) is unit-tested separately; this pins
+    // the end-to-end propagation through the served session.
+    let transfer_id = daemon
+        .active_jobs
+        .snapshot()
+        .into_iter()
+        .next()
+        .expect("an active transfer row")
+        .transfer_id;
+    assert_eq!(
+        daemon.active_jobs.cancel(&transfer_id),
+        crate::active_jobs::CancelOutcome::Cancelled,
+        "the served session's row honors cancellation"
+    );
+
+    // The client must surface CANCELLED promptly (no hang).
+    let result = tokio::time::timeout(std::time::Duration::from_secs(10), client)
+        .await
+        .expect("client must not hang on a mid-transfer cancel")
+        .expect("client task joins");
+    let err = result.expect_err("a cancelled transfer fails");
+    assert_eq!(
+        fault_of(&err).code,
+        session_error::Code::Cancelled,
+        "the client surfaces the peer's framed CANCELLED, not the data-plane break: {err:#}"
+    );
+
+    // Daemon tears down cleanly: the active row drains.
+    let mut drained = false;
+    for _ in 0..200 {
+        if daemon.active_jobs.snapshot().is_empty() {
+            drained = true;
+            break;
+        }
+        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
+    }
+    assert!(
+        drained,
+        "the daemon must drain the cancelled job from active[]"
+    );
+
+    daemon.stop().await;
+}
+
 // ---------------------------------------------------------------------------
 // Tests
 // ---------------------------------------------------------------------------

exec
/usr/bin/zsh -lc "rg -n \"otp-4b-3|ONE_TRANSFER_PATH|CancelJob|source_send_half|recv_peer_fault|prefer_peer_fault|TRANSFER_STALL_TIMEOUT|SessionFault|CANCELLED|1513|1515\" docs/plan/ONE_TRANSFER_PATH.md .review/findings/otp-4b-data-plane.md src tests crates . -g '"'!target'"'" in /home/michael/dev/Blit
 exited 2 in 0ms:
rg: src: No such file or directory (os error 2)
rg: tests: No such file or directory (os error 2)
.review/findings/otp-4b-data-plane.md:3:**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-4.
.review/findings/otp-4b-data-plane.md:14:→ **1513/0**. 4b-3 (deterministic mid-transfer cancel e2e + source-side
.review/findings/otp-4b-data-plane.md:15:cancel responsiveness) **implemented** — suite 1513 → **1515/0**; codex
.review/findings/otp-4b-data-plane.md:20:## otp-4b-3 (deterministic mid-transfer cancel e2e) — implemented
.review/findings/otp-4b-data-plane.md:23:Pin, deterministically, that a `CancelJob` fired while payload bytes are
.review/findings/otp-4b-data-plane.md:25:`SessionFault{CANCELLED}` (the peer's framed abort reason) — not the
.review/findings/otp-4b-data-plane.md:32:The daemon side was already correct: on a `CancelJob` the served
.review/findings/otp-4b-data-plane.md:35:`SessionError{CANCELLED}` on the control lane. But the SOURCE only
.review/findings/otp-4b-data-plane.md:41:client surfaced `SessionFault{INTERNAL}` "Broken pipe", and if a worker
.review/findings/otp-4b-data-plane.md:47:`source_send_half` now races the data-plane drain against a peer-framed
.review/findings/otp-4b-data-plane.md:49:- `recv_peer_fault(events)` — awaits the next `SourceEvent::Fault` the
.review/findings/otp-4b-data-plane.md:55:  terminate as CANCELLED instead of hanging.
.review/findings/otp-4b-data-plane.md:56:- `prefer_peer_fault(events, dp_err)` — when the socket break makes
.review/findings/otp-4b-data-plane.md:58:  lane delivers one within `TRANSFER_STALL_TIMEOUT` (the peer runs the
.review/findings/otp-4b-data-plane.md:64:- `crates/blit-core/src/transfer_session/mod.rs` — `recv_peer_fault` +
.review/findings/otp-4b-data-plane.md:65:  `prefer_peer_fault` helpers; `source_send_half`'s finish() drain wrapped
.review/findings/otp-4b-data-plane.md:66:  in the `select!`; `use …stall_guard::TRANSFER_STALL_TIMEOUT`.
.review/findings/otp-4b-data-plane.md:73:Suite 1513 → **1515** (+2):
.review/findings/otp-4b-data-plane.md:78:  client returns `SessionFault{CANCELLED}` within 10 s (no hang) and the
.review/findings/otp-4b-data-plane.md:80:- `prefer_peer_fault_prefers_a_framed_fault` (blit-core unit) — a framed
.review/findings/otp-4b-data-plane.md:81:  `CANCELLED` on the events channel wins over a `DATA_PLANE_FAILED`
.review/findings/otp-4b-data-plane.md:89:- unit: reverting `prefer_peer_fault` to return `dp_err` yields
.review/findings/otp-4b-data-plane.md:90:  `DataPlaneFailed` and the assert FAILS ("framed CANCELLED must win").
.review/findings/otp-4b-data-plane.md:96:  error rather than CANCELLED — `queue()` is not raced (racing it would
.review/findings/otp-4b-data-plane.md:101:- The RPC-level `CancelJob` mapping (auth via `cancel_authorized`,
.review/findings/otp-4b-data-plane.md:128:  `mod.rs`: `source_send_half` accumulates `needed_bytes/count`,
.review/findings/otp-4b-data-plane.md:162:  `source_recv_half` forwards it; `source_send_half` shape-correction +
.review/findings/otp-4b-data-plane.md:174:- Mid-transfer cancel e2e → otp-4b-3.
.review/findings/otp-4b-data-plane.md:207:  — the per-direction drivers ONE_TRANSFER_PATH deletes at otp-10. The
.review/findings/otp-4b-data-plane.md:244:- **otp-4b-3 (mid-transfer cancel e2e)** — deterministic test that fires
.review/findings/otp-4b-data-plane.md:245:  `CancelJob` while bytes flow over the data plane and asserts the client
.review/findings/otp-4b-data-plane.md:246:  surfaces `SessionFault{CANCELLED}` and the daemon tears down cleanly.
.review/findings/otp-4b-data-plane.md:273:**Initiator (SOURCE) side — `run_source` / `source_send_half`:**
.review/findings/otp-4b-data-plane.md:313:  `in_stream_bytes` or bind fails); `source_send_half` dials up front and
.review/findings/otp-4b-data-plane.md:355:- Mid-transfer cancel e2e → otp-4b-3.
./DEVLOG.md:8:**2026-07-06 00:30:00Z** - **CODER (otp-4b-2 resize + multi-stream + sf-2 pin, claude)**: The unified session's data plane now grows mid-transfer — the zero-knowledge single-stream grant shape-corrects upward as the need list accumulates, over real sockets (`dce56de`). **SOURCE owns the live dial** (`TransferDial::conservative_within(receiver_capacity)`, seeded to the granted epoch-0 count; pool provisioned for `ceiling_max_streams`, matching old push's lazy-alloc rationale). As needs accumulate it re-runs the shape table (`initial_stream_proposal(needed_bytes, needed_count, ceiling)`) and calls `propose_shape_resize` — one ADD per epoch, one in flight (the dial's `pending_epoch` CAS); mints a 16-byte sub-token, sends `DataPlaneResize{ADD}` on the control lane, and on `DataPlaneResizeAck` dials the epoch-N socket (`session_token ‖ sub_token`) and `SinkControl::Add`s it to the now-**elastic** pipeline (`execute_sink_pipeline_elastic`). `source_send_half` gained `needed_bytes/count` accumulators, `maybe_propose_resize`, `process_source_event` (ResizeAck → dial + settle + propose-next ramp), and `resolve_in_flight_resize` (drains the one in-flight proposal BEFORE `dp.finish()` so no armed dest slot orphans). **DESTINATION** runs a resize-aware `accept_loop` (`ResponderDataPlane::spawn` → select over arm-channel / bounded accept / worker-join); the control loop handles `Frame::Resize` (ceiling-checked arm + `resize_live` bump + `DataPlaneResizeAck`), and at SourceDone `finish()`es the run for the settled stream count on `DestinationOutcome.data_plane_streams`. **Orphan-free by construction**: a source resize-dial failure is FATAL (session faults → AbortOnDrop kills the dest accept loop), a deliberate simplification vs old push's arm-TTL recovery (noted in the finding doc). **Bug caught in pre-commit e2e**: the dest accept loop busy-spun once `arm_tx` dropped — a closed `mpsc` resolves `recv()` to `None` every poll and, as the biased-first select arm, starved `join_next`, so finished receive workers were never collected and `finish()` hung (reproduced on the gRPC data-plane e2e); fixed by parking the arm branch on `pending()` once closed (the same guard `execute_sink_pipeline_elastic` uses). sf-2 pin ported: `many_tiny_files_shape_correct_to_more_than_one_stream` (10k tiny files over the data plane settle `data_plane_streams > 1`, guard-proven: neuter the proposal → settles at 1 → pin fails). **Codex: PASS, no findings** (source resize sequencing, dest accept/finish teardown, byte accounting, StallGuard wiring, sf-2 pin all sound; verdict `.review/results/otp-4b2-data-plane.gpt-verdict.md`). Suite 1512 → **1513/0**, fmt/clippy clean. Known gaps → otp-4b-3 (mid-transfer cancel e2e), cheap-dial live tuner still deferred. Next: otp-4b-3.
./DEVLOG.md:10:**2026-07-05 23:00:00Z** - **CODER (otp-4b-1 TCP data plane onto the session, single stream, claude)**: The push-equivalent now rides the real TCP data plane, not the in-stream carrier (`881d412` + review fix `e1aafcc`). New `blit-core/src/transfer_session/data_plane.rs` owns the session's OWN data-plane choreography, reusing blit-core primitives (`DataPlaneSession` framing, `execute_receive_pipeline`, `execute_sink_pipeline_streaming`, `dial_data_plane`, `TransferDial`/`initial_stream_proposal`, `generate_sub_token`) — **no call into `remote::push` or the daemon push service** (those per-direction drivers die at otp-10; codex confirmed the boundary clean). DESTINATION responder binds `0.0.0.0:0`, mints session_token + epoch0_sub_token (16B each), computes `initial_streams = initial_stream_proposal(0,0,ceiling) == 1` (the grant is issued before any manifest — so the session data plane always starts single-stream; multi-stream is resize-only, otp-4b-2), and grants them in `SessionAccept`; a bind/RNG failure falls back to a grant-less accept (in-stream). SOURCE dials up front (before streaming the manifest, so the DEST accept task sees the connection before its bounded-accept timeout), authenticates `session_token ‖ epoch0_sub_token`, and feeds planned payloads to `execute_sink_pipeline_streaming` over a `DataPlaneSink`; `finish()` drains + writes each socket's END before `SourceDone`. DEST arms the accept+receive task concurrent with the diff loop (AbortOnDrop-bounded); control-lane payload frames under a data plane are `PROTOCOL_VIOLATION`. Default carrier flipped to the data plane (`PushSessionOptions.in_stream_bytes`, default false); in-stream stays the requested `--force-grpc`-shaped fallback. **A/B parity over the data plane** vs old push holds byte-identically. **Codex: FAIL, 2/2 accepted+fixed** (`e1aafcc`): **F1 (High)** completion was a weak count proxy (`files_written == needed_paths.len()`) — a peer could substitute/duplicate paths or send non-resume block records and pass; fixed by a shared `outstanding` need set both carriers claim from (control loop inserts before each NeedBatch → insert-happens-before-payload, no race) enforced on the data plane by a new `NeedListSink` decorator (membership + no-duplicate + reject resume blocks), completion is `outstanding.is_empty()` for both carriers; guard-proven by `need_list_sink_enforces_membership_and_rejects_blocks`. **F2 (High)** no read-side StallGuard on the data-plane receive — a peer that auths then stalls hung the DEST at `recv.join()`; fixed by wrapping each accepted socket in `StallGuard::new(socket, TRANSFER_STALL_TIMEOUT)` (matching old push). Nothing rejected. Suite 1509 → **1512/0**, fmt/clippy clean. Re-review of `e1aafcc` (the fix added shared-set concurrency + a sink decorator) in flight. Known gaps → otp-4b-2 (resize + multi-stream + sf-2 pin), otp-4b-3 (deterministic mid-transfer cancel e2e). Next: otp-4b-2.
./DEVLOG.md:12:**2026-07-05 21:30:00Z** - **CODER (otp-4a daemon serves Transfer, client SOURCE, claude)**: The push-equivalent rides the unified session end to end (`4b07bbb` + review fix `25f538b` + sha `fe4ad6d`). The daemon `Transfer` RPC — UNIMPLEMENTED since otp-1 — now runs `run_destination` as Responder; `core.rs::transfer` mirrors `push` (jobs row + cancel/hangup race). New gRPC `FrameTransport` adapters (`transport.rs`: `GrpcFrameRx` over `tonic::Streaming`, client/daemon `FrameTx`, `grpc_{client,daemon}_transport`). New client entry `session_client::run_push_session` runs `run_source` as SOURCE initiator. **Responder-resolution API**: `run_destination`'s 3rd param is now `DestinationTarget::{Fixed,Resolve}`; a `Resolve` responder maps the received `SessionOpen`→root via an async `OpenResolver` threaded through `establish` (after `validate_open`, before `SessionAccept`, so a refusal REPLACES the accept). Daemon closure (`service/transfer.rs::make_open_resolver`) wraps the push module-resolution + F2 containment chain, mapping `tonic::Status`→`SessionFault` (blit-core stays Status-free). **Compare fork resolved** (workflow-verified): the sole push/pull divergence is same-size+dest-newer (push clobbers, pull/session safely skip); session keeps the safe skip (converge-up + data-safety; shared arm untouched → live pull_sync unchanged; no test pinned push's clobber). Pinned by `same_size_newer_destination_is_skipped_not_clobbered`. **⚠ narrow owner-ack flagged**: "byte-identical vs old push" isn't literally achievable in that cell; `--force` overrides (STATE open question). **A/B parity proven**: same fixture through old push (TCP data plane) and the session (in-stream) → byte-identical trees + equal shared summary counters. **Codex: NEEDS FIXES, 1/1 accepted+fixed**: F1 (Medium) cancel emitted `Status::cancelled` not a `SessionError{CANCELLED}` frame → `session_error_frame` helper + `resolve_transfer_session_outcome`, unit-guarded by revert. Nothing rejected. Known gaps → otp-4b: TCP data plane, resize, sf-2 pin ported, deterministic mid-transfer cancel e2e; jobs-row endpoint population; daemon-as-SOURCE (otp-5). Suite 1501 → **1509/0**, fmt/clippy clean. Next: otp-4b.
./DEVLOG.md:16:**2026-07-05 10:00:00Z** - **DECISION + CODER (D-2026-07-05-4 Active flip; otp-1 wire contract, claude)**: Owner: "flip the plan and go" + per-slice codex loop re-affirmed → **ONE_TRANSFER_PATH Active** (D-2026-07-05-4, records `3a87ba0`). **otp-1 landed through the codex loop** (`a3e2acb` + review fix `f861579`): `docs/TRANSFER_SESSION.md` (authoritative contract) + the `Transfer` RPC and full session message set in proto + UNIMPLEMENTED stubs (daemon service + five test fakes) + an in-process reachability pin (real client → real service → UNIMPLEMENTED, not UNKNOWN). Contract highlights: ONE role-tagged frame vocabulary both wire directions (no push/pull-shaped message sets exist); SessionHello same-build exact-match handshake FIRST (D-2026-07-05-2, mismatch names both build ids); DESTINATION owns the diff; receiver capacity travels DESTINATION→SOURCE whichever end holds the role (D-2026-06-20-1/-2); sf-2 shape correction = the only stream policy, SOURCE always the resize controller; resume = strictly-ordered RELIABLE exception; mirror deletes destination-local (no delete list on the wire); deliberately absent: PeerCapabilities, spec_version, delete lists. Codex **NEEDS FIXES 6/6 accepted**: closing flow was drawn on initiator/responder lanes (wrong for initiator=DESTINATION — redrawn on ROLE lanes); `initial_streams` respecified as an ACCEPT CEILING so a destination responder can never choose the sender's dial; exact per-socket auth handshake; in-stream carrier record grammar (strict serialization, size-inferred completion, payload-after-ManifestComplete); flow control + NeedComplete ordering named; error-field doc/proto drift. Suite 1483 → **1484/0**, fmt/clippy clean. REVIEW.md gains the otp section. Also this session, before the flip: aarch64-musl cross-build recipe proven (static blit/blit-daemon EXECUTING on zoey, the UNAS zero-copy rig — recipe + rig facts in STATE queue item 5; all zoey activity restricted to the owner's blit-temp folder, zero-copy test pre-authorized there). Next: otp-3 (TransferSession core) — otp-2 (symmetric baseline) is rig-gated and may run out of order, before otp-10.
./DEVLOG.md:18:**2026-07-05 08:45:00Z** - **DECISION (D-2026-07-05-3 — zero-copy receive unparked, claude)**: Owner declared the D-2026-06-12-1 revisit gate met: his UniFi UNAS 8 Pro daemon target is CPU-bound below 10 GbE even from SSD cache — exactly the CPU-bound-receiver case the gate contemplated (skippy's 32-core rig measured 1.43 receive cores at 9.5 Gbit/s and did NOT trip it). Recorded with two clarifications: the dead `zero_copy.rs` module still deletes in w8-1 (EAGAIN busy-wait draft — rewrite, not revival), and the capability returns the one-path way per the same-day owner exchange: a runtime-selected write strategy inside ONE_TRANSFER_PATH's unified receive sink (eval doc's AsyncFd splice design as input; buffered relay stays the universal fallback; selection reads capability + payload type, never role/initiator), sequenced as a follow-on slice set after otp-10 cutover. UNAS is the measurement rig, symmetric-endpoint methodology applies. Resolves the STATE Blocked zero-copy a/b/c question and shrinks the 10 GbE owner declarations from four to three (ue-1, ue-2, REV4 → Shipped). Propagated: eval doc annotated, ONE_TRANSFER_PATH Non-goals + Design write-strategy seam, STATE queue/blocked. Codex review of this docs commit follows.
./DEVLOG.md:20:**2026-07-05 08:15:00Z** - **DECISION (D-2026-07-05-2 — no version compatibility, ever, claude)**: While reviewing the plan-review fixes the owner caught the REV4 "mixed old/new peers must negotiate down" clause surviving into ONE_TRANSFER_PATH ("governs until cutover") and rejected it root-and-branch: **"backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product."** — a rule he had stated repeatedly in chat but which was never recorded, so the written REV4 doc kept winning document-conflict resolutions (exactly the unrecorded-intent failure mode; the codex F1 fix resolved the conflict in the wrong direction). Recorded as **D-2026-07-05-2**: same-build peers only; the unified session's handshake REFUSES mismatched builds at session open (strict build/protocol identity, specified in otp-1, pinned by test); no negotiate-down, no advisory fields, no feature-capability bits (same build = same features); receiver capacity profile explicitly unaffected (hardware negotiation, D-2026-06-20-1/-2). Propagated: REV4 §Constraints clause struck + annotated (effective immediately, not at cutover); SMALL_FILE_CEILING's "mixed-version peers keep working" clause and sf-6's mixed-version-test deliverable struck + annotated; D-2026-07-05-1's "only at cutover" scoping struck; ONE_TRANSFER_PATH Non-goals rewritten + otp-1 gains the same-build handshake deliverable. Codex review of this docs commit follows per D-2026-07-04-1.
./DEVLOG.md:22:**2026-07-05 07:30:00Z** - **DIRECTIVE + PLAN (ONE_TRANSFER_PATH draft, D-2026-07-05-1, claude)**: Owner rejected the push/pull disparity, the mixed-fs benchmark methodology ("tmp on one side, spinning rust on the other is not a valid test"), and the explanation that direction symmetry lives in four separate driver loops — then issued the directive now recorded verbatim in `docs/plan/ONE_TRANSFER_PATH.md`: **"ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"**; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did"; identical whether initiated as push from skippy or pull from this machine. Scope/wire/process explicitly delegated to the agent. **Plan drafted through the plan procedure**: one `TransferSession` (roles SOURCE/DESTINATION selected by initiator/verb, never code paths), one bidi `Transfer` RPC replacing `Push`+`PullSync` (no back-compat, lockstep upgrade — repo precedent PullSyncHeader), one choreography (source streams manifest immediately; destination diffs incrementally; sf-2 shape-corrected dial as the only stream policy — absorbs the pull 1s-start residue), gRPC fallback demoted to a byte-carrier option, delegated pull = daemon-initiated session, local transfers on an in-process transport in the final phase; slices otp-1..12 ending in a **deletion slice** (the four drivers + both RPCs removed, file-by-file proof) and a **symmetric-rig acceptance matrix** (same-fs disk-to-disk verdict cells, cold caches, tmpfs as labeled wire-reference only; initiator/verb A/B within ±10%). Converge-up constraint: per cell the unified path must match the better of today's directions, not the average. **D-2026-07-05-1 recorded**: directive + SMALL_FILE_CEILING paused at sf-2 (sf-3a+ blocked; principle D-2026-07-04-4 stands) + design queue paused (w7-1 likely absorbed by otp-5). STATE rewritten around the new queue. Next: codex plan review of this commit, adjudicate, fix, then STOP for the owner's Active flip — no implementation anywhere until then.
./DEVLOG.md:50:**2026-07-04 14:25:35Z** - **CODER (w4-5-supports-cancellation-flip, claude)**: Landed w4-5 through the codex loop (owner go: "continue" → topmost open row per the 11th handoff). D-2026-07-04-3 executed: `ActiveJobKind::supports_cancellation` flipped from DelegatedPull-only to `!matches!(self, Pull)` — every kind that can hold an active row is now cancellable; only history-only `Pull` stays gated (its RPC died at ue-r2-1h, so the Unsupported/FailedPrecondition/exit-2 arm survives end-to-end purely as the contract's escape hatch). `blit jobs cancel` and the TUI `K`/`Shift+X` now fire the row token for attached Push/PullSync; the w4-3 dispatcher race makes teardown + client `Status::cancelled` real. Slice `05a8b39`. A 3-agent workflow sweep (prod code / tests / docs) enumerated every surface encoding the old policy before editing: confirmed the TUI has NO kind-based cancel gating and the CLI 0/1/2 mapping is outcome-based — the daemon-side predicate is the entire behavior change; zero CLI/TUI logic edits. Comment surfaces updated: `supports_cancellation` rustdoc rewritten as the decision requires (old "disconnecting IS the cancel" rationale recorded as superseded), `CancelOutcome::Unsupported` + blit-app `CancelJobOutcome::Unsupported` docs, `cancel_job`'s FailedPrecondition message de-rationalized, `resolve_streaming_outcome`'s "production-unreachable" note now says the arm is live, `proto/blit.proto`'s CancelJob wire-contract comment (the last flat old-policy statement), `jobs_lifecycle.rs` header, w4-3 finding doc scope note annotated per the decision's Supersedes line. Tests (blit-daemon 168 → 170): policy pin flipped; `cancel_fires_token_for_push_and_pull_sync` + RPC-level `cancel_job_ok_for_push_and_pull_sync` pin the new contract; Pull-only Unsupported pins split out; authz-precedence test re-anchored on Pull AND extended — a different-host caller on a Push row now gets `Unauthorized`, pinning that flipped kinds went under the audit-9 authz gate, not around it. Mutation-verified: reverting the predicate fails exactly those four tests (4/15 under the `cancel` filter), restore → all 19 green. Review: codex **NEEDS FIXES (1 Low)** — the module-level "Scope so far" changelog still said the `pull` dispatch site is wired and "all four kinds are now constructed on the wire path", contradicting the new policy rustdoc 140 lines below; accepted (same doc-drift class codex caught in w1-4), fixed `1708075` (bullets annotated in place, changelog framing kept). Known gap: no e2e drives a live mid-flight attached cancel (needs a test seam or timing the w9-3-flaky daemon-spawn family can't afford; the pin chain covers every link the e2e would compose — same evidence shape w4-3 was graded on). Validation both commits: fmt/clippy clean, `cargo test --workspace` 1448/0/2 across 37 suites (baseline 1446). All on master, unpushed. Next: w2-2 (stream-ladder owner) tops the open queue; design-3 remains the sanctioned smaller alternative.
./DEVLOG.md:52:**2026-07-04 13:53:22Z** - **DECISIONS (owner Q&A, claude)**: The owner asked for the four standing questions "one at a time, no idea what these refer to" — each was presented in plain English with options and answered: (1) **commit erratum → leave as-is** (D-2026-07-04-2; mirrors the D-2026-06-07-1 no-rewrite calculus — two bisect-skippable commits beat force-pushing shared history); (2) **10 GbE session → "soon, but keep coding first"** (STATE Blocked reworded: not a daily blocker; owner will call "benchmark"); (3) **D-2026-06-20-1 stale warmup/size-gate wording → "follow the existing pattern"** — the ledger's own precedent (D-2026-06-20-2's veto annotation, D-2026-06-20-6's struck scope clause) IS edit-in-place-with-annotation, so the superseded framings are struck with pointers to -2 q1 and REV4/-5 (bounded-unilateral untouched — still true), and -5's "remains an open question" note resolved; (4) **supports_cancellation → flip it** (D-2026-07-04-3): CancelJob + TUI F2 will work on attached Push/PullSync transfers; policy-only after w4-3's race wiring; contract change (exit 2→0) recorded; implementation queued as **w4-5-supports-cancellation-flip**, now the topmost open REVIEW.md row. Batch `2a21d6f` through the codex loop per D-2026-07-04-1: **NEEDS FIXES (1 Medium + 1 Low, both STATE.md coherence)** — the Now bullet still called the erratum an open owner call, and the queue rewrite dropped the coder's-pick clause (design-3-vs-w2-2 ordering contradiction); both accepted, fixed `a928193`. The decision content itself passed all cross-checks (ledger consistency, w4-3 scope-note agreement, strike precision). check-docs.sh green. All on master, unpushed.
./DEVLOG.md:54:**2026-07-04 13:36:48Z** - **CODER (w1-4-accept-token-constants, claude)**: Landed w1-4 through the codex loop (same session as w1-2/w1-3 — the W1 transport-policy family is now complete). The accept(30s)/token(15s) bound pair was declared three times at HEAD (the audit counted four; one died with the Pull RPC at ue-r2-1h): the push pair with the full R46-F7 rationale, pull_sync's module-scope `PULL_*` pair, and the resume path's function-local pair behind a `StdDuration2` alias. Slice `6a19e1d`: the pair now lives in `blit_core::remote::transfer::socket` (`DATA_PLANE_ACCEPT_TIMEOUT`/`DATA_PLANE_TOKEN_TIMEOUT`, the push side's doc comments moved with it — the data-plane policy module from w1-2/w1-3 is the natural home), all three local declarations deleted, every daemon use site renamed to the one name (`RESIZE_ARM_TTL` still equals the accept bound); values byte-identical, no behavior change, no new tests (a constant-equality test would be vacuous; the bounded-accept behavior is pinned by the existing audit-h3a-family tests). Records `484e70b`. Review: codex **NEEDS FIXES (1 Low)** — stall_guard.rs's module doc + `TRANSFER_STALL_TIMEOUT` rustdoc still named the deleted `PULL_*` pair (doc drift); accepted, fixed `d17b089` (plus the same lines' stale `daemon::service::{pull, pull_sync}` reference — the pull service died at ue-r2-1h; disclosed in the verdict, not silently bundled). Codex's sweep confirmed no fourth declaration or stray data-plane 30s/15s literal survives (remaining `from_secs(30)` hits are control-plane connect bounds, test harness timeouts, and `TRANSFER_STALL_TIMEOUT` — different policy families, deliberately untouched per the finding doc's Known gaps). Environment note recorded: the codex first attempt hung on the known inherited-stdin quirk (chained `git && codex` invocation); killed, re-run standalone with `< /dev/null` — worked. Validation: fmt/clippy clean, `cargo test --workspace` 1446/0/2 across 37 suites (unchanged). All on master, unpushed. Next open ratified row: design-3 (data-plane connect timeouts — now trivially placeable, the call sites are consolidated and the bound can import the shared pair).
./DEVLOG.md:60:**2026-07-04 05:59:38Z** - **CODER (w4-3-daemon-disconnect-racing, claude)**: Landed w4-3 through the codex loop (D-2026-07-04-1, second slice fully under it). The push and pull_sync dispatchers bare-awaited their handlers, so a client that disconnected during a send-free compute phase (pull_sync's enumerate+checksum collection is the longest window) left the daemon running the whole remaining handler for a dead peer — while `active_jobs.rs` claimed a `tx.closed()` drop mechanism only `delegated_pull` actually had. Slice `37d7f91`: `resolve_delegated_pull_outcome` generalized to `resolve_transfer_outcome<T>` (same audit-10 biased handler-first select, single owner of the three-way race); new `resolve_streaming_outcome` races a streaming handler against `tx.closed()` + the row's cancel token and maps a race loss onto the ring pair ("client cancelled" / "cancelled via CancelJob", delegated_pull's vocabulary, terminal `Status::cancelled` to a still-connected client); both spawn closures rewired (revert-to-bare-await leaves the helper dead → clippy `-D warnings` catches the unwiring); the false `supports_cancellation` comment rewritten as policy-vs-capability (+ the stale test comment and `CancelOutcome::Unsupported` doc). The audit's "three spawn closures" is two at HEAD — the pull closure died with the Pull RPC at ue-r2-1h. **Deliberately out of scope**: `supports_cancellation` policy unchanged (ratified spec asked for the race + comment fix, not a CancelJob contract change); the token arm is production-unreachable for push/pull_sync until the owner flips the policy — now policy-only, filed as an open question. +5 deterministic unit tests (hangup, CancelJob, bias, error-forward, clean-success; ready/pending futures, no timing), each select arm mutation-verified (M1 drop tx_closed arm → hangup test hangs, cancel test passes; M2 drop cancelled arm → both cancel tests hang, hangup passes; M3 handler-last bias → both bias tests fail). Records `b7382ac`. Review: codex **PASS, zero findings** — it independently traced sink-pipeline drop-propagation ("does not create unbounded orphaned transfer work") and confirmed the policy freeze intentional; verdict recorded, no fix commit. Validation (on the owner's Windows host = parity run): fmt/clippy clean, `cargo test --workspace` all 37 suites green, blit-daemon 162 → 167. Known gaps: in-flight `spawn_blocking` enumeration/checksum batches still run to natural end on drop (the audit's stated follow-up slice); `pull_sync.rs` resize-validation `Vec<JoinHandle>` noted for the w4-1 family ledger. Environment note: `cargo test` under Git Bash fails to link (coreutils `link` shadows MSVC `link.exe`) — use PowerShell. All on master, unpushed. Next: w1-2 (data-socket policy helper) heads the ratified queue.
./REVIEW.md:50:## One transfer path (ONE_TRANSFER_PATH) — code→GPT-review→fix loop
./REVIEW.md:52:Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4).
./REVIEW.md:63:| otp-4a | Daemon serves `Transfer` (runs `run_destination` as Responder; client `run_source`s as SOURCE initiator over a gRPC `FrameTransport`, in-stream carrier). Responder-resolution API (`DestinationTarget` + async `OpenResolver` through `establish`); read-only/unknown-module refusals as `SessionError` frames; A/B byte-identical parity vs old push; unified SizeMtime = safe-skip (⚠ narrow owner-ack, STATE). Codex FAIL (1/1 accepted: cancel must emit a framed `SessionError{CANCELLED}`). | `[x]` | `4b07bbb` + review fix `25f538b` |
./REVIEW.md:81:| w4-5-supports-cancellation-flip | Medium | Flip supports_cancellation for Push/PullSync (owner-authorized D-2026-07-04-3): CancelJob + TUI F2 work on attached transfers; policy-only after w4-3's race wiring (one-predicate flip — Pull history-only stays gated; TUI/CLI needed zero logic changes); contract change exit 2→0 pinned at table + RPC-handler level, authz now covers flipped kinds; every old-policy comment surface updated incl. proto wire-contract doc. Codex NEEDS FIXES (1 Low: module-doc scope log still claimed Pull wired) → fixed `1708075` | `[x]` | master | `05a8b39`+`1708075` |
./REVIEW.md:120:| design-2-orphaned-daemon-data-planes | High | Daemon data-plane tasks detach (not abort) on control-stream death at 3 spawn sites; orphan unreachable by CancelJob. AbortOnDrop fix exists but never propagated. From design map §1.9, hand-verified. Fixed by w4-1 (2 of 3 sites deleted with the Pull RPC at ue-r2-1h; remaining push/control.rs site now wrapped); graded within the w4-1 codex round | `[x]` | master | `65ecb93` |
./REVIEW.md:128:| audit-h3a-push-receive-stall | Robustness | StallGuard on the daemon push-receive socket (`TRANSFER_STALL_TIMEOUT` hoist) — closes one of three remaining stall-guard gaps from R3 H3; symmetric with audit-1c CLI pull-receive | `[x]` | `master` | `dd51a1c` |
./REVIEW.md:185:| audit-9-cancel-auth | Bug | CancelJob now authorizes the caller against the transfer's originating peer (host/IP-only, port-insensitive; loopback + UDS bypass); cross-tenant cancel → PermissionDenied. New CancelOutcome::Unauthorized | `[x]` | `phase5/a1` | `3c5a398` |
./REVIEW.md:186:| audit-10-cancel-completion-race | Bug | DelegatedPull select: order the handler branch first in the biased select (via resolve_delegated_pull_outcome helper) so a completion wins over a simultaneous CancelJob — was mis-recording a success as "cancelled via CancelJob" | `[x]` | `phase5/a1` | `3601f1e` |
./REVIEW.md:222:| M-Jobs     | Feature  | Daemon-owned transfer lifecycle (`CancelJob`, `detach`)  | `phase5/m-jobs` |
./REVIEW.md:237:| audit-9-cancel-auth | Bug | CancelJob RPC lacks peer authorization — any client can cancel any transfer | |
./REVIEW.md:284:- `66df256` CancelJob RPC + `blit jobs cancel` CLI (`m-jobs-2-cancel-rpc`)
crates/blit-tui/src/main.rs:282:    /// an anchored cursor; `Sending` while the CancelJob
crates/blit-tui/src/main.rs:355:        /// press (the cursor may move before `y`). CancelJob targets
crates/blit-tui/src/main.rs:377:        /// prompt creation — each CancelJob targets its own daemon.
crates/blit-tui/src/main.rs:404:        // CancelJobOutcome variant has its own
crates/blit-tui/src/main.rs:407:        outcome: blit_app::admin::jobs::CancelJobOutcome,
crates/blit-tui/src/main.rs:442:/// Reply envelope from the spawned CancelJob task.
crates/blit-tui/src/main.rs:446:    result: Result<blit_app::admin::jobs::CancelJobOutcome, String>,
crates/blit-tui/src/main.rs:1855:                    // m2f-8: batch cancel sends each CancelJob to the
crates/blit-tui/src/main.rs:2811:    // on `y`, fire CancelJob with the old daemon's transfer
crates/blit-tui/src/main.rs:3839:    /// (CancelJob targets that daemon).
crates/blit-tui/src/main.rs:3846:/// back into a connectable endpoint for CancelJob. The identity has no
crates/blit-tui/src/main.rs:3848:/// CancelJob — so `RemoteEndpoint::parse` of `host` / `host:port`
crates/blit-tui/src/main.rs:3860:/// batch cancel sends each `CancelJob` to the daemon that owns the
crates/blit-tui/src/main.rs:3871:/// d-30 / d-30 R2: spawn one CancelJob RPC per id.
crates/blit-tui/src/main.rs:3883:/// m2f-8: spawn one CancelJob per `(daemon, id)` target, each against
crates/blit-tui/src/main.rs:3904:/// d-22: spawn a CancelJob RPC against `endpoint` for
crates/blit-tui/src/main.rs:5072:    /// active transfer via the daemon's CancelJob RPC.
crates/blit-tui/src/main.rs:6561:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
crates/blit-tui/src/main.rs:6962:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
crates/blit-tui/src/main.rs:6982:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
crates/blit-tui/src/main.rs:7016:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
crates/blit-tui/src/main.rs:7036:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
crates/blit-tui/src/main.rs:7091:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
crates/blit-tui/src/main.rs:7123:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
crates/blit-tui/src/main.rs:7140:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
crates/blit-tui/src/main.rs:8189:    /// connectable cancel endpoint — host:port preserved (so CancelJob
crates/blit-tui/src/state.rs:382:    /// Callers MUST check `Some` before firing CancelJob;
crates/blit-tui/src/state.rs:388:        // bare transfer_id (CancelJob targets it). Look the row up and
crates/blit-tui/src/state.rs:393:    /// m2f-7: the source daemon of the cursor's transfer — CancelJob
crates/blit-tui/src/state.rs:792:    /// m2f-7: the cursor exposes its row's source daemon (CancelJob's
crates/blit-tui/src/display_f2.rs:21:    use blit_app::admin::jobs::CancelJobOutcome;
crates/blit-tui/src/display_f2.rs:53:                CancelJobOutcome::Cancelled { transfer_id: id } => F2CancelDisplay::Cancelled {
crates/blit-tui/src/display_f2.rs:56:                CancelJobOutcome::NotFound { transfer_id: id } => F2CancelDisplay::NotFound {
crates/blit-tui/src/display_f2.rs:59:                CancelJobOutcome::Unsupported {
crates/blit-tui/src/config.rs:501:    /// on screen after a CancelJob reply lands. Sending
crates/blit-tui/src/config.rs:509:    /// CancelJob RPC immediately. `y` confirms, `n` or
crates/blit-app/src/admin/jobs.rs:10:    CancelJobRequest, ClearRecentRequest, DaemonEvent, DaemonState, GetStateRequest,
crates/blit-app/src/admin/jobs.rs:34:/// Outcome of a `CancelJob` RPC. The wire surface encodes
crates/blit-app/src/admin/jobs.rs:39:pub enum CancelJobOutcome {
crates/blit-app/src/admin/jobs.rs:46:    /// `CancelJob` off. Since D-2026-07-04-3 flipped push and
crates/blit-app/src/admin/jobs.rs:57:/// Issue the `CancelJob` RPC against `remote`. Errors only on
crates/blit-app/src/admin/jobs.rs:60:/// Ok) get mapped onto [`CancelJobOutcome`] for the caller to
crates/blit-app/src/admin/jobs.rs:62:pub async fn cancel(remote: &RemoteEndpoint, transfer_id: &str) -> Result<CancelJobOutcome> {
crates/blit-app/src/admin/jobs.rs:67:        .cancel_job(CancelJobRequest {
crates/blit-app/src/admin/jobs.rs:84:            Ok(CancelJobOutcome::Cancelled { transfer_id: id })
crates/blit-app/src/admin/jobs.rs:87:            Code::NotFound => Ok(CancelJobOutcome::NotFound {
crates/blit-app/src/admin/jobs.rs:90:            Code::FailedPrecondition => Ok(CancelJobOutcome::Unsupported {
crates/blit-app/src/admin/jobs.rs:95:                "CancelJob failed ({}): {}",
crates/blit-daemon/Cargo.toml:36:# the production 30 s TRANSFER_STALL_TIMEOUT without wall-clock waits.
crates/blit-daemon/src/active_jobs.rs:42://!   forthcoming `CancelJob` RPC can drop in-flight
crates/blit-daemon/src/active_jobs.rs:62://! - `CancelJob` RPC + CLI verb (`m-jobs-2-cancel-rpc`).
crates/blit-daemon/src/active_jobs.rs:159:    /// Whether `CancelJob` dispatch fires this kind's cancellation
crates/blit-daemon/src/active_jobs.rs:180:/// `CancelJob` RPC handler will map each variant onto a
crates/blit-daemon/src/active_jobs.rs:185:/// - `Unsupported` → `Code::FailedPrecondition` — CancelJob
crates/blit-daemon/src/active_jobs.rs:460:    /// the upcoming `CancelJob` RPC needs to map onto gRPC
crates/blit-daemon/src/active_jobs.rs:917:    /// `TransferStarted.transfer_id`, M-Jobs `CancelJob`) can
crates/blit-daemon/src/active_jobs.rs:983:    /// (via the CancelJob RPC in m-jobs-2).
crates/blit-daemon/src/active_jobs.rs:1839:        // D-2026-07-04-3 flipped CancelJob dispatch on for the
crates/blit-daemon/src/service/transfer_session_e2e.rs:1://! ONE_TRANSFER_PATH otp-4a/4b loopback e2e: the daemon serves the
crates/blit-daemon/src/service/transfer_session_e2e.rs:35:use blit_core::transfer_session::SessionFault;
crates/blit-daemon/src/service/transfer_session_e2e.rs:193:fn fault_of(err: &eyre::Report) -> &SessionFault {
crates/blit-daemon/src/service/transfer_session_e2e.rs:194:    err.downcast_ref::<SessionFault>()
crates/blit-daemon/src/service/transfer_session_e2e.rs:195:        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
crates/blit-daemon/src/service/transfer_session_e2e.rs:198:// --- otp-4b-3: deterministic mid-transfer cancel over the data plane ---
crates/blit-daemon/src/service/transfer_session_e2e.rs:271:/// otp-4b-3: fire a `CancelJob`-equivalent (the row's cancellation token,
crates/blit-daemon/src/service/transfer_session_e2e.rs:274:/// `SessionFault{CANCELLED}` — the peer's framed abort reason — rather
crates/blit-daemon/src/service/transfer_session_e2e.rs:304:    // Fire the row's cancellation token — exactly what the `CancelJob` RPC
crates/blit-daemon/src/service/transfer_session_e2e.rs:321:    // The client must surface CANCELLED promptly (no hang).
crates/blit-daemon/src/service/transfer_session_e2e.rs:330:        "the client surfaces the peer's framed CANCELLED, not the data-plane break: {err:#}"
crates/blit-daemon/src/service/core.rs:17:    daemon_event, ActiveTransfer, CancelJobRequest, CancelJobResponse, ClearRecentRequest,
crates/blit-daemon/src/service/core.rs:353:    /// ONE_TRANSFER_PATH otp-4a: the daemon serves the unified session
crates/blit-daemon/src/service/core.rs:377:        // the row still supports CancelJob and appears in GetState, and
crates/blit-daemon/src/service/core.rs:395:            // SessionError{CANCELLED}, not a bare Status (codex F1).
crates/blit-daemon/src/service/core.rs:721:        // failure, or `CancelJob(transfer_id)` regardless of
crates/blit-daemon/src/service/core.rs:773:            //   cancel_token.cancelled() → `CancelJob` RPC fired the
crates/blit-daemon/src/service/core.rs:780:            //   None         → cancelled (client OR CancelJob)
crates/blit-daemon/src/service/core.rs:790:            // a hangup / `CancelJob`. See that helper for the rationale.
crates/blit-daemon/src/service/core.rs:818:            //   None        → client hangup or CancelJob.
crates/blit-daemon/src/service/core.rs:822:            //                  CancelJob; otherwise it was the
crates/blit-daemon/src/service/core.rs:828:                    (false, Some("cancelled via CancelJob".to_string()))
crates/blit-daemon/src/service/core.rs:1118:        request: Request<CancelJobRequest>,
crates/blit-daemon/src/service/core.rs:1119:    ) -> Result<Response<CancelJobResponse>, Status> {
crates/blit-daemon/src/service/core.rs:1127:                "CancelJobRequest.transfer_id must not be empty",
crates/blit-daemon/src/service/core.rs:1135:            CancelOutcome::Cancelled => Ok(Response::new(CancelJobResponse {
crates/blit-daemon/src/service/core.rs:1320:/// `CancelJob` cancel, both of which resolve to `None` so the caller
crates/blit-daemon/src/service/core.rs:1324:/// transfer that completed at the same instant `CancelJob` fired its
crates/blit-daemon/src/service/core.rs:1325:/// token was mis-recorded as "cancelled via CancelJob" despite having
crates/blit-daemon/src/service/core.rs:1352:/// row's `CancelJob` token via [`resolve_transfer_outcome`].
crates/blit-daemon/src/service/core.rs:1358:/// unobservable work that `CancelJob` also refused to touch
crates/blit-daemon/src/service/core.rs:1378:/// - cancel token fired → `(false, "cancelled via CancelJob")`, and the
crates/blit-daemon/src/service/core.rs:1405:        // token means the cause was CancelJob; otherwise the client
crates/blit-daemon/src/service/core.rs:1409:                .send(Err(Status::cancelled("transfer cancelled via CancelJob")))
crates/blit-daemon/src/service/core.rs:1411:            (false, Some("cancelled via CancelJob".to_string()))
crates/blit-daemon/src/service/core.rs:1419:/// `CancelJob` it emits a framed `SessionError{CANCELLED}` on the
crates/blit-daemon/src/service/core.rs:1450:                    "transfer cancelled via CancelJob",
crates/blit-daemon/src/service/core.rs:1453:            (false, Some("cancelled via CancelJob".to_string()))
crates/blit-daemon/src/service/core.rs:1485:    /// instant `CancelJob` fired gets mis-recorded as cancelled.
crates/blit-daemon/src/service/core.rs:1503:    /// `CancelJob` cancel — the fix must not make transfers
crates/blit-daemon/src/service/core.rs:1511:            ready(()),         // CancelJob fired
crates/blit-daemon/src/service/core.rs:1518:    /// otp-4a codex F1: a `CancelJob` on a served `Transfer` session
crates/blit-daemon/src/service/core.rs:1519:    /// must reach the client as a framed `SessionError{CANCELLED}` on
crates/blit-daemon/src/service/core.rs:1537:        assert_eq!(msg.as_deref(), Some("cancelled via CancelJob"));
crates/blit-daemon/src/service/core.rs:1548:                "cancel must emit a framed CANCELLED SessionError"
crates/blit-daemon/src/service/core.rs:1550:            other => panic!("expected a CANCELLED error frame, got {other:?}"),
crates/blit-daemon/src/service/core.rs:1597:    /// handler as `(false, "cancelled via CancelJob")` and deliver a
crates/blit-daemon/src/service/core.rs:1611:        assert_eq!(err.as_deref(), Some("cancelled via CancelJob"));
crates/blit-daemon/src/service/core.rs:1820:            .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/core.rs:1835:        // D-2026-07-04-3: CancelJob dispatch fires the row token for
crates/blit-daemon/src/service/core.rs:1847:                .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/core.rs:1855:                "{}: CancelJob must fire the row token",
crates/blit-daemon/src/service/core.rs:1878:            .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/core.rs:1882:            .expect_err("a policy-gated kind must reject CancelJob");
crates/blit-daemon/src/service/core.rs:1886:            "token must NOT be fired when CancelJob is unsupported"
crates/blit-daemon/src/service/core.rs:1894:            .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/core.rs:1906:            .cancel_job(Request::new(CancelJobRequest {
crates/blit-daemon/src/service/transfer.rs:1://! ONE_TRANSFER_PATH unified `Transfer` session — daemon side.
crates/blit-daemon/src/service/transfer.rs:35:    ResolvedEndpoint, SessionEndpoint, SessionFault,
crates/blit-daemon/src/service/transfer.rs:46:fn status_to_fault(status: Status) -> SessionFault {
crates/blit-daemon/src/service/transfer.rs:52:    SessionFault::refusal(code, status.message().to_string())
crates/blit-daemon/src/service/transfer.rs:118:                .downcast_ref::<SessionFault>()
crates/blit-daemon/src/service/push/data_plane.rs:9:use blit_core::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
crates/blit-daemon/src/service/push/data_plane.rs:196:    // TimedOut after TRANSFER_STALL_TIMEOUT of no progress.
crates/blit-daemon/src/service/push/data_plane.rs:1074:/// by `TRANSFER_STALL_TIMEOUT` rather than holding the receive worker
crates/blit-daemon/src/service/push/data_plane.rs:1085:    let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
crates/blit-daemon/src/service/push/data_plane.rs:1119:    /// with the production `TRANSFER_STALL_TIMEOUT` — so a future
crates/blit-daemon/src/service/push/data_plane.rs:1149:        tokio::time::advance(TRANSFER_STALL_TIMEOUT + Duration::from_secs(1)).await;
crates/blit-daemon/src/service/push/control.rs:815:    //! running with no owner — unreachable by `CancelJob`. This pins
crates/blit-core/tests/transfer_session_roles.rs:8://! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1) in its first
crates/blit-core/tests/transfer_session_roles.rs:28:    HelloConfig, SessionEndpoint, SessionFault, SourceSessionConfig, CONTRACT_VERSION,
crates/blit-core/tests/transfer_session_roles.rs:199:fn fault_of(err: &eyre::Report) -> &SessionFault {
crates/blit-core/tests/transfer_session_roles.rs:200:    err.downcast_ref::<SessionFault>()
crates/blit-core/tests/transfer_session_roles.rs:201:        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
crates/blit-core/tests/pull_sync_with_spec_wire.rs:180:        _: Request<blit_core::generated::CancelJobRequest>,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:181:    ) -> Result<Response<blit_core::generated::CancelJobResponse>, Status> {
crates/blit-core/tests/pull_sync_with_spec_wire.rs:608:        _: Request<blit_core::generated::CancelJobRequest>,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:609:    ) -> Result<Response<blit_core::generated::CancelJobResponse>, Status> {
./proto/blit.proto:7:  // (Deleted at ONE_TRANSFER_PATH cutover, otp-10 — replaced by Transfer.)
./proto/blit.proto:10:  // ONE_TRANSFER_PATH (otp-1): the single role-tagged transfer session
./proto/blit.proto:76:  // client receives a terminal CANCELLED status.
./proto/blit.proto:92:  rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);
./proto/blit.proto:845:  // failure, or `CancelJob(transfer_id)`. The CLI is free to
./proto/blit.proto:1043:// CancelJob — fire the cancellation token of an active transfer.
./proto/blit.proto:1047:message CancelJobRequest {
./proto/blit.proto:1051:message CancelJobResponse {
./proto/blit.proto:1217:// ONE_TRANSFER_PATH unified session (otp-1 wire contract).
./proto/blit.proto:1341:    CANCELLED = 8;
./TODO.md:310:**Decisions taken** (`TUI_DESIGN.md` §10): separate `blit-app` library + `blit-tui` binary; local-only TUI mode first-class with "local" as a sentinel endpoint in F1; foundation-first milestone order; cancellation via server-side `CancelJob`; `--detach` CLI flag ships with M-Jobs; `AppProgressEvent` is channel-based.
./docs/STATE.md:5:sockets; ONE_TRANSFER_PATH otp-1 + otp-3 + otp-4a + otp-4b-1 + otp-4b-2
./docs/STATE.md:6:`[x]`, current slice otp-4b-3 (mid-transfer cancel e2e).
./docs/STATE.md:17:- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
./docs/STATE.md:51:    Suite → **1513/0**.
./docs/STATE.md:52:  - Current: **otp-4b-3** (deterministic mid-transfer cancel e2e), then
./docs/STATE.md:57:  blocked** until ONE_TRANSFER_PATH ships, then resume/re-derive on
./docs/STATE.md:70:1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
./docs/STATE.md:73:   otp-4b-1, otp-4b-2 `[x]`. Current: **otp-4b-3** (deterministic
./docs/STATE.md:74:   mid-transfer cancel e2e — fire `CancelJob` while bytes flow over the
./docs/STATE.md:75:   data plane; assert client surfaces `SessionFault{CANCELLED}` +
./docs/STATE.md:87:   resumes/re-derives after ONE_TRANSFER_PATH ships.
./docs/STATE.md:103:   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
./docs/STATE.md:110:- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
./docs/STATE.md:190:  commit.** **Codex PASS, no findings.** Suite 1512 → **1513/0**.
./docs/STATE.md:191:  In-flight: none. **Exact first action next session**: otp-4b-3
./docs/TRANSFER_SESSION.md:5:**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
./docs/TRANSFER_SESSION.md:11:truth lives in `proto/blit.proto` under "ONE_TRANSFER_PATH unified
./docs/TRANSFER_SESSION.md:216:  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
./docs/TRANSFER_SESSION.md:219:- `CancelJob` interop: the responder registers the session in
./docs/TRANSFER_SESSION.md:222:  peer receives `SessionError{CANCELLED}`.
./docs/DECISIONS.md:119:## D-2026-07-04-3 — Flip `supports_cancellation` for Push/PullSync: CancelJob works on attached transfers
./docs/DECISIONS.md:120:- Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
./docs/DECISIONS.md:130:- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
./docs/DECISIONS.md:132:- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.
./docs/DECISIONS.md:135:- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
./docs/DECISIONS.md:136:- Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
./docs/DECISIONS.md:137:- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).
./docs/DECISIONS.md:140:- Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
./docs/DECISIONS.md:144:## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
./docs/DECISIONS.md:145:- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
crates/blit-core/src/transfer_session/mod.rs:2://! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
crates/blit-core/src/transfer_session/mod.rs:43:use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
crates/blit-core/src/transfer_session/mod.rs:140:pub struct SessionFault {
crates/blit-core/src/transfer_session/mod.rs:153:impl SessionFault {
crates/blit-core/src/transfer_session/mod.rs:206:impl fmt::Display for SessionFault {
crates/blit-core/src/transfer_session/mod.rs:212:impl std::error::Error for SessionFault {}
crates/blit-core/src/transfer_session/mod.rs:217:fn fault_from_report(report: eyre::Report) -> SessionFault {
crates/blit-core/src/transfer_session/mod.rs:218:    match report.downcast::<SessionFault>() {
crates/blit-core/src/transfer_session/mod.rs:220:        Err(other) => SessionFault::internal(format!("{other:#}")),
crates/blit-core/src/transfer_session/mod.rs:228:fn error_frame(fault: &SessionFault) -> TransferFrame {
crates/blit-core/src/transfer_session/mod.rs:269:/// so the daemon dispatcher can emit `CANCELLED` when a `CancelJob`
crates/blit-core/src/transfer_session/mod.rs:286:type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;
crates/blit-core/src/transfer_session/mod.rs:305:/// `tonic::Status` errors to [`SessionFault`], so blit-core stays free
crates/blit-core/src/transfer_session/mod.rs:312:        -> Pin<Box<dyn Future<Output = std::result::Result<ResolvedEndpoint, SessionFault>> + Send>>
crates/blit-core/src/transfer_session/mod.rs:328:fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
crates/blit-core/src/transfer_session/mod.rs:330:        return Err(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:339:        return Err(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:346:fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
crates/blit-core/src/transfer_session/mod.rs:348:        return Err(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:353:        return Err(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:405:                SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs:417:        let fault = SessionFault {
crates/blit-core/src/transfer_session/mod.rs:440:                        SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs:461:                        SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs:476:                    SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs:504:                                SessionFault::read_only(
crates/blit-core/src/transfer_session/mod.rs:557:        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
crates/blit-core/src/transfer_session/mod.rs:560:            SessionFault::protocol_violation("frame with empty oneof"),
crates/blit-core/src/transfer_session/mod.rs:562:        None => Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:570:async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
crates/blit-core/src/transfer_session/mod.rs:592:    Fault(SessionFault),
crates/blit-core/src/transfer_session/mod.rs:646:    match source_send_half(
crates/blit-core/src/transfer_session/mod.rs:683:                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:689:                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
crates/blit-core/src/transfer_session/mod.rs:699:                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:717:                                SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs:733:                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:751:                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
crates/blit-core/src/transfer_session/mod.rs:755:                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:764:async fn source_send_half(
crates/blit-core/src/transfer_session/mod.rs:786:                eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:915:                return Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:939:    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
crates/blit-core/src/transfer_session/mod.rs:940:    // the served session frames `SessionError{CANCELLED}`, and the source
crates/blit-core/src/transfer_session/mod.rs:945:    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
crates/blit-core/src/transfer_session/mod.rs:950:    //     stall window (`prefer_peer_fault`).
crates/blit-core/src/transfer_session/mod.rs:954:            fault = recv_peer_fault(&mut events) => {
crates/blit-core/src/transfer_session/mod.rs:959:                    return Err(prefer_peer_fault(&mut events, dp_err).await);
crates/blit-core/src/transfer_session/mod.rs:972:        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:976:            SessionFault::protocol_violation("duplicate NeedComplete"),
crates/blit-core/src/transfer_session/mod.rs:979:            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
crates/blit-core/src/transfer_session/mod.rs:981:        None => Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:1034:                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1045:                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1054:                eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1079:        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1141:                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1146:                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1151:                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/mod.rs:1156:                return Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:1166:/// data-plane drain (otp-4b-3): a mid-transfer `SessionError` (e.g. a
crates/blit-core/src/transfer_session/mod.rs:1167:/// `CancelJob` → `CANCELLED`) must abort the send and surface as the
crates/blit-core/src/transfer_session/mod.rs:1172:async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
crates/blit-core/src/transfer_session/mod.rs:1183:/// *symptom* of a peer abort — within `TRANSFER_STALL_TIMEOUT` the peer
crates/blit-core/src/transfer_session/mod.rs:1187:async fn prefer_peer_fault(
crates/blit-core/src/transfer_session/mod.rs:1191:    match tokio::time::timeout(TRANSFER_STALL_TIMEOUT, recv_peer_fault(events)).await {
crates/blit-core/src/transfer_session/mod.rs:1347:                return Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:1368:    eyre::Report::new(SessionFault::protocol_violation(message))
crates/blit-core/src/transfer_session/mod.rs:1453:                return Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:1650:                return Err(eyre::Report::new(SessionFault::from_wire(err)));
crates/blit-core/src/transfer_session/mod.rs:1753:        SessionFault::protocol_violation(format!(
crates/blit-core/src/transfer_session/mod.rs:1803:                    return Err(eyre::Report::new(SessionFault::internal(format!(
crates/blit-core/src/transfer_session/mod.rs:1862:                return Err(eyre::Report::new(SessionFault::internal(
crates/blit-core/src/transfer_session/mod.rs:1914:    /// otp-4b-3: a data-plane break during the drain prefers the peer's
crates/blit-core/src/transfer_session/mod.rs:1916:    /// `SessionError{CANCELLED}` on the control lane, `prefer_peer_fault`
crates/blit-core/src/transfer_session/mod.rs:1921:    async fn prefer_peer_fault_prefers_a_framed_fault() {
crates/blit-core/src/transfer_session/mod.rs:1923:        // The peer framed CANCELLED on the control lane before we ask.
crates/blit-core/src/transfer_session/mod.rs:1924:        tx.send(SourceEvent::Fault(SessionFault {
crates/blit-core/src/transfer_session/mod.rs:1926:            message: "transfer cancelled via CancelJob".into(),
crates/blit-core/src/transfer_session/mod.rs:1933:        let dp_err = eyre::Report::new(SessionFault::refusal(
crates/blit-core/src/transfer_session/mod.rs:1937:        let chosen = prefer_peer_fault(&mut rx, dp_err).await;
crates/blit-core/src/transfer_session/mod.rs:1939:            .downcast_ref::<SessionFault>()
crates/blit-core/src/transfer_session/mod.rs:1940:            .expect("a SessionFault");
crates/blit-core/src/transfer_session/mod.rs:1944:            "the framed CANCELLED must win over the data-plane break"
crates/blit-core/src/transfer_session/mod.rs:1950:        let fault = SessionFault {
crates/blit-core/src/transfer_session/mod.rs:1958:        let back = SessionFault::from_wire(wire);
./docs/ARCHITECTURE.md:130:| `admin` | Admin-verb implementations (`ls`, `find`, `du`, `df`, `rm`, `jobs`, `list_modules`) — `jobs` is what the TUI and the Prometheus bridge call for `GetState`/`Subscribe`/`CancelJob`/`ClearRecent` |
./docs/ARCHITECTURE.md:155:`CancelJob` / `ClearRecent`, and supports configurable keybindings
./docs/ARCHITECTURE.md:371:  rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);
./docs/ARCHITECTURE.md:378:state; `CancelJob` cancels an in-flight transfer (authorized to the
crates/blit-core/src/transfer_session/data_plane.rs:8://! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
crates/blit-core/src/transfer_session/data_plane.rs:49:use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
crates/blit-core/src/transfer_session/data_plane.rs:55:use super::SessionFault;
crates/blit-core/src/transfer_session/data_plane.rs:65:    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
crates/blit-core/src/transfer_session/data_plane.rs:318:        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
crates/blit-core/src/transfer_session/data_plane.rs:612:            eyre::Report::new(SessionFault::internal("data plane already finished"))
crates/blit-core/src/transfer_session/data_plane.rs:676:            Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/data_plane.rs:700:                return Err(eyre::Report::new(SessionFault::protocol_violation(
crates/blit-core/src/transfer_session/data_plane.rs:773:        // Off-need-list path faults with a SessionFault.
crates/blit-core/src/transfer_session/data_plane.rs:779:            err.downcast_ref::<SessionFault>().is_some(),
crates/blit-core/src/transfer_session/data_plane.rs:780:            "off-list rejection is a SessionFault: {err:#}"
./docs/bench/10gbe-2026-07-05/tool_comparison.csv:27:rsyncd,push,small,2,1513,0
./docs/API.md:29:> `GetState`, `CancelJob`, `ClearRecent`, and `Subscribe` are live RPCs
./crates/blit-tui/src/main.rs:282:    /// an anchored cursor; `Sending` while the CancelJob
./crates/blit-tui/src/main.rs:355:        /// press (the cursor may move before `y`). CancelJob targets
./crates/blit-tui/src/main.rs:377:        /// prompt creation — each CancelJob targets its own daemon.
./crates/blit-tui/src/main.rs:404:        // CancelJobOutcome variant has its own
./crates/blit-tui/src/main.rs:407:        outcome: blit_app::admin::jobs::CancelJobOutcome,
./crates/blit-tui/src/main.rs:442:/// Reply envelope from the spawned CancelJob task.
./crates/blit-tui/src/main.rs:446:    result: Result<blit_app::admin::jobs::CancelJobOutcome, String>,
./crates/blit-tui/src/main.rs:1855:                    // m2f-8: batch cancel sends each CancelJob to the
./crates/blit-tui/src/main.rs:2811:    // on `y`, fire CancelJob with the old daemon's transfer
./crates/blit-tui/src/main.rs:3839:    /// (CancelJob targets that daemon).
./crates/blit-tui/src/main.rs:3846:/// back into a connectable endpoint for CancelJob. The identity has no
./crates/blit-tui/src/main.rs:3848:/// CancelJob — so `RemoteEndpoint::parse` of `host` / `host:port`
./crates/blit-tui/src/main.rs:3860:/// batch cancel sends each `CancelJob` to the daemon that owns the
./crates/blit-tui/src/main.rs:3871:/// d-30 / d-30 R2: spawn one CancelJob RPC per id.
./crates/blit-tui/src/main.rs:3883:/// m2f-8: spawn one CancelJob per `(daemon, id)` target, each against
./crates/blit-tui/src/main.rs:3904:/// d-22: spawn a CancelJob RPC against `endpoint` for
./crates/blit-tui/src/main.rs:5072:    /// active transfer via the daemon's CancelJob RPC.
./crates/blit-tui/src/main.rs:6561:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
./crates/blit-tui/src/main.rs:6962:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
./crates/blit-tui/src/main.rs:6982:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
./crates/blit-tui/src/main.rs:7016:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
./crates/blit-tui/src/main.rs:7036:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
./crates/blit-tui/src/main.rs:7091:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
./crates/blit-tui/src/main.rs:7123:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
./crates/blit-tui/src/main.rs:7140:            outcome: blit_app::admin::jobs::CancelJobOutcome::Cancelled {
./crates/blit-tui/src/main.rs:8189:    /// connectable cancel endpoint — host:port preserved (so CancelJob
./docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:169:      are deleted with the unified session, ONE_TRANSFER_PATH otp-10)*.
./docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:363:  unified session, ONE_TRANSFER_PATH otp-10)*
./docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:419:  unified session, ONE_TRANSFER_PATH otp-10.)*
./docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:453:   tests are deleted with the unified session, ONE_TRANSFER_PATH
./docs/plan/ZERO_COPY_RECEIVE_EVAL.md:81:lands as a runtime-selected write strategy inside ONE_TRANSFER_PATH's
./docs/plan/ZERO_COPY_RECEIVE_EVAL.md:100:  justification; implementation is sequenced after ONE_TRANSFER_PATH
./docs/plan/ZERO_COPY_RECEIVE_EVAL.md:109:      through ONE_TRANSFER_PATH's unified sink instead of reviving the
./crates/blit-tui/src/state.rs:382:    /// Callers MUST check `Some` before firing CancelJob;
./crates/blit-tui/src/state.rs:388:        // bare transfer_id (CancelJob targets it). Look the row up and
./crates/blit-tui/src/state.rs:393:    /// m2f-7: the source daemon of the cursor's transfer — CancelJob
./crates/blit-tui/src/state.rs:792:    /// m2f-7: the cursor exposes its row's source daemon (CancelJob's
./docs/plan/TUI_DESIGN.md:90:| Job lifecycle (cancel + daemon-owned transfers) | gRPC `CancelJob` + `detach` field on transfer specs | **new** — see §6.5 | ⏳ Not yet on the wire |
./docs/plan/TUI_DESIGN.md:96:- `CancelJob` (cancel without holding the transfer's stream)
./docs/plan/TUI_DESIGN.md:202:  `CancelJob(transfer_id)` RPC (§6.5). Cancellation works on
./docs/plan/TUI_DESIGN.md:495:a `CancellationToken` so `CancelJob` can fire it; C (§6.2)
./docs/plan/TUI_DESIGN.md:524:### 6.5 `CancelJob` + `detach` — daemon-owned transfer lifecycle
./docs/plan/TUI_DESIGN.md:587:   `CancelJob` fires. Same per-job allocation, just one more
./docs/plan/TUI_DESIGN.md:595:   completion or `CancelJob(id)`. Push / pull / pull_sync sites
./docs/plan/TUI_DESIGN.md:599:4. **`CancelJob` RPC.**
./docs/plan/TUI_DESIGN.md:604:     rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);
./docs/plan/TUI_DESIGN.md:607:   message CancelJobRequest {
./docs/plan/TUI_DESIGN.md:611:   message CancelJobResponse {
./docs/plan/TUI_DESIGN.md:674:- `blit jobs cancel <remote> <transfer_id>` — calls `CancelJob`.
./docs/plan/TUI_DESIGN.md:836:  daemon, cancellable via `CancelJob`. Same table, more on
./docs/plan/TUI_DESIGN.md:846:the table is what `CancelJob` cancels against and what
./docs/plan/TUI_DESIGN.md:914:  `CancelJob(id)`
./docs/plan/TUI_DESIGN.md:915:- `CancelJob(transfer_id)` RPC
./docs/plan/TUI_DESIGN.md:953:  the network. Cancel hotkey fires `CancelJob`.
./docs/plan/TUI_DESIGN.md:1002:- **Cancellation: server-side via `CancelJob`.** TUI's cancel
./docs/plan/TUI_DESIGN.md:1003:  hotkey fires `CancelJob(transfer_id)` (M-Jobs, §6.5). Works
./docs/plan/TUI_DESIGN.md:1068:| 3 | M-Jobs — daemon-owned lifecycle (delegated-only) + `CancelJob` + `detach` | +`CancelJob`, +`detach` on `DelegatedPullRequest`, +`transfer_id` on `DelegatedPullStarted` | ~500 daemon + ~200 CLI (`--detach` on remote→remote only, `jobs cancel/watch` polling) | ✅ Remote→remote transfers detachable; CLI gains cancel + polling watch; daemon ready for TUI |
./docs/plan/TUI_DESIGN.md:1103:- Three new RPCs (`GetState`, `Subscribe`, `CancelJob`) plus
./docs/plan/TUI_DESIGN.md:1108:  `CancelJob` / `detach`; C lands `Subscribe` and
./docs/audit/DESIGN_MAP_2026-06-11.md:65:   to TCP error — unreachable by CancelJob. `blitd` itself has no graceful
./docs/audit/DESIGN_MAP_2026-06-11.md:181:- `TRANSFER_STALL_TIMEOUT` = 30s (crates/blit-core/src/remote/transfer/stall_guard.rs:69) — none — idle-progress guard, re-armed on progress; shared by push writes and pull reads.
./docs/audit/DESIGN_MAP_2026-06-11.md:223:No single module owns timeouts & liveness. The closest thing to an owner is crates/blit-core/src/remote/transfer/stall_guard.rs (StallGuard/StallGuardWriter + the owner-decided 30s TRANSFER_STALL_TIMEOUT), but it deliberately covers only raw-TCP data-plane progress, and at least four other strata each grew their own local policy: blit-daemon's net_timeout::within helper, blit-app's client.rs CONNECT_TIMEOUT, the TUI's SUBSCRIBE_OPEN/SNAPSHOT_FETCH constants, and the prometheus bridge's 5/8/10s set — while the two blit-core channel builders carry bare inline 30s literals. The smear is asymmetric in a dangerous way: connects and daemon-side accepts are well bounded (30s/15s, declared four separate times), but once connected, every client-side gRPC await in the workspace is structurally unbounded — no client channel sets HTTP/2 keepalive, TCP keepalive, or a per-RPC deadline, so delegation progress (remote.rs:734), push responses (helpers.rs:245), all three pull fallback loops (pull.rs:330/505/790), and `blit jobs watch` by default will wait forever on a silently-dead daemon; only the daemon server protects itself (main.rs:138). The sharpest single problem is that the planned fix — the audit-h3c slice-2 progress watchdog — exists only as comments plus a pass-through helper (grpc_fallback.rs:150), and its own TODOs record that the surrounding error-conversion sites already strip the error chain the watchdog would need for retries to work. Secondary findings: raw TCP data-plane connects are unbounded (pull.rs:1710, data_plane.rs:92) while control-plane connects are triple-bounded; pull_sync.rs redefines its own accept/token constants twice in one file under a comment claiming reuse; and the two set_keepalive(true) calls run at OS-default ~2h timing while comments claim they prevent idle-stream timeouts.
./docs/audit/DESIGN_MAP_2026-06-11.md:253:  - sites: stall_guard.rs:69 (TRANSFER_STALL_TIMEOUT); pull.rs:241-242 / push/client/mod.rs:309-310 / blit-app/client.rs:24 (connect); 4× accept timeouts (daemon); delegated_pull.rs:35 (SOURCE_CONNECT_TIMEOUT); blit-daemon/src/main.rs:138 (h2 keepalive interval); tar_stream.rs:38 (send_timeout_ms 30_000); blit-tui/src/main.rs:2426,5639 (SNAPSHOT_FETCH/SUBSCRIBE_OPEN)
./docs/audit/DESIGN_MAP_2026-06-11.md:254:  - divergence: Roughly a dozen sites independently chose 30 seconds with no shared definition; only TRANSFER_STALL_TIMEOUT is documented as an owner decision. The bridge crate alone chose different numbers (5/8/10s), justified by Prometheus's scrape_timeout — the only constants in the workspace whose values are derived from an external constraint rather than vibes.
./docs/audit/DESIGN_MAP_2026-06-11.md:258:- `TRANSFER_STALL_TIMEOUT` = 30s (idle, re-armed on progress) (crates/blit-core/src/remote/transfer/stall_guard.rs:69) — RELIABLE: good — converts infinite stalls into clean TimedOut with explanatory text. FAST/SIMPLE: grpc_fallback.rs:73-80 itself argues hardcoded wall-clock constants violate the adapt-at-runtime principle; the idle (not total) semantics mostly defuse it.
./docs/audit/DESIGN_MAP_2026-06-11.md:277:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69 — Partial owner: TRANSFER_STALL_TIMEOUT (30s) + StallGuard (AsyncRead, :75) + StallGuardWriter (AsyncWrite, :139). Idle timeout re-armed on progress, owner-decided 30s. Module doc (:35-41) records that gRPC-fallback coverage (audit-h3c slice 2) is PENDING. — The only deliberately designed liveness mechanism in the repo; covers TCP data-plane paths only.
./docs/audit/DESIGN_MAP_2026-06-11.md:285:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:1764 — StallGuard wrap on CLI pull-receive TCP socket (audit-1c) with TRANSFER_STALL_TIMEOUT — the good path.
./docs/audit/DESIGN_MAP_2026-06-11.md:461:  - divergence: Not a copy-paste duplicate — single shared TRANSFER_STALL_TIMEOUT constant and shared adapters (good ownership). The divergence is coverage, not configuration: every TCP byte path is guarded, the gRPC fallback paths have NO stall detection at all (audit-h3c slice 2 unshipped), and the daemon accept/token phases use separate ad-hoc constants (daemon pull.rs:698-699).
./docs/audit/DESIGN_MAP_2026-06-11.md:465:- `TRANSFER_STALL_TIMEOUT` = Duration::from_secs(30) (/home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69) — SIMPLE: zero-tuning, owner-decided, idle-not-total semantics — good. But grpc_fallback.rs:73-80 itself declares hardcoded wall-clock constants a violation of the FAST principle and promises a progress-cadence-derived policy for slice 2; the 30s figure is exactly such a magic number, just a defensible one.
./docs/audit/DESIGN_MAP_2026-06-11.md:471:- `PULL_ACCEPT_TIMEOUT / PULL_TOKEN_TIMEOUT` = 30s / 15s (/home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:698) — Function-local consts, not co-located with TRANSFER_STALL_TIMEOUT despite stall_guard.rs:31 and :65 documenting them as part of the same coverage story. Hardcoded but bounded-failure-is-better-than-hang, so RELIABLE-positive.
./docs/audit/DESIGN_MAP_2026-06-11.md:484:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69 — TRANSFER_STALL_TIMEOUT = 30s; StallGuard (line 75) read-side and StallGuardWriter (line 139) write-side idle watchdogs that mint io::ErrorKind::TimedOut — This is the designed producer of classifier-visible retryable errors: an idle stall becomes TimedOut, which is_retryable accepts.
./docs/audit/DESIGN_MAP_2026-06-11.md:553:- `TRANSFER_STALL_TIMEOUT` = Duration::from_secs(30) (/home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69) — SIMPLE: good — adaptive watchdog re-armed on progress, no tuning knob. RELIABLE tension: its entire purpose (a downcastable TimedOut for --retry) is voided on the pull-sync path by PullSyncError stringification (pull.rs:780 TODO).
./docs/audit/DESIGN_MAP_2026-06-11.md:568:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69 — TRANSFER_STALL_TIMEOUT=30s; synthesizes sentinel io::Error::new(io::ErrorKind::TimedOut, ...) at :112 and :193 specifically so the retry classifier can downcast it — the producer half of the contract pull.rs:780 breaks.
./docs/audit/DESIGN_MAP_2026-06-11.md:645:- `TRANSFER_STALL_TIMEOUT` = 30 s idle (/home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69) — none for progress per se — it is the only liveness signal that actually exists (DaemonHeartbeat is reserved proto fields only, proto/blit.proto:901-905).
./docs/audit/DESIGN_MAP_2026-06-11.md:688:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69 — Failure-side liveness (closest thing to a heartbeat): TRANSFER_STALL_TIMEOUT idle guard on data-plane reads
./docs/audit/DESIGN_MAP_2026-06-11.md:744:- `TRANSFER_STALL_TIMEOUT` = 30s of zero byte progress (crates/blit-core/src/remote/transfer/stall_guard.rs:69) — RELIABLE: good fail-plainly design and properly shared; value itself is structural-leaning. Minor: not scaled to link class, but byte-level progress observation makes 30s safe even on slow links.
./docs/audit/DESIGN_MAP_2026-06-11.md:794:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/data_plane.rs:15 — CONTROL_PLANE_CHUNK_SIZE=1 MiB (15), RECEIVE_CHUNK_SIZE=1 MiB (546), chunk floor .max(64*1024) (66), client-side socket buffer set from tuning.tcp_buffer_size (110-119), write stall guard wired to TRANSFER_STALL_TIMEOUT (68) — tcp_buffer_size only applied on connect (client) side; accepted sockets on the daemon get only set_tcp_nodelay (daemon push data_plane.rs:119).
./docs/audit/DESIGN_MAP_2026-06-11.md:795:- /home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69 — TRANSFER_STALL_TIMEOUT=30s — single shared stall constant used by core pull (pull.rs:1764), core data-plane writes (data_plane.rs:68), daemon push receive (daemon data_plane.rs:841) — Good single-owner example; value itself is frozen, not link-speed-aware.
./docs/audit/DESIGN_MAP_2026-06-11.md:934:No single module owns cancellation; it is smeared across three non-composing mechanisms written by different strata: tokio_util CancellationToken lives exclusively in blit-daemon/src/active_jobs.rs (minted for all four transfer kinds, honored by exactly one — DelegatedPull, via the three-way select in service/core.rs:741-783); client-side cancellation is future-drop cascading through the AbortOnDrop wrapper private to blit-core/src/remote/pull.rs:31; and cancel-on-disconnect is a tx.closed() race present in only two of five daemon spawn closures. The CLI has no SIGINT handling at all — Ctrl-C cancellation is process death, which works for remote paths only because sockets close, and the local copy engine (orchestrator/, copy/) has zero cancellation hooks. The sharpest problems: the daemon pull/push handlers hold bare JoinHandles for their TCP data planes (service/pull.rs:180,297; push/control.rs:57), so a client disconnect detaches — rather than aborts — an in-flight data plane that CancelJob cannot reach; and blitd itself has no graceful shutdown (main.rs:137-142) while the trivial prometheus bridge does (server.rs:114-121). Field-level divergence hunted and found: the 30s/15s accept/token timeout pair is independently declared four times (twice in pull_sync.rs alone), and the drop-cancellation fix (R32-F2) was applied to pull only, leaving the push client pipeline (push/client/mod.rs:104) with the identical detach-on-drop bug class.
./docs/audit/DESIGN_MAP_2026-06-11.md:938:- Daemon-side cancel reaches exactly ONE of four transfer kinds. supports_cancellation (active_jobs.rs:162-164) limits CancelJob to DelegatedPull by design, but tokens are still minted for all kinds (register, active_jobs.rs:418) and never raced by the push/pull/pull_sync handlers (core.rs:499/579/631) — dead machinery plus a user-visible 'cannot cancel' wall for three of four kinds shown in jobs list / TUI.
./docs/audit/DESIGN_MAP_2026-06-11.md:939:- Orphaned daemon pull data plane: stream_pull_streaming (service/pull.rs:297) holds a bare JoinHandle; when the client cancels the gRPC stream, the handler errors on its next tx.send (e.g. pull.rs:325 manifest batch) and the `?` return DROPS the handle, detaching the TCP data plane mid-transfer. There is no StallGuard on the daemon's send side; the orphan runs until a TCP error/reset, or indefinitely if the client process keeps the data socket open while abandoning the control stream — unreachable by CancelJob (Pull is Unsupported) and invisible cleanup-wise. Same pattern in push/control.rs:57.
./docs/audit/DESIGN_MAP_2026-06-11.md:943:- CancelJob authorization is IP-equality with loopback bypass (active_jobs.rs:1121-1132): a detached transfer started from a NAT'd or multi-homed client may be uncancellable by its own owner (Unauthorized), while any local process on the daemon host can cancel anything — both failure modes silent until attempted.
./docs/audit/DESIGN_MAP_2026-06-11.md:944:- CancelJob returns Cancelled when the token FIRES, not when the transfer stops (core.rs:1112-1115); correctness rests on the handler's select arm. The proto doc (blit.proto:68) is honest about this, but a future kind flipping supports_cancellation without adding a select arm would make the daemon lie to callers — there is no structural link between the policy fn and the handler race.
./docs/audit/DESIGN_MAP_2026-06-11.md:945:- detach=true disables the only client-disconnect cancel arm (core.rs:746-749 gate), making CancelJob the SOLE stop mechanism for detached transfers — combined with the IP-auth risk above, a detached transfer can become unstoppable except by killing blitd.
./docs/audit/DESIGN_MAP_2026-06-11.md:962:  - sites: /home/michael/dev/Blit/crates/blit-daemon/src/service/push/data_plane.rs:146 ('data plane worker cancelled'); /home/michael/dev/Blit/crates/blit-daemon/src/service/push/control.rs:313 ('data plane task cancelled'); /home/michael/dev/Blit/crates/blit-daemon/src/service/core.rs:805-807 ('cancelled via CancelJob' / 'client cancelled'); /home/michael/dev/Blit/crates/blit-daemon/src/active_jobs.rs:1065 ('cancelled before outcome recorded'); /home/michael/dev/Blit/crates/blit-daemon/src/service/pull.rs:823 ('enumeration cancelled')
./docs/audit/DESIGN_MAP_2026-06-11.md:967:- `TRANSFER_STALL_TIMEOUT` = Duration::from_secs(30) (/home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69) — SIMPLE: no user knob (good). RELIABLE: a legitimately quiet link (laptop sleep, transient route flap >30s) hard-fails the transfer with TimedOut; it is also the de-facto bound on how long an orphaned (detached, uncancelled) receive task lives — but only on read paths, not the daemon pull SEND path.
./docs/audit/DESIGN_MAP_2026-06-11.md:983:- /home/michael/dev/Blit/crates/blit-daemon/src/service/core.rs:1093 — CancelJob RPC handler — maps CancelOutcome to OK/FailedPrecondition/NotFound/PermissionDenied with plain explanatory text — good RELIABLE posture
./docs/audit/DESIGN_MAP_2026-06-11.md:985:- /home/michael/dev/Blit/crates/blit-daemon/src/service/core.rs:1303 — resolve_delegated_pull_outcome biased select — audit-10: handler branch first so a completed transfer beats a simultaneous cancel; None decoded at 801-808 into 'cancelled via CancelJob' vs 'client cancelled'
./docs/audit/DESIGN_MAP_2026-06-11.md:992:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:1764 — StallGuard on pull receive socket (audit-1c) — transfer that goes silent fails after TRANSFER_STALL_TIMEOUT instead of hanging — failure bound complementing cancellation
./docs/audit/DESIGN_MAP_2026-06-11.md:993:- /home/michael/dev/Blit/crates/blit-daemon/src/service/delegated_pull.rs:344 — cancellation-by-future-drop contract — comment: CLI disconnect drops this future; inner pull_sync_with_spec (352-360) cleans up via AbortOnDrop cascade. This is the path CancelJob actually stops
./docs/audit/DESIGN_MAP_2026-06-11.md:1003:- /home/michael/dev/Blit/crates/blit-app/src/admin/jobs.rs:59 — CancelJob client wrapper — maps gRPC codes onto CancelJobOutcome::{Cancelled,NotFound,Unsupported}; consumed by CLI and TUI
./docs/audit/DESIGN_MAP_2026-06-11.md:1007:- /home/michael/dev/Blit/proto/blit.proto:76 — CancelJob rpc contract — doc at 59-75 states only delegated pulls honor cancellation; CancelJobRequest/Response at 823-836; detach field at 642
./docs/audit/DESIGN_MAP_2026-06-11.md:1008:- /home/michael/dev/Blit/crates/blit-tui/src/state.rs:382 — TUI cancel target resolution (site only, not dug into) — cursor row -> (daemon, transfer_id) for CancelJob
./docs/audit/DESIGN_MAP_2026-06-11.md:1009:- /home/michael/dev/Blit/crates/blit-tui/src/main.rs:3909 — TUI spawn_cancel_transfer / spawn_cancels_for_targets (3886) (sites only) — d-22/d-30/m2f-8: one CancelJob RPC per (daemon, id); confirm prompt state machine at 285-449; Esc/Ctrl+C always quit (5778-5786)
./docs/audit/DESIGN_MAP_2026-06-11.md:1124:- **Jobs/TUI milestone stratum: slice-ID vocabulary (`b-1`..`b-5`, `c-1a/c-1b`, `c-3 round 2`, `c-4`, `c-5a/c-5b`, `m-jobs-1..3`, `rec-1/rec-2`), module-level `//!` changelogs enumerating landed slices, and `#[allow(dead_code)] // lands in b-4` annotations** — The ActiveJobs registry, Subscribe broadcast/forwarder, GetState, CancelJob, recents persistence, and metrics were built in narrated micro-slices. Distinctive: free functions factored explicitly "so tests can call it without a runtime" (tick_progress_once, resolve_delegated_pull_outcome, build_transfer_finished_event), RAII-guard discipline, and dead_code allowances that were never removed after the consumer landed.
./docs/audit/DESIGN_MAP_2026-06-11.md:1258:- **Lettered micro-slice IDs (b-5, c-5a, c-5b, c-6, rec-2, m-jobs-3, d-25, d-68 R4) and gRPC-status-code → typed-outcome mapping enums.** — The jobs/observability era (admin/jobs.rs, detach flow in transfers/remote.rs, late endpoint-parsing fixes). Distinct error idiom: instead of bare eyre strings, status codes are matched into typed enums with remediation text (CancelJobOutcome, the daemon-too-old refusal on empty transfer_id). Streaming returned as raw tonic::Streaming<DaemonEvent> — a third streaming idiom alongside the du/find callback style and the consume-internally style of run_delegated_pull.
./docs/audit/DESIGN_MAP_2026-06-11.md:1259:  - crates/blit-app/src/admin/jobs.rs:4 ('sub-slice b-5'), jobs.rs:130-140 ('c-5a' / 'c-5b' replay semantics), jobs.rs:100 ('rec-2'), jobs.rs:38-52 (CancelJobOutcome typed mapping vs jobs.rs:29 bare eyre!(status.message()))
./docs/audit/DESIGN_MAP_2026-06-11.md:1461:  - evidence: No local stall detection exists; the only stall guard is the remote-transfer 30s idle AsyncRead/AsyncWrite adapter (crates/blit-core/src/remote/transfer/stall_guard.rs:69, TRANSFER_STALL_TIMEOUT = 30s, from the 2026-06 audit)
./docs/audit/DESIGN_MAP_2026-06-11.md:1753:- **[complete]** Milestone M-Jobs: detach field on DelegatedPullRequest (delegated-only), CancellationToken rows, CancelJob RPC, blit jobs cancel/watch, CLI --detach rejected on push/pull
./docs/audit/DESIGN_MAP_2026-06-11.md:1754:  - evidence: proto/blit.proto:638-642 (detach=32 with delegated-only comment); :76 (CancelJob); crates/blit-daemon/src/service/core.rs:700 (let detach = req.detach); crates/blit-cli/src/jobs.rs:27-28 (Cancel/Watch); crates/blit-cli/src/cli.rs:339-349 (--detach with relay/byte-path restrictions)
crates/blit-core/src/remote/pull.rs:1904:    use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
crates/blit-core/src/remote/pull.rs:1934:    // goes silent (no bytes for TRANSFER_STALL_TIMEOUT) fails fast with a
crates/blit-core/src/remote/pull.rs:1944:    let mut stream = StallGuard::new(stream, TRANSFER_STALL_TIMEOUT);
./crates/blit-tui/src/config.rs:501:    /// on screen after a CancelJob reply lands. Sending
./crates/blit-tui/src/config.rs:509:    /// CancelJob RPC immediately. `y` confirms, `n` or
./crates/blit-tui/src/display_f2.rs:21:    use blit_app::admin::jobs::CancelJobOutcome;
./crates/blit-tui/src/display_f2.rs:53:                CancelJobOutcome::Cancelled { transfer_id: id } => F2CancelDisplay::Cancelled {
./crates/blit-tui/src/display_f2.rs:56:                CancelJobOutcome::NotFound { transfer_id: id } => F2CancelDisplay::NotFound {
./crates/blit-tui/src/display_f2.rs:59:                CancelJobOutcome::Unsupported {
./docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:116:**Claim**: Only delegated_pull's spawn closure races client-hangup and CancelJob; the push, pull, and pull_sync closures just await their handler, so a client that disconnects during a send-free compute phase leaves the daemon doing unbounded unobservable work that CancelJob explicitly refuses to touch, while an active_jobs comment asserts a tx.closed() drop mechanism that does not exist for these kinds.
./docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:149:**Mechanism**: Workers are spawned at data_plane.rs:130 into a plain Vec<JoinHandle>; the join loop at :143-146 does `handle.await.map_err(...)??` — the first Err returns from the function, dropping every remaining handle (detach, not abort). Likewise the accept-timeout arm at :103-110 returns Err after some workers were already spawned. Each detached worker continues running handle_data_plane_stream → receive_push_data_plane → FsTransferSink, writing client bytes to disk with no owner, unreachable by CancelJob (push reports supports_cancellation=false, active_jobs.rs:163-164), until its socket EOFs or the 30s StallGuard fires — and if the client's own detached pipeline (sibling finding) keeps sending, indefinitely. The failed RPC meanwhile drives the client to retry, producing a second writer set against the same destination paths. design-2 (.review/findings/design-2-orphaned-daemon-data-planes.md) names exactly three sites (service/pull.rs:180/:297, push/control.rs:57); this Vec of per-stream workers is a fourth, one level deeper, and needs the same AbortOnDrop/JoinSet-abort treatment or design-2's fix will still leak the inner layer.
./docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:155:- /home/michael/dev/Blit/crates/blit-daemon/src/active_jobs.rs:163 — supports_cancellation() is DelegatedPull-only — orphaned push workers unreachable by CancelJob
./docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:707:**Claim**: The entire detached-job lifecycle — Subscribe stream, watch loop with GetState fallback reconciliation, CancelJob exit-code contract, --detach output — is never executed end-to-end by cargo test; coverage stops at formatting/exit-code unit tests.
./docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:709:**Mechanism**: rg for 'jobs|Subscribe|cancel' across blit-cli/tests hits only remote_remote.rs, and those hits (e.g. :649, :717) are the fake server's unimplemented trait-method stubs, not jobs-verb tests. No test passes 'jobs' or '--detach' to the CLI binary (rg 'detach' over tests: zero hits). What exists: blit-cli/src/jobs.rs:795+ unit tests (pure formatting/exit-code mapping) and blit-daemon/src/active_jobs.rs unit tests (29, in-process registry). The wire path — jobs watch opening Subscribe, the stream-error fallback to one final GetState (jobs.rs:348-362), cancel_exit_code's 0/1 contract against a real daemon's CancelJob — runs in zero tests. Given the already-filed design-2 (orphaned daemon data planes, cancellation reaching one of four transfer kinds), the team is actively changing cancellation behavior with no harness to detect regressions in the user-facing verbs.
./docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:713:- /home/michael/dev/Blit/crates/blit-cli/src/jobs.rs:60 — cancel_exit_code maps CancelJobOutcome to the exit-code contract — contract never exercised against a live daemon
./docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1167:Verified from code this session (not map trust): every finding's mechanism was re-read at the cited lines. Checked and found CLEAN (good single-owner patterns, valuable for Phase C): TRANSFER_STALL_TIMEOUT is declared once in stall_guard.rs and imported everywhere else (rg showed only use-sites in blit-core pull/data_plane and blit-daemon push/data_plane — the one liveness constant that did consolidate correctly); mDNS is genuinely owned by blit-core/src/mdns.rs (ServiceDaemon::new appears only there); relative_path_to_posix has a single definition in path_posix.rs with both the push client (helpers.rs:52) and daemon (util.rs:153) delegating to it — the normalize_relative_path 'twins' are thin wrappers, not duplication. MAX_TAR_SHARD_BYTES is single-sourced in tar_safety.rs and referenced (not re-typed) by pipeline.rs per the map; I did not re-verify every wire cap. Deliberately NOT reported (cross-referenced instead): the triplicated gRPC channel builder, client keepalive absence, tonic decode limit, and the Status->eyre stringification family (all inside queued slice-2 transport work, STATE.md Queue item 2); unbounded data-plane connects (design-3); the CLI pull byte double-count bug itself (design-1 — I filed only the structural folding-rule duplication around it); orphaned daemon data planes / AbortOnDrop-vs-bare-JoinHandle (design-2 territory). Dropped as low severity: tar-shard 1 MiB reservation duplicated twice; TUI byte-formatter ladder duplicated in f2.rs/f4.rs plus blit-app/display.rs (display-only; TUI light-pass rule); throughput smoothing triplication (per-layer cadences arguably legitimate); mpsc send-error fixed-string family (likely reshaped by queued error-chain work); the '1 MiB' five-constant family (its real risk is the decode-limit invariant, which is queued). Not covered in depth: blit-prometheus-bridge (map reports it self-consistent; spot-checked only), blit-tui internals beyond progress_accum (Phase 6 rule), the double-buffered send/receive loop twins in data_plane.rs (read but judged a FAST/design question for the adaptive-streams landing rather than a consolidation slice), and Windows casefold-key divergence (already tracked as h-paths-2 in docs/audit/findings/inconsistency-paths.md per the map; not re-filed).
./docs/audit/inventory/plan-wire.md:127:**Source**: blit.proto:CancelJob comment (lines 59-65)
./docs/audit/inventory/plan-wire.md:130:Only delegated remote→remote pulls support cancellation today — push/pull/pull_sync have the CLI in the byte path, so a client-side cancel already drops the handler future and `CancelJob` from another client wouldn't have a meaningful semantic.
./docs/audit/inventory/plan-wire.md:133:**Source**: blit.proto:CancelJob comment (lines 67-75)
./docs/audit/inventory/plan-wire.md:136:CancelJob status: OK → cancellation token fired; NOT_FOUND → no active transfer matches transfer_id (already completed or never existed); FAILED_PRECONDITION → transfer exists but its kind doesn't honor cancellation today.
./docs/audit/inventory/plan-wire.md:318:`rpc CancelJob(CancelJobRequest) returns (CancelJobResponse)` — cancel a daemon-side in-flight transfer by `transfer_id`.
./docs/audit/inventory/plan-wire.md:630:`CancelJobRequest`: `transfer_id` (1). `CancelJobResponse`: `transfer_id` (1, echoed for confirmation; outcome encoded in gRPC Status not response body).
./docs/audit/inventory/plan-wire.md:874:TUI is read-mostly control surface over the daemon: Subscribe's to each discovered daemon's `DaemonEvent` stream and renders live transfer state from `GetState`; can launch transfers / CancelJob / ClearRecent. Daemon discovery is mDNS; multi-daemon F2 merges per-daemon Subscribe streams into one event channel.
./docs/audit/inventory/plan-wire.md:898:When `detach=false` (historical), destination daemon races transfer against `tx.closed()` so CLI disconnect drops in-flight pull future and data plane tears down (R30-F2). When `detach=true`, the race disarms: destination daemon owns the transfer through completion, failure, or `CancelJob(transfer_id)`. CLI free to exit immediately after `Started` event.
./docs/audit/inventory/plan-wire.md:983:**Source**: blit.proto:CancelJob comment (lines 59-65)
./docs/audit/inventory/plan-wire.md:986:Push/pull/pull_sync do not honor CancelJob — they have CLI in byte path so client-side cancel suffices. Return FAILED_PRECONDITION.
./docs/audit/inventory/plan-tui.md:122:"Three new RPCs (`GetState`, `Subscribe`, `CancelJob`) plus the `detach: bool` field on `DelegatedPullRequest`, the `transfer_id` field on `DelegatedPullStarted`, and the `transfer_id_filter` field on `SubscribeRequest` — names and message fields are the contract. M-Jobs lands `GetState` / `CancelJob` / `detach`; C lands `Subscribe` and `transfer_id_filter`."
./docs/audit/inventory/plan-tui.md:262:"`CancelJob` RPC: `rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);` `CancelJobRequest { string transfer_id; }` `CancelJobResponse { bool cancelled (true if job existed and cancellation was initiated, false if not found); string reason (human-readable; empty when cancelled=true); }`"
./docs/audit/inventory/plan-tui.md:354:"[`CancelJob`] Implementation reaches into the `ActiveJobs` entry, fires a `CancellationToken` the spawn closure is watching, and waits briefly for the transfer to wind down (sink finish, partial-file cleanup if any). Returns once the job is removed from the active table or after a 5-second timeout, whichever first."
./docs/audit/inventory/plan-tui.md:408:"Spawn-closure lifecycle change. The `delegated_pull` dispatch site consults `req.detach`. When false (default), behavior is unchanged — `tx.closed()` cancellation race still arms (R30-F2). When true, the cancellation race is **disarmed**: the daemon owns the transfer through to completion or `CancelJob(id)`. Push / pull / pull_sync sites are unchanged; they cannot detach."
./docs/audit/inventory/plan-tui.md:504:CLI surface: `blit jobs list <remote>` calls `GetState` and prints active+recent. `blit jobs cancel <remote> <transfer_id>` calls `CancelJob`. `blit jobs watch <remote> <transfer_id>` follows a transfer to completion; ships in M-Jobs as `GetState` polling loop with `--interval-ms`, `--timeout-secs`, `--json`; milestone C upgrades to `Subscribe` stream with `transfer_id_filter`.
./docs/audit/inventory/plan-tui.md:584:Phasing table: 1) A.0 — extract `blit-app` (no wire changes, ~4–5 days mechanical moves). 2) B — `GetState` + `ActiveJobs` table + recent ring (+`GetState`, ~500 daemon + ~100 CLI). 3) M-Jobs — daemon-owned lifecycle (delegated-only) + `CancelJob` + `detach` (+`CancelJob`, +`detach` on `DelegatedPullRequest`, +`transfer_id` on `DelegatedPullStarted`, ~500 daemon + ~200 CLI). 4) C — `Subscribe` + per-job event ring + byte-level instrumentation (+`Subscribe`, +`SubscribeRequest.transfer_id_filter`, ~1500 daemon + ~100 CLI). 5) A.1 — the TUI itself (none, ~3000). 6) D — Verify + diagnostics (none, ~400 TUI). 7) E — polish (none, ~600).
./docs/audit/inventory/plan-tui.md:834:"Cancellation: server-side via `CancelJob`. TUI's cancel hotkey fires `CancelJob(transfer_id)` (M-Jobs, §6.5). Works on transfers the TUI didn't initiate. Replaces the original draft's 'client-side Ctrl-C' approach."
./docs/audit/inventory/plan-tui.md:990:"Milestone M-Jobs — Daemon-owned transfer lifecycle. Extends the `ActiveJobs` table with the cancellation + detach lifecycle bits. Adds: `detach: bool` field on `DelegatedPullRequest` (delegated-only); `CancellationToken` field on each `ActiveJob` row; spawn-closure lifecycle change in `delegated_pull` only; `CancelJob(transfer_id)` RPC; CLI surface: `--detach` flag (remote→remote only), `blit jobs cancel`, `blit jobs watch` (GetState polling)."
./docs/audit/inventory/plan-tui.md:1002:"Milestone A.1 — The TUI itself. Now the TUI screens land. All foundation is in place: F1 Daemons (mDNS list, per-daemon detail pane lit up by `GetState`, 'local' sentinel endpoint); F2 Transfers (active pane fed by `Subscribe`, history fed by `GetState.recent`. Cancel hotkey fires `CancelJob`); F3 Browse (`List`/`Find`/`DiskUsage`/`FilesystemStats`. Multi-select + c/m/v/D modal actions dispatch through `blit_app` with `detach=true`); F4 Profile (reads `~/.config/blit/perf_local.jsonl` directly). Result: real single-pane-of-glass from day one."
./docs/audit/inventory/plan-tui.md:1071:§3 row: "Job lifecycle (cancel + daemon-owned transfers) — gRPC `CancelJob` + `detach` field on transfer specs — **new** — see §6.5". The phrase "on transfer specs" implies `TransferOperationSpec`, but §6.5 explicitly rejects that location: "Field lives on the delegated-pull request itself, **not** on `TransferOperationSpec`." The §3 summary text was not updated to match the §6.5 decision.
./docs/audit/inventory/plan-tui.md:1076:TUI_REWORK.md §11: "No daemon wire change is required for the shell itself." TUI_DESIGN.md §6 plans three new RPCs (`GetState`, `Subscribe`, `CancelJob`) and several new message fields. Not strictly contradictory (REWORK speaks to the shell rework only, DESIGN to overall Phase 5), but a reader looking only at REWORK might miss that Phase 5 wire work continues in parallel.
./docs/audit/inventory/plan-tui.md:1081:The phasing table in §11 correctly says "+`detach` on `DelegatedPullRequest`"; the §3 "What lives where" row's column "Mechanism" says "`CancelJob` + `detach` field on transfer specs". Minor naming inconsistency.
./docs/audit/inventory/plan-tui.md:1092:| docs/plan/TUI_DESIGN.md | 1118 | Read end-to-end in one pass; covers Purpose (§1), CLI parity map (§2), what-lives-where table (§3), 5 design principles (§4), four screens with sketches (§5.1-5.4), wire surface §6 with full proto specs for Subscribe/GetState/CancelJob (§6.1-6.5), crate/dependency shape §7.1-7.5 including blit-app rationale and module-mapping table, milestones A.0/B/M-Jobs/C/A.1/D/E §8, non-goals §9, open questions §10 (Q1-Q7), phasing summary table §11, structural commitments §12. |
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:140:**Remediation status**: `TRANSFER_STALL_TIMEOUT` constant hoisted (audit-h3a).
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:219:`--detach`, `jobs watch`, and `CancelJob` all shipped.
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:527:### M1. CancelJob `Unauthorized` outcome is not modeled by the CLI / app consumer
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:533:`CancelJobOutcome` enum only models `Cancelled` / `NotFound` / `Unsupported`
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:753:DelegatedPull, GetState, CancelJob, ClearRecent, Subscribe)
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:897:**Canonical pattern**: One `TRANSFER_STALL_TIMEOUT` constant. One `connect_with_timeout` +
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:948:| 4 | CancelJob Unauthorized not propagated | none | **NEW** → M1 |
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:1002:2. **H3** One `TRANSFER_STALL_TIMEOUT` constant + wrap every receive path in it (daemon
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:1034:20. **M1** Add `Unauthorized` to app-layer `CancelJobOutcome`.
./docs/audit/AUDIT_REPORT_2026-06-04_R2.md:1121:- `crates/blit-app/src/admin/jobs.rs:39` — M1 (CancelJobOutcome missing Unauthorized)
crates/blit-core/src/remote/transfer/stall_guard.rs:27://!   trips after `TRANSFER_STALL_TIMEOUT` of no successful write
crates/blit-core/src/remote/transfer/stall_guard.rs:71:pub const TRANSFER_STALL_TIMEOUT: Duration = Duration::from_secs(30);
crates/blit-core/src/remote/transfer/stall_guard.rs:134:/// `io::ErrorKind::TimedOut` after `TRANSFER_STALL_TIMEOUT` of no
./docs/audit/AUDIT_REPORT_2026-06-04.md:61:**Code does**: `build_delegated_execution` hardcodes `detach: false` with a comment "Always attached; detached/F2-visible delegation is a follow-up." Meanwhile the wire surface (proto field), daemon-side detach lifecycle (M-Jobs select arm `service/core.rs:1314-1320`), CLI's `--detach`, `jobs watch`, and `CancelJob` all shipped.
./docs/audit/AUDIT_REPORT_2026-06-04.md:117:**Suggested remediation pointer**: Wrap every `execute_receive_pipeline(...)` and `DataPlaneSession::from_stream(...)` in `StallGuard(_, TRANSFER_STALL_TIMEOUT)`. Hoist the 30 s constant out of `stall_guard.rs` into a shared `transfer::TRANSFER_STALL_TIMEOUT`. Add a push-receive stall test paralleling `pipeline.rs::receive_pipeline_aborts_on_stall`.
./docs/audit/AUDIT_REPORT_2026-06-04.md:419:**Canonical pattern**: One shared `TRANSFER_STALL_TIMEOUT` constant. One `connect_with_timeout` + `CONNECT_TIMEOUT` in `blit-core::remote::client`. One token-rejection helper returning one Status code. Reconcile `retry.rs` and `errors.rs` to one classifier; extend it to handle tonic Status codes. Document HTTP/2 keepalive rationale at every server-side `stream.message().await` site.
./docs/audit/AUDIT_REPORT_2026-06-04.md:462:5. **Add stall guard to daemon push-receive** (finding #9) — DoS-class hardening; wrap `execute_receive_pipeline(socket, ...)` in `StallGuard(_, TRANSFER_STALL_TIMEOUT)`. Single-file change with regression test.
./docs/audit/inventory/code-daemon.md:34:- **rpc-delegated-pull-dispatch** — `crates/blit-daemon/src/service/core.rs:663-843` — Three-way `tokio::select!` via `resolve_delegated_pull_outcome`: handler-first biased select races handler completion vs `tx.closed()` (client hangup, disabled by `detach=true`) vs `cancel_token.cancelled()` (CancelJob). Maps outcome `None` to either "cancelled via CancelJob" (if token fired) or "client cancelled". _(notes: audit-10 fix — handler ordered first so a transfer that completed at the same instant CancelJob fired isn't mis-recorded as cancelled. R30-F2 client-hangup race. m-jobs-3 detach gating)_
./docs/audit/findings/inconsistency-timeouts.md:18:**Recommendation**: Wrap every `execute_receive_pipeline(stream, ...)` and every `DataPlaneSession::from_stream(stream, ...)` call (both sender- and receiver-side) in `StallGuard` with the existing `PULL_STALL_TIMEOUT` constant. Hoist the constant out of `stall_guard.rs` into a shared `transfer::TRANSFER_STALL_TIMEOUT`. Add a test analogous to `pipeline.rs::receive_pipeline_aborts_on_stall` for the push-receive path.
./docs/audit/inventory/code-tests-scripts.md:142:- _(none observed)_ — No test exercises Ctrl-C or `CancelJob` RPC despite `CancelJobRequest`/`CancelJobResponse` being part of the trait stub in `remote_remote.rs:713-718` and `pull_sync_with_spec_wire.rs:168-173`.
./docs/audit/findings/drift-tui.md:34:**Notes**: The wire surface, daemon-side detach lifecycle (M-Jobs), CLI's `--detach` flag, `jobs watch`, and `CancelJob` all shipped. The single place that should set `detach=true` — the TUI's delegated execution builder — instead sets it false. This breaks the "single-pane-of-glass survives initiator disconnect" promise (TUI_DESIGN §3 closing: "any TUI on the LAN can list, watch, cancel, or initiate transfers on any reachable daemon, and transfers survive their initiator disconnecting"). Remediation: flip to `detach: true` for remote→remote and surface a banner on the trigger modal for local-endpoint transfers per §5.2.
./docs/audit/findings/drift-tui.md:49:**Code does**: blit-app exists (A.0 ✓), `GetState`/`ActiveJobs`/recent ring exist (B ✓), `CancelJob` + `detach` field exist (M-Jobs ✓), `Subscribe` exists (C ✓). But A.1 ("the TUI itself" — TUI_DESIGN §8 — listing screens F1 Daemons / F2 Transfers / F3 Browse / F4 Profile) shipped while the Dual screen (TUI_REWORK §1 rework, scheduled M1-M8) became the default *without* the rework's transfer-execution layer wired (see `dual-pane-actions-are-display-only`). The default screen is `ScreenArg::Dual` (`main.rs:105`), but the productive transfer paths remain on F1/F3/F4 (the pre-rework model).
./docs/audit/findings/drift-tui.md:120:- **GetState/Subscribe/CancelJob/ClearRecent RPCs**: All four shipped on the wire per spec (`proto/blit.proto:57, 76, 89, 107`); daemon implementations land in `service/core.rs:353-1146`.
./docs/audit/findings/drift-wire.md:187:- **GetState/Subscribe/CancelJob/ClearRecent RPCs**: proto definitions (`proto/blit.proto:50-107`) match daemon implementations (`crates/blit-daemon/src/service/core.rs:353-1146`). Field numbers, default `recent_limit`, `JOB_EVENT_RING_CAP`, replay semantics — all match.
./docs/audit/inventory/code-tui-state.md:163:- **F2 `selected_active_daemon` API not surfaced as `is_present`** — `state.rs:398-401` — `selected_active_daemon` returns the daemon STRING; callers like CancelJob need both daemon AND id together. The two accessors are called separately — risk of mismatched pair if state mutates between calls.
./docs/audit/findings/inconsistency-errors.md:34:7. `crates/blit-app/src/admin/jobs.rs:91-95` — `cancel`: preserves code: `"CancelJob failed ({code}): {msg}"`
./docs/audit/findings/inconsistency-errors.md:40:**Recommendation**: Single helper `fn status_to_eyre(rpc_name: &str, status: Status) -> eyre::Report` used everywhere. Without it, `blit rm /server:/module/foo` against a read-only module shows `"module 'foo' is read-only"` (no code) while `blit jobs cancel ...` against same-condition daemon shows `"CancelJob failed (FailedPrecondition): ..."`. The shape divergence also blocks retry classification: see errors-3.
./crates/blit-app/src/admin/jobs.rs:10:    CancelJobRequest, ClearRecentRequest, DaemonEvent, DaemonState, GetStateRequest,
./crates/blit-app/src/admin/jobs.rs:34:/// Outcome of a `CancelJob` RPC. The wire surface encodes
./crates/blit-app/src/admin/jobs.rs:39:pub enum CancelJobOutcome {
./crates/blit-app/src/admin/jobs.rs:46:    /// `CancelJob` off. Since D-2026-07-04-3 flipped push and
./crates/blit-app/src/admin/jobs.rs:57:/// Issue the `CancelJob` RPC against `remote`. Errors only on
./crates/blit-app/src/admin/jobs.rs:60:/// Ok) get mapped onto [`CancelJobOutcome`] for the caller to
./crates/blit-app/src/admin/jobs.rs:62:pub async fn cancel(remote: &RemoteEndpoint, transfer_id: &str) -> Result<CancelJobOutcome> {
./crates/blit-app/src/admin/jobs.rs:67:        .cancel_job(CancelJobRequest {
./crates/blit-app/src/admin/jobs.rs:84:            Ok(CancelJobOutcome::Cancelled { transfer_id: id })
./crates/blit-app/src/admin/jobs.rs:87:            Code::NotFound => Ok(CancelJobOutcome::NotFound {
./crates/blit-app/src/admin/jobs.rs:90:            Code::FailedPrecondition => Ok(CancelJobOutcome::Unsupported {
./crates/blit-app/src/admin/jobs.rs:95:                "CancelJob failed ({}): {}",
./docs/audit/inventory/code-tui-main.md:48:- **spawn-cancels-for-targets** — `crates/blit-tui/src/main.rs:3876-3892` — One CancelJob per (daemon, id); skips targets whose daemon identity won't parse.
./docs/audit/inventory/code-tui-main.md:61:- **reset-f2-for-resubscribe** — `crates/blit-tui/src/main.rs:2786-2808` — d-47/d-48: F1 Enter on a daemon row repoints F2. Clears transfers, repoints `parsed_remote`/label, drops `cancel_status` to Idle (d-48 R2: stale confirm against old daemon could fire CancelJob with wrong ids).
crates/blit-core/src/remote/transfer/data_plane.rs:11:use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
crates/blit-core/src/remote/transfer/data_plane.rs:53:/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
crates/blit-core/src/remote/transfer/data_plane.rs:82:    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
crates/blit-core/src/remote/transfer/data_plane.rs:176:            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
./crates/blit-daemon/Cargo.toml:36:# the production 30 s TRANSFER_STALL_TIMEOUT without wall-clock waits.
./crates/blit-daemon/src/active_jobs.rs:42://!   forthcoming `CancelJob` RPC can drop in-flight
./crates/blit-daemon/src/active_jobs.rs:62://! - `CancelJob` RPC + CLI verb (`m-jobs-2-cancel-rpc`).
./crates/blit-daemon/src/active_jobs.rs:159:    /// Whether `CancelJob` dispatch fires this kind's cancellation
./crates/blit-daemon/src/active_jobs.rs:180:/// `CancelJob` RPC handler will map each variant onto a
./crates/blit-daemon/src/active_jobs.rs:185:/// - `Unsupported` → `Code::FailedPrecondition` — CancelJob
./crates/blit-daemon/src/active_jobs.rs:460:    /// the upcoming `CancelJob` RPC needs to map onto gRPC
./crates/blit-daemon/src/active_jobs.rs:917:    /// `TransferStarted.transfer_id`, M-Jobs `CancelJob`) can
./crates/blit-daemon/src/active_jobs.rs:983:    /// (via the CancelJob RPC in m-jobs-2).
./crates/blit-daemon/src/active_jobs.rs:1839:        // D-2026-07-04-3 flipped CancelJob dispatch on for the
./crates/blit-daemon/src/service/transfer_session_e2e.rs:1://! ONE_TRANSFER_PATH otp-4a/4b loopback e2e: the daemon serves the
./crates/blit-daemon/src/service/transfer_session_e2e.rs:35:use blit_core::transfer_session::SessionFault;
./crates/blit-daemon/src/service/transfer_session_e2e.rs:193:fn fault_of(err: &eyre::Report) -> &SessionFault {
./crates/blit-daemon/src/service/transfer_session_e2e.rs:194:    err.downcast_ref::<SessionFault>()
./crates/blit-daemon/src/service/transfer_session_e2e.rs:195:        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
./crates/blit-daemon/src/service/transfer_session_e2e.rs:198:// --- otp-4b-3: deterministic mid-transfer cancel over the data plane ---
./crates/blit-daemon/src/service/transfer_session_e2e.rs:271:/// otp-4b-3: fire a `CancelJob`-equivalent (the row's cancellation token,
./crates/blit-daemon/src/service/transfer_session_e2e.rs:274:/// `SessionFault{CANCELLED}` — the peer's framed abort reason — rather
./crates/blit-daemon/src/service/transfer_session_e2e.rs:304:    // Fire the row's cancellation token — exactly what the `CancelJob` RPC
./crates/blit-daemon/src/service/transfer_session_e2e.rs:321:    // The client must surface CANCELLED promptly (no hang).
./crates/blit-daemon/src/service/transfer_session_e2e.rs:330:        "the client surfaces the peer's framed CANCELLED, not the data-plane break: {err:#}"
crates/blit-cli/tests/remote_remote.rs:459:        _: tonic::Request<blit_core::generated::CancelJobRequest>,
crates/blit-cli/tests/remote_remote.rs:460:    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
crates/blit-cli/tests/remote_remote.rs:622:        _: tonic::Request<blit_core::generated::CancelJobRequest>,
crates/blit-cli/tests/remote_remote.rs:623:    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
crates/blit-cli/src/jobs.rs:3:use blit_app::admin::jobs::{CancelJobOutcome, WatchSnapshot};
crates/blit-cli/src/jobs.rs:57:/// Map [`CancelJobOutcome`] to the contract's exit codes.
crates/blit-cli/src/jobs.rs:60:pub(crate) fn cancel_exit_code(outcome: &CancelJobOutcome) -> ExitCode {
crates/blit-cli/src/jobs.rs:62:        CancelJobOutcome::Cancelled { .. } => ExitCode::SUCCESS,
crates/blit-cli/src/jobs.rs:63:        CancelJobOutcome::NotFound { .. } => ExitCode::from(1),
crates/blit-cli/src/jobs.rs:64:        CancelJobOutcome::Unsupported { .. } => ExitCode::from(2),
crates/blit-cli/src/jobs.rs:560:fn print_cancel_json(outcome: &CancelJobOutcome) {
crates/blit-cli/src/jobs.rs:563:        CancelJobOutcome::Cancelled { transfer_id } => json!({
crates/blit-cli/src/jobs.rs:567:        CancelJobOutcome::NotFound { transfer_id } => json!({
crates/blit-cli/src/jobs.rs:571:        CancelJobOutcome::Unsupported {
crates/blit-cli/src/jobs.rs:586:fn print_cancel_human(remote: &RemoteEndpoint, outcome: &CancelJobOutcome) {
crates/blit-cli/src/jobs.rs:588:        CancelJobOutcome::Cancelled { transfer_id } => {
crates/blit-cli/src/jobs.rs:591:        CancelJobOutcome::NotFound { transfer_id } => {
crates/blit-cli/src/jobs.rs:597:        CancelJobOutcome::Unsupported {
crates/blit-cli/src/jobs.rs:896:        let cancelled = CancelJobOutcome::Cancelled {
crates/blit-cli/src/jobs.rs:899:        let not_found = CancelJobOutcome::NotFound {
crates/blit-cli/src/jobs.rs:902:        let unsupported = CancelJobOutcome::Unsupported {
crates/blit-cli/tests/jobs_lifecycle.rs:10://! CancelJob dispatch on for attached push/pull_sync, so exit 2 no
crates/blit-cli/tests/jobs_lifecycle.rs:444:        _: tonic::Request<blit_core::generated::CancelJobRequest>,
crates/blit-cli/tests/jobs_lifecycle.rs:445:    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
./crates/blit-daemon/src/service/core.rs:17:    daemon_event, ActiveTransfer, CancelJobRequest, CancelJobResponse, ClearRecentRequest,
./crates/blit-daemon/src/service/core.rs:353:    /// ONE_TRANSFER_PATH otp-4a: the daemon serves the unified session
./crates/blit-daemon/src/service/core.rs:377:        // the row still supports CancelJob and appears in GetState, and
./crates/blit-daemon/src/service/core.rs:395:            // SessionError{CANCELLED}, not a bare Status (codex F1).
./crates/blit-daemon/src/service/core.rs:721:        // failure, or `CancelJob(transfer_id)` regardless of
./crates/blit-daemon/src/service/core.rs:773:            //   cancel_token.cancelled() → `CancelJob` RPC fired the
./crates/blit-daemon/src/service/core.rs:780:            //   None         → cancelled (client OR CancelJob)
./crates/blit-daemon/src/service/core.rs:790:            // a hangup / `CancelJob`. See that helper for the rationale.
./crates/blit-daemon/src/service/core.rs:818:            //   None        → client hangup or CancelJob.
./crates/blit-daemon/src/service/core.rs:822:            //                  CancelJob; otherwise it was the
./crates/blit-daemon/src/service/core.rs:828:                    (false, Some("cancelled via CancelJob".to_string()))
./crates/blit-daemon/src/service/core.rs:1118:        request: Request<CancelJobRequest>,
./crates/blit-daemon/src/service/core.rs:1119:    ) -> Result<Response<CancelJobResponse>, Status> {
./crates/blit-daemon/src/service/core.rs:1127:                "CancelJobRequest.transfer_id must not be empty",
./crates/blit-daemon/src/service/core.rs:1135:            CancelOutcome::Cancelled => Ok(Response::new(CancelJobResponse {
./crates/blit-daemon/src/service/core.rs:1320:/// `CancelJob` cancel, both of which resolve to `None` so the caller
./crates/blit-daemon/src/service/core.rs:1324:/// transfer that completed at the same instant `CancelJob` fired its
./crates/blit-daemon/src/service/core.rs:1325:/// token was mis-recorded as "cancelled via CancelJob" despite having
./crates/blit-daemon/src/service/core.rs:1352:/// row's `CancelJob` token via [`resolve_transfer_outcome`].
./crates/blit-daemon/src/service/core.rs:1358:/// unobservable work that `CancelJob` also refused to touch
./crates/blit-daemon/src/service/core.rs:1378:/// - cancel token fired → `(false, "cancelled via CancelJob")`, and the
./crates/blit-daemon/src/service/core.rs:1405:        // token means the cause was CancelJob; otherwise the client
./crates/blit-daemon/src/service/core.rs:1409:                .send(Err(Status::cancelled("transfer cancelled via CancelJob")))
./crates/blit-daemon/src/service/core.rs:1411:            (false, Some("cancelled via CancelJob".to_string()))
./crates/blit-daemon/src/service/core.rs:1419:/// `CancelJob` it emits a framed `SessionError{CANCELLED}` on the
./crates/blit-daemon/src/service/core.rs:1450:                    "transfer cancelled via CancelJob",
./crates/blit-daemon/src/service/core.rs:1453:            (false, Some("cancelled via CancelJob".to_string()))
./crates/blit-daemon/src/service/core.rs:1485:    /// instant `CancelJob` fired gets mis-recorded as cancelled.
./crates/blit-daemon/src/service/core.rs:1503:    /// `CancelJob` cancel — the fix must not make transfers
./crates/blit-daemon/src/service/core.rs:1511:            ready(()),         // CancelJob fired
./crates/blit-daemon/src/service/core.rs:1518:    /// otp-4a codex F1: a `CancelJob` on a served `Transfer` session
./crates/blit-daemon/src/service/core.rs:1519:    /// must reach the client as a framed `SessionError{CANCELLED}` on
./crates/blit-daemon/src/service/core.rs:1537:        assert_eq!(msg.as_deref(), Some("cancelled via CancelJob"));
./crates/blit-daemon/src/service/core.rs:1548:                "cancel must emit a framed CANCELLED SessionError"
./crates/blit-daemon/src/service/core.rs:1550:            other => panic!("expected a CANCELLED error frame, got {other:?}"),
./crates/blit-daemon/src/service/core.rs:1597:    /// handler as `(false, "cancelled via CancelJob")` and deliver a
./crates/blit-daemon/src/service/core.rs:1611:        assert_eq!(err.as_deref(), Some("cancelled via CancelJob"));
./crates/blit-daemon/src/service/core.rs:1820:            .cancel_job(Request::new(CancelJobRequest {
./crates/blit-daemon/src/service/core.rs:1835:        // D-2026-07-04-3: CancelJob dispatch fires the row token for
./crates/blit-daemon/src/service/core.rs:1847:                .cancel_job(Request::new(CancelJobRequest {
./crates/blit-daemon/src/service/core.rs:1855:                "{}: CancelJob must fire the row token",
./crates/blit-daemon/src/service/core.rs:1878:            .cancel_job(Request::new(CancelJobRequest {
./crates/blit-daemon/src/service/core.rs:1882:            .expect_err("a policy-gated kind must reject CancelJob");
./crates/blit-daemon/src/service/core.rs:1886:            "token must NOT be fired when CancelJob is unsupported"
./crates/blit-daemon/src/service/core.rs:1894:            .cancel_job(Request::new(CancelJobRequest {
./crates/blit-daemon/src/service/core.rs:1906:            .cancel_job(Request::new(CancelJobRequest {
./crates/blit-daemon/src/service/transfer.rs:1://! ONE_TRANSFER_PATH unified `Transfer` session — daemon side.
./crates/blit-daemon/src/service/transfer.rs:35:    ResolvedEndpoint, SessionEndpoint, SessionFault,
./crates/blit-daemon/src/service/transfer.rs:46:fn status_to_fault(status: Status) -> SessionFault {
./crates/blit-daemon/src/service/transfer.rs:52:    SessionFault::refusal(code, status.message().to_string())
./crates/blit-daemon/src/service/transfer.rs:118:                .downcast_ref::<SessionFault>()
./crates/blit-cli/tests/remote_remote.rs:459:        _: tonic::Request<blit_core::generated::CancelJobRequest>,
./crates/blit-cli/tests/remote_remote.rs:460:    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
./crates/blit-cli/tests/remote_remote.rs:622:        _: tonic::Request<blit_core::generated::CancelJobRequest>,
./crates/blit-cli/tests/remote_remote.rs:623:    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
./crates/blit-daemon/src/service/push/data_plane.rs:9:use blit_core::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
./crates/blit-daemon/src/service/push/data_plane.rs:196:    // TimedOut after TRANSFER_STALL_TIMEOUT of no progress.
./crates/blit-daemon/src/service/push/data_plane.rs:1074:/// by `TRANSFER_STALL_TIMEOUT` rather than holding the receive worker
./crates/blit-daemon/src/service/push/data_plane.rs:1085:    let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
./crates/blit-daemon/src/service/push/data_plane.rs:1119:    /// with the production `TRANSFER_STALL_TIMEOUT` — so a future
./crates/blit-daemon/src/service/push/data_plane.rs:1149:        tokio::time::advance(TRANSFER_STALL_TIMEOUT + Duration::from_secs(1)).await;
./crates/blit-daemon/src/service/push/control.rs:815:    //! running with no owner — unreachable by `CancelJob`. This pins
./crates/blit-core/tests/transfer_session_roles.rs:8://! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1) in its first
./crates/blit-core/tests/transfer_session_roles.rs:28:    HelloConfig, SessionEndpoint, SessionFault, SourceSessionConfig, CONTRACT_VERSION,
./crates/blit-core/tests/transfer_session_roles.rs:199:fn fault_of(err: &eyre::Report) -> &SessionFault {
./crates/blit-core/tests/transfer_session_roles.rs:200:    err.downcast_ref::<SessionFault>()
./crates/blit-core/tests/transfer_session_roles.rs:201:        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
./crates/blit-cli/tests/jobs_lifecycle.rs:10://! CancelJob dispatch on for attached push/pull_sync, so exit 2 no
./crates/blit-cli/tests/jobs_lifecycle.rs:444:        _: tonic::Request<blit_core::generated::CancelJobRequest>,
./crates/blit-cli/tests/jobs_lifecycle.rs:445:    ) -> Result<tonic::Response<blit_core::generated::CancelJobResponse>, tonic::Status> {
./crates/blit-core/tests/pull_sync_with_spec_wire.rs:180:        _: Request<blit_core::generated::CancelJobRequest>,
./crates/blit-core/tests/pull_sync_with_spec_wire.rs:181:    ) -> Result<Response<blit_core::generated::CancelJobResponse>, Status> {
./crates/blit-core/tests/pull_sync_with_spec_wire.rs:608:        _: Request<blit_core::generated::CancelJobRequest>,
./crates/blit-core/tests/pull_sync_with_spec_wire.rs:609:    ) -> Result<Response<blit_core::generated::CancelJobResponse>, Status> {
./crates/blit-cli/src/jobs.rs:3:use blit_app::admin::jobs::{CancelJobOutcome, WatchSnapshot};
./crates/blit-cli/src/jobs.rs:57:/// Map [`CancelJobOutcome`] to the contract's exit codes.
./crates/blit-cli/src/jobs.rs:60:pub(crate) fn cancel_exit_code(outcome: &CancelJobOutcome) -> ExitCode {
./crates/blit-cli/src/jobs.rs:62:        CancelJobOutcome::Cancelled { .. } => ExitCode::SUCCESS,
./crates/blit-cli/src/jobs.rs:63:        CancelJobOutcome::NotFound { .. } => ExitCode::from(1),
./crates/blit-cli/src/jobs.rs:64:        CancelJobOutcome::Unsupported { .. } => ExitCode::from(2),
./crates/blit-cli/src/jobs.rs:560:fn print_cancel_json(outcome: &CancelJobOutcome) {
./crates/blit-cli/src/jobs.rs:563:        CancelJobOutcome::Cancelled { transfer_id } => json!({
./crates/blit-cli/src/jobs.rs:567:        CancelJobOutcome::NotFound { transfer_id } => json!({
./crates/blit-cli/src/jobs.rs:571:        CancelJobOutcome::Unsupported {
./crates/blit-cli/src/jobs.rs:586:fn print_cancel_human(remote: &RemoteEndpoint, outcome: &CancelJobOutcome) {
./crates/blit-cli/src/jobs.rs:588:        CancelJobOutcome::Cancelled { transfer_id } => {
./crates/blit-cli/src/jobs.rs:591:        CancelJobOutcome::NotFound { transfer_id } => {
./crates/blit-cli/src/jobs.rs:597:        CancelJobOutcome::Unsupported {
./crates/blit-cli/src/jobs.rs:896:        let cancelled = CancelJobOutcome::Cancelled {
./crates/blit-cli/src/jobs.rs:899:        let not_found = CancelJobOutcome::NotFound {
./crates/blit-cli/src/jobs.rs:902:        let unsupported = CancelJobOutcome::Unsupported {
./crates/blit-core/src/remote/pull.rs:1904:    use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
./crates/blit-core/src/remote/pull.rs:1934:    // goes silent (no bytes for TRANSFER_STALL_TIMEOUT) fails fast with a
./crates/blit-core/src/remote/pull.rs:1944:    let mut stream = StallGuard::new(stream, TRANSFER_STALL_TIMEOUT);
./crates/blit-core/src/transfer_session/mod.rs:2://! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
./crates/blit-core/src/transfer_session/mod.rs:43:use crate::remote::transfer::stall_guard::TRANSFER_STALL_TIMEOUT;
./crates/blit-core/src/transfer_session/mod.rs:140:pub struct SessionFault {
./crates/blit-core/src/transfer_session/mod.rs:153:impl SessionFault {
./crates/blit-core/src/transfer_session/mod.rs:206:impl fmt::Display for SessionFault {
./crates/blit-core/src/transfer_session/mod.rs:212:impl std::error::Error for SessionFault {}
./crates/blit-core/src/transfer_session/mod.rs:217:fn fault_from_report(report: eyre::Report) -> SessionFault {
./crates/blit-core/src/transfer_session/mod.rs:218:    match report.downcast::<SessionFault>() {
./crates/blit-core/src/transfer_session/mod.rs:220:        Err(other) => SessionFault::internal(format!("{other:#}")),
./crates/blit-core/src/transfer_session/mod.rs:228:fn error_frame(fault: &SessionFault) -> TransferFrame {
./crates/blit-core/src/transfer_session/mod.rs:269:/// so the daemon dispatcher can emit `CANCELLED` when a `CancelJob`
./crates/blit-core/src/transfer_session/mod.rs:286:type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;
./crates/blit-core/src/transfer_session/mod.rs:305:/// `tonic::Status` errors to [`SessionFault`], so blit-core stays free
./crates/blit-core/src/transfer_session/mod.rs:312:        -> Pin<Box<dyn Future<Output = std::result::Result<ResolvedEndpoint, SessionFault>> + Send>>
./crates/blit-core/src/transfer_session/mod.rs:328:fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
./crates/blit-core/src/transfer_session/mod.rs:330:        return Err(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:339:        return Err(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:346:fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
./crates/blit-core/src/transfer_session/mod.rs:348:        return Err(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:353:        return Err(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:405:                SessionFault::protocol_violation(format!(
./crates/blit-core/src/transfer_session/mod.rs:417:        let fault = SessionFault {
./crates/blit-core/src/transfer_session/mod.rs:440:                        SessionFault::protocol_violation(format!(
./crates/blit-core/src/transfer_session/mod.rs:461:                        SessionFault::protocol_violation(format!(
./crates/blit-core/src/transfer_session/mod.rs:476:                    SessionFault::protocol_violation(format!(
./crates/blit-core/src/transfer_session/mod.rs:504:                                SessionFault::read_only(
./crates/blit-core/src/transfer_session/mod.rs:557:        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
./crates/blit-core/src/transfer_session/mod.rs:560:            SessionFault::protocol_violation("frame with empty oneof"),
./crates/blit-core/src/transfer_session/mod.rs:562:        None => Err(eyre::Report::new(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:570:async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
./crates/blit-core/src/transfer_session/mod.rs:592:    Fault(SessionFault),
./crates/blit-core/src/transfer_session/mod.rs:646:    match source_send_half(
./crates/blit-core/src/transfer_session/mod.rs:683:                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:689:                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
./crates/blit-core/src/transfer_session/mod.rs:699:                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:717:                                SessionFault::protocol_violation(format!(
./crates/blit-core/src/transfer_session/mod.rs:733:                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:751:                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
./crates/blit-core/src/transfer_session/mod.rs:755:                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:764:async fn source_send_half(
./crates/blit-core/src/transfer_session/mod.rs:786:                eyre::Report::new(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:915:                return Err(eyre::Report::new(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:939:    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
./crates/blit-core/src/transfer_session/mod.rs:940:    // the served session frames `SessionError{CANCELLED}`, and the source
./crates/blit-core/src/transfer_session/mod.rs:945:    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
./crates/blit-core/src/transfer_session/mod.rs:950:    //     stall window (`prefer_peer_fault`).
./crates/blit-core/src/transfer_session/mod.rs:954:            fault = recv_peer_fault(&mut events) => {
./crates/blit-core/src/transfer_session/mod.rs:959:                    return Err(prefer_peer_fault(&mut events, dp_err).await);
./crates/blit-core/src/transfer_session/mod.rs:972:        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:976:            SessionFault::protocol_violation("duplicate NeedComplete"),
./crates/blit-core/src/transfer_session/mod.rs:979:            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
./crates/blit-core/src/transfer_session/mod.rs:981:        None => Err(eyre::Report::new(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:1034:                return Err(eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:1045:                return Err(eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:1054:                eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:1079:        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:1141:                return Err(eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:1146:                return Err(eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:1151:                return Err(eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/mod.rs:1156:                return Err(eyre::Report::new(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:1166:/// data-plane drain (otp-4b-3): a mid-transfer `SessionError` (e.g. a
./crates/blit-core/src/transfer_session/mod.rs:1167:/// `CancelJob` → `CANCELLED`) must abort the send and surface as the
./crates/blit-core/src/transfer_session/mod.rs:1172:async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
./crates/blit-core/src/transfer_session/mod.rs:1183:/// *symptom* of a peer abort — within `TRANSFER_STALL_TIMEOUT` the peer
./crates/blit-core/src/transfer_session/mod.rs:1187:async fn prefer_peer_fault(
./crates/blit-core/src/transfer_session/mod.rs:1191:    match tokio::time::timeout(TRANSFER_STALL_TIMEOUT, recv_peer_fault(events)).await {
./crates/blit-core/src/transfer_session/mod.rs:1347:                return Err(eyre::Report::new(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:1368:    eyre::Report::new(SessionFault::protocol_violation(message))
./crates/blit-core/src/transfer_session/mod.rs:1453:                return Err(eyre::Report::new(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:1650:                return Err(eyre::Report::new(SessionFault::from_wire(err)));
./crates/blit-core/src/transfer_session/mod.rs:1753:        SessionFault::protocol_violation(format!(
./crates/blit-core/src/transfer_session/mod.rs:1803:                    return Err(eyre::Report::new(SessionFault::internal(format!(
./crates/blit-core/src/transfer_session/mod.rs:1862:                return Err(eyre::Report::new(SessionFault::internal(
./crates/blit-core/src/transfer_session/mod.rs:1914:    /// otp-4b-3: a data-plane break during the drain prefers the peer's
./crates/blit-core/src/transfer_session/mod.rs:1916:    /// `SessionError{CANCELLED}` on the control lane, `prefer_peer_fault`
./crates/blit-core/src/transfer_session/mod.rs:1921:    async fn prefer_peer_fault_prefers_a_framed_fault() {
./crates/blit-core/src/transfer_session/mod.rs:1923:        // The peer framed CANCELLED on the control lane before we ask.
./crates/blit-core/src/transfer_session/mod.rs:1924:        tx.send(SourceEvent::Fault(SessionFault {
./crates/blit-core/src/transfer_session/mod.rs:1926:            message: "transfer cancelled via CancelJob".into(),
./crates/blit-core/src/transfer_session/mod.rs:1933:        let dp_err = eyre::Report::new(SessionFault::refusal(
./crates/blit-core/src/transfer_session/mod.rs:1937:        let chosen = prefer_peer_fault(&mut rx, dp_err).await;
./crates/blit-core/src/transfer_session/mod.rs:1939:            .downcast_ref::<SessionFault>()
./crates/blit-core/src/transfer_session/mod.rs:1940:            .expect("a SessionFault");
./crates/blit-core/src/transfer_session/mod.rs:1944:            "the framed CANCELLED must win over the data-plane break"
./crates/blit-core/src/transfer_session/mod.rs:1950:        let fault = SessionFault {
./crates/blit-core/src/transfer_session/mod.rs:1958:        let back = SessionFault::from_wire(wire);
./crates/blit-core/src/transfer_session/data_plane.rs:8://! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
./crates/blit-core/src/transfer_session/data_plane.rs:49:use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
./crates/blit-core/src/transfer_session/data_plane.rs:55:use super::SessionFault;
./crates/blit-core/src/transfer_session/data_plane.rs:65:    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
./crates/blit-core/src/transfer_session/data_plane.rs:318:        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
./crates/blit-core/src/transfer_session/data_plane.rs:612:            eyre::Report::new(SessionFault::internal("data plane already finished"))
./crates/blit-core/src/transfer_session/data_plane.rs:676:            Err(eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/data_plane.rs:700:                return Err(eyre::Report::new(SessionFault::protocol_violation(
./crates/blit-core/src/transfer_session/data_plane.rs:773:        // Off-need-list path faults with a SessionFault.
./crates/blit-core/src/transfer_session/data_plane.rs:779:            err.downcast_ref::<SessionFault>().is_some(),
./crates/blit-core/src/transfer_session/data_plane.rs:780:            "off-list rejection is a SessionFault: {err:#}"
./crates/blit-core/src/remote/transfer/stall_guard.rs:27://!   trips after `TRANSFER_STALL_TIMEOUT` of no successful write
./crates/blit-core/src/remote/transfer/stall_guard.rs:71:pub const TRANSFER_STALL_TIMEOUT: Duration = Duration::from_secs(30);
./crates/blit-core/src/remote/transfer/stall_guard.rs:134:/// `io::ErrorKind::TimedOut` after `TRANSFER_STALL_TIMEOUT` of no
./crates/blit-core/src/remote/transfer/data_plane.rs:11:use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
./crates/blit-core/src/remote/transfer/data_plane.rs:53:/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
./crates/blit-core/src/remote/transfer/data_plane.rs:82:    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
./crates/blit-core/src/remote/transfer/data_plane.rs:176:            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),

codex
The commit is at `HEAD` and touches the expected three files. I’m reading the control-lane event flow and data-plane finish semantics now, because the risk is mostly in whether the new race can consume a non-fault event or mask the wrong error.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '990,1210p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   990	#[allow(clippy::too_many_arguments)]
   991	async fn drain_ready_source_events(
   992	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
   993	    pending: &mut Vec<FileHeader>,
   994	    need_complete: &mut bool,
   995	    needed_bytes: &mut u64,
   996	    needed_count: &mut usize,
   997	    data_plane: Option<&data_plane::SourceDataPlane>,
   998	    tx: &mut Box<dyn FrameTx>,
   999	    pending_resize: &mut Option<data_plane::PendingResize>,
  1000	) -> Result<()> {
  1001	    while let Ok(event) = events.try_recv() {
  1002	        process_source_event(
  1003	            event,
  1004	            pending,
  1005	            need_complete,
  1006	            needed_bytes,
  1007	            needed_count,
  1008	            data_plane,
  1009	            tx,
  1010	            pending_resize,
  1011	        )
  1012	        .await?;
  1013	    }
  1014	    Ok(())
  1015	}
  1016	
  1017	/// Handle one source event. Needs accumulate into `pending` and the
  1018	/// shape totals; a resize ack dials its epoch-N socket and proposes the
  1019	/// next ADD (the one-per-epoch ramp).
  1020	#[allow(clippy::too_many_arguments)]
  1021	async fn process_source_event(
  1022	    event: SourceEvent,
  1023	    pending: &mut Vec<FileHeader>,
  1024	    need_complete: &mut bool,
  1025	    needed_bytes: &mut u64,
  1026	    needed_count: &mut usize,
  1027	    data_plane: Option<&data_plane::SourceDataPlane>,
  1028	    tx: &mut Box<dyn FrameTx>,
  1029	    pending_resize: &mut Option<data_plane::PendingResize>,
  1030	) -> Result<()> {
  1031	    match event {
  1032	        SourceEvent::Need(header) => {
  1033	            if *need_complete {
  1034	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1035	                    format!("need for '{}' after NeedComplete", header.relative_path),
  1036	                )));
  1037	            }
  1038	            *needed_bytes = needed_bytes.saturating_add(header.size);
  1039	            *needed_count += 1;
  1040	            pending.push(header);
  1041	            Ok(())
  1042	        }
  1043	        SourceEvent::NeedComplete => {
  1044	            if *need_complete {
  1045	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1046	                    "duplicate NeedComplete",
  1047	                )));
  1048	            }
  1049	            *need_complete = true;
  1050	            Ok(())
  1051	        }
  1052	        SourceEvent::ResizeAck(ack) => {
  1053	            let dp = data_plane.ok_or_else(|| {
  1054	                eyre::Report::new(SessionFault::protocol_violation(
  1055	                    "DataPlaneResizeAck on a session with no data plane",
  1056	                ))
  1057	            })?;
  1058	            // Match the ack to the in-flight proposal; stale/unsolicited
  1059	            // acks (wrong epoch, or none pending) are ignored, matching
  1060	            // old push. `take()` + restore keeps the borrow simple.
  1061	            let pending_r = match pending_resize.take() {
  1062	                Some(p) if p.epoch == ack.epoch => p,
  1063	                restored => {
  1064	                    *pending_resize = restored;
  1065	                    return Ok(());
  1066	                }
  1067	            };
  1068	            if ack.accepted {
  1069	                dp.add_stream(&pending_r.sub_token).await?;
  1070	                dp.dial()
  1071	                    .resize_settled(pending_r.epoch, pending_r.target_streams as usize, true);
  1072	            } else {
  1073	                dp.dial()
  1074	                    .resize_settled(pending_r.epoch, dp.dial().live_streams(), false);
  1075	            }
  1076	            // Ramp one stream per accepted epoch: propose the next ADD.
  1077	            maybe_propose_resize(dp, tx, *needed_bytes, *needed_count, pending_resize).await
  1078	        }
  1079	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
  1080	            "TransferSummary before SourceDone",
  1081	        ))),
  1082	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
  1083	    }
  1084	}
  1085	
  1086	/// Propose one shape-correction resize (`DataPlaneResize{ADD}`) toward
  1087	/// the stream count the accumulated need list implies, if none is in
  1088	/// flight. A no-op when the shape wants no more than the live count (the
  1089	/// dial returns `None`). Sends the frame and records the in-flight
  1090	/// proposal for the ack to match.
  1091	async fn maybe_propose_resize(
  1092	    dp: &data_plane::SourceDataPlane,
  1093	    tx: &mut Box<dyn FrameTx>,
  1094	    needed_bytes: u64,
  1095	    needed_count: usize,
  1096	    pending_resize: &mut Option<data_plane::PendingResize>,
  1097	) -> Result<()> {
  1098	    if pending_resize.is_some() {
  1099	        return Ok(());
  1100	    }
  1101	    if let Some(proposal) = dp.propose_resize(needed_bytes, needed_count)? {
  1102	        tx.send(frame(Frame::Resize(DataPlaneResize {
  1103	            op: DataPlaneResizeOp::Add as i32,
  1104	            epoch: proposal.epoch,
  1105	            target_stream_count: proposal.target_streams,
  1106	            sub_token: proposal.sub_token.clone(),
  1107	        })))
  1108	        .await?;
  1109	        *pending_resize = Some(proposal);
  1110	    }
  1111	    Ok(())
  1112	}
  1113	
  1114	/// Block for the ack of the one in-flight resize and dial its socket (or
  1115	/// settle it refused). Does NOT propose further — it resolves exactly the
  1116	/// pending proposal so the destination's armed slot is consumed before we
  1117	/// finish the data plane.
  1118	async fn resolve_in_flight_resize(
  1119	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1120	    dp: &data_plane::SourceDataPlane,
  1121	    pending: data_plane::PendingResize,
  1122	) -> Result<()> {
  1123	    loop {
  1124	        match events.recv().await {
  1125	            Some(SourceEvent::ResizeAck(ack)) if ack.epoch == pending.epoch => {
  1126	                if ack.accepted {
  1127	                    dp.add_stream(&pending.sub_token).await?;
  1128	                    dp.dial()
  1129	                        .resize_settled(pending.epoch, pending.target_streams as usize, true);
  1130	                } else {
  1131	                    dp.dial()
  1132	                        .resize_settled(pending.epoch, dp.dial().live_streams(), false);
  1133	                }
  1134	                return Ok(());
  1135	            }
  1136	            // A stale ack for an already-settled epoch: ignore, keep
  1137	            // waiting for ours.
  1138	            Some(SourceEvent::ResizeAck(_)) => continue,
  1139	            Some(SourceEvent::Fault(fault)) => return Err(eyre::Report::new(fault)),
  1140	            Some(SourceEvent::Need(h)) => {
  1141	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1142	                    format!("need for '{}' after NeedComplete", h.relative_path),
  1143	                )))
  1144	            }
  1145	            Some(SourceEvent::NeedComplete) => {
  1146	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1147	                    "duplicate NeedComplete",
  1148	                )))
  1149	            }
  1150	            Some(SourceEvent::Summary(_)) => {
  1151	                return Err(eyre::Report::new(SessionFault::protocol_violation(
  1152	                    "TransferSummary before SourceDone",
  1153	                )))
  1154	            }
  1155	            None => {
  1156	                return Err(eyre::Report::new(SessionFault::internal(
  1157	                    "source receive half ended with a resize in flight",
  1158	                )))
  1159	            }
  1160	        }
  1161	    }
  1162	}
  1163	
  1164	/// Await the next peer-framed fault the receive half forwards on the
  1165	/// control lane, ignoring any non-fault event. Used to race the
  1166	/// data-plane drain (otp-4b-3): a mid-transfer `SessionError` (e.g. a
  1167	/// `CancelJob` → `CANCELLED`) must abort the send and surface as the
  1168	/// fault. Parks forever once the channel closes with no fault so the
  1169	/// data-plane future it races decides the outcome instead — during the
  1170	/// drain the receive half only ever forwards a fault (SourceDone has not
  1171	/// gone out, so no summary; the resize was already resolved).
  1172	async fn recv_peer_fault(events: &mut mpsc::UnboundedReceiver<SourceEvent>) -> SessionFault {
  1173	    loop {
  1174	        match events.recv().await {
  1175	            Some(SourceEvent::Fault(fault)) => return fault,
  1176	            Some(_) => continue,
  1177	            None => std::future::pending().await,
  1178	        }
  1179	    }
  1180	}
  1181	
  1182	/// A data-plane operation failed mid-transfer. The break is usually the
  1183	/// *symptom* of a peer abort — within `TRANSFER_STALL_TIMEOUT` the peer
  1184	/// (which runs the same stall guard on its receive workers) always frames
  1185	/// the real reason on the control lane. Prefer that framed fault; fall
  1186	/// back to the raw data-plane error if none arrives in that window.
  1187	async fn prefer_peer_fault(
  1188	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
  1189	    dp_err: eyre::Report,
  1190	) -> eyre::Report {
  1191	    match tokio::time::timeout(TRANSFER_STALL_TIMEOUT, recv_peer_fault(events)).await {
  1192	        Ok(fault) => eyre::Report::new(fault),
  1193	        Err(_) => dp_err,
  1194	    }
  1195	}
  1196	
  1197	/// Plan one batch of needed headers with the engine planner and emit
  1198	/// the resulting payload records per the in-stream grammar.
  1199	async fn send_payload_records(
  1200	    tx: &mut Box<dyn FrameTx>,
  1201	    source: &Arc<dyn TransferSource>,
  1202	    plan_options: PlanOptions,
  1203	    batch: Vec<FileHeader>,
  1204	    read_buf: &mut [u8],
  1205	) -> Result<()> {
  1206	    let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
  1207	    for payload in payloads {
  1208	        match source.prepare_payload(payload).await? {
  1209	            PreparedPayload::File(header) => {
  1210	                tx.send(frame(Frame::FileBegin(header.clone()))).await?;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '580,990p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   580	/// Events the source's receive half forwards to its send half. The
   581	/// channel is unbounded but bounded by construction: every `Need`
   582	/// consumes a distinct sent-manifest entry (unknown or repeated paths
   583	/// fault the session), so the queue never exceeds the source's own
   584	/// manifest size — the contract's bounded-buffering rule holds.
   585	enum SourceEvent {
   586	    Need(FileHeader),
   587	    NeedComplete,
   588	    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
   589	    /// send half dials the epoch-N socket on `accepted`.
   590	    ResizeAck(DataPlaneResizeAck),
   591	    Summary(TransferSummary),
   592	    Fault(SessionFault),
   593	}
   594	
   595	/// Run the SOURCE role of one transfer session over `transport`.
   596	/// Returns the destination-computed `TransferSummary` (contract: the
   597	/// end that wrote the bytes is the end that attests to them).
   598	pub async fn run_source(
   599	    cfg: SourceSessionConfig,
   600	    transport: FrameTransport,
   601	    source: Arc<dyn TransferSource>,
   602	) -> Result<TransferSummary> {
   603	    let mut transport = transport;
   604	    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
   605	        // Own-config coherence: a source initiator declares SOURCE.
   606	        let declared = TransferRole::try_from(open.initiator_role);
   607	        if declared != Ok(TransferRole::Source) {
   608	            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
   609	        }
   610	        if let Err(fault) = source_open_validator(open) {
   611	            eyre::bail!("run_source initiator config unsupported: {fault}");
   612	        }
   613	    }
   614	
   615	    let negotiated = establish(
   616	        &mut transport,
   617	        &cfg.hello,
   618	        &cfg.endpoint,
   619	        TransferRole::Source,
   620	        &source_open_validator,
   621	        // A SOURCE responder's endpoint resolution (module→root for a
   622	        // daemon-send) lands with otp-5; otp-4a's daemon is always the
   623	        // DESTINATION responder, so the source never resolves here.
   624	        None,
   625	    )
   626	    .await?;
   627	
   628	    let (mut tx, rx) = transport.split();
   629	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
   630	    // Set by the send half the moment ManifestComplete goes out. On
   631	    // an ordered transport, a NeedComplete arriving while this is
   632	    // still false is provably premature — the peer cannot have
   633	    // received what we have not sent (contract: NeedComplete only
   634	    // after ManifestComplete received + all entries diffed).
   635	    let manifest_sent = Arc::new(AtomicBool::new(false));
   636	    let (event_tx, event_rx) = mpsc::unbounded_channel();
   637	    // AbortOnDrop: an early error return below must abort the receive
   638	    // half instead of leaking it (same rationale as design-2 / w4-1).
   639	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
   640	        rx,
   641	        Arc::clone(&sent),
   642	        Arc::clone(&manifest_sent),
   643	        event_tx,
   644	    )));
   645	
   646	    match source_send_half(
   647	        &cfg,
   648	        &negotiated,
   649	        &mut tx,
   650	        source,
   651	        sent,
   652	        &manifest_sent,
   653	        event_rx,
   654	    )
   655	    .await
   656	    {
   657	        Ok(summary) => Ok(summary),
   658	        Err(report) => {
   659	            let mut fault = fault_from_report(report);
   660	            if !fault.peer_notified {
   661	                let _ = tx.send(error_frame(&fault)).await;
   662	                fault.peer_notified = true;
   663	            }
   664	            Err(eyre::Report::new(fault))
   665	        }
   666	    }
   667	}
   668	
   669	/// Receive half of the source driver: drains the transport for the
   670	/// whole session so destination sends can never deadlock against a
   671	/// blocked source send, and routes the destination lane to the send
   672	/// half. Terminates on summary, error, close, or violation.
   673	async fn source_recv_half(
   674	    mut rx: Box<dyn FrameRx>,
   675	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   676	    manifest_sent: Arc<AtomicBool>,
   677	    events: mpsc::UnboundedSender<SourceEvent>,
   678	) {
   679	    loop {
   680	        let received = match rx.recv().await {
   681	            Ok(Some(f)) => f,
   682	            Ok(None) => {
   683	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
   684	                    "peer closed before TransferSummary",
   685	                )));
   686	                return;
   687	            }
   688	            Err(err) => {
   689	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
   690	                    "transport receive failed: {err:#}"
   691	                ))));
   692	                return;
   693	            }
   694	        };
   695	        match received.frame {
   696	            Some(Frame::NeedBatch(batch)) => {
   697	                for entry in batch.entries {
   698	                    if entry.resume {
   699	                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   700	                            format!(
   701	                                "resume-flagged need for '{}' in a session opened without resume",
   702	                                entry.relative_path
   703	                            ),
   704	                        )));
   705	                        return;
   706	                    }
   707	                    let header = sent
   708	                        .lock()
   709	                        .expect("sent-manifest lock poisoned")
   710	                        .remove(&entry.relative_path);
   711	                    match header {
   712	                        Some(h) => {
   713	                            let _ = events.send(SourceEvent::Need(h));
   714	                        }
   715	                        None => {
   716	                            let _ = events.send(SourceEvent::Fault(
   717	                                SessionFault::protocol_violation(format!(
   718	                                    "need for unknown or already-needed path '{}'",
   719	                                    entry.relative_path
   720	                                )),
   721	                            ));
   722	                            return;
   723	                        }
   724	                    }
   725	                }
   726	            }
   727	            Some(Frame::NeedComplete(_)) => {
   728	                if !manifest_sent.load(Ordering::Acquire) {
   729	                    // Fail fast at arrival time (otp-3 codex F2): the
   730	                    // event queue would otherwise let an early
   731	                    // NeedComplete be processed late and pass as
   732	                    // legitimate.
   733	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   734	                        "NeedComplete before the source's ManifestComplete",
   735	                    )));
   736	                    return;
   737	                }
   738	                let _ = events.send(SourceEvent::NeedComplete);
   739	            }
   740	            Some(Frame::ResizeAck(ack)) => {
   741	                // The destination's response to a shape-resize proposal
   742	                // (otp-4b-2). Forward it to the send half, which owns the
   743	                // dial and dials the epoch-N socket on `accepted`.
   744	                let _ = events.send(SourceEvent::ResizeAck(ack));
   745	            }
   746	            Some(Frame::Summary(summary)) => {
   747	                let _ = events.send(SourceEvent::Summary(summary));
   748	                return;
   749	            }
   750	            Some(Frame::Error(err)) => {
   751	                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
   752	                return;
   753	            }
   754	            other => {
   755	                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   756	                    format!("{} on the source's receive lane", frame_name(&other)),
   757	                )));
   758	                return;
   759	            }
   760	        }
   761	    }
   762	}
   763	
   764	async fn source_send_half(
   765	    cfg: &SourceSessionConfig,
   766	    negotiated: &Negotiated,
   767	    tx: &mut Box<dyn FrameTx>,
   768	    source: Arc<dyn TransferSource>,
   769	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   770	    manifest_sent: &AtomicBool,
   771	    mut events: mpsc::UnboundedReceiver<SourceEvent>,
   772	) -> Result<TransferSummary> {
   773	    let mut pending: Vec<FileHeader> = Vec::new();
   774	    let mut need_complete = false;
   775	
   776	    // Data plane (otp-4b): dial the granted TCP sockets up front —
   777	    // BEFORE streaming the manifest — so the destination's accept loop
   778	    // (armed the moment it sent SessionAccept) sees the connections
   779	    // promptly rather than waiting out its bounded-accept timeout while
   780	    // a long manifest streams. The sockets sit idle (keepalive covers
   781	    // that) until payloads are queued below. `None` = the in-stream
   782	    // carrier (fallback), which needs no early setup.
   783	    let mut data_plane = match &negotiated.accept.data_plane {
   784	        Some(grant) => {
   785	            let host = cfg.data_plane_host.as_deref().ok_or_else(|| {
   786	                eyre::Report::new(SessionFault::internal(
   787	                    "responder granted a TCP data plane but this initiator has no host to dial",
   788	                ))
   789	            })?;
   790	            Some(
   791	                data_plane::dial_source_data_plane(
   792	                    host,
   793	                    grant,
   794	                    negotiated.accept.receiver_capacity.as_ref(),
   795	                    Arc::clone(&source),
   796	                )
   797	                .await?,
   798	            )
   799	        }
   800	        None => None,
   801	    };
   802	
   803	    // sf-2 shape correction (otp-4b-2): running totals of the need list,
   804	    // fed to the shape table so the SOURCE grows the data-plane stream
   805	    // count as the workload's shape becomes known. Append-only (a need is
   806	    // counted once, when it arrives), and the in-flight resize record the
   807	    // ack is matched against (at most one — the dial enforces it).
   808	    let mut needed_bytes: u64 = 0;
   809	    let mut needed_count: usize = 0;
   810	    let mut pending_resize: Option<data_plane::PendingResize> = None;
   811	
   812	    // Streaming manifest: entries go out as enumeration produces them
   813	    // (immediate start in every direction — plan §Design 2). The open
   814	    // carries no source path: the source end owns its local endpoint.
   815	    let _ = &negotiated.open;
   816	    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
   817	    let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
   818	    while let Some(header) = header_rx.recv().await {
   819	        sent.lock()
   820	            .expect("sent-manifest lock poisoned")
   821	            .insert(header.relative_path.clone(), header.clone());
   822	        tx.send(frame(Frame::ManifestEntry(header))).await?;
   823	        // Faults detected by the receive half abort the stream now,
   824	        // not after the full scan; needs just accumulate. (Resize acks
   825	        // cannot arrive yet — none is proposed before the payload phase.)
   826	        drain_ready_source_events(
   827	            &mut events,
   828	            &mut pending,
   829	            &mut need_complete,
   830	            &mut needed_bytes,
   831	            &mut needed_count,
   832	            data_plane.as_ref(),
   833	            tx,
   834	            &mut pending_resize,
   835	        )
   836	        .await?;
   837	    }
   838	    let scanned = scan_handle
   839	        .await
   840	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
   841	    let scan_complete = unreadable
   842	        .lock()
   843	        .expect("unreadable list lock poisoned")
   844	        .is_empty();
   845	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
   846	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
   847	        scan_complete,
   848	    })))
   849	    .await?;
   850	    manifest_sent.store(true, Ordering::Release);
   851	
   852	    // Payload phase. The byte carrier is either the TCP data plane
   853	    // (dialed above) or the in-stream record grammar (fallback). Needs
   854	    // accumulated while a batch was being sent become the next planner
   855	    // batch (contract §Transport selection); payloads only flow after
   856	    // ManifestComplete.
   857	    // The in-stream carrier reuses one read buffer across records; the
   858	    // data plane owns its own pooled buffers, so skip that allocation.
   859	    let mut read_buf = if data_plane.is_none() {
   860	        vec![0u8; IN_STREAM_CHUNK]
   861	    } else {
   862	        Vec::new()
   863	    };
   864	    loop {
   865	        drain_ready_source_events(
   866	            &mut events,
   867	            &mut pending,
   868	            &mut need_complete,
   869	            &mut needed_bytes,
   870	            &mut needed_count,
   871	            data_plane.as_ref(),
   872	            tx,
   873	            &mut pending_resize,
   874	        )
   875	        .await?;
   876	        if !pending.is_empty() {
   877	            let batch = std::mem::take(&mut pending);
   878	            match &mut data_plane {
   879	                Some(dp) => {
   880	                    // sf-2: correct the stream count toward the shape the
   881	                    // accumulated need list implies before queueing this
   882	                    // batch (one ADD per epoch; a no-op while one is in
   883	                    // flight or the shape wants no more).
   884	                    maybe_propose_resize(dp, tx, needed_bytes, needed_count, &mut pending_resize)
   885	                        .await?;
   886	                    let payloads =
   887	                        diff_planner::plan_push_payloads(batch, source.root(), cfg.plan_options)?;
   888	                    dp.queue(payloads).await?;
   889	                }
   890	                None => {
   891	                    send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf)
   892	                        .await?;
   893	                }
   894	            }
   895	            continue;
   896	        }
   897	        if need_complete {
   898	            break;
   899	        }
   900	        match events.recv().await {
   901	            Some(event) => {
   902	                process_source_event(
   903	                    event,
   904	                    &mut pending,
   905	                    &mut need_complete,
   906	                    &mut needed_bytes,
   907	                    &mut needed_count,
   908	                    data_plane.as_ref(),
   909	                    tx,
   910	                    &mut pending_resize,
   911	                )
   912	                .await?;
   913	            }
   914	            None => {
   915	                return Err(eyre::Report::new(SessionFault::internal(
   916	                    "source receive half ended before NeedComplete",
   917	                )))
   918	            }
   919	        }
   920	    }
   921	
   922	    // A resize proposed on the last batch may still be in flight. Resolve
   923	    // it BEFORE finishing so the destination's armed slot is consumed by
   924	    // the dialed socket — an armed-but-never-dialed credential would hang
   925	    // its accept loop (which waits for every arm to be claimed). We do not
   926	    // propose further here: exactly the one in-flight resize is drained.
   927	    if let Some(dp) = &data_plane {
   928	        if let Some(pending) = pending_resize.take() {
   929	            resolve_in_flight_resize(&mut events, dp, pending).await?;
   930	        }
   931	    }
   932	
   933	    // Close the data plane BEFORE SourceDone so the destination's receive
   934	    // pipeline sees each socket's END record and completes; SourceDone on
   935	    // the control lane then lets the destination score and summarize.
   936	    //
   937	    // The drain is the byte-transfer phase's wall-time sink, so a
   938	    // mid-transfer cancel almost always lands here. Race it against a
   939	    // peer-framed fault on the control lane (otp-4b-3): a `CancelJob` on
   940	    // the served session frames `SessionError{CANCELLED}`, and the source
   941	    // must surface THAT — not the data-plane transport break it also
   942	    // causes. Two orderings, both covered:
   943	    //   * fault arrives while the drain is still pending (e.g. a worker
   944	    //     blocked reading a slow file, so the socket break never unblocks
   945	    //     it) → the `recv_peer_fault` arm wins; dropping the unfinished
   946	    //     `finish()` future drops the data plane, and its `AbortOnDrop`
   947	    //     stops the in-flight workers.
   948	    //   * the socket break makes `finish()` return `Err` first → prefer
   949	    //     the framed reason if the control lane delivers one within the
   950	    //     stall window (`prefer_peer_fault`).
   951	    if let Some(dp) = data_plane.take() {
   952	        tokio::select! {
   953	            biased;
   954	            fault = recv_peer_fault(&mut events) => {
   955	                return Err(eyre::Report::new(fault));
   956	            }
   957	            res = dp.finish() => {
   958	                if let Err(dp_err) = res {
   959	                    return Err(prefer_peer_fault(&mut events, dp_err).await);
   960	                }
   961	            }
   962	        }
   963	    }
   964	
   965	    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
   966	
   967	    // CLOSING: the destination is the scorer; the next event must be
   968	    // its summary (the receive half ends after forwarding it).
   969	    match events.recv().await {
   970	        Some(SourceEvent::Summary(summary)) => Ok(summary),
   971	        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
   972	        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
   973	            format!("need for '{}' after NeedComplete", h.relative_path),
   974	        ))),
   975	        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
   976	            SessionFault::protocol_violation("duplicate NeedComplete"),
   977	        )),
   978	        Some(SourceEvent::ResizeAck(_)) => Err(eyre::Report::new(
   979	            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
   980	        )),
   981	        None => Err(eyre::Report::new(SessionFault::internal(
   982	            "source receive half ended before TransferSummary",
   983	        ))),
   984	    }
   985	}
   986	
   987	/// Process every event ready right now (needs accumulating, resize acks
   988	/// dialing their epoch-N socket) without blocking. Called between
   989	/// manifest sends and at the top of the payload loop.
   990	#[allow(clippy::too_many_arguments)]

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Session-side TCP data-plane orchestration (otp-4b).
     2	//!
     3	//! The unified session reuses blit-core's data-plane byte plumbing —
     4	//! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
     5	//! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
     6	//! its OWN choreography here. The push-specific bind/arm/accept loop
     7	//! (`blit-daemon` push service) and the multi-stream send driver
     8	//! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
     9	//! deletes at cutover (otp-10), so nothing in this file calls into them.
    10	//!
    11	//! The RESPONDER (whichever end is DESTINATION for otp-4/-5) binds a
    12	//! listener, mints the tokens, grants them in `SessionAccept`, and
    13	//! accepts + receives; the INITIATOR (SOURCE here) dials, authenticates,
    14	//! and sends. Because the grant is issued before any manifest is seen,
    15	//! the zero-knowledge `initial_stream_proposal` is 1 — the session data
    16	//! plane always starts single-stream (otp-4b-1).
    17	//!
    18	//! otp-4b-2 adds mid-transfer growth: the SOURCE owns a [`TransferDial`]
    19	//! (bounded by the receiver's advertised capacity) and drives the sf-2
    20	//! shape correction — as the need list accumulates it re-runs the shape
    21	//! table and proposes `DataPlaneResize{ADD}` (one stream per epoch) on
    22	//! the control lane; the DESTINATION arms the credential, replies
    23	//! `DataPlaneResizeAck`, and accepts one more socket; the SOURCE dials
    24	//! the epoch-N socket and hands it to the running elastic pipeline via
    25	//! [`SinkControl::Add`]. The cheap-dial live tuner (chunk/prefetch) is
    26	//! still future work — otp-4b-2 moves only the stream count.
    27	
    28	use std::collections::HashSet;
    29	use std::path::{Path, PathBuf};
    30	use std::sync::{Arc, Mutex as StdMutex};
    31	
    32	use async_trait::async_trait;
    33	use eyre::Result;
    34	use tokio::io::AsyncReadExt;
    35	use tokio::net::{TcpListener, TcpStream};
    36	use tokio::sync::mpsc;
    37	use tokio::task::JoinSet;
    38	
    39	use crate::buffer::BufferPool;
    40	use crate::engine::{initial_stream_proposal, local_receiver_capacity, TransferDial};
    41	use crate::generated::{session_error::Code, CapacityProfile, DataPlaneGrant, FileHeader};
    42	use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
    43	use crate::remote::transfer::pipeline::execute_receive_pipeline;
    44	use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
    45	use crate::remote::transfer::socket::{
    46	    configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
    47	};
    48	use crate::remote::transfer::source::TransferSource;
    49	use crate::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
    50	use crate::remote::transfer::{
    51	    execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession, SinkControl,
    52	    SUB_TOKEN_LEN,
    53	};
    54	
    55	use super::SessionFault;
    56	
    57	/// The set of granted-but-not-yet-received needs, shared between the
    58	/// destination's control loop (which inserts each path before sending
    59	/// its `NeedBatch`) and the data-plane receive (which claims each path
    60	/// as its payload lands). Completion is an empty set — the same signal
    61	/// the in-stream carrier uses via its inline `outstanding.remove`.
    62	pub(super) type OutstandingNeeds = Arc<StdMutex<HashSet<String>>>;
    63	
    64	fn dp_fault(msg: impl Into<String>) -> eyre::Report {
    65	    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
    66	}
    67	
    68	// ---------------------------------------------------------------------------
    69	// Responder (DESTINATION) — bind, grant, accept, receive
    70	// ---------------------------------------------------------------------------
    71	
    72	/// A bound data-plane listener plus the credentials the responder
    73	/// advertises in its `SessionAccept`. Held by the responder driver
    74	/// across the handshake so the accept loop can run after establish.
    75	pub(super) struct ResponderDataPlane {
    76	    listener: TcpListener,
    77	    session_token: Vec<u8>,
    78	    epoch0_sub_token: Vec<u8>,
    79	    initial_streams: u32,
    80	    port: u16,
    81	}
    82	
    83	/// Bind a data-plane listener and mint credentials for the grant. Any
    84	/// failure (bind, addr, RNG) logs and returns `None` — the caller then
    85	/// issues a grant-less `SessionAccept` and the session falls back to the
    86	/// in-stream carrier (contract §Transport selection: a responder that
    87	/// cannot bind grants no data plane).
    88	pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
    89	    let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
    90	        Ok(listener) => listener,
    91	        Err(err) => {
    92	            log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
    93	            return None;
    94	        }
    95	    };
    96	    let port = match listener.local_addr() {
    97	        Ok(addr) => addr.port(),
    98	        Err(err) => {
    99	            log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
   100	            return None;
   101	        }
   102	    };
   103	    // Two independent 16-byte credentials (contract §Transport: a socket
   104	    // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
   105	    // is the fallible-RNG minter — a missing system RNG is an error, not
   106	    // a weaker credential.
   107	    let session_token = match generate_sub_token() {
   108	        Ok(token) => token,
   109	        Err(err) => {
   110	            log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
   111	            return None;
   112	        }
   113	    };
   114	    let epoch0_sub_token = match generate_sub_token() {
   115	        Ok(token) => token,
   116	        Err(err) => {
   117	            log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
   118	            return None;
   119	        }
   120	    };
   121	    // The grant is issued before any manifest is seen, so the proposal
   122	    // has zero knowledge: initial_streams == 1. All growth is via resize
   123	    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
   124	    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   125	    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
   126	    Some(ResponderDataPlane {
   127	        listener,
   128	        session_token,
   129	        epoch0_sub_token,
   130	        initial_streams,
   131	        port,
   132	    })
   133	}
   134	
   135	/// Aggregated destination-side receive result: the write outcome plus
   136	/// the number of data sockets accepted (epoch-0 + accepted resizes),
   137	/// which IS the settled live stream count this end observed. The sf-2
   138	/// pin reads it through [`super::DestinationOutcome::data_plane_streams`].
   139	pub(super) struct ReceiveTotals {
   140	    pub(super) outcome: SinkOutcome,
   141	    pub(super) streams: usize,
   142	}
   143	
   144	/// Live handle to a running responder data plane. The control loop arms
   145	/// resize credentials through [`Self::arm`] and joins the accept loop at
   146	/// `SourceDone` via [`Self::finish`].
   147	pub(super) struct ResponderDataPlaneRun {
   148	    arm_tx: mpsc::UnboundedSender<Vec<u8>>,
   149	    task: AbortOnDrop<Result<ReceiveTotals>>,
   150	    /// The `session_token` half of every socket credential (the control
   151	    /// loop does not need it, but keeping it here documents the shape).
   152	    #[allow(dead_code)]
   153	    session_token: Vec<u8>,
   154	    /// The receiver's advertised `max_streams` — the control loop refuses
   155	    /// a resize that would grow past it (defense in depth; the source's
   156	    /// dial already clamps to the same ceiling).
   157	    pub(super) ceiling: usize,
   158	}
   159	
   160	impl ResponderDataPlane {
   161	    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
   162	    pub(super) fn grant(&self) -> DataPlaneGrant {
   163	        DataPlaneGrant {
   164	            tcp_port: self.port as u32,
   165	            session_token: self.session_token.clone(),
   166	            initial_streams: self.initial_streams,
   167	            epoch0_sub_token: self.epoch0_sub_token.clone(),
   168	        }
   169	    }
   170	
   171	    /// The epoch-0 stream count this responder granted (always 1 — the
   172	    /// zero-knowledge proposal). The control loop seeds its `resize_live`
   173	    /// counter from it.
   174	    pub(super) fn initial_streams(&self) -> u32 {
   175	        self.initial_streams
   176	    }
   177	
   178	    /// Spawn the accept+receive loop and return a live handle. The loop
   179	    /// accepts the epoch-0 socket(s) immediately, then accepts one more
   180	    /// socket per armed resize credential until the control loop signals
   181	    /// `SourceDone` (drops the arm sender) and every receive worker has
   182	    /// drained its END. Runs concurrently with the control-stream diff
   183	    /// loop; the DESTINATION is the scorer, so it returns the totals.
   184	    pub(super) fn spawn(self, sink: Arc<dyn TransferSink>) -> ResponderDataPlaneRun {
   185	        let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   186	        let session_token = self.session_token.clone();
   187	        let (arm_tx, arm_rx) = mpsc::unbounded_channel::<Vec<u8>>();
   188	        let task = AbortOnDrop::new(tokio::spawn(self.accept_loop(sink, arm_rx)));
   189	        ResponderDataPlaneRun {
   190	            arm_tx,
   191	            task,
   192	            session_token,
   193	            ceiling,
   194	        }
   195	    }
   196	
   197	    async fn accept_loop(
   198	        self,
   199	        sink: Arc<dyn TransferSink>,
   200	        arm_rx: mpsc::UnboundedReceiver<Vec<u8>>,
   201	    ) -> Result<ReceiveTotals> {
   202	        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
   203	        let mut epoch0 = self.session_token.clone();
   204	        epoch0.extend_from_slice(&self.epoch0_sub_token);
   205	
   206	        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   207	        let mut total = SinkOutcome::default();
   208	        let mut streams = 0usize;
   209	
   210	        // Accept the initial epoch-0 socket(s) first (the zero-knowledge
   211	        // grant is always 1; the loop handles N for symmetry).
   212	        for _ in 0..self.initial_streams {
   213	            let socket = accept_authenticated(&self.listener, &epoch0).await?;
   214	            streams += 1;
   215	            spawn_receive(&mut receives, socket, &sink);
   216	        }
   217	
   218	        // Resize ADDs: each arms a `session_token ‖ sub_token` credential
   219	        // whose socket the SOURCE dials right after its ack. `no_more` is
   220	        // set when the control loop drops the arm sender at `SourceDone`;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '220,720p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   220	        // set when the control loop drops the arm sender at `SourceDone`;
   221	        // the loop then drains the last armed sockets and workers. Because
   222	        // the SOURCE only dials a credential it was acked for (and a dial
   223	        // failure faults the whole session, aborting this task via
   224	        // AbortOnDrop), an armed slot is always consumed — no orphan hang.
   225	        let mut armed: Vec<Vec<u8>> = Vec::new();
   226	        let mut arm_rx = Some(arm_rx);
   227	        let mut no_more = false;
   228	        loop {
   229	            if no_more && armed.is_empty() && receives.is_empty() {
   230	                break;
   231	            }
   232	            // A closed arm channel resolves `recv()` instantly to `None`
   233	            // every poll; parking it on `pending()` once closed keeps the
   234	            // biased select from starving the accept/join arms (otherwise
   235	            // the None arm wins every race and the loop spins without ever
   236	            // collecting a finished worker).
   237	            let arm_recv = async {
   238	                match arm_rx.as_mut() {
   239	                    Some(rx) => rx.recv().await,
   240	                    None => std::future::pending().await,
   241	                }
   242	            };
   243	            tokio::select! {
   244	                biased;
   245	                // Control FIRST: an arm must register before its socket
   246	                // (which the SOURCE dials only after the ack the control
   247	                // loop sends right after arming), so the accept arm below
   248	                // always sees a populated `armed` set.
   249	                arm = arm_recv => match arm {
   250	                    Some(sub_token) => armed.push(sub_token),
   251	                    // Arm sender dropped at SourceDone: no more resizes.
   252	                    None => {
   253	                        arm_rx = None;
   254	                        no_more = true;
   255	                    }
   256	                },
   257	                // Accept only when a resize credential is armed. `accept`
   258	                // is cancel-safe, so losing this arm to another (its
   259	                // pending connection stays queued) drops no socket. The
   260	                // credential read happens OUTSIDE the select (below) so a
   261	                // select cancel can never truncate a half-read socket.
   262	                accepted = accept_raw(&self.listener), if !armed.is_empty() => {
   263	                    let socket = accepted?;
   264	                    let socket =
   265	                        authenticate_resize(socket, &self.session_token, &mut armed).await?;
   266	                    streams += 1;
   267	                    spawn_receive(&mut receives, socket, &sink);
   268	                }
   269	                joined = receives.join_next(), if !receives.is_empty() => {
   270	                    let outcome = joined
   271	                        .expect("join_next is None only when empty, guarded above")
   272	                        .map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   273	                    total.files_written += outcome.files_written;
   274	                    total.bytes_written += outcome.bytes_written;
   275	                }
   276	            }
   277	        }
   278	        Ok(ReceiveTotals {
   279	            outcome: total,
   280	            streams,
   281	        })
   282	    }
   283	}
   284	
   285	impl ResponderDataPlaneRun {
   286	    /// Arm a resize credential so the next socket presenting
   287	    /// `session_token ‖ sub_token` is accepted. Returns false if the
   288	    /// accept loop is gone (its receiver dropped) — the control loop then
   289	    /// acks the resize as refused.
   290	    pub(super) fn arm(&self, sub_token: Vec<u8>) -> bool {
   291	        self.arm_tx.send(sub_token).is_ok()
   292	    }
   293	
   294	    /// Signal `SourceDone` (no more resizes) and join the accept loop for
   295	    /// the aggregated receive totals.
   296	    pub(super) async fn finish(self) -> Result<ReceiveTotals> {
   297	        let ResponderDataPlaneRun { arm_tx, task, .. } = self;
   298	        // Dropping the arm sender is the "no more resizes" signal.
   299	        drop(arm_tx);
   300	        task.join()
   301	            .await
   302	            .map_err(|err| dp_fault(format!("data-plane receive task panicked: {err}")))?
   303	    }
   304	}
   305	
   306	/// Spawn one receive worker draining `socket` into `sink` via the shared
   307	/// receive pipeline, guarded by the transfer stall timeout (carried REV4
   308	/// RELIABLE invariant, matching the old push receive: a peer that
   309	/// authenticates then stalls mid-record trips the stall timeout rather
   310	/// than pinning the task until TCP keepalive).
   311	fn spawn_receive(
   312	    receives: &mut JoinSet<Result<SinkOutcome>>,
   313	    socket: TcpStream,
   314	    sink: &Arc<dyn TransferSink>,
   315	) {
   316	    let sink = Arc::clone(sink);
   317	    receives.spawn(async move {
   318	        let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
   319	        execute_receive_pipeline(&mut guarded, sink, None).await
   320	    });
   321	}
   322	
   323	/// Accept one data socket under the shared bounded-accept timeout and
   324	/// apply the data-plane socket policy. Cancel-safe (the accept itself is;
   325	/// no bytes are read here).
   326	async fn accept_raw(listener: &TcpListener) -> Result<TcpStream> {
   327	    let accept = tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await;
   328	    let socket = match accept {
   329	        Ok(Ok((socket, _peer))) => socket,
   330	        Ok(Err(err)) => return Err(dp_fault(format!("data-plane accept failed: {err}"))),
   331	        Err(_) => {
   332	            return Err(dp_fault(format!(
   333	            "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
   334	        )))
   335	        }
   336	    };
   337	    configure_data_socket(&socket, None)
   338	        .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
   339	    Ok(socket)
   340	}
   341	
   342	/// Read the fixed-length epoch-0 credential and verify it whole. A socket
   343	/// presenting anything else is a `DATA_PLANE_FAILED` fault (the session
   344	/// arms exactly the sockets it dials, so a mismatch is fatal here).
   345	async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
   346	    let mut socket = accept_raw(listener).await?;
   347	    let mut buf = vec![0u8; expected.len()];
   348	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   349	    match read {
   350	        Ok(Ok(_)) => {}
   351	        Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
   352	        Err(_) => {
   353	            return Err(dp_fault(format!(
   354	                "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   355	            )))
   356	        }
   357	    }
   358	    // Constant-time comparison is not required: the tokens are 16 random
   359	    // bytes read once per socket, single-session; a timing oracle buys
   360	    // nothing against per-transfer secrets (same posture as the old push
   361	    // acceptor's `token == expected_token`).
   362	    if buf != expected {
   363	        return Err(dp_fault(
   364	            "data-plane socket presented an invalid credential",
   365	        ));
   366	    }
   367	    Ok(socket)
   368	}
   369	
   370	/// Read a resize socket's `session_token ‖ sub_token(16)` credential
   371	/// (bounded), verify the session token, and match the sub-token against
   372	/// an armed credential — removing it so each arm is consumed once. Runs
   373	/// in the accept loop body (never a select arm), so a select cancel can
   374	/// never truncate a half-read socket.
   375	async fn authenticate_resize(
   376	    socket: TcpStream,
   377	    session_token: &[u8],
   378	    armed: &mut Vec<Vec<u8>>,
   379	) -> Result<TcpStream> {
   380	    let mut socket = socket;
   381	    let mut buf = vec![0u8; session_token.len() + SUB_TOKEN_LEN];
   382	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   383	    match read {
   384	        Ok(Ok(_)) => {}
   385	        Ok(Err(err)) => {
   386	            return Err(dp_fault(format!(
   387	                "reading resize data-plane credential: {err}"
   388	            )))
   389	        }
   390	        Err(_) => {
   391	            return Err(dp_fault(format!(
   392	                "resize data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   393	            )))
   394	        }
   395	    }
   396	    if buf[..session_token.len()] != *session_token {
   397	        return Err(dp_fault(
   398	            "resize data socket presented a wrong session token",
   399	        ));
   400	    }
   401	    let sub = &buf[session_token.len()..];
   402	    match armed.iter().position(|t| t.as_slice() == sub) {
   403	        Some(idx) => {
   404	            armed.swap_remove(idx);
   405	            Ok(socket)
   406	        }
   407	        None => Err(dp_fault(
   408	            "resize data socket presented an unarmed credential",
   409	        )),
   410	    }
   411	}
   412	
   413	// ---------------------------------------------------------------------------
   414	// Initiator (SOURCE) — dial, authenticate, send, resize
   415	// ---------------------------------------------------------------------------
   416	
   417	/// A resize the SOURCE has proposed and minted a credential for but not
   418	/// yet completed: the driver has sent (or will send) the matching
   419	/// `DataPlaneResize{ADD}` on the control lane and, on the peer's
   420	/// `DataPlaneResizeAck`, dials the epoch-N socket. At most one is in
   421	/// flight (the dial's `pending_epoch` enforces it; this is the
   422	/// driver-side record the ack is matched against).
   423	pub(super) struct PendingResize {
   424	    pub(super) epoch: u32,
   425	    pub(super) target_streams: u32,
   426	    pub(super) sub_token: Vec<u8>,
   427	}
   428	
   429	/// A running source-side data plane: the dialed socket(s) wrapped as an
   430	/// ELASTIC sink pipeline that `SinkControl::Add` grows mid-run (the sf-2
   431	/// shape correction). Planned payloads are fed via [`Self::queue`];
   432	/// closing via [`Self::finish`] drains the pipeline, emits each socket's
   433	/// END record, and returns the bytes this end sent.
   434	pub(super) struct SourceDataPlane {
   435	    payload_tx: Option<mpsc::Sender<TransferPayload>>,
   436	    control_tx: mpsc::UnboundedSender<SinkControl>,
   437	    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
   438	    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
   439	    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
   440	    // The byte SENDER owns the live dial, bounded by the byte RECEIVER's
   441	    // advertised capacity (contract §Invariants 5). otp-4b-2 drives only
   442	    // its shape-correction stream count; the cheap-dial tuner is future
   443	    // work, so `chunk_bytes()`/`prefetch_count()` stay at the floor.
   444	    dial: Arc<TransferDial>,
   445	    source: Arc<dyn TransferSource>,
   446	    host: String,
   447	    tcp_port: u32,
   448	    session_token: Vec<u8>,
   449	    pool: Arc<BufferPool>,
   450	}
   451	
   452	/// Dial the granted data plane and start the elastic send pipeline.
   453	/// `host` is the responder's host (the initiator connected the control
   454	/// plane to it; the data plane rides the same host on the granted port —
   455	/// contract §Transport: the initiator always dials). `receiver_capacity`
   456	/// is the DESTINATION's advertised profile from `SessionAccept`; it
   457	/// bounds the sender's dial ceiling (0/absent fields ⇒ conservative,
   458	/// never unlimited).
   459	pub(super) async fn dial_source_data_plane(
   460	    host: &str,
   461	    grant: &DataPlaneGrant,
   462	    receiver_capacity: Option<&CapacityProfile>,
   463	    source: Arc<dyn TransferSource>,
   464	) -> Result<SourceDataPlane> {
   465	    let initial = grant.initial_streams.max(1) as usize;
   466	    // The byte sender's dial, bounded by the receiver's advertised
   467	    // capacity. Seed the settled live count to the granted epoch-0
   468	    // streams — every shape-resize proposal steps from here.
   469	    let dial = TransferDial::conservative_within(receiver_capacity).shared();
   470	    dial.set_negotiated_streams(initial);
   471	
   472	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   473	    let mut handshake = grant.session_token.clone();
   474	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   475	
   476	    // Provision the pool for the dial ceiling so resize-added sockets
   477	    // draw buffers from the same pool without re-pooling (as old push
   478	    // does — a shared pool sized for the maximum stream count).
   479	    let pool = Arc::new(BufferPool::for_data_plane(
   480	        dial.chunk_bytes(),
   481	        dial.ceiling_max_streams().max(1),
   482	    ));
   483	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(initial);
   484	    for _ in 0..initial {
   485	        let session = DataPlaneSession::connect(
   486	            host,
   487	            grant.tcp_port,
   488	            &handshake,
   489	            dial.chunk_bytes(),
   490	            dial.prefetch_count(),
   491	            false,
   492	            dial.tcp_buffer_bytes(),
   493	            Arc::clone(&pool),
   494	        )
   495	        .await
   496	        .map_err(|err| dp_fault(format!("dialing session data plane: {err:#}")))?;
   497	        // The source-side sink never reads its dst_root (it only sends);
   498	        // `root()` is consulted by the relay/receive case, not here.
   499	        sinks.push(Arc::new(DataPlaneSink::new(
   500	            session,
   501	            Arc::clone(&source),
   502	            PathBuf::new(),
   503	        )));
   504	    }
   505	
   506	    let prefetch = dial.prefetch_count().max(1);
   507	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(prefetch);
   508	    let (control_tx, control_rx) = mpsc::unbounded_channel::<SinkControl>();
   509	    let pipe_source = Arc::clone(&source);
   510	    // Bounded by AbortOnDrop: a fault on the control lane that drops the
   511	    // SourceDataPlane aborts the pipeline task instead of leaking it.
   512	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   513	        execute_sink_pipeline_elastic(
   514	            pipe_source,
   515	            sinks,
   516	            payload_rx,
   517	            prefetch,
   518	            None,
   519	            Some(control_rx),
   520	        )
   521	        .await
   522	    }));
   523	    Ok(SourceDataPlane {
   524	        payload_tx: Some(payload_tx),
   525	        control_tx,
   526	        pipeline: Some(pipeline),
   527	        dial,
   528	        source,
   529	        host: host.to_string(),
   530	        tcp_port: grant.tcp_port,
   531	        session_token: grant.session_token.clone(),
   532	        pool,
   533	    })
   534	}
   535	
   536	impl SourceDataPlane {
   537	    /// The live dial (the byte sender owns it). The driver reads
   538	    /// `live_streams()` for observability and calls `resize_settled` as
   539	    /// each proposal completes.
   540	    pub(super) fn dial(&self) -> &Arc<TransferDial> {
   541	        &self.dial
   542	    }
   543	
   544	    /// sf-2 shape correction: propose one ADD toward the stream count the
   545	    /// accumulated need list implies, if none is in flight and the shape
   546	    /// wants more than the current live count. Mints the resize
   547	    /// credential; the driver sends the `DataPlaneResize{ADD}` and hands
   548	    /// the record back on the matching ack.
   549	    pub(super) fn propose_resize(
   550	        &self,
   551	        needed_bytes: u64,
   552	        needed_count: usize,
   553	    ) -> Result<Option<PendingResize>> {
   554	        let desired =
   555	            initial_stream_proposal(needed_bytes, needed_count, self.dial.ceiling_max_streams())
   556	                as usize;
   557	        let Some(proposal) = self.dial.propose_shape_resize(desired) else {
   558	            return Ok(None);
   559	        };
   560	        let sub_token = generate_sub_token()
   561	            .map_err(|err| dp_fault(format!("minting resize sub-token: {err:#}")))?;
   562	        Ok(Some(PendingResize {
   563	            epoch: proposal.epoch,
   564	            target_streams: proposal.target_streams as u32,
   565	            sub_token,
   566	        }))
   567	    }
   568	
   569	    /// Dial the epoch-N data socket for an accepted resize and hand it to
   570	    /// the running pipeline (`SinkControl::Add`). A dial failure is FATAL
   571	    /// (fail-fast): a same-build peer whose listener already accepted
   572	    /// epoch-0 failing an epoch-N dial is a transport fault worth
   573	    /// surfacing — and faulting the session aborts the peer's accept loop
   574	    /// via AbortOnDrop, so its armed slot never orphans. (Old push
   575	    /// recovers non-fatally via an arm TTL; the session trades that for
   576	    /// simplicity — noted in the finding doc.) If the pipeline is already
   577	    /// gone (transfer completing under the ADD), the just-dialed socket
   578	    /// is closed cleanly so the peer's worker sees its END, not a reset.
   579	    pub(super) async fn add_stream(&self, sub_token: &[u8]) -> Result<()> {
   580	        let mut handshake = self.session_token.clone();
   581	        handshake.extend_from_slice(sub_token);
   582	        let session = DataPlaneSession::connect(
   583	            &self.host,
   584	            self.tcp_port,
   585	            &handshake,
   586	            self.dial.chunk_bytes(),
   587	            self.dial.prefetch_count(),
   588	            false,
   589	            self.dial.tcp_buffer_bytes(),
   590	            Arc::clone(&self.pool),
   591	        )
   592	        .await
   593	        .map_err(|err| dp_fault(format!("dialing resize data socket: {err:#}")))?;
   594	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   595	            session,
   596	            Arc::clone(&self.source),
   597	            PathBuf::new(),
   598	        ));
   599	        if let Err(returned) = self.control_tx.send(SinkControl::Add(sink)) {
   600	            if let SinkControl::Add(sink) = returned.0 {
   601	                let _ = sink.finish().await;
   602	            }
   603	        }
   604	        Ok(())
   605	    }
   606	
   607	    /// Feed one planned batch into the send pipeline. The pipeline
   608	    /// prepares each payload (tar-shard/file) and writes it through the
   609	    /// data-plane record framing across the live socket(s).
   610	    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   611	        let tx = self.payload_tx.as_ref().ok_or_else(|| {
   612	            eyre::Report::new(SessionFault::internal("data plane already finished"))
   613	        })?;
   614	        for payload in payloads {
   615	            tx.send(payload).await.map_err(|_| {
   616	                dp_fault("data-plane send pipeline closed before all payloads sent")
   617	            })?;
   618	        }
   619	        Ok(())
   620	    }
   621	
   622	    /// Signal end-of-stream, drain the pipeline (each worker emits its
   623	    /// socket's END record on drain), and return the bytes sent. Must be
   624	    /// awaited before `SourceDone` goes out so the destination's receive
   625	    /// pipeline sees END and completes.
   626	    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
   627	        // Drop the sender: workers observe the closed queue, drain what
   628	        // is left, then `finish()` (END record) and exit.
   629	        self.payload_tx = None;
   630	        let pipeline = self
   631	            .pipeline
   632	            .take()
   633	            .expect("SourceDataPlane::finish called once");
   634	        pipeline
   635	            .join()
   636	            .await
   637	            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))?
   638	    }
   639	}
   640	
   641	// ---------------------------------------------------------------------------
   642	// Need-list enforcement for the data-plane receive
   643	// ---------------------------------------------------------------------------
   644	
   645	/// Sink decorator that enforces the session's need-list contract on the
   646	/// data-plane receive, giving it the SAME strictness the in-stream
   647	/// carrier applies inline in the control loop (`outstanding.remove`).
   648	/// `execute_receive_pipeline` writes socket-provided paths directly, so
   649	/// without this a peer could substitute an off-need-list path for a
   650	/// needed one (count-preserving), duplicate one, or send resume block
   651	/// records the non-resume session never negotiated (codex otp-4b-1 F1).
   652	/// Every written path must be a granted, not-yet-received need; resume
   653	/// block records are rejected outright. The shared [`OutstandingNeeds`]
   654	/// set makes completion `is_empty()` for both carriers.
   655	pub(super) struct NeedListSink {
   656	    inner: Arc<dyn TransferSink>,
   657	    outstanding: OutstandingNeeds,
   658	}
   659	
   660	impl NeedListSink {
   661	    pub(super) fn new(inner: Arc<dyn TransferSink>, outstanding: OutstandingNeeds) -> Self {
   662	        Self { inner, outstanding }
   663	    }
   664	
   665	    /// Remove `path` from the outstanding set, or fault: a path that is
   666	    /// not present is either off the need list or a duplicate delivery.
   667	    fn claim(&self, path: &str) -> Result<()> {
   668	        if self
   669	            .outstanding
   670	            .lock()
   671	            .expect("outstanding-needs lock poisoned")
   672	            .remove(path)
   673	        {
   674	            Ok(())
   675	        } else {
   676	            Err(eyre::Report::new(SessionFault::protocol_violation(
   677	                format!(
   678	                    "data-plane payload for '{path}' which is not an outstanding need \
   679	                 (off the need list, or a duplicate delivery)"
   680	                ),
   681	            )))
   682	        }
   683	    }
   684	}
   685	
   686	#[async_trait]
   687	impl TransferSink for NeedListSink {
   688	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   689	        match &payload {
   690	            PreparedPayload::File(header) => self.claim(&header.relative_path)?,
   691	            PreparedPayload::TarShard { headers, .. } => {
   692	                for header in headers {
   693	                    self.claim(&header.relative_path)?;
   694	                }
   695	            }
   696	            // The session did not negotiate resume (otp-7), so a block
   697	            // record on the data plane is a protocol violation, not a
   698	            // silently-applied patch.
   699	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   700	                return Err(eyre::Report::new(SessionFault::protocol_violation(
   701	                    "resume block record on the data plane of a non-resume session",
   702	                )));
   703	            }
   704	        }
   705	        self.inner.write_payload(payload).await
   706	    }
   707	
   708	    async fn write_file_stream(
   709	        &self,
   710	        header: &FileHeader,
   711	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   712	    ) -> Result<SinkOutcome> {
   713	        self.claim(&header.relative_path)?;
   714	        self.inner.write_file_stream(header, reader).await
   715	    }
   716	
   717	    async fn finish(&self) -> Result<()> {
   718	        self.inner.finish().await
   719	    }
   720	

codex
So far, the new race is only entered after `NeedComplete` and after a pending resize ack has been resolved. I’m checking the destination close path next to verify that `Summary` cannot be a valid event during that window.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1210,1705p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1210	                tx.send(frame(Frame::FileBegin(header.clone()))).await?;
  1211	                if header.size == 0 {
  1212	                    continue; // record complete at 0 cumulative bytes
  1213	                }
  1214	                let mut reader = source.open_file(&header).await?;
  1215	                let mut remaining = header.size;
  1216	                while remaining > 0 {
  1217	                    let want = read_buf.len().min(remaining as usize);
  1218	                    let got = reader.read(&mut read_buf[..want]).await?;
  1219	                    if got == 0 {
  1220	                        // Shorter on disk than the manifest promised —
  1221	                        // the record can no longer complete at
  1222	                        // header.size; abort rather than pad.
  1223	                        eyre::bail!(
  1224	                            "'{}' hit EOF with {} bytes still promised",
  1225	                            header.relative_path,
  1226	                            remaining
  1227	                        );
  1228	                    }
  1229	                    tx.send(frame(Frame::FileData(FileData {
  1230	                        content: read_buf[..got].to_vec(),
  1231	                    })))
  1232	                    .await?;
  1233	                    remaining -= got as u64;
  1234	                }
  1235	            }
  1236	            PreparedPayload::TarShard { headers, data } => {
  1237	                tx.send(frame(Frame::TarShardHeader(TarShardHeader {
  1238	                    files: headers,
  1239	                    archive_size: data.len() as u64,
  1240	                })))
  1241	                .await?;
  1242	                for chunk in data.chunks(IN_STREAM_CHUNK) {
  1243	                    tx.send(frame(Frame::TarShardChunk(
  1244	                        crate::generated::TarShardChunk {
  1245	                            content: chunk.to_vec(),
  1246	                        },
  1247	                    )))
  1248	                    .await?;
  1249	                }
  1250	                tx.send(frame(Frame::TarShardComplete(TarShardComplete {})))
  1251	                    .await?;
  1252	            }
  1253	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
  1254	                // The outbound planner never emits these (resume is
  1255	                // receive-originated and lands at otp-7).
  1256	                eyre::bail!("resume payload planned in a non-resume session");
  1257	            }
  1258	        }
  1259	    }
  1260	    Ok(())
  1261	}
  1262	
  1263	// ---------------------------------------------------------------------------
  1264	// DESTINATION driver
  1265	// ---------------------------------------------------------------------------
  1266	
  1267	/// What the destination end can report after a completed session.
  1268	#[derive(Debug, Clone)]
  1269	pub struct DestinationOutcome {
  1270	    /// The summary this end computed and sent (contract: DESTINATION
  1271	    /// is the scorer).
  1272	    pub summary: TransferSummary,
  1273	    /// Paths this end put on the need list, in emission order. The
  1274	    /// role suite pins these identical across role assignments — the
  1275	    /// executable form of the owner's invariance requirement.
  1276	    pub needed_paths: Vec<String>,
  1277	    /// The settled data-plane stream count this end observed (epoch-0 +
  1278	    /// accepted resizes), or `None` for the in-stream carrier. The sf-2
  1279	    /// pin (otp-4b-2) reads it to assert shape correction grew the
  1280	    /// stream set past the zero-knowledge single-stream grant.
  1281	    pub data_plane_streams: Option<usize>,
  1282	}
  1283	
  1284	/// Run the DESTINATION role of one transfer session over `transport`,
  1285	/// writing under the root named by `target`. Diffs the streamed
  1286	/// manifest against its own filesystem (the destination is the one
  1287	/// diff owner — plan §Design 3), returns the summary it computed and
  1288	/// sent.
  1289	///
  1290	/// `target` is [`DestinationTarget::Fixed`] when the root is known up
  1291	/// front (an Initiator's own local root, or a test), or
  1292	/// [`DestinationTarget::Resolve`] when the root must be resolved from
  1293	/// the received `SessionOpen` mid-handshake (the daemon Responder,
  1294	/// where the wire module name selects the root).
  1295	pub async fn run_destination(
  1296	    cfg: DestinationSessionConfig,
  1297	    transport: FrameTransport,
  1298	    target: DestinationTarget,
  1299	) -> Result<DestinationOutcome> {
  1300	    let mut transport = transport;
  1301	    let endpoint = match cfg.endpoint {
  1302	        SessionEndpoint::Initiator { mut open } => {
  1303	            let declared = TransferRole::try_from(open.initiator_role);
  1304	            if declared != Ok(TransferRole::Destination) {
  1305	                eyre::bail!(
  1306	                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
  1307	                );
  1308	            }
  1309	            if let Err(fault) = destination_open_validator(&open) {
  1310	                eyre::bail!("run_destination initiator config unsupported: {fault}");
  1311	            }
  1312	            // Dial contract: the byte receiver advertises capacity in
  1313	            // its open when it is the initiator (contract §Invariants 5).
  1314	            if open.receiver_capacity.is_none() {
  1315	                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
  1316	            }
  1317	            SessionEndpoint::Initiator { open }
  1318	        }
  1319	        SessionEndpoint::Responder => SessionEndpoint::Responder,
  1320	    };
  1321	
  1322	    let resolve_open: Option<&OpenResolver> = match &target {
  1323	        DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
  1324	        DestinationTarget::Fixed(_) => None,
  1325	    };
  1326	
  1327	    let negotiated = establish(
  1328	        &mut transport,
  1329	        &cfg.hello,
  1330	        &endpoint,
  1331	        TransferRole::Destination,
  1332	        &destination_open_validator,
  1333	        resolve_open,
  1334	    )
  1335	    .await?;
  1336	
  1337	    // The resolver's root (Responder + Resolve) wins; otherwise the
  1338	    // caller-supplied Fixed root.
  1339	    let dst_root = match negotiated.resolved_root.clone() {
  1340	        Some(root) => root,
  1341	        None => match &target {
  1342	            DestinationTarget::Fixed(root) => root.clone(),
  1343	            // Unreachable: a Resolve target always yields a root on the
  1344	            // Responder branch, and establish only skips resolution on
  1345	            // the Initiator branch (which pairs with a Fixed root).
  1346	            DestinationTarget::Resolve(_) => {
  1347	                return Err(eyre::Report::new(SessionFault::internal(
  1348	                    "resolver target produced no destination root",
  1349	                )));
  1350	            }
  1351	        },
  1352	    };
  1353	
  1354	    match destination_session(&mut transport, negotiated, &dst_root).await {
  1355	        Ok(outcome) => Ok(outcome),
  1356	        Err(report) => {
  1357	            let mut fault = fault_from_report(report);
  1358	            if !fault.peer_notified {
  1359	                let _ = transport.send(error_frame(&fault)).await;
  1360	                fault.peer_notified = true;
  1361	            }
  1362	            Err(eyre::Report::new(fault))
  1363	        }
  1364	    }
  1365	}
  1366	
  1367	fn violation(message: String) -> eyre::Report {
  1368	    eyre::Report::new(SessionFault::protocol_violation(message))
  1369	}
  1370	
  1371	async fn destination_session(
  1372	    transport: &mut FrameTransport,
  1373	    negotiated: Negotiated,
  1374	    dst_root: &Path,
  1375	) -> Result<DestinationOutcome> {
  1376	    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
  1377	        .unwrap_or(ComparisonMode::Unspecified);
  1378	    let compare_opts = CompareOptions {
  1379	        mode: compare_mode.into(),
  1380	        ignore_existing: negotiated.open.ignore_existing,
  1381	        include_deletions: false, // mirror lands at otp-6
  1382	    };
  1383	    // src_root is only consumed by local File payloads, which never
  1384	    // occur on a session destination (payload bytes arrive as records
  1385	    // and go through the stream/tar write paths). `Arc` so the data-plane
  1386	    // receive task (otp-4b) can share the one sink across sockets.
  1387	    let sink = Arc::new(FsTransferSink::new(
  1388	        PathBuf::new(),
  1389	        dst_root.to_path_buf(),
  1390	        FsSinkConfig {
  1391	            preserve_times: true,
  1392	            dry_run: false,
  1393	            checksum: None,
  1394	            resume: false,
  1395	            compare_mode,
  1396	        },
  1397	    ));
  1398	    // Same canonical-containment chokepoint the sink write paths use
  1399	    // (R46-F3), applied to diff stats so a hostile manifest path can't
  1400	    // make the destination stat outside its root.
  1401	    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
  1402	
  1403	    // Two sets, deliberately separate (codex otp-4b-1 fix-review F1):
  1404	    // `granted` is the ever-granted DEDUP set — control-loop-local,
  1405	    // insert-only, never removed, so a concurrent data-plane claim can
  1406	    // never re-open a grant (a duplicate manifest path is granted at
  1407	    // most once regardless of delivery timing). `outstanding` is the
  1408	    // not-yet-delivered COMPLETION set — inserted for each freshly
  1409	    // granted path before its NeedBatch, claimed by both carriers (the
  1410	    // in-stream arms inline, the data-plane NeedListSink as payloads
  1411	    // land), and empty at SourceDone. A count proxy was insufficient
  1412	    // (F1); merging the two into one set raced the data-plane claim
  1413	    // against the diff (fix-review F1).
  1414	    let mut granted: HashSet<String> = HashSet::new();
  1415	    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
  1416	
  1417	    // Data plane (otp-4b): when the responder granted a TCP data plane,
  1418	    // payload bytes arrive on sockets (not the control lane). Arm the
  1419	    // accept+receive task NOW — concurrent with the diff loop below, and
  1420	    // before the source dials — so the connections are accepted promptly.
  1421	    // The NeedListSink gives the socket receive the same need-list
  1422	    // strictness the in-stream control loop applies inline. AbortOnDrop
  1423	    // bounds it to this future: a control-lane fault that returns from
  1424	    // this fn aborts the receive task instead of leaking it.
  1425	    // `resize_live` tracks the stream count this end has granted (epoch-0
  1426	    // plus each accepted resize ADD); `resize_ceiling` is the receiver's
  1427	    // advertised max_streams, the cumulative bound a resize may not cross.
  1428	    let (mut data_plane_recv, mut resize_live, resize_ceiling) =
  1429	        match negotiated.responder_data_plane {
  1430	            Some(rdp) => {
  1431	                let initial = rdp.initial_streams() as usize;
  1432	                let recv_sink: Arc<dyn TransferSink> = Arc::new(data_plane::NeedListSink::new(
  1433	                    Arc::clone(&sink) as Arc<dyn TransferSink>,
  1434	                    Arc::clone(&outstanding),
  1435	                ));
  1436	                let run = rdp.spawn(recv_sink);
  1437	                let ceiling = run.ceiling;
  1438	                (Some(run), initial, ceiling)
  1439	            }
  1440	            None => (None, 0usize, 0usize),
  1441	        };
  1442	
  1443	    let mut pending: Vec<FileHeader> = Vec::new();
  1444	    let mut needed_paths: Vec<String> = Vec::new();
  1445	    let mut manifest_complete = false;
  1446	    let mut files_written: u64 = 0;
  1447	    let mut bytes_written: u64 = 0;
  1448	
  1449	    loop {
  1450	        let received = match transport.recv().await? {
  1451	            Some(f) => f,
  1452	            None => {
  1453	                return Err(eyre::Report::new(SessionFault::internal(
  1454	                    "peer closed mid-session",
  1455	                )))
  1456	            }
  1457	        };
  1458	        match received.frame {
  1459	            Some(Frame::ManifestEntry(header)) => {
  1460	                if manifest_complete {
  1461	                    return Err(violation(format!(
  1462	                        "manifest entry '{}' after ManifestComplete",
  1463	                        header.relative_path
  1464	                    )));
  1465	                }
  1466	                pending.push(header);
  1467	                if pending.len() >= DEST_DIFF_CHUNK {
  1468	                    let chunk = std::mem::take(&mut pending);
  1469	                    diff_chunk_and_send_needs(
  1470	                        transport,
  1471	                        chunk,
  1472	                        dst_root,
  1473	                        canonical_dst_root.as_deref(),
  1474	                        &compare_opts,
  1475	                        &mut granted,
  1476	                        &outstanding,
  1477	                        &mut needed_paths,
  1478	                    )
  1479	                    .await?;
  1480	                }
  1481	            }
  1482	            Some(Frame::ManifestComplete(_complete)) => {
  1483	                if manifest_complete {
  1484	                    return Err(violation("duplicate ManifestComplete".into()));
  1485	                }
  1486	                // (scan_complete gates mirror purges from otp-6 on;
  1487	                // nothing consumes it in otp-3.)
  1488	                let chunk = std::mem::take(&mut pending);
  1489	                diff_chunk_and_send_needs(
  1490	                    transport,
  1491	                    chunk,
  1492	                    dst_root,
  1493	                    canonical_dst_root.as_deref(),
  1494	                    &compare_opts,
  1495	                    &mut granted,
  1496	                    &outstanding,
  1497	                    &mut needed_paths,
  1498	                )
  1499	                .await?;
  1500	                // NeedComplete only after ManifestComplete received
  1501	                // AND every entry diffed — both true here.
  1502	                transport
  1503	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
  1504	                    .await?;
  1505	                manifest_complete = true;
  1506	            }
  1507	            Some(Frame::FileBegin(header)) => {
  1508	                // Payload records ride the control lane only under the
  1509	                // in-stream carrier; with a TCP data plane active they
  1510	                // flow over the sockets, so one here is a violation.
  1511	                if data_plane_recv.is_some() {
  1512	                    return Err(violation(format!(
  1513	                        "file record '{}' on the control lane while a TCP data plane is active",
  1514	                        header.relative_path
  1515	                    )));
  1516	                }
  1517	                if !manifest_complete {
  1518	                    return Err(violation(format!(
  1519	                        "payload record for '{}' before ManifestComplete",
  1520	                        header.relative_path
  1521	                    )));
  1522	                }
  1523	                if !outstanding
  1524	                    .lock()
  1525	                    .expect("outstanding-needs lock poisoned")
  1526	                    .remove(&header.relative_path)
  1527	                {
  1528	                    return Err(violation(format!(
  1529	                        "payload for '{}' which is not on the need list",
  1530	                        header.relative_path
  1531	                    )));
  1532	                }
  1533	                let outcome = receive_file_record(transport, &sink, &header).await?;
  1534	                files_written += outcome.files_written as u64;
  1535	                bytes_written += outcome.bytes_written;
  1536	            }
  1537	            Some(Frame::TarShardHeader(shard)) => {
  1538	                if data_plane_recv.is_some() {
  1539	                    return Err(violation(
  1540	                        "tar shard record on the control lane while a TCP data plane is active"
  1541	                            .into(),
  1542	                    ));
  1543	                }
  1544	                if !manifest_complete {
  1545	                    return Err(violation("tar shard record before ManifestComplete".into()));
  1546	                }
  1547	                {
  1548	                    let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
  1549	                    for h in &shard.files {
  1550	                        if !out.remove(&h.relative_path) {
  1551	                            return Err(violation(format!(
  1552	                                "tar shard entry '{}' which is not on the need list",
  1553	                                h.relative_path
  1554	                            )));
  1555	                        }
  1556	                    }
  1557	                }
  1558	                let outcome = receive_tar_record(transport, &sink, shard).await?;
  1559	                files_written += outcome.files_written as u64;
  1560	                bytes_written += outcome.bytes_written;
  1561	            }
  1562	            Some(Frame::Resize(resize)) => {
  1563	                // sf-2 shape correction (otp-4b-2): the SOURCE proposes
  1564	                // one ADD; arm the credential, grant it (bump `resize_live`),
  1565	                // and ack so the SOURCE dials the epoch-N socket. Only ADD
  1566	                // occurs on the session (REMOVE is a tuner concern, future
  1567	                // work); anything else fails fast.
  1568	                let run = data_plane_recv.as_ref().ok_or_else(|| {
  1569	                    violation("DataPlaneResize on a session with no data plane".into())
  1570	                })?;
  1571	                let op = DataPlaneResizeOp::try_from(resize.op)
  1572	                    .unwrap_or(DataPlaneResizeOp::Unspecified);
  1573	                if op != DataPlaneResizeOp::Add {
  1574	                    return Err(violation(format!(
  1575	                        "unsupported data-plane resize op {}",
  1576	                        op.as_str_name()
  1577	                    )));
  1578	                }
  1579	                if resize.sub_token.len() != crate::remote::transfer::SUB_TOKEN_LEN {
  1580	                    return Err(violation(
  1581	                        "DataPlaneResize sub_token must be 16 bytes".into(),
  1582	                    ));
  1583	                }
  1584	                // Cumulative ceiling bound (defense in depth — the
  1585	                // source's dial already clamps to the same profile).
  1586	                let accepted = resize_live < resize_ceiling && run.arm(resize.sub_token.clone());
  1587	                if accepted {
  1588	                    resize_live += 1;
  1589	                }
  1590	                let effective = if accepted {
  1591	                    resize.target_stream_count
  1592	                } else {
  1593	                    resize_live as u32
  1594	                };
  1595	                transport
  1596	                    .send(frame(Frame::ResizeAck(DataPlaneResizeAck {
  1597	                        epoch: resize.epoch,
  1598	                        effective_stream_count: effective,
  1599	                        accepted,
  1600	                    })))
  1601	                    .await?;
  1602	            }
  1603	            Some(Frame::SourceDone(_)) => {
  1604	                if !manifest_complete {
  1605	                    return Err(violation("SourceDone before ManifestComplete".into()));
  1606	                }
  1607	                // Completion, both carriers: the shared `outstanding`
  1608	                // set must be empty (every granted need claimed exactly
  1609	                // once). In-stream claims inline above; the data-plane
  1610	                // NeedListSink claims as payloads land, so joining the
  1611	                // receive task first drains the last of them (and
  1612	                // surfaces any receive error / stall). Set membership —
  1613	                // not a file count — is the contract (codex F1: a count
  1614	                // proxy let a peer substitute or duplicate paths).
  1615	                // `finish()` drops the arm sender (no more resizes), joins
  1616	                // the accept loop, and reports the settled stream count.
  1617	                let (in_stream_carrier_used, data_plane_streams) = match data_plane_recv.take() {
  1618	                    Some(run) => {
  1619	                        let totals = run.finish().await?;
  1620	                        files_written = totals.outcome.files_written as u64;
  1621	                        bytes_written = totals.outcome.bytes_written;
  1622	                        (false, Some(totals.streams))
  1623	                    }
  1624	                    None => (true, None),
  1625	                };
  1626	                let unfulfilled = outstanding
  1627	                    .lock()
  1628	                    .expect("outstanding-needs lock poisoned")
  1629	                    .len();
  1630	                if unfulfilled != 0 {
  1631	                    return Err(violation(format!(
  1632	                        "SourceDone with {unfulfilled} needed file(s) never delivered"
  1633	                    )));
  1634	                }
  1635	                let summary = TransferSummary {
  1636	                    files_transferred: files_written,
  1637	                    bytes_transferred: bytes_written,
  1638	                    entries_deleted: 0, // mirror lands at otp-6
  1639	                    in_stream_carrier_used,
  1640	                    files_resumed: 0, // resume lands at otp-7
  1641	                };
  1642	                transport.send(frame(Frame::Summary(summary))).await?;
  1643	                return Ok(DestinationOutcome {
  1644	                    summary,
  1645	                    needed_paths,
  1646	                    data_plane_streams,
  1647	                });
  1648	            }
  1649	            Some(Frame::Error(err)) => {
  1650	                return Err(eyre::Report::new(SessionFault::from_wire(err)));
  1651	            }
  1652	            other => {
  1653	                // Everything else is off-lane or off-phase here:
  1654	                // destination-lane frames echoed back (a ResizeAck the
  1655	                // destination would never receive), resume frames in a
  1656	                // non-resume session (otp-7), stray handshake frames,
  1657	                // bare FileData/TarShardChunk outside a record. Fail
  1658	                // fast, no tolerant parsing.
  1659	                return Err(violation(format!(
  1660	                    "{} not valid on the destination's receive lane in this phase",
  1661	                    frame_name(&other)
  1662	                )));
  1663	            }
  1664	        }
  1665	    }
  1666	}
  1667	
  1668	/// Stat-and-compare one chunk of manifest entries on the blocking
  1669	/// pool (2+ syscalls per entry — same rationale as the daemon's
  1670	/// w4-4 chunked checks), then stream the resulting need batch.
  1671	async fn diff_chunk_and_send_needs(
  1672	    transport: &mut FrameTransport,
  1673	    chunk: Vec<FileHeader>,
  1674	    dst_root: &Path,
  1675	    canonical_dst_root: Option<&Path>,
  1676	    compare_opts: &CompareOptions,
  1677	    // Ever-granted DEDUP set (control-loop-local, insert-only): a path
  1678	    // the source manifests twice is granted at most once, and because it
  1679	    // is never removed, a concurrent data-plane claim can't re-open the
  1680	    // grant (fix-review F1).
  1681	    granted: &mut HashSet<String>,
  1682	    // Not-yet-delivered COMPLETION set (shared with the receive).
  1683	    outstanding: &data_plane::OutstandingNeeds,
  1684	    needed_paths: &mut Vec<String>,
  1685	) -> Result<()> {
  1686	    if chunk.is_empty() {
  1687	        return Ok(());
  1688	    }
  1689	    let dst_root = dst_root.to_path_buf();
  1690	    let canonical = canonical_dst_root.map(Path::to_path_buf);
  1691	    let opts = compare_opts.clone();
  1692	    let needed: Vec<String> = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
  1693	        let mut needed = Vec::new();
  1694	        for header in &chunk {
  1695	            if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
  1696	                needed.push(header.relative_path.clone());
  1697	            }
  1698	        }
  1699	        Ok(needed)
  1700	    })
  1701	    .await
  1702	    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;
  1703	
  1704	    // Dedup on the ever-granted set (no lock — control-loop-local), then
  1705	    // insert the freshly granted paths into the shared `outstanding`

exec
/usr/bin/zsh -lc "rg -n \"run_destination|destination|SourceDone|NeedComplete|outstanding|ResponderDataPlaneRun|finish\\(\" crates/blit-core/src/transfer_session/mod.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
34:    FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch, NeedComplete, NeedEntry,
35:    SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone, TarShardComplete,
58:/// Manifest entries buffered per destination diff batch. Mirrors the
65:/// into `FsTransferSink::write_file_stream`. Bounds destination-side
241:        Some(Frame::NeedComplete(_)) => "NeedComplete",
252:        Some(Frame::SourceDone(_)) => "SourceDone",
346:fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
505:                                    "destination module is read-only".to_string(),
587:    NeedComplete,
588:    /// The destination's ack of a `DataPlaneResize{ADD}` (otp-4b-2). The
596:/// Returns the destination-computed `TransferSummary` (contract: the
631:    // an ordered transport, a NeedComplete arriving while this is
633:    // received what we have not sent (contract: NeedComplete only
670:/// whole session so destination sends can never deadlock against a
671:/// blocked source send, and routes the destination lane to the send
727:            Some(Frame::NeedComplete(_)) => {
731:                    // NeedComplete be processed late and pass as
734:                        "NeedComplete before the source's ManifestComplete",
738:                let _ = events.send(SourceEvent::NeedComplete);
741:                // The destination's response to a shape-resize proposal
777:    // BEFORE streaming the manifest — so the destination's accept loop
916:                    "source receive half ended before NeedComplete",
923:    // it BEFORE finishing so the destination's armed slot is consumed by
933:    // Close the data plane BEFORE SourceDone so the destination's receive
934:    // pipeline sees each socket's END record and completes; SourceDone on
935:    // the control lane then lets the destination score and summarize.
946:    //     `finish()` future drops the data plane, and its `AbortOnDrop`
948:    //   * the socket break makes `finish()` return `Err` first → prefer
957:            res = dp.finish() => {
965:    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
967:    // CLOSING: the destination is the scorer; the next event must be
973:            format!("need for '{}' after NeedComplete", h.relative_path),
975:        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
976:            SessionFault::protocol_violation("duplicate NeedComplete"),
979:            SessionFault::protocol_violation("DataPlaneResizeAck after SourceDone"),
1035:                    format!("need for '{}' after NeedComplete", header.relative_path),
1043:        SourceEvent::NeedComplete => {
1046:                    "duplicate NeedComplete",
1080:            "TransferSummary before SourceDone",
1116:/// pending proposal so the destination's armed slot is consumed before we
1142:                    format!("need for '{}' after NeedComplete", h.relative_path),
1145:            Some(SourceEvent::NeedComplete) => {
1147:                    "duplicate NeedComplete",
1152:                    "TransferSummary before SourceDone",
1170:/// drain the receive half only ever forwards a fault (SourceDone has not
1267:/// What the destination end can report after a completed session.
1286:/// manifest against its own filesystem (the destination is the one
1295:pub async fn run_destination(
1306:                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
1309:            if let Err(fault) = destination_open_validator(&open) {
1310:                eyre::bail!("run_destination initiator config unsupported: {fault}");
1332:        &destination_open_validator,
1348:                    "resolver target produced no destination root",
1354:    match destination_session(&mut transport, negotiated, &dst_root).await {
1371:async fn destination_session(
1384:    // occur on a session destination (payload bytes arrive as records
1400:    // make the destination stat outside its root.
1407:    // most once regardless of delivery timing). `outstanding` is the
1411:    // land), and empty at SourceDone. A count proxy was insufficient
1415:    let outstanding: data_plane::OutstandingNeeds = Arc::new(StdMutex::new(HashSet::new()));
1434:                    Arc::clone(&outstanding),
1476:                        &outstanding,
1496:                    &outstanding,
1500:                // NeedComplete only after ManifestComplete received
1503:                    .send(frame(Frame::NeedComplete(NeedComplete {})))
1523:                if !outstanding
1525:                    .expect("outstanding-needs lock poisoned")
1548:                    let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
1603:            Some(Frame::SourceDone(_)) => {
1605:                    return Err(violation("SourceDone before ManifestComplete".into()));
1607:                // Completion, both carriers: the shared `outstanding`
1615:                // `finish()` drops the arm sender (no more resizes), joins
1619:                        let totals = run.finish().await?;
1626:                let unfulfilled = outstanding
1628:                    .expect("outstanding-needs lock poisoned")
1632:                        "SourceDone with {unfulfilled} needed file(s) never delivered"
1654:                // destination-lane frames echoed back (a ResizeAck the
1655:                // destination would never receive), resume frames in a
1660:                    "{} not valid on the destination's receive lane in this phase",
1683:    outstanding: &data_plane::OutstandingNeeds,
1695:            if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
1702:    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;
1705:    // insert the freshly granted paths into the shared `outstanding`
1714:        let mut out = outstanding.lock().expect("outstanding-needs lock poisoned");
1736:/// Does the destination need this manifest entry? Stats its own file
1740:fn destination_needs(
1754:            "manifest path '{}' escapes the destination root: {err:#}",

exec
/usr/bin/zsh -lc "nl -ba .review/findings/otp-4b-data-plane.md | sed -n '1,130p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	# otp-4b — TCP data plane onto the unified session
     2	
     3	**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-4.
     4	**Contract**: `docs/TRANSFER_SESSION.md` §Transport selection.
     5	**Builds on**: otp-4a (`4b07bbb`+`25f538b`) — daemon serves `Transfer`,
     6	client `run_source`s as SOURCE over the **in-stream** carrier.
     7	**Status**: 4b-1 (single-stream data plane) **CLOSED** — codex loop, 3
     8	passes (`881d412`; fix `e1aafcc` for 2 High; fix `777dfc5` for the race
     9	that fix introduced; confirming re-review PASS). Suite 1509 → **1512/0**.
    10	4b-2 (resize + multi-stream + sf-2 pin) **CLOSED** — `dce56de`, codex
    11	**PASS** (no findings; the one load-bearing busy-spin bug was caught in
    12	the author's pre-commit e2e and fixed before the reviewed commit —
    13	verdict `.review/results/otp-4b2-data-plane.gpt-verdict.md`). Suite 1512
    14	→ **1513/0**. 4b-3 (deterministic mid-transfer cancel e2e + source-side
    15	cancel responsiveness) **implemented** — suite 1513 → **1515/0**; codex
    16	review pending.
    17	
    18	---
    19	
    20	## otp-4b-3 (deterministic mid-transfer cancel e2e) — implemented
    21	
    22	### What
    23	Pin, deterministically, that a `CancelJob` fired while payload bytes are
    24	in flight over the TCP data plane surfaces to the client as
    25	`SessionFault{CANCELLED}` (the peer's framed abort reason) — not the
    26	data-plane transport break the cancel also causes — and that the daemon
    27	tears the job down cleanly. Building the e2e surfaced that the current
    28	source could **not** meet that contract, so this slice is a small
    29	source-side reliability fix plus its guard tests.
    30	
    31	### Problem found (empirically, before the fix)
    32	The daemon side was already correct: on a `CancelJob` the served
    33	`Transfer` dispatcher (`core.rs::resolve_transfer_session_outcome`,
    34	otp-4a codex F1) drops the `run_destination` future and frames
    35	`SessionError{CANCELLED}` on the control lane. But the SOURCE only
    36	consulted the control lane when it happened to be parked at
    37	`events.recv()`. During the **payload drain** (`SourceDataPlane::finish`,
    38	where a push spends its byte-transfer wall time) the send half awaited
    39	only the data-plane pipeline. So a mid-transfer cancel dropped the
    40	destination → the source's socket write hit `Broken pipe` first → the
    41	client surfaced `SessionFault{INTERNAL}` "Broken pipe", and if a worker
    42	was blocked reading a slow file (never writing) the socket break never
    43	unblocked it and the client **hung**. (Both observed with a throwaway
    44	gated-source experiment.)
    45	
    46	### Approach (source-side fix, `transfer_session/mod.rs`)
    47	`source_send_half` now races the data-plane drain against a peer-framed
    48	fault on the control lane, covering both orderings:
    49	- `recv_peer_fault(events)` — awaits the next `SourceEvent::Fault` the
    50	  receive half forwards. In a `biased` `select!` against `dp.finish()`,
    51	  if the framed fault arrives while the drain is still pending (e.g. a
    52	  worker blocked reading), it wins; dropping the unfinished `finish()`
    53	  future drops the `SourceDataPlane`, whose `AbortOnDrop` stops the
    54	  in-flight workers. This is the fix that makes the blocked-reader case
    55	  terminate as CANCELLED instead of hanging.
    56	- `prefer_peer_fault(events, dp_err)` — when the socket break makes
    57	  `finish()` return `Err` first, prefer the framed reason if the control
    58	  lane delivers one within `TRANSFER_STALL_TIMEOUT` (the peer runs the
    59	  same stall guard on its receive workers, so within that window it
    60	  always frames the real reason); otherwise fall back to the raw
    61	  data-plane error.
    62	
    63	### Files
    64	- `crates/blit-core/src/transfer_session/mod.rs` — `recv_peer_fault` +
    65	  `prefer_peer_fault` helpers; `source_send_half`'s finish() drain wrapped
    66	  in the `select!`; `use …stall_guard::TRANSFER_STALL_TIMEOUT`.
    67	- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — the harness
    68	  now retains an `ActiveJobs` clone (to fire the row's cancel token, which
    69	  is exactly what `cancel_authorized` fires); `StuckAfterFirstChunkSource`;
    70	  the cancel e2e.
    71	
    72	### Tests
    73	Suite 1513 → **1515** (+2):
    74	- `mid_transfer_cancel_surfaces_cancelled_over_the_data_plane`
    75	  (blit-daemon e2e) — a `StuckAfterFirstChunkSource` writes one 64 KiB
    76	  chunk over the data plane then blocks; the test waits for that chunk
    77	  (bytes provably flowed), fires the row's cancel token, and asserts the
    78	  client returns `SessionFault{CANCELLED}` within 10 s (no hang) and the
    79	  daemon drains the row from `active[]`.
    80	- `prefer_peer_fault_prefers_a_framed_fault` (blit-core unit) — a framed
    81	  `CANCELLED` on the events channel wins over a `DATA_PLANE_FAILED`
    82	  data-plane error.
    83	
    84	### Guard proof
    85	- e2e: reverting the `select!` to `dp.finish().await?` makes the blocked
    86	  reader hang → the client's 10 s timeout trips → test FAILS
    87	  ("client must not hang on a mid-transfer cancel: Elapsed"). Restored →
    88	  passes.
    89	- unit: reverting `prefer_peer_fault` to return `dp_err` yields
    90	  `DataPlaneFailed` and the assert FAILS ("framed CANCELLED must win").
    91	  Restored → passes.
    92	
    93	### Known gaps (new)
    94	- A cancel while an *earlier* `dp.queue()` batch is blocked on pipeline
    95	  backpressure (multi-file, sustained) still surfaces the data-plane
    96	  error rather than CANCELLED — `queue()` is not raced (racing it would
    97	  consume live `Need` events on the happy path). The drain (`finish()`)
    98	  is where a push spends its transfer wall time, so this is the dominant
    99	  path; the queue-backpressure edge is a follow-up. The peer's stall
   100	  guard still bounds it.
   101	- The RPC-level `CancelJob` mapping (auth via `cancel_authorized`,
   102	  gRPC outcome codes) is exercised by its own unit tests; this e2e fires
   103	  the same cancellation token directly to keep the session-propagation
   104	  assertion deterministic.
   105	
   106	## otp-4b-2 (resize + multi-stream + sf-2 pin) — implemented
   107	
   108	### What
   109	Port mid-transfer stream growth onto the unified session so the
   110	zero-knowledge single-stream grant shape-corrects upward as the need
   111	list accumulates (the sf-2 mechanism), over real data-plane sockets.
   112	No proto change — consumes the frames otp-1 froze (`DataPlaneResize`
   113	16, `DataPlaneResizeAck` 17).
   114	
   115	### Approach
   116	- **SOURCE owns the live dial** (`TransferDial::conservative_within(
   117	  receiver_capacity)`, seeded to the granted epoch-0 count). As needs
   118	  accumulate it re-runs the shape table
   119	  (`initial_stream_proposal(needed_bytes, needed_count, ceiling)`) and
   120	  calls `propose_shape_resize` — one ADD per epoch, one in flight. The
   121	  driver mints a 16-byte sub-token, sends `DataPlaneResize{ADD}` on the
   122	  control lane, and on the `DataPlaneResizeAck` dials the epoch-N socket
   123	  (`session_token ‖ sub_token`) and hands it to the running elastic
   124	  pipeline via `SinkControl::Add`. `resize_settled` advances the live
   125	  count. (`transfer_session/data_plane.rs`: `SourceDataPlane` +
   126	  `dial_source_data_plane` now build the dial and an
   127	  `execute_sink_pipeline_elastic` with a `SinkControl` channel;
   128	  `mod.rs`: `source_send_half` accumulates `needed_bytes/count`,
   129	  `maybe_propose_resize`, `process_source_event` handles `ResizeAck`,
   130	  `resolve_in_flight_resize` drains the last proposal before finish.)

exec
/usr/bin/zsh -lc "nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	# One transfer path — direction-invariant by construction
     2	
     3	**Status**: Active
     4	**Created**: 2026-07-05
     5	**Supersedes**: post-REV4 residue item "pull 1s-start restructuring"
     6	(absorbed here); pauses `docs/plan/SMALL_FILE_CEILING.md` after sf-2
     7	(D-2026-07-05-1). REV4's mixed-version-peers constraint is superseded
     8	outright by **D-2026-07-05-2 (no version compatibility, ever — same
     9	build only)** — annotated in REV4 §Constraints
    10	**Decision ref**: D-2026-07-05-1 (directive + pause);
    11	**D-2026-07-05-4 (Draft → Active, owner "flip the plan and go",
    12	2026-07-05)**
    13	
    14	## Directive (owner, 2026-07-05, verbatim)
    15	
    16	> "make ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF
    17	> ANYTHING EVER using anything else because anything else does not
    18	> exist."
    19	
    20	> "just make it so that I NEVER see a situation where pull is faster
    21	> than push or vice versa. that CAN NEVER be possible because of
    22	> something blit did. it should be identical if I start the transfer
    23	> from skippy and push to this machine or if I start the transfer on
    24	> this machine and pull from skippy."
    25	
    26	> On benchmark methodology: "tmp on one side, spinning rust on the
    27	> other is not a valid test."
    28	
    29	Scope, wire, and process were explicitly delegated to the agent
    30	("no idea. you architected this"; "I DO NOT CARE. FIX IT."). The
    31	owner's requirement is the invariant; everything below is the
    32	architecture that makes the invariant impossible to violate rather
    33	than merely maintained by discipline.
    34	
    35	## Goal
    36	
    37	One `TransferSession` implementation owns every byte transfer blit
    38	performs. A transfer has a SOURCE role and a DESTINATION role; which
    39	end initiated, and which CLI verb was used, select roles — they do not
    40	select code. When this plan ships, the per-direction drivers (client
    41	push driver, daemon push-receive, client pull driver, daemon
    42	pull-send, delegated-pull driver, local orchestration) **do not
    43	exist**: for fixed endpoints and dataset, direction/initiator/verb
    44	cannot affect behavior or wall time by blit's doing, because there is
    45	no second code path to differ.
    46	
    47	## Non-goals
    48	
    49	- Version compatibility of ANY kind (D-2026-07-05-2, owner standing
    50	  rule: "backward compatibility is NOT a consideration... same build
    51	  only. do not engineer tech debt into an unshipped product"). A blit
    52	  client talks only to a blit-daemon from the same build; the session
    53	  handshake REFUSES a mismatched peer outright. No negotiate-down, no
    54	  advisory fields, no feature-capability bits for version skew.
    55	  `Push`/`PullSync` are deleted at cutover with no bridge. (Old-path
    56	  code coexists in-tree during the migration slices solely so each
    57	  slice lands green — that is migration scaffolding, not wire
    58	  compatibility.)
    59	- Making different hardware perform identically. If src and dst sit
    60	  on different disks, the two *data directions* still differ by
    61	  physics; the invariant is that the same data direction between the
    62	  same endpoints is identical regardless of who initiates and which
    63	  verb is used.
    64	- WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
    65	- New features. This is a consolidation; capability parity with
    66	  today (mirror, filters, resume, fallback, delegation, progress,
    67	  jobs, cancellation) is the bar. Zero-copy receive is **unparked**
    68	  (D-2026-07-05-3, CPU-bound UNAS rig) but is a follow-on slice set
    69	  after cutover, not one of this plan's slices — see the Design note
    70	  on the write-strategy seam.
    71	
    72	## Constraints
    73	
    74	- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
    75	  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
    76	  violated at the choreography layer.
    77	- **Converge up, not down**: per benchmark cell, the unified session
    78	  must match the better of today's two directions (within ±10% run
    79	  noise), not their average. Unification that slows the fast
    80	  direction fails review.
    81	- REV4 invariants carry: byte-identical results, StallGuard,
    82	  cancellation, byte-accounting. Existing pins are ported (not
    83	  dropped) as tests become role-parameterized; test count never
    84	  drops.
    85	- The sf-2 shape-correction behavior (stream count corrects as the
    86	  need list accumulates) becomes the one and only stream policy —
    87	  both directions inherit it by construction; its pins carry over.
    88	- **The bounded-unilateral dial contract carries unchanged**
    89	  (D-2026-06-20-1/-2, REV4 Design §4): the byte SENDER owns the live
    90	  dial, bounded by the byte RECEIVER's advertised capacity profile
    91	  (`ue-r2-1b` fields; 0/absent = unknown = conservative, never
    92	  unlimited). The session's role model must express this — profile
    93	  travels DESTINATION→SOURCE at setup regardless of who initiated —
    94	  and otp-1's contract names it explicitly.
    95	- Wire contract discipline (REV4 rule): the unified session's proto —
    96	  messages, field numbers, capability negotiation, transport
    97	  selection — is a reviewed doc+proto slice **before** any behavior
    98	  depends on it.
    99	- Every slice through the codex loop (D-2026-07-04-1); tree green
   100	  after every slice; transitional coexistence of old+new paths is
   101	  scaffolding only — the plan is not Shipped until the deletion slice
   102	  lands and the deletion proof is recorded.
   103	- Windows parity: suite green on the owner's machine + windows-latest
   104	  CI before Shipped.
   105	
   106	## Acceptance criteria
   107	
   108	- [ ] **Initiator/verb invariance (the owner's sentence, measured)**:
   109	      on a symmetric rig (same filesystem class both ends, cold
   110	      caches, disk-to-disk), for each data direction and workload
   111	      (large / 10k-small / mixed): wall time initiating from end A vs
   112	      end B, and via push-verb vs pull-verb, differs only within
   113	      run-to-run noise (±10%). Matrix committed as evidence.
   114	- [ ] **Converge up, measured (codex F4)**: before cutover, the
   115	      corrected symmetric-fs harness records a per-cell baseline of
   116	      the OLD paths, both directions; after cutover, every unified
   117	      cell must be ≤ the better of that cell's two old directions
   118	      + run noise (±10%). A symmetric-but-slower result fails.
   119	- [ ] **Deletion proof**: `remote/pull.rs` (driver), `remote/push/`
   120	      (driver), daemon `push/control.rs` choreography, daemon
   121	      `pull_sync.rs` choreography, the delegated-pull driver, the
   122	      separate local orchestration path, and the `Push`/`PullSync`
   123	      RPCs no longer exist in the tree; one `TransferSession` and one
   124	      `Transfer` RPC remain. The `DelegatedPull` RPC may survive only
   125	      as trigger + progress relay — the proof must show it carries no
   126	      payload bytes (codex F3). Recorded file-by-file in the final
   127	      slice's finding doc.
   128	- [ ] Capability parity: mirror (both mirror-kinds + scan-complete
   129	      guard), filters, block-resume, gRPC fallback carrier, delegated
   130	      transfer, progress events, jobs/cancel, read-only enforcement —
   131	      each demonstrated by ported tests on the session.
   132	- [ ] Suite green throughout; final test count ≥ pre-plan baseline
   133	      (1483); all REV4 invariant pins and the sf-2 pin pass
   134	      role-parameterized.
   135	- [ ] Benchmark methodology corrected and recorded: symmetric-fs
   136	      cells are the verdict cells; tmpfs cells remain only as
   137	      explicitly-labeled wire-reference rows (never compared across
   138	      directions with asymmetric endpoints).
   139	- [ ] Windows: full suite green (owner machine) + windows-latest CI.
   140	
   141	## Design
   142	
   143	**What already is one code** (kept, becomes the session's engine):
   144	`remote/transfer/` — pipeline, sink/source abstractions, data plane,
   145	diff planner, tar-shard, stall guard, progress, `operation_spec` (the
   146	REV4 unified contract), and the engine dial (stream policy incl. sf-2
   147	shape correction). The defect layer is above it: four driver loops
   148	choreograph these pieces differently per direction.
   149	
   150	**The one choreography** (roles, not directions):
   151	
   152	1. Initiator opens the single bidi `Transfer` RPC and sends the
   153	   operation spec: which end is SOURCE, which is DESTINATION, path/
   154	   module, filters, mirror/resume flags, capabilities.
   155	2. SOURCE enumerates and **streams** its manifest immediately (no
   156	   buffered-enumeration phase — this generalizes push's fast start;
   157	   pull's full-enumeration-then-negotiate slow start is deleted, which
   158	   absorbs the "pull 1s-start" residue item).
   159	3. DESTINATION diffs incrementally against its own filesystem and
   160	   returns need-list batches (one diff owner, always the end that
   161	   owns the target fs — push's proven model; pull_sync's
   162	   source-side diff is deleted).
   163	4. The data plane opens at the dial floor immediately; stream count
   164	   shape-corrects as the need list accumulates (sf-2 mechanism, now
   165	   the only policy, both roles).
   166	5. SOURCE feeds payloads (files / tar-shards / resume blocks) through
   167	   the one pipeline into the data plane; DESTINATION writes through
   168	   the one receive path. The receive sink is built with a
   169	   **runtime-selected write-strategy seam**: buffered relay is the
   170	   universal strategy; capability-gated alternatives slot in behind
   171	   it without new paths — the first is zero-copy/splice
   172	   (D-2026-07-05-3, unparked for CPU-bound receivers like the
   173	   owner's UNAS 8 Pro; design input:
   174	   `ZERO_COPY_RECEIVE_EVAL.md` §If-FAST-evidence), landing as a
   175	   follow-on slice set after cutover. Strategy selection reads
   176	   capability and payload type, never role or initiator.
   177	6. Mirror: DESTINATION computes deletions from the completed source
   178	   manifest it received (filter-scoped, scan-complete-guarded) and
   179	   executes them locally. One rule, no per-direction delete
   180	   choreography.
   181	7. Resume: optional block-hash phase inside the same session, same
   182	   messages regardless of roles.
   183	8. Summary/byte-accounting: one record shape.
   184	
   185	**Transport facts vs choreography**: the connection-initiating end
   186	dials TCP data-plane sockets (NAT reality) — byte direction within a
   187	socket is set by role, not by who dialed. The gRPC-fallback lane
   188	becomes a *byte-carrier option* inside the same session (control-
   189	stream frames instead of TCP sockets), selected at negotiation — not
   190	a separate transfer path. Resize keeps its controller-at-sender rule.
   191	
   192	**Delegated transfer**: a daemon receiving a delegated request simply
   193	becomes an initiator of the same session against the other daemon
   194	(destination role on its module fs). The bespoke delegated-pull
   195	driver is deleted; the delegation *gate* (authorization) stays. The
   196	`DelegatedPull` RPC itself is client↔daemon trigger + progress relay
   197	(`DelegatedPullProgress` stream) — it never carries payload bytes;
   198	its handler shrinks to "authorize, spawn the session, relay the
   199	session's progress events." It stays wire-compatible or is folded at
   200	cutover — either way the deletion proof asserts no bytes flow
   201	through it (codex F3).
   202	
   203	**Resume ordering (RELIABLE exception, codex F5)**: resumed files use
   204	a strictly-ordered block-hash exchange — the DESTINATION's block map
   205	for a file must complete before the SOURCE sends any block of that
   206	file, and stale/mismatched partials fall back to full-file transfer.
   207	This is an explicit exception to the immediate-start rule, exactly as
   208	today's resume path is an explicit single-stream RELIABLE exception
   209	(ue-r2-1g finding note). otp-1 pins the phase ordering in the wire
   210	contract; otp-7 pins the stale-partial and mid-resume-failure cases
   211	in tests.
   212	
   213	**Local transfers**: the same session driver over an in-process
   214	transport (both roles in one process, no wire). The engine underneath
   215	is already shared; the separate local orchestration path is deleted
   216	in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
   217	guard the migration.
   218	
   219	**Affected crates**: `blit-core` (new `transfer_session` module;
   220	`remote/pull.rs`, `remote/push/` drivers deleted at cutover),
   221	`blit-daemon` (one `Transfer` handler replaces push/pull_sync/
   222	delegated handlers), `blit-cli`/`blit-app` (verbs map to roles),
   223	`proto/blit.proto` (one `Transfer` RPC; `Push`/`PullSync` deleted),
   224	`blit-tui` (progress/jobs consume the same events).
   225	
   226	**Risks**: largest consolidation since REV1 — pull.rs alone is ~108K;
   227	mitigated by strangler slices with the tree green throughout and a
   228	non-optional deletion slice. Per-cell regression risk on today's
   229	faster direction — mitigated by the converge-up constraint and
   230	baseline parity pins per slice. Wire break — lockstep upgrade,
   231	owner-controlled fleet. Windows receive paths (win_fs) — parity gate.
   232	Progress/jobs/TUI integration churn — the session emits the existing
   233	event contract (w6-1) at the same boundaries.
   234	
   235	## Slices
   236	
   237	One coherent, testable change per slice — sized for the `.review/`
   238	loop. Tree green after every slice; old paths keep working until
   239	otp-9 deletes them.
   240	
   241	1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
   242	   `Transfer` RPC and message set — roles, phases, field numbers,
   243	   the **strict same-build handshake** (exact protocol/build identity
   244	   exchanged at session open; any mismatch is refused with a clear
   245	   error — D-2026-07-05-2; pinned by test when the session lands),
   246	   the receiver capacity profile + bounded-unilateral dial contract
   247	   (D-2026-06-20-1/-2 — hardware negotiation, the only negotiation
   248	   that exists), transport selection, resume phase ordering (the
   249	   RELIABLE exception above), mirror phase, error/cancel semantics.
   250	   No feature-capability bits: same build implies same features.
   251	   The new proto text must carry NO version-tolerance semantics; the
   252	   capacity profile's absent/0 fields mean "unknown hardware value"
   253	   only, never "old peer" (today's proto comments frame some of that
   254	   contract as old-peer fallback — those comment blocks describe live
   255	   pre-cutover code and die with their messages at otp-10, per the
   256	   D-2026-07-05-2 review adjudication). Codex-reviewed before any
   257	   code consumes it.
   258	2. **otp-2 symmetric baseline (harness + rig, no production code)**:
   259	   correct the sf-1 harness matrix — same-fs disk-to-disk verdict
   260	   cells, cold caches, tmpfs rows re-labeled wire-reference only —

exec
/usr/bin/zsh -lc "nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '258,370p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   258	2. **otp-2 symmetric baseline (harness + rig, no production code)**:
   259	   correct the sf-1 harness matrix — same-fs disk-to-disk verdict
   260	   cells, cold caches, tmpfs rows re-labeled wire-reference only —
   261	   and record the OLD paths' per-cell, per-direction baseline on the
   262	   rig. This is the converge-up reference the acceptance criteria
   263	   compare against (codex F4).
   264	3. **otp-3 TransferSession core (blit-core)**: role-parameterized
   265	   state machine over the existing engine with an in-process
   266	   transport; unit/e2e tests run BOTH role assignments over the same
   267	   fixtures — the invariance property enters the test suite here.
   268	4. **otp-4 daemon serves `Transfer`, client initiates as SOURCE**
   269	   (remote push-equivalent rides the session); A/B parity pins vs
   270	   old push (byte-identical trees, summary parity, sf-2 pin ported).
   271	5. **otp-5 roles swapped: client initiates as DESTINATION** (pull-
   272	   equivalent) — the same code with roles flipped; the parity suite
   273	   reruns with no per-direction test code.
   274	6. **otp-6 mirror + filters** on the session (one delete rule).
   275	7. **otp-7 resume** block phase (ordering + stale-partial pins per
   276	   the Design's RELIABLE exception).
   277	8. **otp-8 fallback byte-carrier** (control-stream frames) as the
   278	   session's alternate transport.
   279	9. **otp-9 delegated transfer** = daemon-initiated session; bespoke
   280	   delegated-pull driver retired behind the existing gate;
   281	   `DelegatedPull` RPC reduced to trigger + progress relay.
   282	10. **otp-10 cutover + deletion**: CLI/app/TUI route every remote
   283	    operation through the session; `Push`/`PullSync` and all four
   284	    drivers deleted from the tree and the proto, no bridge
   285	    (D-2026-07-05-2); ported-test accounting proves count never
   286	    dropped. Deletion proof recorded, incl. the DelegatedPull
   287	    no-payload-bytes assertion.
   288	11. **otp-11 local transfers** ride the in-process transport; the
   289	    separate local orchestration is deleted; local perf pins hold.
   290	12. **otp-12 symmetric-rig acceptance run**: rerun the otp-2 matrix
   291	    on the unified path — initiator/verb invariance A/B within noise
   292	    AND every cell ≤ the better old direction + noise; committed as
   293	    this plan's acceptance evidence.
   294	13. **otp-13 verdict**: acceptance checklist walked with the owner;
   295	    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
   296	    against the unified baseline — owner call at that point.
   297	
   298	## Open questions
   299	
   300	- None requiring owner input now — scope, wire, and process were
   301	  delegated (Directive section). Slice-level unknowns (exact proto
   302	  shapes, resume edge semantics, TUI event wiring) are settled inside
   303	  their slices through the codex loop. — owner

exec
/usr/bin/zsh -lc "nl -ba docs/TRANSFER_SESSION.md | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	# TransferSession wire + session contract (otp-1)
     2	
     3	**Status**: Active (contract; implementation lands otp-3..otp-10)
     4	**Created**: 2026-07-05
     5	**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
     6	**Decision refs**: D-2026-07-05-1 (one path), D-2026-07-05-2
     7	(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)
     8	
     9	This document is the authoritative contract for the single `Transfer`
    10	RPC that replaces `Push` and `PullSync` at cutover (otp-10). Proto
    11	truth lives in `proto/blit.proto` under "ONE_TRANSFER_PATH unified
    12	session"; this doc explains the state machine the proto cannot.
    13	
    14	## Invariants
    15	
    16	1. **One vocabulary, role-tagged.** Both wire directions carry the
    17	   same frame type (`TransferFrame`). Which frames an end may send is
    18	   determined by its ROLE (SOURCE or DESTINATION), never by whether
    19	   it is the gRPC client or server. This is the structural form of
    20	   the owner's invariant: there is no push-shaped or pull-shaped
    21	   message set to diverge.
    22	2. **Same build only (D-2026-07-05-2).** The first frame each way is
    23	   `SessionHello{build_id, contract_version}`. Both ends compare for
    24	   EXACT equality; any mismatch → `SessionError{BUILD_MISMATCH}`
    25	   naming both ids, then stream close. No negotiate-down, no advisory
    26	   fields, no feature-capability bits — same build implies same
    27	   features. `build_id` = `<crate version>+<git commit hash>`
    28	   composed at compile time; `contract_version` is a belt-and-braces
    29	   integer bumped on any wire-shape change (exact match required).
    30	   Imprecise identities never false-match (otp-3 codex F1): a dirty
    31	   tree composes `<sha>.dirty.<content hash>` (deterministic — only
    32	   byte-identical dirty trees match), and a build without git
    33	   identity composes `unknown.<per-compilation entropy>` (only the
    34	   selfsame binary matches itself).
    35	3. **Roles.** The initiator (the end that opened the RPC — a CLI
    36	   client, or a daemon acting as delegated initiator) declares in
    37	   `SessionOpen` whether it is SOURCE or DESTINATION; the responder
    38	   (always a daemon) takes the other role. All four
    39	   initiator/role combinations run the identical state machine.
    40	4. **Diff owner = DESTINATION, always.** SOURCE streams its manifest
    41	   from live enumeration (immediate start — no buffered-enumeration
    42	   phase in any direction). DESTINATION diffs incrementally against
    43	   its own filesystem and streams need batches back. DESTINATION is
    44	   authoritative for what it has; SOURCE is authoritative for what
    45	   exists to send.
    46	5. **Dial contract carries (D-2026-06-20-1/-2).** The byte RECEIVER
    47	   (whichever end holds DESTINATION) advertises its
    48	   `CapacityProfile` at session open — in `SessionOpen` when the
    49	   initiator is DESTINATION, in `SessionAccept` when the responder
    50	   is. The byte SENDER (SOURCE) owns the live dial bounded by that
    51	   profile. Absent/0 profile fields mean "unknown hardware value" —
    52	   conservative defaults, never unlimited, and NEVER "old peer"
    53	   (there are no old peers).
    54	6. **One stream policy.** The data plane opens at the dial floor
    55	   immediately; SOURCE shape-corrects the stream count upward via
    56	   resize as the need list accumulates (the sf-2 mechanism —
    57	   `TransferDial::propose_shape_resize` — now the only policy).
    58	   SOURCE is the resize controller in every session.
    59	
    60	## Phase state machine
    61	
    62	```
    63	INITIATOR                                RESPONDER
    64	  |-- SessionHello ----------------------->|   (phase: HELLO)
    65	  |<------------------------ SessionHello--|
    66	  |     both verify build_id exact match; mismatch => SessionError + close
    67	  |-- SessionOpen ------------------------>|   (phase: OPEN)
    68	  |<---------------------- SessionAccept --|
    69	  |     responder validates module/path/read-only/gate here;
    70	  |     refusal is a SessionError, never a silent close
    71	  |                                        |
    72	  |==== from here the lanes are ROLES, not initiator/responder ====|
    73	  |  (whichever end holds SOURCE sends source-lane frames,          |
    74	  |   regardless of which end opened the RPC)                       |
    75	  |                                                                 |
    76	  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
    77	  |  DEST streams:    NeedBatch* ... NeedComplete                  |
    78	  |  SOURCE streams:  payload (data plane sockets, or in-stream    |
    79	  |                   frames when the in-stream carrier is chosen) |
    80	  |  SOURCE resize:   ResizeRequest -> DEST ResizeAck (per epoch)  |
    81	  |                                                                 |
    82	  |  resume exception (RELIABLE): a NeedBatch entry flagged         |
    83	  |  `resume=true` is followed by DEST's BlockHashList for that     |
    84	  |  file BEFORE SOURCE may send any byte of that file; stale or    |
    85	  |  mismatched partials fall back to full-file transfer.           |
    86	  |                                                                 |
    87	  |  mirror: DEST computes deletions LOCALLY from the completed     |
    88	  |  source manifest (filter-scoped, scan-complete-guarded) and     |
    89	  |  executes them itself. No delete list crosses the wire.         |
    90	  |                                                                 |
    91	  |  CLOSING (role-directed, both initiator layouts):               |
    92	  |    SOURCE -> DEST:  SourceDone (all requested payloads flushed) |
    93	  |    DEST -> SOURCE:  TransferSummary (DEST is the scorer)        |
    94	  |  then the INITIATOR closes the RPC stream.                      |
    95	```
    96	
    97	- Phase violations (a frame arriving in a phase where its role may
    98	  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
    99	  fail-fast, no tolerant parsing.
   100	- `NeedComplete` is DESTINATION's promise that no further need
   101	  batches follow (SOURCE may finish after flushing what was asked).
   102	  It may be sent only after BOTH: the source's `ManifestComplete`
   103	  has been received AND the destination has finished diffing every
   104	  received manifest entry. Mirror deletions additionally require the
   105	  scan-complete guard, as above.
   106	- **Flow control is the transport's, deliberately:** manifest, need,
   107	  and in-stream payload frames ride gRPC/HTTP-2 stream flow control;
   108	  each end holds only bounded internal queues (the engine's existing
   109	  batching — 128-entry manifest check chunks, need-list batcher).
   110	  Nothing in the contract requires unbounded buffering of the peer's
   111	  stream, and implementations must not introduce it.
   112	- `TransferSummary` always travels DESTINATION → SOURCE (the end
   113	  that wrote bytes and executed deletes is the end that can attest
   114	  to them), then the initiator surfaces it to the operator.
   115	
   116	## Frame set and field numbers
   117	
   118	`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`
   119	
   120	`TransferFrame.frame` oneof (field numbers frozen by this doc):
   121	
   122	| # | frame | sender | phase |
   123	|---|-------|--------|-------|
   124	| 1 | `SessionHello` | both, first frame | HELLO |
   125	| 2 | `SessionOpen` | initiator | OPEN |
   126	| 3 | `SessionAccept` | responder | OPEN |
   127	| 4 | `FileHeader manifest_entry` | SOURCE | streaming |
   128	| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
   129	| 6 | `NeedBatch need_batch` | DESTINATION | streaming |
   130	| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
   131	| 8 | `BlockHashList block_hashes` | DESTINATION | resume, per flagged file |
   132	| 9 | `FileHeader file_begin` | SOURCE | in-stream carrier |
   133	| 10 | `FileData file_data` | SOURCE | in-stream carrier |
   134	| 11 | `TarShardHeader tar_shard_header` | SOURCE | in-stream carrier |
   135	| 12 | `TarShardChunk tar_shard_chunk` | SOURCE | in-stream carrier |
   136	| 13 | `TarShardComplete tar_shard_complete` | SOURCE | in-stream carrier |
   137	| 14 | `BlockTransfer block` | SOURCE | resume |
   138	| 15 | `BlockTransferComplete block_complete` | SOURCE | resume |
   139	| 16 | `DataPlaneResize resize` | SOURCE | any (post-accept) |
   140	| 17 | `DataPlaneResizeAck resize_ack` | DESTINATION | any (post-accept) |
   141	| 18 | `SourceDone source_done` | SOURCE | closing |
   142	| 19 | `TransferSummary summary` | DESTINATION | closing |
   143	| 20 | `SessionError error` | both | any |
   144	
   145	Reused messages (`FileHeader`, `FileData`, `TarShard*`,
   146	`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
   147	`DataPlaneResize`/`Ack`, `FilterSpec`, `ComparisonMode`,
   148	`MirrorMode`, `ResumeSettings`, `CapacityProfile`) keep their
   149	existing shapes — the session reuses the engine's payload vocabulary
   150	verbatim. New messages (`SessionHello`, `SessionOpen`,
   151	`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
   152	`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
   153	defined in the proto with their field numbers.
   154	
   155	Deliberately absent: `PeerCapabilities` (same build = same
   156	features), `spec_version` negotiation (the hello's exact match
   157	replaces it), any delete list (mirror is destination-local), any
   158	push/pull-specific message.
   159	
   160	## Transport selection
   161	
   162	- **TCP data plane (default):** the RESPONDER binds the listener and
   163	  issues `DataPlaneGrant{tcp_port, session_token, initial_streams,
   164	  epoch0_sub_token}` inside `SessionAccept`; the INITIATOR always
   165	  dials (NAT/firewall reality — connection topology, not
   166	  choreography). Byte direction on the sockets is set by role:
   167	  SOURCE writes, DESTINATION reads.
   168	  **`initial_streams` is an ACCEPT ceiling, not a dial order**
   169	  (D-2026-06-20-1/-2 preserved): it is the number of epoch-0 accept
   170	  slots the responder arms, computed as min(engine dial floor,
   171	  DESTINATION's capacity ceiling). SOURCE — wherever it sits — owns
   172	  the dial and may use fewer epoch-0 sockets than armed; unclaimed
   173	  slots expire harmlessly. Growth beyond epoch 0 happens only via
   174	  SOURCE-initiated resize (sf-2 shape correction / tuner), one armed
   175	  accept per ADD epoch, exactly as ue-r2-2 built.
   176	  **Socket auth, exact:** every epoch-0 socket opens with
   177	  `session_token` (16 bytes) immediately followed by
   178	  `epoch0_sub_token` (16 bytes); every resize-ADD socket opens with
   179	  `session_token` followed by that epoch's `sub_token` from the
   180	  `DataPlaneResize` frame. Tokens are single-session; each armed
   181	  accept slot admits exactly one socket (no replay within a
   182	  session); armed slots that go unclaimed expire, as today's resize
   183	  wiring already does. A socket presenting anything else is closed
   184	  without response.
   185	- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
   186	  (operator `--force-grpc` diagnostics) or granted by the responder
   187	  when it cannot bind a data plane (`SessionAccept` with no grant).
   188	  Payload frames 9-15 ride the RPC itself. Same choreography, same
   189	  planner decisions, different byte carrier.
   190	  **Record grammar (fail-fast):** payload records on the
   191	  source-lane are STRICTLY SERIALIZED — after `file_begin(header)`,
   192	  only `file_data` frames for that file may follow on the lane until
   193	  the record completes; completion is inferred at exactly
   194	  `header.size` cumulative bytes (a `file_begin`/`tar_shard_header`/
   195	  `block` arriving early, or bytes overrunning `size`, is
   196	  `PROTOCOL_VIOLATION`). Tar-shard records run
   197	  `tar_shard_header … tar_shard_chunk* … tar_shard_complete`; block
   198	  records complete with `block_complete`. Payload records may begin
   199	  only AFTER the source's `ManifestComplete` — this per-transport
   200	  ordering rule applies identically to both roles and mirrors the
   201	  design-4-proven fallback ordering, so manifest frames and payload
   202	  records never interleave. DESTINATION-lane frames (need batches,
   203	  acks, summary) are unaffected — they travel the other direction.
   204	- **Local (in-process):** the identical session state machine runs
   205	  with both roles in one process over an in-process frame channel —
   206	  no RPC, no sockets (otp-11). Strategy selection (tar-shard vs
   207	  file vs block) is planner-owned and reads workload shape +
   208	  capability, never role/initiator/transport.
   209	
   210	## Errors, cancel, stall
   211	
   212	- `SessionError{code, message}` codes (plus both build ids on
   213	  BUILD_MISMATCH):
   214	  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
   215	  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
   216	  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
   217	  or aborts says why before closing; operators never diagnose from a
   218	  bare stream reset.
   219	- `CancelJob` interop: the responder registers the session in
   220	  `ActiveJobs` at OPEN (same transfer_id contract as today); the
   221	  cancel token races the session exactly as w4-3 wired, and the
   222	  peer receives `SessionError{CANCELLED}`.
   223	- StallGuard, byte-accounting, and progress events (w6-1 contract)
   224	  attach at the same boundaries they do today; the session emits the
   225	  existing `DaemonEvent` payloads.
   226	
   227	## What this replaces
   228	
   229	At cutover (otp-10): `Push`, `PullSync`, and their message
   230	choreographies are deleted from the proto and the tree; the four
   231	per-direction drivers die with them; `DelegatedPull` shrinks to
   232	trigger + progress relay (no payload bytes). Until then the old paths
   233	stay live alongside the session as migration scaffolding.
   234	
   235	Progress: otp-3 landed the role-parameterized drivers over the
   236	in-process transport; **otp-4a** made the daemon serve `Transfer` for
   237	real (runs `run_destination` as Responder; a client `run_source`s as
   238	SOURCE initiator over gRPC, in-stream carrier) — the RPC no longer
   239	returns `UNIMPLEMENTED`. The TCP data plane grant + resize land at
   240	otp-4b; the daemon-as-SOURCE (pull-equivalent) layout at otp-5.

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/core.rs | sed -n '1400,1565p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1400	                let _ = tx.send(Err(status)).await;
  1401	            }
  1402	            (ok, err_msg)
  1403	        }
  1404	        // Same disambiguation the delegated_pull closure uses: a fired
  1405	        // token means the cause was CancelJob; otherwise the client
  1406	        // hung up.
  1407	        None if cancel_token.is_cancelled() => {
  1408	            let _ = tx
  1409	                .send(Err(Status::cancelled("transfer cancelled via CancelJob")))
  1410	                .await;
  1411	            (false, Some("cancelled via CancelJob".to_string()))
  1412	        }
  1413	        None => (false, Some("client cancelled".to_string())),
  1414	    }
  1415	}
  1416	
  1417	/// Session variant of [`resolve_streaming_outcome`] for the `Transfer`
  1418	/// RPC: identical hangup / completion / fault handling, but on
  1419	/// `CancelJob` it emits a framed `SessionError{CANCELLED}` on the
  1420	/// response stream instead of a bare `Status::cancelled` (otp-4a codex
  1421	/// F1). The session speaks `TransferFrame`s, so the client reads the
  1422	/// framed error — and the aborted session future can't send it itself
  1423	/// once the select drops it, so the dispatcher does. A session that
  1424	/// faults on its own already framed the reason; the trailing `Status`
  1425	/// on that branch is belt-and-braces for a pre-frame transport break.
  1426	async fn resolve_transfer_session_outcome<H>(
  1427	    handler: H,
  1428	    tx: &mpsc::Sender<Result<blit_core::generated::TransferFrame, Status>>,
  1429	    cancel_token: &CancellationToken,
  1430	    metrics: &TransferMetrics,
  1431	) -> (bool, Option<String>)
  1432	where
  1433	    H: std::future::Future<Output = Result<(), Status>>,
  1434	{
  1435	    let outcome =
  1436	        resolve_transfer_outcome(handler, tx.closed(), cancel_token.cancelled(), false).await;
  1437	    match outcome {
  1438	        Some(result) => {
  1439	            let (ok, err_msg) = outcome_from_status(&result);
  1440	            if let Err(status) = result {
  1441	                metrics.inc_error();
  1442	                let _ = tx.send(Err(status)).await;
  1443	            }
  1444	            (ok, err_msg)
  1445	        }
  1446	        None if cancel_token.is_cancelled() => {
  1447	            let _ = tx
  1448	                .send(Ok(blit_core::transfer_session::session_error_frame(
  1449	                    blit_core::generated::session_error::Code::Cancelled,
  1450	                    "transfer cancelled via CancelJob",
  1451	                )))
  1452	                .await;
  1453	            (false, Some("cancelled via CancelJob".to_string()))
  1454	        }
  1455	        None => (false, Some("client cancelled".to_string())),
  1456	    }
  1457	}
  1458	
  1459	/// Translate a handler's `Result<_, Status>` into the
  1460	/// `(ok, error_message)` pair the ActiveJobs guard expects.
  1461	/// Used inside [`resolve_streaming_outcome`] for the `push` /
  1462	/// `pull_sync` dispatchers. `delegated_pull` has its own shape
  1463	/// (handler returns `bool` inside a select) and inlines the
  1464	/// equivalent mapping there.
  1465	fn outcome_from_status<T>(result: &Result<T, Status>) -> (bool, Option<String>) {
  1466	    match result {
  1467	        Ok(_) => (true, None),
  1468	        Err(status) => (false, Some(status.message().to_string())),
  1469	    }
  1470	}
  1471	
  1472	#[cfg(test)]
  1473	mod tests {
  1474	    use super::*;
  1475	    use crate::active_jobs::ActiveJobKind;
  1476	    use blit_core::generated::TransferKind as WireKind;
  1477	
  1478	    fn empty_service() -> BlitService {
  1479	        BlitService::with_modules(HashMap::new(), false)
  1480	    }
  1481	
  1482	    /// audit-10: a handler that has completed must win the `biased`
  1483	    /// select even when the cancel token (and the client-hangup signal)
  1484	    /// have ALSO fired — otherwise a transfer that succeeded at the same
  1485	    /// instant `CancelJob` fired gets mis-recorded as cancelled.
  1486	    /// (Helper renamed `resolve_delegated_pull_outcome` →
  1487	    /// `resolve_transfer_outcome` in w4-3; same select, now generic.)
  1488	    #[tokio::test]
  1489	    async fn resolve_pull_handler_completion_wins_over_simultaneous_cancel() {
  1490	        use std::future::ready;
  1491	        // Handler ready(success); client hung up; cancel fired — all
  1492	        // simultaneously. Handler-first ordering must yield Some(true).
  1493	        let outcome = resolve_transfer_outcome(ready(true), ready(()), ready(()), false).await;
  1494	        assert_eq!(outcome, Some(true), "ready success must win the race");
  1495	
  1496	        // The same holds for a handler that completed with failure: a
  1497	        // real result beats a simultaneous cancel.
  1498	        let outcome = resolve_transfer_outcome(ready(false), ready(()), ready(()), false).await;
  1499	        assert_eq!(outcome, Some(false), "ready failure must win the race");
  1500	    }
  1501	
  1502	    /// audit-10: a still-running (Pending) handler must still yield to a
  1503	    /// `CancelJob` cancel — the fix must not make transfers
  1504	    /// uncancellable.
  1505	    #[tokio::test]
  1506	    async fn resolve_pull_pending_handler_yields_to_cancel() {
  1507	        use std::future::{pending, ready};
  1508	        let outcome = resolve_transfer_outcome(
  1509	            pending::<bool>(), // handler still running
  1510	            pending::<()>(),   // client still connected
  1511	            ready(()),         // CancelJob fired
  1512	            false,
  1513	        )
  1514	        .await;
  1515	        assert_eq!(outcome, None, "a running handler must yield to cancel");
  1516	    }
  1517	
  1518	    /// otp-4a codex F1: a `CancelJob` on a served `Transfer` session
  1519	    /// must reach the client as a framed `SessionError{CANCELLED}` on
  1520	    /// the response stream — not a bare `Status::cancelled` (the
  1521	    /// session speaks frames, and the aborted session future can't
  1522	    /// send it itself). Guard: with the cancel branch reverted to
  1523	    /// `Err(Status::cancelled)` this fails (no `Ok` error frame lands).
  1524	    #[tokio::test]
  1525	    async fn transfer_cancel_emits_framed_cancelled_error() {
  1526	        use blit_core::generated::session_error::Code;
  1527	        use blit_core::generated::transfer_frame::Frame as WireFrame;
  1528	
  1529	        let (tx, mut rx) = mpsc::channel::<Result<blit_core::generated::TransferFrame, Status>>(4);
  1530	        let cancel = CancellationToken::new();
  1531	        cancel.cancel();
  1532	        let metrics = TransferMetrics::disabled();
  1533	        // A session that never completes on its own — cancel must win.
  1534	        let never = std::future::pending::<Result<(), Status>>();
  1535	        let (ok, msg) = resolve_transfer_session_outcome(never, &tx, &cancel, &metrics).await;
  1536	        assert!(!ok, "cancel is not a success");
  1537	        assert_eq!(msg.as_deref(), Some("cancelled via CancelJob"));
  1538	
  1539	        let frame = rx
  1540	            .recv()
  1541	            .await
  1542	            .expect("a terminal frame")
  1543	            .expect("a framed SessionError, not a gRPC Status");
  1544	        match frame.frame {
  1545	            Some(WireFrame::Error(err)) => assert_eq!(
  1546	                err.code,
  1547	                Code::Cancelled as i32,
  1548	                "cancel must emit a framed CANCELLED SessionError"
  1549	            ),
  1550	            other => panic!("expected a CANCELLED error frame, got {other:?}"),
  1551	        }
  1552	    }
  1553	
  1554	    /// audit-10 / m-jobs-3: with `detach = true` the client-hangup branch
  1555	    /// is disabled, so a closed tx must NOT terminate the pull.
  1556	    #[tokio::test]
  1557	    async fn resolve_pull_detach_disables_client_hangup() {
  1558	        use std::future::{pending, ready};
  1559	        let fut = resolve_transfer_outcome(
  1560	            pending::<bool>(), // handler still running
  1561	            ready(()),         // client hung up...
  1562	            pending::<()>(),   // ...but no cancel
  1563	            true,              // detached
  1564	        );
  1565	        // tx_closed is ready but gated off by detach; nothing else is

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/transfer_session_e2e.rs | sed -n '1,380p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! ONE_TRANSFER_PATH otp-4a/4b loopback e2e: the daemon serves the
     2	//! unified `Transfer` session and a real client initiates it as SOURCE
     3	//! over gRPC. otp-4b makes the default carrier the **TCP data plane**
     4	//! (the responder grants it in `SessionAccept`, the client dials +
     5	//! authenticates + sends payloads over sockets); the in-stream carrier
     6	//! stays live as the requested fallback. These tests pin the
     7	//! push-equivalent behavior over both carriers:
     8	//!
     9	//! - a session lands bytes byte-identically and scores them correctly,
    10	//!   over the data plane and over the in-stream fallback;
    11	//! - **A/B parity**: the same fixture through OLD push and the NEW
    12	//!   session (data plane) yields byte-identical destination trees +
    13	//!   equal shared summary counters (the converge-up bar);
    14	//! - responder refusals (read-only module, unknown module) arrive as
    15	//!   `SessionError` frames, surfaced to the client as faults;
    16	//! - the unified SizeMtime semantic: a same-size destination file that
    17	//!   is NEWER than the source is SKIPPED (the data-safe, pull-style
    18	//!   converged behavior — see the finding doc's compare decision).
    19	//!
    20	//! Harness mirrors `push/shape_resize_e2e.rs`: a real in-process
    21	//! `BlitService` on loopback + a real client. Only in-crate tests can
    22	//! build `ModuleConfig`/`BlitService::with_modules`, so this lives in
    23	//! blit-daemon.
    24	
    25	use std::collections::{BTreeMap, HashMap};
    26	use std::path::{Path, PathBuf};
    27	use std::sync::Arc;
    28	
    29	use blit_core::fs_enum::FileFilter;
    30	use blit_core::generated::blit_server::BlitServer;
    31	use blit_core::generated::{session_error, MirrorMode};
    32	use blit_core::remote::transfer::session_client::{run_push_session, PushSessionOptions};
    33	use blit_core::remote::transfer::source::FsTransferSource;
    34	use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePushClient};
    35	use blit_core::transfer_session::SessionFault;
    36	use tokio::sync::oneshot;
    37	
    38	use crate::runtime::ModuleConfig;
    39	use crate::service::BlitService;
    40	
    41	// ---------------------------------------------------------------------------
    42	// Harness
    43	// ---------------------------------------------------------------------------
    44	
    45	/// A running in-process daemon exposing module "test" over a writable
    46	/// (or read-only) temp dir, and the loopback endpoint targeting it.
    47	struct Daemon {
    48	    endpoint: RemoteEndpoint,
    49	    shutdown: Option<oneshot::Sender<()>>,
    50	    server: Option<tokio::task::JoinHandle<()>>,
    51	    _dest: tempfile::TempDir,
    52	    dest_root: PathBuf,
    53	    active_jobs: crate::active_jobs::ActiveJobs,
    54	}
    55	
    56	impl Daemon {
    57	    async fn start(read_only: bool) -> Self {
    58	        let dest = tempfile::tempdir().expect("dest dir");
    59	        let canonical = dest.path().canonicalize().expect("canonical dest");
    60	        let mut modules = HashMap::new();
    61	        modules.insert(
    62	            "test".to_string(),
    63	            ModuleConfig {
    64	                name: "test".into(),
    65	                path: canonical.clone(),
    66	                canonical_root: canonical.clone(),
    67	                read_only,
    68	                _comment: None,
    69	                delegation_allowed: true,
    70	            },
    71	        );
    72	        let service = BlitService::with_modules(modules, false);
    73	        let active_jobs = service.active_jobs.clone();
    74	        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
    75	            .await
    76	            .expect("bind loopback listener");
    77	        let port = listener.local_addr().expect("listener addr").port();
    78	        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    79	        let server = tokio::spawn(async move {
    80	            blit_core::remote::grpc_server::production_server_builder()
    81	                .add_service(BlitServer::new(service))
    82	                .serve_with_incoming_shutdown(
    83	                    tokio_stream::wrappers::TcpListenerStream::new(listener),
    84	                    async {
    85	                        let _ = shutdown_rx.await;
    86	                    },
    87	                )
    88	                .await
    89	                .expect("in-process daemon serves");
    90	        });
    91	        let endpoint = RemoteEndpoint {
    92	            host: "127.0.0.1".into(),
    93	            port,
    94	            path: RemotePath::Module {
    95	                module: "test".into(),
    96	                rel_path: PathBuf::new(),
    97	            },
    98	        };
    99	        Daemon {
   100	            endpoint,
   101	            shutdown: Some(shutdown_tx),
   102	            server: Some(server),
   103	            _dest: dest,
   104	            dest_root: canonical,
   105	            active_jobs,
   106	        }
   107	    }
   108	
   109	    /// Endpoint pointing at a module name that isn't configured.
   110	    fn endpoint_for_missing_module(&self) -> RemoteEndpoint {
   111	        RemoteEndpoint {
   112	            host: self.endpoint.host.clone(),
   113	            port: self.endpoint.port,
   114	            path: RemotePath::Module {
   115	                module: "nope".into(),
   116	                rel_path: PathBuf::new(),
   117	            },
   118	        }
   119	    }
   120	
   121	    async fn stop(mut self) {
   122	        if let Some(tx) = self.shutdown.take() {
   123	            let _ = tx.send(());
   124	        }
   125	        if let Some(server) = self.server.take() {
   126	            server.await.expect("server task joins");
   127	        }
   128	    }
   129	}
   130	
   131	type FileSpec = (&'static str, &'static [u8], i64);
   132	
   133	fn write_tree(root: &Path, files: &[FileSpec]) {
   134	    for (rel, content, mtime) in files {
   135	        let path = root.join(rel);
   136	        if let Some(parent) = path.parent() {
   137	            std::fs::create_dir_all(parent).unwrap();
   138	        }
   139	        std::fs::write(&path, content).unwrap();
   140	        filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(*mtime, 0)).unwrap();
   141	    }
   142	}
   143	
   144	/// rel-path → bytes for every regular file under `root`. Content only
   145	/// (byte-identical), copied from the role suite — no shared test util
   146	/// exists across crates yet.
   147	fn collect_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
   148	    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
   149	        for entry in std::fs::read_dir(dir).unwrap() {
   150	            let entry = entry.unwrap();
   151	            let path = entry.path();
   152	            if path.is_dir() {
   153	                walk(root, &path, out);
   154	            } else {
   155	                let rel = path
   156	                    .strip_prefix(root)
   157	                    .unwrap()
   158	                    .to_string_lossy()
   159	                    .replace('\\', "/");
   160	                out.insert(rel, std::fs::read(&path).unwrap());
   161	            }
   162	        }
   163	    }
   164	    let mut out = BTreeMap::new();
   165	    if root.exists() {
   166	        walk(root, root, &mut out);
   167	    }
   168	    out
   169	}
   170	
   171	fn assert_trees_identical(a: &Path, b: &Path) {
   172	    let ta = collect_tree(a);
   173	    let tb = collect_tree(b);
   174	    assert_eq!(
   175	        ta.keys().collect::<Vec<_>>(),
   176	        tb.keys().collect::<Vec<_>>(),
   177	        "path sets differ between {a:?} and {b:?}"
   178	    );
   179	    for (rel, bytes) in &ta {
   180	        assert_eq!(bytes, &tb[rel], "content differs for '{rel}'");
   181	    }
   182	}
   183	
   184	fn small_tree() -> Vec<FileSpec> {
   185	    vec![
   186	        ("a.txt", b"alpha", 1_600_000_001),
   187	        ("empty.bin", b"", 1_600_000_002),
   188	        ("dir one/b.log", b"beta beta beta", 1_600_000_003),
   189	        ("dir one/deeper/c.dat", b"gamma-content", 1_600_000_004),
   190	    ]
   191	}
   192	
   193	fn fault_of(err: &eyre::Report) -> &SessionFault {
   194	    err.downcast_ref::<SessionFault>()
   195	        .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
   196	}
   197	
   198	// --- otp-4b-3: deterministic mid-transfer cancel over the data plane ---
   199	
   200	/// A `TransferSource` that puts a transfer into a provably-stuck
   201	/// mid-payload state: `open_file` writes exactly one 64 KiB chunk over
   202	/// the data plane (so bytes have demonstrably flowed), signals `started`,
   203	/// then blocks forever without emitting the rest of the file. The
   204	/// transfer therefore cannot complete on its own — the only exits are the
   205	/// cancel under test or the reader being dropped when the session aborts.
   206	/// Everything else delegates to the real filesystem source.
   207	struct StuckAfterFirstChunkSource {
   208	    inner: FsTransferSource,
   209	    started: Arc<tokio::sync::Notify>,
   210	}
   211	
   212	#[async_trait::async_trait]
   213	impl blit_core::remote::transfer::source::TransferSource for StuckAfterFirstChunkSource {
   214	    fn scan(
   215	        &self,
   216	        filter: Option<FileFilter>,
   217	        unreadable: Arc<std::sync::Mutex<Vec<String>>>,
   218	    ) -> (
   219	        tokio::sync::mpsc::Receiver<blit_core::generated::FileHeader>,
   220	        tokio::task::JoinHandle<eyre::Result<u64>>,
   221	    ) {
   222	        self.inner.scan(filter, unreadable)
   223	    }
   224	
   225	    async fn prepare_payload(
   226	        &self,
   227	        payload: blit_core::remote::transfer::payload::TransferPayload,
   228	    ) -> eyre::Result<blit_core::remote::transfer::payload::PreparedPayload> {
   229	        self.inner.prepare_payload(payload).await
   230	    }
   231	
   232	    async fn check_availability(
   233	        &self,
   234	        headers: Vec<blit_core::generated::FileHeader>,
   235	        unreadable: Arc<std::sync::Mutex<Vec<String>>>,
   236	    ) -> eyre::Result<Vec<blit_core::generated::FileHeader>> {
   237	        self.inner.check_availability(headers, unreadable).await
   238	    }
   239	
   240	    async fn open_file(
   241	        &self,
   242	        header: &blit_core::generated::FileHeader,
   243	    ) -> eyre::Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
   244	        let mut inner = self.inner.open_file(header).await?;
   245	        // A generous duplex buffer so the one chunk lands without the
   246	        // writer parking on backpressure.
   247	        let (mut w, r) = tokio::io::duplex(256 * 1024);
   248	        let started = Arc::clone(&self.started);
   249	        tokio::spawn(async move {
   250	            use tokio::io::{AsyncReadExt, AsyncWriteExt};
   251	            let mut buf = vec![0u8; 64 * 1024];
   252	            if let Ok(n) = inner.read(&mut buf).await {
   253	                if n > 0 && w.write_all(&buf[..n]).await.is_ok() {
   254	                    started.notify_one();
   255	                }
   256	            }
   257	            // Hold the write half open (no EOF) and never write again:
   258	            // the transfer is now stuck mid-payload until the session is
   259	            // aborted (which drops this task) or cancelled.
   260	            std::future::pending::<()>().await;
   261	            drop(w);
   262	        });
   263	        Ok(Box::new(r))
   264	    }
   265	
   266	    fn root(&self) -> &Path {
   267	        self.inner.root()
   268	    }
   269	}
   270	
   271	/// otp-4b-3: fire a `CancelJob`-equivalent (the row's cancellation token,
   272	/// exactly what the RPC handler fires) while a payload is stuck mid-flight
   273	/// over the TCP data plane. The client must surface
   274	/// `SessionFault{CANCELLED}` — the peer's framed abort reason — rather
   275	/// than the data-plane transport break it also causes, and it must not
   276	/// hang. The daemon must then tear the job down cleanly (the active row
   277	/// drains).
   278	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   279	async fn mid_transfer_cancel_surfaces_cancelled_over_the_data_plane() {
   280	    let daemon = Daemon::start(false).await;
   281	    let src = tempfile::tempdir().unwrap();
   282	    // One file larger than a single chunk, so the stuck reader keeps the
   283	    // transfer provably incomplete after its first 64 KiB.
   284	    std::fs::write(src.path().join("big.bin"), vec![0xABu8; 4 * 1024 * 1024]).unwrap();
   285	
   286	    let started = Arc::new(tokio::sync::Notify::new());
   287	    let source = Arc::new(StuckAfterFirstChunkSource {
   288	        inner: FsTransferSource::new(src.path().to_path_buf()),
   289	        started: Arc::clone(&started),
   290	    });
   291	
   292	    let ep = daemon.endpoint.clone();
   293	    let client =
   294	        tokio::spawn(
   295	            async move { run_push_session(&ep, source, PushSessionOptions::default()).await },
   296	        );
   297	
   298	    // Bytes have flowed over the data plane and the transfer is now stuck
   299	    // mid-payload — a deterministic mid-transfer point.
   300	    tokio::time::timeout(std::time::Duration::from_secs(10), started.notified())
   301	        .await
   302	        .expect("payload bytes should flow over the data plane before cancel");
   303	
   304	    // Fire the row's cancellation token — exactly what the `CancelJob` RPC
   305	    // handler does via `cancel_authorized` (audit-9). The RPC-level
   306	    // mapping (auth, outcome codes) is unit-tested separately; this pins
   307	    // the end-to-end propagation through the served session.
   308	    let transfer_id = daemon
   309	        .active_jobs
   310	        .snapshot()
   311	        .into_iter()
   312	        .next()
   313	        .expect("an active transfer row")
   314	        .transfer_id;
   315	    assert_eq!(
   316	        daemon.active_jobs.cancel(&transfer_id),
   317	        crate::active_jobs::CancelOutcome::Cancelled,
   318	        "the served session's row honors cancellation"
   319	    );
   320	
   321	    // The client must surface CANCELLED promptly (no hang).
   322	    let result = tokio::time::timeout(std::time::Duration::from_secs(10), client)
   323	        .await
   324	        .expect("client must not hang on a mid-transfer cancel")
   325	        .expect("client task joins");
   326	    let err = result.expect_err("a cancelled transfer fails");
   327	    assert_eq!(
   328	        fault_of(&err).code,
   329	        session_error::Code::Cancelled,
   330	        "the client surfaces the peer's framed CANCELLED, not the data-plane break: {err:#}"
   331	    );
   332	
   333	    // Daemon tears down cleanly: the active row drains.
   334	    let mut drained = false;
   335	    for _ in 0..200 {
   336	        if daemon.active_jobs.snapshot().is_empty() {
   337	            drained = true;
   338	            break;
   339	        }
   340	        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
   341	    }
   342	    assert!(
   343	        drained,
   344	        "the daemon must drain the cancelled job from active[]"
   345	    );
   346	
   347	    daemon.stop().await;
   348	}
   349	
   350	// ---------------------------------------------------------------------------
   351	// Tests
   352	// ---------------------------------------------------------------------------
   353	
   354	#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
   355	async fn session_lands_bytes_over_the_data_plane() {
   356	    let daemon = Daemon::start(false).await;
   357	    let src = tempfile::tempdir().unwrap();
   358	    write_tree(src.path(), &small_tree());
   359	
   360	    // Default options ⇒ TCP data plane: the responder grants it and the
   361	    // client dials + sends payloads over sockets (otp-4b).
   362	    let source = Arc::new(FsTransferSource::new(src.path().to_path_buf()));
   363	    let summary = run_push_session(&daemon.endpoint, source, PushSessionOptions::default())
   364	        .await
   365	        .expect("session push succeeds");
   366	
   367	    assert_eq!(summary.files_transferred, small_tree().len() as u64);
   368	    assert_eq!(
   369	        summary.bytes_transferred,
   370	        small_tree()
   371	            .iter()
   372	            .map(|(_, c, _)| c.len() as u64)
   373	            .sum::<u64>()
   374	    );
   375	    assert!(
   376	        !summary.in_stream_carrier_used,
   377	        "otp-4b default rides the TCP data plane, not the in-stream carrier"
   378	    );
   379	    assert_trees_identical(src.path(), &daemon.dest_root);
   380	    daemon.stop().await;

codex
One concrete test concern is emerging: the new e2e’s “bytes flowed” signal is tied to a duplex buffer write inside the fake source, not to an observed TCP data-plane write/read. I’m checking whether there are other production-path issues before I call that out.
exec
/usr/bin/zsh -lc "git show --format= --patch 3ae0a5f -- '*.rs' | rg -n \""'^[+-]'"\\s*#\\[(tokio::test|test)\"" in /home/michael/dev/Blit
 succeeded in 0ms:
98:+    #[tokio::test]
241:+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]

exec
/usr/bin/zsh -lc "git grep -n \"fn execute_sink_pipeline_elastic\\|execute_sink_pipeline_elastic\" 3ae0a5f -- crates/blit-core/src/remote crates/blit-core/src/transfer_session" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "git grep -n \"struct AbortOnDrop\\|impl.*AbortOnDrop\\|pub struct AbortOnDrop\" 3ae0a5f -- crates/blit-core/src crates/blit-daemon/src" in /home/michael/dev/Blit
 succeeded in 0ms:
3ae0a5f:crates/blit-core/src/remote/push/client/mod.rs:36:    execute_sink_pipeline, execute_sink_pipeline_elastic, SinkControl,
3ae0a5f:crates/blit-core/src/remote/push/client/mod.rs:298:            execute_sink_pipeline_elastic(
3ae0a5f:crates/blit-core/src/remote/transfer/mod.rs:29:    execute_sink_pipeline, execute_sink_pipeline_elastic, execute_sink_pipeline_streaming,
3ae0a5f:crates/blit-core/src/remote/transfer/pipeline.rs:84:    execute_sink_pipeline_elastic(source, sinks, payload_rx, prefetch, progress, None).await
3ae0a5f:crates/blit-core/src/remote/transfer/pipeline.rs:110:pub async fn execute_sink_pipeline_elastic(
3ae0a5f:crates/blit-core/src/remote/transfer/pipeline.rs:1643:            execute_sink_pipeline_elastic(source, vec![first], rx, 2, None, Some(ctl_rx)).await
3ae0a5f:crates/blit-core/src/remote/transfer/pipeline.rs:1717:            execute_sink_pipeline_elastic(source, vec![keep, victim], rx, 2, None, Some(ctl_rx))
3ae0a5f:crates/blit-core/src/remote/transfer/pipeline.rs:1775:        let outcome = execute_sink_pipeline_elastic(source, vec![only], rx, 2, None, Some(ctl_rx))
3ae0a5f:crates/blit-core/src/remote/transfer/pipeline.rs:1818:            execute_sink_pipeline_elastic(source, vec![first], rx, 2, None, Some(ctl_rx)).await
3ae0a5f:crates/blit-core/src/transfer_session/data_plane.rs:51:    execute_sink_pipeline_elastic, generate_sub_token, AbortOnDrop, DataPlaneSession, SinkControl,
3ae0a5f:crates/blit-core/src/transfer_session/data_plane.rs:513:        execute_sink_pipeline_elastic(

 succeeded in 0ms:
3ae0a5f:crates/blit-core/src/remote/transfer/abort_on_drop.rs:25:pub struct AbortOnDrop<T>(Option<JoinHandle<T>>);
3ae0a5f:crates/blit-core/src/remote/transfer/abort_on_drop.rs:27:impl<T> AbortOnDrop<T> {
3ae0a5f:crates/blit-core/src/remote/transfer/abort_on_drop.rs:56:impl<T> Drop for AbortOnDrop<T> {

exec
/usr/bin/zsh -lc "git grep -n \"trait TransferSink\\|write_file_stream\\|struct DataPlaneSink\\|impl TransferSink for DataPlaneSink\" 3ae0a5f -- crates/blit-core/src/remote/transfer" in /home/michael/dev/Blit
 succeeded in 0ms:
3ae0a5f:crates/blit-core/src/remote/transfer/payload.rs:74:/// payload variant — they go through `TransferSink::write_file_stream`
3ae0a5f:crates/blit-core/src/remote/transfer/pipeline.rs:447:                    .write_file_stream(&header, &mut reader)
3ae0a5f:crates/blit-core/src/remote/transfer/pipeline.rs:1120:        async fn write_file_stream(
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:44:pub trait TransferSink: Send + Sync {
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:55:    async fn write_file_stream(
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:61:            "{} does not support write_file_stream (called for {})",
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:130:    /// `write_payload`/`write_file_stream` pushes its `relative_path`.
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:133:    /// `write_file_stream` passes it into
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:174:    /// `write_file_stream` reports every chunk the data plane
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:300:        // write_payload, not write_file_stream, so the chunk-
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:309:        // `write_file_stream`'s dry-run branch.
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:319:    async fn write_file_stream(
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:462:    // write_file_stream uses.
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:718:    // dance as write_file_stream — see commit 946bd77).
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:749:pub struct DataPlaneSink<P: Probe = NoProbe> {
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:807:    async fn write_file_stream(
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:884:    async fn write_file_stream(
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:1375:    /// R58-F4 regression for the streaming receive path. `write_file_stream`
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:1380:    async fn fs_sink_dry_run_write_file_stream_does_not_create_dirs() {
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:1401:        let outcome = sink.write_file_stream(&header, &mut reader).await.unwrap();
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:1899:        // Use write_file_stream so we exercise the sink.rs:218 site that
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:1903:        let result = sink.write_file_stream(&header, &mut empty).await;
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:1977:            .write_file_stream(&header, &mut reader)
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:2013:            .write_file_stream(&header, &mut reader)
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:2062:            .write_file_stream(&header, &mut reader)
3ae0a5f:crates/blit-core/src/remote/transfer/sink.rs:2208:    /// not `write_file_stream`, so the chunk-granular byte hook

codex
The diff adds exactly two Rust test attributes and removes none, matching the advertised 1513→1515 accounting at the diff level. I’m verifying the data-plane write path now to make the e2e rationale precise.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '360,510p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   360	                        }
   361	                    }
   362	                }
   363	            }
   364	        }
   365	    }
   366	    // ue-r2-2 review (panel F2, second half): an Add can still be
   367	    // queued in the instant between the last join and the break.
   368	    // Close its sink cleanly — the END record is what keeps the
   369	    // already-authorized peer worker from dying on a reset.
   370	    if let Some(rx) = control_rx.as_mut() {
   371	        while let Ok(cmd) = rx.try_recv() {
   372	            if let SinkControl::Add(sink) = cmd {
   373	                let _ = sink.finish().await;
   374	            }
   375	        }
   376	    }
   377	    drop(work_rx);
   378	    let _ = forwarder.await;
   379	
   380	    if let Some(err) = first_err {
   381	        return Err(err);
   382	    }
   383	
   384	    let result = total.lock().unwrap().clone();
   385	    Ok(result)
   386	}
   387	
   388	// =====================================================================
   389	// Receive pipeline — symmetric counterpart of execute_sink_pipeline.
   390	// =====================================================================
   391	
   392	use crate::generated::FileHeader;
   393	use eyre::bail;
   394	use tokio::io::{AsyncRead, AsyncReadExt};
   395	
   396	use super::data_plane::{
   397	    DATA_PLANE_RECORD_BLOCK, DATA_PLANE_RECORD_BLOCK_COMPLETE, DATA_PLANE_RECORD_END,
   398	    DATA_PLANE_RECORD_FILE, DATA_PLANE_RECORD_TAR_SHARD,
   399	};
   400	
   401	/// Drive a `TransferSink` from a TCP wire stream.
   402	///
   403	/// This is the symmetric counterpart to [`execute_sink_pipeline_streaming`]:
   404	/// where the outbound executor takes a [`TransferSource`] and dispatches
   405	/// payloads round-robin across N sinks, this one consumes a single
   406	/// inbound wire (parsing record headers and producing
   407	/// [`PreparedPayload::FileStream`] / [`PreparedPayload::TarShard`] /
   408	/// [`PreparedPayload::FileBlock`] events) and feeds them to a single sink
   409	/// sequentially. Multi-stream parallelism comes from spawning N invocations,
   410	/// one per inbound TCP connection.
   411	///
   412	/// Both directions converge on `TransferSink::write_payload`: file data
   413	/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
   414	/// which uses the same `receive_stream_double_buffered` helper as the
   415	/// daemon's push receiver and the client's pull receiver — one path,
   416	/// one optimization surface.
   417	pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
   418	    socket: &mut R,
   419	    sink: Arc<dyn TransferSink>,
   420	    progress: Option<&RemoteTransferProgress>,
   421	) -> Result<SinkOutcome> {
   422	    let mut total = SinkOutcome::default();
   423	
   424	    loop {
   425	        let mut tag = [0u8; 1];
   426	        socket
   427	            .read_exact(&mut tag)
   428	            .await
   429	            .context("reading data-plane record tag")?;
   430	
   431	        match tag[0] {
   432	            DATA_PLANE_RECORD_END => break,
   433	            DATA_PLANE_RECORD_FILE => {
   434	                let mut header = read_file_header(socket).await?;
   435	                let file_size = read_u64(socket).await?;
   436	                let mtime = read_i64(socket).await?;
   437	                let perms = read_u32(socket).await?;
   438	                header.size = file_size;
   439	                header.mtime_seconds = mtime;
   440	                header.permissions = perms;
   441	                // Use AsyncReadExt::take to give the sink exactly
   442	                // file_size bytes of the wire. tokio's Take is the
   443	                // canonical way to limit a borrowed AsyncRead.
   444	                use tokio::io::AsyncReadExt;
   445	                let mut reader = (&mut *socket).take(file_size);
   446	                let outcome = sink
   447	                    .write_file_stream(&header, &mut reader)
   448	                    .await
   449	                    .with_context(|| format!("receiving {}", header.relative_path))?;
   450	                if let Some(p) = progress {
   451	                    p.report_payload(0, outcome.bytes_written);
   452	                    p.report_file_complete(header.relative_path.clone());
   453	                }
   454	                total.merge(&outcome);
   455	            }
   456	            DATA_PLANE_RECORD_TAR_SHARD => {
   457	                let (headers, data) = read_tar_shard(socket).await?;
   458	                let bytes = data.len() as u64;
   459	                // Capture member paths for the per-file lane before the
   460	                // payload takes ownership; skip the allocation when no
   461	                // one is listening (the daemon receive path).
   462	                let member_paths: Option<Vec<String>> =
   463	                    progress.map(|_| headers.iter().map(|h| h.relative_path.clone()).collect());
   464	                let payload = PreparedPayload::TarShard { headers, data };
   465	                let outcome = sink
   466	                    .write_payload(payload)
   467	                    .await
   468	                    .context("writing payload")?;
   469	                if let Some(p) = progress {
   470	                    p.report_payload(0, bytes);
   471	                    for path in member_paths.unwrap_or_default() {
   472	                        p.report_file_complete(path);
   473	                    }
   474	                }
   475	                total.merge(&outcome);
   476	            }
   477	            DATA_PLANE_RECORD_BLOCK => {
   478	                let path = read_string(socket).await?;
   479	                let offset = read_u64(socket).await?;
   480	                let len = read_u32(socket).await? as usize;
   481	                if len > MAX_WIRE_BLOCK_BYTES {
   482	                    bail!(
   483	                        "wire block payload {} bytes exceeds max {} (rejecting to avoid OOM)",
   484	                        len,
   485	                        MAX_WIRE_BLOCK_BYTES
   486	                    );
   487	                }
   488	                let mut bytes = vec![0u8; len];
   489	                socket
   490	                    .read_exact(&mut bytes)
   491	                    .await
   492	                    .context("reading block bytes")?;
   493	                let payload = PreparedPayload::FileBlock {
   494	                    relative_path: path,
   495	                    offset,
   496	                    bytes,
   497	                };
   498	                let outcome = sink
   499	                    .write_payload(payload)
   500	                    .await
   501	                    .context("writing payload")?;
   502	                if let Some(p) = progress {
   503	                    p.report_payload(0, outcome.bytes_written);
   504	                }
   505	                total.merge(&outcome);
   506	            }
   507	            DATA_PLANE_RECORD_BLOCK_COMPLETE => {
   508	                let path = read_string(socket).await?;
   509	                let total_size = read_u64(socket).await?;
   510	                let mtime = read_i64(socket).await?;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '110,250p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   110	pub async fn execute_sink_pipeline_elastic(
   111	    source: Arc<dyn TransferSource>,
   112	    sinks: Vec<Arc<dyn TransferSink>>,
   113	    mut payload_rx: mpsc::Receiver<TransferPayload>,
   114	    prefetch: usize,
   115	    progress: Option<&RemoteTransferProgress>,
   116	    control_rx: Option<mpsc::UnboundedReceiver<SinkControl>>,
   117	) -> Result<SinkOutcome> {
   118	    use std::sync::atomic::{AtomicBool, Ordering};
   119	
   120	    if sinks.is_empty() {
   121	        // Drain incoming channel so the producer isn't left dangling.
   122	        while payload_rx.recv().await.is_some() {}
   123	        return Ok(SinkOutcome::default());
   124	    }
   125	
   126	    let sink_count = sinks.len();
   127	    let capacity = prefetch.max(1) * sink_count;
   128	    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));
   129	
   130	    // Single shared work queue. Each worker owns exactly one sink but
   131	    // pulls payloads from the common queue, so work is stolen by
   132	    // whichever sink is free rather than pre-assigned round-robin.
   133	    let (work_tx, work_rx) = flume::bounded::<TransferPayload>(capacity);
   134	
   135	    // Cancellation flag set by the first worker that errors. Without it,
   136	    // one sink failing only drops that worker's `work_rx` clone; as long
   137	    // as any other worker is alive `send_async` keeps succeeding, so the
   138	    // forwarder would keep draining `payload_rx` and queueing payloads
   139	    // that can never complete — delaying first-error-wins propagation
   140	    // (Codex review, PR2). With it, the forwarder stops at the next
   141	    // payload boundary and closes the queue so the survivors drain and
   142	    // finish promptly.
   143	    let cancelled = Arc::new(AtomicBool::new(false));
   144	
   145	    // Dynamic worker membership (`ue-r2-2`): a JoinSet instead of a
   146	    // fixed Vec of handles, plus a per-worker retire flag so a REMOVE
   147	    // can drain exactly one worker. `retire_flags` holds the workers
   148	    // that are live and not yet asked to retire — its length is the
   149	    // count the retire floor checks.
   150	    let mut join_set: tokio::task::JoinSet<(usize, Result<()>)> = tokio::task::JoinSet::new();
   151	    let mut retire_flags: Vec<(usize, tokio::sync::watch::Sender<bool>)> = Vec::new();
   152	    let mut next_slot = 0usize;
   153	
   154	    #[allow(clippy::too_many_arguments)]
   155	    fn spawn_sink_worker(
   156	        join_set: &mut tokio::task::JoinSet<(usize, Result<()>)>,
   157	        slot: usize,
   158	        sink: Arc<dyn TransferSink>,
   159	        work_rx: flume::Receiver<TransferPayload>,
   160	        source: Arc<dyn TransferSource>,
   161	        progress: Option<RemoteTransferProgress>,
   162	        total: Arc<std::sync::Mutex<SinkOutcome>>,
   163	        cancelled: Arc<std::sync::atomic::AtomicBool>,
   164	        mut retire: tokio::sync::watch::Receiver<bool>,
   165	    ) {
   166	        use std::sync::atomic::Ordering;
   167	        join_set.spawn(async move {
   168	            // Wrap the body so any early-return error trips the shared
   169	            // cancel flag before the `?` unwinds the task.
   170	            let run = async {
   171	                loop {
   172	                    // Stop pulling queued work once a sibling worker has
   173	                    // errored: first-error-wins should surface without the
   174	                    // survivors draining the rest of the bounded queue.
   175	                    // Interrupting an in-flight prepare/write (true prompt
   176	                    // cancellation) is the AbortOnDrop family, w4-1.
   177	                    if cancelled.load(Ordering::Relaxed) {
   178	                        break;
   179	                    }
   180	                    // ue-r2-2: a retired worker stops at the same payload
   181	                    // boundary; queued payloads stay in the shared queue
   182	                    // for the survivors (dequeue = ownership, so
   183	                    // exactly-once is preserved — flume's RecvFut only
   184	                    // takes an item when it resolves, so racing it is
   185	                    // safe). The watch (not a flag) also frees a worker
   186	                    // parked on an IDLE queue. Its `finish()` below emits
   187	                    // the per-stream END record — the receiver-side
   188	                    // teardown signal.
   189	                    let payload = tokio::select! {
   190	                        biased;
   191	                        _ = retire.changed() => break,
   192	                        recv = work_rx.recv_async() => match recv {
   193	                            Ok(p) => p,
   194	                            Err(_) => break, // queue closed and drained
   195	                        },
   196	                    };
   197	                    let prepared = source
   198	                        .prepare_payload(payload)
   199	                        .await
   200	                        .context("preparing payload")?;
   201	                    let files: Vec<(String, u64)> = match &prepared {
   202	                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
   203	                        PreparedPayload::TarShard { headers, .. } => headers
   204	                            .iter()
   205	                            .map(|h| (h.relative_path.clone(), h.size))
   206	                            .collect(),
   207	                        // Resume-block payloads patch existing files; no
   208	                        // file-completion event from one-block-at-a-time.
   209	                        PreparedPayload::FileBlock { .. }
   210	                        | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
   211	                    };
   212	                    let outcome = sink
   213	                        .write_payload(prepared)
   214	                        .await
   215	                        .context("writing payload")?;
   216	                    if let Some(p) = &progress {
   217	                        // Contract (progress.rs): bytes ride Payload, one
   218	                        // FileComplete per file. `size` is the planned
   219	                        // manifest size — the value this lane has always
   220	                        // reported, now on the right variant.
   221	                        for (name, size) in &files {
   222	                            p.report_payload(0, *size);
   223	                            p.report_file_complete(name.clone());
   224	                        }
   225	                    }
   226	                    let mut t = total.lock().unwrap();
   227	                    t.merge(&outcome);
   228	                }
   229	                sink.finish().await?;
   230	                Ok::<(), eyre::Report>(())
   231	            }
   232	            .await;
   233	            if run.is_err() {
   234	                // Signal the forwarder (and implicitly the other workers,
   235	                // once the queue closes) to stop feeding new work.
   236	                cancelled.store(true, Ordering::Relaxed);
   237	            }
   238	            (slot, run)
   239	        });
   240	    }
   241	
   242	    for sink in sinks {
   243	        let (retire_tx, retire_rx) = tokio::sync::watch::channel(false);
   244	        let slot = next_slot;
   245	        next_slot += 1;
   246	        retire_flags.push((slot, retire_tx));
   247	        spawn_sink_worker(
   248	            &mut join_set,
   249	            slot,
   250	            sink,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/sink.rs | sed -n '740,855p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   740	
   741	// ---------------------------------------------------------------------------
   742	// DataPlaneSink — TCP data plane writer
   743	// ---------------------------------------------------------------------------
   744	
   745	/// Writes payloads to a remote daemon via the TCP data plane binary protocol.
   746	///
   747	/// Each instance wraps a single TCP stream (DataPlaneSession). For multi-stream
   748	/// transfers, the pipeline executor creates multiple DataPlaneSink instances.
   749	pub struct DataPlaneSink<P: Probe = NoProbe> {
   750	    session: tokio::sync::Mutex<DataPlaneSession<P>>,
   751	    source: Arc<dyn TransferSource>,
   752	    dst_root: PathBuf,
   753	}
   754	
   755	impl<P: Probe> DataPlaneSink<P> {
   756	    pub fn new(
   757	        session: DataPlaneSession<P>,
   758	        source: Arc<dyn TransferSource>,
   759	        dst_root: PathBuf,
   760	    ) -> Self {
   761	        Self {
   762	            session: tokio::sync::Mutex::new(session),
   763	            source,
   764	            dst_root,
   765	        }
   766	    }
   767	}
   768	
   769	#[async_trait]
   770	impl<P: Probe> TransferSink for DataPlaneSink<P> {
   771	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   772	        let mut session = self.session.lock().await;
   773	        match payload {
   774	            PreparedPayload::File(header) => {
   775	                let size = header.size;
   776	                session
   777	                    .send_file(self.source.clone(), &header)
   778	                    .await
   779	                    .with_context(|| format!("sending {}", header.relative_path))?;
   780	                Ok(SinkOutcome {
   781	                    files_written: 1,
   782	                    bytes_written: size,
   783	                })
   784	            }
   785	            PreparedPayload::TarShard { headers, data } => {
   786	                let bytes: u64 = headers.iter().map(|h| h.size).sum();
   787	                let count = headers.len();
   788	                session
   789	                    .send_prepared_tar_shard(headers, &data)
   790	                    .await
   791	                    .context("sending tar shard")?;
   792	                Ok(SinkOutcome {
   793	                    files_written: count,
   794	                    bytes_written: bytes,
   795	                })
   796	            }
   797	            // Resume payloads can't be relayed without a reverse-resume
   798	            // protocol on the next hop. Reject explicitly.
   799	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   800	                eyre::bail!("DataPlaneSink does not relay resume-block payloads")
   801	            }
   802	        }
   803	    }
   804	
   805	    /// Relay case: bytes arrive on `reader` (e.g. from a DataPlaneSource
   806	    /// during a remote→remote transfer) and forward to the next hop.
   807	    async fn write_file_stream(
   808	        &self,
   809	        header: &FileHeader,
   810	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   811	    ) -> Result<SinkOutcome> {
   812	        let size = header.size;
   813	        let mut session = self.session.lock().await;
   814	        session
   815	            .send_file_from_reader(header, reader)
   816	            .await
   817	            .with_context(|| format!("relaying {}", header.relative_path))?;
   818	        Ok(SinkOutcome {
   819	            files_written: 1,
   820	            bytes_written: size,
   821	        })
   822	    }
   823	
   824	    async fn finish(&self) -> Result<()> {
   825	        let mut session = self.session.lock().await;
   826	        session.finish().await
   827	    }
   828	
   829	    fn root(&self) -> &Path {
   830	        &self.dst_root
   831	    }
   832	}
   833	
   834	// ---------------------------------------------------------------------------
   835	// NullSink — discard data, count bytes (for benchmarking)
   836	// ---------------------------------------------------------------------------
   837	
   838	/// Discards all payload data, counting files and bytes.
   839	///
   840	/// Useful for benchmarking source + network throughput without destination
   841	/// I/O as a bottleneck. The pipeline still prepares payloads (reading source
   842	/// files, building tar shards) so this measures everything except the write.
   843	pub struct NullSink {
   844	    label: PathBuf,
   845	}
   846	
   847	impl Default for NullSink {
   848	    fn default() -> Self {
   849	        Self {
   850	            label: PathBuf::from("/dev/null"),
   851	        }
   852	    }
   853	}
   854	
   855	impl NullSink {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/abort_on_drop.rs | sed -n '1,90p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! RAII wrapper that aborts the underlying tokio task when dropped
     2	//! without awaiting (R32-F2; hoisted to a shared location under
     3	//! `w4-1` so every spawn family — not just `pull.rs` — can use it).
     4	//!
     5	//! `JoinHandle::drop` detaches; it does NOT cancel the spawned task.
     6	//! That's a real bug wherever a spawned task's lifetime is meant to
     7	//! be bounded by a calling future: when the outer future is dropped
     8	//! (e.g. CLI Ctrl-C cancels the gRPC stream from the daemon's
     9	//! `delegated_pull` handler, or an early `?` return exits a handler
    10	//! while a data-plane task is still running), a bare `JoinHandle`
    11	//! left running would otherwise keep reading sockets and writing
    12	//! files with no owner.
    13	//!
    14	//! Usage: wrap every `tokio::spawn` whose lifetime should be bounded
    15	//! by the calling future. Await with `.join().await` — that holds
    16	//! `self` across the await so a parent-future cancellation during
    17	//! the await still triggers `abort()` via Drop. Do NOT add an
    18	//! `into_inner()` accessor: returning the bare `JoinHandle` and then
    19	//! awaiting it re-introduces the cancellation gap (R34-F2 — the bare
    20	//! handle is dropped on parent-future cancel and detaches the task
    21	//! instead of aborting it).
    22	
    23	use tokio::task::JoinHandle;
    24	
    25	pub struct AbortOnDrop<T>(Option<JoinHandle<T>>);
    26	
    27	impl<T> AbortOnDrop<T> {
    28	    pub fn new(handle: JoinHandle<T>) -> Self {
    29	        Self(Some(handle))
    30	    }
    31	
    32	    /// Await the spawned task while keeping `self` alive across the
    33	    /// await. If the surrounding future is cancelled during the
    34	    /// await, `self` is dropped and our `Drop` impl fires `abort()`.
    35	    /// Compare to a hypothetical `into_inner().await` pattern, which
    36	    /// would release the guard before awaiting — that's the
    37	    /// cancellation-gap bug R34-F2 fixed.
    38	    pub async fn join(mut self) -> std::result::Result<T, tokio::task::JoinError> {
    39	        // Borrow the JoinHandle out of the Option, but DON'T move it
    40	        // out of `self`. `self` lives across this await; if the
    41	        // surrounding future is cancelled here, `self` drops and
    42	        // `Drop::drop` aborts the still-owned handle.
    43	        let handle = self
    44	            .0
    45	            .as_mut()
    46	            .expect("AbortOnDrop already consumed (programming error)");
    47	        let result = handle.await;
    48	        // Task completed (success or panic). Clear the slot so the
    49	        // trailing Drop after this returns is a no-op rather than
    50	        // calling abort() on an already-finished handle.
    51	        self.0 = None;
    52	        result
    53	    }
    54	}
    55	
    56	impl<T> Drop for AbortOnDrop<T> {
    57	    fn drop(&mut self) {
    58	        if let Some(handle) = self.0.take() {
    59	            handle.abort();
    60	        }
    61	    }
    62	}
    63	
    64	#[cfg(test)]
    65	mod tests {
    66	    //! Regression tests for the `AbortOnDrop` wrapper that bounds
    67	    //! spawned tasks across the workspace (R32-F2, hoisted under
    68	    //! `w4-1`). Without this, dropping the `JoinHandle` would detach
    69	    //! the spawned task — meaning a cancelled parent future (CLI
    70	    //! Ctrl-C, an early `?` return, etc.) couldn't actually stop a
    71	    //! running background task.
    72	
    73	    use super::AbortOnDrop;
    74	    use std::sync::atomic::{AtomicBool, Ordering};
    75	    use std::sync::Arc;
    76	    use std::time::Duration;
    77	
    78	    // Paused virtual time (w4-1 codex review): the relocated test
    79	    // waited only 150ms real time against the task's 500ms natural
    80	    // completion, so it passed whether or not Drop aborted — vacuous
    81	    // since its pull.rs days. Under start_paused the auto-advancing
    82	    // clock deterministically runs a detached task's 500ms sleep
    83	    // BEFORE the test's 700ms wake, so a Drop impl that detaches
    84	    // instead of aborting now fails the assertion, with no wall-clock
    85	    // sensitivity.
    86	    #[tokio::test(start_paused = true)]
    87	    async fn drop_without_consume_aborts_running_task() {
    88	        // The task tries to set the "completed" flag after a delay;
    89	        // wrapping in AbortOnDrop and dropping immediately must
    90	        // prevent the flag from ever being set.

exec
/usr/bin/zsh -lc "git grep -n \"fn send_file\\|send_file_from_reader\\|DATA_PLANE_RECORD_FILE\" 3ae0a5f -- crates/blit-core/src/remote/transfer/data_plane.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
3ae0a5f:crates/blit-core/src/remote/transfer/data_plane.rs:16:pub const DATA_PLANE_RECORD_FILE: u8 = 0;
3ae0a5f:crates/blit-core/src/remote/transfer/data_plane.rs:251:    pub async fn send_file(
3ae0a5f:crates/blit-core/src/remote/transfer/data_plane.rs:261:        self.send_file_from_reader(header, &mut file).await
3ae0a5f:crates/blit-core/src/remote/transfer/data_plane.rs:270:    pub async fn send_file_from_reader(
3ae0a5f:crates/blit-core/src/remote/transfer/data_plane.rs:284:            .write_all(&[DATA_PLANE_RECORD_FILE])
3ae0a5f:crates/blit-core/src/remote/transfer/data_plane.rs:326:    async fn send_file_double_buffered(

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use eyre::{bail, Context, Result};
     2	use futures::StreamExt;
     3	use tokio::io::{AsyncReadExt, AsyncWriteExt};
     4	use tokio::net::TcpStream;
     5	
     6	use crate::buffer::BufferPool;
     7	use crate::generated::FileHeader;
     8	
     9	use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
    10	use super::progress::{NoProbe, Probe};
    11	use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
    12	use crate::remote::transfer::source::TransferSource;
    13	use std::sync::Arc;
    14	
    15	pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1024 * 1024;
    16	pub const DATA_PLANE_RECORD_FILE: u8 = 0;
    17	pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
    18	pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
    19	pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
    20	pub const DATA_PLANE_RECORD_END: u8 = 0xFF;
    21	
    22	/// ue-r2-2: length of the per-epoch resize credential a data socket
    23	/// echoes after the one-time token when resize was negotiated
    24	/// (`DataTransferNegotiation.epoch0_sub_token` for the initial
    25	/// sockets, `DataPlaneResize.sub_token` for an ADD epoch's socket).
    26	pub const SUB_TOKEN_LEN: usize = 16;
    27	
    28	/// Generate one 16-byte resize sub-token. Same fallible-RNG posture
    29	/// as the daemon's one-time token (audit-3b): a missing system RNG is
    30	/// an error, never a weaker credential.
    31	pub fn generate_sub_token() -> eyre::Result<Vec<u8>> {
    32	    use rand::{rngs::SysRng, TryRng};
    33	    let mut buf = vec![0u8; SUB_TOKEN_LEN];
    34	    SysRng
    35	        .try_fill_bytes(&mut buf)
    36	        .map_err(|err| eyre::eyre!("system RNG unavailable: {err}"))?;
    37	    Ok(buf)
    38	}
    39	
    40	/// A single data-plane TCP stream and its send loop.
    41	///
    42	/// Generic over a [`Probe`] so the byte-copy hot path can carry
    43	/// per-stream telemetry under adaptive mode at **zero cost** when the
    44	/// probe is [`NoProbe`] (the default): the instrumented branches are
    45	/// gated on `P::ACTIVE`, a compile-time constant, so they fold away
    46	/// entirely for `DataPlaneSession<NoProbe>`. Existing callers name the
    47	/// bare type and get the `NoProbe` default; the adaptive controller
    48	/// constructs `DataPlaneSession<LiveProbe>` via
    49	/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
    50	///
    51	/// audit-h3b: writes go through [`StallGuardWriter`] so a stalled
    52	/// reader (TCP backpressure from a slow / wedged peer) trips after
    53	/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
    54	/// of pinning the worker for OS-level TCP retransmit exhaustion
    55	/// (15+ minutes). All existing `self.stream.write_all/.flush` call
    56	/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
    57	/// so no per-site change was needed.
    58	pub struct DataPlaneSession<P: Probe = NoProbe> {
    59	    stream: StallGuardWriter<TcpStream>,
    60	    pool: Arc<BufferPool>,
    61	    trace: bool,
    62	    chunk_bytes: usize,
    63	    payload_prefetch: usize,
    64	    bytes_sent: u64,
    65	    probe: P,
    66	}
    67	
    68	macro_rules! trace_client {
    69	    ($session:expr, $($arg:tt)*) => {
    70	        if $session.trace {
    71	            eprintln!("[data-plane-client] {}", format_args!($($arg)*));
    72	        }
    73	    };
    74	}
    75	
    76	impl DataPlaneSession<NoProbe> {
    77	    /// Create a session from an existing stream with buffer pooling.
    78	    ///
    79	    /// Produces the un-instrumented `NoProbe` variant — the default for
    80	    /// every non-adaptive caller. audit-h3b: the stream is wrapped in
    81	    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
    82	    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
    83	    /// observable write progress instead of pinning the worker for
    84	    /// OS-level TCP retransmit exhaustion. The production call sites
    85	    /// (`daemon/service/pull.rs`, `daemon/service/pull_sync.rs`, and the
    86	    /// resume path) inherit the guard without code changes.
    87	    pub async fn from_stream(
    88	        stream: TcpStream,
    89	        trace: bool,
    90	        chunk_bytes: usize,
    91	        payload_prefetch: usize,
    92	        pool: Arc<BufferPool>,
    93	    ) -> Self {
    94	        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
    95	            .await
    96	    }
    97	
    98	    /// Connect to a data plane endpoint with buffer pooling.
    99	    #[allow(clippy::too_many_arguments)]
   100	    pub async fn connect(
   101	        host: &str,
   102	        port: u32,
   103	        token: &[u8],
   104	        chunk_bytes: usize,
   105	        payload_prefetch: usize,
   106	        trace: bool,
   107	        tcp_buffer_size: Option<usize>,
   108	        pool: Arc<BufferPool>,
   109	    ) -> Result<Self> {
   110	        Self::connect_with_probe(
   111	            host,
   112	            port,
   113	            token,
   114	            chunk_bytes,
   115	            payload_prefetch,
   116	            trace,
   117	            tcp_buffer_size,
   118	            pool,
   119	            NoProbe,
   120	        )
   121	        .await
   122	    }
   123	}
   124	
   125	impl<P: Probe> DataPlaneSession<P> {
   126	    /// `connect` with an explicit probe (ue-r2-1e: the dial tuner
   127	    /// attaches `LiveProbe` telemetry to the push data plane; the
   128	    /// probe-free path monomorphizes to `NoProbe` and reads no clock).
   129	    #[allow(clippy::too_many_arguments)]
   130	    pub async fn connect_with_probe(
   131	        host: &str,
   132	        port: u32,
   133	        token: &[u8],
   134	        chunk_bytes: usize,
   135	        payload_prefetch: usize,
   136	        trace: bool,
   137	        tcp_buffer_size: Option<usize>,
   138	        pool: Arc<BufferPool>,
   139	        probe: P,
   140	    ) -> Result<Self> {
   141	        let addr = format!("{}:{}", host, port);
   142	        if trace {
   143	            eprintln!("[data-plane-client] connecting to {}", addr);
   144	        }
   145	        // design-3: bounded dial (connect + w1-2 socket policy +
   146	        // negotiation-token write) via the shared data-plane helper —
   147	        // one owner for every client-side data-plane dial, both
   148	        // directions.
   149	        let stream = super::socket::dial_data_plane(&addr, token, tcp_buffer_size)
   150	            .await
   151	            .context("dialing push data plane")?;
   152	
   153	        Ok(
   154	            Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, probe)
   155	                .await,
   156	        )
   157	    }
   158	}
   159	
   160	impl<P: Probe> DataPlaneSession<P> {
   161	    /// Create a session carrying an arbitrary [`Probe`]. The generic
   162	    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
   163	    /// the adaptive controller calls this with a `LiveProbe` to enable
   164	    /// per-stream telemetry.
   165	    pub async fn from_stream_with_probe(
   166	        stream: TcpStream,
   167	        trace: bool,
   168	        chunk_bytes: usize,
   169	        payload_prefetch: usize,
   170	        pool: Arc<BufferPool>,
   171	        probe: P,
   172	    ) -> Self {
   173	        let payload_prefetch = payload_prefetch.max(1);
   174	        let chunk_bytes = chunk_bytes.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);
   175	        Self {
   176	            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
   177	            pool,
   178	            trace,
   179	            chunk_bytes,
   180	            payload_prefetch,
   181	            bytes_sent: 0,
   182	            probe,
   183	        }
   184	    }
   185	
   186	    pub async fn send_payloads(
   187	        &mut self,
   188	        source: Arc<dyn TransferSource>,
   189	        payloads: Vec<TransferPayload>,
   190	    ) -> Result<()> {
   191	        self.send_payloads_with_progress(source, payloads, None)
   192	            .await
   193	    }
   194	
   195	    pub async fn send_payloads_with_progress(
   196	        &mut self,
   197	        source: Arc<dyn TransferSource>,
   198	        payloads: Vec<TransferPayload>,
   199	        progress: Option<&super::progress::RemoteTransferProgress>,
   200	    ) -> Result<()> {
   201	        let mut stream = prepared_payload_stream(payloads, source.clone(), self.payload_prefetch);
   202	        while let Some(prepared) = stream.next().await {
   203	            match prepared? {
   204	                PreparedPayload::File(header) => {
   205	                    if let Err(err) = self.send_file(source.clone(), &header).await {
   206	                        return Err(err.wrap_err(format!("sending {}", header.relative_path)));
   207	                    }
   208	                    self.bytes_sent = self.bytes_sent.saturating_add(header.size);
   209	                    if let Some(progress) = progress {
   210	                        progress.report_payload(0, header.size);
   211	                        progress.report_file_complete(header.relative_path.clone());
   212	                    }
   213	                }
   214	                PreparedPayload::TarShard { headers, data } => {
   215	                    let shard_bytes: u64 = headers.iter().map(|h| h.size).sum();
   216	                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
   217	                        return Err(err.wrap_err("sending tar shard"));
   218	                    }
   219	                    self.bytes_sent = self.bytes_sent.saturating_add(shard_bytes);
   220	                    if let Some(progress) = progress {
   221	                        for header in &headers {
   222	                            progress.report_payload(0, header.size);
   223	                            progress.report_file_complete(header.relative_path.clone());
   224	                        }
   225	                    }
   226	                }
   227	                PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   228	                    bail!("DataPlaneSession::send_payloads does not handle resume payloads");
   229	                }
   230	            }
   231	        }
   232	
   233	        Ok(())
   234	    }
   235	
   236	    pub async fn finish(&mut self) -> Result<()> {
   237	        self.stream
   238	            .write_all(&[DATA_PLANE_RECORD_END])
   239	            .await
   240	            .context("writing transfer terminator")?;
   241	        self.stream
   242	            .flush()
   243	            .await
   244	            .context("flushing data plane stream")
   245	    }
   246	
   247	    pub fn bytes_sent(&self) -> u64 {
   248	        self.bytes_sent
   249	    }
   250	
   251	    pub async fn send_file(
   252	        &mut self,
   253	        source: Arc<dyn TransferSource>,
   254	        header: &FileHeader,
   255	    ) -> Result<()> {
   256	        let rel = &header.relative_path;
   257	        let mut file = source
   258	            .open_file(header)
   259	            .await
   260	            .with_context(|| format!("opening {}", rel))?;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '250,380p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   250	
   251	    pub async fn send_file(
   252	        &mut self,
   253	        source: Arc<dyn TransferSource>,
   254	        header: &FileHeader,
   255	    ) -> Result<()> {
   256	        let rel = &header.relative_path;
   257	        let mut file = source
   258	            .open_file(header)
   259	            .await
   260	            .with_context(|| format!("opening {}", rel))?;
   261	        self.send_file_from_reader(header, &mut file).await
   262	    }
   263	
   264	    /// Send a file payload whose bytes come from an arbitrary async
   265	    /// reader (not a local file). Used by `DataPlaneSink` for the
   266	    /// remote→remote relay case, where bytes arrive from an inbound
   267	    /// `DataPlaneSource` and need to be forwarded to the next hop.
   268	    ///
   269	    /// Same wire format and double-buffered loop as `send_file`.
   270	    pub async fn send_file_from_reader(
   271	        &mut self,
   272	        header: &FileHeader,
   273	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   274	    ) -> Result<()> {
   275	        let rel = &header.relative_path;
   276	        trace_client!(self, "sending file '{}' ({} bytes)", rel, header.size);
   277	
   278	        let path_bytes = rel.as_bytes();
   279	        if path_bytes.len() > u32::MAX as usize {
   280	            bail!("relative path too long for transfer: {}", rel);
   281	        }
   282	
   283	        self.stream
   284	            .write_all(&[DATA_PLANE_RECORD_FILE])
   285	            .await
   286	            .context("writing data-plane record tag")?;
   287	        self.stream
   288	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   289	            .await
   290	            .context("writing path length")?;
   291	        self.stream
   292	            .write_all(path_bytes)
   293	            .await
   294	            .context("writing path bytes")?;
   295	
   296	        self.stream
   297	            .write_all(&header.size.to_be_bytes())
   298	            .await
   299	            .context("writing file size")?;
   300	        // Wire-format extension (2026-05-01): include mtime + permissions
   301	        // inline so push and pull data plane records carry the same
   302	        // information. Lets the receive pipeline apply metadata via
   303	        // FsTransferSink without consulting an out-of-band manifest cache.
   304	        self.stream
   305	            .write_all(&header.mtime_seconds.to_be_bytes())
   306	            .await
   307	            .context("writing mtime")?;
   308	        self.stream
   309	            .write_all(&header.permissions.to_be_bytes())
   310	            .await
   311	            .context("writing permissions")?;
   312	
   313	        // Double-buffered I/O: overlaps source reads with network writes
   314	        self.send_file_double_buffered(reader, header, rel).await?;
   315	
   316	        trace_client!(self, "file '{}' sent ({} bytes)", rel, header.size);
   317	
   318	        Ok(())
   319	    }
   320	
   321	    /// Double-buffered file sending: overlaps disk reads with network writes.
   322	    /// Uses two buffers from the pool to enable concurrent I/O operations.
   323	    ///
   324	    /// Pattern: While buffer A is being written to network, buffer B is filled from disk.
   325	    /// This hides disk latency behind network latency for improved throughput.
   326	    async fn send_file_double_buffered(
   327	        &mut self,
   328	        file: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   329	        header: &FileHeader,
   330	        rel: &str,
   331	    ) -> Result<()> {
   332	        let mut remaining = header.size;
   333	        if remaining == 0 {
   334	            return Ok(());
   335	        }
   336	
   337	        // Acquire two buffers for double-buffering
   338	        let mut buf_a = self.pool.acquire().await;
   339	        let mut buf_b = self.pool.acquire().await;
   340	
   341	        // Initial read into buf_a
   342	        let mut bytes_a = file
   343	            .read(buf_a.as_mut_slice())
   344	            .await
   345	            .with_context(|| format!("reading {}", rel))?;
   346	
   347	        if bytes_a == 0 {
   348	            bail!(
   349	                "unexpected EOF while reading {} ({} bytes remaining)",
   350	                rel,
   351	                remaining
   352	            );
   353	        }
   354	        // Clamp to the declared size before subtracting. A source that
   355	        // returns more bytes than `header.size` — a file that grew after
   356	        // the manifest was computed, or a lying `TransferSource` — would
   357	        // otherwise underflow `remaining` (debug: panic; release: wrap to
   358	        // u64::MAX → runaway loop) and push undeclared bytes onto the
   359	        // framed stream. We send exactly `header.size` and ignore excess.
   360	        bytes_a = (bytes_a as u64).min(remaining) as usize;
   361	        remaining -= bytes_a as u64;
   362	
   363	        // Main loop: write buf_a while reading into buf_b
   364	        while remaining > 0 {
   365	            // Per-stream telemetry: time ONLY the socket write as the
   366	            // backpressure signal. ue-r2-1e (carried ue-r2-1a review
   367	            // finding): the old code timed the whole overlapped
   368	            // write+read join, so a slow disk READ inflated
   369	            // "write blocked" and would bias the dial tuner
   370	            // conservative. The async block's clock starts when the
   371	            // join first polls it and stops when write_all completes —
   372	            // the concurrent read neither extends nor shortens it.
   373	            // Gated on the compile-time `P::ACTIVE` constant so
   374	            // `DataPlaneSession<NoProbe>` reads no clock.
   375	            let write_slice = &buf_a.as_slice()[..bytes_a];
   376	            let stream = &mut self.stream;
   377	            let (write_outcome, read_result) = tokio::join!(
   378	                async {
   379	                    let started = if P::ACTIVE {
   380	                        Some(std::time::Instant::now())

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '375,455p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   375	            let write_slice = &buf_a.as_slice()[..bytes_a];
   376	            let stream = &mut self.stream;
   377	            let (write_outcome, read_result) = tokio::join!(
   378	                async {
   379	                    let started = if P::ACTIVE {
   380	                        Some(std::time::Instant::now())
   381	                    } else {
   382	                        None
   383	                    };
   384	                    let result = stream.write_all(write_slice).await;
   385	                    (result, started.map(|t| t.elapsed()))
   386	                },
   387	                file.read(buf_b.as_mut_slice())
   388	            );
   389	
   390	            let (write_result, write_elapsed) = write_outcome;
   391	            write_result.with_context(|| format!("sending {}", rel))?;
   392	            if let Some(elapsed) = write_elapsed {
   393	                self.probe.note_write_blocked(elapsed.as_nanos() as u64);
   394	            }
   395	            self.probe.record_bytes(bytes_a as u64);
   396	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   397	
   398	            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;
   399	
   400	            if bytes_b == 0 && remaining > 0 {
   401	                bail!(
   402	                    "unexpected EOF while reading {} ({} bytes remaining)",
   403	                    rel,
   404	                    remaining
   405	                );
   406	            }
   407	            // Same clamp as the initial read: never subtract more than
   408	            // `remaining`, so an over-returning reader can neither
   409	            // underflow the counter nor send undeclared bytes.
   410	            let bytes_b = (bytes_b as u64).min(remaining) as usize;
   411	            remaining -= bytes_b as u64;
   412	
   413	            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
   414	            std::mem::swap(&mut buf_a, &mut buf_b);
   415	            bytes_a = bytes_b;
   416	        }
   417	
   418	        // Final write: send the last chunk in buf_a. This is a pure
   419	        // write (no overlapped read), so the timing is cleanly
   420	        // attributable to socket-write backpressure.
   421	        if bytes_a > 0 {
   422	            let tail_start = if P::ACTIVE {
   423	                Some(std::time::Instant::now())
   424	            } else {
   425	                None
   426	            };
   427	            self.stream
   428	                .write_all(&buf_a.as_slice()[..bytes_a])
   429	                .await
   430	                .with_context(|| format!("sending {}", rel))?;
   431	            if P::ACTIVE {
   432	                if let Some(t) = tail_start {
   433	                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   434	                }
   435	            }
   436	            self.probe.record_bytes(bytes_a as u64);
   437	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   438	        }
   439	
   440	        // Buffers return to pool automatically on drop
   441	        Ok(())
   442	    }
   443	
   444	    pub async fn send_prepared_tar_shard(
   445	        &mut self,
   446	        headers: Vec<FileHeader>,
   447	        data: &[u8],
   448	    ) -> Result<()> {
   449	        let shard_len = headers.len();
   450	        let preview = headers
   451	            .first()
   452	            .map(|h| h.relative_path.as_str())
   453	            .unwrap_or("<empty>");
   454	        trace_client!(
   455	            self,

codex
- `crates/blit-core/src/transfer_session/mod.rs:888` — **High** — `dp.queue(payloads).await?` is still not raced against `SourceEvent::Fault`. A cancel while earlier queued payloads are actively moving and this send half is blocked on pipeline backpressure can still surface the data-plane/pipeline error instead of the peer-framed `CANCELLED`, which violates the stated mid-transfer cancel contract.

- `crates/blit-daemon/src/service/transfer_session_e2e.rs:253` — **Medium** — the e2e’s “bytes flowed” gate fires after writing 64 KiB into a local `tokio::io::duplex` buffer, not after the data-plane socket writes or the daemon receives payload bytes. Because the duplex has 256 KiB capacity, the notify can happen before any file body bytes cross TCP, so the test does not deterministically prove the requested “payload bytes flow over TCP data plane” condition.

- `crates/blit-core/src/transfer_session/mod.rs:1176` — **Medium** — `recv_peer_fault` silently discards non-fault events during the pre-`SourceDone` drain. Those events are not legitimate in that phase, but they are protocol violations that previously would be surfaced later; now an early `Summary`/duplicate `NeedComplete`/late `Need` can be dropped, potentially converting a fail-fast protocol error into a delayed data-plane error or a hang if `dp.finish()` is also stuck.

VERDICT: **NEEDS FIXES**. Diff-level test accounting adds exactly two tests and removes none, matching 1513 → 1515.
tokens used
196,385
- `crates/blit-core/src/transfer_session/mod.rs:888` — **High** — `dp.queue(payloads).await?` is still not raced against `SourceEvent::Fault`. A cancel while earlier queued payloads are actively moving and this send half is blocked on pipeline backpressure can still surface the data-plane/pipeline error instead of the peer-framed `CANCELLED`, which violates the stated mid-transfer cancel contract.

- `crates/blit-daemon/src/service/transfer_session_e2e.rs:253` — **Medium** — the e2e’s “bytes flowed” gate fires after writing 64 KiB into a local `tokio::io::duplex` buffer, not after the data-plane socket writes or the daemon receives payload bytes. Because the duplex has 256 KiB capacity, the notify can happen before any file body bytes cross TCP, so the test does not deterministically prove the requested “payload bytes flow over TCP data plane” condition.

- `crates/blit-core/src/transfer_session/mod.rs:1176` — **Medium** — `recv_peer_fault` silently discards non-fault events during the pre-`SourceDone` drain. Those events are not legitimate in that phase, but they are protocol violations that previously would be surfaced later; now an early `Summary`/duplicate `NeedComplete`/late `Need` can be dropped, potentially converting a fail-fast protocol error into a delayed data-plane error or a hang if `dp.finish()` is also stuck.

VERDICT: **NEEDS FIXES**. Diff-level test accounting adds exactly two tests and removes none, matching 1513 → 1515.
