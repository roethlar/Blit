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
session id: 019f31d6-2bfa-7370-a8a4-0097bceb9088
--------
user
Review the diff of commit c70c2ac (run: git show c70c2ac). It implements sf-2 of docs/plan/SMALL_FILE_CEILING.md (Active): client-side shape-correction stream resize — the daemon proposes the push epoch-0 stream count from a partial manifest at its early flush, so many-tiny-file pushes rode 1 stream; the client now re-runs the engine shape table (initial_stream_proposal) over the accumulated need list and corrects upward via the existing ue-r2-2 resize wire, one ADD epoch at a time. Check: correctness regressions, the slice's acceptance criteria (proposal-table unit pins for the plan's cells + loopback e2e pin that a 10k-file push opens >1 data-plane connection), FAST/SIMPLE/RELIABLE, the invariants relevant to transfer code (byte-identical / StallGuard / cancellation / byte-accounting), interaction between the two resize proposers (tuner resize_tick vs propose_shape_resize — epoch allocation, one-in-flight, flap risk), mixed-version behavior (old daemon / gRPC fallback unchanged), and that the test count did not drop (1479 -> 1483). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll review this as a code review only, starting from the requested `git show c70c2ac` and then checking the plan/tests and relevant transfer code around the changed lines.
exec
/usr/bin/zsh -lc 'git show --stat --oneline --decorate --find-renames c70c2ac' in /home/michael/dev/Blit
 succeeded in 0ms:
c70c2ac (HEAD -> master) sf-2: shape-correction stream resize for many-file pushes
 .review/findings/sf-2-shape-correction-resize.md   | 105 +++++++++++
 crates/blit-core/src/engine/dial.rs                | 132 ++++++++++++-
 crates/blit-core/src/remote/push/client/mod.rs     | 205 +++++++++++++++++----
 crates/blit-core/src/remote/push/client/types.rs   |   5 +
 crates/blit-daemon/src/service/push/mod.rs         |   2 +
 .../src/service/push/shape_resize_e2e.rs           | 117 ++++++++++++
 6 files changed, 529 insertions(+), 37 deletions(-)

exec
/usr/bin/zsh -lc 'git show --find-renames --format=fuller --stat --patch c70c2ac' in /home/michael/dev/Blit
 succeeded in 0ms:
commit c70c2ac64ebbd9ffb45633e8cc3a8e6075e272c5
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sun Jul 5 06:32:09 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sun Jul 5 06:32:09 2026 -0400

    sf-2: shape-correction stream resize for many-file pushes
    
    The daemon proposes the push epoch-0 stream count at its early
    manifest flush (128 entries), so a many-tiny-file push negotiated
    from a ~128-file prefix -> 1 stream and rode it for the whole
    transfer (the measured 10 GbE small-cell gap; sf-1 probe: 1000
    files -> 1 stream where the table says 2). The client now re-runs
    the engine shape table over the accumulated need list and corrects
    upward through the existing ue-r2-2 resize wire, one ADD epoch at
    a time. Dial owns the policy (propose_shape_resize; CAS epoch
    allocation now that two proposers exist); tuner REMOVE permanently
    disables shape corrections for the transfer. New loopback e2e pins
    a 10k-file push ending above 1 stream (guard proven by revert);
    unit pins map the plan's three cells through the table. Suite
    1479 -> 1483.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
---
 .review/findings/sf-2-shape-correction-resize.md   | 105 +++++++++++
 crates/blit-core/src/engine/dial.rs                | 132 ++++++++++++-
 crates/blit-core/src/remote/push/client/mod.rs     | 205 +++++++++++++++++----
 crates/blit-core/src/remote/push/client/types.rs   |   5 +
 crates/blit-daemon/src/service/push/mod.rs         |   2 +
 .../src/service/push/shape_resize_e2e.rs           | 117 ++++++++++++
 6 files changed, 529 insertions(+), 37 deletions(-)

diff --git a/.review/findings/sf-2-shape-correction-resize.md b/.review/findings/sf-2-shape-correction-resize.md
new file mode 100644
index 0000000..081d4bc
--- /dev/null
+++ b/.review/findings/sf-2-shape-correction-resize.md
@@ -0,0 +1,105 @@
+# sf-2 — Shape-correction stream resize (dial file-count weighting, e2e)
+
+**Plan**: `docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4), slice sf-2.
+**Status**: implemented, codex review pending.
+
+## What
+
+Makes a many-tiny-file push actually run at the stream count the
+engine's shape table assigns it. The plan's diagnosis said
+`initial_stream_proposal` was byte-weighted; in reality the table has
+had file-count tiers since ue-r2-1f (`dial.rs`) — the defect is the
+**input**: on push the daemon proposes the epoch-0 stream count at its
+early manifest flush (`FILE_LIST_EARLY_FLUSH_ENTRIES` = 128,
+`control.rs`), so a 10k-file push negotiated from a ~128-file prefix →
+1 stream, and rode it for the whole transfer. That is the measured
+10 GbE small-cell gap and the sf-1 loopback probe finding (1000 files →
+1 stream where the table says 2).
+
+Fix: **client-side shape-correction resize**. As the need list
+accumulates (the true transfer shape — an incremental push may move a
+tiny subset of a large manifest), the client re-runs
+`initial_stream_proposal` over the accumulated need bytes/count and
+corrects the live stream count upward through the existing ue-r2-2
+resize wire, one ADD epoch at a time. No daemon change, no wire change.
+
+## Approach
+
+- `TransferDial::propose_shape_resize(desired)` (engine `dial.rs`, the
+  single stream-policy owner per w2-2): one-in-flight, ceiling-clamped,
+  one stream per epoch (the wire carries one `sub_token` per ADD),
+  ADD-only. Unlike `resize_tick` there is **no sustain/cooldown** — the
+  shape is a definite signal, not throughput inference. Epoch
+  allocation switched from store to CAS in both proposers: two tasks
+  (tuner + client loop) now allocate epochs, and a plain store could
+  stack two live proposals onto one epoch number.
+- Push client (`push/client/mod.rs`): correction fires at the three
+  points where shape knowledge or send capacity changes — negotiation
+  (need batches can predate it), each need-list batch (DataPlane mode),
+  and each resize-ack settle (continues the ramp). Gated on
+  resize-negotiated transfers; **flips off permanently the first time
+  the tuner proposes REMOVE** — live throughput evidence outranks the
+  static table, and re-adding what the tuner retired would flap.
+- The tuner arm's inline ADD send is extracted into `send_resize_add`,
+  shared with the shape path (identical wire behavior, one copy).
+- `RemotePushReport.data_plane_streams: Option<usize>` — the dial's
+  settled live count at finish (`None` on gRPC fallback). This is the
+  e2e pin's observable; also useful diagnostics.
+- Pull side checked and NOT touched: `negotiated_pull_streams`
+  (`pull_sync.rs:344`) proposes from the complete post-diff
+  `entries_to_send` — pull never had this defect.
+
+## Files
+
+- `crates/blit-core/src/engine/dial.rs` — `propose_shape_resize`, CAS
+  epoch allocation in `resize_tick`, unit pins.
+- `crates/blit-core/src/remote/push/client/mod.rs` — `send_resize_add`
+  + `maybe_shape_resize` helpers, three correction call sites, REMOVE
+  gate, report field.
+- `crates/blit-core/src/remote/push/client/types.rs` — report field.
+- `crates/blit-daemon/src/service/push/shape_resize_e2e.rs` (new) +
+  `push/mod.rs` — loopback e2e pin.
+
+## Tests
+
+Suite 1479 → **1483 passed / 0 failed** (37 suites; same 2 ignored) —
+count grew by 4, fmt + clippy clean.
+
+- `shape_table_covers_the_small_file_ceiling_cells` — the plan's three
+  measured cells mapped through the table (10k×4 KiB → 8 via the
+  file-count tier; 1×1 GiB → 8 via bytes, unchanged; mixed → 8 via
+  bytes) plus the sf-1 probe cell (1000 files → 2).
+- `shape_resize_ramps_one_epoch_at_a_time_toward_the_target`,
+  `shape_resize_clamps_to_the_profile_ceiling` — proposal semantics:
+  no-op at/below live, one-in-flight blocks both proposers, no
+  cooldown, refusal retries, receiver-ceiling clamp.
+- `many_tiny_file_push_opens_more_than_one_data_plane_connection`
+  (blit-daemon, in-process loopback e2e): REAL push service served via
+  `production_server_builder`, REAL `RemotePushClient` pushes 10,000
+  tiny files, asserts `!fallback_used`, all files transferred, and
+  `data_plane_streams > 1`. **Guard proven**: with
+  `propose_shape_resize` forced to `None` (temporary revert) the test
+  fails with "settled at 1" — the exact pre-fix behavior; restored and
+  re-passed. Runtime ~0.35 s.
+
+## Known gaps
+
+- The ramp is one stream per acked epoch: 1→8 takes 7 control
+  roundtrips. Negligible on LAN (the e2e settles multi-stream in
+  well under its 0.35 s); on high-RTT links the ramp is slower — WAN
+  tuning is a plan non-goal.
+- Old daemon (no `resize_enabled`) or gRPC fallback: behavior
+  unchanged by design — the correction needs the resize wire. Rig
+  cells with both ends current are what sf-4 re-measures.
+- The daemon's early-flush proposal itself still lowballs; the
+  correction is client-side. Carrying workload totals in `PushHeader`
+  would fix it at the source but is wire-visible (sf-6-class owner
+  gate) and unnecessary while the ramp closes the gap.
+- Shape corrections stop for the transfer after any tuner REMOVE; the
+  ue-2/sf-5 backlog-signal feed (mid-transfer dynamics) stays a
+  separate slice.
+- Whether 8 streams reaches the *hardware* ceiling on the small cells
+  is sf-4's rig question; this slice removes the policy binder only.
+- Windows: touched code is platform-neutral (client loop + dial), but
+  the parity run on the owner's machine per repo policy has not been
+  done this slice.
diff --git a/crates/blit-core/src/engine/dial.rs b/crates/blit-core/src/engine/dial.rs
index 421c2fc..611820c 100644
--- a/crates/blit-core/src/engine/dial.rs
+++ b/crates/blit-core/src/engine/dial.rs
@@ -313,7 +313,16 @@ impl TransferDial {
             return None;
         }
         let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
-        self.pending_epoch.store(epoch, Ordering::Relaxed);
+        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
+        // another task, and a plain store here could stack two live
+        // proposals onto one epoch number.
+        if self
+            .pending_epoch
+            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
+            .is_err()
+        {
+            return None;
+        }
         self.resize_sustain.store(0, Ordering::Relaxed);
         Some(ResizeProposal {
             epoch,
@@ -322,6 +331,42 @@ impl TransferDial {
         })
     }
 
+    /// sf-2: shape-correction proposal. On push the daemon proposes the
+    /// epoch-0 stream count from whatever manifest prefix it has seen at
+    /// the early flush (`FILE_LIST_EARLY_FLUSH_ENTRIES`), so a
+    /// many-tiny-file push can negotiate far fewer streams than
+    /// [`initial_stream_proposal`] assigns the full workload. As the
+    /// need list accumulates client-side, the client re-runs the shape
+    /// table and corrects upward through the normal resize wire.
+    ///
+    /// Unlike [`Self::resize_tick`] this is a definite signal — the
+    /// shape is known, not inferred from throughput — so there is no
+    /// sustain/cooldown discipline. It still honors one-in-flight and
+    /// the receiver-profile ceiling, still moves ONE stream per epoch
+    /// (the wire carries one `sub_token` per ADD), and never proposes
+    /// REMOVE: shrinking below a live count is throughput evidence and
+    /// stays the tuner's call.
+    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
+        let desired = desired_streams.clamp(1, self.ceiling_max_streams.max(1));
+        let live = self.live_streams.load(Ordering::Relaxed).max(1);
+        if desired <= live {
+            return None;
+        }
+        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
+        if self
+            .pending_epoch
+            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
+            .is_err()
+        {
+            return None;
+        }
+        Some(ResizeProposal {
+            epoch,
+            target_streams: live + 1,
+            add: true,
+        })
+    }
+
     /// Settle the in-flight proposal with what ACTUALLY happened:
     /// `effective_streams` is the live count now in effect (from the
     /// peer's ack, or the local count if a post-ack dial failed and
@@ -876,6 +921,91 @@ mod tests {
         }
     }
 
+    // ── sf-2 shape-correction resize ─────────────────────────────────
+
+    /// The plan's three measured 10 GbE cells mapped through the shape
+    /// table (`docs/plan/SMALL_FILE_CEILING.md`): the small and mixed
+    /// cells must NOT ride the byte tiers alone.
+    #[test]
+    fn shape_table_covers_the_small_file_ceiling_cells() {
+        const KIB: u64 = 1024;
+        const MIB64: u64 = 1024 * KIB;
+        const GIB: u64 = 1024 * MIB64;
+        // push/pull 10k × 4 KiB: 40 MiB is the 2-stream byte tier, but
+        // 10_000 files must key the 8-stream file-count tier.
+        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
+        // 1 × 1 GiB: byte-keyed, file count is irrelevant — unchanged.
+        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
+        // mixed 512 MiB + 5k × 2 KiB: the byte tier already reaches 8;
+        // the 5_001 files alone would say 4 — bytes win.
+        assert_eq!(
+            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
+            8
+        );
+        // sf-1 loopback probe evidence: 1_000 tiny files must propose 2
+        // (the measured transfer rode 1 — the input, not this table,
+        // was wrong).
+        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
+    }
+
+    #[test]
+    fn shape_resize_ramps_one_epoch_at_a_time_toward_the_target() {
+        let dial = TransferDial::conservative();
+        dial.set_negotiated_streams(1);
+
+        // At or below live: nothing to correct.
+        assert_eq!(dial.propose_shape_resize(0), None);
+        assert_eq!(dial.propose_shape_resize(1), None);
+
+        // Target 3 from live 1: epoch 1 proposes 2 (one per epoch),
+        // and the in-flight epoch blocks both proposers.
+        let p1 = dial.propose_shape_resize(3).expect("live 1 → target 3");
+        assert_eq!(
+            p1,
+            ResizeProposal {
+                epoch: 1,
+                target_streams: 2,
+                add: true
+            }
+        );
+        assert_eq!(dial.propose_shape_resize(3), None, "one in flight");
+        assert_eq!(dial.resize_tick(1024, 0.0), None, "tuner blocked too");
+
+        // Settle → next step; no cooldown for the definite shape signal.
+        dial.resize_settled(1, 2, true);
+        let p2 = dial.propose_shape_resize(3).expect("live 2 → target 3");
+        assert_eq!(p2.epoch, 2);
+        assert_eq!(p2.target_streams, 3);
+        dial.resize_settled(2, 3, true);
+        assert_eq!(dial.live_streams(), 3);
+        assert_eq!(dial.propose_shape_resize(3), None, "target reached");
+
+        // A refused epoch leaves live untouched; the next call retries.
+        let p3 = dial.propose_shape_resize(4).expect("live 3 → target 4");
+        dial.resize_settled(p3.epoch, dial.live_streams(), false);
+        assert_eq!(dial.live_streams(), 3);
+        assert!(
+            dial.propose_shape_resize(4).is_some(),
+            "retry after refusal"
+        );
+    }
+
+    #[test]
+    fn shape_resize_clamps_to_the_profile_ceiling() {
+        let dial = TransferDial::conservative_within(Some(&profile(2, 0, 0)));
+        dial.set_negotiated_streams(1);
+        let p = dial
+            .propose_shape_resize(100)
+            .expect("clamped, not refused");
+        assert_eq!(p.target_streams, 2);
+        dial.resize_settled(p.epoch, 2, true);
+        assert_eq!(
+            dial.propose_shape_resize(100),
+            None,
+            "at the receiver's advertised ceiling"
+        );
+    }
+
     #[tokio::test(start_paused = true)]
     async fn tuner_forwards_resize_proposals_over_the_shared_registry() {
         use crate::remote::transfer::progress::{StreamId, StreamProbe};
diff --git a/crates/blit-core/src/remote/push/client/mod.rs b/crates/blit-core/src/remote/push/client/mod.rs
index 10e2907..265dc28 100644
--- a/crates/blit-core/src/remote/push/client/mod.rs
+++ b/crates/blit-core/src/remote/push/client/mod.rs
@@ -476,6 +476,74 @@ fn ensure_dial(
         .expect("dial set by preceding assignment")
 }
 
+/// ue-r2-2 / sf-2 shared pre-dial ADD: mint the epoch credential, send
+/// the `DataPlaneResize` ADD, and record the in-flight epoch (the
+/// socket itself is dialed on the daemon's ack). A missing credential
+/// source settles the epoch failed and is not an error; a send error
+/// is returned for the caller to route through `prefer_server_error`.
+async fn send_resize_add(
+    tx: &mpsc::Sender<ClientPushRequest>,
+    dial: &crate::engine::TransferDial,
+    proposal: crate::engine::ResizeProposal,
+    resize_pending: &mut Option<PendingResize>,
+) -> Result<()> {
+    match crate::remote::transfer::generate_sub_token() {
+        Ok(sub) => {
+            send_payload(
+                tx,
+                ClientPayload::DataPlaneResize(DataPlaneResize {
+                    op: DataPlaneResizeOp::Add as i32,
+                    epoch: proposal.epoch,
+                    target_stream_count: proposal.target_streams as u32,
+                    sub_token: sub.clone(),
+                }),
+            )
+            .await?;
+            *resize_pending = Some(PendingResize {
+                epoch: proposal.epoch,
+                target: proposal.target_streams,
+                add: true,
+                sub_token: sub,
+            });
+        }
+        Err(err) => {
+            log::warn!("resize ADD skipped (no credential source): {err:#}");
+            dial.resize_settled(proposal.epoch, dial.live_streams(), false);
+        }
+    }
+    Ok(())
+}
+
+/// sf-2: one shape-correction step. The daemon proposes the epoch-0
+/// stream count from whatever manifest prefix it had seen at its early
+/// flush, so a many-tiny-file push can negotiate far fewer streams
+/// than the shape table assigns the full workload
+/// (`.review/findings/sf-1-tripwire-harness.md` Known gaps: a
+/// 1000-file push measured 1 stream where the table says 2). As the
+/// need list accumulates, re-run [`crate::engine::initial_stream_proposal`]
+/// over the ACTUAL transfer shape (need-list files + bytes, not the
+/// manifest — an incremental push of a large tree may move only a few
+/// files) and correct upward one ADD epoch at a time. Call sites gate
+/// on the transfer running resize-enabled on the data plane.
+async fn maybe_shape_resize(
+    tx: &mpsc::Sender<ClientPushRequest>,
+    dial: &crate::engine::TransferDial,
+    need_bytes: u64,
+    need_count: usize,
+    resize_pending: &mut Option<PendingResize>,
+) -> Result<()> {
+    if resize_pending.is_some() {
+        return Ok(());
+    }
+    let target =
+        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())
+            as usize;
+    match dial.propose_shape_resize(target) {
+        Some(proposal) => send_resize_add(tx, dial, proposal, resize_pending).await,
+        None => Ok(()),
+    }
+}
+
 fn prune_unrequested_payloads(
     payloads: &mut Vec<TransferPayload>,
     requested: &mut HashSet<String>,
@@ -674,6 +742,14 @@ impl RemotePushClient {
             tokio::sync::mpsc::UnboundedReceiver<crate::engine::ResizeProposal>,
         > = None;
         let mut resize_pending: Option<PendingResize> = None;
+        // sf-2: shape-correction gate. `resize_negotiated` records that
+        // this transfer's data plane went elastic (epoch-0 sub-token
+        // present). `shape_resize_enabled` flips off permanently the
+        // first time the tuner proposes a REMOVE — live throughput
+        // evidence outranks the static shape table, and re-adding what
+        // the tuner just retired would flap.
+        let mut resize_negotiated = false;
+        let mut shape_resize_enabled = true;
 
         let mut manifest_done = false;
         // Track whether we received new need-list entries this iteration.
@@ -777,6 +853,32 @@ impl RemotePushClient {
                                             }
                                         }
                                         TransferMode::DataPlane => {
+                                            // sf-2: the need list just grew —
+                                            // re-run the shape table and
+                                            // correct the stream count before
+                                            // queueing the batch.
+                                            if resize_negotiated
+                                                && shape_resize_enabled
+                                                && data_plane_sender.is_some()
+                                            {
+                                                if let Some(dial_ref) = dial.as_ref() {
+                                                    if let Err(send_err) = maybe_shape_resize(
+                                                        &tx,
+                                                        dial_ref,
+                                                        transfer_size_hint,
+                                                        requested_files.len(),
+                                                        &mut resize_pending,
+                                                    )
+                                                    .await
+                                                    {
+                                                        return Err(prefer_server_error(
+                                                            &mut response_rx,
+                                                            send_err,
+                                                        )
+                                                        .await);
+                                                    }
+                                                }
+                                            }
                                             if let Some(sender) = data_plane_sender.as_mut() {
                                                 let headers =
                                                     drain_pending_headers(&mut pending_queue, &manifest_lookup);
@@ -907,6 +1009,7 @@ impl RemotePushClient {
                                                 && neg.epoch0_sub_token.len()
                                                     == crate::remote::transfer::SUB_TOKEN_LEN)
                                                 .then(|| neg.epoch0_sub_token.clone());
+                                            resize_negotiated = resize_sub.is_some();
                                             let mut sender = MultiStreamSender::connect(
                                                 &self.endpoint.host,
                                                 neg.tcp_port,
@@ -925,6 +1028,29 @@ impl RemotePushClient {
                                             resize_proposal_rx = sender.take_resize_rx();
                                             data_plane_sender = Some(sender);
                                             data_port = Some(neg.tcp_port);
+
+                                            // sf-2: need-list batches can
+                                            // predate the negotiation — the
+                                            // accumulated shape may already
+                                            // outgrow the daemon's
+                                            // partial-manifest stream count.
+                                            if resize_negotiated && shape_resize_enabled {
+                                                if let Err(send_err) = maybe_shape_resize(
+                                                    &tx,
+                                                    &dial,
+                                                    transfer_size_hint,
+                                                    requested_files.len(),
+                                                    &mut resize_pending,
+                                                )
+                                                .await
+                                                {
+                                                    return Err(prefer_server_error(
+                                                        &mut response_rx,
+                                                        send_err,
+                                                    )
+                                                    .await);
+                                                }
+                                            }
                                         }
 
                                         if let Some(sender) = data_plane_sender.as_mut() {
@@ -1032,6 +1158,32 @@ impl RemotePushClient {
                                                     false,
                                                 );
                                             }
+                                            // sf-2: the epoch settled — if the
+                                            // need-list shape still wants more
+                                            // streams, propose the next single
+                                            // ADD (the ramp is one stream per
+                                            // acked epoch).
+                                            if resize_negotiated
+                                                && shape_resize_enabled
+                                                && data_plane_sender.is_some()
+                                            {
+                                                let dial_ref = dial_ref.clone();
+                                                if let Err(send_err) = maybe_shape_resize(
+                                                    &tx,
+                                                    &dial_ref,
+                                                    transfer_size_hint,
+                                                    requested_files.len(),
+                                                    &mut resize_pending,
+                                                )
+                                                .await
+                                                {
+                                                    return Err(prefer_server_error(
+                                                        &mut response_rx,
+                                                        send_err,
+                                                    )
+                                                    .await);
+                                                }
+                                            }
                                         }
                                         other => {
                                             resize_pending = other;
@@ -1226,44 +1378,21 @@ impl RemotePushClient {
                                 // Pre-dial ADD: mint the epoch credential,
                                 // ask the daemon to register it and arm an
                                 // accept; the dial happens on the ack.
-                                match crate::remote::transfer::generate_sub_token() {
-                                    Ok(sub) => {
-                                        if let Err(send_err) = send_payload(
-                                            &tx,
-                                            ClientPayload::DataPlaneResize(DataPlaneResize {
-                                                op: DataPlaneResizeOp::Add as i32,
-                                                epoch: p.epoch,
-                                                target_stream_count: p.target_streams as u32,
-                                                sub_token: sub.clone(),
-                                            }),
-                                        )
-                                        .await
-                                        {
-                                            return Err(prefer_server_error(
-                                                &mut response_rx,
-                                                send_err,
-                                            )
-                                            .await);
-                                        }
-                                        resize_pending = Some(PendingResize {
-                                            epoch: p.epoch,
-                                            target: p.target_streams,
-                                            add: true,
-                                            sub_token: sub,
-                                        });
-                                    }
-                                    Err(err) => {
-                                        log::warn!(
-                                            "resize ADD skipped (no credential source): {err:#}"
-                                        );
-                                        dial_ref.resize_settled(
-                                            p.epoch,
-                                            dial_ref.live_streams(),
-                                            false,
-                                        );
-                                    }
+                                if let Err(send_err) =
+                                    send_resize_add(&tx, dial_ref, p, &mut resize_pending).await
+                                {
+                                    return Err(prefer_server_error(
+                                        &mut response_rx,
+                                        send_err,
+                                    )
+                                    .await);
                                 }
                             } else {
+                                // sf-2: the tuner wants FEWER streams — live
+                                // throughput evidence outranks the static
+                                // shape table from here on. Never re-add what
+                                // the tuner retires.
+                                shape_resize_enabled = false;
                                 // REMOVE: retire locally first — the drained
                                 // worker's END record is the daemon-side
                                 // teardown — then tell the daemon
@@ -1385,6 +1514,10 @@ impl RemotePushClient {
             data_port,
             summary,
             first_payload_elapsed,
+            data_plane_streams: match (&dial, data_port) {
+                (Some(dial), Some(_)) => Some(dial.live_streams()),
+                _ => None,
+            },
         })
     }
 }
diff --git a/crates/blit-core/src/remote/push/client/types.rs b/crates/blit-core/src/remote/push/client/types.rs
index 73fb2a2..f175bda 100644
--- a/crates/blit-core/src/remote/push/client/types.rs
+++ b/crates/blit-core/src/remote/push/client/types.rs
@@ -9,6 +9,11 @@ pub struct RemotePushReport {
     pub data_port: Option<u32>,
     pub summary: PushSummary,
     pub first_payload_elapsed: Option<Duration>,
+    /// sf-2: the dial's settled live stream count when the transfer
+    /// finished (`None` on the gRPC fallback path — no data plane).
+    /// Observable pin for the shape-correction resize: a many-tiny-file
+    /// push must end above the 1-stream partial-manifest proposal.
+    pub data_plane_streams: Option<usize>,
 }
 
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
diff --git a/crates/blit-daemon/src/service/push/mod.rs b/crates/blit-daemon/src/service/push/mod.rs
index a5226b2..6fa15bf 100644
--- a/crates/blit-daemon/src/service/push/mod.rs
+++ b/crates/blit-daemon/src/service/push/mod.rs
@@ -1,5 +1,7 @@
 mod control;
 mod data_plane;
+#[cfg(test)]
+mod shape_resize_e2e;
 
 pub(crate) use control::handle_push_stream;
 pub(crate) use data_plane::{bind_data_plane_listener, generate_token, TransferStats};
diff --git a/crates/blit-daemon/src/service/push/shape_resize_e2e.rs b/crates/blit-daemon/src/service/push/shape_resize_e2e.rs
new file mode 100644
index 0000000..d7f61aa
--- /dev/null
+++ b/crates/blit-daemon/src/service/push/shape_resize_e2e.rs
@@ -0,0 +1,117 @@
+//! sf-2 loopback e2e pin (`docs/plan/SMALL_FILE_CEILING.md`, slice
+//! sf-2): a many-tiny-file push must open more than one data-plane
+//! connection.
+//!
+//! The daemon proposes the epoch-0 stream count at its early manifest
+//! flush (`FILE_LIST_EARLY_FLUSH_ENTRIES` = 128 entries), so a 10k-file
+//! push used to negotiate from a ~128-file prefix — 1 stream — and ride
+//! it for the whole transfer (measured on the 10 GbE rig and again by
+//! the sf-1 loopback probe; see DIAGNOSIS.md in
+//! `docs/bench/10gbe-2026-07-05/`). The client-side shape-correction
+//! resize (`maybe_shape_resize` in blit-core's push client) re-runs the
+//! shape table over the accumulated need list and corrects upward
+//! through the ue-r2-2 resize wire. This test runs the REAL daemon push
+//! service in-process and the REAL client against it, then pins the
+//! settled stream count above 1.
+
+use std::collections::HashMap;
+use std::path::PathBuf;
+use std::sync::Arc;
+
+use blit_core::fs_enum::FileFilter;
+use blit_core::generated::blit_server::BlitServer;
+use blit_core::generated::MirrorMode;
+use blit_core::remote::transfer::source::FsTransferSource;
+use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePushClient};
+
+use crate::runtime::ModuleConfig;
+use crate::service::BlitService;
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+async fn many_tiny_file_push_opens_more_than_one_data_plane_connection() {
+    let dest = tempfile::tempdir().expect("dest dir");
+    let canonical = dest.path().canonicalize().expect("canonical dest");
+    let mut modules = HashMap::new();
+    modules.insert(
+        "test".to_string(),
+        ModuleConfig {
+            name: "test".into(),
+            path: canonical.clone(),
+            canonical_root: canonical.clone(),
+            read_only: false,
+            _comment: None,
+            delegation_allowed: true,
+        },
+    );
+    let service = BlitService::with_modules(modules, false);
+
+    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
+        .await
+        .expect("bind loopback listener");
+    let port = listener.local_addr().expect("listener addr").port();
+    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
+    let server = tokio::spawn(async move {
+        blit_core::remote::grpc_server::production_server_builder()
+            .add_service(BlitServer::new(service))
+            .serve_with_incoming_shutdown(
+                tokio_stream::wrappers::TcpListenerStream::new(listener),
+                async {
+                    let _ = shutdown_rx.await;
+                },
+            )
+            .await
+            .expect("in-process daemon serves");
+    });
+
+    // The plan's small-file cell: 10k tiny files. The shape table
+    // assigns 8 streams (file-count tier); the early-flush proposal
+    // sees only the first manifest chunk and starts at 1.
+    const FILE_COUNT: usize = 10_000;
+    let src = tempfile::tempdir().expect("src dir");
+    for i in 0..FILE_COUNT {
+        std::fs::write(src.path().join(format!("f{i:05}.bin")), b"x").expect("seed source file");
+    }
+
+    let endpoint = RemoteEndpoint {
+        host: "127.0.0.1".into(),
+        port,
+        path: RemotePath::Module {
+            module: "test".into(),
+            rel_path: PathBuf::new(),
+        },
+    };
+    let mut client = RemotePushClient::connect(endpoint)
+        .await
+        .expect("client connects");
+    let source = Arc::new(FsTransferSource::new(src.path().to_path_buf()));
+    let report = client
+        .push(
+            source,
+            &FileFilter::default(),
+            false,
+            MirrorMode::FilteredSubset,
+            false,
+            false,
+            None,
+            false,
+        )
+        .await
+        .expect("push succeeds");
+
+    assert!(!report.fallback_used, "must ride the TCP data plane");
+    assert_eq!(
+        report.summary.files_transferred as usize, FILE_COUNT,
+        "every file arrives"
+    );
+    let streams = report
+        .data_plane_streams
+        .expect("data plane ran, stream count recorded");
+    assert!(
+        streams > 1,
+        "a {FILE_COUNT}-file push must correct the partial-manifest \
+         1-stream proposal upward via shape resize; settled at {streams}"
+    );
+
+    let _ = shutdown_tx.send(());
+    server.await.expect("server task joins");
+}

exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/plan/SMALL_FILE_CEILING.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Small-file transfer to the hardware ceiling

**Status**: Active
**Created**: 2026-07-05
**Supersedes**: nothing
**Decision ref**: D-2026-07-04-4 (Draft → Active, owner "go")

## Principle (owner, 2026-07-05)

blit's guiding principles are **FAST, SIMPLE, RELIABLE** — every
change serves at least one or it's scrapped. blit must be the
fastest way to transfer files in **any** scenario. Goals are
therefore **ceiling-driven, never competitor-relative**: a
"beat tool X by N%" bar embeds a stopping condition and is the wrong
way to engineer this tool. Other tools function only as
**tripwires** — any scenario where any tool measures faster than
blit is, by definition, proof blit is off its hardware ceiling and
is a finding to fix, regardless of margins.

## Goal

For the workload classes where the 2026-07-04/05 10 GbE session
measured blit off its ceiling — many-tiny-file and mixed transfers —
blit's wall time becomes bounded by a **named hardware limit** (wire,
target-filesystem parallel create floor, source enumeration floor),
demonstrated by profile evidence and a stream-scaling curve, not by
blit's own stream policy or per-file overhead.

Measured gap analysis (durable evidence:
`docs/bench/10gbe-2026-07-05/` — DIAGNOSIS.md carries the daemon-log
extracts and arithmetic; the CSVs carry every matrix cell; DEVLOG
2026-07-05 entries are the narrative record):

| cell | blit today | ceiling arithmetic | tripwire |
|---|---|---|---|
| push 10k×4 KiB | 2.4–3.3 s | wire: **34 ms** (40 MiB @ 9.9 Gbit); fs floor: ~150 µs/file proven single-pipe on this ZFS, ÷ parallelism → **~0.2–0.5 s** | rsyncd 1.5 s |
| pull 10k×4 KiB | 446–484 ms | client fs = tmpfs (µs creates); wire+protocol class: **≪ 200 ms** | rsyncd 367 ms |
| push mixed 512 MiB+5k | 1.8–2.2 s | big file alone: ~450 ms wire; small remainder as above | rsyncd 1.24 s |

Diagnosis (from the session's daemon logs): the 10k push rode **one
stream** — `engine::initial_stream_proposal` is byte-weighted, so
40 MiB proposes a single stream despite 10,000 files — and paid
~215 µs/file sequentially on the daemon. The parallel machinery
(elastic streams, work-stealing, mid-transfer resize) exists and
negotiated 8 connections for the 1 GiB push in the same session.
This is a policy gap plus per-file overhead, not missing machinery.

## Non-goals

- Competitor-relative targets of any kind (see Principle).
- WAN/latency-shaped tuning (separate scenario class; gets its own
  ceiling analysis when a rig exists).
- Non-Linux rig ceiling targets (no measurement hardware this plan
  can bind to; Windows/macOS must not regress — suite + CI guard).
- Encrypted-transport scenarios (ssh-wrapped tools measured only as
  tripwires; blit's transport security model is unchanged by this
  plan).

## Constraints

- Every slice serves FAST without violating SIMPLE (dial stays the
  single tuning owner; no second engine, no special-case paths that
  survive past their measured need) or RELIABLE (REV4 invariants:
  byte-identical, StallGuard, cancellation, byte accounting).
- No wire-visible protocol change without a dedicated owner gate on
  the wire design before code (sf-6); mixed-version peers keep
  working via existing negotiation.
- No measured cell regresses beyond run-to-run noise (±10%),
  guarded by the committed baseline.
- Test count never drops; every slice through the codex loop
  (D-2026-07-04-1).
- Small-file parallel writes must respect the receiver capacity
  profile (spinning-pool receivers bound their own parallelism —
  the existing bounded-unilateral dial contract, D-2026-06-20-1).

## Acceptance criteria

- [ ] For each cell above: a recorded **limiter analysis** (profile
      + stream-scaling curve, committed with the slice records)
      demonstrating wall time is bound by a named hardware limit,
      not by stream policy or blit-controlled per-file overhead.
- [ ] Scaling evidence: files/s rises with stream count until the
      named limiter binds — the curve flattens at hardware, not at
      policy.
- [ ] **Tripwires clean**: no tool in the committed sf-1 harness
      matrix — rsyncd, rsync-over-ssh, rclone in its best measured
      config (`--ignore-checksum`, tuned `--transfers`), and `cp -a`
      for local cells — measures faster than blit on any cell. (The
      harness and this list are the same set by construction; adding
      a tripwire tool means adding it to the harness.)
- [ ] All baseline matrix cells stay within run-to-run noise (±10%)
      of the committed `docs/bench/10gbe-2026-07-05/` baseline.
- [ ] The comparison + scaling harness is committed and the owner
      can rerun it against any daemon host in one command.

## Design

Levers, cheapest first, measuring between each — sequencing exists
to find the ceiling with the least machinery, not to stop early:

1. **File-count-aware stream proposal** (blit-core `engine/`):
   `initial_stream_proposal` (and the pull-side equivalent) weight
   file count alongside bytes so many-tiny-file manifests open
   multiple streams; work-stealing spreads per-file cost across
   daemon workers. Push knows counts from enumeration, pull from
   the manifest.
2. **Per-file cost to the syscall floor** (daemon receive + client
   pull write paths): profile first (`strace -c`/`perf` during a
   small transfer), then cut — candidates: temp-file+rename
   pattern, separate set-times/set-perms syscalls, per-file
   need-list echo. The profile, not intuition, names the cuts.
3. **Resize-on-file-backlog**: feed the existing ue-2 resize
   machinery a backlog signal so a stream drowning in tiny files
   triggers mid-transfer ADD — this is also the organic resize
   trigger byte-bound workloads can never produce.
4. **Tar-shard push lane** (wire-visible, own owner gate): bundle
   tiny files into shard frames on the push wire as the local
   engine and delegated lane already do — amortizes both protocol
   roundtrips and daemon syscalls. Reached when the limiter
   analysis shows per-file framing itself is the binding cost.

Risks: parallel small-file writes can seek-storm spinning pools —
bounded by the receiver capacity profile (constraint above); lever 2
touches platform-sensitive syscall paths — Windows suite must stay
green; lever 4 adds wire complexity — SIMPLE requires the limiter
analysis to prove it earns its keep before design review.

## Slices

1. **sf-1 tripwire harness**: commit `scripts/bench_tripwires.sh`
   (derived from the session's ad-hoc runner): full matrix — blit,
   rsyncd (spun on the daemon host over ssh), rsync-over-ssh,
   rclone best-config, `cp -a` local — fresh targets every run,
   plus a stream-scaling probe mode (files/s vs stream count). The
   2026-07-05 baseline already lives in `docs/bench/10gbe-2026-07-05/`
   (committed with this plan); sf-1 makes it re-runnable in one
   command. No production code.
2. **sf-2 dial file-count weighting**: proposal-table unit pins
   (10k tiny → multi-stream; 1×1 GiB unchanged; mixed →
   intermediate) + loopback e2e pin that a 10k-file push opens >1
   data-plane connection.
3. **sf-3a per-file cost limiter analysis** (analysis-only, w8-1b
   precedent): `strace -c`/`perf` profile of daemon receive and
   client pull-write during a small transfer; deliverable is a
   committed analysis naming each per-file syscall cost and the
   ordered list of candidate cuts, each with its expected saving.
   No code.
4. **sf-3b… one cut per slice**: each accepted cut from sf-3a lands
   as its own review-loop slice with its own loopback
   per-file-cost proxy pin (so CI catches gross regressions
   without the rig). The count of sf-3x slices is set by sf-3a's
   list, not guessed here.
5. **sf-4 rig re-measure + limiter analysis**: rerun sf-1 harness on
   the 10 GbE rig; record the limiter analysis per cell. Hardware-
   bound everywhere + tripwires clean → acceptance review with the
   owner. Otherwise the analysis names what binds; continue.
6. **sf-5 resize-on-backlog feed** (if sf-4 names stream count
   under load as a binder, or the owner wants the ue-2 organic
   trigger regardless — flagged at sf-4).
7. **sf-6 tar-shard push lane** (if sf-4/sf-5's analysis names
   per-file wire framing as the binder). Wire-visible; the owner
   gate consumes the full REV4 wire-contract deliverable set
   **before any code**: the proto messages/fields and their field
   numbers named; capability negotiation for the shard lane
   specified; old-client→new-daemon and new-client→old-daemon
   behavior stated; and mixed-version compatibility tests specified
   and landing **before** any behavior depends on the lane.
8. **sf-7 verdict**: final rig run, limiter analyses committed,
   acceptance checklist walked with the owner; plan → Shipped or
   the remaining gap gets its own named follow-on.

## Open questions

- **sf-6 wire gate** (standing): the tar-shard lane's wire design
  needs explicit owner sign-off at execution time — recorded here
  so no session treats sf-6 as pre-authorized code. — owner

codex
The commit is small but touches shared resize state, so I’m checking the surrounding dial and push-client event loop rather than only the patch hunks. That should expose any epoch, cancellation, or mixed-version edge cases hidden outside the diff.
exec
/usr/bin/zsh -lc 'rg -n "struct TransferDial|pending_epoch|resize_epoch|resize_tick|resize_settled|initial_stream_proposal|ceiling_max_streams|live_streams" crates/blit-core/src/engine/dial.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
86:pub struct TransferDial {
97:    /// `resize_settled` on an accepted epoch.
98:    live_streams: AtomicUsize,
100:    resize_epoch: AtomicU32,
104:    pending_epoch: AtomicU32,
114:    ceiling_max_streams: usize,
121:/// [`TransferDial::resize_settled`] for the epoch — with what actually
125:    /// The wire epoch for this change (`resize_epoch() + 1`).
176:            live_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
177:            resize_epoch: AtomicU32::new(0),
178:            pending_epoch: AtomicU32::new(0),
183:            ceiling_max_streams: ceiling_streams,
214:    pub fn ceiling_max_streams(&self) -> usize {
215:        self.ceiling_max_streams
220:    /// it also seeds `live_streams`, the baseline every `ue-r2-2`
223:        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
225:        self.live_streams.store(clamped, Ordering::Relaxed);
233:    pub fn live_streams(&self) -> usize {
234:        self.live_streams.load(Ordering::Relaxed)
238:    pub fn resize_epoch(&self) -> u32 {
239:        self.resize_epoch.load(Ordering::Relaxed)
242:    /// True while a proposal is awaiting `resize_settled`.
244:        self.pending_epoch.load(Ordering::Relaxed) != 0
266:    /// tuner. Bounds: `1..=ceiling_max_streams` (the receiver profile
271:    /// call [`Self::resize_settled`] with the outcome; until then
273:    pub fn resize_tick(&self, delta_bytes: u64, blocked_ratio: f64) -> Option<ResizeProposal> {
274:        if self.pending_epoch.load(Ordering::Relaxed) != 0 {
285:        let live = self.live_streams.load(Ordering::Relaxed).max(1);
304:            (live + 1).min(self.ceiling_max_streams.max(1))
315:        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
320:            .pending_epoch
338:    /// [`initial_stream_proposal`] assigns the full workload. As the
342:    /// Unlike [`Self::resize_tick`] this is a definite signal — the
350:        let desired = desired_streams.clamp(1, self.ceiling_max_streams.max(1));
351:        let live = self.live_streams.load(Ordering::Relaxed).max(1);
355:        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
357:            .pending_epoch
376:    pub fn resize_settled(&self, epoch: u32, effective_streams: usize, accepted: bool) {
377:        if self.pending_epoch.load(Ordering::Relaxed) != epoch || epoch == 0 {
380:        self.pending_epoch.store(0, Ordering::Relaxed);
384:            let clamped = effective_streams.clamp(1, self.ceiling_max_streams.max(1));
385:            self.live_streams.store(clamped, Ordering::Relaxed);
386:            self.resize_epoch.store(epoch, Ordering::Relaxed);
393:        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
474:pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
534:/// provided — each [`TransferDial::resize_tick`] proposal is forwarded
570:                    dial.resize_tick(0, 0.0);
582:            // F3): the idle tick must still reach `resize_tick` so a
587:                    dial.resize_tick(0, 0.0);
594:                if let Some(proposal) = dial.resize_tick(delta_bytes, ratio) {
599:                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
642:        assert_eq!(dial.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
653:        assert_eq!(dial.ceiling_max_streams(), 4);
666:        assert_eq!(generous.ceiling_max_streams(), DIAL_CEILING_MAX_STREAMS);
687:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
691:        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
695:        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
696:        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
697:        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
698:        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
699:        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
700:        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
701:        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
702:        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
703:        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
704:        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
705:        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
706:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
708:        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
709:        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
710:        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
711:        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
712:        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
713:        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
715:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
716:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
782:        assert_eq!(dial.live_streams(), 3, "negotiation seeds the live count");
790:            assert_eq!(dial.resize_tick(1024, 0.15), None, "in-band tick holds");
802:            assert_eq!(dial.resize_tick(1024, 0.0), None);
808:        assert_eq!(dial.resize_tick(1024, 0.0), None, "sustain tick 1");
810:            .resize_tick(1024, 0.0)
824:            assert_eq!(dial.resize_tick(1024, 0.0), None, "pending blocks");
829:        dial.resize_settled(1, 5, true);
830:        assert_eq!(dial.live_streams(), 5);
831:        assert_eq!(dial.resize_epoch(), 1);
834:            assert_eq!(dial.resize_tick(1024, 0.0), None, "cooldown holds");
838:        let next = dial.resize_tick(1024, 0.0).expect("epoch 2 proposes");
851:        assert_eq!(dial.resize_tick(1024, 0.9), None, "sustain tick 1");
852:        let proposal = dial.resize_tick(1024, 0.9).expect("sustained block drops");
861:        dial.resize_settled(1, 1, true);
862:        assert_eq!(dial.live_streams(), 1);
867:            assert_eq!(dial.resize_tick(1024, 0.9), None, "floor at 1");
880:        assert_eq!(dial.resize_tick(1024, 0.0), None);
881:        assert_eq!(dial.resize_tick(0, 0.0), None, "idle resets");
882:        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
884:        assert_eq!(dial.resize_tick(1024, 0.15), None, "in-band resets");
885:        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
886:        assert!(dial.resize_tick(1024, 0.0).is_some(), "streak completes");
895:        assert_eq!(dial.resize_tick(1024, 0.0), None);
896:        let proposal = dial.resize_tick(1024, 0.0).expect("proposes");
899:        dial.resize_settled(proposal.epoch + 7, 9, true);
903:        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
905:        assert_eq!(dial.live_streams(), 4);
906:        assert_eq!(dial.resize_epoch(), 0, "refused epoch never settles");
917:                dial.resize_tick(1024, 0.0),
936:        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
938:        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
942:            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
948:        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
972:        assert_eq!(dial.resize_tick(1024, 0.0), None, "tuner blocked too");
975:        dial.resize_settled(1, 2, true);
979:        dial.resize_settled(2, 3, true);
980:        assert_eq!(dial.live_streams(), 3);
985:        dial.resize_settled(p3.epoch, dial.live_streams(), false);
986:        assert_eq!(dial.live_streams(), 3);
1001:        dial.resize_settled(p.epoch, 2, true);

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/dial.rs | sed -n '1,430p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! The single live transfer dial (`ue-r2-1e`, REV4 Design §4).
     2	//!
     3	//! Sender-owned, receiver-bounded: the byte sender constructs one
     4	//! `TransferDial` per transfer, clamped by the receiver's advertised
     5	//! [`CapacityProfile`] (the `ue-r2-1b` wire fields), starts at the
     6	//! conservative floor (D-2026-06-20-1/-2: no probe phase, no
     7	//! size-gated start — begin immediately and tune live), and a tuner
     8	//! steps the cheap dials from the PR1 stream telemetry.
     9	//!
    10	//! Mutability model (the C-ready seam `ue-r2-2` builds on):
    11	//! - **Cheap dials** — `chunk_bytes`, `prefetch_count`: atomics the
    12	//!   tuner steps mid-transfer. Consumers read them when a session,
    13	//!   pipeline, or fallback batch is set up, so a step takes effect for
    14	//!   sockets/batches started afterwards (epoch-N resize adds, the next
    15	//!   gRPC-fallback batch) — existing sessions keep their snapshot.
    16	//! - **Connect-time dials** — `tcp_buffer_bytes`, buffer-pool sizing:
    17	//!   read when a socket/pool is built; changes affect sockets opened
    18	//!   afterwards (no setsockopt on live sockets this slice).
    19	//! - **Negotiated once** — `initial_streams`/`max_streams`: stream
    20	//!   count becomes live at `ue-r2-2` (DataPlaneResize); until then the
    21	//!   dial only carries the negotiation-time value and the
    22	//!   profile-clamped ceiling.
    23	//!
    24	//! This replaces the size-keyed `determine_remote_tuning` static
    25	//! ladder: the ladder's floor tier is the dial's start, its top tier
    26	//! is the dial's default ceiling, and everything between is reached by
    27	//! ramping on evidence instead of guessing from `total_bytes`.
    28	
    29	use std::sync::atomic::{AtomicI32, AtomicU32, AtomicUsize, Ordering};
    30	use std::sync::Arc;
    31	
    32	use crate::generated::CapacityProfile;
    33	
    34	const MIB: usize = 1024 * 1024;
    35	
    36	/// Floor (conservative start) values — the old ladder's smallest tier.
    37	pub const DIAL_FLOOR_CHUNK_BYTES: usize = 16 * MIB;
    38	pub const DIAL_FLOOR_PREFETCH: usize = 4;
    39	pub const DIAL_FLOOR_INITIAL_STREAMS: usize = 4;
    40	pub const DIAL_FLOOR_MAX_STREAMS: usize = 8;
    41	
    42	/// Default ceilings — the old ladder's top tier (a fully ramped dial
    43	/// matches today's best static behavior).
    44	pub const DIAL_CEILING_CHUNK_BYTES: usize = 64 * MIB;
    45	pub const DIAL_CEILING_PREFETCH: usize = 32;
    46	pub const DIAL_CEILING_MAX_STREAMS: usize = 32;
    47	pub const DIAL_CEILING_TCP_BUFFER_BYTES: usize = 8 * MIB;
    48	
    49	/// Tuner policy (initial, deliberately simple): sampled every
    50	/// [`DIAL_TUNER_TICK`]; below [`DIAL_STEP_UP_BLOCKED_RATIO`] blocked
    51	/// time the pipe is not back-pressured → step up; above
    52	/// [`DIAL_STEP_DOWN_BLOCKED_RATIO`] → step down. One step per tick
    53	/// (hysteresis by construction).
    54	pub const DIAL_TUNER_TICK: std::time::Duration = std::time::Duration::from_millis(500);
    55	pub const DIAL_STEP_UP_BLOCKED_RATIO: f64 = 0.05;
    56	pub const DIAL_STEP_DOWN_BLOCKED_RATIO: f64 = 0.30;
    57	
    58	/// Resize policy (`ue-r2-2`): streams are the EXPENSIVE dial — a step
    59	/// costs a control round-trip plus a TCP connect — so they move only
    60	/// after the cheap dials are pinned at a bound and the signal has held
    61	/// for [`RESIZE_SUSTAIN_TICKS`] consecutive ticks, and never within
    62	/// [`RESIZE_COOLDOWN_TICKS`] of the previous settle. One stream per
    63	/// epoch (the wire carries one `sub_token` per ADD).
    64	pub const RESIZE_COOLDOWN_TICKS: u32 = 4;
    65	pub const RESIZE_SUSTAIN_TICKS: i32 = 2;
    66	
    67	/// The capacity profile this host advertises when it is the byte
    68	/// RECEIVER (ue-r2-1e: the first real sender of the ue-r2-1b wire
    69	/// fields). Honest system facts only — fields we cannot measure yet
    70	/// stay 0 (= unknown per the wire contract), never fabricated:
    71	/// ceilings mirror what today's receive paths actually accept.
    72	pub fn local_receiver_capacity() -> CapacityProfile {
    73	    CapacityProfile {
    74	        cpu_cores: num_cpus::get() as u32,
    75	        drain_class: 0,
    76	        load_percent: 0,
    77	        max_streams: DIAL_CEILING_MAX_STREAMS as u32,
    78	        drain_rate_bytes_per_sec: 0,
    79	        max_chunk_bytes: DIAL_CEILING_CHUNK_BYTES as u64,
    80	        max_inflight_bytes: (DIAL_CEILING_CHUNK_BYTES * DIAL_CEILING_PREFETCH) as u64,
    81	    }
    82	}
    83	
    84	/// The one mutable tuning object for a transfer.
    85	#[derive(Debug)]
    86	pub struct TransferDial {
    87	    chunk_bytes: AtomicUsize,
    88	    prefetch_count: AtomicUsize,
    89	    /// 0 = unset (kernel default), matching the old `Option<usize>`.
    90	    tcp_buffer_bytes: AtomicUsize,
    91	    initial_streams: AtomicUsize,
    92	    max_streams: AtomicUsize,
    93	    // ── ue-r2-2 resize state (all epochs are the wire's monotonic
    94	    // resize ids; 0 is reserved for the initial stream set) ──────────
    95	    /// Settled live stream count. Epoch-0 write is
    96	    /// `set_negotiated_streams`; later writes come from
    97	    /// `resize_settled` on an accepted epoch.
    98	    live_streams: AtomicUsize,
    99	    /// Last settled epoch (0 until the first accepted resize).
   100	    resize_epoch: AtomicU32,
   101	    /// In-flight proposal's epoch; 0 = none. While non-zero no new
   102	    /// proposal is produced (the wire is idempotent but overlapping
   103	    /// epochs would complicate sub-token registration).
   104	    pending_epoch: AtomicU32,
   105	    /// Resize-eligible ticks since the last settle (cooldown clock).
   106	    ticks_since_settle: AtomicU32,
   107	    /// Consecutive same-direction tick counter: positive = "pipe clean
   108	    /// AND cheap dials maxed" streak, negative = "blocked AND cheap
   109	    /// dials floored" streak. Any other tick resets it.
   110	    resize_sustain: AtomicI32,
   111	    // Profile-clamped bounds, fixed at construction.
   112	    ceiling_chunk_bytes: usize,
   113	    ceiling_prefetch: usize,
   114	    ceiling_max_streams: usize,
   115	    ceiling_tcp_buffer_bytes: usize,
   116	}
   117	
   118	/// One engine resize decision (`ue-r2-2`). The adapter that owns the
   119	/// control stream turns this into a wire `DataPlaneResize` (the engine
   120	/// stays wire-type-free here on purpose) and MUST eventually call
   121	/// [`TransferDial::resize_settled`] for the epoch — with what actually
   122	/// happened — or no further proposals are produced.
   123	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
   124	pub struct ResizeProposal {
   125	    /// The wire epoch for this change (`resize_epoch() + 1`).
   126	    pub epoch: u32,
   127	    /// Absolute desired live count (idempotent, per the proto).
   128	    pub target_streams: usize,
   129	    /// Convenience: `target_streams > live` at proposal time.
   130	    pub add: bool,
   131	}
   132	
   133	impl TransferDial {
   134	    /// Conservative start with default ceilings (no receiver profile).
   135	    pub fn conservative() -> Self {
   136	        Self::conservative_within(None)
   137	    }
   138	
   139	    /// Conservative start bounded by the receiver's advertised
   140	    /// capacity profile. Per the `ue-r2-1b` contract, `0`/absent
   141	    /// fields mean UNKNOWN and keep the (already conservative)
   142	    /// default ceiling — never "unlimited". A profile can only lower
   143	    /// ceilings, never raise them above the defaults this slice.
   144	    pub fn conservative_within(profile: Option<&CapacityProfile>) -> Self {
   145	        let mut ceiling_chunk = DIAL_CEILING_CHUNK_BYTES;
   146	        let mut ceiling_prefetch = DIAL_CEILING_PREFETCH;
   147	        let mut ceiling_streams = DIAL_CEILING_MAX_STREAMS;
   148	        let ceiling_tcp = DIAL_CEILING_TCP_BUFFER_BYTES;
   149	        if let Some(profile) = profile {
   150	            if profile.max_chunk_bytes > 0 {
   151	                ceiling_chunk = ceiling_chunk.min(profile.max_chunk_bytes as usize);
   152	            }
   153	            if profile.max_streams > 0 {
   154	                ceiling_streams = ceiling_streams.min(profile.max_streams as usize);
   155	            }
   156	            if profile.max_inflight_bytes > 0 {
   157	                // The in-flight budget bounds the CHUNK ceiling first
   158	                // (codex ue-r2-1e F1: with max_chunk unknown, a budget
   159	                // smaller than one chunk must still be honored — floor
   160	                // 64 KiB, matching the session's minimum buffer), then
   161	                // prefetch so prefetch × chunk stays within budget
   162	                // (floor of 1 so work still moves).
   163	                let inflight = profile.max_inflight_bytes as usize;
   164	                ceiling_chunk =
   165	                    ceiling_chunk.min(inflight.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR));
   166	                let by_inflight = (inflight / ceiling_chunk.max(1)).max(1);
   167	                ceiling_prefetch = ceiling_prefetch.min(by_inflight);
   168	            }
   169	        }
   170	        Self {
   171	            chunk_bytes: AtomicUsize::new(DIAL_FLOOR_CHUNK_BYTES.min(ceiling_chunk)),
   172	            prefetch_count: AtomicUsize::new(DIAL_FLOOR_PREFETCH.min(ceiling_prefetch)),
   173	            tcp_buffer_bytes: AtomicUsize::new(0),
   174	            initial_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
   175	            max_streams: AtomicUsize::new(DIAL_FLOOR_MAX_STREAMS.clamp(1, ceiling_streams.max(1))),
   176	            live_streams: AtomicUsize::new(DIAL_FLOOR_INITIAL_STREAMS.min(ceiling_streams)),
   177	            resize_epoch: AtomicU32::new(0),
   178	            pending_epoch: AtomicU32::new(0),
   179	            ticks_since_settle: AtomicU32::new(0),
   180	            resize_sustain: AtomicI32::new(0),
   181	            ceiling_chunk_bytes: ceiling_chunk,
   182	            ceiling_prefetch,
   183	            ceiling_max_streams: ceiling_streams,
   184	            ceiling_tcp_buffer_bytes: ceiling_tcp,
   185	        }
   186	    }
   187	
   188	    pub fn shared(self) -> Arc<Self> {
   189	        Arc::new(self)
   190	    }
   191	
   192	    // ── live reads ───────────────────────────────────────────────────
   193	    pub fn chunk_bytes(&self) -> usize {
   194	        self.chunk_bytes.load(Ordering::Relaxed)
   195	    }
   196	    pub fn prefetch_count(&self) -> usize {
   197	        self.prefetch_count.load(Ordering::Relaxed)
   198	    }
   199	    /// `None` = leave the kernel default (old `tcp_buffer_size`
   200	    /// semantics). Connect-time dial.
   201	    pub fn tcp_buffer_bytes(&self) -> Option<usize> {
   202	        match self.tcp_buffer_bytes.load(Ordering::Relaxed) {
   203	            0 => None,
   204	            n => Some(n),
   205	        }
   206	    }
   207	    pub fn initial_streams(&self) -> usize {
   208	        self.initial_streams.load(Ordering::Relaxed)
   209	    }
   210	    /// Ceiling on the negotiated stream count (profile-clamped).
   211	    pub fn max_streams(&self) -> usize {
   212	        self.max_streams.load(Ordering::Relaxed)
   213	    }
   214	    pub fn ceiling_max_streams(&self) -> usize {
   215	        self.ceiling_max_streams
   216	    }
   217	
   218	    /// Record the stream count the negotiation actually settled on
   219	    /// (clamped to the dial's ceiling). This is the epoch-0 settle:
   220	    /// it also seeds `live_streams`, the baseline every `ue-r2-2`
   221	    /// resize proposal steps from.
   222	    pub fn set_negotiated_streams(&self, streams: usize) -> usize {
   223	        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
   224	        self.initial_streams.store(clamped, Ordering::Relaxed);
   225	        self.live_streams.store(clamped, Ordering::Relaxed);
   226	        clamped
   227	    }
   228	
   229	    // ── ue-r2-2 resize policy ────────────────────────────────────────
   230	
   231	    /// The settled live stream count (epoch-0 negotiation, then each
   232	    /// accepted resize).
   233	    pub fn live_streams(&self) -> usize {
   234	        self.live_streams.load(Ordering::Relaxed)
   235	    }
   236	
   237	    /// Last settled resize epoch (0 = only the initial stream set).
   238	    pub fn resize_epoch(&self) -> u32 {
   239	        self.resize_epoch.load(Ordering::Relaxed)
   240	    }
   241	
   242	    /// True while a proposal is awaiting `resize_settled`.
   243	    pub fn resize_pending(&self) -> bool {
   244	        self.pending_epoch.load(Ordering::Relaxed) != 0
   245	    }
   246	
   247	    fn cheap_dials_maxed(&self) -> bool {
   248	        self.chunk_bytes.load(Ordering::Relaxed) >= self.ceiling_chunk_bytes
   249	            && self.prefetch_count.load(Ordering::Relaxed) >= self.ceiling_prefetch
   250	    }
   251	
   252	    fn cheap_dials_floored(&self) -> bool {
   253	        self.chunk_bytes.load(Ordering::Relaxed)
   254	            <= DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes)
   255	            && self.prefetch_count.load(Ordering::Relaxed)
   256	                <= DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch).max(1)
   257	    }
   258	
   259	    /// One resize-eligible tuner tick. Streams move only as the LAST
   260	    /// escalation step in either direction: the cheap dials must
   261	    /// already be pinned at their ceiling (ADD) or floor (REMOVE), the
   262	    /// signal must hold for [`RESIZE_SUSTAIN_TICKS`] consecutive
   263	    /// ticks, at least [`RESIZE_COOLDOWN_TICKS`] must have passed
   264	    /// since the last settle, and no proposal may be in flight. Idle
   265	    /// ticks (`delta_bytes == 0`) are no signal, matching the cheap
   266	    /// tuner. Bounds: `1..=ceiling_max_streams` (the receiver profile
   267	    /// folded in at construction — `CapacityProfile.max_streams` is
   268	    /// authoritative per the proto). One stream per epoch.
   269	    ///
   270	    /// The caller must forward the returned proposal to the peer and
   271	    /// call [`Self::resize_settled`] with the outcome; until then
   272	    /// every subsequent tick returns `None`.
   273	    pub fn resize_tick(&self, delta_bytes: u64, blocked_ratio: f64) -> Option<ResizeProposal> {
   274	        if self.pending_epoch.load(Ordering::Relaxed) != 0 {
   275	            return None;
   276	        }
   277	        let ticks = self
   278	            .ticks_since_settle
   279	            .fetch_add(1, Ordering::Relaxed)
   280	            .saturating_add(1);
   281	        if delta_bytes == 0 {
   282	            self.resize_sustain.store(0, Ordering::Relaxed);
   283	            return None;
   284	        }
   285	        let live = self.live_streams.load(Ordering::Relaxed).max(1);
   286	        let sustain = if blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO && self.cheap_dials_maxed() {
   287	            let prev = self.resize_sustain.load(Ordering::Relaxed).max(0);
   288	            let next = prev.saturating_add(1);
   289	            self.resize_sustain.store(next, Ordering::Relaxed);
   290	            next
   291	        } else if blocked_ratio > DIAL_STEP_DOWN_BLOCKED_RATIO && self.cheap_dials_floored() {
   292	            let prev = self.resize_sustain.load(Ordering::Relaxed).min(0);
   293	            let next = prev.saturating_sub(1);
   294	            self.resize_sustain.store(next, Ordering::Relaxed);
   295	            next
   296	        } else {
   297	            self.resize_sustain.store(0, Ordering::Relaxed);
   298	            0
   299	        };
   300	        if ticks < RESIZE_COOLDOWN_TICKS {
   301	            return None;
   302	        }
   303	        let target = if sustain >= RESIZE_SUSTAIN_TICKS {
   304	            (live + 1).min(self.ceiling_max_streams.max(1))
   305	        } else if sustain <= -RESIZE_SUSTAIN_TICKS {
   306	            live.saturating_sub(1).max(1)
   307	        } else {
   308	            return None;
   309	        };
   310	        if target == live {
   311	            // Already at the bound in the wanted direction.
   312	            self.resize_sustain.store(0, Ordering::Relaxed);
   313	            return None;
   314	        }
   315	        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
   316	        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
   317	        // another task, and a plain store here could stack two live
   318	        // proposals onto one epoch number.
   319	        if self
   320	            .pending_epoch
   321	            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
   322	            .is_err()
   323	        {
   324	            return None;
   325	        }
   326	        self.resize_sustain.store(0, Ordering::Relaxed);
   327	        Some(ResizeProposal {
   328	            epoch,
   329	            target_streams: target,
   330	            add: target > live,
   331	        })
   332	    }
   333	
   334	    /// sf-2: shape-correction proposal. On push the daemon proposes the
   335	    /// epoch-0 stream count from whatever manifest prefix it has seen at
   336	    /// the early flush (`FILE_LIST_EARLY_FLUSH_ENTRIES`), so a
   337	    /// many-tiny-file push can negotiate far fewer streams than
   338	    /// [`initial_stream_proposal`] assigns the full workload. As the
   339	    /// need list accumulates client-side, the client re-runs the shape
   340	    /// table and corrects upward through the normal resize wire.
   341	    ///
   342	    /// Unlike [`Self::resize_tick`] this is a definite signal — the
   343	    /// shape is known, not inferred from throughput — so there is no
   344	    /// sustain/cooldown discipline. It still honors one-in-flight and
   345	    /// the receiver-profile ceiling, still moves ONE stream per epoch
   346	    /// (the wire carries one `sub_token` per ADD), and never proposes
   347	    /// REMOVE: shrinking below a live count is throughput evidence and
   348	    /// stays the tuner's call.
   349	    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
   350	        let desired = desired_streams.clamp(1, self.ceiling_max_streams.max(1));
   351	        let live = self.live_streams.load(Ordering::Relaxed).max(1);
   352	        if desired <= live {
   353	            return None;
   354	        }
   355	        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
   356	        if self
   357	            .pending_epoch
   358	            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
   359	            .is_err()
   360	        {
   361	            return None;
   362	        }
   363	        Some(ResizeProposal {
   364	            epoch,
   365	            target_streams: live + 1,
   366	            add: true,
   367	        })
   368	    }
   369	
   370	    /// Settle the in-flight proposal with what ACTUALLY happened:
   371	    /// `effective_streams` is the live count now in effect (from the
   372	    /// peer's ack, or the local count if a post-ack dial failed and
   373	    /// nothing changed). `accepted = false` leaves the live count
   374	    /// untouched. Stale epochs (not the pending one) are ignored.
   375	    /// Either way the cooldown clock restarts.
   376	    pub fn resize_settled(&self, epoch: u32, effective_streams: usize, accepted: bool) {
   377	        if self.pending_epoch.load(Ordering::Relaxed) != epoch || epoch == 0 {
   378	            return;
   379	        }
   380	        self.pending_epoch.store(0, Ordering::Relaxed);
   381	        self.ticks_since_settle.store(0, Ordering::Relaxed);
   382	        self.resize_sustain.store(0, Ordering::Relaxed);
   383	        if accepted {
   384	            let clamped = effective_streams.clamp(1, self.ceiling_max_streams.max(1));
   385	            self.live_streams.store(clamped, Ordering::Relaxed);
   386	            self.resize_epoch.store(epoch, Ordering::Relaxed);
   387	        }
   388	    }
   389	
   390	    /// Raise max_streams toward the ceiling (used when a peer's
   391	    /// negotiation allows more than the floor; still profile-bounded).
   392	    pub fn allow_streams_up_to(&self, streams: usize) {
   393	        let clamped = streams.clamp(1, self.ceiling_max_streams.max(1));
   394	        self.max_streams.store(clamped, Ordering::Relaxed);
   395	    }
   396	
   397	    // ── tuner steps ──────────────────────────────────────────────────
   398	    /// One upward step of the cheap dials: chunk ×2 toward the
   399	    /// ceiling, prefetch +50% (at least +1) toward the ceiling, and
   400	    /// the tcp buffer to its ceiling (affects future sockets).
   401	    /// Returns true if anything moved.
   402	    pub fn step_up_cheap_dials(&self) -> bool {
   403	        let mut moved = false;
   404	        let chunk = self.chunk_bytes.load(Ordering::Relaxed);
   405	        let next = (chunk.saturating_mul(2)).min(self.ceiling_chunk_bytes);
   406	        if next > chunk {
   407	            self.chunk_bytes.store(next, Ordering::Relaxed);
   408	            moved = true;
   409	        }
   410	        let prefetch = self.prefetch_count.load(Ordering::Relaxed);
   411	        let next = (prefetch + (prefetch / 2).max(1)).min(self.ceiling_prefetch);
   412	        if next > prefetch {
   413	            self.prefetch_count.store(next, Ordering::Relaxed);
   414	            moved = true;
   415	        }
   416	        let tcp = self.tcp_buffer_bytes.load(Ordering::Relaxed);
   417	        if tcp < self.ceiling_tcp_buffer_bytes {
   418	            self.tcp_buffer_bytes
   419	                .store(self.ceiling_tcp_buffer_bytes, Ordering::Relaxed);
   420	            moved = true;
   421	        }
   422	        moved
   423	    }
   424	
   425	    /// One downward step toward the floors. Returns true if anything
   426	    /// moved.
   427	    pub fn step_down_cheap_dials(&self) -> bool {
   428	        let mut moved = false;
   429	        let chunk = self.chunk_bytes.load(Ordering::Relaxed);
   430	        let next = (chunk / 2).max(DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes));

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/push/client/mod.rs | sed -n '430,560p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   430	
   431	    /// Close the payload channel and wait for the pipeline to drain.
   432	    async fn finish(mut self) -> Result<()> {
   433	        // ue-r2-1e: stop the tuner promptly (it would otherwise idle
   434	        // until its Weak<dial> dies at the end of the push).
   435	        if let Some(tuner) = self.tuner_handle.take() {
   436	            tuner.abort();
   437	        }
   438	        // Drop the sender so the pipeline sees end-of-stream.
   439	        drop(self.payload_tx.take());
   440	        let handle = self
   441	            .pipeline_handle
   442	            .take()
   443	            .ok_or_else(|| eyre!("data plane pipeline handle missing"))?;
   444	        // Route both Ok and Err through the shared drain helper so
   445	        // the failure-path wrapping ("data plane pipeline failed:
   446	        // <cause>" / "data plane pipeline panicked: <join>") matches
   447	        // exactly what `queue()` would produce. R43 follow-up to
   448	        // R42-F2 — earlier this was a hand-rolled match that
   449	        // duplicated the helper's arms.
   450	        let outcome = drain_pipeline_outcome(handle).await?;
   451	        let elapsed = self.started.elapsed().as_secs_f64().max(1e-6);
   452	        let throughput = (outcome.bytes_written as f64 * 8.0) / elapsed / 1e9;
   453	        eprintln!(
   454	            "[data-plane-client] aggregate {:.2} Gbps ({:.2} MiB in {:.2}s)",
   455	            throughput.max(0.0),
   456	            outcome.bytes_written as f64 / 1024.0 / 1024.0,
   457	            elapsed
   458	        );
   459	        Ok(())
   460	    }
   461	}
   462	
   463	/// ue-r2-1e: one dial per push, created at first need. Replaces the
   464	/// memoized size-keyed `determine_remote_tuning` ladder: conservative
   465	/// start, ceilings bounded by the daemon's advertised receiver profile
   466	/// when the negotiation carried one (first-wins, like the old memo).
   467	fn ensure_dial(
   468	    dial: &mut Option<Arc<crate::engine::TransferDial>>,
   469	    receiver_capacity: Option<&crate::generated::CapacityProfile>,
   470	) -> Arc<crate::engine::TransferDial> {
   471	    if dial.is_none() {
   472	        *dial = Some(crate::engine::TransferDial::conservative_within(receiver_capacity).shared());
   473	    }
   474	    dial.as_ref()
   475	        .cloned()
   476	        .expect("dial set by preceding assignment")
   477	}
   478	
   479	/// ue-r2-2 / sf-2 shared pre-dial ADD: mint the epoch credential, send
   480	/// the `DataPlaneResize` ADD, and record the in-flight epoch (the
   481	/// socket itself is dialed on the daemon's ack). A missing credential
   482	/// source settles the epoch failed and is not an error; a send error
   483	/// is returned for the caller to route through `prefer_server_error`.
   484	async fn send_resize_add(
   485	    tx: &mpsc::Sender<ClientPushRequest>,
   486	    dial: &crate::engine::TransferDial,
   487	    proposal: crate::engine::ResizeProposal,
   488	    resize_pending: &mut Option<PendingResize>,
   489	) -> Result<()> {
   490	    match crate::remote::transfer::generate_sub_token() {
   491	        Ok(sub) => {
   492	            send_payload(
   493	                tx,
   494	                ClientPayload::DataPlaneResize(DataPlaneResize {
   495	                    op: DataPlaneResizeOp::Add as i32,
   496	                    epoch: proposal.epoch,
   497	                    target_stream_count: proposal.target_streams as u32,
   498	                    sub_token: sub.clone(),
   499	                }),
   500	            )
   501	            .await?;
   502	            *resize_pending = Some(PendingResize {
   503	                epoch: proposal.epoch,
   504	                target: proposal.target_streams,
   505	                add: true,
   506	                sub_token: sub,
   507	            });
   508	        }
   509	        Err(err) => {
   510	            log::warn!("resize ADD skipped (no credential source): {err:#}");
   511	            dial.resize_settled(proposal.epoch, dial.live_streams(), false);
   512	        }
   513	    }
   514	    Ok(())
   515	}
   516	
   517	/// sf-2: one shape-correction step. The daemon proposes the epoch-0
   518	/// stream count from whatever manifest prefix it had seen at its early
   519	/// flush, so a many-tiny-file push can negotiate far fewer streams
   520	/// than the shape table assigns the full workload
   521	/// (`.review/findings/sf-1-tripwire-harness.md` Known gaps: a
   522	/// 1000-file push measured 1 stream where the table says 2). As the
   523	/// need list accumulates, re-run [`crate::engine::initial_stream_proposal`]
   524	/// over the ACTUAL transfer shape (need-list files + bytes, not the
   525	/// manifest — an incremental push of a large tree may move only a few
   526	/// files) and correct upward one ADD epoch at a time. Call sites gate
   527	/// on the transfer running resize-enabled on the data plane.
   528	async fn maybe_shape_resize(
   529	    tx: &mpsc::Sender<ClientPushRequest>,
   530	    dial: &crate::engine::TransferDial,
   531	    need_bytes: u64,
   532	    need_count: usize,
   533	    resize_pending: &mut Option<PendingResize>,
   534	) -> Result<()> {
   535	    if resize_pending.is_some() {
   536	        return Ok(());
   537	    }
   538	    let target =
   539	        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())
   540	            as usize;
   541	    match dial.propose_shape_resize(target) {
   542	        Some(proposal) => send_resize_add(tx, dial, proposal, resize_pending).await,
   543	        None => Ok(()),
   544	    }
   545	}
   546	
   547	fn prune_unrequested_payloads(
   548	    payloads: &mut Vec<TransferPayload>,
   549	    requested: &mut HashSet<String>,
   550	) -> usize {
   551	    let mut filtered: Vec<TransferPayload> = Vec::with_capacity(payloads.len());
   552	    let mut skipped = 0usize;
   553	
   554	    for payload in payloads.drain(..) {
   555	        match payload {
   556	            TransferPayload::File(header) => {
   557	                if requested.remove(header.relative_path.as_str()) {
   558	                    filtered.push(TransferPayload::File(header));
   559	                } else {
   560	                    skipped += 1;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/push/client/mod.rs | sed -n '700,1090p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   700	
   701	        let (manifest_rx, manifest_task) = source.scan(
   702	            Some(filter.clone_without_cache()),
   703	            Arc::clone(&unreadable_paths),
   704	        );
   705	
   706	        let mut manifest_rx = manifest_rx;
   707	
   708	        let mut files_requested: Vec<String> = Vec::new();
   709	        let mut pending_queue: VecDeque<String> = VecDeque::new();
   710	        let mut fallback_upload_complete_sent = false;
   711	        let mut fallback_files_sent: usize = 0;
   712	        let mut need_list_received = false;
   713	        let mut data_plane_sender: Option<MultiStreamSender> = None;
   714	        let mut data_plane_outstanding: usize = 0;
   715	        let mut data_plane_files_sent: usize = 0;
   716	        let mut data_port: Option<u32> = None;
   717	        let mut fallback_used = force_grpc;
   718	        let mut summary: Option<PushSummary> = None;
   719	
   720	        let mut transfer_mode = if force_grpc {
   721	            TransferMode::Fallback
   722	        } else {
   723	            TransferMode::Undecided
   724	        };
   725	        // design-4: the daemon's wire contract rejects FileData while its
   726	        // manifest loop is still running ("data payload received before
   727	        // negotiation"). Even in forced-gRPC mode the client must therefore
   728	        // hold its fallback payloads until the daemon announces
   729	        // Negotiation(tcp_fallback) — which the daemon only sends after it
   730	        // has seen ManifestComplete. Pre-fix, force_grpc initialized
   731	        // Fallback mode and the first mid-manifest need-list batch
   732	        // triggered FileData sends that raced the daemon's manifest loop:
   733	        // every forced-gRPC push of ≥128 files (one early need-list flush)
   734	        // died, and ~100 files was a coin flip.
   735	        let mut fallback_negotiated = false;
   736	
   737	        // ue-r2-2: resize controller state. The tuner's proposal stream
   738	        // appears once a resize-enabled negotiation lands;
   739	        // `resize_pending` is the single epoch awaiting the daemon's
   740	        // ack (the dial enforces one-in-flight too).
   741	        let mut resize_proposal_rx: Option<
   742	            tokio::sync::mpsc::UnboundedReceiver<crate::engine::ResizeProposal>,
   743	        > = None;
   744	        let mut resize_pending: Option<PendingResize> = None;
   745	        // sf-2: shape-correction gate. `resize_negotiated` records that
   746	        // this transfer's data plane went elastic (epoch-0 sub-token
   747	        // present). `shape_resize_enabled` flips off permanently the
   748	        // first time the tuner proposes a REMOVE — live throughput
   749	        // evidence outranks the static shape table, and re-adding what
   750	        // the tuner just retired would flap.
   751	        let mut resize_negotiated = false;
   752	        let mut shape_resize_enabled = true;
   753	
   754	        let mut manifest_done = false;
   755	        // Track whether we received new need-list entries this iteration.
   756	        // Don't finish the data plane until a full iteration passes with
   757	        // no new entries — this ensures all in-flight gRPC batches arrive.
   758	        let mut need_list_fresh: bool;
   759	        // Set when the daemon signals "no more need_lists coming" by
   760	        // sending an empty FilesToUpload terminator. Gates the early
   761	        // finish() so we don't close the data plane while the daemon
   762	        // is still streaming need_list batches.
   763	        let mut need_lists_done = false;
   764	        loop {
   765	            if manifest_done && summary.is_some() {
   766	                break;
   767	            }
   768	            need_list_fresh = false;
   769	
   770	            tokio::select! {
   771	                biased;
   772	
   773	                maybe_message = response_rx.recv() => {
   774	                    match maybe_message {
   775	                        Some(Ok(message)) => {
   776	                            match message.payload {
   777	                                Some(ServerPayload::Ack(_)) => {}
   778	                                Some(ServerPayload::FilesToUpload(list)) => {
   779	                                    if list.relative_paths.is_empty() {
   780	                                        // Empty terminator — no more need_lists coming.
   781	                                        // Fall through to the bottom of the loop so the
   782	                                        // early-finish check can fire on this iteration;
   783	                                        // don't `continue` (that would skip the check
   784	                                        // and require another response message to wake
   785	                                        // the select, which never arrives).
   786	                                        need_lists_done = true;
   787	                                    } else {
   788	                                    need_list_fresh = true;
   789	                                    let mut rels = list.relative_paths;
   790	                                    files_requested.extend(rels.iter().cloned());
   791	                                    let newly_requested = rels.len();
   792	                                    let mut batch_bytes = 0u64;
   793	                                    for rel in &rels {
   794	                                        requested_files.insert(rel.clone());
   795	                                        if let Some(header) = manifest_lookup.get(rel) {
   796	                                            batch_bytes =
   797	                                                batch_bytes.saturating_add(header.size);
   798	                                        }
   799	                                        // w5-1: was an unconditional per-file
   800	                                        // eprintln — stderr spam proportional
   801	                                        // to file count. Debug-level now;
   802	                                        // visible with BLIT_LOG=debug.
   803	                                        log::debug!("push need-list includes {}", rel);
   804	                                    }
   805	                                    pending_queue.extend(rels.drain(..));
   806	                                    transfer_size_hint =
   807	                                        transfer_size_hint.saturating_add(batch_bytes);
   808	                                    need_list_received = true;
   809	
   810	                                    if !matches!(transfer_mode, TransferMode::Fallback) {
   811	                                        data_plane_outstanding =
   812	                                            data_plane_outstanding.saturating_add(newly_requested);
   813	                                    }
   814	
   815	                                    if let Some(progress) = progress {
   816	                                        if newly_requested > 0 {
   817	                                            progress.report_manifest_batch(newly_requested);
   818	                                        }
   819	                                    }
   820	
   821	                                    match transfer_mode {
   822	                                        TransferMode::Fallback => {
   823	                                            // design-4: hold payloads until the
   824	                                            // daemon's fallback negotiation;
   825	                                            // until then entries just accumulate
   826	                                            // in pending_queue (drained by the
   827	                                            // Negotiation arm).
   828	                                            if fallback_negotiated && need_list_received {
   829	                                                let dial = ensure_dial(
   830	                                                    &mut dial,
   831	                                                    None,
   832	                                                );
   833	                                                let result = stream_fallback_from_queue(
   834	                                                    source.clone(),
   835	                                                    &mut pending_queue,
   836	                                                    &manifest_lookup,
   837	                                                    &tx,
   838	                                                    progress,
   839	                                                    plan_options,
   840	                                                    dial.chunk_bytes(),
   841	                                                    dial.initial_streams(),
   842	                                                    &unreadable_paths,
   843	                                                ).await?;
   844	                                                if result.files_sent > 0 {
   845	                                                    fallback_files_sent =
   846	                                                        fallback_files_sent.saturating_add(result.files_sent);
   847	                                                }
   848	                                                if result.payloads_dispatched
   849	                                                    && first_payload_elapsed.is_none()
   850	                                                {
   851	                                                    first_payload_elapsed = Some(start.elapsed());
   852	                                                }
   853	                                            }
   854	                                        }
   855	                                        TransferMode::DataPlane => {
   856	                                            // sf-2: the need list just grew —
   857	                                            // re-run the shape table and
   858	                                            // correct the stream count before
   859	                                            // queueing the batch.
   860	                                            if resize_negotiated
   861	                                                && shape_resize_enabled
   862	                                                && data_plane_sender.is_some()
   863	                                            {
   864	                                                if let Some(dial_ref) = dial.as_ref() {
   865	                                                    if let Err(send_err) = maybe_shape_resize(
   866	                                                        &tx,
   867	                                                        dial_ref,
   868	                                                        transfer_size_hint,
   869	                                                        requested_files.len(),
   870	                                                        &mut resize_pending,
   871	                                                    )
   872	                                                    .await
   873	                                                    {
   874	                                                        return Err(prefer_server_error(
   875	                                                            &mut response_rx,
   876	                                                            send_err,
   877	                                                        )
   878	                                                        .await);
   879	                                                    }
   880	                                                }
   881	                                            }
   882	                                            if let Some(sender) = data_plane_sender.as_mut() {
   883	                                                let headers =
   884	                                                    drain_pending_headers(&mut pending_queue, &manifest_lookup);
   885	                                                if !headers.is_empty() {
   886	                                                    let headers = source.check_availability(
   887	                                                        headers,
   888	                                                        Arc::clone(&unreadable_paths),
   889	                                                    )
   890	                                                    .await?;
   891	                                                    if headers.is_empty() {
   892	                                                        continue;
   893	                                                    }
   894	                                                    // Dial exists before the first
   895	                                                    // data-plane batch (first-wins).
   896	                                                    ensure_dial(&mut dial, None);
   897	                                            let planned =
   898	                                                plan_transfer_payloads(headers, source_root, plan_options)?;
   899	                                            for payload in &planned {
   900	                                                match payload {
   901	                                                    TransferPayload::File(header) => {
   902	                                                        // w5-1: was unconditional per-file
   903	                                                        // eprintln; BLIT_LOG=debug shows it.
   904	                                                        log::debug!(
   905	                                                            "push enqueue {} for TCP stream",
   906	                                                            header.relative_path
   907	                                                        );
   908	                                                    }
   909	                                                    TransferPayload::TarShard { headers } => {
   910	                                                        for header in headers {
   911	                                                            log::debug!(
   912	                                                                "push enqueue {} via tar shard",
   913	                                                                header.relative_path
   914	                                                            );
   915	                                                        }
   916	                                                    }
   917	                                                    TransferPayload::FileBlock { .. }
   918	                                                    | TransferPayload::FileBlockComplete { .. } => {
   919	                                                        // Receive-only — never produced by the outbound planner.
   920	                                                    }
   921	                                                }
   922	                                            }
   923	                                            if !planned.is_empty() {
   924	                                                        let sent = payload_file_count(&planned);
   925	                                                        sender.queue(planned).await?;
   926	                                                        if sent > 0 && first_payload_elapsed.is_none() {
   927	                                                            first_payload_elapsed = Some(start.elapsed());
   928	                                                        }
   929	                                                        data_plane_files_sent += sent;
   930	                                                        data_plane_outstanding =
   931	                                                            data_plane_outstanding.saturating_sub(sent);
   932	                                                    }
   933	                                                }
   934	                                            }
   935	                                        }
   936	                                        TransferMode::Undecided => {}
   937	                                    }
   938	                                    } // end else (non-empty need_list)
   939	                                }
   940	                                Some(ServerPayload::Negotiation(neg)) => {
   941	                                    if neg.tcp_fallback {
   942	                                        fallback_used = true;
   943	                                        transfer_mode = TransferMode::Fallback;
   944	                                        // design-4: only now may fallback
   945	                                        // payloads flow — the daemon is past
   946	                                        // its manifest loop and ready to
   947	                                        // receive FileData.
   948	                                        fallback_negotiated = true;
   949	
   950	                                            if need_list_received {
   951	                                            let dial = ensure_dial(
   952	                                                &mut dial,
   953	                                                neg.receiver_capacity.as_ref(),
   954	                                            );
   955	                                            let result = stream_fallback_from_queue(
   956	                                                source.clone(),
   957	                                                &mut pending_queue,
   958	                                                &manifest_lookup,
   959	                                                &tx,
   960	                                                progress,
   961	                                                plan_options,
   962	                                                dial.chunk_bytes(),
   963	                                                dial.prefetch_count(),
   964	                                                &unreadable_paths,
   965	                                            ).await?;
   966	                                            if result.files_sent > 0 {
   967	                                                fallback_files_sent =
   968	                                                    fallback_files_sent.saturating_add(result.files_sent);
   969	                                            }
   970	                                            if result.payloads_dispatched
   971	                                                && first_payload_elapsed.is_none()
   972	                                            {
   973	                                                first_payload_elapsed = Some(start.elapsed());
   974	                                            }
   975	                                        }
   976	
   977	                                        data_plane_outstanding = 0;
   978	                                        if let Some(sender) = data_plane_sender.take() {
   979	                                            sender.finish().await?;
   980	                                        }
   981	                                    } else {
   982	                                        if neg.tcp_port == 0 {
   983	                                            eyre::bail!("server reported zero data port for negotiated transfer");
   984	                                        }
   985	
   986	                                        let token_bytes = decode_token(&neg.one_time_token)?;
   987	                                        // ue-r2-1e: the daemon (byte
   988	                                        // receiver) advertised its profile
   989	                                        // on this negotiation — the dial's
   990	                                        // ceilings honor it (first-wins,
   991	                                        // like the old tuning memo).
   992	                                        let dial = ensure_dial(
   993	                                            &mut dial,
   994	                                            neg.receiver_capacity.as_ref(),
   995	                                        );
   996	                                        if data_plane_sender.is_none() {
   997	                                            let stream_target = dial.set_negotiated_streams(
   998	                                                neg.stream_count.max(1) as usize,
   999	                                            );
  1000	                                            let payload_prefetch = dial.prefetch_count();
  1001	                                            // ue-r2-2: the daemon's fold said
  1002	                                            // resize is on for this transfer —
  1003	                                            // epoch-0 sockets carry the
  1004	                                            // sub-token suffix and the sender
  1005	                                            // goes elastic. A malformed token
  1006	                                            // length reads as "not enabled"
  1007	                                            // (fail toward today's behavior).
  1008	                                            let resize_sub = (neg.resize_enabled
  1009	                                                && neg.epoch0_sub_token.len()
  1010	                                                    == crate::remote::transfer::SUB_TOKEN_LEN)
  1011	                                                .then(|| neg.epoch0_sub_token.clone());
  1012	                                            resize_negotiated = resize_sub.is_some();
  1013	                                            let mut sender = MultiStreamSender::connect(
  1014	                                                &self.endpoint.host,
  1015	                                                neg.tcp_port,
  1016	                                                &token_bytes,
  1017	                                                dial.chunk_bytes(),
  1018	                                                payload_prefetch,
  1019	                                                stream_target,
  1020	                                                trace_data_plane,
  1021	                                                source.clone(),
  1022	                                                dial.tcp_buffer_bytes(),
  1023	                                                progress.cloned(),
  1024	                                                Some(dial.clone()),
  1025	                                                resize_sub,
  1026	                                            )
  1027	                                            .await?;
  1028	                                            resize_proposal_rx = sender.take_resize_rx();
  1029	                                            data_plane_sender = Some(sender);
  1030	                                            data_port = Some(neg.tcp_port);
  1031	
  1032	                                            // sf-2: need-list batches can
  1033	                                            // predate the negotiation — the
  1034	                                            // accumulated shape may already
  1035	                                            // outgrow the daemon's
  1036	                                            // partial-manifest stream count.
  1037	                                            if resize_negotiated && shape_resize_enabled {
  1038	                                                if let Err(send_err) = maybe_shape_resize(
  1039	                                                    &tx,
  1040	                                                    &dial,
  1041	                                                    transfer_size_hint,
  1042	                                                    requested_files.len(),
  1043	                                                    &mut resize_pending,
  1044	                                                )
  1045	                                                .await
  1046	                                                {
  1047	                                                    return Err(prefer_server_error(
  1048	                                                        &mut response_rx,
  1049	                                                        send_err,
  1050	                                                    )
  1051	                                                    .await);
  1052	                                                }
  1053	                                            }
  1054	                                        }
  1055	
  1056	                                        if let Some(sender) = data_plane_sender.as_mut() {
  1057	                                            let headers =
  1058	                                                drain_pending_headers(&mut pending_queue, &manifest_lookup);
  1059	                                            if !headers.is_empty() {
  1060	                                                let headers = source
  1061	                                                    .check_availability(headers, unreadable_paths.clone())
  1062	                                                    .await?;
  1063	                                                if headers.is_empty() {
  1064	                                                    continue;
  1065	                                                }
  1066	                                            let mut planned = plan_transfer_payloads(
  1067	                                                headers,
  1068	                                                source_root,
  1069	                                                plan_options,
  1070	                                            )?;
  1071	                                            let skipped = prune_unrequested_payloads(
  1072	                                                &mut planned,
  1073	                                                &mut requested_files,
  1074	                                            );
  1075	                                            if skipped > 0 {
  1076	                                                log::debug!(
  1077	                                                    "push: daemon did not request {} payload file(s); skipping",
  1078	                                                    skipped
  1079	                                                );
  1080	                                            }
  1081	                                            if !planned.is_empty() {
  1082	                                                let sent = payload_file_count(&planned);
  1083	                                                sender.queue(planned).await?;
  1084	                                                if sent > 0 && first_payload_elapsed.is_none() {
  1085	                                                    first_payload_elapsed = Some(start.elapsed());
  1086	                                                }
  1087	                                                data_plane_files_sent += sent;
  1088	                                                data_plane_outstanding =
  1089	                                                    data_plane_outstanding.saturating_sub(sent);
  1090	                                                }

codex
The first likely risk area is after the proposal is sent: whether failed sends or refused epochs fully release the dial and local pending state. I’m tracing that path through the resize ack and tuner branches now.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/push/client/mod.rs | sed -n '1088,1535p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/push/client/types.rs | sed -n '1,80p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use crate::generated::PushSummary;
     2	use crate::remote::transfer::progress::RemoteTransferProgress;
     3	use std::time::Duration;
     4	
     5	#[derive(Debug, Clone)]
     6	pub struct RemotePushReport {
     7	    pub files_requested: Vec<String>,
     8	    pub fallback_used: bool,
     9	    pub data_port: Option<u32>,
    10	    pub summary: PushSummary,
    11	    pub first_payload_elapsed: Option<Duration>,
    12	    /// sf-2: the dial's settled live stream count when the transfer
    13	    /// finished (`None` on the gRPC fallback path — no data plane).
    14	    /// Observable pin for the shape-correction resize: a many-tiny-file
    15	    /// push must end above the 1-stream partial-manifest proposal.
    16	    pub data_plane_streams: Option<usize>,
    17	}
    18	
    19	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    20	pub enum TransferMode {
    21	    Undecided,
    22	    DataPlane,
    23	    Fallback,
    24	}
    25	
    26	pub type RemotePushProgress = RemoteTransferProgress;

 succeeded in 0ms:
  1088	                                                data_plane_outstanding =
  1089	                                                    data_plane_outstanding.saturating_sub(sent);
  1090	                                                }
  1091	                                            }
  1092	                                        }
  1093	                                        transfer_mode = TransferMode::DataPlane;
  1094	                                    }
  1095	                                }
  1096	                                Some(ServerPayload::Summary(push_summary)) => {
  1097	                                    summary = Some(push_summary);
  1098	                                }
  1099	                                Some(ServerPayload::DataPlaneResizeAck(ack)) => {
  1100	                                    // ue-r2-2: settle the in-flight epoch with
  1101	                                    // what actually happened. An unsolicited or
  1102	                                    // stale ack is ignored exactly as before.
  1103	                                    match resize_pending.take() {
  1104	                                        Some(pending) if ack.epoch == pending.epoch => {
  1105	                                            let dial_ref = dial
  1106	                                                .as_ref()
  1107	                                                .expect("resize only negotiated on the dial path");
  1108	                                            if pending.add && ack.accepted {
  1109	                                                // Daemon armed the accept —
  1110	                                                // dial the new socket. A failed
  1111	                                                // dial must NOT kill a healthy
  1112	                                                // transfer: the armed slot
  1113	                                                // expires daemon-side and the
  1114	                                                // live count simply stands.
  1115	                                                let added = match data_plane_sender.as_mut() {
  1116	                                                    Some(sender) => {
  1117	                                                        match sender
  1118	                                                            .add_stream(&pending.sub_token)
  1119	                                                            .await
  1120	                                                        {
  1121	                                                            Ok(()) => true,
  1122	                                                            Err(err) => {
  1123	                                                                log::warn!(
  1124	                                                                    "resize ADD (epoch {}) dial \
  1125	                                                                     failed; continuing at the \
  1126	                                                                     current stream count: {err:#}",
  1127	                                                                    pending.epoch
  1128	                                                                );
  1129	                                                                false
  1130	                                                            }
  1131	                                                        }
  1132	                                                    }
  1133	                                                    None => false,
  1134	                                                };
  1135	                                                if added {
  1136	                                                    dial_ref.resize_settled(
  1137	                                                        pending.epoch,
  1138	                                                        pending.target,
  1139	                                                        true,
  1140	                                                    );
  1141	                                                } else {
  1142	                                                    dial_ref.resize_settled(
  1143	                                                        pending.epoch,
  1144	                                                        dial_ref.live_streams(),
  1145	                                                        true,
  1146	                                                    );
  1147	                                                }
  1148	                                            } else if !pending.add && ack.accepted {
  1149	                                                dial_ref.resize_settled(
  1150	                                                    pending.epoch,
  1151	                                                    pending.target,
  1152	                                                    true,
  1153	                                                );
  1154	                                            } else {
  1155	                                                dial_ref.resize_settled(
  1156	                                                    pending.epoch,
  1157	                                                    dial_ref.live_streams(),
  1158	                                                    false,
  1159	                                                );
  1160	                                            }
  1161	                                            // sf-2: the epoch settled — if the
  1162	                                            // need-list shape still wants more
  1163	                                            // streams, propose the next single
  1164	                                            // ADD (the ramp is one stream per
  1165	                                            // acked epoch).
  1166	                                            if resize_negotiated
  1167	                                                && shape_resize_enabled
  1168	                                                && data_plane_sender.is_some()
  1169	                                            {
  1170	                                                let dial_ref = dial_ref.clone();
  1171	                                                if let Err(send_err) = maybe_shape_resize(
  1172	                                                    &tx,
  1173	                                                    &dial_ref,
  1174	                                                    transfer_size_hint,
  1175	                                                    requested_files.len(),
  1176	                                                    &mut resize_pending,
  1177	                                                )
  1178	                                                .await
  1179	                                                {
  1180	                                                    return Err(prefer_server_error(
  1181	                                                        &mut response_rx,
  1182	                                                        send_err,
  1183	                                                    )
  1184	                                                    .await);
  1185	                                                }
  1186	                                            }
  1187	                                        }
  1188	                                        other => {
  1189	                                            resize_pending = other;
  1190	                                            log::debug!(
  1191	                                                "ignoring unsolicited/stale DataPlaneResizeAck \
  1192	                                                 (epoch {})",
  1193	                                                ack.epoch
  1194	                                            );
  1195	                                        }
  1196	                                    }
  1197	                                }
  1198	                                None => {}
  1199	                            }
  1200	                        }
  1201	                        Some(Err(err)) => return Err(err),
  1202	                        None => break,
  1203	                    }
  1204	                }
  1205	                maybe_header = manifest_rx.recv(), if !manifest_done => {
  1206	                    match maybe_header {
  1207	                        Some(header) => {
  1208	                            // Normalize path to ensure consistency with server requests
  1209	                            let rel = if header.relative_path.starts_with("./") {
  1210	                                header.relative_path[2..].to_string()
  1211	                            } else {
  1212	                                header.relative_path.clone()
  1213	                            };
  1214	                            let mut header = header;
  1215	                            header.relative_path = rel.clone();
  1216	
  1217	                            // Check availability via the source abstraction
  1218	                            let available = source.check_availability(vec![header.clone()], Arc::clone(&unreadable_paths)).await?;
  1219	                            if available.is_empty() {
  1220	                                continue;
  1221	                            }
  1222	
  1223	                            manifest_total_bytes =
  1224	                                manifest_total_bytes.saturating_add(header.size);
  1225	                            // design-5: if the daemon already rejected the
  1226	                            // push (e.g. read-only module), this send loses
  1227	                            // a race with the terminal status — surface the
  1228	                            // daemon's reason, not the transport symptom.
  1229	                            if let Err(send_err) =
  1230	                                send_payload(&tx, ClientPayload::FileManifest(header.clone()))
  1231	                                    .await
  1232	                            {
  1233	                                return Err(
  1234	                                    prefer_server_error(&mut response_rx, send_err).await
  1235	                                );
  1236	                            }
  1237	                            manifest_lookup.insert(rel.clone(), header);
  1238	
  1239	                            match transfer_mode {
  1240	                                TransferMode::Fallback => {
  1241	                                    // design-4: never interleave FileData
  1242	                                    // between our own manifest sends — wait
  1243	                                    // for the daemon's fallback negotiation.
  1244	                                    if fallback_negotiated && need_list_received {
  1245	                                        let dial = ensure_dial(
  1246	                                            &mut dial,
  1247	                                            None,
  1248	                                        );
  1249	                                        let result = stream_fallback_from_queue(
  1250	                                            source.clone(),
  1251	                                            &mut pending_queue,
  1252	                                            &manifest_lookup,
  1253	                                            &tx,
  1254	                                            progress,
  1255	                                            plan_options,
  1256	                                            dial.chunk_bytes(),
  1257	                                            dial.initial_streams(),
  1258	                                            &unreadable_paths,
  1259	                                        ).await?;
  1260	                                        if result.files_sent > 0 {
  1261	                                            fallback_files_sent =
  1262	                                                fallback_files_sent.saturating_add(result.files_sent);
  1263	                                        }
  1264	                                        if result.payloads_dispatched
  1265	                                            && first_payload_elapsed.is_none()
  1266	                                        {
  1267	                                            first_payload_elapsed = Some(start.elapsed());
  1268	                                        }
  1269	                                    }
  1270	                                }
  1271	                                TransferMode::DataPlane => {
  1272	                                    if let Some(sender) = data_plane_sender.as_mut() {
  1273	                                        let headers =
  1274	                                            drain_pending_headers(&mut pending_queue, &manifest_lookup);
  1275	                                        if !headers.is_empty() {
  1276	                                            let headers = source.check_availability(
  1277	                                                headers,
  1278	                                                Arc::clone(&unreadable_paths),
  1279	                                            )
  1280	                                            .await?;
  1281	                                            if headers.is_empty() {
  1282	                                                continue;
  1283	                                            }
  1284	                                            // Dial exists before the first
  1285	                                            // data-plane batch (first-wins).
  1286	                                            ensure_dial(&mut dial, None);
  1287	                                            let mut planned =
  1288	                                                plan_transfer_payloads(headers, source_root, plan_options)?;
  1289	                                            let skipped = prune_unrequested_payloads(
  1290	                                                &mut planned,
  1291	                                                &mut requested_files,
  1292	                                            );
  1293	                                            if skipped > 0 {
  1294	                                                log::debug!(
  1295	                                                    "push: daemon did not request {} payload file(s); skipping",
  1296	                                                    skipped
  1297	                                                );
  1298	                                            }
  1299	                                            for payload in &planned {
  1300	                                                match payload {
  1301	                                                    TransferPayload::File(header) => {
  1302	                                                        // w5-1: was unconditional per-file
  1303	                                                        // eprintln; BLIT_LOG=debug shows it.
  1304	                                                        log::debug!(
  1305	                                                            "push enqueue {} for TCP stream",
  1306	                                                            header.relative_path
  1307	                                                        );
  1308	                                                    }
  1309	                                                    TransferPayload::TarShard { headers } => {
  1310	                                                        for header in headers {
  1311	                                                            log::debug!(
  1312	                                                                "push enqueue {} via tar shard",
  1313	                                                                header.relative_path
  1314	                                                            );
  1315	                                                        }
  1316	                                                    }
  1317	                                                    TransferPayload::FileBlock { .. }
  1318	                                                    | TransferPayload::FileBlockComplete { .. } => {
  1319	                                                        // Receive-only — never produced by the outbound planner.
  1320	                                                    }
  1321	                                                }
  1322	                                            }
  1323	                                            if !planned.is_empty() {
  1324	                                                let sent = payload_file_count(&planned);
  1325	                                                sender.queue(planned).await?;
  1326	                                                if sent > 0 && first_payload_elapsed.is_none() {
  1327	                                                    first_payload_elapsed = Some(start.elapsed());
  1328	                                                }
  1329	                                                data_plane_files_sent += sent;
  1330	                                                data_plane_outstanding =
  1331	                                                    data_plane_outstanding.saturating_sub(sent);
  1332	                                            }
  1333	                                        }
  1334	                                    }
  1335	                                }
  1336	                                TransferMode::Undecided => {}
  1337	                            }
  1338	                        }
  1339	                        None => {
  1340	                            manifest_done = true;
  1341	                            // R59 #1 F1: report scan completeness to the
  1342	                            // daemon at ManifestComplete time. Walkdir
  1343	                            // errors land in `unreadable_paths` synchronously
  1344	                            // during the scan; the channel closing (None)
  1345	                            // guarantees the manifest task has finished
  1346	                            // pushing them, so reading here is race-free.
  1347	                            let scan_complete = unreadable_paths
  1348	                                .lock()
  1349	                                .map(|g| g.is_empty())
  1350	                                .unwrap_or(false);
  1351	                            if let Err(send_err) =
  1352	                                send_manifest_complete(&tx, scan_complete).await
  1353	                            {
  1354	                                return Err(
  1355	                                    prefer_server_error(&mut response_rx, send_err).await
  1356	                                );
  1357	                            }
  1358	                        }
  1359	                    }
  1360	                }
  1361	
  1362	                // ue-r2-2: the tuner proposed a stream-count change.
  1363	                // Lowest select priority (biased): control frames and
  1364	                // manifest flow always come first, and at most one
  1365	                // epoch is in flight.
  1366	                proposal = async {
  1367	                    match resize_proposal_rx.as_mut() {
  1368	                        Some(rx) => rx.recv().await,
  1369	                        None => std::future::pending().await,
  1370	                    }
  1371	                }, if resize_pending.is_none() => {
  1372	                    match proposal {
  1373	                        Some(p) => {
  1374	                            let dial_ref = dial
  1375	                                .as_ref()
  1376	                                .expect("resize only negotiated on the dial path");
  1377	                            if p.add {
  1378	                                // Pre-dial ADD: mint the epoch credential,
  1379	                                // ask the daemon to register it and arm an
  1380	                                // accept; the dial happens on the ack.
  1381	                                if let Err(send_err) =
  1382	                                    send_resize_add(&tx, dial_ref, p, &mut resize_pending).await
  1383	                                {
  1384	                                    return Err(prefer_server_error(
  1385	                                        &mut response_rx,
  1386	                                        send_err,
  1387	                                    )
  1388	                                    .await);
  1389	                                }
  1390	                            } else {
  1391	                                // sf-2: the tuner wants FEWER streams — live
  1392	                                // throughput evidence outranks the static
  1393	                                // shape table from here on. Never re-add what
  1394	                                // the tuner retires.
  1395	                                shape_resize_enabled = false;
  1396	                                // REMOVE: retire locally first — the drained
  1397	                                // worker's END record is the daemon-side
  1398	                                // teardown — then tell the daemon
  1399	                                // (accounting). Settle IMMEDIATELY with what
  1400	                                // actually happened (review: the retire is
  1401	                                // fait accompli; waiting on the
  1402	                                // accounting-only ack could diverge the dial
  1403	                                // from the real worker count on a refusal).
  1404	                                // The daemon's ack then matches no pending
  1405	                                // epoch and is ignored as unsolicited.
  1406	                                let retired = data_plane_sender
  1407	                                    .as_mut()
  1408	                                    .map(|s| s.retire_stream())
  1409	                                    .unwrap_or(false);
  1410	                                if retired {
  1411	                                    if let Err(send_err) = send_payload(
  1412	                                        &tx,
  1413	                                        ClientPayload::DataPlaneResize(DataPlaneResize {
  1414	                                            op: DataPlaneResizeOp::Remove as i32,
  1415	                                            epoch: p.epoch,
  1416	                                            target_stream_count: p.target_streams as u32,
  1417	                                            sub_token: Vec::new(),
  1418	                                        }),
  1419	                                    )
  1420	                                    .await
  1421	                                    {
  1422	                                        return Err(prefer_server_error(
  1423	                                            &mut response_rx,
  1424	                                            send_err,
  1425	                                        )
  1426	                                        .await);
  1427	                                    }
  1428	                                    dial_ref.resize_settled(
  1429	                                        p.epoch,
  1430	                                        p.target_streams,
  1431	                                        true,
  1432	                                    );
  1433	                                } else {
  1434	                                    dial_ref.resize_settled(
  1435	                                        p.epoch,
  1436	                                        dial_ref.live_streams(),
  1437	                                        false,
  1438	                                    );
  1439	                                }
  1440	                            }
  1441	                        }
  1442	                        None => resize_proposal_rx = None,
  1443	                    }
  1444	                }
  1445	            }
  1446	
  1447	            if matches!(transfer_mode, TransferMode::Fallback)
  1448	                && !fallback_upload_complete_sent
  1449	                && !need_list_fresh
  1450	                && need_list_received
  1451	                && manifest_done
  1452	                && pending_queue.is_empty()
  1453	                && (files_requested.is_empty() || fallback_files_sent >= files_requested.len())
  1454	            {
  1455	                // Send UploadComplete via a temporary GrpcFallbackSink.
  1456	                let finish_sink = GrpcFallbackSink::new(
  1457	                    source.clone(),
  1458	                    tx.clone(),
  1459	                    CONTROL_PLANE_CHUNK_SIZE,
  1460	                    PathBuf::from("grpc-fallback"),
  1461	                );
  1462	                finish_sink.finish().await?;
  1463	                fallback_upload_complete_sent = true;
  1464	            }
  1465	
  1466	            if matches!(transfer_mode, TransferMode::DataPlane)
  1467	                && !need_list_fresh
  1468	                && need_lists_done
  1469	                && pending_queue.is_empty()
  1470	                && manifest_done
  1471	                && data_plane_outstanding == 0
  1472	                && data_plane_files_sent >= files_requested.len()
  1473	            {
  1474	                if let Some(sender) = data_plane_sender.take() {
  1475	                    sender.finish().await?;
  1476	                }
  1477	            }
  1478	        }
  1479	
  1480	        manifest_task
  1481	            .await
  1482	            .map_err(|err| eyre::eyre!("manifest enumeration task failed: {}", err))??;
  1483	
  1484	        if let Some(sender) = data_plane_sender.take() {
  1485	            sender.finish().await?;
  1486	        }
  1487	
  1488	        if let Err(join_err) = response_task.join().await {
  1489	            return Err(eyre::eyre!("response stream task failed: {}", join_err));
  1490	        }
  1491	
  1492	        let summary = summary.ok_or_else(|| eyre::eyre!("push stream ended without summary"))?;
  1493	
  1494	        let unreadable = unreadable_paths
  1495	            .lock()
  1496	            .map_err(|err| eyre!("manifest warnings poisoned: {}", err))?;
  1497	        if !unreadable.is_empty() {
  1498	            let preview: Vec<_> = unreadable.iter().take(5).cloned().collect();
  1499	            let mut message = format!(
  1500	                "{} file(s) were skipped due to permission or access errors: {}",
  1501	                unreadable.len(),
  1502	                preview.join(", ")
  1503	            );
  1504	            if unreadable.len() > preview.len() {
  1505	                let remaining = unreadable.len() - preview.len();
  1506	                message.push_str(&format!(" (and {} more)", remaining));
  1507	            }
  1508	            return Err(eyre!(message));
  1509	        }
  1510	
  1511	        Ok(RemotePushReport {
  1512	            files_requested,
  1513	            fallback_used,
  1514	            data_port,
  1515	            summary,
  1516	            first_payload_elapsed,
  1517	            data_plane_streams: match (&dial, data_port) {
  1518	                (Some(dial), Some(_)) => Some(dial.live_streams()),
  1519	                _ => None,
  1520	            },
  1521	        })
  1522	    }
  1523	}
  1524	
  1525	async fn stream_fallback_from_queue(
  1526	    source: Arc<dyn TransferSource>,
  1527	    pending_queue: &mut VecDeque<String>,
  1528	    manifest_lookup: &HashMap<String, FileHeader>,
  1529	    tx: &mpsc::Sender<ClientPushRequest>,
  1530	    progress: Option<&RemotePushProgress>,
  1531	    plan_options: PlanOptions,
  1532	    chunk_bytes: usize,
  1533	    payload_prefetch: usize,
  1534	    unreadable: &Arc<Mutex<Vec<String>>>,
  1535	) -> Result<FallbackStreamResult> {

exec
/usr/bin/zsh -lc "rg -n \"data_plane_streams|RemotePushReport \\{|fallback_used|RemotePushReport\" crates -g'*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-tui/src/state.rs:616:                tcp_fallback_used: false,
crates/blit-tui/src/state.rs:649:                tcp_fallback_used: false,
crates/blit-tui/src/state.rs:677:                tcp_fallback_used: false,
crates/blit-tui/src/state.rs:735:                tcp_fallback_used: false,
crates/blit-tui/src/state.rs:962:                        tcp_fallback_used: false,
crates/blit-tui/src/state.rs:1011:                        tcp_fallback_used: false,
crates/blit-tui/src/state.rs:1062:                        tcp_fallback_used: false,
crates/blit-tui/src/state.rs:1206:                    tcp_fallback_used: false,
crates/blit-tui/src/state.rs:1364:                        tcp_fallback_used: false,
crates/blit-tui/src/state.rs:1413:                        tcp_fallback_used: false,
crates/blit-tui/src/state.rs:1444:                        tcp_fallback_used: false,
crates/blit-app/src/transfers/remote.rs:60:use blit_core::remote::push::RemotePushReport;
crates/blit-app/src/transfers/remote.rs:435:    pub report: RemotePushReport,
crates/blit-app/src/transfers/remote.rs:445:/// surface through the returned [`RemotePushReport`].
crates/blit-daemon/src/service/pull_sync.rs:593:            tcp_fallback_used: tcp_fallback,
crates/blit-daemon/src/service/delegated_pull.rs:548:        tcp_fallback_used: s.map(|x| x.tcp_fallback_used).unwrap_or(false),
crates/blit-daemon/src/service/delegated_pull.rs:870:            tcp_fallback_used: false,
crates/blit-daemon/src/service/delegated_pull.rs:889:            tcp_fallback_used: false,
crates/blit-daemon/src/service/core.rs:327:                // `tcp_fallback_used` plumbs through the handler's
crates/blit-daemon/src/service/core.rs:329:                tcp_fallback_used: false,
crates/blit-daemon/src/service/core.rs:1939:        // - files/tcp_fallback_used follow the documented zero/
crates/blit-daemon/src/service/core.rs:1961:                assert!(!c.tcp_fallback_used);
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:101:    assert!(!report.fallback_used, "must ride the TCP data plane");
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:107:        .data_plane_streams
crates/blit-daemon/src/service/push/control.rs:96:    let mut fallback_used = false;
crates/blit-daemon/src/service/push/control.rs:251:                                    fallback_used = true;
crates/blit-daemon/src/service/push/control.rs:384:        fallback_used = true;
crates/blit-daemon/src/service/push/control.rs:525:            tcp_fallback_used: fallback_used,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:658:        tcp_fallback_used: true,
crates/blit-core/src/remote/mod.rs:11:pub use push::{RemotePushClient, RemotePushProgress, RemotePushReport};
crates/blit-core/src/remote/pull.rs:331:            receive_data_plane_streams_owned(
crates/blit-core/src/remote/pull.rs:1672:async fn receive_data_plane_streams_owned(
crates/blit-core/src/remote/pull.rs:2034:    //! `receive_data_plane_streams_owned` — the machinery the PullSync
crates/blit-core/src/remote/pull.rs:2047:    use super::receive_data_plane_streams_owned;
crates/blit-core/src/remote/pull.rs:2093:            receive_data_plane_streams_owned(
crates/blit-core/src/remote/pull.rs:2202:        let guard = super::AbortOnDrop::new(tokio::spawn(receive_data_plane_streams_owned(
crates/blit-core/src/remote/pull.rs:2252:            receive_data_plane_streams_owned(
crates/blit-core/src/remote/push/mod.rs:5:pub use client::{ProgressEvent, RemotePushClient, RemotePushProgress, RemotePushReport};
crates/blit-core/src/remote/push/client/types.rs:6:pub struct RemotePushReport {
crates/blit-core/src/remote/push/client/types.rs:8:    pub fallback_used: bool,
crates/blit-core/src/remote/push/client/types.rs:16:    pub data_plane_streams: Option<usize>,
crates/blit-core/src/remote/push/client/mod.rs:5:pub use types::{RemotePushProgress, RemotePushReport, TransferMode};
crates/blit-core/src/remote/push/client/mod.rs:626:    ) -> Result<RemotePushReport> {
crates/blit-core/src/remote/push/client/mod.rs:717:        let mut fallback_used = force_grpc;
crates/blit-core/src/remote/push/client/mod.rs:942:                                        fallback_used = true;
crates/blit-core/src/remote/push/client/mod.rs:1511:        Ok(RemotePushReport {
crates/blit-core/src/remote/push/client/mod.rs:1513:            fallback_used,
crates/blit-core/src/remote/push/client/mod.rs:1517:            data_plane_streams: match (&dial, data_port) {
crates/blit-cli/src/jobs.rs:843:    /// tcp_fallback_used, ok), missing kind/peer/module/path/
crates/blit-cli/src/jobs.rs:854:            tcp_fallback_used: false,
crates/blit-cli/src/transfers/remote.rs:15:use blit_core::remote::{RemoteEndpoint, RemotePullReport, RemotePushReport};
crates/blit-cli/src/transfers/remote.rs:185:    pub report: blit_core::remote::push::RemotePushReport,
crates/blit-cli/src/transfers/remote.rs:463:        "tcp_fallback": report.summary.as_ref().map(|s| s.tcp_fallback_used).unwrap_or(false),
crates/blit-cli/src/transfers/remote.rs:469:fn print_push_json(report: &RemotePushReport, destination: &str) {
crates/blit-cli/src/transfers/remote.rs:479:        "tcp_fallback": report.summary.tcp_fallback_used,
crates/blit-cli/src/transfers/remote.rs:492:            if summary.tcp_fallback_used {
crates/blit-cli/src/transfers/remote.rs:510:    report: &RemotePushReport,
crates/blit-cli/src/transfers/remote.rs:520:    } else if report.fallback_used {
crates/blit-cli/src/transfers/remote.rs:543:        if summary.tcp_fallback_used {
crates/blit-cli/src/transfers/remote_remote_direct.rs:174:            tcp_fallback_used: false,
crates/blit-cli/src/transfers/remote_remote_direct.rs:249:        "tcp_fallback": summary.tcp_fallback_used,
crates/blit-cli/src/transfers/remote_remote_direct.rs:265:        if summary.tcp_fallback_used {

exec
/usr/bin/zsh -lc "rg -n \"DataPlaneResize|ResizeAck|resize_pending|PendingResize|take_resize_rx|resize_settled|resize_tick\" crates/blit-core/src crates/blit-daemon/src -g'*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/pull_sync.rs:20:    BlockTransferComplete, CapacityProfile, ClientPullMessage, ComparisonMode, DataPlaneResize,
crates/blit-daemon/src/service/pull_sync.rs:21:    DataPlaneResizeOp, DataTransferNegotiation, FileHeader, FileList, ManifestBatch, MirrorMode,
crates/blit-daemon/src/service/pull_sync.rs:843:                        Ok(sub) => (DataPlaneResizeOp::Add, sub),
crates/blit-daemon/src/service/pull_sync.rs:846:                            dial.resize_settled(p.epoch, dial.live_streams(), false);
crates/blit-daemon/src/service/pull_sync.rs:851:                    (DataPlaneResizeOp::Remove, Vec::new())
crates/blit-daemon/src/service/pull_sync.rs:855:                        payload: Some(server_pull_message::Payload::DataPlaneResize(
crates/blit-daemon/src/service/pull_sync.rs:856:                            DataPlaneResize {
crates/blit-daemon/src/service/pull_sync.rs:875:                    dial.resize_settled(p.epoch, dial.live_streams(), false);
crates/blit-daemon/src/service/pull_sync.rs:880:                    if let Some(client_pull_message::Payload::DataPlaneResizeAck(ack)) =
crates/blit-daemon/src/service/pull_sync.rs:888:                                    dial.resize_settled(epoch, dial.live_streams(), false);
crates/blit-daemon/src/service/pull_sync.rs:909:                                    dial.resize_settled(epoch, target, true);
crates/blit-daemon/src/service/pull_sync.rs:974:                                    dial.resize_settled(epoch, target, true);
crates/blit-daemon/src/service/pull_sync.rs:977:                                    dial.resize_settled(epoch, dial.live_streams(), false);
crates/blit-daemon/src/service/pull_sync.rs:985:                                dial.resize_settled(epoch, dial.live_streams(), false);
crates/blit-daemon/src/service/pull_sync.rs:999:                    dial.resize_settled(epoch, dial.live_streams(), false);
crates/blit-core/src/engine/dial.rs:20://!   count becomes live at `ue-r2-2` (DataPlaneResize); until then the
crates/blit-core/src/engine/dial.rs:97:    /// `resize_settled` on an accepted epoch.
crates/blit-core/src/engine/dial.rs:119:/// control stream turns this into a wire `DataPlaneResize` (the engine
crates/blit-core/src/engine/dial.rs:121:/// [`TransferDial::resize_settled`] for the epoch — with what actually
crates/blit-core/src/engine/dial.rs:242:    /// True while a proposal is awaiting `resize_settled`.
crates/blit-core/src/engine/dial.rs:243:    pub fn resize_pending(&self) -> bool {
crates/blit-core/src/engine/dial.rs:271:    /// call [`Self::resize_settled`] with the outcome; until then
crates/blit-core/src/engine/dial.rs:273:    pub fn resize_tick(&self, delta_bytes: u64, blocked_ratio: f64) -> Option<ResizeProposal> {
crates/blit-core/src/engine/dial.rs:342:    /// Unlike [`Self::resize_tick`] this is a definite signal — the
crates/blit-core/src/engine/dial.rs:376:    pub fn resize_settled(&self, epoch: u32, effective_streams: usize, accepted: bool) {
crates/blit-core/src/engine/dial.rs:534:/// provided — each [`TransferDial::resize_tick`] proposal is forwarded
crates/blit-core/src/engine/dial.rs:570:                    dial.resize_tick(0, 0.0);
crates/blit-core/src/engine/dial.rs:582:            // F3): the idle tick must still reach `resize_tick` so a
crates/blit-core/src/engine/dial.rs:587:                    dial.resize_tick(0, 0.0);
crates/blit-core/src/engine/dial.rs:594:                if let Some(proposal) = dial.resize_tick(delta_bytes, ratio) {
crates/blit-core/src/engine/dial.rs:599:                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
crates/blit-core/src/engine/dial.rs:790:            assert_eq!(dial.resize_tick(1024, 0.15), None, "in-band tick holds");
crates/blit-core/src/engine/dial.rs:802:            assert_eq!(dial.resize_tick(1024, 0.0), None);
crates/blit-core/src/engine/dial.rs:808:        assert_eq!(dial.resize_tick(1024, 0.0), None, "sustain tick 1");
crates/blit-core/src/engine/dial.rs:810:            .resize_tick(1024, 0.0)
crates/blit-core/src/engine/dial.rs:820:        assert!(dial.resize_pending());
crates/blit-core/src/engine/dial.rs:824:            assert_eq!(dial.resize_tick(1024, 0.0), None, "pending blocks");
crates/blit-core/src/engine/dial.rs:829:        dial.resize_settled(1, 5, true);
crates/blit-core/src/engine/dial.rs:832:        assert!(!dial.resize_pending());
crates/blit-core/src/engine/dial.rs:834:            assert_eq!(dial.resize_tick(1024, 0.0), None, "cooldown holds");
crates/blit-core/src/engine/dial.rs:838:        let next = dial.resize_tick(1024, 0.0).expect("epoch 2 proposes");
crates/blit-core/src/engine/dial.rs:851:        assert_eq!(dial.resize_tick(1024, 0.9), None, "sustain tick 1");
crates/blit-core/src/engine/dial.rs:852:        let proposal = dial.resize_tick(1024, 0.9).expect("sustained block drops");
crates/blit-core/src/engine/dial.rs:861:        dial.resize_settled(1, 1, true);
crates/blit-core/src/engine/dial.rs:867:            assert_eq!(dial.resize_tick(1024, 0.9), None, "floor at 1");
crates/blit-core/src/engine/dial.rs:880:        assert_eq!(dial.resize_tick(1024, 0.0), None);
crates/blit-core/src/engine/dial.rs:881:        assert_eq!(dial.resize_tick(0, 0.0), None, "idle resets");
crates/blit-core/src/engine/dial.rs:882:        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
crates/blit-core/src/engine/dial.rs:884:        assert_eq!(dial.resize_tick(1024, 0.15), None, "in-band resets");
crates/blit-core/src/engine/dial.rs:885:        assert_eq!(dial.resize_tick(1024, 0.0), None, "streak restarted");
crates/blit-core/src/engine/dial.rs:886:        assert!(dial.resize_tick(1024, 0.0).is_some(), "streak completes");
crates/blit-core/src/engine/dial.rs:895:        assert_eq!(dial.resize_tick(1024, 0.0), None);
crates/blit-core/src/engine/dial.rs:896:        let proposal = dial.resize_tick(1024, 0.0).expect("proposes");
crates/blit-core/src/engine/dial.rs:899:        dial.resize_settled(proposal.epoch + 7, 9, true);
crates/blit-core/src/engine/dial.rs:900:        assert!(dial.resize_pending(), "stale settle ignored");
crates/blit-core/src/engine/dial.rs:903:        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
crates/blit-core/src/engine/dial.rs:904:        assert!(!dial.resize_pending());
crates/blit-core/src/engine/dial.rs:917:                dial.resize_tick(1024, 0.0),
crates/blit-core/src/engine/dial.rs:972:        assert_eq!(dial.resize_tick(1024, 0.0), None, "tuner blocked too");
crates/blit-core/src/engine/dial.rs:975:        dial.resize_settled(1, 2, true);
crates/blit-core/src/engine/dial.rs:979:        dial.resize_settled(2, 3, true);
crates/blit-core/src/engine/dial.rs:985:        dial.resize_settled(p3.epoch, dial.live_streams(), false);
crates/blit-core/src/engine/dial.rs:1001:        dial.resize_settled(p.epoch, 2, true);
crates/blit-core/src/engine/dial.rs:1042:        assert!(dial.resize_pending());
crates/blit-daemon/src/service/push/control.rs:13:    client_push_request, server_push_response, Ack, ClientPushRequest, DataPlaneResize,
crates/blit-daemon/src/service/push/control.rs:14:    DataPlaneResizeAck, DataPlaneResizeOp, DataTransferNegotiation, FileHeader, FileList,
crates/blit-daemon/src/service/push/control.rs:360:            Some(client_push_request::Payload::DataPlaneResize(req)) => {
crates/blit-daemon/src/service/push/control.rs:445:            // plane runs — the client's DataPlaneResize frames arrive
crates/blit-daemon/src/service/push/control.rs:466:                            if let Some(client_push_request::Payload::DataPlaneResize(req)) =
crates/blit-daemon/src/service/push/control.rs:541:/// ue-r2-2: answer a client `DataPlaneResize`. ADD registers the
crates/blit-daemon/src/service/push/control.rs:558:    req: DataPlaneResize,
crates/blit-daemon/src/service/push/control.rs:560:    let op = DataPlaneResizeOp::try_from(req.op).unwrap_or(DataPlaneResizeOp::Unspecified);
crates/blit-daemon/src/service/push/control.rs:566:        (DataPlaneResizeOp::Add, Some(cmd_tx)) => {
crates/blit-daemon/src/service/push/control.rs:576:        (DataPlaneResizeOp::Remove, Some(_)) => true,
crates/blit-daemon/src/service/push/control.rs:581:            DataPlaneResizeOp::Add => *resize_live = resize_live.saturating_add(1),
crates/blit-daemon/src/service/push/control.rs:582:            DataPlaneResizeOp::Remove => *resize_live = resize_live.saturating_sub(1).max(1),
crates/blit-daemon/src/service/push/control.rs:588:            "push: refusing DataPlaneResize (op {:?}, epoch {}, target {})",
crates/blit-daemon/src/service/push/control.rs:596:        server_push_response::Payload::DataPlaneResizeAck(DataPlaneResizeAck {
crates/blit-core/src/remote/pull.rs:1012:                Some(server_pull_message::Payload::DataPlaneResize(cmd)) => {
crates/blit-core/src/remote/pull.rs:1023:                    let op = crate::generated::DataPlaneResizeOp::try_from(cmd.op)
crates/blit-core/src/remote/pull.rs:1024:                        .unwrap_or(crate::generated::DataPlaneResizeOp::Unspecified);
crates/blit-core/src/remote/pull.rs:1030:                        crate::generated::DataPlaneResizeOp::Add => {
crates/blit-core/src/remote/pull.rs:1046:                        crate::generated::DataPlaneResizeOp::Remove => data_plane_growth.is_some(),
crates/blit-core/src/remote/pull.rs:1051:                            crate::generated::DataPlaneResizeOp::Add => data_plane_live += 1,
crates/blit-core/src/remote/pull.rs:1052:                            crate::generated::DataPlaneResizeOp::Remove => {
crates/blit-core/src/remote/pull.rs:1060:                            "pull: refusing DataPlaneResize (op {}, epoch {}, target {})",
crates/blit-core/src/remote/pull.rs:1067:                        payload: Some(client_pull_message::Payload::DataPlaneResizeAck(
crates/blit-core/src/remote/pull.rs:1068:                            crate::generated::DataPlaneResizeAck {
crates/blit-core/src/remote/push/client/mod.rs:16:use crate::generated::{DataPlaneResize, DataPlaneResizeOp, FileHeader, PushSummary};
crates/blit-core/src/remote/push/client/mod.rs:124:    /// loop via `take_resize_rx` (the loop owns ack correlation).
crates/blit-core/src/remote/push/client/mod.rs:129:/// held from the `DataPlaneResize` send until the daemon's ack.
crates/blit-core/src/remote/push/client/mod.rs:130:struct PendingResize {
crates/blit-core/src/remote/push/client/mod.rs:322:    fn take_resize_rx(
crates/blit-core/src/remote/push/client/mod.rs:480:/// the `DataPlaneResize` ADD, and record the in-flight epoch (the
crates/blit-core/src/remote/push/client/mod.rs:488:    resize_pending: &mut Option<PendingResize>,
crates/blit-core/src/remote/push/client/mod.rs:494:                ClientPayload::DataPlaneResize(DataPlaneResize {
crates/blit-core/src/remote/push/client/mod.rs:495:                    op: DataPlaneResizeOp::Add as i32,
crates/blit-core/src/remote/push/client/mod.rs:502:            *resize_pending = Some(PendingResize {
crates/blit-core/src/remote/push/client/mod.rs:511:            dial.resize_settled(proposal.epoch, dial.live_streams(), false);
crates/blit-core/src/remote/push/client/mod.rs:533:    resize_pending: &mut Option<PendingResize>,
crates/blit-core/src/remote/push/client/mod.rs:535:    if resize_pending.is_some() {
crates/blit-core/src/remote/push/client/mod.rs:542:        Some(proposal) => send_resize_add(tx, dial, proposal, resize_pending).await,
crates/blit-core/src/remote/push/client/mod.rs:739:        // `resize_pending` is the single epoch awaiting the daemon's
crates/blit-core/src/remote/push/client/mod.rs:744:        let mut resize_pending: Option<PendingResize> = None;
crates/blit-core/src/remote/push/client/mod.rs:870:                                                        &mut resize_pending,
crates/blit-core/src/remote/push/client/mod.rs:1028:                                            resize_proposal_rx = sender.take_resize_rx();
crates/blit-core/src/remote/push/client/mod.rs:1043:                                                    &mut resize_pending,
crates/blit-core/src/remote/push/client/mod.rs:1099:                                Some(ServerPayload::DataPlaneResizeAck(ack)) => {
crates/blit-core/src/remote/push/client/mod.rs:1103:                                    match resize_pending.take() {
crates/blit-core/src/remote/push/client/mod.rs:1136:                                                    dial_ref.resize_settled(
crates/blit-core/src/remote/push/client/mod.rs:1142:                                                    dial_ref.resize_settled(
crates/blit-core/src/remote/push/client/mod.rs:1149:                                                dial_ref.resize_settled(
crates/blit-core/src/remote/push/client/mod.rs:1155:                                                dial_ref.resize_settled(
crates/blit-core/src/remote/push/client/mod.rs:1176:                                                    &mut resize_pending,
crates/blit-core/src/remote/push/client/mod.rs:1189:                                            resize_pending = other;
crates/blit-core/src/remote/push/client/mod.rs:1191:                                                "ignoring unsolicited/stale DataPlaneResizeAck \
crates/blit-core/src/remote/push/client/mod.rs:1371:                }, if resize_pending.is_none() => {
crates/blit-core/src/remote/push/client/mod.rs:1382:                                    send_resize_add(&tx, dial_ref, p, &mut resize_pending).await
crates/blit-core/src/remote/push/client/mod.rs:1413:                                        ClientPayload::DataPlaneResize(DataPlaneResize {
crates/blit-core/src/remote/push/client/mod.rs:1414:                                            op: DataPlaneResizeOp::Remove as i32,
crates/blit-core/src/remote/push/client/mod.rs:1428:                                    dial_ref.resize_settled(
crates/blit-core/src/remote/push/client/mod.rs:1434:                                    dial_ref.resize_settled(
crates/blit-core/src/remote/transfer/data_plane.rs:25:/// sockets, `DataPlaneResize.sub_token` for an ADD epoch's socket).

codex
There’s one suspicious behavior: `send_resize_add` records client pending state only after the control send succeeds, but the dial’s pending epoch is set before that. I’m checking whether the error path unwinds it elsewhere or can leave the dial permanently pending.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/push/client/mod.rs | sed -n '1,430p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	pub mod helpers;
     2	mod types;
     3	
     4	pub use crate::remote::transfer::progress::ProgressEvent;
     5	pub use types::{RemotePushProgress, RemotePushReport, TransferMode};
     6	
     7	use self::helpers::{
     8	    decode_token, destination_path, drain_pending_headers, map_status, module_and_path,
     9	    prefer_server_error, send_manifest_complete, send_payload, spawn_response_task,
    10	};
    11	use crate::buffer::BufferPool;
    12	use crate::fs_enum::FileFilter;
    13	use crate::generated::client_push_request::Payload as ClientPayload;
    14	use crate::generated::server_push_response::Payload as ServerPayload;
    15	use crate::generated::ClientPushRequest;
    16	use crate::generated::{DataPlaneResize, DataPlaneResizeOp, FileHeader, PushSummary};
    17	use crate::remote::endpoint::RemoteEndpoint;
    18	use crate::remote::transfer::CONTROL_PLANE_CHUNK_SIZE;
    19	use crate::transfer_plan::PlanOptions;
    20	use eyre::{eyre, Result};
    21	use std::collections::{HashMap, HashSet, VecDeque};
    22	use std::path::PathBuf;
    23	use std::sync::{Arc, Mutex};
    24	use std::time::{Duration, Instant};
    25	use tokio::sync::mpsc;
    26	use tokio::task::JoinHandle;
    27	use tokio_stream::wrappers::ReceiverStream;
    28	
    29	use super::data_plane::DataPlaneSession;
    30	use super::payload::{payload_file_count, TransferPayload};
    31	// Push planning routes through the unified diff_planner module so the
    32	// canonical entry point is the same regardless of origin type. Push's
    33	// "diff" itself lives on the daemon side (NeedList) — see plan_push_payloads.
    34	use crate::remote::transfer::diff_planner::plan_push_payloads as plan_transfer_payloads;
    35	use crate::remote::transfer::pipeline::{
    36	    execute_sink_pipeline, execute_sink_pipeline_elastic, SinkControl,
    37	};
    38	use crate::remote::transfer::progress::RemoteTransferProgress;
    39	use crate::remote::transfer::sink::{DataPlaneSink, GrpcFallbackSink, SinkOutcome, TransferSink};
    40	use crate::remote::transfer::source::TransferSource;
    41	use crate::remote::transfer::AbortOnDrop;
    42	
    43	/// Await a pipeline JoinHandle and return the outcome with
    44	/// consistent error wrapping. Used by both `MultiStreamSender::queue`
    45	/// (via `drain_pipeline_error`) and `MultiStreamSender::finish` so
    46	/// the failure-path messages are identical regardless of which side
    47	/// noticed the pipeline died first.
    48	///
    49	/// Terminal states:
    50	///
    51	/// - `Ok(Ok(o))` → `Ok(o)` — pipeline returned cleanly with the
    52	///   accumulated `SinkOutcome`.
    53	/// - `Ok(Err(e))` → `Err(e.wrap_err("data plane pipeline failed"))` —
    54	///   the eyre cause chain reads "data plane pipeline failed: <inner>"
    55	///   so the underlying disk-full / channel-closed / etc. surfaces in
    56	///   the user-visible message.
    57	/// - `Err(join)` → `Err(eyre!("data plane pipeline panicked: {join}"))`
    58	///   — the panic message surfaces rather than being hidden.
    59	///
    60	/// Closes R43 follow-up to R42-F2: previously `finish()` duplicated
    61	/// these match arms while a comment claimed they routed through the
    62	/// helper. Now there's actually one helper.
    63	///
    64	/// w4-1: takes `AbortOnDrop` (not a bare `JoinHandle`) and drains via
    65	/// `.join()` — if the caller's future is cancelled mid-await, the
    66	/// wrapper's Drop aborts the pipeline task instead of detaching it.
    67	async fn drain_pipeline_outcome(handle: AbortOnDrop<Result<SinkOutcome>>) -> Result<SinkOutcome> {
    68	    match handle.join().await {
    69	        Ok(Ok(o)) => Ok(o),
    70	        Ok(Err(e)) => Err(e.wrap_err("data plane pipeline failed")),
    71	        Err(join) => Err(eyre!("data plane pipeline panicked: {join}")),
    72	    }
    73	}
    74	
    75	/// Drain a pipeline JoinHandle into a clear `eyre::Report` for the
    76	/// producer-side path where we already know the channel closed.
    77	/// Wraps `drain_pipeline_outcome` so the failure formatting is
    78	/// shared, then converts the `Ok` case (channel closed but pipeline
    79	/// returned cleanly) into a diagnostic message — that combination is
    80	/// the rare race in pipeline shutdown that we surface rather than
    81	/// hide behind silence.
    82	///
    83	/// Extracted to a free function so the join-error-drain logic is
    84	/// directly testable without spinning up a full
    85	/// `MultiStreamSender::connect` (which requires real TCP streams).
    86	/// Closes R42-F2.
    87	async fn drain_pipeline_error(handle: AbortOnDrop<Result<SinkOutcome>>) -> eyre::Report {
    88	    match drain_pipeline_outcome(handle).await {
    89	        Ok(_) => eyre!(
    90	            "data plane pipeline closed cleanly but the producer \
    91	             channel was already closed — likely a race in \
    92	             pipeline shutdown"
    93	        ),
    94	        Err(report) => report,
    95	    }
    96	}
    97	
    98	/// Feeds payloads into N TCP data-plane sinks via the unified streaming
    99	/// pipeline. The event loop pushes payloads as need-list batches arrive;
   100	/// round-robin distribution across sinks is handled by the pipeline.
   101	struct MultiStreamSender {
   102	    payload_tx: Option<mpsc::Sender<TransferPayload>>,
   103	    /// ue-r2-1e: live tuner sampling the per-stream telemetry into the
   104	    /// dial. Aborted on finish(); self-terminates via its Weak<dial>
   105	    /// if the sender is dropped without finishing.
   106	    tuner_handle: Option<JoinHandle<()>>,
   107	    /// Pipeline handle. `Option` so `queue()` can `take()` it on
   108	    /// the unhappy path: if `tx.send().await` fails the receiver has
   109	    /// been dropped, which means the pipeline died with an error
   110	    /// inside the spawned task. We surface that real error instead
   111	    /// of the previous generic "data plane pipeline closed
   112	    /// unexpectedly" string. POST_REVIEW_FIXES §1.1b.
   113	    ///
   114	    /// w4-1: `AbortOnDrop`, not a bare `JoinHandle` — if `push()`
   115	    /// returns early via `?` while a `MultiStreamSender` is still
   116	    /// live, dropping it must abort the pipeline task (which owns
   117	    /// the sink workers' `JoinSet`) rather than leaving it running
   118	    /// with no owner.
   119	    pipeline_handle: Option<AbortOnDrop<Result<SinkOutcome>>>,
   120	    started: Instant,
   121	    /// ue-r2-2: present only when the negotiation enabled resize.
   122	    resize: Option<ResizeRuntime>,
   123	    /// ue-r2-2: the tuner's proposal stream, handed to the control
   124	    /// loop via `take_resize_rx` (the loop owns ack correlation).
   125	    resize_rx: Option<tokio::sync::mpsc::UnboundedReceiver<crate::engine::ResizeProposal>>,
   126	}
   127	
   128	/// ue-r2-2: the client-side controller's one in-flight resize epoch,
   129	/// held from the `DataPlaneResize` send until the daemon's ack.
   130	struct PendingResize {
   131	    epoch: u32,
   132	    target: usize,
   133	    add: bool,
   134	    /// The credential the epoch-N socket will present (ADD only).
   135	    sub_token: Vec<u8>,
   136	}
   137	
   138	/// ue-r2-2: everything an epoch-N dial needs, retained from connect
   139	/// time, plus the live handles into the running pipeline and tuner.
   140	struct ResizeRuntime {
   141	    ctl_tx: mpsc::UnboundedSender<SinkControl>,
   142	    probes: crate::engine::SharedStreamProbes,
   143	    host: String,
   144	    port: u32,
   145	    token: Vec<u8>,
   146	    trace: bool,
   147	    pool: Arc<BufferPool>,
   148	    source: Arc<dyn TransferSource>,
   149	    dst_root: PathBuf,
   150	    dial: Arc<crate::engine::TransferDial>,
   151	    next_stream_id: u32,
   152	}
   153	
   154	impl MultiStreamSender {
   155	    #[allow(clippy::too_many_arguments)]
   156	    async fn connect(
   157	        host: &str,
   158	        port: u32,
   159	        token: &[u8],
   160	        chunk_bytes: usize,
   161	        payload_prefetch: usize,
   162	        stream_count: usize,
   163	        trace: bool,
   164	        source: Arc<dyn TransferSource>,
   165	        tcp_buffer_size: Option<usize>,
   166	        progress: Option<RemoteTransferProgress>,
   167	        dial: Option<Arc<crate::engine::TransferDial>>,
   168	        // ue-r2-2: `Some(epoch0_sub_token)` when the daemon's
   169	        // negotiation set `resize_enabled` — every epoch-0 socket
   170	        // echoes it after the one-time token, and the sender becomes
   171	        // elastic (proposal stream + add/retire plumbing). Requires
   172	        // the dial path (telemetry drives the policy).
   173	        resize_sub: Option<Vec<u8>>,
   174	    ) -> Result<Self> {
   175	        let streams = stream_count.max(1);
   176	
   177	        // Shared buffer pool across all sinks (w3-1: the constructor
   178	        // owns the formula + available-memory cap). Elastic senders
   179	        // authorize the dial's resize ceiling instead of the epoch-0
   180	        // count — allocation is lazy, so this costs nothing until
   181	        // resize actually ADDs streams, and an ADDed stream draws from
   182	        // a budget that already covers it instead of queueing against
   183	        // an epoch-0 authorization forever.
   184	        let authorized_streams = match (&resize_sub, dial.as_ref()) {
   185	            (Some(_), Some(dial)) => dial.ceiling_max_streams().max(streams),
   186	            _ => streams,
   187	        };
   188	        let pool = Arc::new(BufferPool::for_data_plane(chunk_bytes, authorized_streams));
   189	
   190	        let dst_root = PathBuf::from(format!("{}:{}", host, port));
   191	
   192	        // ue-r2-2: epoch-0 sockets present token ‖ epoch0_sub_token
   193	        // when resize was negotiated; the plain token otherwise (the
   194	        // handshake is a raw byte write, so pre-concatenation IS the
   195	        // suffix contract).
   196	        let handshake: Vec<u8> = match &resize_sub {
   197	            Some(sub) => {
   198	                let mut h = token.to_vec();
   199	                h.extend_from_slice(sub);
   200	                h
   201	            }
   202	            None => token.to_vec(),
   203	        };
   204	
   205	        // Control channel into the (elastic) pipeline. Without resize
   206	        // the sender is simply never used and drops with this scope.
   207	        let (ctl_tx, ctl_rx) = mpsc::unbounded_channel::<SinkControl>();
   208	
   209	        // ue-r2-1e: with a dial, every stream carries LiveProbe
   210	        // telemetry and a tuner task steps the dial's cheap dials from
   211	        // it. Without one (no live tuning), the NoProbe path
   212	        // monomorphizes the telemetry away exactly as before.
   213	        let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(streams);
   214	        let mut tuner_handle = None;
   215	        let mut resize = None;
   216	        let mut resize_rx = None;
   217	        if let Some(dial) = dial.as_ref() {
   218	            use crate::engine::spawn_dial_tuner_with_resize;
   219	            use crate::remote::transfer::progress::{LiveProbe, StreamId, StreamProbe};
   220	            let mut tuner_probes = Vec::with_capacity(streams);
   221	            for idx in 0..streams {
   222	                let probe = StreamProbe::new(StreamId(idx as u32));
   223	                tuner_probes.push(StreamProbe::from_telemetry(
   224	                    StreamId(idx as u32),
   225	                    probe.telemetry(),
   226	                ));
   227	                let session = DataPlaneSession::connect_with_probe(
   228	                    host,
   229	                    port,
   230	                    &handshake,
   231	                    chunk_bytes,
   232	                    payload_prefetch,
   233	                    trace,
   234	                    tcp_buffer_size,
   235	                    Arc::clone(&pool),
   236	                    LiveProbe(probe),
   237	                )
   238	                .await?;
   239	                sinks.push(Arc::new(DataPlaneSink::new(
   240	                    session,
   241	                    source.clone(),
   242	                    dst_root.clone(),
   243	                )));
   244	            }
   245	            let probes: crate::engine::SharedStreamProbes =
   246	                Arc::new(std::sync::Mutex::new(tuner_probes));
   247	            if resize_sub.is_some() {
   248	                let (proposal_tx, proposal_rx) = tokio::sync::mpsc::unbounded_channel();
   249	                tuner_handle = Some(spawn_dial_tuner_with_resize(
   250	                    dial,
   251	                    Arc::clone(&probes),
   252	                    Some(proposal_tx),
   253	                ));
   254	                resize_rx = Some(proposal_rx);
   255	                resize = Some(ResizeRuntime {
   256	                    ctl_tx: ctl_tx.clone(),
   257	                    probes,
   258	                    host: host.to_string(),
   259	                    port,
   260	                    token: token.to_vec(),
   261	                    trace,
   262	                    pool: Arc::clone(&pool),
   263	                    source: source.clone(),
   264	                    dst_root: dst_root.clone(),
   265	                    dial: Arc::clone(dial),
   266	                    next_stream_id: streams as u32,
   267	                });
   268	            } else {
   269	                tuner_handle = Some(spawn_dial_tuner_with_resize(dial, probes, None));
   270	            }
   271	        } else {
   272	            for _ in 0..streams {
   273	                let session = DataPlaneSession::connect(
   274	                    host,
   275	                    port,
   276	                    &handshake,
   277	                    chunk_bytes,
   278	                    payload_prefetch,
   279	                    trace,
   280	                    tcp_buffer_size,
   281	                    Arc::clone(&pool),
   282	                )
   283	                .await?;
   284	                sinks.push(Arc::new(DataPlaneSink::new(
   285	                    session,
   286	                    source.clone(),
   287	                    dst_root.clone(),
   288	                )));
   289	            }
   290	        }
   291	
   292	        let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(payload_prefetch.max(1));
   293	
   294	        let source_clone = source.clone();
   295	        let prefetch = payload_prefetch.max(1);
   296	        drop(ctl_tx);
   297	        let pipeline_handle = AbortOnDrop::new(tokio::spawn(async move {
   298	            execute_sink_pipeline_elastic(
   299	                source_clone,
   300	                sinks,
   301	                payload_rx,
   302	                prefetch,
   303	                progress.as_ref(),
   304	                Some(ctl_rx),
   305	            )
   306	            .await
   307	        }));
   308	
   309	        Ok(Self {
   310	            payload_tx: Some(payload_tx),
   311	            tuner_handle,
   312	            pipeline_handle: Some(pipeline_handle),
   313	            started: Instant::now(),
   314	            resize,
   315	            resize_rx,
   316	        })
   317	    }
   318	
   319	    /// ue-r2-2: the tuner's proposal stream (present only when resize
   320	    /// was negotiated). The control loop takes it once and correlates
   321	    /// proposals with the daemon's acks.
   322	    fn take_resize_rx(
   323	        &mut self,
   324	    ) -> Option<tokio::sync::mpsc::UnboundedReceiver<crate::engine::ResizeProposal>> {
   325	        self.resize_rx.take()
   326	    }
   327	
   328	    /// ue-r2-2 ADD: dial one more data socket with the per-epoch
   329	    /// credential (token ‖ sub_token), register its probe with the
   330	    /// tuner, and hand its sink to the running pipeline. Errors are
   331	    /// the caller's to treat as NON-fatal — a failed optional ADD
   332	    /// must never kill a healthy transfer (the daemon's armed accept
   333	    /// slot simply expires).
   334	    async fn add_stream(&mut self, sub_token: &[u8]) -> Result<()> {
   335	        use crate::remote::transfer::progress::{LiveProbe, StreamId, StreamProbe};
   336	        let rt = self
   337	            .resize
   338	            .as_mut()
   339	            .ok_or_else(|| eyre!("resize was not negotiated for this transfer"))?;
   340	        let probe = StreamProbe::new(StreamId(rt.next_stream_id));
   341	        let tuner_probe = StreamProbe::from_telemetry(probe.id(), probe.telemetry());
   342	        let mut handshake = rt.token.clone();
   343	        handshake.extend_from_slice(sub_token);
   344	        let session = DataPlaneSession::connect_with_probe(
   345	            &rt.host,
   346	            rt.port,
   347	            &handshake,
   348	            // Live dial values: an epoch-N socket starts at the
   349	            // CURRENT tuning, not the connect-time snapshot.
   350	            rt.dial.chunk_bytes(),
   351	            rt.dial.prefetch_count(),
   352	            rt.trace,
   353	            rt.dial.tcp_buffer_bytes(),
   354	            Arc::clone(&rt.pool),
   355	            LiveProbe(probe),
   356	        )
   357	        .await?;
   358	        let sink: Arc<dyn TransferSink> = Arc::new(DataPlaneSink::new(
   359	            session,
   360	            rt.source.clone(),
   361	            rt.dst_root.clone(),
   362	        ));
   363	        if let Err(returned) = rt.ctl_tx.send(SinkControl::Add(sink)) {
   364	            // Pipeline already finished (transfer completing under the
   365	            // ADD). Close the just-authorized socket CLEANLY — the END
   366	            // record keeps the daemon's epoch-N worker from dying on a
   367	            // reset, which would fail an otherwise-complete push
   368	            // (post-handshake stream errors are fatal by design).
   369	            if let SinkControl::Add(sink) = returned.0 {
   370	                let _ = sink.finish().await;
   371	            }
   372	            return Err(eyre!("data plane pipeline is no longer running"));
   373	        }
   374	        rt.next_stream_id += 1;
   375	        rt.probes
   376	            .lock()
   377	            .expect("probe registry poisoned")
   378	            .push(tuner_probe);
   379	        Ok(())
   380	    }
   381	
   382	    /// ue-r2-2 REMOVE: retire the most recently added live stream —
   383	    /// its worker drains at the payload boundary and emits its END —
   384	    /// and drop its probe from the tuner registry. Returns false when
   385	    /// nothing can be retired (floor of one stream, or the pipeline is
   386	    /// gone), so the caller can settle the epoch as refused. The probe
   387	    /// pops only AFTER the pipeline accepted the retire (review: the
   388	    /// old order lost a probe when the pipeline was already gone).
   389	    fn retire_stream(&mut self) -> bool {
   390	        let Some(rt) = self.resize.as_mut() else {
   391	            return false;
   392	        };
   393	        {
   394	            let probes = rt.probes.lock().expect("probe registry poisoned");
   395	            if probes.len() <= 1 {
   396	                return false;
   397	            }
   398	        }
   399	        if rt.ctl_tx.send(SinkControl::RetireOne).is_err() {
   400	            return false;
   401	        }
   402	        rt.probes.lock().expect("probe registry poisoned").pop();
   403	        true
   404	    }
   405	
   406	    /// Feed one or more payloads to the streaming pipeline.
   407	    async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   408	        let tx = self
   409	            .payload_tx
   410	            .as_ref()
   411	            .ok_or_else(|| eyre!("data plane sender already finished"))?;
   412	        for payload in payloads {
   413	            if tx.send(payload).await.is_err() {
   414	                // Receiver dropped → pipeline task already exited.
   415	                // Drain `pipeline_handle` to surface the underlying
   416	                // error (sink worker errored, remote daemon closed,
   417	                // disk full on dest…) instead of the previous
   418	                // generic "data plane pipeline closed unexpectedly".
   419	                // POST_REVIEW_FIXES §1.1b.
   420	                drop(self.payload_tx.take());
   421	                let handle = self
   422	                    .pipeline_handle
   423	                    .take()
   424	                    .ok_or_else(|| eyre!("data plane pipeline handle missing"))?;
   425	                return Err(drain_pipeline_error(handle).await);
   426	            }
   427	        }
   428	        Ok(())
   429	    }
   430	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/dial.rs | sed -n '430,640p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   430	        let next = (chunk / 2).max(DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes));
   431	        if next < chunk {
   432	            self.chunk_bytes.store(next, Ordering::Relaxed);
   433	            moved = true;
   434	        }
   435	        let prefetch = self.prefetch_count.load(Ordering::Relaxed);
   436	        let next = (prefetch / 2)
   437	            .max(DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch))
   438	            .max(1);
   439	        if next < prefetch {
   440	            self.prefetch_count.store(next, Ordering::Relaxed);
   441	            moved = true;
   442	        }
   443	        moved
   444	    }
   445	
   446	    /// One tuner tick: adjust from the observed blocked-time ratio
   447	    /// (write-blocked nanos across streams ÷ wall nanos × streams for
   448	    /// the tick window). Between the thresholds nothing moves
   449	    /// (hysteresis band).
   450	    pub fn apply_tick(&self, blocked_ratio: f64) -> bool {
   451	        if blocked_ratio < DIAL_STEP_UP_BLOCKED_RATIO {
   452	            self.step_up_cheap_dials()
   453	        } else if blocked_ratio > DIAL_STEP_DOWN_BLOCKED_RATIO {
   454	            self.step_down_cheap_dials()
   455	        } else {
   456	            false
   457	        }
   458	    }
   459	}
   460	
   461	/// Workload-shape-aware initial stream proposal (`ue-r2-1f`): the
   462	/// end that KNOWS the workload shape proposes a starting stream
   463	/// count — file count matters as much as bytes (many small files
   464	/// parallelize on per-file overhead even at low byte totals). On push
   465	/// that is the receiving daemon (it has the manifest) clamped to its
   466	/// own advertised ceiling; on pull_sync it is the sending daemon (it
   467	/// enumerated the source) clamped to the CLIENT's advertised
   468	/// `receiver_capacity.max_streams` (`ue-r2-1g`) — either way the byte
   469	/// receiver's profile is the bound. Table carried over verbatim from
   470	/// the daemon push `desired_streams` ladder it retires (the ladder
   471	/// the old `tuning.rs` doc said "wins"), now engine-owned. The
   472	/// sender's dial clamps again on its side (`set_negotiated_streams`).
   473	/// Live mid-transfer stream changes arrive with `ue-r2-2` resize.
   474	pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
   475	    if file_count == 0 {
   476	        return 1;
   477	    }
   478	    let proposal: u32 = if total_bytes >= 32 * 1024 * 1024 * 1024 || file_count >= 200_000 {
   479	        16
   480	    } else if total_bytes >= 8 * 1024 * 1024 * 1024 || file_count >= 80_000 {
   481	        12
   482	    } else if total_bytes >= 2 * 1024 * 1024 * 1024 || file_count >= 50_000 {
   483	        10
   484	    } else if total_bytes >= 512 * 1024 * 1024 || file_count >= 10_000 {
   485	        8
   486	    } else if total_bytes >= 128 * 1024 * 1024 || file_count >= 2_000 {
   487	        4
   488	    } else if total_bytes >= 32 * 1024 * 1024 || file_count >= 256 {
   489	        2
   490	    } else {
   491	        1
   492	    };
   493	    proposal.min(ceiling.max(1) as u32)
   494	}
   495	
   496	/// Blocked-time ratio for one tuner tick: the share of the tick's
   497	/// wall-clock (× stream count) the senders spent inside socket writes.
   498	/// 0 streams or a zero-length tick reads as "no signal" (0.0 — the
   499	/// hysteresis band holds the dial still rather than guessing).
   500	pub(crate) fn blocked_ratio(
   501	    delta_blocked_nanos: u64,
   502	    elapsed: std::time::Duration,
   503	    streams: usize,
   504	) -> f64 {
   505	    let denom = elapsed.as_nanos().saturating_mul(streams as u128);
   506	    if denom == 0 {
   507	        return 0.0;
   508	    }
   509	    (delta_blocked_nanos as f64 / denom as f64).clamp(0.0, 1.0)
   510	}
   511	
   512	/// Growable per-transfer probe registry (`ue-r2-2`): resize adds a
   513	/// probe when a stream joins and removes it when one retires, and the
   514	/// tuner samples whatever is live each tick. Plain std mutex — locked
   515	/// only for a snapshot fold every 500ms and on resize events.
   516	pub type SharedStreamProbes =
   517	    Arc<std::sync::Mutex<Vec<crate::remote::transfer::progress::StreamProbe>>>;
   518	
   519	/// Spawn the live tuner for one transfer (ue-r2-1e): every
   520	/// [`DIAL_TUNER_TICK`] it sums the PR1 per-stream `write_blocked`
   521	/// telemetry and steps the dial's cheap dials. Holds only a `Weak` to
   522	/// the dial, so it self-terminates within one tick of the transfer
   523	/// dropping its dial; callers may also abort the handle for prompt
   524	/// shutdown (`MultiStreamSender::finish` does).
   525	pub fn spawn_dial_tuner(
   526	    dial: &Arc<TransferDial>,
   527	    probes: Vec<crate::remote::transfer::progress::StreamProbe>,
   528	) -> tokio::task::JoinHandle<()> {
   529	    spawn_dial_tuner_with_resize(dial, Arc::new(std::sync::Mutex::new(probes)), None)
   530	}
   531	
   532	/// `ue-r2-2` tuner: same cheap-dial stepping, but over a growable
   533	/// probe registry, plus the stream-resize policy when `resize_tx` is
   534	/// provided — each [`TransferDial::resize_tick`] proposal is forwarded
   535	/// to the adapter that owns the control stream (unbounded so a
   536	/// momentarily busy adapter cannot lose a proposal while the dial
   537	/// holds it pending). Callers without resize pass `None` and get
   538	/// exactly the ue-r2-1e behavior.
   539	pub fn spawn_dial_tuner_with_resize(
   540	    dial: &Arc<TransferDial>,
   541	    probes: SharedStreamProbes,
   542	    resize_tx: Option<tokio::sync::mpsc::UnboundedSender<ResizeProposal>>,
   543	) -> tokio::task::JoinHandle<()> {
   544	    let weak = Arc::downgrade(dial);
   545	    tokio::spawn(async move {
   546	        let mut last_blocked: u64 = 0;
   547	        let mut last_bytes: u64 = 0;
   548	        let mut last_tick = tokio::time::Instant::now();
   549	        loop {
   550	            tokio::time::sleep(DIAL_TUNER_TICK).await;
   551	            let Some(dial) = weak.upgrade() else { return };
   552	            let (blocked, bytes, streams) = {
   553	                let probes = probes.lock().expect("probe registry poisoned");
   554	                let (b, n) = probes.iter().fold((0u64, 0u64), |(b, n), p| {
   555	                    let snap = p.snapshot();
   556	                    (b + snap.write_blocked_nanos, n + snap.bytes_sent)
   557	                });
   558	                (b, n, probes.len())
   559	            };
   560	            let elapsed = last_tick.elapsed();
   561	            last_tick = tokio::time::Instant::now();
   562	            // A retired stream leaves the registry, so the monotonic
   563	            // sums can shrink across a REMOVE. Re-baseline and treat
   564	            // the tick as no-signal rather than reading a bogus delta.
   565	            if blocked < last_blocked || bytes < last_bytes {
   566	                last_blocked = blocked;
   567	                last_bytes = bytes;
   568	                if let Some(tx) = &resize_tx {
   569	                    let _ = tx; // no proposal possible on a no-signal tick
   570	                    dial.resize_tick(0, 0.0);
   571	                }
   572	                continue;
   573	            }
   574	            let delta_blocked = blocked.saturating_sub(last_blocked);
   575	            let delta_bytes = bytes.saturating_sub(last_bytes);
   576	            last_blocked = blocked;
   577	            last_bytes = bytes;
   578	            // codex ue-r2-1e F2: an idle tick (no bytes moved) is NO
   579	            // SIGNAL, not a clean pipe — stepping up during manifest /
   580	            // preparation stalls would ramp without evidence and break
   581	            // the conservative-start contract. ue-r2-2 review (panel
   582	            // F3): the idle tick must still reach `resize_tick` so a
   583	            // sustain streak cannot survive a stall — "consecutive
   584	            // busy ticks" means consecutive.
   585	            if delta_bytes == 0 {
   586	                if resize_tx.is_some() {
   587	                    dial.resize_tick(0, 0.0);
   588	                }
   589	                continue;
   590	            }
   591	            let ratio = blocked_ratio(delta_blocked, elapsed, streams);
   592	            dial.apply_tick(ratio);
   593	            if let Some(tx) = &resize_tx {
   594	                if let Some(proposal) = dial.resize_tick(delta_bytes, ratio) {
   595	                    if tx.send(proposal).is_err() {
   596	                        // Controller gone (transfer tearing down):
   597	                        // release the pending slot so the dial state
   598	                        // stays honest for late readers.
   599	                        dial.resize_settled(proposal.epoch, dial.live_streams(), false);
   600	                    }
   601	                }
   602	            }
   603	        }
   604	    })
   605	}
   606	
   607	#[cfg(test)]
   608	mod tests {
   609	    use super::*;
   610	
   611	    fn profile(max_streams: u32, max_chunk: u64, max_inflight: u64) -> CapacityProfile {
   612	        CapacityProfile {
   613	            cpu_cores: 0,
   614	            drain_class: 0,
   615	            load_percent: 0,
   616	            max_streams,
   617	            drain_rate_bytes_per_sec: 0,
   618	            max_chunk_bytes: max_chunk,
   619	            max_inflight_bytes: max_inflight,
   620	        }
   621	    }
   622	
   623	    #[test]
   624	    fn conservative_start_is_the_old_floor_tier() {
   625	        let dial = TransferDial::conservative();
   626	        assert_eq!(dial.chunk_bytes(), 16 * MIB);
   627	        assert_eq!(dial.prefetch_count(), 4);
   628	        assert_eq!(dial.tcp_buffer_bytes(), None);
   629	        assert_eq!(dial.initial_streams(), 4);
   630	        assert_eq!(dial.max_streams(), 8);
   631	    }
   632	
   633	    #[test]
   634	    fn unknown_profile_fields_keep_default_ceilings() {
   635	        let dial = TransferDial::conservative_within(Some(&profile(0, 0, 0)));
   636	        // Ramp fully: unknown (0) fields must not lower — or lift —
   637	        // anything relative to the defaults.
   638	        while dial.step_up_cheap_dials() {}
   639	        assert_eq!(dial.chunk_bytes(), DIAL_CEILING_CHUNK_BYTES);
   640	        assert_eq!(dial.prefetch_count(), DIAL_CEILING_PREFETCH);

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/control.rs | sed -n '320,620p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   320	                                        epoch0_sub_token: epoch0_sub,
   321	                                    },
   322	                                ),
   323	                            )
   324	                            .await?;
   325	
   326	                            data_plane_handle = Some(transfer_task);
   327	                        }
   328	                    }
   329	                }
   330	            }
   331	            Some(client_push_request::Payload::ManifestComplete(mc)) => {
   332	                // w4-4: drain the sub-chunk remainder before leaving the
   333	                // manifest phase — `need_list_sender.finish()` below and
   334	                // the post-manifest negotiation both need the complete
   335	                // need list / files_to_upload. No mid-manifest data-plane
   336	                // spin-up here: the post-manifest path owns negotiation
   337	                // once the manifest is done.
   338	                if !pending_manifest.is_empty() {
   339	                    let module_ref = module.as_ref().ok_or_else(|| {
   340	                        Status::failed_precondition("push manifest received before header")
   341	                    })?;
   342	                    drain_manifest_checks(
   343	                        module_ref,
   344	                        &mut pending_manifest,
   345	                        &mut need_list_sender,
   346	                        &mut files_to_upload,
   347	                    )
   348	                    .await?;
   349	                }
   350	                manifest_complete = true;
   351	                scan_complete = mc.scan_complete;
   352	                break;
   353	            }
   354	            Some(client_push_request::Payload::FileData(_)) => {
   355	                return Err(Status::failed_precondition(
   356	                    "data payload received before negotiation",
   357	                ));
   358	            }
   359	            Some(client_push_request::Payload::UploadComplete(_)) => {}
   360	            Some(client_push_request::Payload::DataPlaneResize(req)) => {
   361	                // ue-r2-2: an ADD can land while the manifest loop is
   362	                // still running (the data plane starts at the early
   363	                // flush) — same handling as the transfer phase.
   364	                handle_resize_request(&tx, &resize_cmd_tx, &mut resize_live, req).await?;
   365	            }
   366	            None => {}
   367	        }
   368	    }
   369	
   370	    let module = module.ok_or_else(|| Status::invalid_argument("push stream missing header"))?;
   371	    if !manifest_complete {
   372	        return Err(Status::invalid_argument(
   373	            "push stream ended before manifest completion",
   374	        ));
   375	    }
   376	
   377	    need_list_sender.finish().await?;
   378	
   379	    let force_grpc_effective = force_grpc_effective || force_grpc_client;
   380	
   381	    let transfer_stats = if files_to_upload.is_empty() {
   382	        TransferStats::default()
   383	    } else if force_grpc_effective {
   384	        fallback_used = true;
   385	        execute_grpc_fallback(&tx, &mut stream, &module, files_to_upload.clone()).await?
   386	    } else {
   387	        if data_plane_handle.is_none() {
   388	            let listener = bind_data_plane_listener()
   389	                .await
   390	                .map_err(|err| Status::internal(format!("failed to bind data plane: {}", err)))?;
   391	            let port = listener
   392	                .local_addr()
   393	                .map_err(|err| Status::internal(format!("querying listener addr: {}", err)))?
   394	                .port();
   395	            let token = generate_token()?;
   396	            let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);
   397	            let module_for_transfer = module.clone();
   398	            let stream_target = engine_stream_proposal(&files_to_upload);
   399	            // ue-r2-2: same fold as the early-flush site.
   400	            let resize_on = client_supports_resize;
   401	            let epoch0_sub = if resize_on {
   402	                generate_resize_sub_token()?
   403	            } else {
   404	                Vec::new()
   405	            };
   406	            let transfer_task = if resize_on {
   407	                let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
   408	                resize_cmd_tx = Some(cmd_tx);
   409	                resize_live = stream_target.max(1);
   410	                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream_resizable(
   411	                    listener,
   412	                    token.clone(),
   413	                    epoch0_sub.clone(),
   414	                    module_for_transfer,
   415	                    stream_target,
   416	                    cmd_rx,
   417	                )))
   418	            } else {
   419	                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream(
   420	                    listener,
   421	                    token.clone(),
   422	                    module_for_transfer,
   423	                    stream_target,
   424	                )))
   425	            };
   426	            send_control_message(
   427	                &tx,
   428	                server_push_response::Payload::Negotiation(DataTransferNegotiation {
   429	                    tcp_port: port as u32,
   430	                    one_time_token: token_string,
   431	                    tcp_fallback: false,
   432	                    stream_count: stream_target,
   433	                    // ue-r2-1e: see the early-flush negotiation above.
   434	                    receiver_capacity: Some(blit_core::engine::local_receiver_capacity()),
   435	                    resize_enabled: resize_on,
   436	                    epoch0_sub_token: epoch0_sub,
   437	                }),
   438	            )
   439	            .await?;
   440	            data_plane_handle = Some(transfer_task);
   441	        }
   442	
   443	        if let Some(handle) = data_plane_handle.take() {
   444	            // ue-r2-2: keep servicing the request stream while the data
   445	            // plane runs — the client's DataPlaneResize frames arrive
   446	            // mid-transfer. Everything else on the stream during this
   447	            // phase was previously unread; ignore it the same way.
   448	            //
   449	            // design-2 / w4-1: `handle.join()` is pinned across loop
   450	            // iterations rather than polling a bare `JoinHandle`
   451	            // directly — `AbortOnDrop::join` holds `self` across its
   452	            // internal await, so if `msg?` below errors and this
   453	            // function returns, dropping `join_fut` mid-poll drops the
   454	            // still-owned `AbortOnDrop`, which aborts the data-plane
   455	            // task instead of detaching it.
   456	            let mut client_stream_done = false;
   457	            let join_fut = handle.join();
   458	            tokio::pin!(join_fut);
   459	            loop {
   460	                tokio::select! {
   461	                    res = &mut join_fut => {
   462	                        break res.map_err(|_| Status::internal("data plane task cancelled"))??;
   463	                    }
   464	                    msg = stream.message(), if !client_stream_done => match msg? {
   465	                        Some(request) => {
   466	                            if let Some(client_push_request::Payload::DataPlaneResize(req)) =
   467	                                request.payload
   468	                            {
   469	                                handle_resize_request(&tx, &resize_cmd_tx, &mut resize_live, req).await?;
   470	                            }
   471	                        }
   472	                        None => client_stream_done = true,
   473	                    },
   474	                }
   475	            }
   476	        } else {
   477	            TransferStats::default()
   478	        }
   479	    };
   480	
   481	    let mut entries_deleted = 0u64;
   482	    if mirror_mode {
   483	        // R59 #1 F1: if the client demanded a complete source scan
   484	        // (mandatory for mirror), refuse to purge when the actual
   485	        // scan was incomplete. Pre-fix the daemon purged
   486	        // unconditionally, so a permission error mid-scan caused
   487	        // silent dest-side data loss when files absent from the
   488	        // (incomplete) manifest were deleted from destination.
   489	        if require_complete_scan && !scan_complete {
   490	            return Err(Status::failed_precondition(
   491	                "source scan was incomplete (unreadable paths); \
   492	                 refusing to purge destination to prevent data loss. \
   493	                 Resolve the unreadable source path(s) and retry.",
   494	            ));
   495	        }
   496	        // R59 #1 F2: choose the purge filter based on mirror_kind.
   497	        // ALL = full destination tree (no filter, historical
   498	        // behavior). FILTERED_SUBSET (default) = honor user's filter
   499	        // so out-of-scope destination entries aren't deleted.
   500	        let scoped_filter = match mirror_kind {
   501	            blit_core::generated::MirrorMode::All => blit_core::fs_enum::FileFilter::default(),
   502	            // FilteredSubset is the default for mirror_mode=true with
   503	            // an unspecified mirror_kind (back-compat: older clients
   504	            // that don't send the field still get the safe scope).
   505	            blit_core::generated::MirrorMode::Unspecified
   506	            | blit_core::generated::MirrorMode::FilteredSubset
   507	            | blit_core::generated::MirrorMode::Off => purge_filter.clone_without_cache(),
   508	        };
   509	        let purge_stats = purge_extraneous_entries(
   510	            module.path.clone(),
   511	            module.canonical_root.clone(),
   512	            expected_rel_files,
   513	            scoped_filter,
   514	        )
   515	        .await?;
   516	        entries_deleted = purge_stats.total();
   517	    }
   518	
   519	    send_control_message(
   520	        &tx,
   521	        server_push_response::Payload::Summary(PushSummary {
   522	            files_transferred: transfer_stats.files_transferred,
   523	            bytes_transferred: transfer_stats.bytes_transferred,
   524	            bytes_zero_copy: transfer_stats.bytes_zero_copy,
   525	            tcp_fallback_used: fallback_used,
   526	            entries_deleted,
   527	        }),
   528	    )
   529	    .await?;
   530	
   531	    Ok(())
   532	}
   533	
   534	/// ue-r2-2: 16 random bytes for the resize handshake suffix, minted
   535	/// beside the one-time token (`Status`-mapped like `generate_token`).
   536	fn generate_resize_sub_token() -> Result<Vec<u8>, Status> {
   537	    blit_core::remote::transfer::generate_sub_token()
   538	        .map_err(|err| Status::internal(format!("{err:#}")))
   539	}
   540	
   541	/// ue-r2-2: answer a client `DataPlaneResize`. ADD registers the
   542	/// epoch's credential with the acceptor (which arms exactly one
   543	/// accept, TTL-bounded) BEFORE the ack goes out, so the client's dial
   544	/// can never race an unarmed listener. REMOVE is accounting-only —
   545	/// the client retires a worker and that worker's END record tears the
   546	/// daemon-side stream down through the normal path. Refusals
   547	/// (`accepted: false`) cover: resize never negotiated, a malformed
   548	/// credential, a target beyond this daemon's advertised ceiling, a
   549	/// CUMULATIVE count at the ceiling (codex review: per-request target
   550	/// checks alone would let replayed ADDs with fresh credentials grow
   551	/// the worker set unboundedly — `resize_live` counts every armed ADD,
   552	/// conservatively including ones whose dial later lapses), or an
   553	/// acceptor that already finished.
   554	async fn handle_resize_request(
   555	    tx: &PushSender,
   556	    resize_cmd_tx: &Option<tokio::sync::mpsc::UnboundedSender<ResizeArm>>,
   557	    resize_live: &mut u32,
   558	    req: DataPlaneResize,
   559	) -> Result<(), Status> {
   560	    let op = DataPlaneResizeOp::try_from(req.op).unwrap_or(DataPlaneResizeOp::Unspecified);
   561	    let ceiling = blit_core::engine::local_receiver_capacity()
   562	        .max_streams
   563	        .max(1);
   564	    let within_ceiling = req.target_stream_count <= ceiling && *resize_live < ceiling;
   565	    let accepted = match (op, resize_cmd_tx) {
   566	        (DataPlaneResizeOp::Add, Some(cmd_tx)) => {
   567	            req.sub_token.len() == blit_core::remote::transfer::SUB_TOKEN_LEN
   568	                && within_ceiling
   569	                && cmd_tx
   570	                    .send(ResizeArm {
   571	                        epoch: req.epoch,
   572	                        sub_token: req.sub_token.clone(),
   573	                    })
   574	                    .is_ok()
   575	        }
   576	        (DataPlaneResizeOp::Remove, Some(_)) => true,
   577	        _ => false,
   578	    };
   579	    if accepted {
   580	        match op {
   581	            DataPlaneResizeOp::Add => *resize_live = resize_live.saturating_add(1),
   582	            DataPlaneResizeOp::Remove => *resize_live = resize_live.saturating_sub(1).max(1),
   583	            _ => {}
   584	        }
   585	    }
   586	    if !accepted {
   587	        log::warn!(
   588	            "push: refusing DataPlaneResize (op {:?}, epoch {}, target {})",
   589	            op,
   590	            req.epoch,
   591	            req.target_stream_count
   592	        );
   593	    }
   594	    send_control_message(
   595	        tx,
   596	        server_push_response::Payload::DataPlaneResizeAck(DataPlaneResizeAck {
   597	            epoch: req.epoch,
   598	            effective_stream_count: req.target_stream_count,
   599	            accepted,
   600	        }),
   601	    )
   602	    .await
   603	}
   604	
   605	struct FileListBatcher {
   606	    tx: PushSender,
   607	    batch: Vec<String>,
   608	    batch_bytes: usize,
   609	    sent_any: bool,
   610	    last_flush: Instant,
   611	}
   612	
   613	impl FileListBatcher {
   614	    fn new(tx: PushSender) -> Self {
   615	        Self {
   616	            tx,
   617	            batch: Vec::new(),
   618	            batch_bytes: 0,
   619	            sent_any: false,
   620	            last_flush: Instant::now(),

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/pull.rs | sed -n '980,1080p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   980	                        canonical_dest_root.as_deref(),
   981	                        &relative_path,
   982	                    )?;
   983	
   984	                    // Truncate file to the correct final size
   985	                    let file = tokio::fs::OpenOptions::new()
   986	                        .write(true)
   987	                        .open(&dest_path)
   988	                        .await
   989	                        .with_context(|| {
   990	                            format!("opening {} for truncation", dest_path.display())
   991	                        })?;
   992	
   993	                    file.set_len(complete.total_bytes).await.with_context(|| {
   994	                        format!(
   995	                            "truncating {} to {} bytes",
   996	                            dest_path.display(),
   997	                            complete.total_bytes
   998	                        )
   999	                    })?;
  1000	
  1001	                    if let Some(progress) = progress {
  1002	                        // Resumed file finished patching — count it on
  1003	                        // the per-file lane (block bytes already rode
  1004	                        // the BlockTransfer Payloads).
  1005	                        progress.report_file_complete(complete.relative_path.clone());
  1006	                    }
  1007	                    if track_paths {
  1008	                        report.downloaded_paths.push(relative_path);
  1009	                    }
  1010	                    report.files_transferred += 1;
  1011	                }
  1012	                Some(server_pull_message::Payload::DataPlaneResize(cmd)) => {
  1013	                    // ue-r2-2: the daemon (sender/controller on pull)
  1014	                    // wants to resize the stream set. ADD: forward the
  1015	                    // credential to the receiver task, which dials one
  1016	                    // more socket; REMOVE: passive — after this ack the
  1017	                    // daemon retires a sink whose END record ends one
  1018	                    // of our workers. The target is clamped to the
  1019	                    // ceiling this client itself advertises (the weak
  1020	                    // end protects itself receive-side too); a command
  1021	                    // on a session that never negotiated resize is
  1022	                    // refused, preserving the old peer-bug posture.
  1023	                    let op = crate::generated::DataPlaneResizeOp::try_from(cmd.op)
  1024	                        .unwrap_or(crate::generated::DataPlaneResizeOp::Unspecified);
  1025	                    let ceiling =
  1026	                        crate::engine::local_receiver_capacity().max_streams.max(1) as usize;
  1027	                    let within_ceiling = bounded_stream_count(cmd.target_stream_count)
  1028	                        == cmd.target_stream_count.max(1) as usize;
  1029	                    let accepted = match op {
  1030	                        crate::generated::DataPlaneResizeOp::Add => {
  1031	                            within_ceiling
  1032	                                // ue-r2-2 review (codex): bound the
  1033	                                // CUMULATIVE count too — replayed ADDs
  1034	                                // with fresh credentials must not grow
  1035	                                // the worker set past the ceiling.
  1036	                                && data_plane_live < ceiling
  1037	                                && cmd.sub_token.len() == crate::remote::transfer::SUB_TOKEN_LEN
  1038	                                && data_plane_growth.as_ref().is_some_and(|growth| {
  1039	                                    growth
  1040	                                        .send(PullStreamAdd {
  1041	                                            sub_token: cmd.sub_token.clone(),
  1042	                                        })
  1043	                                        .is_ok()
  1044	                                })
  1045	                        }
  1046	                        crate::generated::DataPlaneResizeOp::Remove => data_plane_growth.is_some(),
  1047	                        _ => false,
  1048	                    };
  1049	                    if accepted {
  1050	                        match op {
  1051	                            crate::generated::DataPlaneResizeOp::Add => data_plane_live += 1,
  1052	                            crate::generated::DataPlaneResizeOp::Remove => {
  1053	                                data_plane_live = data_plane_live.saturating_sub(1).max(1);
  1054	                            }
  1055	                            _ => {}
  1056	                        }
  1057	                    }
  1058	                    if !accepted {
  1059	                        log::warn!(
  1060	                            "pull: refusing DataPlaneResize (op {}, epoch {}, target {})",
  1061	                            cmd.op,
  1062	                            cmd.epoch,
  1063	                            cmd.target_stream_count
  1064	                        );
  1065	                    }
  1066	                    tx.send(ClientPullMessage {
  1067	                        payload: Some(client_pull_message::Payload::DataPlaneResizeAck(
  1068	                            crate::generated::DataPlaneResizeAck {
  1069	                                epoch: cmd.epoch,
  1070	                                effective_stream_count: cmd.target_stream_count,
  1071	                                accepted,
  1072	                            },
  1073	                        )),
  1074	                    })
  1075	                    .await
  1076	                    .map_err(|_| eyre!("failed to send resize ack"))?;
  1077	                }
  1078	                None => {}
  1079	            }
  1080	        }

codex
A second risk is emerging in the shape input itself: `requested_files` is also the pruning set and gets drained as files are queued. I’m checking the daemon need-list batching to see whether the new e2e actually exercises the intended accumulated-count path or only a larger outstanding batch.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/control.rs | sed -n '1,180p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use super::super::admin::purge_extraneous_entries;
     2	use super::super::util::{
     3	    metadata_mtime_seconds, resolve_manifest_relative_path, resolve_module, resolve_relative_path,
     4	};
     5	use super::super::PushSender;
     6	use super::data_plane::{
     7	    accept_data_connection_stream, accept_data_connection_stream_resizable,
     8	    bind_data_plane_listener, execute_grpc_fallback, generate_token, ResizeArm, TransferStats,
     9	};
    10	use crate::runtime::{ModuleConfig, RootExport};
    11	use base64::{engine::general_purpose, Engine as _};
    12	use blit_core::generated::{
    13	    client_push_request, server_push_response, Ack, ClientPushRequest, DataPlaneResize,
    14	    DataPlaneResizeAck, DataPlaneResizeOp, DataTransferNegotiation, FileHeader, FileList,
    15	    PushSummary, ServerPushResponse,
    16	};
    17	use blit_core::remote::transfer::AbortOnDrop;
    18	use std::collections::HashMap;
    19	use std::fs;
    20	use std::mem;
    21	use std::path::{Path, PathBuf};
    22	use std::sync::Arc;
    23	use std::time::{Duration, Instant};
    24	use tokio::sync::Mutex;
    25	use tonic::{Status, Streaming};
    26	
    27	const FILE_LIST_BATCH_MAX_ENTRIES: usize = 16 * 1024;
    28	const FILE_LIST_BATCH_MAX_BYTES: usize = 512 * 1024;
    29	const FILE_LIST_BATCH_MAX_DELAY: Duration = Duration::from_millis(25);
    30	const FILE_LIST_EARLY_FLUSH_ENTRIES: usize = 128;
    31	const FILE_LIST_EARLY_FLUSH_BYTES: usize = 64 * 1024;
    32	const FILE_LIST_EARLY_FLUSH_DELAY: Duration = Duration::from_millis(5);
    33	/// w4-4: manifest entries are buffered and their requires-upload
    34	/// checks (canonical containment + stat — 3+ blocking syscalls each)
    35	/// run in chunked `spawn_blocking` batches instead of inline on the
    36	/// runtime per entry. Sized to the need-list early-flush threshold so
    37	/// the reply cadence a fast-streaming push sees is unchanged; a
    38	/// trickling manifest (client still scanning) is covered by the
    39	/// delay trigger in [`manifest_drain_due`] instead — without it the
    40	/// batcher's own 64 KiB/5 ms early-flush triggers could never fire
    41	/// between chunk boundaries (codex w4-4 review, 1 Medium).
    42	const MANIFEST_CHECK_CHUNK: usize = FILE_LIST_EARLY_FLUSH_ENTRIES;
    43	
    44	/// w4-4 (codex review): when a buffered manifest entry has waited
    45	/// this long, drain the chunk even if it is not full — mirrors the
    46	/// batcher's `FILE_LIST_EARLY_FLUSH_DELAY` so a slowly-enumerating
    47	/// client still gets its first need-list (and mid-manifest TCP
    48	/// spin-up) within milliseconds, not after 128 entries trickle in.
    49	/// Under a fast manifest stream 128 entries arrive well inside this
    50	/// window, so the chunk cap dominates and syscall batching is kept.
    51	const MANIFEST_CHECK_MAX_DELAY: Duration = FILE_LIST_EARLY_FLUSH_DELAY;
    52	
    53	/// The two drain triggers for the buffered manifest checks: chunk
    54	/// full, or the oldest buffered entry has waited past the delay
    55	/// bound. Pure so the trigger contract is unit-testable.
    56	fn manifest_drain_due(pending_len: usize, oldest_buffered: Option<Instant>) -> bool {
    57	    pending_len >= MANIFEST_CHECK_CHUNK
    58	        || matches!(oldest_buffered, Some(t) if t.elapsed() >= MANIFEST_CHECK_MAX_DELAY)
    59	}
    60	
    61	pub(crate) async fn handle_push_stream(
    62	    modules: Arc<Mutex<HashMap<String, ModuleConfig>>>,
    63	    default_root: Option<RootExport>,
    64	    mut stream: Streaming<ClientPushRequest>,
    65	    tx: PushSender,
    66	    force_grpc_data: bool,
    67	    active_job: &crate::active_jobs::ActiveJobGuard,
    68	) -> Result<(), Status> {
    69	    let mut module: Option<ModuleConfig> = None;
    70	    let mut files_to_upload: Vec<FileHeader> = Vec::new();
    71	    let mut manifest_complete = false;
    72	    let mut mirror_mode = false;
    73	    let mut expected_rel_files: Vec<PathBuf> = Vec::new();
    74	    let mut force_grpc_client = false;
    75	    // R59 #1 F1/F2: state captured from PushHeader + ManifestComplete
    76	    // so the purge phase can refuse on a partial scan (F1) and
    77	    // honor the user's filter scope (F2).
    78	    let mut require_complete_scan = false;
    79	    let mut mirror_kind = blit_core::generated::MirrorMode::Unspecified;
    80	    let mut purge_filter = blit_core::fs_enum::FileFilter::default();
    81	    let mut scan_complete = false;
    82	    let mut need_list_sender = FileListBatcher::new(tx.clone());
    83	    // w4-4: manifest entries awaiting their chunked requires-upload
    84	    // check (see MANIFEST_CHECK_CHUNK / drain_manifest_checks), and
    85	    // when the oldest of them was buffered (drives the delay trigger;
    86	    // evaluated on the next arrival, matching the batcher's own
    87	    // push-time flush semantics).
    88	    let mut pending_manifest: Vec<PendingManifestEntry> = Vec::new();
    89	    let mut manifest_buffered_at: Option<Instant> = None;
    90	    // design-2 / w4-1: `AbortOnDrop`, not a bare `JoinHandle` — an
    91	    // early `?` return anywhere in this handler while a data-plane
    92	    // task is running (or the `stream.message()` race below erroring)
    93	    // must abort the accept/receive task instead of detaching it.
    94	    let mut data_plane_handle: Option<AbortOnDrop<Result<TransferStats, Status>>> = None;
    95	    let mut force_grpc_effective = force_grpc_data;
    96	    let mut fallback_used = false;
    97	    // ue-r2-2: the client's advertised resize capability (PushHeader
    98	    // bit) and, once a resize-enabled TCP negotiation is out, the
    99	    // channel that arms the acceptor for each ADD epoch.
   100	    let mut client_supports_resize = false;
   101	    let mut resize_cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<ResizeArm>> = None;
   102	    // ue-r2-2 review (codex): cumulative armed-stream count, seeded at
   103	    // negotiation - the ADD refusal bound.
   104	    let mut resize_live: u32 = 0;
   105	
   106	    while let Some(request) = stream.message().await? {
   107	        match request.payload {
   108	            Some(client_push_request::Payload::Header(header)) => {
   109	                if module.is_some() {
   110	                    return Err(Status::invalid_argument("duplicate push header received"));
   111	                }
   112	                // Populate the ActiveJobs row now that we know
   113	                // the endpoint (b-2-set-endpoint). The wire
   114	                // `destination_path` is what the user supplied;
   115	                // we record it verbatim — containment is
   116	                // verified below when joining onto the module
   117	                // root.
   118	                active_job.set_endpoint(header.module.clone(), header.destination_path.clone());
   119	                let mut config =
   120	                    resolve_module(&modules, default_root.as_ref(), &header.module).await?;
   121	                if config.read_only {
   122	                    return Err(Status::permission_denied(format!(
   123	                        "module '{}' is read-only",
   124	                        config.name
   125	                    )));
   126	                }
   127	                mirror_mode = header.mirror_mode;
   128	                force_grpc_client = header.force_grpc;
   129	                force_grpc_effective = force_grpc_data || force_grpc_client;
   130	                // ue-r2-2: fold input (a) of the resize gate — the
   131	                // peer's capability bit. (b) own support and (c)/(d)
   132	                // the live-TCP conditions fold in at the negotiation
   133	                // literals, which only exist on the TCP path.
   134	                client_supports_resize = header.supports_stream_resize;
   135	                // R59 #1: capture F1 / F2 fields from the new wire shape.
   136	                require_complete_scan = header.require_complete_scan;
   137	                mirror_kind = blit_core::generated::MirrorMode::try_from(header.mirror_kind)
   138	                    .unwrap_or(blit_core::generated::MirrorMode::Unspecified);
   139	                if let Some(wire_filter) = header.filter.as_ref() {
   140	                    let mut f = blit_core::fs_enum::FileFilter::default();
   141	                    f.include_files = wire_filter.include.clone();
   142	                    f.exclude_files = wire_filter.exclude.clone();
   143	                    f.min_size = wire_filter.min_size;
   144	                    f.max_size = wire_filter.max_size;
   145	                    f.min_age = wire_filter.min_age_secs.map(std::time::Duration::from_secs);
   146	                    f.max_age = wire_filter.max_age_secs.map(std::time::Duration::from_secs);
   147	                    f.reference_time = Some(std::time::SystemTime::now());
   148	                    f.files_from = if wire_filter.files_from.is_empty() {
   149	                        None
   150	                    } else {
   151	                        Some(wire_filter.files_from.iter().map(PathBuf::from).collect())
   152	                    };
   153	                    purge_filter = f;
   154	                }
   155	                let dest_path = header.destination_path.trim();
   156	                if !dest_path.is_empty() {
   157	                    let rel = resolve_relative_path(dest_path)?;
   158	                    let new_path = config.path.join(rel);
   159	                    // F2 / R13-F1: verify the rewritten module path
   160	                    // stays inside the canonical module root before
   161	                    // any downstream operation runs against it. Without
   162	                    // this, a destination_path traversing an in-module
   163	                    // symlink to outside would have all subsequent
   164	                    // ops (file writes, mirror-purge enumeration)
   165	                    // operate outside the module. Per-file write paths
   166	                    // are already individually checked, but mirror
   167	                    // purge enumerates module.path before any per-file
   168	                    // check can fire.
   169	                    blit_core::path_safety::verify_contained(&config.canonical_root, &new_path)
   170	                        .map_err(|e| {
   171	                            Status::permission_denied(format!(
   172	                                "destination path containment: {e:#}"
   173	                            ))
   174	                        })?;
   175	                    config.path = new_path;
   176	                }
   177	                module = Some(config);
   178	                send_control_message(&tx, server_push_response::Payload::Ack(Ack {})).await?;
   179	            }
   180	            Some(

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/control.rs | sed -n '605,760p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   605	struct FileListBatcher {
   606	    tx: PushSender,
   607	    batch: Vec<String>,
   608	    batch_bytes: usize,
   609	    sent_any: bool,
   610	    last_flush: Instant,
   611	}
   612	
   613	impl FileListBatcher {
   614	    fn new(tx: PushSender) -> Self {
   615	        Self {
   616	            tx,
   617	            batch: Vec::new(),
   618	            batch_bytes: 0,
   619	            sent_any: false,
   620	            last_flush: Instant::now(),
   621	        }
   622	    }
   623	
   624	    async fn push(&mut self, path: String) -> Result<bool, Status> {
   625	        let entry_bytes = path.len();
   626	        if self.batch.is_empty() {
   627	            self.last_flush = Instant::now();
   628	        }
   629	
   630	        self.batch_bytes = self.batch_bytes.saturating_add(entry_bytes + 1);
   631	        self.batch.push(path);
   632	
   633	        if self.should_flush() {
   634	            self.flush().await?;
   635	            Ok(true)
   636	        } else {
   637	            Ok(false)
   638	        }
   639	    }
   640	
   641	    async fn flush(&mut self) -> Result<(), Status> {
   642	        if self.batch.is_empty() {
   643	            return Ok(());
   644	        }
   645	
   646	        self.sent_any = true;
   647	        let payload = server_push_response::Payload::FilesToUpload(FileList {
   648	            relative_paths: mem::take(&mut self.batch),
   649	        });
   650	        self.batch_bytes = 0;
   651	        self.last_flush = Instant::now();
   652	        send_control_message(&self.tx, payload).await
   653	    }
   654	
   655	    async fn finish(mut self) -> Result<(), Status> {
   656	        if !self.batch.is_empty() {
   657	            self.flush().await?;
   658	        }
   659	        // Always emit an empty FilesToUpload terminator so the client
   660	        // can distinguish "more need_lists may arrive" from "no more
   661	        // coming". Without this, the client races between its early-
   662	        // finish condition (looks complete) and the daemon still
   663	        // streaming batches — closes the data plane prematurely and
   664	        // late manifest entries can't be queued.
   665	        send_control_message(
   666	            &self.tx,
   667	            server_push_response::Payload::FilesToUpload(FileList {
   668	                relative_paths: Vec::new(),
   669	            }),
   670	        )
   671	        .await?;
   672	        Ok(())
   673	    }
   674	
   675	    fn should_flush(&self) -> bool {
   676	        if self.batch.is_empty() {
   677	            return false;
   678	        }
   679	
   680	        if !self.sent_any
   681	            && (self.batch.len() >= FILE_LIST_EARLY_FLUSH_ENTRIES
   682	                || self.batch_bytes >= FILE_LIST_EARLY_FLUSH_BYTES
   683	                || self.last_flush.elapsed() >= FILE_LIST_EARLY_FLUSH_DELAY)
   684	        {
   685	            return true;
   686	        }
   687	
   688	        self.batch.len() >= FILE_LIST_BATCH_MAX_ENTRIES
   689	            || self.batch_bytes >= FILE_LIST_BATCH_MAX_BYTES
   690	            || self.last_flush.elapsed() >= FILE_LIST_BATCH_MAX_DELAY
   691	    }
   692	}
   693	
   694	pub(super) async fn send_control_message(
   695	    tx: &PushSender,
   696	    payload: server_push_response::Payload,
   697	) -> Result<(), Status> {
   698	    tx.send(Ok(ServerPushResponse {
   699	        payload: Some(payload),
   700	    }))
   701	    .await
   702	    .map_err(|_| Status::internal("failed to send push response"))
   703	}
   704	
   705	/// w4-4: one manifest entry buffered for the chunked requires-upload
   706	/// check. `rel` is the validated relative path (containment input),
   707	/// `sanitized` its canonical POSIX wire form (need-list echo), `file`
   708	/// the header (already rewritten to the sanitized path) queued for
   709	/// upload if the check says so.
   710	struct PendingManifestEntry {
   711	    rel: PathBuf,
   712	    sanitized: String,
   713	    file: FileHeader,
   714	}
   715	
   716	/// w4-4: run the buffered entries' requires-upload checks in ONE
   717	/// `spawn_blocking` call (each check is a canonical-containment
   718	/// ancestor walk plus a stat — blocking syscalls that used to run
   719	/// per-entry on the runtime), then feed the need list in the original
   720	/// manifest order. Returns true if any need-list push flushed a batch
   721	/// to the client (the caller's cue to spin up the data plane
   722	/// mid-manifest on the TCP path).
   723	async fn drain_manifest_checks(
   724	    module: &ModuleConfig,
   725	    pending: &mut Vec<PendingManifestEntry>,
   726	    need_list: &mut FileListBatcher,
   727	    files_to_upload: &mut Vec<FileHeader>,
   728	) -> Result<bool, Status> {
   729	    if pending.is_empty() {
   730	        return Ok(false);
   731	    }
   732	    let batch = mem::take(pending);
   733	    let module_for_check = module.clone();
   734	    let (batch, decisions) = tokio::task::spawn_blocking(move || {
   735	        let decisions: Result<Vec<bool>, Status> = batch
   736	            .iter()
   737	            .map(|entry| file_requires_upload(&module_for_check, &entry.rel, &entry.file))
   738	            .collect();
   739	        (batch, decisions)
   740	    })
   741	    .await
   742	    .map_err(|err| Status::internal(format!("manifest check task failed: {err}")))?;
   743	    let decisions = decisions?;
   744	
   745	    let mut any_flushed = false;
   746	    for (entry, requires_upload) in batch.into_iter().zip(decisions) {
   747	        if requires_upload {
   748	            // w4-2: the 262,144-slot upload channel that used to sit
   749	            // here is gone. Headers travel on the wire post-Phase-5;
   750	            // the TCP receiver drained it into the void, and in gRPC
   751	            // fallback nothing read it at all — so manifest entry
   752	            // #262,145 wedged daemon and client forever with no
   753	            // timeout in scope.
   754	            // w5-1: was an unconditional per-file eprintln — stderr
   755	            // spam proportional to file count. Debug-level now;
   756	            // visible with BLIT_LOG=debug.
   757	            log::debug!("push server queued {}", entry.sanitized);
   758	            let flushed = need_list.push(entry.sanitized).await?;
   759	            any_flushed = any_flushed || flushed;
   760	            files_to_upload.push(entry.file);

exec
/usr/bin/zsh -lc 'rg -n "requested_files|files_requested|transfer_size_hint|pending_queue|prune_unrequested_payloads" crates/blit-core/src/remote/push/client/mod.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
547:fn prune_unrequested_payloads(
635:        let mut requested_files: HashSet<String> = HashSet::new();
639:        let mut transfer_size_hint: u64 = 0;
708:        let mut files_requested: Vec<String> = Vec::new();
709:        let mut pending_queue: VecDeque<String> = VecDeque::new();
790:                                    files_requested.extend(rels.iter().cloned());
794:                                        requested_files.insert(rel.clone());
805:                                    pending_queue.extend(rels.drain(..));
806:                                    transfer_size_hint =
807:                                        transfer_size_hint.saturating_add(batch_bytes);
826:                                            // in pending_queue (drained by the
835:                                                    &mut pending_queue,
868:                                                        transfer_size_hint,
869:                                                        requested_files.len(),
884:                                                    drain_pending_headers(&mut pending_queue, &manifest_lookup);
957:                                                &mut pending_queue,
1041:                                                    transfer_size_hint,
1042:                                                    requested_files.len(),
1058:                                                drain_pending_headers(&mut pending_queue, &manifest_lookup);
1071:                                            let skipped = prune_unrequested_payloads(
1073:                                                &mut requested_files,
1174:                                                    transfer_size_hint,
1175:                                                    requested_files.len(),
1251:                                            &mut pending_queue,
1274:                                            drain_pending_headers(&mut pending_queue, &manifest_lookup);
1289:                                            let skipped = prune_unrequested_payloads(
1291:                                                &mut requested_files,
1452:                && pending_queue.is_empty()
1453:                && (files_requested.is_empty() || fallback_files_sent >= files_requested.len())
1469:                && pending_queue.is_empty()
1472:                && data_plane_files_sent >= files_requested.len()
1512:            files_requested,
1527:    pending_queue: &mut VecDeque<String>,
1536:    let headers = drain_pending_headers(pending_queue, manifest_lookup);

exec
/usr/bin/zsh -lc 'git show c70c2ac''^:crates/blit-core/src/remote/push/client/mod.rs | nl -ba | sed -n '"'660,1120p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   660	        // hold its fallback payloads until the daemon announces
   661	        // Negotiation(tcp_fallback) — which the daemon only sends after it
   662	        // has seen ManifestComplete. Pre-fix, force_grpc initialized
   663	        // Fallback mode and the first mid-manifest need-list batch
   664	        // triggered FileData sends that raced the daemon's manifest loop:
   665	        // every forced-gRPC push of ≥128 files (one early need-list flush)
   666	        // died, and ~100 files was a coin flip.
   667	        let mut fallback_negotiated = false;
   668	
   669	        // ue-r2-2: resize controller state. The tuner's proposal stream
   670	        // appears once a resize-enabled negotiation lands;
   671	        // `resize_pending` is the single epoch awaiting the daemon's
   672	        // ack (the dial enforces one-in-flight too).
   673	        let mut resize_proposal_rx: Option<
   674	            tokio::sync::mpsc::UnboundedReceiver<crate::engine::ResizeProposal>,
   675	        > = None;
   676	        let mut resize_pending: Option<PendingResize> = None;
   677	
   678	        let mut manifest_done = false;
   679	        // Track whether we received new need-list entries this iteration.
   680	        // Don't finish the data plane until a full iteration passes with
   681	        // no new entries — this ensures all in-flight gRPC batches arrive.
   682	        let mut need_list_fresh: bool;
   683	        // Set when the daemon signals "no more need_lists coming" by
   684	        // sending an empty FilesToUpload terminator. Gates the early
   685	        // finish() so we don't close the data plane while the daemon
   686	        // is still streaming need_list batches.
   687	        let mut need_lists_done = false;
   688	        loop {
   689	            if manifest_done && summary.is_some() {
   690	                break;
   691	            }
   692	            need_list_fresh = false;
   693	
   694	            tokio::select! {
   695	                biased;
   696	
   697	                maybe_message = response_rx.recv() => {
   698	                    match maybe_message {
   699	                        Some(Ok(message)) => {
   700	                            match message.payload {
   701	                                Some(ServerPayload::Ack(_)) => {}
   702	                                Some(ServerPayload::FilesToUpload(list)) => {
   703	                                    if list.relative_paths.is_empty() {
   704	                                        // Empty terminator — no more need_lists coming.
   705	                                        // Fall through to the bottom of the loop so the
   706	                                        // early-finish check can fire on this iteration;
   707	                                        // don't `continue` (that would skip the check
   708	                                        // and require another response message to wake
   709	                                        // the select, which never arrives).
   710	                                        need_lists_done = true;
   711	                                    } else {
   712	                                    need_list_fresh = true;
   713	                                    let mut rels = list.relative_paths;
   714	                                    files_requested.extend(rels.iter().cloned());
   715	                                    let newly_requested = rels.len();
   716	                                    let mut batch_bytes = 0u64;
   717	                                    for rel in &rels {
   718	                                        requested_files.insert(rel.clone());
   719	                                        if let Some(header) = manifest_lookup.get(rel) {
   720	                                            batch_bytes =
   721	                                                batch_bytes.saturating_add(header.size);
   722	                                        }
   723	                                        // w5-1: was an unconditional per-file
   724	                                        // eprintln — stderr spam proportional
   725	                                        // to file count. Debug-level now;
   726	                                        // visible with BLIT_LOG=debug.
   727	                                        log::debug!("push need-list includes {}", rel);
   728	                                    }
   729	                                    pending_queue.extend(rels.drain(..));
   730	                                    transfer_size_hint =
   731	                                        transfer_size_hint.saturating_add(batch_bytes);
   732	                                    need_list_received = true;
   733	
   734	                                    if !matches!(transfer_mode, TransferMode::Fallback) {
   735	                                        data_plane_outstanding =
   736	                                            data_plane_outstanding.saturating_add(newly_requested);
   737	                                    }
   738	
   739	                                    if let Some(progress) = progress {
   740	                                        if newly_requested > 0 {
   741	                                            progress.report_manifest_batch(newly_requested);
   742	                                        }
   743	                                    }
   744	
   745	                                    match transfer_mode {
   746	                                        TransferMode::Fallback => {
   747	                                            // design-4: hold payloads until the
   748	                                            // daemon's fallback negotiation;
   749	                                            // until then entries just accumulate
   750	                                            // in pending_queue (drained by the
   751	                                            // Negotiation arm).
   752	                                            if fallback_negotiated && need_list_received {
   753	                                                let dial = ensure_dial(
   754	                                                    &mut dial,
   755	                                                    None,
   756	                                                );
   757	                                                let result = stream_fallback_from_queue(
   758	                                                    source.clone(),
   759	                                                    &mut pending_queue,
   760	                                                    &manifest_lookup,
   761	                                                    &tx,
   762	                                                    progress,
   763	                                                    plan_options,
   764	                                                    dial.chunk_bytes(),
   765	                                                    dial.initial_streams(),
   766	                                                    &unreadable_paths,
   767	                                                ).await?;
   768	                                                if result.files_sent > 0 {
   769	                                                    fallback_files_sent =
   770	                                                        fallback_files_sent.saturating_add(result.files_sent);
   771	                                                }
   772	                                                if result.payloads_dispatched
   773	                                                    && first_payload_elapsed.is_none()
   774	                                                {
   775	                                                    first_payload_elapsed = Some(start.elapsed());
   776	                                                }
   777	                                            }
   778	                                        }
   779	                                        TransferMode::DataPlane => {
   780	                                            if let Some(sender) = data_plane_sender.as_mut() {
   781	                                                let headers =
   782	                                                    drain_pending_headers(&mut pending_queue, &manifest_lookup);
   783	                                                if !headers.is_empty() {
   784	                                                    let headers = source.check_availability(
   785	                                                        headers,
   786	                                                        Arc::clone(&unreadable_paths),
   787	                                                    )
   788	                                                    .await?;
   789	                                                    if headers.is_empty() {
   790	                                                        continue;
   791	                                                    }
   792	                                                    // Dial exists before the first
   793	                                                    // data-plane batch (first-wins).
   794	                                                    ensure_dial(&mut dial, None);
   795	                                            let planned =
   796	                                                plan_transfer_payloads(headers, source_root, plan_options)?;
   797	                                            for payload in &planned {
   798	                                                match payload {
   799	                                                    TransferPayload::File(header) => {
   800	                                                        // w5-1: was unconditional per-file
   801	                                                        // eprintln; BLIT_LOG=debug shows it.
   802	                                                        log::debug!(
   803	                                                            "push enqueue {} for TCP stream",
   804	                                                            header.relative_path
   805	                                                        );
   806	                                                    }
   807	                                                    TransferPayload::TarShard { headers } => {
   808	                                                        for header in headers {
   809	                                                            log::debug!(
   810	                                                                "push enqueue {} via tar shard",
   811	                                                                header.relative_path
   812	                                                            );
   813	                                                        }
   814	                                                    }
   815	                                                    TransferPayload::FileBlock { .. }
   816	                                                    | TransferPayload::FileBlockComplete { .. } => {
   817	                                                        // Receive-only — never produced by the outbound planner.
   818	                                                    }
   819	                                                }
   820	                                            }
   821	                                            if !planned.is_empty() {
   822	                                                        let sent = payload_file_count(&planned);
   823	                                                        sender.queue(planned).await?;
   824	                                                        if sent > 0 && first_payload_elapsed.is_none() {
   825	                                                            first_payload_elapsed = Some(start.elapsed());
   826	                                                        }
   827	                                                        data_plane_files_sent += sent;
   828	                                                        data_plane_outstanding =
   829	                                                            data_plane_outstanding.saturating_sub(sent);
   830	                                                    }
   831	                                                }
   832	                                            }
   833	                                        }
   834	                                        TransferMode::Undecided => {}
   835	                                    }
   836	                                    } // end else (non-empty need_list)
   837	                                }
   838	                                Some(ServerPayload::Negotiation(neg)) => {
   839	                                    if neg.tcp_fallback {
   840	                                        fallback_used = true;
   841	                                        transfer_mode = TransferMode::Fallback;
   842	                                        // design-4: only now may fallback
   843	                                        // payloads flow — the daemon is past
   844	                                        // its manifest loop and ready to
   845	                                        // receive FileData.
   846	                                        fallback_negotiated = true;
   847	
   848	                                            if need_list_received {
   849	                                            let dial = ensure_dial(
   850	                                                &mut dial,
   851	                                                neg.receiver_capacity.as_ref(),
   852	                                            );
   853	                                            let result = stream_fallback_from_queue(
   854	                                                source.clone(),
   855	                                                &mut pending_queue,
   856	                                                &manifest_lookup,
   857	                                                &tx,
   858	                                                progress,
   859	                                                plan_options,
   860	                                                dial.chunk_bytes(),
   861	                                                dial.prefetch_count(),
   862	                                                &unreadable_paths,
   863	                                            ).await?;
   864	                                            if result.files_sent > 0 {
   865	                                                fallback_files_sent =
   866	                                                    fallback_files_sent.saturating_add(result.files_sent);
   867	                                            }
   868	                                            if result.payloads_dispatched
   869	                                                && first_payload_elapsed.is_none()
   870	                                            {
   871	                                                first_payload_elapsed = Some(start.elapsed());
   872	                                            }
   873	                                        }
   874	
   875	                                        data_plane_outstanding = 0;
   876	                                        if let Some(sender) = data_plane_sender.take() {
   877	                                            sender.finish().await?;
   878	                                        }
   879	                                    } else {
   880	                                        if neg.tcp_port == 0 {
   881	                                            eyre::bail!("server reported zero data port for negotiated transfer");
   882	                                        }
   883	
   884	                                        let token_bytes = decode_token(&neg.one_time_token)?;
   885	                                        // ue-r2-1e: the daemon (byte
   886	                                        // receiver) advertised its profile
   887	                                        // on this negotiation — the dial's
   888	                                        // ceilings honor it (first-wins,
   889	                                        // like the old tuning memo).
   890	                                        let dial = ensure_dial(
   891	                                            &mut dial,
   892	                                            neg.receiver_capacity.as_ref(),
   893	                                        );
   894	                                        if data_plane_sender.is_none() {
   895	                                            let stream_target = dial.set_negotiated_streams(
   896	                                                neg.stream_count.max(1) as usize,
   897	                                            );
   898	                                            let payload_prefetch = dial.prefetch_count();
   899	                                            // ue-r2-2: the daemon's fold said
   900	                                            // resize is on for this transfer —
   901	                                            // epoch-0 sockets carry the
   902	                                            // sub-token suffix and the sender
   903	                                            // goes elastic. A malformed token
   904	                                            // length reads as "not enabled"
   905	                                            // (fail toward today's behavior).
   906	                                            let resize_sub = (neg.resize_enabled
   907	                                                && neg.epoch0_sub_token.len()
   908	                                                    == crate::remote::transfer::SUB_TOKEN_LEN)
   909	                                                .then(|| neg.epoch0_sub_token.clone());
   910	                                            let mut sender = MultiStreamSender::connect(
   911	                                                &self.endpoint.host,
   912	                                                neg.tcp_port,
   913	                                                &token_bytes,
   914	                                                dial.chunk_bytes(),
   915	                                                payload_prefetch,
   916	                                                stream_target,
   917	                                                trace_data_plane,
   918	                                                source.clone(),
   919	                                                dial.tcp_buffer_bytes(),
   920	                                                progress.cloned(),
   921	                                                Some(dial.clone()),
   922	                                                resize_sub,
   923	                                            )
   924	                                            .await?;
   925	                                            resize_proposal_rx = sender.take_resize_rx();
   926	                                            data_plane_sender = Some(sender);
   927	                                            data_port = Some(neg.tcp_port);
   928	                                        }
   929	
   930	                                        if let Some(sender) = data_plane_sender.as_mut() {
   931	                                            let headers =
   932	                                                drain_pending_headers(&mut pending_queue, &manifest_lookup);
   933	                                            if !headers.is_empty() {
   934	                                                let headers = source
   935	                                                    .check_availability(headers, unreadable_paths.clone())
   936	                                                    .await?;
   937	                                                if headers.is_empty() {
   938	                                                    continue;
   939	                                                }
   940	                                            let mut planned = plan_transfer_payloads(
   941	                                                headers,
   942	                                                source_root,
   943	                                                plan_options,
   944	                                            )?;
   945	                                            let skipped = prune_unrequested_payloads(
   946	                                                &mut planned,
   947	                                                &mut requested_files,
   948	                                            );
   949	                                            if skipped > 0 {
   950	                                                log::debug!(
   951	                                                    "push: daemon did not request {} payload file(s); skipping",
   952	                                                    skipped
   953	                                                );
   954	                                            }
   955	                                            if !planned.is_empty() {
   956	                                                let sent = payload_file_count(&planned);
   957	                                                sender.queue(planned).await?;
   958	                                                if sent > 0 && first_payload_elapsed.is_none() {
   959	                                                    first_payload_elapsed = Some(start.elapsed());
   960	                                                }
   961	                                                data_plane_files_sent += sent;
   962	                                                data_plane_outstanding =
   963	                                                    data_plane_outstanding.saturating_sub(sent);
   964	                                                }
   965	                                            }
   966	                                        }
   967	                                        transfer_mode = TransferMode::DataPlane;
   968	                                    }
   969	                                }
   970	                                Some(ServerPayload::Summary(push_summary)) => {
   971	                                    summary = Some(push_summary);
   972	                                }
   973	                                Some(ServerPayload::DataPlaneResizeAck(ack)) => {
   974	                                    // ue-r2-2: settle the in-flight epoch with
   975	                                    // what actually happened. An unsolicited or
   976	                                    // stale ack is ignored exactly as before.
   977	                                    match resize_pending.take() {
   978	                                        Some(pending) if ack.epoch == pending.epoch => {
   979	                                            let dial_ref = dial
   980	                                                .as_ref()
   981	                                                .expect("resize only negotiated on the dial path");
   982	                                            if pending.add && ack.accepted {
   983	                                                // Daemon armed the accept —
   984	                                                // dial the new socket. A failed
   985	                                                // dial must NOT kill a healthy
   986	                                                // transfer: the armed slot
   987	                                                // expires daemon-side and the
   988	                                                // live count simply stands.
   989	                                                let added = match data_plane_sender.as_mut() {
   990	                                                    Some(sender) => {
   991	                                                        match sender
   992	                                                            .add_stream(&pending.sub_token)
   993	                                                            .await
   994	                                                        {
   995	                                                            Ok(()) => true,
   996	                                                            Err(err) => {
   997	                                                                log::warn!(
   998	                                                                    "resize ADD (epoch {}) dial \
   999	                                                                     failed; continuing at the \
  1000	                                                                     current stream count: {err:#}",
  1001	                                                                    pending.epoch
  1002	                                                                );
  1003	                                                                false
  1004	                                                            }
  1005	                                                        }
  1006	                                                    }
  1007	                                                    None => false,
  1008	                                                };
  1009	                                                if added {
  1010	                                                    dial_ref.resize_settled(
  1011	                                                        pending.epoch,
  1012	                                                        pending.target,
  1013	                                                        true,
  1014	                                                    );
  1015	                                                } else {
  1016	                                                    dial_ref.resize_settled(
  1017	                                                        pending.epoch,
  1018	                                                        dial_ref.live_streams(),
  1019	                                                        true,
  1020	                                                    );
  1021	                                                }
  1022	                                            } else if !pending.add && ack.accepted {
  1023	                                                dial_ref.resize_settled(
  1024	                                                    pending.epoch,
  1025	                                                    pending.target,
  1026	                                                    true,
  1027	                                                );
  1028	                                            } else {
  1029	                                                dial_ref.resize_settled(
  1030	                                                    pending.epoch,
  1031	                                                    dial_ref.live_streams(),
  1032	                                                    false,
  1033	                                                );
  1034	                                            }
  1035	                                        }
  1036	                                        other => {
  1037	                                            resize_pending = other;
  1038	                                            log::debug!(
  1039	                                                "ignoring unsolicited/stale DataPlaneResizeAck \
  1040	                                                 (epoch {})",
  1041	                                                ack.epoch
  1042	                                            );
  1043	                                        }
  1044	                                    }
  1045	                                }
  1046	                                None => {}
  1047	                            }
  1048	                        }
  1049	                        Some(Err(err)) => return Err(err),
  1050	                        None => break,
  1051	                    }
  1052	                }
  1053	                maybe_header = manifest_rx.recv(), if !manifest_done => {
  1054	                    match maybe_header {
  1055	                        Some(header) => {
  1056	                            // Normalize path to ensure consistency with server requests
  1057	                            let rel = if header.relative_path.starts_with("./") {
  1058	                                header.relative_path[2..].to_string()
  1059	                            } else {
  1060	                                header.relative_path.clone()
  1061	                            };
  1062	                            let mut header = header;
  1063	                            header.relative_path = rel.clone();
  1064	
  1065	                            // Check availability via the source abstraction
  1066	                            let available = source.check_availability(vec![header.clone()], Arc::clone(&unreadable_paths)).await?;
  1067	                            if available.is_empty() {
  1068	                                continue;
  1069	                            }
  1070	
  1071	                            manifest_total_bytes =
  1072	                                manifest_total_bytes.saturating_add(header.size);
  1073	                            // design-5: if the daemon already rejected the
  1074	                            // push (e.g. read-only module), this send loses
  1075	                            // a race with the terminal status — surface the
  1076	                            // daemon's reason, not the transport symptom.
  1077	                            if let Err(send_err) =
  1078	                                send_payload(&tx, ClientPayload::FileManifest(header.clone()))
  1079	                                    .await
  1080	                            {
  1081	                                return Err(
  1082	                                    prefer_server_error(&mut response_rx, send_err).await
  1083	                                );
  1084	                            }
  1085	                            manifest_lookup.insert(rel.clone(), header);
  1086	
  1087	                            match transfer_mode {
  1088	                                TransferMode::Fallback => {
  1089	                                    // design-4: never interleave FileData
  1090	                                    // between our own manifest sends — wait
  1091	                                    // for the daemon's fallback negotiation.
  1092	                                    if fallback_negotiated && need_list_received {
  1093	                                        let dial = ensure_dial(
  1094	                                            &mut dial,
  1095	                                            None,
  1096	                                        );
  1097	                                        let result = stream_fallback_from_queue(
  1098	                                            source.clone(),
  1099	                                            &mut pending_queue,
  1100	                                            &manifest_lookup,
  1101	                                            &tx,
  1102	                                            progress,
  1103	                                            plan_options,
  1104	                                            dial.chunk_bytes(),
  1105	                                            dial.initial_streams(),
  1106	                                            &unreadable_paths,
  1107	                                        ).await?;
  1108	                                        if result.files_sent > 0 {
  1109	                                            fallback_files_sent =
  1110	                                                fallback_files_sent.saturating_add(result.files_sent);
  1111	                                        }
  1112	                                        if result.payloads_dispatched
  1113	                                            && first_payload_elapsed.is_none()
  1114	                                        {
  1115	                                            first_payload_elapsed = Some(start.elapsed());
  1116	                                        }
  1117	                                    }
  1118	                                }
  1119	                                TransferMode::DataPlane => {
  1120	                                    if let Some(sender) = data_plane_sender.as_mut() {

codex
The need-list batcher confirms the first post-early batches can be large, so the new test can pass even if the client is not truly using accumulated file count. I’m checking whether that creates a real under-resize case for incremental batches and whether there are tests pinning the intended accumulator.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/push/client/mod.rs | sed -n '600,660p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   600	        // hang a remote push for the OS TCP timeout (60-127s). The outer
   601	        // `tokio::time::timeout` is what bounds slow DNS too —
   602	        // `connect_timeout` alone only bounds the post-resolution TCP
   603	        // attempt (tonic/hyper-util resolve the name first).
   604	        let conn = tonic::transport::Endpoint::from_shared(uri.clone())
   605	            .map_err(|err| eyre::eyre!("invalid endpoint {}: {}", uri, err))?
   606	            .connect_timeout(std::time::Duration::from_secs(30));
   607	        let channel = tokio::time::timeout(std::time::Duration::from_secs(30), conn.connect())
   608	            .await
   609	            .map_err(|_| eyre::eyre!("connecting to {} timed out", uri))?
   610	            .map_err(|err| eyre::eyre!("failed to connect to {}: {}", uri, err))?;
   611	        let client = crate::generated::blit_client::BlitClient::new(channel);
   612	
   613	        Ok(Self { endpoint, client })
   614	    }
   615	
   616	    pub async fn push(
   617	        &mut self,
   618	        source: Arc<dyn TransferSource>,
   619	        filter: &FileFilter,
   620	        mirror_mode: bool,
   621	        mirror_kind: crate::generated::MirrorMode,
   622	        force_grpc: bool,
   623	        require_complete_scan: bool,
   624	        progress: Option<&RemotePushProgress>,
   625	        trace_data_plane: bool,
   626	    ) -> Result<RemotePushReport> {
   627	        let source_root = source.root();
   628	        // We don't check source_root.exists() here because source might be remote/virtual.
   629	        // If it's FsTransferSource, it should have been checked before creation or we trust it.
   630	
   631	        let start = Instant::now();
   632	        let mut first_payload_elapsed: Option<Duration> = None;
   633	
   634	        let mut manifest_lookup: HashMap<String, FileHeader> = HashMap::new();
   635	        let mut requested_files: HashSet<String> = HashSet::new();
   636	        let plan_options = PlanOptions::default();
   637	        let mut dial: Option<Arc<crate::engine::TransferDial>> = None;
   638	        let mut manifest_total_bytes: u64 = 0;
   639	        let mut transfer_size_hint: u64 = 0;
   640	
   641	        let (tx, rx) = mpsc::channel(32);
   642	        let outbound = ReceiverStream::new(rx);
   643	
   644	        let response_stream = self
   645	            .client
   646	            .push(outbound)
   647	            .await
   648	            .map_err(map_status)?
   649	            .into_inner();
   650	        let (mut response_rx, response_task) = spawn_response_task(response_stream);
   651	
   652	        let (module, rel_path) = module_and_path(&self.endpoint)?;
   653	        let destination_path = destination_path(&rel_path);
   654	
   655	        // R59 #1 F2: translate the client's FileFilter to wire FilterSpec
   656	        // so the daemon's purge enumerator can honor scope. Pre-fix the
   657	        // daemon used FileFilter::default() and would delete user-excluded
   658	        // destination entries it considered "extraneous".
   659	        let wire_filter = crate::generated::FilterSpec {
   660	            include: filter.include_files.clone(),

exec
/usr/bin/zsh -lc 'git show --numstat --format= c70c2ac' in /home/michael/dev/Blit
 succeeded in 0ms:
105	0	.review/findings/sf-2-shape-correction-resize.md
131	1	crates/blit-core/src/engine/dial.rs
169	36	crates/blit-core/src/remote/push/client/mod.rs
5	0	crates/blit-core/src/remote/push/client/types.rs
2	0	crates/blit-daemon/src/service/push/mod.rs
117	0	crates/blit-daemon/src/service/push/shape_resize_e2e.rs

exec
/usr/bin/zsh -lc "rg -n \"shape_resize|many_tiny_file|initial_stream_proposal|1479|1483|test result|passed\" .review docs crates -g'*.md' -g'*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
docs/STATE.md:62:  1478 → 1479/0/2 same-method A/B.
docs/STATE.md:184:  verified by execution in all three modes; suite 1479/0 held).
crates/blit-tui/src/main.rs:9221:    /// test passed for the wrong reason). The paired subpath test
crates/blit-tui/src/screens/f2.rs:95:/// d-22: F2 cancel-selected fragment passed to the
crates/blit-app/src/transfers/remote.rs:38://!   [`Endpoint`] passed in (Local → `FsTransferSource`,
crates/blit-app/src/transfers/remote.rs:191:/// passed.
crates/blit-app/src/admin/jobs.rs:136:/// for future event-category filtering and passed through as 0.
crates/blit-daemon/src/service/pull_sync.rs:16:use blit_core::engine::{initial_stream_proposal, TransferDial};
crates/blit-daemon/src/service/pull_sync.rs:437:    let proposal = initial_stream_proposal(total_bytes, file_count, dial.ceiling_max_streams());
crates/blit-daemon/src/service/pull_sync.rs:618:    // dial-owned and passed to the sink directly (w2-2: the planner
crates/blit-daemon/src/service/pull_sync.rs:1303:    // practice — passed for symmetry with the other accept sites.
crates/blit-daemon/src/service/delegated_pull.rs:412:    // already passed `NormalizedTransferOperation::from_spec`, so
crates/blit-daemon/src/service/core.rs:166:    /// Caller-passed values rather than re-reading from the
crates/blit-daemon/src/service/push/mod.rs:4:mod shape_resize_e2e;
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:11://! resize (`maybe_shape_resize` in blit-core's push client) re-runs the
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:31:async fn many_tiny_file_push_opens_more_than_one_data_plane_connection() {
crates/blit-daemon/src/service/push/control.rs:800:    blit_core::engine::initial_stream_proposal(
crates/blit-core/tests/pull_sync_with_spec_wire.rs:320:        "spec on the wire diverged from the spec passed to pull_sync_with_spec"
crates/blit-core/tests/local_transfers.rs:153:/// ue-r2-1c: the single-file shortcut historically bypassed
crates/blit-core/src/engine/dial.rs:263:    /// ticks, at least [`RESIZE_COOLDOWN_TICKS`] must have passed
crates/blit-core/src/engine/dial.rs:316:        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
crates/blit-core/src/engine/dial.rs:338:    /// [`initial_stream_proposal`] assigns the full workload. As the
crates/blit-core/src/engine/dial.rs:349:    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
crates/blit-core/src/engine/dial.rs:474:pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
crates/blit-core/src/engine/dial.rs:687:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
crates/blit-core/src/engine/dial.rs:691:        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
crates/blit-core/src/engine/dial.rs:695:        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
crates/blit-core/src/engine/dial.rs:696:        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
crates/blit-core/src/engine/dial.rs:697:        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
crates/blit-core/src/engine/dial.rs:698:        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
crates/blit-core/src/engine/dial.rs:699:        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
crates/blit-core/src/engine/dial.rs:700:        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
crates/blit-core/src/engine/dial.rs:701:        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
crates/blit-core/src/engine/dial.rs:702:        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
crates/blit-core/src/engine/dial.rs:703:        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
crates/blit-core/src/engine/dial.rs:704:        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
crates/blit-core/src/engine/dial.rs:705:        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
crates/blit-core/src/engine/dial.rs:706:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
crates/blit-core/src/engine/dial.rs:708:        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
crates/blit-core/src/engine/dial.rs:709:        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
crates/blit-core/src/engine/dial.rs:710:        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
crates/blit-core/src/engine/dial.rs:711:        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
crates/blit-core/src/engine/dial.rs:712:        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
crates/blit-core/src/engine/dial.rs:713:        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
crates/blit-core/src/engine/dial.rs:715:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
crates/blit-core/src/engine/dial.rs:716:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
crates/blit-core/src/engine/dial.rs:936:        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
crates/blit-core/src/engine/dial.rs:938:        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
crates/blit-core/src/engine/dial.rs:942:            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
crates/blit-core/src/engine/dial.rs:948:        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
crates/blit-core/src/engine/dial.rs:952:    fn shape_resize_ramps_one_epoch_at_a_time_toward_the_target() {
crates/blit-core/src/engine/dial.rs:957:        assert_eq!(dial.propose_shape_resize(0), None);
crates/blit-core/src/engine/dial.rs:958:        assert_eq!(dial.propose_shape_resize(1), None);
crates/blit-core/src/engine/dial.rs:962:        let p1 = dial.propose_shape_resize(3).expect("live 1 → target 3");
crates/blit-core/src/engine/dial.rs:971:        assert_eq!(dial.propose_shape_resize(3), None, "one in flight");
crates/blit-core/src/engine/dial.rs:976:        let p2 = dial.propose_shape_resize(3).expect("live 2 → target 3");
crates/blit-core/src/engine/dial.rs:981:        assert_eq!(dial.propose_shape_resize(3), None, "target reached");
crates/blit-core/src/engine/dial.rs:984:        let p3 = dial.propose_shape_resize(4).expect("live 3 → target 4");
crates/blit-core/src/engine/dial.rs:988:            dial.propose_shape_resize(4).is_some(),
crates/blit-core/src/engine/dial.rs:994:    fn shape_resize_clamps_to_the_profile_ceiling() {
crates/blit-core/src/engine/dial.rs:998:            .propose_shape_resize(100)
crates/blit-core/src/engine/dial.rs:1003:            dial.propose_shape_resize(100),
crates/blit-core/src/engine/mod.rs:30:    initial_stream_proposal, local_receiver_capacity, spawn_dial_tuner,
crates/blit-core/src/engine/single_file.rs:21:/// shortcut bypassed perf-history/predictor recording entirely — the
crates/blit-core/src/engine/tuning.rs:305:    /// even if the wrapper were bypassed.
crates/blit-core/src/fs_enum.rs:43:///   other rules above are bypassed for the inclusion test.
crates/blit-core/src/fs_enum.rs:202:        // are bypassed because the user explicitly enumerated targets.
crates/blit-core/src/perf_predictor.rs:23:/// records (which the orchestrator passed through anyway) shifted
crates/blit-core/src/perf_predictor.rs:1327:        // passed even when the production load() skipped the
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md:228:  --resume is passed on push. House rule: help + manpage + README in one slice.
crates/blit-core/src/remote/push/client/mod.rs:523:/// need list accumulates, re-run [`crate::engine::initial_stream_proposal`]
crates/blit-core/src/remote/push/client/mod.rs:528:async fn maybe_shape_resize(
crates/blit-core/src/remote/push/client/mod.rs:539:        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())
crates/blit-core/src/remote/push/client/mod.rs:541:    match dial.propose_shape_resize(target) {
crates/blit-core/src/remote/push/client/mod.rs:747:        // present). `shape_resize_enabled` flips off permanently the
crates/blit-core/src/remote/push/client/mod.rs:752:        let mut shape_resize_enabled = true;
crates/blit-core/src/remote/push/client/mod.rs:861:                                                && shape_resize_enabled
crates/blit-core/src/remote/push/client/mod.rs:865:                                                    if let Err(send_err) = maybe_shape_resize(
crates/blit-core/src/remote/push/client/mod.rs:1037:                                            if resize_negotiated && shape_resize_enabled {
crates/blit-core/src/remote/push/client/mod.rs:1038:                                                if let Err(send_err) = maybe_shape_resize(
crates/blit-core/src/remote/push/client/mod.rs:1167:                                                && shape_resize_enabled
crates/blit-core/src/remote/push/client/mod.rs:1171:                                                if let Err(send_err) = maybe_shape_resize(
crates/blit-core/src/remote/push/client/mod.rs:1395:                                shape_resize_enabled = false;
crates/blit-core/src/remote/transfer/diff_planner.rs:321:        // bypassed, so identical files still appear in the planned
crates/blit-core/src/remote/transfer/abort_on_drop.rs:80:    // completion, so it passed whether or not Drop aborted — vacuous
crates/blit-core/src/remote/transfer/source.rs:645:        // Verify the decorator's filter wins over any filter passed to
crates/blit-core/src/remote/transfer/source.rs:646:        // scan() — this ensures the universal chokepoint isn't bypassed
.review/findings/sf-2-shape-correction-resize.md:10:`initial_stream_proposal` was byte-weighted; in reality the table has
.review/findings/sf-2-shape-correction-resize.md:22:`initial_stream_proposal` over the accumulated need bytes/count and
.review/findings/sf-2-shape-correction-resize.md:28:- `TransferDial::propose_shape_resize(desired)` (engine `dial.rs`, the
.review/findings/sf-2-shape-correction-resize.md:54:- `crates/blit-core/src/engine/dial.rs` — `propose_shape_resize`, CAS
.review/findings/sf-2-shape-correction-resize.md:57:  + `maybe_shape_resize` helpers, three correction call sites, REMOVE
.review/findings/sf-2-shape-correction-resize.md:60:- `crates/blit-daemon/src/service/push/shape_resize_e2e.rs` (new) +
.review/findings/sf-2-shape-correction-resize.md:65:Suite 1479 → **1483 passed / 0 failed** (37 suites; same 2 ignored) —
.review/findings/sf-2-shape-correction-resize.md:72:- `shape_resize_ramps_one_epoch_at_a_time_toward_the_target`,
.review/findings/sf-2-shape-correction-resize.md:73:  `shape_resize_clamps_to_the_profile_ceiling` — proposal semantics:
.review/findings/sf-2-shape-correction-resize.md:76:- `many_tiny_file_push_opens_more_than_one_data_plane_connection`
.review/findings/sf-2-shape-correction-resize.md:81:  `propose_shape_resize` forced to `None` (temporary revert) the test
.review/findings/sf-2-shape-correction-resize.md:83:  re-passed. Runtime ~0.35 s.
.review/results/sf-2-shape-correction-resize.codex.md:14:Review the diff of commit c70c2ac (run: git show c70c2ac). It implements sf-2 of docs/plan/SMALL_FILE_CEILING.md (Active): client-side shape-correction stream resize — the daemon proposes the push epoch-0 stream count from a partial manifest at its early flush, so many-tiny-file pushes rode 1 stream; the client now re-runs the engine shape table (initial_stream_proposal) over the accumulated need list and corrects upward via the existing ue-r2-2 resize wire, one ADD epoch at a time. Check: correctness regressions, the slice's acceptance criteria (proposal-table unit pins for the plan's cells + loopback e2e pin that a 10k-file push opens >1 data-plane connection), FAST/SIMPLE/RELIABLE, the invariants relevant to transfer code (byte-identical / StallGuard / cancellation / byte-accounting), interaction between the two resize proposers (tuner resize_tick vs propose_shape_resize — epoch allocation, one-in-flight, flap risk), mixed-version behavior (old daemon / gRPC fallback unchanged), and that the test count did not drop (1479 -> 1483). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
.review/results/sf-2-shape-correction-resize.codex.md:26: .../src/service/push/shape_resize_e2e.rs           | 117 ++++++++++++
.review/results/sf-2-shape-correction-resize.codex.md:47:    a time. Dial owns the policy (propose_shape_resize; CAS epoch
.review/results/sf-2-shape-correction-resize.codex.md:52:    1479 -> 1483.
.review/results/sf-2-shape-correction-resize.codex.md:61: .../src/service/push/shape_resize_e2e.rs           | 117 ++++++++++++
.review/results/sf-2-shape-correction-resize.codex.md:79:+`initial_stream_proposal` was byte-weighted; in reality the table has
.review/results/sf-2-shape-correction-resize.codex.md:91:+`initial_stream_proposal` over the accumulated need bytes/count and
.review/results/sf-2-shape-correction-resize.codex.md:97:+- `TransferDial::propose_shape_resize(desired)` (engine `dial.rs`, the
.review/results/sf-2-shape-correction-resize.codex.md:123:+- `crates/blit-core/src/engine/dial.rs` — `propose_shape_resize`, CAS
.review/results/sf-2-shape-correction-resize.codex.md:126:+  + `maybe_shape_resize` helpers, three correction call sites, REMOVE
.review/results/sf-2-shape-correction-resize.codex.md:129:+- `crates/blit-daemon/src/service/push/shape_resize_e2e.rs` (new) +
.review/results/sf-2-shape-correction-resize.codex.md:134:+Suite 1479 → **1483 passed / 0 failed** (37 suites; same 2 ignored) —
.review/results/sf-2-shape-correction-resize.codex.md:141:+- `shape_resize_ramps_one_epoch_at_a_time_toward_the_target`,
.review/results/sf-2-shape-correction-resize.codex.md:142:+  `shape_resize_clamps_to_the_profile_ceiling` — proposal semantics:
.review/results/sf-2-shape-correction-resize.codex.md:145:+- `many_tiny_file_push_opens_more_than_one_data_plane_connection`
.review/results/sf-2-shape-correction-resize.codex.md:150:+  `propose_shape_resize` forced to `None` (temporary revert) the test
.review/results/sf-2-shape-correction-resize.codex.md:152:+  re-passed. Runtime ~0.35 s.
.review/results/sf-2-shape-correction-resize.codex.md:184:+        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
.review/results/sf-2-shape-correction-resize.codex.md:205:+    /// [`initial_stream_proposal`] assigns the full workload. As the
.review/results/sf-2-shape-correction-resize.codex.md:216:+    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
.review/results/sf-2-shape-correction-resize.codex.md:256:+        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
.review/results/sf-2-shape-correction-resize.codex.md:258:+        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
.review/results/sf-2-shape-correction-resize.codex.md:262:+            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
.review/results/sf-2-shape-correction-resize.codex.md:268:+        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
.review/results/sf-2-shape-correction-resize.codex.md:272:+    fn shape_resize_ramps_one_epoch_at_a_time_toward_the_target() {
.review/results/sf-2-shape-correction-resize.codex.md:277:+        assert_eq!(dial.propose_shape_resize(0), None);
.review/results/sf-2-shape-correction-resize.codex.md:278:+        assert_eq!(dial.propose_shape_resize(1), None);
.review/results/sf-2-shape-correction-resize.codex.md:282:+        let p1 = dial.propose_shape_resize(3).expect("live 1 → target 3");
.review/results/sf-2-shape-correction-resize.codex.md:291:+        assert_eq!(dial.propose_shape_resize(3), None, "one in flight");
.review/results/sf-2-shape-correction-resize.codex.md:296:+        let p2 = dial.propose_shape_resize(3).expect("live 2 → target 3");
.review/results/sf-2-shape-correction-resize.codex.md:301:+        assert_eq!(dial.propose_shape_resize(3), None, "target reached");
.review/results/sf-2-shape-correction-resize.codex.md:304:+        let p3 = dial.propose_shape_resize(4).expect("live 3 → target 4");
.review/results/sf-2-shape-correction-resize.codex.md:308:+            dial.propose_shape_resize(4).is_some(),
.review/results/sf-2-shape-correction-resize.codex.md:314:+    fn shape_resize_clamps_to_the_profile_ceiling() {
.review/results/sf-2-shape-correction-resize.codex.md:318:+            .propose_shape_resize(100)
.review/results/sf-2-shape-correction-resize.codex.md:323:+            dial.propose_shape_resize(100),
.review/results/sf-2-shape-correction-resize.codex.md:384:+/// need list accumulates, re-run [`crate::engine::initial_stream_proposal`]
.review/results/sf-2-shape-correction-resize.codex.md:389:+async fn maybe_shape_resize(
.review/results/sf-2-shape-correction-resize.codex.md:400:+        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())
.review/results/sf-2-shape-correction-resize.codex.md:402:+    match dial.propose_shape_resize(target) {
.review/results/sf-2-shape-correction-resize.codex.md:417:+        // present). `shape_resize_enabled` flips off permanently the
.review/results/sf-2-shape-correction-resize.codex.md:422:+        let mut shape_resize_enabled = true;
.review/results/sf-2-shape-correction-resize.codex.md:435:+                                                && shape_resize_enabled
.review/results/sf-2-shape-correction-resize.codex.md:439:+                                                    if let Err(send_err) = maybe_shape_resize(
.review/results/sf-2-shape-correction-resize.codex.md:477:+                                            if resize_negotiated && shape_resize_enabled {
.review/results/sf-2-shape-correction-resize.codex.md:478:+                                                if let Err(send_err) = maybe_shape_resize(
.review/results/sf-2-shape-correction-resize.codex.md:507:+                                                && shape_resize_enabled
.review/results/sf-2-shape-correction-resize.codex.md:511:+                                                if let Err(send_err) = maybe_shape_resize(
.review/results/sf-2-shape-correction-resize.codex.md:584:+                                shape_resize_enabled = false;
.review/results/sf-2-shape-correction-resize.codex.md:623:+mod shape_resize_e2e;
.review/results/sf-2-shape-correction-resize.codex.md:627:diff --git a/crates/blit-daemon/src/service/push/shape_resize_e2e.rs b/crates/blit-daemon/src/service/push/shape_resize_e2e.rs
.review/results/sf-2-shape-correction-resize.codex.md:631:+++ b/crates/blit-daemon/src/service/push/shape_resize_e2e.rs
.review/results/sf-2-shape-correction-resize.codex.md:643:+//! resize (`maybe_shape_resize` in blit-core's push client) re-runs the
.review/results/sf-2-shape-correction-resize.codex.md:663:+async fn many_tiny_file_push_opens_more_than_one_data_plane_connection() {
.review/results/sf-2-shape-correction-resize.codex.md:794:stream** — `engine::initial_stream_proposal` is byte-weighted, so
.review/results/sf-2-shape-correction-resize.codex.md:855:   `initial_stream_proposal` (and the pull-side equivalent) weight
.review/results/sf-2-shape-correction-resize.codex.md:934:/usr/bin/zsh -lc 'rg -n "struct TransferDial|pending_epoch|resize_epoch|resize_tick|resize_settled|initial_stream_proposal|ceiling_max_streams|live_streams" crates/blit-core/src/engine/dial.rs' in /home/michael/dev/Blit
.review/results/sf-2-shape-correction-resize.codex.md:967:338:    /// [`initial_stream_proposal`] assigns the full workload. As the
.review/results/sf-2-shape-correction-resize.codex.md:980:474:pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
.review/results/sf-2-shape-correction-resize.codex.md:990:687:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
.review/results/sf-2-shape-correction-resize.codex.md:991:691:        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
.review/results/sf-2-shape-correction-resize.codex.md:992:695:        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
.review/results/sf-2-shape-correction-resize.codex.md:993:696:        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
.review/results/sf-2-shape-correction-resize.codex.md:994:697:        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
.review/results/sf-2-shape-correction-resize.codex.md:995:698:        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
.review/results/sf-2-shape-correction-resize.codex.md:996:699:        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
.review/results/sf-2-shape-correction-resize.codex.md:997:700:        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
.review/results/sf-2-shape-correction-resize.codex.md:998:701:        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
.review/results/sf-2-shape-correction-resize.codex.md:999:702:        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
.review/results/sf-2-shape-correction-resize.codex.md:1000:703:        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
.review/results/sf-2-shape-correction-resize.codex.md:1001:704:        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
.review/results/sf-2-shape-correction-resize.codex.md:1002:705:        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
.review/results/sf-2-shape-correction-resize.codex.md:1003:706:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
.review/results/sf-2-shape-correction-resize.codex.md:1004:708:        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
.review/results/sf-2-shape-correction-resize.codex.md:1005:709:        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
.review/results/sf-2-shape-correction-resize.codex.md:1006:710:        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
.review/results/sf-2-shape-correction-resize.codex.md:1007:711:        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
.review/results/sf-2-shape-correction-resize.codex.md:1008:712:        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
.review/results/sf-2-shape-correction-resize.codex.md:1009:713:        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
.review/results/sf-2-shape-correction-resize.codex.md:1010:715:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
.review/results/sf-2-shape-correction-resize.codex.md:1011:716:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
.review/results/sf-2-shape-correction-resize.codex.md:1041:936:        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
.review/results/sf-2-shape-correction-resize.codex.md:1042:938:        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
.review/results/sf-2-shape-correction-resize.codex.md:1043:942:            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
.review/results/sf-2-shape-correction-resize.codex.md:1044:948:        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
.review/results/sf-2-shape-correction-resize.codex.md:1318:   263	    /// ticks, at least [`RESIZE_COOLDOWN_TICKS`] must have passed
.review/results/sf-2-shape-correction-resize.codex.md:1371:   316	        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
.review/results/sf-2-shape-correction-resize.codex.md:1393:   338	    /// [`initial_stream_proposal`] assigns the full workload. As the
.review/results/sf-2-shape-correction-resize.codex.md:1404:   349	    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
.review/results/sf-2-shape-correction-resize.codex.md:1583:   523	/// need list accumulates, re-run [`crate::engine::initial_stream_proposal`]
.review/results/sf-2-shape-correction-resize.codex.md:1588:   528	async fn maybe_shape_resize(
.review/results/sf-2-shape-correction-resize.codex.md:1599:   539	        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())
.review/results/sf-2-shape-correction-resize.codex.md:1601:   541	    match dial.propose_shape_resize(target) {
.review/results/sf-2-shape-correction-resize.codex.md:1672:   747	        // present). `shape_resize_enabled` flips off permanently the
.review/results/sf-2-shape-correction-resize.codex.md:1677:   752	        let mut shape_resize_enabled = true;
.review/results/sf-2-shape-correction-resize.codex.md:1786:   861	                                                && shape_resize_enabled
.review/results/sf-2-shape-correction-resize.codex.md:1790:   865	                                                    if let Err(send_err) = maybe_shape_resize(
.review/results/sf-2-shape-correction-resize.codex.md:1962:  1037	                                            if resize_negotiated && shape_resize_enabled {
.review/results/sf-2-shape-correction-resize.codex.md:1963:  1038	                                                if let Err(send_err) = maybe_shape_resize(
.review/results/sf-2-shape-correction-resize.codex.md:2131:  1167	                                                && shape_resize_enabled
.review/results/sf-2-shape-correction-resize.codex.md:2135:  1171	                                                if let Err(send_err) = maybe_shape_resize(
.review/results/sf-2-shape-correction-resize.codex.md:2359:  1395	                                shape_resize_enabled = false;
.review/results/sf-2-shape-correction-resize.codex.md:2443:  1479	
.review/results/sf-2-shape-correction-resize.codex.md:2447:  1483	
.review/results/sf-2-shape-correction-resize.codex.md:2526:crates/blit-daemon/src/service/push/shape_resize_e2e.rs:101:    assert!(!report.fallback_used, "must ride the TCP data plane");
.review/results/sf-2-shape-correction-resize.codex.md:2527:crates/blit-daemon/src/service/push/shape_resize_e2e.rs:107:        .data_plane_streams
.review/results/sf-2-shape-correction-resize.codex.md:3177:   474	pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
.review/results/sf-1-tripwire-harness.gpt-verdict.md:50:   directly: fmt/clippy clean, `cargo test --workspace` 1479/0 across
crates/blit-cli/src/cli.rs:396:    /// True when `--delete-scope all` was passed. Maps onto
crates/blit-cli/src/transfers/mod.rs:274:                 the front gate in run_transfer was bypassed",
crates/blit-cli/tests/readonly_enforcement.rs:9://! radius) would have passed the full validation suite.
crates/blit-cli/tests/local_move_semantics.rs:4://! Pre-fix `crates/blit-cli/src/transfers/mod.rs:458` passed
crates/blit-cli/tests/common/mod.rs:125:        // be passed through or the daemon lands in the wrong directory.
.review/README.md:51:Tests must show "passed" with zero failures. Test count may grow
docs/plan/SMALL_FILE_CEILING.md:41:stream** — `engine::initial_stream_proposal` is byte-weighted, so
docs/plan/SMALL_FILE_CEILING.md:102:   `initial_stream_proposal` (and the pull-side equivalent) weight
docs/plan/WORKFLOW_PHASE_3.md:11:**Prerequisites**: Phase 2 gate passed (streaming orchestrator stable) and Phase 2.5 benchmarks meeting targets.  
docs/plan/POST_REVIEW_FIXES.md:51:  why the bug fixed in 946bd77 passed every existing test.
docs/plan/PIPELINE_UNIFICATION.md:233:  args are passed (CLI side).
docs/DECISIONS.md:105:- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
docs/agent/GPT_REVIEW_LOOP.md:101:Never pause merely to have the owner bless code that already passed
.review/results/d-65-f1-push-mirror-move.reopened.md:9:- `cargo fmt --all -- --check` passed.
.review/results/d-65-f1-push-mirror-move.reopened.md:10:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-65-f1-push-mirror-move.reopened.md:11:- `cargo test --workspace` passed: 552 tests.
.review/results/d-64-f1-push-ttl.reopened.md:9:- `cargo fmt --all -- --check` passed
.review/results/d-64-f1-push-ttl.reopened.md:10:- `cargo clippy --workspace --all-targets -- -D warnings` passed
.review/results/d-64-f1-push-ttl.reopened.md:11:- `cargo test --workspace` passed (550 TUI tests)
.review/results/d-63-f1-push-progress.reopened.md:9:- `cargo fmt --all -- --check` passed
.review/results/d-63-f1-push-progress.reopened.md:10:- `cargo clippy --workspace --all-targets -- -D warnings` passed
.review/results/d-63-f1-push-progress.reopened.md:11:- `cargo test --workspace` passed (544 TUI tests)
.review/results/d-62-f1-trigger-error.reopened.md:9:- `cargo fmt --all -- --check` passed
.review/results/d-62-f1-trigger-error.reopened.md:10:- `cargo clippy --workspace --all-targets -- -D warnings` passed
.review/results/d-62-f1-trigger-error.reopened.md:11:- `cargo test --workspace` passed (541 TUI tests)
.review/results/d-61-f1-trigger-push.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-61-f1-trigger-push.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-61-f1-trigger-push.reopened.md:10:- `cargo test --workspace` passed (539 tests).
.review/results/d-60-f1-trigger-move.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-60-f1-trigger-move.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-60-f1-trigger-move.reopened.md:10:- `cargo test --workspace` passed (528 tests).
.review/results/d-57-f3-move.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-57-f3-move.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-57-f3-move.reopened.md:10:- `cargo test --workspace` passed (509 tests).
.review/results/d-55-f3-mirror.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-55-f3-mirror.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-55-f3-mirror.reopened.md:10:- `cargo test --workspace` passed.
.review/results/d-55-f3-mirror.reopened.md:16:   `spawn_f3_pull` builds the TUI mirror execution with `options: PullSyncOptions::default()` at `crates/blit-tui/src/main.rs:2881` through `crates/blit-tui/src/main.rs:2886`. The separate `PullSyncExecution.mirror_mode` field is passed to `RemotePullClient::pull_sync` as `track_paths` at `crates/blit-app/src/transfers/remote.rs:343` through `crates/blit-app/src/transfers/remote.rs:349`, but the wire `TransferOperationSpec` is built only from `execution.options` at `crates/blit-core/src/remote/pull.rs:598` through `crates/blit-core/src/remote/pull.rs:606`. Since `PullSyncOptions::default().mirror_mode` is false, `build_spec_from_options` emits `MirrorMode::Off` at `crates/blit-core/src/remote/pull.rs:558` through `crates/blit-core/src/remote/pull.rs:565`.
.review/results/d-53-f3-batch-pull.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-53-f3-batch-pull.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-53-f3-batch-pull.reopened.md:10:- `cargo test --workspace` passed.
.review/results/d-50-f3-batch-delete.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-50-f3-batch-delete.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-50-f3-batch-delete.reopened.md:10:- `cargo test --workspace` passed.
.review/results/d-49-f3-multiselect.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-49-f3-multiselect.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-49-f3-multiselect.reopened.md:10:- `cargo test --workspace` passed.
.review/results/d-48-f2-follows-browse.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-48-f2-follows-browse.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-48-f2-follows-browse.reopened.md:10:- `cargo test --workspace` passed.
.review/results/d-47-f1-browse-nav.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-47-f1-browse-nav.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-47-f1-browse-nav.reopened.md:10:- `cargo test --workspace` passed.
.review/results/d-45-f3-delete.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-45-f3-delete.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-45-f3-delete.reopened.md:10:- `cargo test --workspace` passed.
.review/results/d-41-f3-du.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-41-f3-du.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-41-f3-du.reopened.md:10:- `cargo test --workspace` passed.
.review/results/d-40-config-pull-ttl.reopened.md:8:- `cargo fmt --all -- --check` passed.
.review/results/d-40-config-pull-ttl.reopened.md:9:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-40-config-pull-ttl.reopened.md:10:- `cargo test --workspace` passed.
.review/results/d-37-f3-pull-progress.reopened.md:9:- `cargo fmt --all -- --check` passed.
.review/results/d-37-f3-pull-progress.reopened.md:10:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-37-f3-pull-progress.reopened.md:11:- `cargo test --workspace` passed.
.review/results/d-35-f3-pull-execute.reopened.md:9:- `cargo fmt --all -- --check` passed.
.review/results/d-35-f3-pull-execute.reopened.md:10:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-35-f3-pull-execute.reopened.md:11:- `cargo test --workspace` passed.
.review/results/d-33-f3-pull-source.reopened.md:3:Reviewed commit: `11388002b9bfa2f09980a7008b404a90a1479aa1`
.review/results/d-33-f3-pull-source.reopened.md:9:- `cargo fmt --all -- --check` passed.
.review/results/d-33-f3-pull-source.reopened.md:10:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-33-f3-pull-source.reopened.md:11:- `cargo test --workspace` passed.
.review/results/bridge-2-prometheus-http.reopened.md:7:- `cargo fmt --all -- --check`: passed
.review/results/bridge-2-prometheus-http.reopened.md:8:- `cargo clippy --workspace --all-targets -- -D warnings`: passed
.review/results/bridge-2-prometheus-http.reopened.md:9:- `cargo test --workspace`: passed
.review/results/bridge-2-prometheus-http.reopened.md:10:- `cargo test -p blit-prometheus-bridge`: passed, 10 tests
.review/results/bridge-1-prometheus-scaffold.reopened.md:7:- `cargo fmt --all -- --check` passed.
.review/results/bridge-1-prometheus-scaffold.reopened.md:8:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/bridge-1-prometheus-scaffold.reopened.md:9:- `cargo test --workspace` passed.
.review/results/bridge-1-prometheus-scaffold.reopened.md:10:- Extra targeted check: `cargo test -p blit-prometheus-bridge` passed: 5 tests.
.review/findings/sf-1-tripwire-harness.md:43:  `engine::initial_stream_proposal` tiers (200→1, 1k→2, 5k→4, 10k→8,
.review/findings/sf-1-tripwire-harness.md:68:clippy clean, `cargo test --workspace` **1479 passed / 0 failed**
.review/results/audit-5b1-bridge-listener-write.reopened.md:11:- `cargo fmt --all -- --check`: passed
.review/results/audit-5b1-bridge-listener-write.reopened.md:12:- `cargo clippy --workspace --all-targets -- -D warnings`: passed
.review/results/audit-5b1-bridge-listener-write.reopened.md:13:- `cargo test --workspace`: passed
.review/results/design-3-unbounded-data-plane-connects.gpt-verdict.md:23:workspace 1476 → 1479 passed / 0 failed / 2 ignored across 37 suites.
.review/results/w4-1-abortondrop-family.gpt-verdict.md:12:| 1 | `abort_on_drop.rs:99` Low — `drop_without_consume_aborts_running_task` is vacuous: 150ms wait vs the task's 500ms natural completion, so it passes whether or not `Drop` aborts | **Accepted** | Real, and pre-existing: the test was relocated verbatim from `pull.rs`, where the same 150ms-vs-500ms shape (and a comment contradicting its own code) made it vacuous since R32-F2. Fixed with `start_paused` virtual time + a 700ms wake — auto-advance deterministically runs a detached task's 500ms sleep before the assertion. Mutation-verified: with `Drop` changed to detach, the repaired test fails (the original passed); restored, all 4 module tests green. |
.review/results/w3-1-memory-aware-buffer-pool.gpt-verdict.md:18:validation gate fmt + clippy clean, workspace 1452 → 1460 passed / 0
.review/results/w2-2-stream-ladder-owner.gpt-verdict.md:27:`cargo test --workspace` 37 suites, 1452 passed / 0 failed / 2 ignored
.review/results/w1-4-accept-token-constants.gpt-verdict.md:34:targets, `-D warnings`), `cargo test --workspace` 1446 passed / 0
.review/results/w1-3-tcp-keepalive-honesty.gpt-verdict.md:36:`-D warnings`), `cargo test --workspace` 1446 passed / 2 ignored, 37
.review/results/w1-2-data-socket-policy-helper.gpt-verdict.md:45:`-D warnings`), `cargo test --workspace` 1445 passed / 2 ignored, 37
.review/findings/w9-3-test-harness-builder.md:5:**Codex**: NEEDS FIXES, 1 Medium (fake-server port bypassed the
.review/findings/w9-3-test-harness-builder.md:135:  `test result:` line, doc-test suites included, via `git stash`):
.review/findings/w9-3-test-harness-builder.md:136:  HEAD `3d8326b` = 1478/0/2 across 37 suites → this slice = 1479/0/2
.review/findings/w9-3-test-harness-builder.md:139:  (STATE's recorded "1479" baseline for design-3 came from a different
.review/findings/win-1-push-needlist-separators.md:29:why single-file/flat-layout tests always passed and CI on
docs/bugs/HANDOFF-remote-pull-single-file.md:21:All three were valid concerns. The audit uncovered a **much bigger** bug than single-file remote pulls: **any remote pull with a non-empty subpath double-nests** (files land at `dst/X/X/...` instead of `dst/X/...`). Tests passed because they all used `/test/` (empty subpath).
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:497:module (`feedback-port-cli-safety-guards` rule) is invisibly bypassed by the TUI's silent
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:857:bypassed by three ad-hoc helpers that disagree on empty-path encoding. Push sends `""`;
docs/reviews/codebase_review_2026-05-01.md:17:- `cargo test --workspace` passes when run outside the sandbox with local port binding allowed: 185 tests passed, 1 doc test ignored.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:403:**Mechanism**: control.rs:55 creates (upload_tx, upload_rx) with FILE_UPLOAD_CHANNEL_CAPACITY = FILE_LIST_BATCH_MAX_ENTRIES*16 = 262,144 (control.rs:31). Every file passing file_requires_upload gets file.clone() sent into it (control.rs:157) on the hot per-manifest-entry path. The receiver is taken at control.rs:214-215 and :287 and passed only into accept_data_connection_stream (TCP path); inside data_plane.rs:200-207 a task is spawned that does `while guard.recv().await.is_some() {}` — drain and discard — and :207 voids the companion cache param ('headers come off the wire; cache no longer needed'). The gRPC fallback at control.rs:275 calls execute_grpc_fallback with files_to_upload.clone(), not the channel, directly contradicting the comment at control.rs:151-154. Net cost: one FileHeader clone per uploaded file plus a spawned drain task per data plane, purely to feed a Phase-5 leftover; net risk: the false comment misleads the next editor about liveness.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:461:**Proposed fix**: Scope the claims: amend cli.rs --resume/--retry help, blit.1.md, and retry.rs's module doc to state block resume applies to local/pull (and delegated specs), and that push retries resume at whole-file granularity; optionally emit a warning when --resume is passed to a push (per project rule, help + manpage + README change in the same slice).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:506:- /home/michael/dev/Blit/crates/blit-daemon/src/service/pull_sync.rs:637 — diverged copy: let pool_size = 4; literal prefetch 8 passed at line 641
.review/results/m2f-9-f2-discovery-refan.reopened.md:23:- `cargo test --workspace` (591 passed)
docs/reviews/followup_review_2026-05-02.md:18:- `cargo test --workspace` passed locally after the change: 209 tests, 0
docs/reviews/followup_review_2026-05-02.md:133:- `cargo check --workspace` passed.
docs/reviews/followup_review_2026-05-02.md:134:- `cargo test --workspace` passed: 214 tests, 0 failures, 1 ignored doc test.
docs/reviews/followup_review_2026-05-02.md:280:- `cargo test --workspace` passed: 214 tests, 0 failures, 1 ignored doc test.
docs/reviews/followup_review_2026-05-02.md:337:- `cargo test --workspace` passed: 214 tests, 0 failures, 1 ignored doc test.
docs/reviews/followup_review_2026-05-02.md:338:- `cargo fmt -- --check` passed.
docs/reviews/followup_review_2026-05-02.md:474:- `cargo fmt -- --check` passed.
docs/reviews/followup_review_2026-05-02.md:475:- `cargo test --workspace` passed.
docs/reviews/followup_review_2026-05-02.md:632:- `cargo fmt -- --check` passed.
docs/reviews/followup_review_2026-05-02.md:633:- `cargo test --workspace` passed.
docs/reviews/followup_review_2026-05-02.md:739:- `cargo fmt -- --check` passed.
docs/reviews/followup_review_2026-05-02.md:740:- `cargo test --workspace` passed.
docs/reviews/followup_review_2026-05-02.md:795:- `cargo fmt -- --check` passed.
docs/reviews/followup_review_2026-05-02.md:796:- `cargo test --workspace` passed.
docs/reviews/followup_review_2026-05-02.md:880:- `cargo fmt -- --check` passed.
docs/reviews/followup_review_2026-05-02.md:881:- `cargo test --workspace` passed.
docs/reviews/followup_review_2026-05-02.md:931:- `cargo fmt -- --check` passed.
docs/reviews/followup_review_2026-05-02.md:932:- `cargo test --workspace` passed.
docs/reviews/followup_review_2026-05-02.md:2800:(`cargo test --workspace`: 370 passed, 0 failed).
docs/reviews/followup_review_2026-05-02.md:3001:(`cargo test --workspace`: 374 passed, 0 failed).
docs/reviews/followup_review_2026-05-02.md:3251:All passed. Existing warnings remain: macOS FSEvents deprecation (F14) and the
.review/results/m2f-5-f2-fanout.reopened.md:23:- `cargo test --workspace` (584 passed)
.review/results/m2f-2-f2-composite-key.reopened.md:19:- `cargo fmt --all -- --check` passed.
.review/results/m2f-2-f2-composite-key.reopened.md:20:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/m2f-2-f2-composite-key.reopened.md:21:- `cargo test --workspace` passed: 579 tests.
.review/results/ue-r2-1c.gpt-verdict.md:42:  tests 1394 passed / 0 failed / 2 ignored.
.review/results/e-9-theme-f2-row-highlight.reopened.md:7:- `cargo fmt --all -- --check` passed.
.review/results/e-9-theme-f2-row-highlight.reopened.md:8:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/e-9-theme-f2-row-highlight.reopened.md:9:- `cargo test --workspace` passed: 600 tests.
.review/results/ue-r2-1b.gpt-verdict.md:41:  fmt clean, clippy clean, tests 1391 passed / 0 failed / 2 ignored.
.review/results/dark-2-theme-mode-preset.reopened.md:41:- `cargo fmt --all -- --check` passed
.review/results/dark-2-theme-mode-preset.reopened.md:42:- `cargo clippy --workspace --all-targets -- -D warnings` passed
.review/results/dark-2-theme-mode-preset.reopened.md:43:- `cargo test --workspace` passed (`blit-tui`: 615 tests)
.review/results/d-71-f1-delegated-move.reopened.md:48:All passed on the reviewed SHA.
.review/results/ue-r2-1a.gpt-verdict.md:8:fmt clean, clippy -D warnings clean, `cargo test --workspace` 1378 passed / 0
.review/results/d-68-f1-remote-remote-copy.reopened.md:11:This is a d-68 regression for remote-to-local triggers: before this slice, the remote-source branch passed the raw destination string directly to the F3 pull machine, so a Windows `C:/...` local destination was not rejected by the transfer endpoint parser.
.review/results/d-68-f1-remote-remote-copy.reopened.md:24:- `cargo fmt --all -- --check` passed.
.review/results/d-68-f1-remote-remote-copy.reopened.md:25:- `cargo clippy --workspace --all-targets -- -D warnings` passed.
.review/results/d-68-f1-remote-remote-copy.reopened.md:26:- `cargo test --workspace` passed: 565 tests.
.review/results/ue-r2-1d.gpt-verdict.md:45:  fmt clean, clippy clean, tests 1399 passed / 0 failed / 2 ignored
.review/results/ue-r2-1e.gpt-verdict.md:33:  clippy clean, tests 1402 passed / 0 failed / 2 ignored.
.review/results/ue-r2-1f.gpt-verdict.md:11:   passed. **Fix**: every byte tier now asserts its exact lower
.review/results/ue-r2-1f.gpt-verdict.md:24:  clean, tests 1403 passed / 0 failed / 2 ignored.
docs/audit/AUDIT_REPORT_2026-06-04.md:206:**Why this matters**: The very feature that's supposed to prevent deleting an entire module (`feedback-port-cli-safety-guards` rule) is invisibly bypassed by the TUI's silent-filter. Operators get a misleading success signal.
docs/audit/AUDIT_REPORT_2026-06-04.md:381:**Summary**: A canonical chokepoint exists (`blit_core::path_posix::relative_path_to_posix`) but is bypassed by three ad-hoc helpers that disagree on empty-path encoding (push→`""`, pull→`"."`, helper→`""`). The daemon's own wire encoding also splits: push manifests emit `""` for root, but `du`/`find` emit `"."`. The strict wire-path validator (`validate_wire_path`) rejects `"."` as "normalizes to empty" — yet two callers actively produce `"."` for the same logical "module root." Receive sinks' empty-rel single-file guards (designed to avoid `root.join("")` ENOTDIR) silently fail to fire when the renderer emits `"."`.
.review/results/ue-r2-2.gpt-verdict.md:26:   false (the wire tests passed because they hand-build specs).
.review/results/ue-r2-2.gpt-verdict.md:64:6. **Low — tuner idle ticks bypassed the sustain reset** (panel
.review/results/w6-1-progress-event-contract.gpt-verdict.md:23:validation gate fmt + clippy clean, workspace 1460 → 1472 passed / 0
.review/results/w4-5-supports-cancellation-flip.gpt-verdict.md:31:`cargo test --workspace` 37 suites, 1448 passed / 0 failed / 2 ignored
.review/results/w4-4-blocking-work-off-runtime.gpt-verdict.md:38:1472 → 1476 passed / 0 failed / 2 ignored across 37 suites (1475 at
.review/results/w9-3-test-harness-builder.gpt-verdict.md:37:(fmt ✓ / clippy -D warnings ✓ / workspace 1479 passed, 0 failed,
.review/results/w9-3-test-harness-builder.codex.md:14:Review the diff of commit f6e592e (run: git show f6e592e). It implements review-queue row w9-3-test-harness-builder (REVIEW.md design-review queue; slice spec W9.3 in docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md; source findings tests-five-daemon-harness-clones, tests-per-test-cargo-build-subprocess, duplication-cli-test-daemon-harness, tests-fake-server-config-skew in docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md; implementation record .review/findings/w9-3-test-harness-builder.md in the same commit). Acceptance criteria: (1) TestContext::builder() in crates/blit-cli/tests/common/mod.rs expresses the knobs the deleted per-file harness clones existed for (extra daemon args, delegation, second daemon, read_only) and every clone is gone; (2) shared cli_bin()/run_with_timeout replace the pasted copies; (3) the blit-daemon cargo build runs once per test binary via OnceLock while preserving the R16-F1 no-cross-test-ordering property per process; (4) every in-process fake tonic server carries the production HTTP/2 keepalive config via a single shared builder (blit_core::remote::grpc_server) that daemon main.rs also uses, so test/prod server config cannot drift. Check: correctness regressions in the ported tests (semantics of each preserved — assertions, daemon flags, delegation/read_only wiring, timeouts); the new port-collision fix (process-global claimed-port set + child-death readiness check) — is it sound and race-free within one test binary; that the daemon config the harness serializes is behavior-identical for pre-existing consumers (note: the daemon defaults delegation_allowed=true when absent — the harness now serializes explicit true); FAST/SIMPLE/RELIABLE; and that the test count did not drop (same-method A/B in the finding doc: 1478 -> 1479, +1 new keepalive pin test, per-file #[test] counts unchanged). Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
.review/results/w9-3-test-harness-builder.codex.md:353:  mutation-verified); workspace 1476 → 1479/0/2.
.review/results/w9-3-test-harness-builder.codex.md:506:  findings**. +3 tests, mutation-verified; workspace 1476 → 1479/0/2
.review/results/w9-3-test-harness-builder.codex.md:572:    Tests 1478 -> 1479 (same-method A/B vs HEAD), keepalive pin
.review/results/w9-3-test-harness-builder.codex.md:808:+        // be passed through or the daemon lands in the wrong directory.
.review/results/w9-3-test-harness-builder.codex.md:3135: //! radius) would have passed the full validation suite.
.review/results/w9-3-test-harness-builder.codex.md:3716:   125	        // be passed through or the daemon lands in the wrong directory.
.review/results/w9-3-test-harness-builder.codex.md:4779:+  `test result:` line, doc-test suites included, via `git stash`):
.review/results/w9-3-test-harness-builder.codex.md:4780:+  HEAD `3d8326b` = 1478/0/2 across 37 suites → this slice = 1479/0/2
.review/results/w9-3-test-harness-builder.codex.md:4783:+  (STATE's recorded "1479" baseline for design-3 came from a different
.review/results/w9-3-test-harness-builder.codex.md:5729:     9	//! radius) would have passed the full validation suite.
.review/results/w9-3-test-harness-builder.codex.md:6972:/usr/bin/zsh -lc 'rg -n "cargo test|1478|1479|keepalive|TestContext::builder|OnceLock|port" .review/findings/w9-3-test-harness-builder.md REVIEW.md docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md' in /home/michael/dev/Blit
.review/results/w9-3-test-harness-builder.codex.md:7126:REVIEW.md:93:| design-3-unbounded-data-plane-connects | Medium | Both TCP data-plane connects lacked timeouts (audit-2 fix never reached the data plane); hung 60-127s on black-holed ports. Fixed: shared `socket::dial_data_plane` (bounded connect via DATA_PLANE_ACCEPT_TIMEOUT + w1-2 policy + bounded handshake write via DATA_PLANE_TOKEN_TIMEOUT; TimedOut in the chain → is_retryable transient); both sites collapsed (pull connect_pull_stream incl. resize-ADD, push connect_with_probe incl. elastic). +3 tests incl. deterministic stalled-handshake shape pin, mutation-verified; 1476→1479/0/2. Codex PASS (0 findings) | `[x]` | master | `49dcec6` |
.review/results/w9-3-test-harness-builder.codex.md:7154:.review/findings/w9-3-test-harness-builder.md:133:  HEAD `3d8326b` = 1478/0/2 across 37 suites → this slice = 1479/0/2
.review/results/w9-3-test-harness-builder.codex.md:7155:.review/findings/w9-3-test-harness-builder.md:136:  (STATE's recorded "1479" baseline for design-3 came from a different
.review/results/w9-3-test-harness-builder.codex.md:7377:+        // be passed through or the daemon lands in the wrong directory.
docs/audit/inventory/plan-wire.md:1194:No tests verify mtime preservation end-to-end (bug in 946bd77 passed all existing tests because none checked mtimes).
docs/audit/DESIGN_MAP_2026-06-11.md:720:  - divergence: Push client and deprecated Pull use pool_size = streams*2+4, budget = buffer*pool*2. Pull-sync (the live pull path) hardcodes pool_size=4 and a single stream, with literal prefetch 8 passed to DataPlaneSession (pull_sync.rs:641, 765) — tuning.initial_streams/max_streams/prefetch_count/tcp_buffer_size are computed by determine_remote_tuning and then discarded. All four share the inline floor chunk_bytes.max(64*1024), declared independently a fifth time in data_plane.rs:66.
.review/findings/w4-5-supports-cancellation-flip.md:102:rewritten policy/dispatch tests (4 failed / 15 passed under the `cancel`
docs/WHITEPAPER.md:680:  passed all existing tests because none checked mtimes)
docs/audit/findings/drift-perf.md:125:**Plan says**: "The current 'filter parity' workaround that bails on pull when filter args are passed (CLI side)." (PIPELINE_UNIFICATION.md §What this replaces / rejected-filter-parity-pull-bail) — claims this is being REMOVED.
.review/findings/w3-1-memory-aware-buffer-pool.md:115:all pass. Workspace: 1452 → 1460 passed / 0 failed / 2 ignored across
.review/findings/w2-2-stream-ladder-owner.md:17:(`engine::initial_stream_proposal`, byte- **and** file-count-keyed per
.review/findings/w1-4-accept-token-constants.md:60:`-D warnings`), `cargo test --workspace` 1446 passed / 0 failed /
.review/findings/w1-3-tcp-keepalive-honesty.md:70:`-D warnings`), `cargo test --workspace` 1446 passed / 0 failed /
.review/findings/w1-2-data-socket-policy-helper.md:99:`-D warnings`), `cargo test --workspace` 1445 passed / 0 failed /
.review/findings/a0-resolution-fixup.md:23:   passed through the CLI re-export shim but the public library
.review/results/small-file-ceiling-plan.codex.md:120:+stream** — `engine::initial_stream_proposal` is byte-weighted, so
.review/results/small-file-ceiling-plan.codex.md:178:+   `initial_stream_proposal` (and the pull-side equivalent) weight
.review/results/small-file-ceiling-plan.codex.md:282:    40	stream** — `engine::initial_stream_proposal` is byte-weighted, so
.review/results/small-file-ceiling-plan.codex.md:340:    98	   `initial_stream_proposal` (and the pull-side equivalent) weight
.review/results/small-file-ceiling-plan.codex.md:411:docs/DECISIONS.md:105:- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
.review/results/small-file-ceiling-plan.codex.md:495:    51	  Codex: 1 Medium accepted (fake-server bind bypassed the claimed
.review/results/small-file-ceiling-plan.codex.md:496:    52	  set) → fixed. Tests 1478 → 1479/0/2 by same-method A/B.
.review/results/small-file-ceiling-plan.codex.md:635:   191	  bypassed the claimed set) → fixed `8641bc6`; records `c62d15b`.
.review/results/small-file-ceiling-plan.codex.md:636:   192	  Gate: fmt/clippy clean; 1478 → 1479/0/2 same-method A/B; full suite
.review/results/small-file-ceiling-plan.codex.md:753:Never pause merely to have the owner bless code that already passed
.review/results/small-file-ceiling-plan.codex.md:813:- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
.review/results/small-file-ceiling-plan.codex.md:1482:.review/results/w1-4-accept-token-constants.gpt-verdict.md:34:targets, `-D warnings`), `cargo test --workspace` 1446 passed / 0
.review/results/small-file-ceiling-plan.codex.md:1484:.review/results/w1-3-tcp-keepalive-honesty.gpt-verdict.md:36:`-D warnings`), `cargo test --workspace` 1446 passed / 2 ignored, 37
.review/results/small-file-ceiling-plan.codex.md:1705:.review/findings/w1-4-accept-token-constants.md:60:`-D warnings`), `cargo test --workspace` 1446 passed / 0 failed /
.review/results/small-file-ceiling-plan.codex.md:1720:.review/findings/w1-3-tcp-keepalive-honesty.md:70:`-D warnings`), `cargo test --workspace` 1446 passed / 0 failed /
.review/results/small-file-ceiling-plan.codex.md:1799:**2026-07-04 23:35:54Z** - **CODER (w9-3-test-harness-builder, claude)**: Landed w9-3 through the codex loop (owner go: "continue, use /playbook reviewloop codex" — no playbooks exist in this repo, resolved to the `slice` operator per `.agents/repo-guidance.md` → topmost ratified open row per the 19th handoff). A 6-agent inventory workflow re-derived the audit's 2026-06-11 evidence at HEAD before coding and found the rot had GROWN: **seven** daemon-harness clones, not five — w9-4 (`readonly_enforcement.rs`) and w9-5 (`jobs_lifecycle.rs`) each added another private spawn_daemon/config-struct copy *because* common couldn't express delegation or a second daemon, proving the finding's "the next one will miss at least one" prediction twice — plus 5 `cli_bin` copies, 7 `run_with_timeout`, 4 `ChildGuard`, and **five** bare `Server::builder()` fake servers (not three: `remote_remote.rs` ×2, `jobs_lifecycle.rs`, `pull_sync_with_spec_wire.rs` ×2) vs production's audit-1 keepalive. Slice `f6e592e`: common/mod.rs is now the single owner — `TestContext::builder()` (`.read_only()`/`.delegation()`/`.extra_daemon_args()`; `new()`/`new_read_only()` signature-stable, zero edits in the 13 pre-existing consumers), `spawn_daemon(workspace, name, module_dir, opts)` + `TestContext::spawn_second_daemon` primitives (config superset: `delegation_allowed` serialized explicit `true` = the daemon's own absent-default, verified in runtime.rs before choosing; `[delegation]` table optional), `ensure_daemon_built()` OnceLock'ing the nested `cargo build` (R16-F1 per-process independence kept; ~75 invocations per full run → ≤1 per binary; also fixes remote_remote's dropped `--target` handling and the tcp_fallback/jobs/readonly spawns that ran NO build), shared `spawn_fake_blit_server` scaffold, and new `blit_core::remote::grpc_server::production_server_builder()` (owns the 2026-05-23 keepalive 30s/20s; daemon main.rs + all five fakes route through it; zero bare `Server::builder()` left, grep-verified; +1 mutation-verified pin test). Mid-slice the validation run itself caught the **daemon-spawn load-flakiness live**: `test_admin_find` got an empty listing from another test's daemon — `pick_unused_port`'s probe-drop-to-bind TOCTOU, previously masked by the per-test cargo builds serializing bring-ups; fixed two-layer (process-global claimed-port set — cargo runs test binaries sequentially, so per-process scope is exactly right — plus a `try_wait` child-death check in the readiness poll so an externally stolen port panics with the real reason instead of silently testing a foreign daemon). stderr policy unified to null (was piped-but-never-read; real capture stays w9-6). Review: codex **NEEDS FIXES (1 Medium, accepted — a genuinely sharp catch)**: `spawn_fake_blit_server` still bound `:0` OUTSIDE the claimed set, so a fake could take a port promised to a not-yet-bound daemon in mixed binaries (remote_remote, jobs_lifecycle) — same wrong-listener class, missed path; fixed `8641bc6` (`claim_port()` shared by both paths; the fake keeps its probe listener so its path has no gap at all). Records `c62d15b`. Net −1,251 test-tree lines. Validation: fmt/clippy clean; test-count gate proven by same-method A/B via `git stash` — HEAD 1478/0/2, slice 1479/0/2 across 37 suites, exactly +1, per-file `#[test]` counts identical (STATE's recorded "1479" baseline was a different aggregation, off-by-one vs the same tree); full suite ×2 + `admin_verbs` ×10 post-fix all green. All on master, unpushed. Next: strict design-queue order gives **w7-1** (mirror-executor consolidation) as topmost ratified open row; filed alternatives w6-2a/b/c + relay-1, coder's pick.
.review/results/small-file-ceiling-plan.codex.md:1803:**2026-07-04 21:46:39Z** - **CODER (design-3-unbounded-data-plane-connects, claude)**: Landed design-3 through the codex loop (same session, fourth slice; coder's pick of the long-sanctioned smaller alternative over the large w9-3 harness consolidation — queue policy leaves sequencing to the coder). Both TCP data-plane client connects ran unbounded — the audit-2 wave bounded every control-plane connect at 30 s but never reached the data plane, so a firewalled/black-holed data port (the daemon advertises a fresh ephemeral port per transfer; asymmetric firewalls passing 9031 but blocking ephemerals are common) hung for the kernel SYN timeout (60–127 s) with no message. Sites re-verified at HEAD: the pull site is now `connect_pull_stream` (split at ue-r2-2, shared by resize-ADD dials), the push site `DataPlaneSession::connect_with_probe` (elastic dials included). Slice `49dcec6`: `remote::transfer::socket::dial_data_plane(addr, handshake, tcp_buffer_size)` — the client-side mirror of the daemon's bounded accept, in the w1-family policy module: connect bounded by the shared `DATA_PLANE_ACCEPT_TIMEOUT` (the row's sanctioned constant — no fifth 30 s literal), `configure_data_socket` applied, handshake write bounded by `DATA_PLANE_TOKEN_TIMEOUT` (mirrors the acceptor's bounded token read — the finding's "also the token write" clause); on either timeout the chain carries an `io::ErrorKind::TimedOut` source with text naming addr + the likely-firewall cause, so `remote::retry::is_retryable` classifies it transient and `--retry` re-dials. Both call sites collapsed onto the helper; socket.rs's w1-2-era "connect timeouts live at the call sites" module-doc paragraph rewritten (comment-truth). Tests +3 (blit-core 389 → 392): happy path (policy + handshake delivery), deterministic timeout SHAPE via an accepting-but-never-reading peer against a 64 MiB handshake (TimedOut chain + retryable — mutation-verified: swapping the timeout error for a plain eyre message fails the pin), TEST-NET black-hole connect bounded (environment-tolerant: fast-reject networks skip the shape assertions, the bound is asserted always). Review: codex **PASS, zero findings** (independently confirmed the pull resize-ADD non-fatal-dial posture survived and StallGuard/cancellation/byte accounting untouched). Validation: fmt/clippy clean, `cargo test --workspace` 1476 → 1479/0/2 across 37 suites. All on master, unpushed. Session total: w6-1 (+design-1), w6-2 (filed w6-2a/b/c), w4-4, design-3 — four rows closed, six commits of records. Next: w9-3 (test-harness builder) is the topmost ratified open row and the right size for a fresh session; filed alternatives w6-2a/b/c + relay-1.
.review/results/small-file-ceiling-plan.codex.md:1813:**2026-07-04 15:24:23Z** - **CODER (w2-2-stream-ladder-owner, claude)**: Landed w2-2 through the codex loop (owner go: "continue" → topmost open row per the 12th handoff). The row as filed (2026-06-11) predates REV4, which already delivered its three stream-count legs: the `determine_remote_tuning` ladder died at ue-r2-1e (live dial), daemon `desired_streams` at ue-r2-1f (`engine::initial_stream_proposal` — byte- AND file-count-keyed, satisfying the spec's "takes file_count"), and `pull_stream_count` with the Pull RPC at ue-r2-1h; D-2026-06-20-1 recorded the absorption in v1 slice IDs. The remaining leg — the transfer_plan 16/32 MiB chunk ladder — turned out to be **entirely dead policy**, established by a 5-agent audit workflow + hand verification: every remote path overrode it with `Some(dial.chunk_bytes())` (push client 5 refresh sites + ensure_dial, pull_sync both literals); the only paths where the ladder won (local engine, test callers) discarded the value (`PlanUpdate` carries payloads only); the single workspace read of `PlannedPayloads.chunk_bytes` sat behind a `chunk_bytes == 0` guard no live caller can trigger (all pass the dial value, floored ≥ 64 KiB). The spec's "make transfer_plan take chunk_bytes as input" predates the dial — with zero consumers, threading a value through the planner would be plumbing with no reader, so the honest single-owner outcome was deletion. Slice `01209bc`: ladder + `Plan` wrapper deleted (`build_plan` → `Vec<TransferTask>`); `PlannedPayloads` deleted (`plan_transfer_payloads` → `Result<Vec<TransferPayload>>`, ripple through diff_planner/streaming_plan/pipeline tests/re-exports); `PlanOptions.chunk_bytes_override` + all refresh sites deleted (push `plan_options` now immutable default; two arms keep bare `ensure_dial` calls — first-need creation and first-wins ceilings unchanged); unreachable fallback guard in `stream_fallback_from_queue` deleted; `plan_to_daemon_format` deleted (git log -S: never called in repo history — its "server pull mode" comment was never true); orphaned `TuningParams` deleted (producer died at ue-r2-1e); write-only kickoff histogram collapsed to the `total_bytes` accumulator that was its only read. Comment-truth sweep: dial.rs mutability-model doc no longer claims chunk/prefetch are "read at each use site" (consumers snapshot at session/pipeline/batch setup; steps reach epoch-N sockets and later fallback batches); buffer.rs example cites the dial, not `TuningParams`. Behavior byte-identical on every live path. Tests: +4 transfer_plan unit pins (module had zero) — tier classification/interleave, single-small-file no-tar, force_tar single-file, count-target shard splitting with the 128 clamp; deletions are compile-guarded (w2-1 evidence shape); zero tests deleted. Review: codex **NEEDS FIXES (1 Low)** — the first bare ensure_dial comment said "fallback batch" inside the `TransferMode::DataPlane` branch; accepted (mislabel sits exactly on the invariant under review), fixed `27f53a0` (one word). W3.1's "after W2.2 settles the tuning owner" prerequisite is now settled: the owner is `engine::TransferDial`. New discoveries → STATE Open questions: `725aa07` tracked a 236-file stale worktree snapshot (`.claude/worktrees/vigilant-mayer/`) into the repo; WHITEPAPER still describes the pre-dial tuning world (stale since ue-r2-1e, w10 territory). Validation both commits: fmt/clippy clean, `cargo test --workspace` 1452/0/2 across 37 suites (baseline 1448). All on master, unpushed. Next: w3-1 (memory-aware BufferPool) tops the open queue; design-3 remains the sanctioned smaller alternative.
.review/results/small-file-ceiling-plan.codex.md:1817:**2026-07-04 13:53:22Z** - **DECISIONS (owner Q&A, claude)**: The owner asked for the four standing questions "one at a time, no idea what these refer to" — each was presented in plain English with options and answered: (1) **commit erratum → leave as-is** (D-2026-07-04-2; mirrors the D-2026-06-07-1 no-rewrite calculus — two bisect-skippable commits beat force-pushing shared history); (2) **10 GbE session → "soon, but keep coding first"** (STATE Blocked reworded: not a daily blocker; owner will call "benchmark"); (3) **D-2026-06-20-1 stale warmup/size-gate wording → "follow the existing pattern"** — the ledger's own precedent (D-2026-06-20-2's veto annotation, D-2026-06-20-6's struck scope clause) IS edit-in-place-with-annotation, so the superseded framings are struck with pointers to -2 q1 and REV4/-5 (bounded-unilateral untouched — still true), and -5's "remains an open question" note resolved; (4) **supports_cancellation → flip it** (D-2026-07-04-3): CancelJob + TUI F2 will work on attached Push/PullSync transfers; policy-only after w4-3's race wiring; contract change (exit 2→0) recorded; implementation queued as **w4-5-supports-cancellation-flip**, now the topmost open REVIEW.md row. Batch `2a21d6f` through the codex loop per D-2026-07-04-1: **NEEDS FIXES (1 Medium + 1 Low, both STATE.md coherence)** — the Now bullet still called the erratum an open owner call, and the queue rewrite dropped the coder's-pick clause (design-3-vs-w2-2 ordering contradiction); both accepted, fixed `a928193`. The decision content itself passed all cross-checks (ledger consistency, w4-3 scope-note agreement, strike precision). check-docs.sh green. All on master, unpushed.
.review/results/small-file-ceiling-plan.codex.md:1831:**2026-07-03 19:33:59Z** - **CODER (ue-r2-1g, claude)**: Landed PullSync multistream through the engine (REV4 `ue-r2-1g`, seventh slice through the code→GPT-review→fix loop; absorbs `MULTISTREAM_PULL.md`). Key discovery that shaped the slice: the CLIENT side of multistream PullSync already existed — `pull_sync_with_spec` has routed negotiations through the `stream_count`-honoring fan-out (`receive_data_plane_streams_owned`) since `69d8599` (2025-11-15), months before any client sent a capacity profile (`a0d2c9f`, 1e) — proven from git history, so the profile-presence gate cannot strand any committed client. The slice (`48e583e`) is therefore daemon-side: `negotiated_pull_streams` proposes from `engine::initial_stream_proposal` (the daemon is the byte sender AND shape-knower on pull; the engine fn's doc now states the proposer is the shape-knowing end either direction), gated on the client's advertised `receiver_capacity` (absent/unknown `max_streams` → 1 stream, today's behavior byte-for-byte per REV4 Design §5), recorded on the dial; `accept_and_wrap_sinks` (accept N, bounded token auth, N `DataPlaneSink`s) HARVESTED verbatim from the deprecated Pull RPC into `pull_sync.rs` (the deprecated handlers borrow it back until 1h deletes them); the 1a elastic work-stealing pipeline does the fan-out across N sinks. NO proto changes. Resume keeps its dedicated single-stream path (ordered JIT block-hash protocol — explicit RELIABLE exception); gRPC fallback untouched; delegated daemon→daemon inherits free via the dst-stamped profile. Deliberate deltas called out: prefetch 8→`dial.prefetch_count()`, pool scales with streams. Pull 1s-start explicitly NOT met and cannot be yet — the shape-keyed proposal inherently follows the full scan; it rides on `ue-r2-2` resize (recorded in Known gaps, not silently skipped). Review: codex **NEEDS FIXES → 2 accepted + fixed** (`4a2e58d`): cancellation-mid-transfer test with live sockets (TCP-level teardown observability) + dial bookkeeping on the conservative arm. Additionally ran a 3-lens adversarial self-review panel (concurrency/compat/RELIABLE): 2 more accepted + fixed same commit — client now clamps daemon-advertised `stream_count` to its own advertised ceiling (`bounded_stream_count`, REV4 §4 "weak end protects itself" made real receive-side) and the harvested helper's token-mismatch status restored to UNAUTHENTICATED (pull_sync wire behavior pre-slice-exact; the delta moved to the deprecated path) — 1 deferred (sequential-accept pin growth ~N×, bounded + precedented → W1 socket-policy row). e2e proves >1 stream observably (300 files → 2 streams, marker + byte-identical, revert-proven). Validation: fmt/clippy clean, `cargo test --workspace` **1413 / 0 / 2** (baseline 1403; +8 slice, +2 review). All on master, unpushed (origin at `7603177`). Ladder #3 (`pull_stream_count`) now dies with its RPC at `1h`. Next: `ue-r2-1h` (delete deprecated Pull RPC; must relocate `PullEntry`/`collect_pull_entries_with_checksums` — noted in the finding doc).
.review/results/small-file-ceiling-plan.codex.md:1833:**2026-07-03 18:32:57Z** - **CODER (ue-r2-1f, claude)**: Landed push convergence (REV4 `ue-r2-1f`, sixth slice through the code→GPT-review→fix loop). The daemon-push `desired_streams` ladder — the one the old `tuning.rs` doc said "wins" — is retired into the engine (`a4a9f70`): `engine::initial_stream_proposal` carries the shape table verbatim (bytes OR file-count keyed), clamped to the proposer's advertised receiver ceiling; both daemon negotiation sites call it; the private ladder is deleted (it had zero tests — the engine fn now has full tier-boundary coverage, extended ±1 per review). Wire-identical negotiations today (table max 16 < ceiling 32); the client's dial still clamps sender-side (1e). Second of three ladders gone; `pull_stream_count` retires at 1g/1h. The finding doc states the interpretation of "route push through the engine" explicitly (push's gRPC manifest/need-list loop = protocol boundary per REV4 Design §1's own list; the slice's substance = decision-layer ownership) and put it to the reviewer: codex **PASS with one Low** (boundary-value test gap, fixed `0c8da50`) and explicitly judged the interpretation **plan-conformant**. Validation: fmt/clippy clean, `cargo test --workspace` 1403 / 0 / 2. All on master, unpushed (origin at `7603177`). Next: `ue-r2-1g` (PullSync multistream through the engine).
.review/results/small-file-ceiling-plan.codex.md:1841:**2026-07-03 15:43:45Z** - **CODER (ue-r2-1b, claude)**: Landed the wire dial contract (REV4 `ue-r2-1b`, second slice through the code→GPT-review→fix loop, D-2026-06-20-6). Proto (`2741dc8`): new `CapacityProfile` (cpu_cores, `DrainClass` enum, load_percent, max_streams, drain_rate_bytes_per_sec, max_chunk_bytes, max_inflight_bytes; 0 = unknown = stay conservative) carried as `DataTransferNegotiation.receiver_capacity = 11` (push: daemon is byte receiver) and `TransferOperationSpec.receiver_capacity = 12` (pull_sync/delegated: client/dst is byte receiver) — **spec_version deliberately stays 2** (exact-match gate at `operation_spec.rs:107` means a bump would make old daemons reject new clients; the profile is a skippable hint, unlike v2's safety-critical field). Daemon-authoritative `resize_enabled = 12` + `epoch0_sub_token = 13` on the negotiation; capability bits `PushHeader.supports_stream_resize = 8` / `PeerCapabilities.supports_stream_resize = 5` (all false until ue-r2-2); `DataPlaneResize`/`DataPlaneResizeAck` from adaptive-PR3 prior art (`d9d4ec7`) as oneof variants in all four control streams (ClientPushRequest=9, ServerPushResponse=5, ClientPullMessage=5, ServerPullMessage=16). PR3's field-number clash (its negotiation 11-14) resolved: min/max stream bounds subsumed by `CapacityProfile.max_streams`, floor 1. Zero behavior: all literals stamped with defaults; new variants ignored on receive like unknown payloads; one intentional semantic addition — the delegated dst override now also strips CLI-supplied `receiver_capacity` (R25-F2 boundary; prevents a fabricated ceiling leaking once ue-r2-1e reads the field). Compat tests (`crates/blit-core/tests/proto_wire_compat.rs`, first use of test-local `#[derive(prost::Message)]` old-shape replicas): old→new + new→old for negotiation/spec/PushHeader/caps including normalization through the real `from_spec` chokepoint; resize frames decode as `payload: None` on old peers for all four oneofs (with known-variant controls); new↔new round trips. Review: codex/GPT-5.5 **PASS, zero findings**; supplementary 4-lens adversarial self-review (ultracode session) found 1 Low, accepted — the `receiver_capacity` comment falsely implied deprecated Pull carries a client→daemon profile (PullRequest has no spec channel); comment fixed in `5bd345a`. Validation: fmt/clippy clean, `cargo test --workspace` 1391 passed / 0 failed / 2 ignored (baseline 1378, +13). All on master, unpushed. Next: `ue-r2-1c` (engine shell + local adapter).
.review/results/small-file-ceiling-plan.codex.md:1843:**2026-06-21 03:02:29Z** - **CODER (ue-r2-1a, claude)**: Landed the adaptive-streams substrate (REV4 `ue-r2-1a`, first slice; first end-to-end run of the code→GPT-review→fix loop, D-2026-06-20-6). Cherry-picked over the `-s ours` octopus trap (D-2026-06-07-2, where a plain merge no-ops): PR1 per-stream telemetry zero-cost `Probe` (`e569eea`), PR2 shared work-stealing flume queue (`3844a15`), PR2 forwarder-halt-on-error fix (`ec561f2`). Hand-resolved the `data_plane.rs` StallGuardWriter-vs-`Probe` conflict (compose: stream stays `StallGuardWriter<TcpStream>`, struct gains generic `<P: Probe = NoProbe>`, `from_stream_with_probe` wraps the guard) and `mod.rs` re-exports (dropped `Phase`/`TransferProgress`/`TransferProgressSnapshot` — master had removed them; added telemetry types + the `AtomicU8` import). Excluded `eafb187` (doc-shuffle; embeds `C:/Users` paths) and `d9d4ec7` (PR3 WIP, does not build). Work-stealing behaviour tests added (`771a632`): byte/file exactly-once accounting + producer-cancel graceful wind-down. codex/GPT-5.5 read-only review → fix-then-ship, 4 findings, all accepted, fixed in `90ed43d`: F1 (High→Medium) workers re-check `cancelled` before each recv (bounds survivor work after first error; interrupting in-flight / hard-abort-on-drop stays w4-1); F2 `send_block` now calls `probe.record_bytes`; F3 exactly-once path assertion; F4 multi-sink cancel-under-backpressure test. Carried to `ue-r2-1e`: PR1 `write_blocked_nanos` `join!` over-measure (Medium) + tar-shard timing (Low) — telemetry has no live consumer until the dial. Validation at each step: fmt/clippy clean, `cargo test --workspace` 1378 passed / 0 failed / 2 ignored (baseline 1370). All on master, unpushed.
.review/results/small-file-ceiling-plan.codex.md:1849:**2026-06-12 16:32:00Z** - **REVIEW (gemini-reviewer)**: Graded the two pending sentinels on `master` sequentially: **design-4-fallback-midmanifest-negotiation** (`ddfeb58`) accepted (daemon early-flush branch is now TCP-only; client-side fallback_negotiated flag added to hold need-list/manifest fallback payloads in pending_queue; 2000-file regression test passes) and **design-5-send-failure-masks-rejection** (`08d71a2`) accepted (prefer_server_error helper harvests daemon's terminal status on manifest-phase send failures; 500-file readonly_enforcement regression test passes repeatedly). Both verified.json verdicts created, sentinels deleted via git rm, and REVIEW.md status rows updated to `[x]`. Validation suite re-run: cargo fmt, clippy, and cargo test --workspace all passed successfully (1370 passed, 0 failed, 1 ignored).
.review/results/small-file-ceiling-plan.codex.md:1869:**2026-06-11 00:42:30Z** - **REVIEW**: Verification and acceptance of `audit-h3c-slice1-grpc-fallback-frame-contract` (commit `bf4cc82`, verdict declared by owner). Validation suite re-run at HEAD `1be16bc`: fmt/clippy/test all green (exit 0), blit-core test-function count identical at `bf4cc82` and HEAD (344) — no test drop. The review assessment surfaced four facts the finding doc didn't know, recorded here as input to the slice-2 re-scope: **(1)** the 1 MiB frame cap is load-bearing correctness, not just cadence — tonic 0.14's default 4 MiB decode limit (`DEFAULT_MAX_RECV_MESSAGE_SIZE`, no `max_decoding_message_size` override anywhere in the workspace) means pre-slice-1 gRPC-fallback transfers of files >4 MiB should fail at decode, since `tuning.rs` emits 16/32/64 MiB chunks and `pull_sync.rs:515` passed `tuning.chunk_bytes` straight into `GrpcServerStreamingSink`. Version-skew residual: a new client against an old daemon still receives uncapped frames. **(2)** The bigger H3 hole is missing client-side HTTP/2 keepalive: daemon pings clients (`main.rs:138`) but all three client channel builders (`pull.rs:239`, `push/client/mod.rs:307`, `blit-app/src/client.rs:38` — already-drifted near-duplicates) set only `connect_timeout`, so a dead daemon hangs `message().await` forever; ~3 lines of `Endpoint` keepalive config cover dead-peer/black-hole at transport level and shrink slice 2's watchdog scope to wedged-but-alive peers only. **(3)** Fallback throughput is governed by HTTP/2 flow-control windows, not frame size — hyper defaults: client 2 MiB stream / 5 MiB conn; daemon server 1 MiB conn total → hardware-independent push-fallback ceiling on high-BDP links; `http2_adaptive_window(true)` is the no-tuning fix. **(4)** `clamp_fallback_chunk_size(x.max(1MiB))` ≡ const 1 MiB — the sinks' `chunk_bytes` param is now inert and misleading; root cause is a TCP transport parameter (`chunk_bytes`) embedded in the transport-agnostic `PlannedPayloads` (planner payload composition itself is sound). Owner direction: slice 2 re-scope deferred until after a repo-wide design-coherence review (plan doc next). Sentinel cleared, `REVIEW.md` updated to verified (`[x]`).
.review/results/small-file-ceiling-plan.codex.md:1897:**2026-05-05 20:30:00Z** - **FIX**: Round 45 review of commit `f83a208`. **R45-F1 (High)**: while plumbing scanned features I left a `let total_bytes = scanned_bytes` alias in `crates/blit-core/src/orchestrator/orchestrator.rs:480` and the streaming summary at `:567` then read that local for `summary.total_bytes`. Net effect on the wire: `summary.total_bytes` (contracted as "bytes the pipeline wrote") was reporting scanned bytes, so any incremental skip-unchanged run with all-up-to-date files would have reported the full source size as bytes-written and overcounted throughput. Fix: removed the alias; streaming summary now reads `pipeline_outcome.bytes_written` from `SinkOutcome` directly. The verbose "Planning enumerated" line correctly references `scanned_bytes`. **R45-F2 (Low)**: planner + transfer sub-predictions were still passing `(all_headers.len(), total_bytes)` (which were aliases of scanned_* but only by accident) while the total prediction passed `(scanned_files, scanned_bytes)` explicitly. Made all three predictor calls share the explicit `(scanned_files, scanned_bytes)` feature vector so a future maintainer editing one branch can't reintroduce drift by missing another. New regression test `incremental_run_total_bytes_excludes_skipped_files` in `orchestrator::orchestrator::async_runtime_tests` runs the full local mirror pipeline twice (mirror=true forces streaming planner; skip_unchanged=true means run 2 writes 0 bytes) and asserts `summary.total_bytes == 0` on the second run. Verified the test catches the bug by reintroducing the alias — assertion fires with `total_bytes=4096, scanned_bytes=4096` matching the pre-R45 state. **Tests**: `cargo test --workspace` 407 pass / 0 fail (was 406, +1 new R45 regression test).
.review/results/small-file-ceiling-plan.codex.md:1925:**2026-05-03 01:05:00Z** - **FIX**: Closed Round 37 findings from `docs/reviews/followup_review_2026-05-02.md`. **R37-F1**: `RemotePullClient::pull_sync_with_spec` now preserves a typed `PullSyncError::Negotiation` for source-side pull-sync refusals before negotiation completes, and the destination `DelegatedPull` handler maps that to `DelegatedPullError::NEGOTIATE` instead of generic `TRANSFER`. Added core wire test coverage for initial RPC rejection classification and remote→remote CLI integration coverage where a fake source rejects `pull_sync`; the CLI surfaces "source refused delegated pull" and relay counters stay zero. **R37-F2**: delegated `BytesProgress` is now treated as cumulative state and converted to deltas before feeding the CLI progress accumulator; duplicate cumulative updates are not double-counted. Proto comments now document the cumulative semantics. **Tests**: `cargo fmt`; `cargo fmt -- --check`; `cargo test -p blit-core pull_sync_with_spec`; `cargo test -p blit-cli remote_remote_direct`; `cargo test -p blit-cli --test remote_remote`; `cargo test -p blit-daemon delegated_pull`; `cargo test --workspace` (all passed; existing macOS FSEvents deprecation/F14 warnings and the pre-existing macOS test unused-variable warning remain).
.review/results/small-file-ceiling-plan.codex.md:1927:**2026-05-03 00:30:00Z** - **ACTION**: Implemented Phase 2 of `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`. Remote→remote `copy`/`mirror`/`move` now dispatch to destination-side `DelegatedPull` by default, with `--relay-via-cli` as the only legacy relay escape hatch. Added `crates/blit-cli/src/transfers/remote_remote_direct.rs` to build the same `TransferOperationSpec` as pull-sync, call `DelegatedPull`, surface stale-daemon/gate/connect/transfer errors without silent fallback, and print delegated summaries. Added env-gated test instrumentation in `blit-core::remote::instrumentation` so integration tests can assert the CLI sent zero data-plane payload bytes and never constructed `RemoteTransferSource` on the direct path, while `--relay-via-cli` does both. Reworked `crates/blit-cli/tests/remote_remote.rs` to cover direct byte-path isolation, gate-rejection no-fallback, stale destination `Unimplemented` no-fallback, and explicit relay. Updated CLI docs/help for `--relay-via-cli`. **Tests**: `cargo fmt`; `cargo check -p blit-cli`; `cargo test -p blit-cli remote_remote_direct`; `cargo test -p blit-cli --test remote_remote`; `cargo test -p blit-core pull_sync_with_spec`; `cargo test -p blit-daemon delegated_pull`; `cargo test --workspace` (all passed; existing macOS FSEvents deprecation/F14 warnings remain).
.review/results/small-file-ceiling-plan.codex.md:1947:**2026-05-02 15:45:00Z** - **ACTION**: Closed Round 8 review findings R8-F1 and R8-F2 from `docs/reviews/followup_review_2026-05-02.md`. Both bugs were in the daemon's gRPC push fallback receive loop (`receive_fallback_data` in `crates/blit-daemon/src/service/push/data_plane.rs`) — the framing layer that runs *before* the shared `tar_safety` helper sees the buffer. **R8-F1** (Medium — DoS): the chunk accumulator was capped only by an 8 MiB initial reservation; the actual `Vec` grew unbounded. `archive_size == 0` skipped the overflow check entirely, and `archive_size > MAX_TAR_SHARD_BYTES` was bypassed at the framing layer. Reject zero archive_size when files are present, reject `archive_size > MAX_TAR_SHARD_BYTES` at TarShardHeader, and enforce both the declared size and the local cap on every chunk via a new `check_fallback_chunk_overflow` helper. **R8-F2** (Medium — silent partial success): stream EOF was treated as graceful end. But FileManifest and TarShardHeader remove entries from `pending` before bytes arrive, so a client could send a header, close the stream, and the final `pending.is_empty()` check would pass despite no data ever landing. Track `upload_complete_seen` and bail on EOF without UploadComplete or with `active.is_some()`. Extracted `validate_fallback_shard_archive_size` and `check_fallback_chunk_overflow` as private helpers so the rules are unit-testable without spinning up a full gRPC server. 7 new framing tests pin every boundary condition (zero size, at-cap, above cap, declared overflow, local-cap overflow, u64 overflow, within-bounds). **Tests**: `cargo fmt`; `cargo test --workspace` (166 lib + 17 daemon + integration suites, all green).
.review/results/small-file-ceiling-plan.codex.md:1971:**2026-05-02 01:15:00Z** - **F1 (path safety)**: Centralized receive-side path validation in `blit-core/src/path_safety.rs`. Two helpers: `validate_wire_path(s) -> PathBuf` (pure validation, normalized form) and `safe_join(root, s) -> PathBuf` (validate + join, with empty input returning root unchanged for the single-file destination case). Rejects: `..` components (not just substrings — the previous tar-shard `rel.contains("..")` check was too weak and also rejected legitimate filenames containing literal `..`), absolute paths, Windows drive prefixes (`C:\…`), UNC and DOS device paths (`\\?\…`, `\\server\share\…`), root components, embedded NUL bytes. Migrated `pull.rs::sanitize_relative_path` and daemon `service/util.rs::resolve_relative_path` / `resolve_manifest_relative_path` to thin wrappers that delegate to the shared module — single chokepoint, no drift between sanitizers. Applied at all five receive-side sites the review identified: `sink.rs:218` (streamed file write), `sink.rs:404` + `:426` (tar shard entry validation + per-entry join), `sink.rs:462` (resume block write), `sink.rs:498` (block completion), and the gRPC fallback block-hash path in `pull.rs:532`. Wire-boundary defense in depth: `pipeline.rs::read_file_header` and per-entry path read in `read_tar_shard` validate at the moment paths arrive off the socket, so unsafe headers never enter the FileHeader stream. **Tests**: 209 passed (was 185 baseline) — 16 new `path_safety::tests` covering the validator surface (`..`, absolute Unix, Windows drive, UNC, NUL, valid `..`-containing filenames, deep paths, Unicode, trailing slash, just-dot), 8 new sink-level integration tests confirming `FsTransferSink::write_file_stream` rejects malicious wire paths before any filesystem write and that the legitimate single-file empty-path case still works. **Verification**: `cargo check --workspace` clean; `cargo test --workspace` 0 failures.
.review/results/small-file-ceiling-plan.codex.md:1975:**2026-05-01 23:43:57Z** - **REVIEW**: Completed a repo-level codebase review and saved it to `docs/reviews/codebase_review_2026-05-01.md`. Highest-priority findings: receive-side path sanitization is incomplete across streamed files, tar shards, and resume block records; daemon `use_chroot` is documented/configured but not enforced; filtered mirror delete semantics need an explicit product decision; metrics lifecycle and endpoint behavior need hardening. Added a short `TODO.md` follow-up section and marked Phase 4 as in progress with the review artifact linked. **Verification**: `cargo check --workspace` passed; `cargo test --workspace` passed with 185 tests after rerunning outside the sandbox to allow local test port binding.
.review/results/small-file-ceiling-plan.codex.md:1977:**2026-04-24 03:15:00Z** - **FIX**: Fixed single-file remote push crash — `blit copy FILE server:/mod/` previously died with "opening FILE/ during payload planning: Not a directory". Root cause was two-sided: on the client, `filter_readable_headers` and `build_tar_shard` computed `source_root.join(&rel)` where `rel=""` for single-file sources, which on Unix preserves a trailing `/` and makes `File::open` reject with ENOTDIR. On the daemon, `resolve_relative_path` folded empty paths to `"."` (which broke `module.path.join("")` = wanted single file, but got `module.path/.`), and the subsequent `module.path.join(empty_rel)` added another trailing `/`. Fix applies the `rel.is_empty() → use root directly` pattern in three places on the client side (matching `FsTransferSource::open_file`'s existing handling), and adds two new helpers on the daemon side: `resolve_manifest_relative_path` preserves empty as empty for per-file manifest entries, and `resolve_dest_path(base, rel)` handles the empty-rel join case. Swapped all four push-side `resolve_relative_path` callers and three `module.path.join` sites. Regression tests in `crates/blit-cli/tests/remote_push_single_file.rs` pin both the container-dir (`blit copy FILE server:/mod/`) and rename (`blit copy FILE server:/mod/new.txt`) cases. Annotated all four `docs/bugs/*.md` files with their resolution status. **Tests**: 173 passed (+2 remote push single-file), clippy clean.
.review/results/small-file-ceiling-plan.codex.md:1979:**2026-04-24 02:54:00Z** - **ACTION**: Completed the remaining UX feedback items #4, #5, #6, #7, #8, #9, #10 from `docs/ux-feedback-migrate-games-poc.md`. #4 TTY auto-progress: `TransferArgs::effective_progress()` enables the progress indicator when stdout is a TTY and `--json` is off, matching rsync/rclone; piped stdout stays silent for scripts. #8 `--help` rsync docs: `after_long_help` on `TransferArgs` appends a PATHS section summarizing the trailing-slash rules to `blit copy/mirror/move --help`. #6/#7/#5 banner cleanup: demoted the `starting copy` line to stderr and dropped the version prefix; added `collapse_slashes()` display-only normalization so script-appended `//` no longer shows in the banner (semantics unchanged); under `-v`, print a follow-up line when rsync resolution materially rewrote the destination. #9 flag grouping: split `TransferArgs` flags across clap `help_heading` buckets (Options / Comparison / Reliability / Performance/debug) so first-time users see common flags before niche debug knobs. #10 `blit diagnostics dump SRC DST [--json]`: new subcommand emits a pasteable snapshot (version, invocation argv, parsed endpoints, rsync resolution, local filesystem caps, free/total disk, same-device check) without performing a transfer — built for one-shot bug reports. Added `sysinfo = "0.31"` (disk-only) to blit-cli, exposed `source_is_contents`/`dest_is_container` as `pub(crate)`, wrote 3 smoke tests pinning the JSON shape. Updated `blit(1)` manpage SYNOPSIS and DIAGNOSTICS sections. **Tests**: 170 passed (added 3), clippy clean.
.review/results/small-file-ceiling-plan.codex.md:1981:**2026-04-24 02:32:00Z** - **ACTION**: Addressed UX feedback items #1 and #3 from `docs/ux-feedback-migrate-games-poc.md`. Added `TransferOutcome` enum (`Transferred` / `JournalSkip` / `UpToDate` / `SourceEmpty`) to `LocalMirrorSummary` so the CLI can emit distinct, honest messages for the three legitimate zero-files cases instead of the ambiguous `Copy complete: 0 files, 0 B`. `FastPathDecision::NoWork` now carries an `examined` count to split empty-source from already-up-to-date; the single-file path also sets `UpToDate` when `skip_unchanged` short-circuits. CLI `print_summary` in `crates/blit-cli/src/transfers/local.rs` matches on outcome and suppresses the `• Throughput / Workers used` line unless verbose, ≥1 MiB, or `>1` file copied. JSON output now includes `outcome` and `files_examined`. Strengthened `single_file_copy_idempotent` to pin `Copy complete: 1 files` on first run and `Up to date` on second run — would have caught the regression where the single-file rerun emitted misleading `Copy complete: 0 files`. **Tests**: 167 passed, clippy clean.
.review/results/small-file-ceiling-plan.codex.md:1983:**2026-04-14 20:00:00Z** - **ACTION**: Completed unified-pipeline remediation across all transfer paths. Previously only local→local and gRPC-fallback push routed through `execute_sink_pipeline`; multi-stream TCP push and both daemon pull paths had bespoke dispatch code. Added `execute_sink_pipeline_streaming(source, sinks, payload_rx, prefetch, progress)` in `crates/blit-core/src/remote/transfer/pipeline.rs` and made the one-shot form a thin wrapper. Rewrote `MultiStreamSender` in `crates/blit-core/src/remote/push/client/mod.rs` as a lightweight `payload_tx` + `JoinHandle` shell — deleted `data_plane_sink_worker`, batch-splitting helpers, and `StreamStats` (~130 lines). Converted both daemon pull paths in `crates/blit-daemon/src/service/pull.rs` to use the pipeline; deleted `handle_pull_stream` and `handle_pull_stream_streaming` (~125 lines). Deleted dead modules: `transfer_engine.rs` (WorkerFactory trait + task-channel infra), `transfer_facade/` (TaskAggregator, PlannerEvent, build_local_plan, stream_local_plan), and `RetryableTask` from `transfer_plan.rs` — 1139 lines total. Updated `docs/ARCHITECTURE.md` with the unified-pipeline diagram and source/sink matrix. Added `blit(1)` command-summary hints and expanded `TransferArgs` doc-comments for `--help`. **Tests**: 152 passed (87 core + 65 integration), clippy clean. **Remaining**: end-to-end 10 GbE benchmark runs.
.review/results/small-file-ceiling-plan.codex.md:1989:**2026-04-07 16:00:00Z** - **ACTION**: Completed blit-utils hardening (P1). Added 21 integration tests in `crates/blit-cli/tests/blit_utils.rs` covering all 9 blit-utils commands: scan (mDNS exit), list-modules (text+JSON), ls (remote/local/JSON), find (pattern/JSON/dirs-only/limit), du (text+JSON), df (text with human-readable check, JSON), rm (file/directory/module-root refusal), completions (prefix/dirs-only), profile (text+JSON). Added human-readable byte formatting to `df` output using `format_bytes()` alongside raw byte counts. **Tests**: `cargo test -p blit-cli --test blit_utils` (21 passed); `cargo test -p blit-cli --test admin_verbs` (10 passed).
.review/results/small-file-ceiling-plan.codex.md:2003:**2025-01-29 12:00:00Z** - **ACTION**: Added resume integration tests (`remote_resume.rs`) covering partial file resume, identical file optimization, and gRPC fallback path. Security hardening: MAX_BLOCK_SIZE (64 MiB) enforced on both client and server to prevent OOM attacks from malicious block_size values. Export added to `copy/mod.rs`. Updated TODO.md to mark P0 parity tests and refactoring items as complete. **Tests**: `cargo test --workspace` (64 passed).
.review/results/small-file-ceiling-plan.codex.md:2005:**2025-01-29 10:00:00Z** - **FIX**: Addressed Gemini code review findings for remote block-level resume. (1) Fixed critical memory vulnerability: `compute_block_hashes` in `pull.rs` and both `stream_via_data_plane_resume`/`stream_via_block_resume_grpc` in `pull_sync.rs` now stream files in block-sized chunks instead of loading entire files into memory. (2) Fixed stop-and-wait protocol in data plane path: `stream_via_data_plane_resume` now pipelines all block hash requests upfront (Phase 1), collects all responses (Phase 2), then processes files (Phase 3) - eliminates per-file RTT penalty. gRPC fallback keeps simpler stop-and-wait since it's for diagnostic use only. **Tests**: `cargo build --package blit-daemon --package blit-cli` (passed).
.review/results/small-file-ceiling-plan.codex.md:2007:**2025-01-28 16:00:00Z** - **ACTION**: Implemented remote block-level resume for TCP data plane (primary path). Added `DATA_PLANE_RECORD_BLOCK` and `DATA_PLANE_RECORD_BLOCK_COMPLETE` record types to data plane protocol. Server-side: `stream_via_data_plane_resume` uses gRPC for block hash exchange then TCP data plane for block transfer - combines best of both worlds. Client-side: `handle_block_record` and `handle_block_complete_record` in data plane receiver handle seek+write for in-place block updates. Remote resume now works with default `--resume` flag (no `--force-grpc` needed). **Tests**: `cargo test -p blit-core` (63 passed).
.review/results/small-file-ceiling-plan.codex.md:2021:**2026-07-04 23:35:54Z** - **CODER (w9-3-test-harness-builder, claude)**: Landed w9-3 through the codex loop (owner go: "continue, use /playbook reviewloop codex" — no playbooks exist in this repo, resolved to the `slice` operator per `.agents/repo-guidance.md` → topmost ratified open row per the 19th handoff). A 6-agent inventory workflow re-derived the audit's 2026-06-11 evidence at HEAD before coding and found the rot had GROWN: **seven** daemon-harness clones, not five — w9-4 (`readonly_enforcement.rs`) and w9-5 (`jobs_lifecycle.rs`) each added another private spawn_daemon/config-struct copy *because* common couldn't express delegation or a second daemon, proving the finding's "the next one will miss at least one" prediction twice — plus 5 `cli_bin` copies, 7 `run_with_timeout`, 4 `ChildGuard`, and **five** bare `Server::builder()` fake servers (not three: `remote_remote.rs` ×2, `jobs_lifecycle.rs`, `pull_sync_with_spec_wire.rs` ×2) vs production's audit-1 keepalive. Slice `f6e592e`: common/mod.rs is now the single owner — `TestContext::builder()` (`.read_only()`/`.delegation()`/`.extra_daemon_args()`; `new()`/`new_read_only()` signature-stable, zero edits in the 13 pre-existing consumers), `spawn_daemon(workspace, name, module_dir, opts)` + `TestContext::spawn_second_daemon` primitives (config superset: `delegation_allowed` serialized explicit `true` = the daemon's own absent-default, verified in runtime.rs before choosing; `[delegation]` table optional), `ensure_daemon_built()` OnceLock'ing the nested `cargo build` (R16-F1 per-process independence kept; ~75 invocations per full run → ≤1 per binary; also fixes remote_remote's dropped `--target` handling and the tcp_fallback/jobs/readonly spawns that ran NO build), shared `spawn_fake_blit_server` scaffold, and new `blit_core::remote::grpc_server::production_server_builder()` (owns the 2026-05-23 keepalive 30s/20s; daemon main.rs + all five fakes route through it; zero bare `Server::builder()` left, grep-verified; +1 mutation-verified pin test). Mid-slice the validation run itself caught the **daemon-spawn load-flakiness live**: `test_admin_find` got an empty listing from another test's daemon — `pick_unused_port`'s probe-drop-to-bind TOCTOU, previously masked by the per-test cargo builds serializing bring-ups; fixed two-layer (process-global claimed-port set — cargo runs test binaries sequentially, so per-process scope is exactly right — plus a `try_wait` child-death check in the readiness poll so an externally stolen port panics with the real reason instead of silently testing a foreign daemon). stderr policy unified to null (was piped-but-never-read; real capture stays w9-6). Review: codex **NEEDS FIXES (1 Medium, accepted — a genuinely sharp catch)**: `spawn_fake_blit_server` still bound `:0` OUTSIDE the claimed set, so a fake could take a port promised to a not-yet-bound daemon in mixed binaries (remote_remote, jobs_lifecycle) — same wrong-listener class, missed path; fixed `8641bc6` (`claim_port()` shared by both paths; the fake keeps its probe listener so its path has no gap at all). Records `c62d15b`. Net −1,251 test-tree lines. Validation: fmt/clippy clean; test-count gate proven by same-method A/B via `git stash` — HEAD 1478/0/2, slice 1479/0/2 across 37 suites, exactly +1, per-file `#[test]` counts identical (STATE's recorded "1479" baseline was a different aggregation, off-by-one vs the same tree); full suite ×2 + `admin_verbs` ×10 post-fix all green. All on master, unpushed. Next: strict design-queue order gives **w7-1** (mirror-executor consolidation) as topmost ratified open row; filed alternatives w6-2a/b/c + relay-1, coder's pick.
.review/results/small-file-ceiling-plan.codex.md:2025:**2026-07-04 21:46:39Z** - **CODER (design-3-unbounded-data-plane-connects, claude)**: Landed design-3 through the codex loop (same session, fourth slice; coder's pick of the long-sanctioned smaller alternative over the large w9-3 harness consolidation — queue policy leaves sequencing to the coder). Both TCP data-plane client connects ran unbounded — the audit-2 wave bounded every control-plane connect at 30 s but never reached the data plane, so a firewalled/black-holed data port (the daemon advertises a fresh ephemeral port per transfer; asymmetric firewalls passing 9031 but blocking ephemerals are common) hung for the kernel SYN timeout (60–127 s) with no message. Sites re-verified at HEAD: the pull site is now `connect_pull_stream` (split at ue-r2-2, shared by resize-ADD dials), the push site `DataPlaneSession::connect_with_probe` (elastic dials included). Slice `49dcec6`: `remote::transfer::socket::dial_data_plane(addr, handshake, tcp_buffer_size)` — the client-side mirror of the daemon's bounded accept, in the w1-family policy module: connect bounded by the shared `DATA_PLANE_ACCEPT_TIMEOUT` (the row's sanctioned constant — no fifth 30 s literal), `configure_data_socket` applied, handshake write bounded by `DATA_PLANE_TOKEN_TIMEOUT` (mirrors the acceptor's bounded token read — the finding's "also the token write" clause); on either timeout the chain carries an `io::ErrorKind::TimedOut` source with text naming addr + the likely-firewall cause, so `remote::retry::is_retryable` classifies it transient and `--retry` re-dials. Both call sites collapsed onto the helper; socket.rs's w1-2-era "connect timeouts live at the call sites" module-doc paragraph rewritten (comment-truth). Tests +3 (blit-core 389 → 392): happy path (policy + handshake delivery), deterministic timeout SHAPE via an accepting-but-never-reading peer against a 64 MiB handshake (TimedOut chain + retryable — mutation-verified: swapping the timeout error for a plain eyre message fails the pin), TEST-NET black-hole connect bounded (environment-tolerant: fast-reject networks skip the shape assertions, the bound is asserted always). Review: codex **PASS, zero findings** (independently confirmed the pull resize-ADD non-fatal-dial posture survived and StallGuard/cancellation/byte accounting untouched). Validation: fmt/clippy clean, `cargo test --workspace` 1476 → 1479/0/2 across 37 suites. All on master, unpushed. Session total: w6-1 (+design-1), w6-2 (filed w6-2a/b/c), w4-4, design-3 — four rows closed, six commits of records. Next: w9-3 (test-harness builder) is the topmost ratified open row and the right size for a fresh session; filed alternatives w6-2a/b/c + relay-1.
.review/results/small-file-ceiling-plan.codex.md:2035:**2026-07-04 15:24:23Z** - **CODER (w2-2-stream-ladder-owner, claude)**: Landed w2-2 through the codex loop (owner go: "continue" → topmost open row per the 12th handoff). The row as filed (2026-06-11) predates REV4, which already delivered its three stream-count legs: the `determine_remote_tuning` ladder died at ue-r2-1e (live dial), daemon `desired_streams` at ue-r2-1f (`engine::initial_stream_proposal` — byte- AND file-count-keyed, satisfying the spec's "takes file_count"), and `pull_stream_count` with the Pull RPC at ue-r2-1h; D-2026-06-20-1 recorded the absorption in v1 slice IDs. The remaining leg — the transfer_plan 16/32 MiB chunk ladder — turned out to be **entirely dead policy**, established by a 5-agent audit workflow + hand verification: every remote path overrode it with `Some(dial.chunk_bytes())` (push client 5 refresh sites + ensure_dial, pull_sync both literals); the only paths where the ladder won (local engine, test callers) discarded the value (`PlanUpdate` carries payloads only); the single workspace read of `PlannedPayloads.chunk_bytes` sat behind a `chunk_bytes == 0` guard no live caller can trigger (all pass the dial value, floored ≥ 64 KiB). The spec's "make transfer_plan take chunk_bytes as input" predates the dial — with zero consumers, threading a value through the planner would be plumbing with no reader, so the honest single-owner outcome was deletion. Slice `01209bc`: ladder + `Plan` wrapper deleted (`build_plan` → `Vec<TransferTask>`); `PlannedPayloads` deleted (`plan_transfer_payloads` → `Result<Vec<TransferPayload>>`, ripple through diff_planner/streaming_plan/pipeline tests/re-exports); `PlanOptions.chunk_bytes_override` + all refresh sites deleted (push `plan_options` now immutable default; two arms keep bare `ensure_dial` calls — first-need creation and first-wins ceilings unchanged); unreachable fallback guard in `stream_fallback_from_queue` deleted; `plan_to_daemon_format` deleted (git log -S: never called in repo history — its "server pull mode" comment was never true); orphaned `TuningParams` deleted (producer died at ue-r2-1e); write-only kickoff histogram collapsed to the `total_bytes` accumulator that was its only read. Comment-truth sweep: dial.rs mutability-model doc no longer claims chunk/prefetch are "read at each use site" (consumers snapshot at session/pipeline/batch setup; steps reach epoch-N sockets and later fallback batches); buffer.rs example cites the dial, not `TuningParams`. Behavior byte-identical on every live path. Tests: +4 transfer_plan unit pins (module had zero) — tier classification/interleave, single-small-file no-tar, force_tar single-file, count-target shard splitting with the 128 clamp; deletions are compile-guarded (w2-1 evidence shape); zero tests deleted. Review: codex **NEEDS FIXES (1 Low)** — the first bare ensure_dial comment said "fallback batch" inside the `TransferMode::DataPlane` branch; accepted (mislabel sits exactly on the invariant under review), fixed `27f53a0` (one word). W3.1's "after W2.2 settles the tuning owner" prerequisite is now settled: the owner is `engine::TransferDial`. New discoveries → STATE Open questions: `725aa07` tracked a 236-file stale worktree snapshot (`.claude/worktrees/vigilant-mayer/`) into the repo; WHITEPAPER still describes the pre-dial tuning world (stale since ue-r2-1e, w10 territory). Validation both commits: fmt/clippy clean, `cargo test --workspace` 1452/0/2 across 37 suites (baseline 1448). All on master, unpushed. Next: w3-1 (memory-aware BufferPool) tops the open queue; design-3 remains the sanctioned smaller alternative.
.review/results/small-file-ceiling-plan.codex.md:2039:**2026-07-04 13:53:22Z** - **DECISIONS (owner Q&A, claude)**: The owner asked for the four standing questions "one at a time, no idea what these refer to" — each was presented in plain English with options and answered: (1) **commit erratum → leave as-is** (D-2026-07-04-2; mirrors the D-2026-06-07-1 no-rewrite calculus — two bisect-skippable commits beat force-pushing shared history); (2) **10 GbE session → "soon, but keep coding first"** (STATE Blocked reworded: not a daily blocker; owner will call "benchmark"); (3) **D-2026-06-20-1 stale warmup/size-gate wording → "follow the existing pattern"** — the ledger's own precedent (D-2026-06-20-2's veto annotation, D-2026-06-20-6's struck scope clause) IS edit-in-place-with-annotation, so the superseded framings are struck with pointers to -2 q1 and REV4/-5 (bounded-unilateral untouched — still true), and -5's "remains an open question" note resolved; (4) **supports_cancellation → flip it** (D-2026-07-04-3): CancelJob + TUI F2 will work on attached Push/PullSync transfers; policy-only after w4-3's race wiring; contract change (exit 2→0) recorded; implementation queued as **w4-5-supports-cancellation-flip**, now the topmost open REVIEW.md row. Batch `2a21d6f` through the codex loop per D-2026-07-04-1: **NEEDS FIXES (1 Medium + 1 Low, both STATE.md coherence)** — the Now bullet still called the erratum an open owner call, and the queue rewrite dropped the coder's-pick clause (design-3-vs-w2-2 ordering contradiction); both accepted, fixed `a928193`. The decision content itself passed all cross-checks (ledger consistency, w4-3 scope-note agreement, strike precision). check-docs.sh green. All on master, unpushed.
.review/results/small-file-ceiling-plan.codex.md:2053:**2026-07-03 19:33:59Z** - **CODER (ue-r2-1g, claude)**: Landed PullSync multistream through the engine (REV4 `ue-r2-1g`, seventh slice through the code→GPT-review→fix loop; absorbs `MULTISTREAM_PULL.md`). Key discovery that shaped the slice: the CLIENT side of multistream PullSync already existed — `pull_sync_with_spec` has routed negotiations through the `stream_count`-honoring fan-out (`receive_data_plane_streams_owned`) since `69d8599` (2025-11-15), months before any client sent a capacity profile (`a0d2c9f`, 1e) — proven from git history, so the profile-presence gate cannot strand any committed client. The slice (`48e583e`) is therefore daemon-side: `negotiated_pull_streams` proposes from `engine::initial_stream_proposal` (the daemon is the byte sender AND shape-knower on pull; the engine fn's doc now states the proposer is the shape-knowing end either direction), gated on the client's advertised `receiver_capacity` (absent/unknown `max_streams` → 1 stream, today's behavior byte-for-byte per REV4 Design §5), recorded on the dial; `accept_and_wrap_sinks` (accept N, bounded token auth, N `DataPlaneSink`s) HARVESTED verbatim from the deprecated Pull RPC into `pull_sync.rs` (the deprecated handlers borrow it back until 1h deletes them); the 1a elastic work-stealing pipeline does the fan-out across N sinks. NO proto changes. Resume keeps its dedicated single-stream path (ordered JIT block-hash protocol — explicit RELIABLE exception); gRPC fallback untouched; delegated daemon→daemon inherits free via the dst-stamped profile. Deliberate deltas called out: prefetch 8→`dial.prefetch_count()`, pool scales with streams. Pull 1s-start explicitly NOT met and cannot be yet — the shape-keyed proposal inherently follows the full scan; it rides on `ue-r2-2` resize (recorded in Known gaps, not silently skipped). Review: codex **NEEDS FIXES → 2 accepted + fixed** (`4a2e58d`): cancellation-mid-transfer test with live sockets (TCP-level teardown observability) + dial bookkeeping on the conservative arm. Additionally ran a 3-lens adversarial self-review panel (concurrency/compat/RELIABLE): 2 more accepted + fixed same commit — client now clamps daemon-advertised `stream_count` to its own advertised ceiling (`bounded_stream_count`, REV4 §4 "weak end protects itself" made real receive-side) and the harvested helper's token-mismatch status restored to UNAUTHENTICATED (pull_sync wire behavior pre-slice-exact; the delta moved to the deprecated path) — 1 deferred (sequential-accept pin growth ~N×, bounded + precedented → W1 socket-policy row). e2e proves >1 stream observably (300 files → 2 streams, marker + byte-identical, revert-proven). Validation: fmt/clippy clean, `cargo test --workspace` **1413 / 0 / 2** (baseline 1403; +8 slice, +2 review). All on master, unpushed (origin at `7603177`). Ladder #3 (`pull_stream_count`) now dies with its RPC at `1h`. Next: `ue-r2-1h` (delete deprecated Pull RPC; must relocate `PullEntry`/`collect_pull_entries_with_checksums` — noted in the finding doc).
.review/results/small-file-ceiling-plan.codex.md:2055:**2026-07-03 18:32:57Z** - **CODER (ue-r2-1f, claude)**: Landed push convergence (REV4 `ue-r2-1f`, sixth slice through the code→GPT-review→fix loop). The daemon-push `desired_streams` ladder — the one the old `tuning.rs` doc said "wins" — is retired into the engine (`a4a9f70`): `engine::initial_stream_proposal` carries the shape table verbatim (bytes OR file-count keyed), clamped to the proposer's advertised receiver ceiling; both daemon negotiation sites call it; the private ladder is deleted (it had zero tests — the engine fn now has full tier-boundary coverage, extended ±1 per review). Wire-identical negotiations today (table max 16 < ceiling 32); the client's dial still clamps sender-side (1e). Second of three ladders gone; `pull_stream_count` retires at 1g/1h. The finding doc states the interpretation of "route push through the engine" explicitly (push's gRPC manifest/need-list loop = protocol boundary per REV4 Design §1's own list; the slice's substance = decision-layer ownership) and put it to the reviewer: codex **PASS with one Low** (boundary-value test gap, fixed `0c8da50`) and explicitly judged the interpretation **plan-conformant**. Validation: fmt/clippy clean, `cargo test --workspace` 1403 / 0 / 2. All on master, unpushed (origin at `7603177`). Next: `ue-r2-1g` (PullSync multistream through the engine).
.review/results/small-file-ceiling-plan.codex.md:2063:**2026-07-03 15:43:45Z** - **CODER (ue-r2-1b, claude)**: Landed the wire dial contract (REV4 `ue-r2-1b`, second slice through the code→GPT-review→fix loop, D-2026-06-20-6). Proto (`2741dc8`): new `CapacityProfile` (cpu_cores, `DrainClass` enum, load_percent, max_streams, drain_rate_bytes_per_sec, max_chunk_bytes, max_inflight_bytes; 0 = unknown = stay conservative) carried as `DataTransferNegotiation.receiver_capacity = 11` (push: daemon is byte receiver) and `TransferOperationSpec.receiver_capacity = 12` (pull_sync/delegated: client/dst is byte receiver) — **spec_version deliberately stays 2** (exact-match gate at `operation_spec.rs:107` means a bump would make old daemons reject new clients; the profile is a skippable hint, unlike v2's safety-critical field). Daemon-authoritative `resize_enabled = 12` + `epoch0_sub_token = 13` on the negotiation; capability bits `PushHeader.supports_stream_resize = 8` / `PeerCapabilities.supports_stream_resize = 5` (all false until ue-r2-2); `DataPlaneResize`/`DataPlaneResizeAck` from adaptive-PR3 prior art (`d9d4ec7`) as oneof variants in all four control streams (ClientPushRequest=9, ServerPushResponse=5, ClientPullMessage=5, ServerPullMessage=16). PR3's field-number clash (its negotiation 11-14) resolved: min/max stream bounds subsumed by `CapacityProfile.max_streams`, floor 1. Zero behavior: all literals stamped with defaults; new variants ignored on receive like unknown payloads; one intentional semantic addition — the delegated dst override now also strips CLI-supplied `receiver_capacity` (R25-F2 boundary; prevents a fabricated ceiling leaking once ue-r2-1e reads the field). Compat tests (`crates/blit-core/tests/proto_wire_compat.rs`, first use of test-local `#[derive(prost::Message)]` old-shape replicas): old→new + new→old for negotiation/spec/PushHeader/caps including normalization through the real `from_spec` chokepoint; resize frames decode as `payload: None` on old peers for all four oneofs (with known-variant controls); new↔new round trips. Review: codex/GPT-5.5 **PASS, zero findings**; supplementary 4-lens adversarial self-review (ultracode session) found 1 Low, accepted — the `receiver_capacity` comment falsely implied deprecated Pull carries a client→daemon profile (PullRequest has no spec channel); comment fixed in `5bd345a`. Validation: fmt/clippy clean, `cargo test --workspace` 1391 passed / 0 failed / 2 ignored (baseline 1378, +13). All on master, unpushed. Next: `ue-r2-1c` (engine shell + local adapter).
.review/results/small-file-ceiling-plan.codex.md:2065:**2026-06-21 03:02:29Z** - **CODER (ue-r2-1a, claude)**: Landed the adaptive-streams substrate (REV4 `ue-r2-1a`, first slice; first end-to-end run of the code→GPT-review→fix loop, D-2026-06-20-6). Cherry-picked over the `-s ours` octopus trap (D-2026-06-07-2, where a plain merge no-ops): PR1 per-stream telemetry zero-cost `Probe` (`e569eea`), PR2 shared work-stealing flume queue (`3844a15`), PR2 forwarder-halt-on-error fix (`ec561f2`). Hand-resolved the `data_plane.rs` StallGuardWriter-vs-`Probe` conflict (compose: stream stays `StallGuardWriter<TcpStream>`, struct gains generic `<P: Probe = NoProbe>`, `from_stream_with_probe` wraps the guard) and `mod.rs` re-exports (dropped `Phase`/`TransferProgress`/`TransferProgressSnapshot` — master had removed them; added telemetry types + the `AtomicU8` import). Excluded `eafb187` (doc-shuffle; embeds `C:/Users` paths) and `d9d4ec7` (PR3 WIP, does not build). Work-stealing behaviour tests added (`771a632`): byte/file exactly-once accounting + producer-cancel graceful wind-down. codex/GPT-5.5 read-only review → fix-then-ship, 4 findings, all accepted, fixed in `90ed43d`: F1 (High→Medium) workers re-check `cancelled` before each recv (bounds survivor work after first error; interrupting in-flight / hard-abort-on-drop stays w4-1); F2 `send_block` now calls `probe.record_bytes`; F3 exactly-once path assertion; F4 multi-sink cancel-under-backpressure test. Carried to `ue-r2-1e`: PR1 `write_blocked_nanos` `join!` over-measure (Medium) + tar-shard timing (Low) — telemetry has no live consumer until the dial. Validation at each step: fmt/clippy clean, `cargo test --workspace` 1378 passed / 0 failed / 2 ignored (baseline 1370). All on master, unpushed.
.review/results/small-file-ceiling-plan.codex.md:2071:**2026-06-12 16:32:00Z** - **REVIEW (gemini-reviewer)**: Graded the two pending sentinels on `master` sequentially: **design-4-fallback-midmanifest-negotiation** (`ddfeb58`) accepted (daemon early-flush branch is now TCP-only; client-side fallback_negotiated flag added to hold need-list/manifest fallback payloads in pending_queue; 2000-file regression test passes) and **design-5-send-failure-masks-rejection** (`08d71a2`) accepted (prefer_server_error helper harvests daemon's terminal status on manifest-phase send failures; 500-file readonly_enforcement regression test passes repeatedly). Both verified.json verdicts created, sentinels deleted via git rm, and REVIEW.md status rows updated to `[x]`. Validation suite re-run: cargo fmt, clippy, and cargo test --workspace all passed successfully (1370 passed, 0 failed, 1 ignored).
.review/results/small-file-ceiling-plan.codex.md:2091:**2026-06-11 00:42:30Z** - **REVIEW**: Verification and acceptance of `audit-h3c-slice1-grpc-fallback-frame-contract` (commit `bf4cc82`, verdict declared by owner). Validation suite re-run at HEAD `1be16bc`: fmt/clippy/test all green (exit 0), blit-core test-function count identical at `bf4cc82` and HEAD (344) — no test drop. The review assessment surfaced four facts the finding doc didn't know, recorded here as input to the slice-2 re-scope: **(1)** the 1 MiB frame cap is load-bearing correctness, not just cadence — tonic 0.14's default 4 MiB decode limit (`DEFAULT_MAX_RECV_MESSAGE_SIZE`, no `max_decoding_message_size` override anywhere in the workspace) means pre-slice-1 gRPC-fallback transfers of files >4 MiB should fail at decode, since `tuning.rs` emits 16/32/64 MiB chunks and `pull_sync.rs:515` passed `tuning.chunk_bytes` straight into `GrpcServerStreamingSink`. Version-skew residual: a new client against an old daemon still receives uncapped frames. **(2)** The bigger H3 hole is missing client-side HTTP/2 keepalive: daemon pings clients (`main.rs:138`) but all three client channel builders (`pull.rs:239`, `push/client/mod.rs:307`, `blit-app/src/client.rs:38` — already-drifted near-duplicates) set only `connect_timeout`, so a dead daemon hangs `message().await` forever; ~3 lines of `Endpoint` keepalive config cover dead-peer/black-hole at transport level and shrink slice 2's watchdog scope to wedged-but-alive peers only. **(3)** Fallback throughput is governed by HTTP/2 flow-control windows, not frame size — hyper defaults: client 2 MiB stream / 5 MiB conn; daemon server 1 MiB conn total → hardware-independent push-fallback ceiling on high-BDP links; `http2_adaptive_window(true)` is the no-tuning fix. **(4)** `clamp_fallback_chunk_size(x.max(1MiB))` ≡ const 1 MiB — the sinks' `chunk_bytes` param is now inert and misleading; root cause is a TCP transport parameter (`chunk_bytes`) embedded in the transport-agnostic `PlannedPayloads` (planner payload composition itself is sound). Owner direction: slice 2 re-scope deferred until after a repo-wide design-coherence review (plan doc next). Sentinel cleared, `REVIEW.md` updated to verified (`[x]`).
.review/results/small-file-ceiling-plan.codex.md:2119:**2026-05-05 20:30:00Z** - **FIX**: Round 45 review of commit `f83a208`. **R45-F1 (High)**: while plumbing scanned features I left a `let total_bytes = scanned_bytes` alias in `crates/blit-core/src/orchestrator/orchestrator.rs:480` and the streaming summary at `:567` then read that local for `summary.total_bytes`. Net effect on the wire: `summary.total_bytes` (contracted as "bytes the pipeline wrote") was reporting scanned bytes, so any incremental skip-unchanged run with all-up-to-date files would have reported the full source size as bytes-written and overcounted throughput. Fix: removed the alias; streaming summary now reads `pipeline_outcome.bytes_written` from `SinkOutcome` directly. The verbose "Planning enumerated" line correctly references `scanned_bytes`. **R45-F2 (Low)**: planner + transfer sub-predictions were still passing `(all_headers.len(), total_bytes)` (which were aliases of scanned_* but only by accident) while the total prediction passed `(scanned_files, scanned_bytes)` explicitly. Made all three predictor calls share the explicit `(scanned_files, scanned_bytes)` feature vector so a future maintainer editing one branch can't reintroduce drift by missing another. New regression test `incremental_run_total_bytes_excludes_skipped_files` in `orchestrator::orchestrator::async_runtime_tests` runs the full local mirror pipeline twice (mirror=true forces streaming planner; skip_unchanged=true means run 2 writes 0 bytes) and asserts `summary.total_bytes == 0` on the second run. Verified the test catches the bug by reintroducing the alias — assertion fires with `total_bytes=4096, scanned_bytes=4096` matching the pre-R45 state. **Tests**: `cargo test --workspace` 407 pass / 0 fail (was 406, +1 new R45 regression test).
.review/results/small-file-ceiling-plan.codex.md:2147:**2026-05-03 01:05:00Z** - **FIX**: Closed Round 37 findings from `docs/reviews/followup_review_2026-05-02.md`. **R37-F1**: `RemotePullClient::pull_sync_with_spec` now preserves a typed `PullSyncError::Negotiation` for source-side pull-sync refusals before negotiation completes, and the destination `DelegatedPull` handler maps that to `DelegatedPullError::NEGOTIATE` instead of generic `TRANSFER`. Added core wire test coverage for initial RPC rejection classification and remote→remote CLI integration coverage where a fake source rejects `pull_sync`; the CLI surfaces "source refused delegated pull" and relay counters stay zero. **R37-F2**: delegated `BytesProgress` is now treated as cumulative state and converted to deltas before feeding the CLI progress accumulator; duplicate cumulative updates are not double-counted. Proto comments now document the cumulative semantics. **Tests**: `cargo fmt`; `cargo fmt -- --check`; `cargo test -p blit-core pull_sync_with_spec`; `cargo test -p blit-cli remote_remote_direct`; `cargo test -p blit-cli --test remote_remote`; `cargo test -p blit-daemon delegated_pull`; `cargo test --workspace` (all passed; existing macOS FSEvents deprecation/F14 warnings and the pre-existing macOS test unused-variable warning remain).
.review/results/small-file-ceiling-plan.codex.md:2149:**2026-05-03 00:30:00Z** - **ACTION**: Implemented Phase 2 of `docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md`. Remote→remote `copy`/`mirror`/`move` now dispatch to destination-side `DelegatedPull` by default, with `--relay-via-cli` as the only legacy relay escape hatch. Added `crates/blit-cli/src/transfers/remote_remote_direct.rs` to build the same `TransferOperationSpec` as pull-sync, call `DelegatedPull`, surface stale-daemon/gate/connect/transfer errors without silent fallback, and print delegated summaries. Added env-gated test instrumentation in `blit-core::remote::instrumentation` so integration tests can assert the CLI sent zero data-plane payload bytes and never constructed `RemoteTransferSource` on the direct path, while `--relay-via-cli` does both. Reworked `crates/blit-cli/tests/remote_remote.rs` to cover direct byte-path isolation, gate-rejection no-fallback, stale destination `Unimplemented` no-fallback, and explicit relay. Updated CLI docs/help for `--relay-via-cli`. **Tests**: `cargo fmt`; `cargo check -p blit-cli`; `cargo test -p blit-cli remote_remote_direct`; `cargo test -p blit-cli --test remote_remote`; `cargo test -p blit-core pull_sync_with_spec`; `cargo test -p blit-daemon delegated_pull`; `cargo test --workspace` (all passed; existing macOS FSEvents deprecation/F14 warnings remain).
.review/results/small-file-ceiling-plan.codex.md:2169:**2026-05-02 15:45:00Z** - **ACTION**: Closed Round 8 review findings R8-F1 and R8-F2 from `docs/reviews/followup_review_2026-05-02.md`. Both bugs were in the daemon's gRPC push fallback receive loop (`receive_fallback_data` in `crates/blit-daemon/src/service/push/data_plane.rs`) — the framing layer that runs *before* the shared `tar_safety` helper sees the buffer. **R8-F1** (Medium — DoS): the chunk accumulator was capped only by an 8 MiB initial reservation; the actual `Vec` grew unbounded. `archive_size == 0` skipped the overflow check entirely, and `archive_size > MAX_TAR_SHARD_BYTES` was bypassed at the framing layer. Reject zero archive_size when files are present, reject `archive_size > MAX_TAR_SHARD_BYTES` at TarShardHeader, and enforce both the declared size and the local cap on every chunk via a new `check_fallback_chunk_overflow` helper. **R8-F2** (Medium — silent partial success): stream EOF was treated as graceful end. But FileManifest and TarShardHeader remove entries from `pending` before bytes arrive, so a client could send a header, close the stream, and the final `pending.is_empty()` check would pass despite no data ever landing. Track `upload_complete_seen` and bail on EOF without UploadComplete or with `active.is_some()`. Extracted `validate_fallback_shard_archive_size` and `check_fallback_chunk_overflow` as private helpers so the rules are unit-testable without spinning up a full gRPC server. 7 new framing tests pin every boundary condition (zero size, at-cap, above cap, declared overflow, local-cap overflow, u64 overflow, within-bounds). **Tests**: `cargo fmt`; `cargo test --workspace` (166 lib + 17 daemon + integration suites, all green).
.review/results/small-file-ceiling-plan.codex.md:2193:**2026-05-02 01:15:00Z** - **F1 (path safety)**: Centralized receive-side path validation in `blit-core/src/path_safety.rs`. Two helpers: `validate_wire_path(s) -> PathBuf` (pure validation, normalized form) and `safe_join(root, s) -> PathBuf` (validate + join, with empty input returning root unchanged for the single-file destination case). Rejects: `..` components (not just substrings — the previous tar-shard `rel.contains("..")` check was too weak and also rejected legitimate filenames containing literal `..`), absolute paths, Windows drive prefixes (`C:\…`), UNC and DOS device paths (`\\?\…`, `\\server\share\…`), root components, embedded NUL bytes. Migrated `pull.rs::sanitize_relative_path` and daemon `service/util.rs::resolve_relative_path` / `resolve_manifest_relative_path` to thin wrappers that delegate to the shared module — single chokepoint, no drift between sanitizers. Applied at all five receive-side sites the review identified: `sink.rs:218` (streamed file write), `sink.rs:404` + `:426` (tar shard entry validation + per-entry join), `sink.rs:462` (resume block write), `sink.rs:498` (block completion), and the gRPC fallback block-hash path in `pull.rs:532`. Wire-boundary defense in depth: `pipeline.rs::read_file_header` and per-entry path read in `read_tar_shard` validate at the moment paths arrive off the socket, so unsafe headers never enter the FileHeader stream. **Tests**: 209 passed (was 185 baseline) — 16 new `path_safety::tests` covering the validator surface (`..`, absolute Unix, Windows drive, UNC, NUL, valid `..`-containing filenames, deep paths, Unicode, trailing slash, just-dot), 8 new sink-level integration tests confirming `FsTransferSink::write_file_stream` rejects malicious wire paths before any filesystem write and that the legitimate single-file empty-path case still works. **Verification**: `cargo check --workspace` clean; `cargo test --workspace` 0 failures.
.review/results/small-file-ceiling-plan.codex.md:2197:**2026-05-01 23:43:57Z** - **REVIEW**: Completed a repo-level codebase review and saved it to `docs/reviews/codebase_review_2026-05-01.md`. Highest-priority findings: receive-side path sanitization is incomplete across streamed files, tar shards, and resume block records; daemon `use_chroot` is documented/configured but not enforced; filtered mirror delete semantics need an explicit product decision; metrics lifecycle and endpoint behavior need hardening. Added a short `TODO.md` follow-up section and marked Phase 4 as in progress with the review artifact linked. **Verification**: `cargo check --workspace` passed; `cargo test --workspace` passed with 185 tests after rerunning outside the sandbox to allow local test port binding.
.review/results/small-file-ceiling-plan.codex.md:2199:**2026-04-24 03:15:00Z** - **FIX**: Fixed single-file remote push crash — `blit copy FILE server:/mod/` previously died with "opening FILE/ during payload planning: Not a directory". Root cause was two-sided: on the client, `filter_readable_headers` and `build_tar_shard` computed `source_root.join(&rel)` where `rel=""` for single-file sources, which on Unix preserves a trailing `/` and makes `File::open` reject with ENOTDIR. On the daemon, `resolve_relative_path` folded empty paths to `"."` (which broke `module.path.join("")` = wanted single file, but got `module.path/.`), and the subsequent `module.path.join(empty_rel)` added another trailing `/`. Fix applies the `rel.is_empty() → use root directly` pattern in three places on the client side (matching `FsTransferSource::open_file`'s existing handling), and adds two new helpers on the daemon side: `resolve_manifest_relative_path` preserves empty as empty for per-file manifest entries, and `resolve_dest_path(base, rel)` handles the empty-rel join case. Swapped all four push-side `resolve_relative_path` callers and three `module.path.join` sites. Regression tests in `crates/blit-cli/tests/remote_push_single_file.rs` pin both the container-dir (`blit copy FILE server:/mod/`) and rename (`blit copy FILE server:/mod/new.txt`) cases. Annotated all four `docs/bugs/*.md` files with their resolution status. **Tests**: 173 passed (+2 remote push single-file), clippy clean.
.review/results/small-file-ceiling-plan.codex.md:2201:**2026-04-24 02:54:00Z** - **ACTION**: Completed the remaining UX feedback items #4, #5, #6, #7, #8, #9, #10 from `docs/ux-feedback-migrate-games-poc.md`. #4 TTY auto-progress: `TransferArgs::effective_progress()` enables the progress indicator when stdout is a TTY and `--json` is off, matching rsync/rclone; piped stdout stays silent for scripts. #8 `--help` rsync docs: `after_long_help` on `TransferArgs` appends a PATHS section summarizing the trailing-slash rules to `blit copy/mirror/move --help`. #6/#7/#5 banner cleanup: demoted the `starting copy` line to stderr and dropped the version prefix; added `collapse_slashes()` display-only normalization so script-appended `//` no longer shows in the banner (semantics unchanged); under `-v`, print a follow-up line when rsync resolution materially rewrote the destination. #9 flag grouping: split `TransferArgs` flags across clap `help_heading` buckets (Options / Comparison / Reliability / Performance/debug) so first-time users see common flags before niche debug knobs. #10 `blit diagnostics dump SRC DST [--json]`: new subcommand emits a pasteable snapshot (version, invocation argv, parsed endpoints, rsync resolution, local filesystem caps, free/total disk, same-device check) without performing a transfer — built for one-shot bug reports. Added `sysinfo = "0.31"` (disk-only) to blit-cli, exposed `source_is_contents`/`dest_is_container` as `pub(crate)`, wrote 3 smoke tests pinning the JSON shape. Updated `blit(1)` manpage SYNOPSIS and DIAGNOSTICS sections. **Tests**: 170 passed (added 3), clippy clean.
.review/results/small-file-ceiling-plan.codex.md:2203:**2026-04-24 02:32:00Z** - **ACTION**: Addressed UX feedback items #1 and #3 from `docs/ux-feedback-migrate-games-poc.md`. Added `TransferOutcome` enum (`Transferred` / `JournalSkip` / `UpToDate` / `SourceEmpty`) to `LocalMirrorSummary` so the CLI can emit distinct, honest messages for the three legitimate zero-files cases instead of the ambiguous `Copy complete: 0 files, 0 B`. `FastPathDecision::NoWork` now carries an `examined` count to split empty-source from already-up-to-date; the single-file path also sets `UpToDate` when `skip_unchanged` short-circuits. CLI `print_summary` in `crates/blit-cli/src/transfers/local.rs` matches on outcome and suppresses the `• Throughput / Workers used` line unless verbose, ≥1 MiB, or `>1` file copied. JSON output now includes `outcome` and `files_examined`. Strengthened `single_file_copy_idempotent` to pin `Copy complete: 1 files` on first run and `Up to date` on second run — would have caught the regression where the single-file rerun emitted misleading `Copy complete: 0 files`. **Tests**: 167 passed, clippy clean.
.review/results/small-file-ceiling-plan.codex.md:2205:**2026-04-14 20:00:00Z** - **ACTION**: Completed unified-pipeline remediation across all transfer paths. Previously only local→local and gRPC-fallback push routed through `execute_sink_pipeline`; multi-stream TCP push and both daemon pull paths had bespoke dispatch code. Added `execute_sink_pipeline_streaming(source, sinks, payload_rx, prefetch, progress)` in `crates/blit-core/src/remote/transfer/pipeline.rs` and made the one-shot form a thin wrapper. Rewrote `MultiStreamSender` in `crates/blit-core/src/remote/push/client/mod.rs` as a lightweight `payload_tx` + `JoinHandle` shell — deleted `data_plane_sink_worker`, batch-splitting helpers, and `StreamStats` (~130 lines). Converted both daemon pull paths in `crates/blit-daemon/src/service/pull.rs` to use the pipeline; deleted `handle_pull_stream` and `handle_pull_stream_streaming` (~125 lines). Deleted dead modules: `transfer_engine.rs` (WorkerFactory trait + task-channel infra), `transfer_facade/` (TaskAggregator, PlannerEvent, build_local_plan, stream_local_plan), and `RetryableTask` from `transfer_plan.rs` — 1139 lines total. Updated `docs/ARCHITECTURE.md` with the unified-pipeline diagram and source/sink matrix. Added `blit(1)` command-summary hints and expanded `TransferArgs` doc-comments for `--help`. **Tests**: 152 passed (87 core + 65 integration), clippy clean. **Remaining**: end-to-end 10 GbE benchmark runs.
.review/results/small-file-ceiling-plan.codex.md:2211:**2026-04-07 16:00:00Z** - **ACTION**: Completed blit-utils hardening (P1). Added 21 integration tests in `crates/blit-cli/tests/blit_utils.rs` covering all 9 blit-utils commands: scan (mDNS exit), list-modules (text+JSON), ls (remote/local/JSON), find (pattern/JSON/dirs-only/limit), du (text+JSON), df (text with human-readable check, JSON), rm (file/directory/module-root refusal), completions (prefix/dirs-only), profile (text+JSON). Added human-readable byte formatting to `df` output using `format_bytes()` alongside raw byte counts. **Tests**: `cargo test -p blit-cli --test blit_utils` (21 passed); `cargo test -p blit-cli --test admin_verbs` (10 passed).
.review/results/small-file-ceiling-plan.codex.md:2225:**2025-01-29 12:00:00Z** - **ACTION**: Added resume integration tests (`remote_resume.rs`) covering partial file resume, identical file optimization, and gRPC fallback path. Security hardening: MAX_BLOCK_SIZE (64 MiB) enforced on both client and server to prevent OOM attacks from malicious block_size values. Export added to `copy/mod.rs`. Updated TODO.md to mark P0 parity tests and refactoring items as complete. **Tests**: `cargo test --workspace` (64 passed).
.review/results/small-file-ceiling-plan.codex.md:2227:**2025-01-29 10:00:00Z** - **FIX**: Addressed Gemini code review findings for remote block-level resume. (1) Fixed critical memory vulnerability: `compute_block_hashes` in `pull.rs` and both `stream_via_data_plane_resume`/`stream_via_block_resume_grpc` in `pull_sync.rs` now stream files in block-sized chunks instead of loading entire files into memory. (2) Fixed stop-and-wait protocol in data plane path: `stream_via_data_plane_resume` now pipelines all block hash requests upfront (Phase 1), collects all responses (Phase 2), then processes files (Phase 3) - eliminates per-file RTT penalty. gRPC fallback keeps simpler stop-and-wait since it's for diagnostic use only. **Tests**: `cargo build --package blit-daemon --package blit-cli` (passed).
.review/results/small-file-ceiling-plan.codex.md:2229:**2025-01-28 16:00:00Z** - **ACTION**: Implemented remote block-level resume for TCP data plane (primary path). Added `DATA_PLANE_RECORD_BLOCK` and `DATA_PLANE_RECORD_BLOCK_COMPLETE` record types to data plane protocol. Server-side: `stream_via_data_plane_resume` uses gRPC for block hash exchange then TCP data plane for block transfer - combines best of both worlds. Client-side: `handle_block_record` and `handle_block_complete_record` in data plane receiver handle seek+write for in-place block updates. Remote resume now works with default `--resume` flag (no `--force-grpc` needed). **Tests**: `cargo test -p blit-core` (63 passed).
docs/audit/findings/inconsistency-endpoints.md:126:**Recommendation**: Treat empty-after-trim as "unset" symmetrically — printing a hint when `--remote ""` is explicitly passed so the user knows the flag was ignored. The current asymmetry is small but it's a footgun for shell-templated launches (`blit-tui --remote "$REMOTE"` with unset `$REMOTE`).
docs/audit/findings/drift-tui.md:37:**Plan says**: TUI_REWORK §10 "Testing Contract — Required workflow tests: Remote file -> local directory copy. Local directory -> local directory mirror. Local directory -> two remote destinations fan-out. Remote A -> remote B delegated copy. Move/delete review cannot be bypassed by a single accidental key. Editable path bar and navigated rows produce the same `Location`. Old F-key/letter aliases, while present, route to the same actions as visible controls."
docs/audit/findings/drift-tui.md:43:**Notes**: Without these tests, the rework principles are unverifiable by CI. The Move/delete-review-cannot-be-bypassed assertion is especially important — d-65/R47-F4 / `feedback-port-cli-safety-guards` already shows that bypassing those gates causes data loss. Remediation: add a `tests/` directory or `mod tests` with W1-W4 scenarios using fake providers (which also requires the `BrowseProvider` trait per drift `transferdraft-types-missing`).
docs/audit/inventory/code-cli.md:10:- **retry-wait-defaults** — `crates/blit-cli/src/cli.rs:264-272` — `--retry` defaults to 0 (off), `--wait` to 5 seconds; both parse u32/u64; wait passed to `run_with_retries` as `Duration::from_secs`. _(notes: no upper bound on retry or wait; no jitter/backoff visible at CLI layer)_
docs/audit/inventory/code-cli.md:42:- **mirror-prompt** — `crates/blit-cli/src/transfers/mod.rs:181-190` — mirror prompts unless `--yes` OR `--dry-run`; bypassed string is "Mirror will delete extraneous files at destination 'X'. Continue?". _(notes: dry-run does NOT need confirmation; prompt happens BEFORE rejected-flag gates if mirror is set)_
docs/audit/inventory/code-cli.md:165:- **--delete-scope is stringly typed** — `crates/blit-cli/src/cli.rs:247-248, 386-388` — `value_parser` is case-sensitive but `delete_scope_all()` is case-insensitive: `blit copy --delete-scope ALL` would be rejected by clap, but `blit copy --delete-scope all` accepted; meanwhile internal code accepts `All` too if it ever bypassed clap. Should be an enum.
.review/findings/ue-r2-2.md:56:  `RESIZE_COOLDOWN_TICKS`(4) passed since the last settle; never while
docs/audit/inventory/code-core-transfer.md:113:- **pull-checksum-mismatch-rejects** — `crates/blit-core/src/remote/pull.rs:764-779` — F11: if user passed `--checksum` and daemon `server_checksums_enabled=false`, return `PullSyncError::Negotiation` BEFORE any data flows (avoids silent degrade to size+mtime).
docs/audit/inventory/code-core-orch.md:81:- **fast-path-label-strings** — `crates/blit-core/src/orchestrator/orchestrator.rs:280,337,374,410,849` — string labels passed to `record_performance_history`: `"journal_no_work"`, `"no_work"`, `"tiny_manifest"`, `"single_huge_file"`, `"null_sink"`. _(notes: these strings are load-bearing — `select_tuning_window` references `"tiny_manifest"` literally as an exclude gate at orch:63.)_
docs/audit/inventory/code-core-orch.md:129:- **files-from-bypasses-other-rules** — `crates/blit-core/src/fs_enum.rs:201-205` — when `files_from.is_some()`, all other filter rules are bypassed; only the explicit path-set lookup decides. Returns false if `rel_path` is None.
docs/audit/inventory/plan-tui.md:1022:"Required workflow tests: Remote file -> local directory copy. Local directory -> local directory mirror. Local directory -> two remote destinations fan-out. Remote A -> remote B delegated copy. Move/delete review cannot be bypassed by a single accidental key. Editable path bar and navigated rows produce the same `Location`. Old F-key/letter aliases, while present, route to the same actions as visible controls."
.review/findings/ue-r2-1h.md:175:  `initial_stream_proposal` (1f/1g) has its own tests.
.review/results/sf-1-tripwire-harness.codex.md:321:  1478 → 1479/0/2 same-method A/B.
.review/results/sf-1-tripwire-harness.codex.md:510:  `engine::initial_stream_proposal` tiers (200→1, 1k→2, 5k→4, 10k→8,
.review/results/sf-1-tripwire-harness.codex.md:531:full workspace suite green; count vs 1479 baseline in verdict file).
.review/results/sf-1-tripwire-harness.codex.md:606:stream** — `engine::initial_stream_proposal` is byte-weighted, so
.review/results/sf-1-tripwire-harness.codex.md:667:   `initial_stream_proposal` (and the pull-side equivalent) weight
.review/results/sf-1-tripwire-harness.codex.md:814:| w2-2-stream-ladder-owner | Medium | Single stream-count/chunk owner: the 3 stream ladders died with REV4 (ue-r2-1e dial / -1f initial_stream_proposal takes file_count / -1h Pull RPC; absorption recorded D-2026-06-20-1); this slice closed the remaining leg — deleted the dead transfer_plan chunk lane (16/32 MiB ladder, Plan/PlannedPayloads wrappers, chunk_bytes_override + refresh sites, never-called plan_to_daemon_format, orphaned TuningParams); dial is the single chunk owner; W3.1's "settled tuning owner" = engine::TransferDial. Codex NEEDS FIXES (1 Low: new ensure_dial comment said "fallback batch" in the data-plane branch) → fixed `27f53a0` | `[x]` | master | `01209bc`+`27f53a0` |
.review/results/sf-1-tripwire-harness.codex.md:823:| w9-3-test-harness-builder | Medium | One daemon-spawn harness: TestContext::builder() (read_only/delegation/extra_daemon_args) + spawn_daemon/spawn_second_daemon absorb the SEVEN clones at HEAD (audit counted 5; w9-4/w9-5 had each added another — the finding's prediction twice proven) plus 5 cli_bin/7 run_with_timeout/4 ChildGuard copies; daemon build OnceLock'd per test binary (R16-F1 independence kept; was ~75 nested cargo invocations serializing on the build-dir flock — the daemon-spawn load-flakiness home); new blit_core::remote::grpc_server owns the audit-1 HTTP/2 keepalive (30s/20s) as production_server_builder() — daemon main.rs + all FIVE fake tonic servers (not 3: remote_remote ×2, jobs_lifecycle, pull_sync_with_spec_wire ×2) route through it, zero bare Server::builder() left; port-collision race surfaced by the build de-serialization fixed two-layer (process-global claimed-port set + child-death readiness check). Net −1,251 test-tree lines; 1478→1479 same-method A/B (+1 keepalive pin, mutation-verified). Codex NEEDS FIXES (1 Medium: fake-server :0 bind bypassed the claimed set — wrong-listener race for mixed fake/daemon binaries) → claim_port() shared, fixed | `[x]` | master | `f6e592e`+`8641bc6` |
.review/results/sf-1-tripwire-harness.codex.md:849:| design-3-unbounded-data-plane-connects | Medium | Both TCP data-plane connects lacked timeouts (audit-2 fix never reached the data plane); hung 60-127s on black-holed ports. Fixed: shared `socket::dial_data_plane` (bounded connect via DATA_PLANE_ACCEPT_TIMEOUT + w1-2 policy + bounded handshake write via DATA_PLANE_TOKEN_TIMEOUT; TimedOut in the chain → is_retryable transient); both sites collapsed (pull connect_pull_stream incl. resize-ADD, push connect_with_probe incl. elastic). +3 tests incl. deterministic stalled-handshake shape pin, mutation-verified; 1476→1479/0/2. Codex PASS (0 findings) | `[x]` | master | `49dcec6` |
.review/results/sf-1-tripwire-harness.codex.md:907:| audit-4-windows-handle-leak | Bug | RAII OwnedHandle guard closes the CreateFileW handle on every exit path in capture_snapshot (was leaked on the GetFileInformationByHandle `?`). Windows target cargo check passed with `CARGO_FEATURE_PURE=1`; target clippy blocked by pre-existing Windows warnings; Darwin gates pass | `[x]` | `phase5/a1` | `4e77897` |
.review/results/sf-1-tripwire-harness.codex.md:1302:  +  `engine::initial_stream_proposal` tiers (200→1, 1k→2, 5k→4, 10k→8,
.review/results/sf-1-tripwire-harness.codex.md:1323:  +full workspace suite green; count vs 1479 baseline in verdict file).
.review/results/sf-1-tripwire-harness.codex.md:1426:    48	#                      (chosen to cross engine::initial_stream_proposal
.review/results/sf-1-tripwire-harness.codex.md:1690:/usr/bin/zsh -lc 'rtk rg -n "stream complete|stream_count|initial_stream_proposal|data_plane.rs" crates scripts docs/bench/10gbe-2026-07-05/DIAGNOSIS.md' in /home/michael/dev/Blit
.review/results/sf-1-tripwire-harness.codex.md:1695:scripts/bench_tripwires.sh:48:#                      (chosen to cross engine::initial_stream_proposal
.review/results/sf-1-tripwire-harness.codex.md:1699:crates/blit-daemon/src/service/pull_sync.rs:16:use blit_core::engine::{initial_stream_proposal, TransferDial};
.review/results/sf-1-tripwire-harness.codex.md:1703:crates/blit-daemon/src/service/pull_sync.rs:437:    let proposal = initial_stream_proposal(total_bytes, file_count, dial.ceiling_max_streams());
.review/results/sf-1-tripwire-harness.codex.md:1724:crates/blit-daemon/src/service/push/control.rs:800:    blit_core::engine::initial_stream_proposal(
.review/results/sf-1-tripwire-harness.codex.md:1763:crates/blit-core/src/engine/mod.rs:30:    initial_stream_proposal, local_receiver_capacity, spawn_dial_tuner,
.review/results/sf-1-tripwire-harness.codex.md:1764:crates/blit-core/src/engine/dial.rs:429:pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
.review/results/sf-1-tripwire-harness.codex.md:1765:crates/blit-core/src/engine/dial.rs:642:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
.review/results/sf-1-tripwire-harness.codex.md:1766:crates/blit-core/src/engine/dial.rs:646:        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
.review/results/sf-1-tripwire-harness.codex.md:1767:crates/blit-core/src/engine/dial.rs:650:        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
.review/results/sf-1-tripwire-harness.codex.md:1768:crates/blit-core/src/engine/dial.rs:651:        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
.review/results/sf-1-tripwire-harness.codex.md:1769:crates/blit-core/src/engine/dial.rs:652:        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
.review/results/sf-1-tripwire-harness.codex.md:1770:crates/blit-core/src/engine/dial.rs:653:        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
.review/results/sf-1-tripwire-harness.codex.md:1771:crates/blit-core/src/engine/dial.rs:654:        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
.review/results/sf-1-tripwire-harness.codex.md:1772:crates/blit-core/src/engine/dial.rs:655:        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
.review/results/sf-1-tripwire-harness.codex.md:1773:crates/blit-core/src/engine/dial.rs:656:        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
.review/results/sf-1-tripwire-harness.codex.md:1774:crates/blit-core/src/engine/dial.rs:657:        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
.review/results/sf-1-tripwire-harness.codex.md:1775:crates/blit-core/src/engine/dial.rs:658:        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
.review/results/sf-1-tripwire-harness.codex.md:1776:crates/blit-core/src/engine/dial.rs:659:        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
.review/results/sf-1-tripwire-harness.codex.md:1777:crates/blit-core/src/engine/dial.rs:660:        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
.review/results/sf-1-tripwire-harness.codex.md:1778:crates/blit-core/src/engine/dial.rs:661:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
.review/results/sf-1-tripwire-harness.codex.md:1779:crates/blit-core/src/engine/dial.rs:663:        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
.review/results/sf-1-tripwire-harness.codex.md:1780:crates/blit-core/src/engine/dial.rs:664:        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
.review/results/sf-1-tripwire-harness.codex.md:1781:crates/blit-core/src/engine/dial.rs:665:        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
.review/results/sf-1-tripwire-harness.codex.md:1782:crates/blit-core/src/engine/dial.rs:666:        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
.review/results/sf-1-tripwire-harness.codex.md:1783:crates/blit-core/src/engine/dial.rs:667:        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
.review/results/sf-1-tripwire-harness.codex.md:1784:crates/blit-core/src/engine/dial.rs:668:        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
.review/results/sf-1-tripwire-harness.codex.md:1785:crates/blit-core/src/engine/dial.rs:670:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
.review/results/sf-1-tripwire-harness.codex.md:1786:crates/blit-core/src/engine/dial.rs:671:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
.review/results/sf-1-tripwire-harness.codex.md:2196:    blit_core::engine::initial_stream_proposal(
.review/results/sf-1-tripwire-harness.codex.md:2246:pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
.review/results/sf-1-tripwire-harness.codex.md:2284:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
.review/results/sf-1-tripwire-harness.codex.md:2288:        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
.review/results/sf-1-tripwire-harness.codex.md:2292:        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
.review/results/sf-1-tripwire-harness.codex.md:2293:        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
.review/results/sf-1-tripwire-harness.codex.md:2294:        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
.review/results/sf-1-tripwire-harness.codex.md:2295:        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
.review/results/sf-1-tripwire-harness.codex.md:2296:        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
.review/results/sf-1-tripwire-harness.codex.md:2297:        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
.review/results/sf-1-tripwire-harness.codex.md:2298:        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
.review/results/sf-1-tripwire-harness.codex.md:2299:        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
.review/results/sf-1-tripwire-harness.codex.md:2300:        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
.review/results/sf-1-tripwire-harness.codex.md:2301:        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
.review/results/sf-1-tripwire-harness.codex.md:2302:        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
.review/results/sf-1-tripwire-harness.codex.md:2303:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
.review/results/sf-1-tripwire-harness.codex.md:2305:        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
.review/results/sf-1-tripwire-harness.codex.md:2306:        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
.review/results/sf-1-tripwire-harness.codex.md:2307:        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
.review/results/sf-1-tripwire-harness.codex.md:2308:        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
.review/results/sf-1-tripwire-harness.codex.md:2309:        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
.review/results/sf-1-tripwire-harness.codex.md:2310:        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
.review/results/sf-1-tripwire-harness.codex.md:2312:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
.review/results/sf-1-tripwire-harness.codex.md:2330:**2026-07-04 23:35:54Z** - **CODER (w9-3-test-harness-builder, claude)**: Landed w9-3 through the codex loop (owner go: "continue, use /playbook reviewloop codex" — no playbooks exist in this repo, resolved to the `slice` operator per `.agents/repo-guidance.md` → topmost ratified open row per the 19th handoff). A 6-agent inventory workflow re-derived the audit's 2026-06-11 evidence at HEAD before coding and found the rot had GROWN: **seven** daemon-harness clones, not five — w9-4 (`readonly_enforcement.rs`) and w9-5 (`jobs_lifecycle.rs`) each added another private spawn_daemon/config-struct copy *because* common couldn't express delegation or a second daemon, proving the finding's "the next one will miss at least one" prediction twice — plus 5 `cli_bin` copies, 7 `run_with_timeout`, 4 `ChildGuard`, and **five** bare `Server::builder()` fake servers (not three: `remote_remote.rs` ×2, `jobs_lifecycle.rs`, `pull_sync_with_spec_wire.rs` ×2) vs production's audit-1 keepalive. Slice `f6e592e`: common/mod.rs is now the single owner — `TestContext::builder()` (`.read_only()`/`.delegation()`/`.extra_daemon_args()`; `new()`/`new_read_only()` signature-stable, zero edits in the 13 pre-existing consumers), `spawn_daemon(workspace, name, module_dir, opts)` + `TestContext::spawn_second_daemon` primitives (config superset: `delegation_allowed` serialized explicit `true` = the daemon's own absent-default, verified in runtime.rs before choosing; `[delegation]` table optional), `ensure_daemon_built()` OnceLock'ing the nested `cargo build` (R16-F1 per-process independence kept; ~75 invocations per full run → ≤1 per binary; also fixes remote_remote's dropped `--target` handling and the tcp_fallback/jobs/readonly spawns that ran NO build), shared `spawn_fake_blit_server` scaffold, and new `blit_core::remote::grpc_server::production_server_builder()` (owns the 2026-05-23 keepalive 30s/20s; daemon main.rs + all five fakes route through it; zero bare `Server::builder()` left, grep-verified; +1 mutation-verified pin test). Mid-slice the validation run itself caught the **daemon-spawn load-flakiness live**: `test_admin_find` got an empty listing from another test's daemon — `pick_unused_port`'s probe-drop-to-bind TOCTOU, previously masked by the per-test cargo builds serializing bring-ups; fixed two-layer (process-global claimed-port set — cargo runs test binaries sequentially, so per-process scope is exactly right — plus a `try_wait` child-death check in the readiness poll so an externally stolen port panics with the real reason instead of silently testing a foreign daemon). stderr policy unified to null (was piped-but-never-read; real capture stays w9-6). Review: codex **NEEDS FIXES (1 Medium, accepted — a genuinely sharp catch)**: `spawn_fake_blit_server` still bound `:0` OUTSIDE the claimed set, so a fake could take a port promised to a not-yet-bound daemon in mixed binaries (remote_remote, jobs_lifecycle) — same wrong-listener class, missed path; fixed `8641bc6` (`claim_port()` shared by both paths; the fake keeps its probe listener so its path has no gap at all). Records `c62d15b`. Net −1,251 test-tree lines. Validation: fmt/clippy clean; test-count gate proven by same-method A/B via `git stash` — HEAD 1478/0/2, slice 1479/0/2 across 37 suites, exactly +1, per-file `#[test]` counts identical (STATE's recorded "1479" baseline was a different aggregation, off-by-one vs the same tree); full suite ×2 + `admin_verbs` ×10 post-fix all green. All on master, unpushed. Next: strict design-queue order gives **w7-1** (mirror-executor consolidation) as topmost ratified open row; filed alternatives w6-2a/b/c + relay-1, coder's pick.
.review/results/sf-1-tripwire-harness.codex.md:2334:**2026-07-04 21:46:39Z** - **CODER (design-3-unbounded-data-plane-connects, claude)**: Landed design-3 through the codex loop (same session, fourth slice; coder's pick of the long-sanctioned smaller alternative over the large w9-3 harness consolidation — queue policy leaves sequencing to the coder). Both TCP data-plane client connects ran unbounded — the audit-2 wave bounded every control-plane connect at 30 s but never reached the data plane, so a firewalled/black-holed data port (the daemon advertises a fresh ephemeral port per transfer; asymmetric firewalls passing 9031 but blocking ephemerals are common) hung for the kernel SYN timeout (60–127 s) with no message. Sites re-verified at HEAD: the pull site is now `connect_pull_stream` (split at ue-r2-2, shared by resize-ADD dials), the push site `DataPlaneSession::connect_with_probe` (elastic dials included). Slice `49dcec6`: `remote::transfer::socket::dial_data_plane(addr, handshake, tcp_buffer_size)` — the client-side mirror of the daemon's bounded accept, in the w1-family policy module: connect bounded by the shared `DATA_PLANE_ACCEPT_TIMEOUT` (the row's sanctioned constant — no fifth 30 s literal), `configure_data_socket` applied, handshake write bounded by `DATA_PLANE_TOKEN_TIMEOUT` (mirrors the acceptor's bounded token read — the finding's "also the token write" clause); on either timeout the chain carries an `io::ErrorKind::TimedOut` source with text naming addr + the likely-firewall cause, so `remote::retry::is_retryable` classifies it transient and `--retry` re-dials. Both call sites collapsed onto the helper; socket.rs's w1-2-era "connect timeouts live at the call sites" module-doc paragraph rewritten (comment-truth). Tests +3 (blit-core 389 → 392): happy path (policy + handshake delivery), deterministic timeout SHAPE via an accepting-but-never-reading peer against a 64 MiB handshake (TimedOut chain + retryable — mutation-verified: swapping the timeout error for a plain eyre message fails the pin), TEST-NET black-hole connect bounded (environment-tolerant: fast-reject networks skip the shape assertions, the bound is asserted always). Review: codex **PASS, zero findings** (independently confirmed the pull resize-ADD non-fatal-dial posture survived and StallGuard/cancellation/byte accounting untouched). Validation: fmt/clippy clean, `cargo test --workspace` 1476 → 1479/0/2 across 37 suites. All on master, unpushed. Session total: w6-1 (+design-1), w6-2 (filed w6-2a/b/c), w4-4, design-3 — four rows closed, six commits of records. Next: w9-3 (test-harness builder) is the topmost ratified open row and the right size for a fresh session; filed alternatives w6-2a/b/c + relay-1.
.review/results/sf-1-tripwire-harness.codex.md:2344:**2026-07-04 15:24:23Z** - **CODER (w2-2-stream-ladder-owner, claude)**: Landed w2-2 through the codex loop (owner go: "continue" → topmost open row per the 12th handoff). The row as filed (2026-06-11) predates REV4, which already delivered its three stream-count legs: the `determine_remote_tuning` ladder died at ue-r2-1e (live dial), daemon `desired_streams` at ue-r2-1f (`engine::initial_stream_proposal` — byte- AND file-count-keyed, satisfying the spec's "takes file_count"), and `pull_stream_count` with the Pull RPC at ue-r2-1h; D-2026-06-20-1 recorded the absorption in v1 slice IDs. The remaining leg — the transfer_plan 16/32 MiB chunk ladder — turned out to be **entirely dead policy**, established by a 5-agent audit workflow + hand verification: every remote path overrode it with `Some(dial.chunk_bytes())` (push client 5 refresh sites + ensure_dial, pull_sync both literals); the only paths where the ladder won (local engine, test callers) discarded the value (`PlanUpdate` carries payloads only); the single workspace read of `PlannedPayloads.chunk_bytes` sat behind a `chunk_bytes == 0` guard no live caller can trigger (all pass the dial value, floored ≥ 64 KiB). The spec's "make transfer_plan take chunk_bytes as input" predates the dial — with zero consumers, threading a value through the planner would be plumbing with no reader, so the honest single-owner outcome was deletion. Slice `01209bc`: ladder + `Plan` wrapper deleted (`build_plan` → `Vec<TransferTask>`); `PlannedPayloads` deleted (`plan_transfer_payloads` → `Result<Vec<TransferPayload>>`, ripple through diff_planner/streaming_plan/pipeline tests/re-exports); `PlanOptions.chunk_bytes_override` + all refresh sites deleted (push `plan_options` now immutable default; two arms keep bare `ensure_dial` calls — first-need creation and first-wins ceilings unchanged); unreachable fallback guard in `stream_fallback_from_queue` deleted; `plan_to_daemon_format` deleted (git log -S: never called in repo history — its "server pull mode" comment was never true); orphaned `TuningParams` deleted (producer died at ue-r2-1e); write-only kickoff histogram collapsed to the `total_bytes` accumulator that was its only read. Comment-truth sweep: dial.rs mutability-model doc no longer claims chunk/prefetch are "read at each use site" (consumers snapshot at session/pipeline/batch setup; steps reach epoch-N sockets and later fallback batches); buffer.rs example cites the dial, not `TuningParams`. Behavior byte-identical on every live path. Tests: +4 transfer_plan unit pins (module had zero) — tier classification/interleave, single-small-file no-tar, force_tar single-file, count-target shard splitting with the 128 clamp; deletions are compile-guarded (w2-1 evidence shape); zero tests deleted. Review: codex **NEEDS FIXES (1 Low)** — the first bare ensure_dial comment said "fallback batch" inside the `TransferMode::DataPlane` branch; accepted (mislabel sits exactly on the invariant under review), fixed `27f53a0` (one word). W3.1's "after W2.2 settles the tuning owner" prerequisite is now settled: the owner is `engine::TransferDial`. New discoveries → STATE Open questions: `725aa07` tracked a 236-file stale worktree snapshot (`.claude/worktrees/vigilant-mayer/`) into the repo; WHITEPAPER still describes the pre-dial tuning world (stale since ue-r2-1e, w10 territory). Validation both commits: fmt/clippy clean, `cargo test --workspace` 1452/0/2 across 37 suites (baseline 1448). All on master, unpushed. Next: w3-1 (memory-aware BufferPool) tops the open queue; design-3 remains the sanctioned smaller alternative.
.review/results/sf-1-tripwire-harness.codex.md:2466:  +#                      (chosen to cross engine::initial_stream_proposal
.review/results/sf-1-tripwire-harness.codex.md:2568:    43	  `engine::initial_stream_proposal` tiers (200→1, 1k→2, 5k→4, 10k→8,
.review/results/sf-1-tripwire-harness.codex.md:2589:    64	full workspace suite green; count vs 1479 baseline in verdict file).
docs/audit/inventory/plan-phases.md:146:> "Holds `tokio::sync::Mutex` guard across `await` for the entire data-plane transfer. … canonical anti-pattern." — restructure so the receiver is owned by exactly one task (mpsc::Receiver passed directly, or use flume).
.review/findings/audit-6f-dns-rebinding-test.md:33:  even though a later resolution would have passed — locking in that only
.review/findings/a0-dispatch.md:96:from 496 → 503 passed. No tests removed.
docs/audit/inventory/plan-perf.md:694:"The current 'filter parity' workaround that bails on pull when filter args are passed (CLI side)."
.review/findings/d-16-help-overlay-keymap-sync.md:123:  passed because `r` appeared somewhere, but it would
.review/findings/a0-delegated-execution.md:121:to `blit-app`; workspace total unchanged at 496 passed.
.review/findings/b-1-active-jobs.md:106:   inside the spawn closure, not via a struct passed back
.review/findings/b-1-active-jobs.md:166:Workspace: 507 passed (was 506; +1 contended-drop test).
.review/findings/b-1-active-jobs.md:170:Reviewer: `codex-reviewer`. Validation: fmt + clippy passed,
.review/findings/b-5-jobs-list.md:115:Workspace: 523 passed (was 518; +5).
.review/findings/a0-final-cleanup.md:116:None new. Workspace stays at 503 passed (same as the
.review/findings/ue-r2-1g.md:45:- stream count: `engine::initial_stream_proposal` (this slice —
.review/findings/ue-r2-1g.md:64:  - otherwise `initial_stream_proposal(bytes_to_send,
.review/findings/ue-r2-1f.md:46:- **`engine/dial.rs`**: new `pub fn initial_stream_proposal(total_bytes:
.review/findings/a0-endpoints-gates.md:95:None new. Workspace total unchanged at 503 passed. The gates
.review/findings/a0-endpoints-gates.md:159:No code changes. Workspace still at 503 passed.
.review/findings/c-1a-byte-counter-api.md:118:test in `service::core::tests`. Workspace: 548 passed (was
.review/findings/d-24-config-cancel-ttl.md:313:all passed — short-TTL operators were silently
.review/findings/d-35-f3-pull-execute.md:163:  round 1 passed the raw typed path straight to
.review/results/bench-script-fix.codex.md:4286:crates/blit-core/tests/pull_sync_with_spec_wire.rs:320:        "spec on the wire diverged from the spec passed to pull_sync_with_spec"
.review/results/bench-script-fix.codex.md:5281:crates/blit-core/src/engine/mod.rs:30:    initial_stream_proposal, local_receiver_capacity, spawn_dial_tuner,
.review/results/bench-script-fix.codex.md:5552:crates/blit-core/src/engine/dial.rs:642:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
.review/results/bench-script-fix.codex.md:7279:crates/blit-cli/tests/local_move_semantics.rs:4://! Pre-fix `crates/blit-cli/src/transfers/mod.rs:458` passed
.review/results/bench-script-fix.codex.md:7366:crates/blit-cli/tests/common/mod.rs:125:        // be passed through or the daemon lands in the wrong directory.
.review/results/bench-script-fix.codex.md:8428:crates/blit-daemon/src/service/pull_sync.rs:16:use blit_core::engine::{initial_stream_proposal, TransferDial};
.review/results/bench-script-fix.codex.md:8645:crates/blit-daemon/src/service/pull_sync.rs:1479:/// 1. If resume-eligible: request block hashes from client, compare, send only differing blocks
.review/results/bench-script-fix.codex.md:9088:crates/blit-daemon/src/service/push/control.rs:800:    blit_core::engine::initial_stream_proposal(
.review/results/bench-script-fix.codex.md:9347:  bypassed the claimed set — same race class, missed path) → fixed.
.review/results/bench-script-fix.codex.md:9349:  1478 → 1479/0/2 (+1 mutation-verified keepalive pin; the previously
.review/results/bench-script-fix.codex.md:9350:  recorded "1479" baseline was a different aggregation).
.review/results/bench-script-fix.codex.md:9486:  bypassed the claimed set) → fixed `8641bc6`; records `c62d15b`.
.review/results/bench-script-fix.codex.md:9487:  Gate: fmt/clippy clean; 1478 → 1479/0/2 same-method A/B; full suite
.review/results/bench-script-fix.codex.md:9508:  findings**. +3 tests, mutation-verified; workspace 1476 → 1479/0/2
.review/results/bench-script-fix.codex.md:11112:docs/audit/inventory/code-cli.md:42:- **mirror-prompt** — `crates/blit-cli/src/transfers/mod.rs:181-190` — mirror prompts unless `--yes` OR `--dry-run`; bypassed string is "Mirror will delete extraneous files at destination 'X'. Continue?". _(notes: dry-run does NOT need confirmation; prompt happens BEFORE rejected-flag gates if mirror is set)_
.review/results/bench-script-fix.codex.md:11122:docs/audit/inventory/code-cli.md:165:- **--delete-scope is stringly typed** — `crates/blit-cli/src/cli.rs:247-248, 386-388` — `value_parser` is case-sensitive but `delete_scope_all()` is case-insensitive: `blit copy --delete-scope ALL` would be rejected by clap, but `blit copy --delete-scope all` accepted; meanwhile internal code accepts `All` too if it ever bypassed clap. Should be an enum.
.review/results/bench-script-fix.codex.md:12945:crates/blit-core/src/perf_predictor.rs:1327:        // passed even when the production load() skipped the
.review/findings/b-4-getstate.md:88:Workspace: 517 passed (was 514; +3 unit tests).
.review/findings/b-4-getstate.md:148:Workspace: 518 passed (was 517; +1).
.review/findings/ue-r2-1a.md:79:`cargo test --workspace` **1377 passed / 0 failed / 2 ignored** (baseline
.review/findings/w9-4-readonly-enforcement-tests.md:13:A dropped gate (mirror-deletion blast radius) would have passed validation.
.review/findings/m-jobs-6-watch.md:117:Workspace: 544 passed (was 540; +4).
.review/findings/d-68-f1-remote-remote-copy.md:15:treated the source as remote→local and passed the raw `dest`
.review/findings/d-33-f3-pull-source.md:137:  remotes.** `main.rs` passed `RemoteEndpoint::host`
.review/findings/b-3-recent-ring.md:100:Workspace: 514 passed (was 509; +5).
.review/findings/d-47-f1-browse-nav.md:10:Until now the F3 browser only ever showed the daemon passed via
.review/findings/keys-2-config-refresh.md:61:`refresh = "r"` (or `refresh = "q"` with the default quit) passed
.review/findings/m-jobs-3-detach.md:112:Workspace: 536 passed (was 533; +3).
.review/findings/m-jobs-3-detach.md:205:Workspace: 537 passed (was 536; +1).
.review/findings/m-jobs-3-detach.md:241:Workspace: 540 passed (was 537; +3).
.review/findings/windows-move-tree-hang.md:18:has ever passed Windows CI).
.review/findings/b-2-set-endpoint.md:96:Workspace: 509 passed (was 507; +2).
.review/findings/m-jobs-2-cancel-rpc.md:104:Workspace: 532 passed (was 528; +4 handler tests).
.review/findings/m-jobs-2-cancel-rpc.md:165:Workspace: 533 passed (was 532; +1).
.review/findings/m-jobs-1-cancel-token.md:90:Workspace: 526 passed (was 523; +3).
.review/findings/m-jobs-1-cancel-token.md:173:Workspace: 528 passed (was 526; +2 net).

exec
/usr/bin/zsh -lc 'git diff --name-only c70c2ac''^ c70c2ac' in /home/michael/dev/Blit
 succeeded in 0ms:
.review/findings/sf-2-shape-correction-resize.md
crates/blit-core/src/engine/dial.rs
crates/blit-core/src/remote/push/client/mod.rs
crates/blit-core/src/remote/push/client/types.rs
crates/blit-daemon/src/service/push/mod.rs
crates/blit-daemon/src/service/push/shape_resize_e2e.rs

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/control.rs | sed -n '180,330p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   180	            Some(
   181	                client_push_request::Payload::TarShardHeader(_)
   182	                | client_push_request::Payload::TarShardChunk(_)
   183	                | client_push_request::Payload::TarShardComplete(_),
   184	            ) => {
   185	                return Err(Status::failed_precondition(
   186	                    "tar shard payload received before manifest enumeration completed",
   187	                ));
   188	            }
   189	            Some(client_push_request::Payload::FileManifest(mut file)) => {
   190	                let module_ref = module.as_ref().ok_or_else(|| {
   191	                    Status::failed_precondition("push manifest received before header")
   192	                })?;
   193	                let rel = resolve_manifest_relative_path(&file.relative_path)?;
   194	                expected_rel_files.push(rel.clone());
   195	                // Wire paths are canonically POSIX (`path_posix`). On
   196	                // Windows, `PathBuf::to_string_lossy` re-joins the
   197	                // validated components with backslashes, so the
   198	                // need-list echoed paths the client's manifest lookup
   199	                // (keyed by its own POSIX strings) could never match —
   200	                // every nested-path push to a Windows daemon planned
   201	                // zero payloads for those files and both ends stalled.
   202	                let sanitized = blit_core::path_posix::relative_path_to_posix(&rel);
   203	                file.relative_path = sanitized.clone();
   204	
   205	                // w4-4: buffer the entry; the requires-upload check
   206	                // (canonical containment + stat, 3+ blocking syscalls)
   207	                // runs in chunked spawn_blocking batches instead of
   208	                // inline on the runtime — a 1M-file push used to run
   209	                // ~3M+ blocking syscalls on an executor worker.
   210	                if manifest_buffered_at.is_none() {
   211	                    manifest_buffered_at = Some(Instant::now());
   212	                }
   213	                pending_manifest.push(PendingManifestEntry {
   214	                    rel,
   215	                    sanitized,
   216	                    file,
   217	                });
   218	                if manifest_drain_due(pending_manifest.len(), manifest_buffered_at) {
   219	                    let flushed = drain_manifest_checks(
   220	                        module_ref,
   221	                        &mut pending_manifest,
   222	                        &mut need_list_sender,
   223	                        &mut files_to_upload,
   224	                    )
   225	                    .await?;
   226	                    manifest_buffered_at = None;
   227	                    // design-4: in forced-gRPC mode the early-flush branch
   228	                    // must NOT announce the fallback negotiation here. The
   229	                    // client reacts to Negotiation(tcp_fallback) by
   230	                    // immediately streaming FileData on this same request
   231	                    // stream — but this loop is still reading the manifest,
   232	                    // and its FileData arm is a hard failed_precondition.
   233	                    // That broke every forced-gRPC push of ≥128 files
   234	                    // (FILE_LIST_EARLY_FLUSH_ENTRIES) and was timing-flaky
   235	                    // near ~100. The post-manifest execute_grpc_fallback
   236	                    // sends the one canonical fallback negotiation — the
   237	                    // path every working small push already takes. Early
   238	                    // negotiation only ever helped the TCP path (it starts
   239	                    // the data plane for pipelining), so it is now TCP-only.
   240	                    // (w4-4 moved this from per-entry to post-chunk-drain:
   241	                    // the data plane still spins up mid-manifest on the
   242	                    // first flush, at chunk granularity.)
   243	                    if flushed && data_plane_handle.is_none() && !force_grpc_effective {
   244	                        {
   245	                            let listener = match bind_data_plane_listener().await {
   246	                                Ok(l) => l,
   247	                                Err(_) => {
   248	                                    // Bind failed: flip to fallback mode but
   249	                                    // stay quiet — announcing mid-manifest
   250	                                    // would trip the same design-4 wedge.
   251	                                    fallback_used = true;
   252	                                    force_grpc_effective = true;
   253	                                    continue;
   254	                                }
   255	                            };
   256	
   257	                            let port = listener
   258	                                .local_addr()
   259	                                .map_err(|err| {
   260	                                    Status::internal(format!("querying listener addr: {}", err))
   261	                                })?
   262	                                .port();
   263	
   264	                            let token = generate_token()?;
   265	                            let token_string = general_purpose::STANDARD_NO_PAD.encode(&token);
   266	
   267	                            let module_for_transfer = module_ref.clone();
   268	
   269	                            let stream_target = engine_stream_proposal(&files_to_upload);
   270	                            // ue-r2-2: full resize fold — peer bit AND
   271	                            // own support AND a live TCP data plane
   272	                            // (this literal only exists on that path;
   273	                            // the fallback literal stays false).
   274	                            let resize_on = client_supports_resize;
   275	                            let epoch0_sub = if resize_on {
   276	                                generate_resize_sub_token()?
   277	                            } else {
   278	                                Vec::new()
   279	                            };
   280	                            let transfer_task = if resize_on {
   281	                                let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
   282	                                resize_cmd_tx = Some(cmd_tx);
   283	                                resize_live = stream_target.max(1);
   284	                                AbortOnDrop::new(tokio::spawn(
   285	                                    accept_data_connection_stream_resizable(
   286	                                        listener,
   287	                                        token.clone(),
   288	                                        epoch0_sub.clone(),
   289	                                        module_for_transfer,
   290	                                        stream_target,
   291	                                        cmd_rx,
   292	                                    ),
   293	                                ))
   294	                            } else {
   295	                                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream(
   296	                                    listener,
   297	                                    token.clone(),
   298	                                    module_for_transfer,
   299	                                    stream_target,
   300	                                )))
   301	                            };
   302	
   303	                            send_control_message(
   304	                                &tx,
   305	                                server_push_response::Payload::Negotiation(
   306	                                    DataTransferNegotiation {
   307	                                        tcp_port: port as u32,
   308	                                        one_time_token: token_string,
   309	                                        tcp_fallback: false,
   310	                                        stream_count: stream_target,
   311	                                        // ue-r2-1e: the daemon is the
   312	                                        // byte receiver on push — it
   313	                                        // advertises its capacity so
   314	                                        // the client's dial can ramp
   315	                                        // within it.
   316	                                        receiver_capacity: Some(
   317	                                            blit_core::engine::local_receiver_capacity(),
   318	                                        ),
   319	                                        resize_enabled: resize_on,
   320	                                        epoch0_sub_token: epoch0_sub,
   321	                                    },
   322	                                ),
   323	                            )
   324	                            .await?;
   325	
   326	                            data_plane_handle = Some(transfer_task);
   327	                        }
   328	                    }
   329	                }
   330	            }

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/control.rs | sed -n '760,820p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   760	            files_to_upload.push(entry.file);
   761	        }
   762	    }
   763	    Ok(any_flushed)
   764	}
   765	
   766	fn file_requires_upload(
   767	    module: &ModuleConfig,
   768	    rel: &Path,
   769	    header: &FileHeader,
   770	) -> Result<bool, Status> {
   771	    use super::super::util::resolve_contained_path;
   772	    // F2: canonical containment check before stat. Same protection
   773	    // as the actual write path — a symlink in the parent could
   774	    // otherwise have us stat outside the module.
   775	    let full_path = resolve_contained_path(module, rel)?;
   776	    let requires_upload = match fs::metadata(&full_path) {
   777	        Ok(meta) => {
   778	            if !meta.is_file() {
   779	                true
   780	            } else {
   781	                let same_size = meta.len() == header.size;
   782	                let same_mtime = metadata_mtime_seconds(&meta)
   783	                    .map(|seconds| seconds == header.mtime_seconds)
   784	                    .unwrap_or(false);
   785	                !(same_size && same_mtime)
   786	            }
   787	        }
   788	        Err(_) => true,
   789	    };
   790	    Ok(requires_upload)
   791	}
   792	
   793	/// ue-r2-1f: the daemon's private `desired_streams` ladder retired
   794	/// into the engine's shared shape-aware proposal (same table), clamped
   795	/// to the receiver ceiling this daemon advertises in its
   796	/// CapacityProfile. Single owner for the push stream-count start; the
   797	/// client's dial clamps again on its side.
   798	fn engine_stream_proposal(files: &[FileHeader]) -> u32 {
   799	    let total_bytes: u64 = files.iter().map(|f| f.size).sum();
   800	    blit_core::engine::initial_stream_proposal(
   801	        total_bytes,
   802	        files.len(),
   803	        blit_core::engine::local_receiver_capacity().max_streams as usize,
   804	    )
   805	}
   806	
   807	#[cfg(test)]
   808	mod data_plane_handle_abort_tests {
   809	    //! design-2 / w4-1: `handle_push_stream`'s `data_plane_handle` was
   810	    //! a bare `Option<JoinHandle<...>>`. Any early `?` return while a
   811	    //! data-plane accept/receive task was running (the manifest
   812	    //! loop's several fallible `send_control_message` calls, or the
   813	    //! `stream.message()?` race in the post-manifest select loop)
   814	    //! dropped the handle without aborting it, leaving the task
   815	    //! running with no owner — unreachable by `CancelJob`. This pins
   816	    //! the fix at the field-type level: wrapping the same
   817	    //! `tokio::spawn` result in `AbortOnDrop` and dropping the
   818	    //! `Option` (simulating the early-return path) must abort the
   819	    //! task instead of detaching it. The full handler is exercised
   820	    //! end-to-end elsewhere; reproducing a real gRPC push stream just

exec
/usr/bin/zsh -lc "rg -n \"sf-2|small-file|shape|many_tiny|data_plane_streams|10k\" crates/blit-core/src/remote/push crates/blit-daemon/src/service/push -g'*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/push/mod.rs:4:mod shape_resize_e2e;
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:1://! sf-2 loopback e2e pin (`docs/plan/SMALL_FILE_CEILING.md`, slice
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:2://! sf-2): a many-tiny-file push must open more than one data-plane
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:6://! flush (`FILE_LIST_EARLY_FLUSH_ENTRIES` = 128 entries), so a 10k-file
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:10://! `docs/bench/10gbe-2026-07-05/`). The client-side shape-correction
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:11://! resize (`maybe_shape_resize` in blit-core's push client) re-runs the
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:12://! shape table over the accumulated need list and corrects upward
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:31:async fn many_tiny_file_push_opens_more_than_one_data_plane_connection() {
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:66:    // The plan's small-file cell: 10k tiny files. The shape table
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:107:        .data_plane_streams
crates/blit-daemon/src/service/push/shape_resize_e2e.rs:112:         1-stream proposal upward via shape resize; settled at {streams}"
crates/blit-daemon/src/service/push/data_plane.rs:186:    // Same call shape as the client's pull-receive side. Tar shards get
crates/blit-daemon/src/service/push/data_plane.rs:287:    // design-2 shape for this path; w4-1 still owns the family).
crates/blit-daemon/src/service/push/data_plane.rs:871:///    threading a streaming-tar receiver into the sink shape, which
crates/blit-daemon/src/service/push/control.rs:135:                // R59 #1: capture F1 / F2 fields from the new wire shape.
crates/blit-daemon/src/service/push/control.rs:794:/// into the engine's shared shape-aware proposal (same table), clamped
crates/blit-core/src/remote/push/client/types.rs:12:    /// sf-2: the dial's settled live stream count when the transfer
crates/blit-core/src/remote/push/client/types.rs:14:    /// Observable pin for the shape-correction resize: a many-tiny-file
crates/blit-core/src/remote/push/client/types.rs:16:    pub data_plane_streams: Option<usize>,
crates/blit-core/src/remote/push/client/mod.rs:479:/// ue-r2-2 / sf-2 shared pre-dial ADD: mint the epoch credential, send
crates/blit-core/src/remote/push/client/mod.rs:517:/// sf-2: one shape-correction step. The daemon proposes the epoch-0
crates/blit-core/src/remote/push/client/mod.rs:520:/// than the shape table assigns the full workload
crates/blit-core/src/remote/push/client/mod.rs:524:/// over the ACTUAL transfer shape (need-list files + bytes, not the
crates/blit-core/src/remote/push/client/mod.rs:528:async fn maybe_shape_resize(
crates/blit-core/src/remote/push/client/mod.rs:541:    match dial.propose_shape_resize(target) {
crates/blit-core/src/remote/push/client/mod.rs:745:        // sf-2: shape-correction gate. `resize_negotiated` records that
crates/blit-core/src/remote/push/client/mod.rs:747:        // present). `shape_resize_enabled` flips off permanently the
crates/blit-core/src/remote/push/client/mod.rs:749:        // evidence outranks the static shape table, and re-adding what
crates/blit-core/src/remote/push/client/mod.rs:752:        let mut shape_resize_enabled = true;
crates/blit-core/src/remote/push/client/mod.rs:856:                                            // sf-2: the need list just grew —
crates/blit-core/src/remote/push/client/mod.rs:857:                                            // re-run the shape table and
crates/blit-core/src/remote/push/client/mod.rs:861:                                                && shape_resize_enabled
crates/blit-core/src/remote/push/client/mod.rs:865:                                                    if let Err(send_err) = maybe_shape_resize(
crates/blit-core/src/remote/push/client/mod.rs:1032:                                            // sf-2: need-list batches can
crates/blit-core/src/remote/push/client/mod.rs:1034:                                            // accumulated shape may already
crates/blit-core/src/remote/push/client/mod.rs:1037:                                            if resize_negotiated && shape_resize_enabled {
crates/blit-core/src/remote/push/client/mod.rs:1038:                                                if let Err(send_err) = maybe_shape_resize(
crates/blit-core/src/remote/push/client/mod.rs:1161:                                            // sf-2: the epoch settled — if the
crates/blit-core/src/remote/push/client/mod.rs:1162:                                            // need-list shape still wants more
crates/blit-core/src/remote/push/client/mod.rs:1167:                                                && shape_resize_enabled
crates/blit-core/src/remote/push/client/mod.rs:1171:                                                if let Err(send_err) = maybe_shape_resize(
crates/blit-core/src/remote/push/client/mod.rs:1391:                                // sf-2: the tuner wants FEWER streams — live
crates/blit-core/src/remote/push/client/mod.rs:1393:                                // shape table from here on. Never re-add what
crates/blit-core/src/remote/push/client/mod.rs:1395:                                shape_resize_enabled = false;
crates/blit-core/src/remote/push/client/mod.rs:1517:            data_plane_streams: match (&dial, data_port) {

codex
For the count gate, I’m not going to run the suite in this read-only sandbox because Cargo would need write access. I can still check the diff mechanically for added/deleted test annotations and compare that to the recorded 1479→1483 claim.
exec
/usr/bin/zsh -lc 'git diff c70c2ac''^ c70c2ac -- '"'*.rs' | rg -n '"'^[+-]'"\\s*#\\[(tokio::test|test)'" in /home/michael/dev/Blit
 succeeded in 0ms:
75:+    #[test]
97:+    #[test]
139:+    #[test]
488:+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]

exec
/usr/bin/zsh -lc 'git diff c70c2ac''^ c70c2ac -- crates/blit-core/src/engine/dial.rs crates/blit-daemon/src/service/push/shape_resize_e2e.rs | rg -n '"'"'^[+].*fn .*'"\\('" in /home/michael/dev/Blit
 succeeded in 0ms:
42:+    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
76:+    fn shape_table_covers_the_small_file_ceiling_cells() {
98:+    fn shape_resize_ramps_one_epoch_at_a_time_toward_the_target() {
140:+    fn shape_resize_clamps_to_the_profile_ceiling() {
194:+async fn many_tiny_file_push_opens_more_than_one_data_plane_connection() {

exec
/usr/bin/zsh -lc 'git diff c70c2ac''^ c70c2ac -- crates/blit-core/src/engine/dial.rs crates/blit-core/src/remote/push/client/mod.rs crates/blit-daemon/src/service/push/shape_resize_e2e.rs | rg -n '"'"'^[+-]'"'" in /home/michael/dev/Blit
 succeeded in 0ms:
3:--- a/crates/blit-core/src/engine/dial.rs
4:+++ b/crates/blit-core/src/engine/dial.rs
9:-        self.pending_epoch.store(epoch, Ordering::Relaxed);
10:+        // CAS, not store: `propose_shape_resize` (sf-2) allocates from
11:+        // another task, and a plain store here could stack two live
12:+        // proposals onto one epoch number.
13:+        if self
14:+            .pending_epoch
15:+            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
16:+            .is_err()
17:+        {
18:+            return None;
19:+        }
27:+    /// sf-2: shape-correction proposal. On push the daemon proposes the
28:+    /// epoch-0 stream count from whatever manifest prefix it has seen at
29:+    /// the early flush (`FILE_LIST_EARLY_FLUSH_ENTRIES`), so a
30:+    /// many-tiny-file push can negotiate far fewer streams than
31:+    /// [`initial_stream_proposal`] assigns the full workload. As the
32:+    /// need list accumulates client-side, the client re-runs the shape
33:+    /// table and corrects upward through the normal resize wire.
34:+    ///
35:+    /// Unlike [`Self::resize_tick`] this is a definite signal — the
36:+    /// shape is known, not inferred from throughput — so there is no
37:+    /// sustain/cooldown discipline. It still honors one-in-flight and
38:+    /// the receiver-profile ceiling, still moves ONE stream per epoch
39:+    /// (the wire carries one `sub_token` per ADD), and never proposes
40:+    /// REMOVE: shrinking below a live count is throughput evidence and
41:+    /// stays the tuner's call.
42:+    pub fn propose_shape_resize(&self, desired_streams: usize) -> Option<ResizeProposal> {
43:+        let desired = desired_streams.clamp(1, self.ceiling_max_streams.max(1));
44:+        let live = self.live_streams.load(Ordering::Relaxed).max(1);
45:+        if desired <= live {
46:+            return None;
47:+        }
48:+        let epoch = self.resize_epoch.load(Ordering::Relaxed).saturating_add(1);
49:+        if self
50:+            .pending_epoch
51:+            .compare_exchange(0, epoch, Ordering::Relaxed, Ordering::Relaxed)
52:+            .is_err()
53:+        {
54:+            return None;
55:+        }
56:+        Some(ResizeProposal {
57:+            epoch,
58:+            target_streams: live + 1,
59:+            add: true,
60:+        })
61:+    }
62:+
70:+    // ── sf-2 shape-correction resize ─────────────────────────────────
71:+
72:+    /// The plan's three measured 10 GbE cells mapped through the shape
73:+    /// table (`docs/plan/SMALL_FILE_CEILING.md`): the small and mixed
74:+    /// cells must NOT ride the byte tiers alone.
75:+    #[test]
76:+    fn shape_table_covers_the_small_file_ceiling_cells() {
77:+        const KIB: u64 = 1024;
78:+        const MIB64: u64 = 1024 * KIB;
79:+        const GIB: u64 = 1024 * MIB64;
80:+        // push/pull 10k × 4 KiB: 40 MiB is the 2-stream byte tier, but
81:+        // 10_000 files must key the 8-stream file-count tier.
82:+        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
83:+        // 1 × 1 GiB: byte-keyed, file count is irrelevant — unchanged.
84:+        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
85:+        // mixed 512 MiB + 5k × 2 KiB: the byte tier already reaches 8;
86:+        // the 5_001 files alone would say 4 — bytes win.
87:+        assert_eq!(
88:+            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
89:+            8
90:+        );
91:+        // sf-1 loopback probe evidence: 1_000 tiny files must propose 2
92:+        // (the measured transfer rode 1 — the input, not this table,
93:+        // was wrong).
94:+        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
95:+    }
96:+
97:+    #[test]
98:+    fn shape_resize_ramps_one_epoch_at_a_time_toward_the_target() {
99:+        let dial = TransferDial::conservative();
100:+        dial.set_negotiated_streams(1);
101:+
102:+        // At or below live: nothing to correct.
103:+        assert_eq!(dial.propose_shape_resize(0), None);
104:+        assert_eq!(dial.propose_shape_resize(1), None);
105:+
106:+        // Target 3 from live 1: epoch 1 proposes 2 (one per epoch),
107:+        // and the in-flight epoch blocks both proposers.
108:+        let p1 = dial.propose_shape_resize(3).expect("live 1 → target 3");
109:+        assert_eq!(
110:+            p1,
111:+            ResizeProposal {
112:+                epoch: 1,
113:+                target_streams: 2,
114:+                add: true
115:+            }
116:+        );
117:+        assert_eq!(dial.propose_shape_resize(3), None, "one in flight");
118:+        assert_eq!(dial.resize_tick(1024, 0.0), None, "tuner blocked too");
119:+
120:+        // Settle → next step; no cooldown for the definite shape signal.
121:+        dial.resize_settled(1, 2, true);
122:+        let p2 = dial.propose_shape_resize(3).expect("live 2 → target 3");
123:+        assert_eq!(p2.epoch, 2);
124:+        assert_eq!(p2.target_streams, 3);
125:+        dial.resize_settled(2, 3, true);
126:+        assert_eq!(dial.live_streams(), 3);
127:+        assert_eq!(dial.propose_shape_resize(3), None, "target reached");
128:+
129:+        // A refused epoch leaves live untouched; the next call retries.
130:+        let p3 = dial.propose_shape_resize(4).expect("live 3 → target 4");
131:+        dial.resize_settled(p3.epoch, dial.live_streams(), false);
132:+        assert_eq!(dial.live_streams(), 3);
133:+        assert!(
134:+            dial.propose_shape_resize(4).is_some(),
135:+            "retry after refusal"
136:+        );
137:+    }
138:+
139:+    #[test]
140:+    fn shape_resize_clamps_to_the_profile_ceiling() {
141:+        let dial = TransferDial::conservative_within(Some(&profile(2, 0, 0)));
142:+        dial.set_negotiated_streams(1);
143:+        let p = dial
144:+            .propose_shape_resize(100)
145:+            .expect("clamped, not refused");
146:+        assert_eq!(p.target_streams, 2);
147:+        dial.resize_settled(p.epoch, 2, true);
148:+        assert_eq!(
149:+            dial.propose_shape_resize(100),
150:+            None,
151:+            "at the receiver's advertised ceiling"
152:+        );
153:+    }
154:+
160:--- a/crates/blit-core/src/remote/push/client/mod.rs
161:+++ b/crates/blit-core/src/remote/push/client/mod.rs
166:+/// ue-r2-2 / sf-2 shared pre-dial ADD: mint the epoch credential, send
167:+/// the `DataPlaneResize` ADD, and record the in-flight epoch (the
168:+/// socket itself is dialed on the daemon's ack). A missing credential
169:+/// source settles the epoch failed and is not an error; a send error
170:+/// is returned for the caller to route through `prefer_server_error`.
171:+async fn send_resize_add(
172:+    tx: &mpsc::Sender<ClientPushRequest>,
173:+    dial: &crate::engine::TransferDial,
174:+    proposal: crate::engine::ResizeProposal,
175:+    resize_pending: &mut Option<PendingResize>,
176:+) -> Result<()> {
177:+    match crate::remote::transfer::generate_sub_token() {
178:+        Ok(sub) => {
179:+            send_payload(
180:+                tx,
181:+                ClientPayload::DataPlaneResize(DataPlaneResize {
182:+                    op: DataPlaneResizeOp::Add as i32,
183:+                    epoch: proposal.epoch,
184:+                    target_stream_count: proposal.target_streams as u32,
185:+                    sub_token: sub.clone(),
186:+                }),
187:+            )
188:+            .await?;
189:+            *resize_pending = Some(PendingResize {
190:+                epoch: proposal.epoch,
191:+                target: proposal.target_streams,
192:+                add: true,
193:+                sub_token: sub,
194:+            });
195:+        }
196:+        Err(err) => {
197:+            log::warn!("resize ADD skipped (no credential source): {err:#}");
198:+            dial.resize_settled(proposal.epoch, dial.live_streams(), false);
199:+        }
200:+    }
201:+    Ok(())
202:+}
203:+
204:+/// sf-2: one shape-correction step. The daemon proposes the epoch-0
205:+/// stream count from whatever manifest prefix it had seen at its early
206:+/// flush, so a many-tiny-file push can negotiate far fewer streams
207:+/// than the shape table assigns the full workload
208:+/// (`.review/findings/sf-1-tripwire-harness.md` Known gaps: a
209:+/// 1000-file push measured 1 stream where the table says 2). As the
210:+/// need list accumulates, re-run [`crate::engine::initial_stream_proposal`]
211:+/// over the ACTUAL transfer shape (need-list files + bytes, not the
212:+/// manifest — an incremental push of a large tree may move only a few
213:+/// files) and correct upward one ADD epoch at a time. Call sites gate
214:+/// on the transfer running resize-enabled on the data plane.
215:+async fn maybe_shape_resize(
216:+    tx: &mpsc::Sender<ClientPushRequest>,
217:+    dial: &crate::engine::TransferDial,
218:+    need_bytes: u64,
219:+    need_count: usize,
220:+    resize_pending: &mut Option<PendingResize>,
221:+) -> Result<()> {
222:+    if resize_pending.is_some() {
223:+        return Ok(());
224:+    }
225:+    let target =
226:+        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())
227:+            as usize;
228:+    match dial.propose_shape_resize(target) {
229:+        Some(proposal) => send_resize_add(tx, dial, proposal, resize_pending).await,
230:+        None => Ok(()),
231:+    }
232:+}
233:+
241:+        // sf-2: shape-correction gate. `resize_negotiated` records that
242:+        // this transfer's data plane went elastic (epoch-0 sub-token
243:+        // present). `shape_resize_enabled` flips off permanently the
244:+        // first time the tuner proposes a REMOVE — live throughput
245:+        // evidence outranks the static shape table, and re-adding what
246:+        // the tuner just retired would flap.
247:+        let mut resize_negotiated = false;
248:+        let mut shape_resize_enabled = true;
256:+                                            // sf-2: the need list just grew —
257:+                                            // re-run the shape table and
258:+                                            // correct the stream count before
259:+                                            // queueing the batch.
260:+                                            if resize_negotiated
261:+                                                && shape_resize_enabled
262:+                                                && data_plane_sender.is_some()
263:+                                            {
264:+                                                if let Some(dial_ref) = dial.as_ref() {
265:+                                                    if let Err(send_err) = maybe_shape_resize(
266:+                                                        &tx,
267:+                                                        dial_ref,
268:+                                                        transfer_size_hint,
269:+                                                        requested_files.len(),
270:+                                                        &mut resize_pending,
271:+                                                    )
272:+                                                    .await
273:+                                                    {
274:+                                                        return Err(prefer_server_error(
275:+                                                            &mut response_rx,
276:+                                                            send_err,
277:+                                                        )
278:+                                                        .await);
279:+                                                    }
280:+                                                }
281:+                                            }
289:+                                            resize_negotiated = resize_sub.is_some();
297:+
298:+                                            // sf-2: need-list batches can
299:+                                            // predate the negotiation — the
300:+                                            // accumulated shape may already
301:+                                            // outgrow the daemon's
302:+                                            // partial-manifest stream count.
303:+                                            if resize_negotiated && shape_resize_enabled {
304:+                                                if let Err(send_err) = maybe_shape_resize(
305:+                                                    &tx,
306:+                                                    &dial,
307:+                                                    transfer_size_hint,
308:+                                                    requested_files.len(),
309:+                                                    &mut resize_pending,
310:+                                                )
311:+                                                .await
312:+                                                {
313:+                                                    return Err(prefer_server_error(
314:+                                                        &mut response_rx,
315:+                                                        send_err,
316:+                                                    )
317:+                                                    .await);
318:+                                                }
319:+                                            }
327:+                                            // sf-2: the epoch settled — if the
328:+                                            // need-list shape still wants more
329:+                                            // streams, propose the next single
330:+                                            // ADD (the ramp is one stream per
331:+                                            // acked epoch).
332:+                                            if resize_negotiated
333:+                                                && shape_resize_enabled
334:+                                                && data_plane_sender.is_some()
335:+                                            {
336:+                                                let dial_ref = dial_ref.clone();
337:+                                                if let Err(send_err) = maybe_shape_resize(
338:+                                                    &tx,
339:+                                                    &dial_ref,
340:+                                                    transfer_size_hint,
341:+                                                    requested_files.len(),
342:+                                                    &mut resize_pending,
343:+                                                )
344:+                                                .await
345:+                                                {
346:+                                                    return Err(prefer_server_error(
347:+                                                        &mut response_rx,
348:+                                                        send_err,
349:+                                                    )
350:+                                                    .await);
351:+                                                }
352:+                                            }
360:-                                match crate::remote::transfer::generate_sub_token() {
361:-                                    Ok(sub) => {
362:-                                        if let Err(send_err) = send_payload(
363:-                                            &tx,
364:-                                            ClientPayload::DataPlaneResize(DataPlaneResize {
365:-                                                op: DataPlaneResizeOp::Add as i32,
366:-                                                epoch: p.epoch,
367:-                                                target_stream_count: p.target_streams as u32,
368:-                                                sub_token: sub.clone(),
369:-                                            }),
370:-                                        )
371:-                                        .await
372:-                                        {
373:-                                            return Err(prefer_server_error(
374:-                                                &mut response_rx,
375:-                                                send_err,
376:-                                            )
377:-                                            .await);
378:-                                        }
379:-                                        resize_pending = Some(PendingResize {
380:-                                            epoch: p.epoch,
381:-                                            target: p.target_streams,
382:-                                            add: true,
383:-                                            sub_token: sub,
384:-                                        });
385:-                                    }
386:-                                    Err(err) => {
387:-                                        log::warn!(
388:-                                            "resize ADD skipped (no credential source): {err:#}"
389:-                                        );
390:-                                        dial_ref.resize_settled(
391:-                                            p.epoch,
392:-                                            dial_ref.live_streams(),
393:-                                            false,
394:-                                        );
395:-                                    }
396:+                                if let Err(send_err) =
397:+                                    send_resize_add(&tx, dial_ref, p, &mut resize_pending).await
398:+                                {
399:+                                    return Err(prefer_server_error(
400:+                                        &mut response_rx,
401:+                                        send_err,
402:+                                    )
403:+                                    .await);
406:+                                // sf-2: the tuner wants FEWER streams — live
407:+                                // throughput evidence outranks the static
408:+                                // shape table from here on. Never re-add what
409:+                                // the tuner retires.
410:+                                shape_resize_enabled = false;
418:+            data_plane_streams: match (&dial, data_port) {
419:+                (Some(dial), Some(_)) => Some(dial.live_streams()),
420:+                _ => None,
421:+            },
428:--- /dev/null
429:+++ b/crates/blit-daemon/src/service/push/shape_resize_e2e.rs
431:+//! sf-2 loopback e2e pin (`docs/plan/SMALL_FILE_CEILING.md`, slice
432:+//! sf-2): a many-tiny-file push must open more than one data-plane
433:+//! connection.
434:+//!
435:+//! The daemon proposes the epoch-0 stream count at its early manifest
436:+//! flush (`FILE_LIST_EARLY_FLUSH_ENTRIES` = 128 entries), so a 10k-file
437:+//! push used to negotiate from a ~128-file prefix — 1 stream — and ride
438:+//! it for the whole transfer (measured on the 10 GbE rig and again by
439:+//! the sf-1 loopback probe; see DIAGNOSIS.md in
440:+//! `docs/bench/10gbe-2026-07-05/`). The client-side shape-correction
441:+//! resize (`maybe_shape_resize` in blit-core's push client) re-runs the
442:+//! shape table over the accumulated need list and corrects upward
443:+//! through the ue-r2-2 resize wire. This test runs the REAL daemon push
444:+//! service in-process and the REAL client against it, then pins the
445:+//! settled stream count above 1.
446:+
447:+use std::collections::HashMap;
448:+use std::path::PathBuf;
449:+use std::sync::Arc;
450:+
451:+use blit_core::fs_enum::FileFilter;
452:+use blit_core::generated::blit_server::BlitServer;
453:+use blit_core::generated::MirrorMode;
454:+use blit_core::remote::transfer::source::FsTransferSource;
455:+use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePushClient};
456:+
457:+use crate::runtime::ModuleConfig;
458:+use crate::service::BlitService;
459:+
460:+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
461:+async fn many_tiny_file_push_opens_more_than_one_data_plane_connection() {
462:+    let dest = tempfile::tempdir().expect("dest dir");
463:+    let canonical = dest.path().canonicalize().expect("canonical dest");
464:+    let mut modules = HashMap::new();
465:+    modules.insert(
466:+        "test".to_string(),
467:+        ModuleConfig {
468:+            name: "test".into(),
469:+            path: canonical.clone(),
470:+            canonical_root: canonical.clone(),
471:+            read_only: false,
472:+            _comment: None,
473:+            delegation_allowed: true,
474:+        },
475:+    );
476:+    let service = BlitService::with_modules(modules, false);
477:+
478:+    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
479:+        .await
480:+        .expect("bind loopback listener");
481:+    let port = listener.local_addr().expect("listener addr").port();
482:+    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
483:+    let server = tokio::spawn(async move {
484:+        blit_core::remote::grpc_server::production_server_builder()
485:+            .add_service(BlitServer::new(service))
486:+            .serve_with_incoming_shutdown(
487:+                tokio_stream::wrappers::TcpListenerStream::new(listener),
488:+                async {
489:+                    let _ = shutdown_rx.await;
490:+                },
491:+            )
492:+            .await
493:+            .expect("in-process daemon serves");
494:+    });
495:+
496:+    // The plan's small-file cell: 10k tiny files. The shape table
497:+    // assigns 8 streams (file-count tier); the early-flush proposal
498:+    // sees only the first manifest chunk and starts at 1.
499:+    const FILE_COUNT: usize = 10_000;
500:+    let src = tempfile::tempdir().expect("src dir");
501:+    for i in 0..FILE_COUNT {
502:+        std::fs::write(src.path().join(format!("f{i:05}.bin")), b"x").expect("seed source file");
503:+    }
504:+
505:+    let endpoint = RemoteEndpoint {
506:+        host: "127.0.0.1".into(),
507:+        port,
508:+        path: RemotePath::Module {
509:+            module: "test".into(),
510:+            rel_path: PathBuf::new(),
511:+        },
512:+    };
513:+    let mut client = RemotePushClient::connect(endpoint)
514:+        .await
515:+        .expect("client connects");
516:+    let source = Arc::new(FsTransferSource::new(src.path().to_path_buf()));
517:+    let report = client
518:+        .push(
519:+            source,
520:+            &FileFilter::default(),
521:+            false,
522:+            MirrorMode::FilteredSubset,
523:+            false,
524:+            false,
525:+            None,
526:+            false,
527:+        )
528:+        .await
529:+        .expect("push succeeds");
530:+
531:+    assert!(!report.fallback_used, "must ride the TCP data plane");
532:+    assert_eq!(
533:+        report.summary.files_transferred as usize, FILE_COUNT,
534:+        "every file arrives"
535:+    );
536:+    let streams = report
537:+        .data_plane_streams
538:+        .expect("data plane ran, stream count recorded");
539:+    assert!(
540:+        streams > 1,
541:+        "a {FILE_COUNT}-file push must correct the partial-manifest \
542:+         1-stream proposal upward via shape resize; settled at {streams}"
543:+    );
544:+
545:+    let _ = shutdown_tx.send(());
546:+    server.await.expect("server task joins");
547:+}

codex
I’m also checking the “finish while ADD is pending” path. Shape resize can fire late, and the finish condition does not explicitly wait for resize acks, so the daemon acceptor behavior matters for whether this can stall or only costs a bounded no-op.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/data_plane.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use crate::runtime::ModuleConfig;
     2	use blit_core::buffer::BufferPool;
     3	use blit_core::generated::{
     4	    client_push_request, server_push_response, ClientPushRequest, DataTransferNegotiation,
     5	    FileHeader,
     6	};
     7	use blit_core::remote::transfer::pipeline::execute_receive_pipeline;
     8	use blit_core::remote::transfer::sink::{SinkOutcome, TransferSink};
     9	use blit_core::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
    10	use blit_core::remote::transfer::tar_safety;
    11	use blit_core::remote::transfer::{
    12	    configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
    13	};
    14	use eyre::Result;
    15	use rand::{rngs::SysRng, TryRng};
    16	use std::collections::HashMap;
    17	use std::path::PathBuf;
    18	use std::sync::Arc;
    19	use std::time::{Duration, Instant};
    20	use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
    21	use tokio::net::{TcpListener, TcpStream};
    22	use tokio::sync::Semaphore;
    23	use tokio::task::JoinSet;
    24	use tonic::{Status, Streaming};
    25	
    26	use super::super::util::resolve_manifest_relative_path;
    27	use super::super::PushSender;
    28	use super::control::send_control_message;
    29	
    30	const TOKEN_LEN: usize = 32;
    31	const MAX_PARALLEL_TAR_TASKS: usize = 4;
    32	
    33	/// Default buffer size for pooled tar shard buffers (4 MiB).
    34	const TAR_BUFFER_SIZE: usize = 4 * 1024 * 1024;
    35	/// Maximum pooled buffers per connection stream.
    36	const TAR_BUFFER_POOL_SIZE: usize = 8;
    37	
    38	#[derive(Debug, Default, Clone, Copy)]
    39	pub(crate) struct TransferStats {
    40	    pub files_transferred: u64,
    41	    pub bytes_transferred: u64,
    42	    pub bytes_zero_copy: u64,
    43	}
    44	
    45	pub(crate) async fn bind_data_plane_listener() -> Result<TcpListener, Status> {
    46	    TcpListener::bind("0.0.0.0:0")
    47	        .await
    48	        .map_err(|err| Status::internal(format!("failed to bind data plane socket: {}", err)))
    49	}
    50	
    51	/// Generate a random data-plane handshake token.
    52	///
    53	/// audit-3b: the OS cryptographic RNG is effectively always available,
    54	/// but `try_fill_bytes` is fallible (a sandboxed / fd-exhausted
    55	/// container can deny it). Pre-fix this `expect`ed and panicked the
    56	/// spawned data-plane task, leaving the control-plane stream hung
    57	/// waiting for a handshake that would never arrive. Now it returns a
    58	/// `Status::Internal` the handler propagates as a clean RPC error.
    59	pub(crate) fn generate_token() -> Result<Vec<u8>, Status> {
    60	    let mut buf = vec![0u8; TOKEN_LEN];
    61	    SysRng
    62	        .try_fill_bytes(&mut buf)
    63	        .map_err(|err| Status::internal(format!("system RNG unavailable: {err}")))?;
    64	    Ok(buf)
    65	}
    66	
    67	pub(crate) async fn accept_data_connection_stream(
    68	    listener: TcpListener,
    69	    expected_token: Vec<u8>,
    70	    module: ModuleConfig,
    71	    stream_count: u32,
    72	) -> Result<TransferStats, Status> {
    73	    let start = Instant::now();
    74	    let streams = stream_count.max(1) as usize;
    75	    // w4-1: a JoinSet, not a Vec<JoinHandle> — dropping a JoinSet
    76	    // aborts every remaining worker, so a first-error return (or this
    77	    // whole future being cancelled) no longer detaches the survivors.
    78	    // Mirrors `accept_data_connection_stream_resizable`, which fixed
    79	    // this same class during ue-r2-2.
    80	    let mut join_set: JoinSet<Result<TransferStats, Status>> = JoinSet::new();
    81	
    82	    for idx in 0..streams {
    83	        let (accepted, addr) =
    84	            match tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await {
    85	                Ok(Ok(pair)) => pair,
    86	                Ok(Err(err)) => {
    87	                    return Err(Status::internal(format!(
    88	                        "data plane accept failed: {}",
    89	                        err
    90	                    )));
    91	                }
    92	                Err(_elapsed) => {
    93	                    return Err(Status::deadline_exceeded(format!(
    94	                        "data plane accept timed out after {:?} waiting for stream {}/{}",
    95	                        DATA_PLANE_ACCEPT_TIMEOUT,
    96	                        idx + 1,
    97	                        streams
    98	                    )));
    99	                }
   100	            };
   101	        // Enable nodelay + keepalive to prevent idle stream timeouts
   102	        // during long transfers on other streams. No tuned buffer:
   103	        // the daemon is the byte receiver here and holds no dial.
   104	        configure_data_socket(&accepted, None)
   105	            .map_err(|err| Status::internal(format!("configuring data socket: {err}")))?;
   106	        let socket = accepted;
   107	        eprintln!(
   108	            "blitd: push data plane: accepted connection {} from {}",
   109	            idx, addr
   110	        );
   111	        let expected_token = expected_token.clone();
   112	        let module_clone = module.clone();
   113	        join_set.spawn(async move {
   114	            handle_data_plane_stream(socket, expected_token, module_clone).await
   115	        });
   116	    }
   117	
   118	    let mut final_stats = TransferStats::default();
   119	    while let Some(joined) = join_set.join_next().await {
   120	        match joined {
   121	            Ok(Ok(stats)) => accumulate_transfer_stats(&mut final_stats, &stats),
   122	            Ok(Err(status)) => return Err(status),
   123	            Err(_) => return Err(Status::internal("data plane worker cancelled")),
   124	        }
   125	    }
   126	
   127	    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
   128	    let gbps = (final_stats.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
   129	    eprintln!(
   130	        "blitd: push data plane: aggregate throughput {:.2} Gbps ({} bytes in {:.2}s)",
   131	        gbps, final_stats.bytes_transferred, elapsed
   132	    );
   133	
   134	    Ok(final_stats)
   135	}
   136	
   137	async fn handle_data_plane_stream(
   138	    mut socket: TcpStream,
   139	    expected_token: Vec<u8>,
   140	    module: ModuleConfig,
   141	) -> Result<TransferStats, Status> {
   142	    let start = Instant::now();
   143	    let mut token_buf = vec![0u8; expected_token.len()];
   144	    // R46-F7: bounded wait on the token. A stalled peer that
   145	    // accepted the socket but never sent bytes would otherwise hold
   146	    // this worker indefinitely.
   147	    match tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut token_buf)).await {
   148	        Ok(Ok(_)) => {}
   149	        Ok(Err(err)) => {
   150	            return Err(Status::internal(format!(
   151	                "failed to read data plane token: {}",
   152	                err
   153	            )));
   154	        }
   155	        Err(_elapsed) => {
   156	            return Err(Status::deadline_exceeded(format!(
   157	                "data plane token read timed out after {:?}",
   158	                DATA_PLANE_TOKEN_TIMEOUT
   159	            )));
   160	        }
   161	    }
   162	    if token_buf != expected_token {
   163	        log::warn!("push data plane: invalid token");
   164	        return Err(Status::permission_denied("invalid data plane token"));
   165	    }
   166	    receive_stream_into_module(socket, module, start).await
   167	}
   168	
   169	/// The per-socket receive tail shared by the fixed and resizable
   170	/// accept paths (`ue-r2-2` split it out of `handle_data_plane_stream`
   171	/// so an epoch-N socket runs the identical byte path after its
   172	/// stronger handshake).
   173	async fn receive_stream_into_module(
   174	    socket: TcpStream,
   175	    module: ModuleConfig,
   176	    start: Instant,
   177	) -> Result<TransferStats, Status> {
   178	    eprintln!(
   179	        "blitd: push data plane: token accepted (module='{}', root={})",
   180	        module.name,
   181	        module.path.display()
   182	    );
   183	
   184	    // Route the inbound wire through the unified receive pipeline:
   185	    //   socket → StallGuard → execute_receive_pipeline → FsTransferSink → disk
   186	    // Same call shape as the client's pull-receive side. Tar shards get
   187	    // extracted inline by FsTransferSink (parallelism across streams
   188	    // already comes from N concurrent invocations of this function).
   189	    //
   190	    // audit-h3a (R2/R3 finding H3): symmetric to the audit-1c CLI
   191	    // pull-receive guard. Before this slice the push-receive socket had
   192	    // no idle deadline at all — a hostile or wedged push client that
   193	    // accepted the data plane, sent the token, then went silent would
   194	    // pin this worker indefinitely (DATA_PLANE_TOKEN_TIMEOUT above only
   195	    // bounds the token read). StallGuard turns that into a clean
   196	    // TimedOut after TRANSFER_STALL_TIMEOUT of no progress.
   197	    use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink};
   198	
   199	    let config = FsSinkConfig {
   200	        preserve_times: true,
   201	        dry_run: false,
   202	        checksum: None,
   203	        resume: false,
   204	        compare_mode: blit_core::generated::ComparisonMode::SizeMtime,
   205	    };
   206	    let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
   207	        PathBuf::new(),
   208	        module.path.clone(),
   209	        config,
   210	    ));
   211	    let outcome = receive_push_data_plane(socket, sink)
   212	        .await
   213	        .map_err(|err| Status::internal(format!("data plane receive: {err:#}")))?;
   214	
   215	    let stats = TransferStats {
   216	        files_transferred: outcome.files_written as u64,
   217	        bytes_transferred: outcome.bytes_written,
   218	        bytes_zero_copy: 0,
   219	    };
   220	
   221	    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
   222	    let gbps = (stats.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
   223	    eprintln!(
   224	        "blitd: push data plane: stream complete: files={}, bytes={} ({:.2} Gbps)",
   225	        stats.files_transferred, stats.bytes_transferred, gbps
   226	    );
   227	    Ok(stats)
   228	}
   229	
   230	// ── ue-r2-2: resizable accept (mid-transfer stream ADD) ──────────────
   231	
   232	/// A control-loop → acceptor registration: the credential the next
   233	/// epoch-N socket must present. Sent BEFORE the daemon acks the ADD,
   234	/// so the accept is armed by the time the client dials.
   235	pub(crate) struct ResizeArm {
   236	    pub(crate) epoch: u32,
   237	    pub(crate) sub_token: Vec<u8>,
   238	}
   239	
   240	struct ArmedEpoch {
   241	    epoch: u32,
   242	    sub_token: Vec<u8>,
   243	    expires: tokio::time::Instant,
   244	}
   245	
   246	/// How long an armed ADD epoch waits for its socket. The client dials
   247	/// immediately after the ack, so an older slot is a failed or
   248	/// abandoned dial. Expiry is NON-fatal: the offer lapses and the
   249	/// transfer continues at its current width (the client settled its
   250	/// side when the dial failed).
   251	const RESIZE_ARM_TTL: Duration = DATA_PLANE_ACCEPT_TIMEOUT;
   252	
   253	/// What a resizable-path socket must present after the one-time token.
   254	enum StreamCredential {
   255	    /// Epoch-0 socket: the negotiation's fixed sub-token; failures are
   256	    /// transfer-fatal, exactly like the fixed path's initial accepts.
   257	    Epoch0(Vec<u8>),
   258	    /// Epoch-N socket: consume a live armed entry. Failures drop the
   259	    /// socket WITHOUT failing the transfer — the accept was an
   260	    /// optional capacity offer, and a stray or hostile dial must not

exec
/usr/bin/zsh -lc "rg -n \"accept_data_connection_stream_resizable|ResizeArm|DATA_PLANE_ACCEPT_TIMEOUT|sub_token|arm\" crates/blit-daemon/src/service/push/data_plane.rs crates/blit-core/src/remote/transfer -g'*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/push/data_plane.rs:12:    configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
crates/blit-daemon/src/service/push/data_plane.rs:78:    // Mirrors `accept_data_connection_stream_resizable`, which fixed
crates/blit-daemon/src/service/push/data_plane.rs:84:            match tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await {
crates/blit-daemon/src/service/push/data_plane.rs:95:                        DATA_PLANE_ACCEPT_TIMEOUT,
crates/blit-daemon/src/service/push/data_plane.rs:234:/// so the accept is armed by the time the client dials.
crates/blit-daemon/src/service/push/data_plane.rs:235:pub(crate) struct ResizeArm {
crates/blit-daemon/src/service/push/data_plane.rs:237:    pub(crate) sub_token: Vec<u8>,
crates/blit-daemon/src/service/push/data_plane.rs:242:    sub_token: Vec<u8>,
crates/blit-daemon/src/service/push/data_plane.rs:246:/// How long an armed ADD epoch waits for its socket. The client dials
crates/blit-daemon/src/service/push/data_plane.rs:251:const RESIZE_ARM_TTL: Duration = DATA_PLANE_ACCEPT_TIMEOUT;
crates/blit-daemon/src/service/push/data_plane.rs:258:    /// Epoch-N socket: consume a live armed entry. Failures drop the
crates/blit-daemon/src/service/push/data_plane.rs:269:/// transfer but only accepts while a live armed slot exists — an
crates/blit-daemon/src/service/push/data_plane.rs:270:/// unarmed listener leaves stray dials in the OS backlog, so the
crates/blit-daemon/src/service/push/data_plane.rs:274:pub(crate) async fn accept_data_connection_stream_resizable(
crates/blit-daemon/src/service/push/data_plane.rs:277:    epoch0_sub_token: Vec<u8>,
crates/blit-daemon/src/service/push/data_plane.rs:280:    mut arm_rx: tokio::sync::mpsc::UnboundedReceiver<ResizeArm>,
crates/blit-daemon/src/service/push/data_plane.rs:293:            match tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await {
crates/blit-daemon/src/service/push/data_plane.rs:304:                        DATA_PLANE_ACCEPT_TIMEOUT,
crates/blit-daemon/src/service/push/data_plane.rs:318:        let sub = epoch0_sub_token.clone();
crates/blit-daemon/src/service/push/data_plane.rs:325:    let armed: Arc<std::sync::Mutex<Vec<ArmedEpoch>>> = Arc::default();
crates/blit-daemon/src/service/push/data_plane.rs:326:    let mut arm_open = true;
crates/blit-daemon/src/service/push/data_plane.rs:329:        // Lapse expired offers so `has_armed` (the accept gate) is
crates/blit-daemon/src/service/push/data_plane.rs:332:        let (has_armed, earliest_expiry) = {
crates/blit-daemon/src/service/push/data_plane.rs:333:            let mut slots = armed.lock().expect("armed registry poisoned");
crates/blit-daemon/src/service/push/data_plane.rs:361:            arm = arm_rx.recv(), if arm_open => match arm {
crates/blit-daemon/src/service/push/data_plane.rs:362:                Some(arm) => {
crates/blit-daemon/src/service/push/data_plane.rs:363:                    armed.lock().expect("armed registry poisoned").push(ArmedEpoch {
crates/blit-daemon/src/service/push/data_plane.rs:364:                        epoch: arm.epoch,
crates/blit-daemon/src/service/push/data_plane.rs:365:                        sub_token: arm.sub_token,
crates/blit-daemon/src/service/push/data_plane.rs:369:                None => arm_open = false,
crates/blit-daemon/src/service/push/data_plane.rs:375:                tokio::time::sleep_until(earliest_expiry.expect("gated on has_armed")).await
crates/blit-daemon/src/service/push/data_plane.rs:377:            accepted = listener.accept(), if has_armed => match accepted {
crates/blit-daemon/src/service/push/data_plane.rs:385:                        let registry = Arc::clone(&armed);
crates/blit-daemon/src/service/push/data_plane.rs:466:            let mut slots = registry.lock().expect("armed registry poisoned");
crates/blit-daemon/src/service/push/data_plane.rs:470:                .position(|slot| slot.sub_token == sub && slot.expires > now)
crates/blit-daemon/src/service/push/data_plane.rs:846:            epoch0_sub_token: Vec::new(),
crates/blit-daemon/src/service/push/data_plane.rs:1099:    /// differ. The failure arm (`Status::Internal`) is unreachable
crates/blit-core/src/remote/transfer/stall_guard.rs:12://! it is an **idle** timeout (re-armed on every read that makes progress)
crates/blit-core/src/remote/transfer/stall_guard.rs:31://!   already bounded by the shared `DATA_PLANE_ACCEPT_TIMEOUT` /
crates/blit-core/src/remote/transfer/stall_guard.rs:66:///   the shared `DATA_PLANE_ACCEPT_TIMEOUT` / `DATA_PLANE_TOKEN_TIMEOUT`
crates/blit-core/src/remote/transfer/stall_guard.rs:74:/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on
crates/blit-core/src/remote/transfer/stall_guard.rs:103:                // that's progress, so re-arm the idle deadline.
crates/blit-core/src/remote/transfer/stall_guard.rs:125:/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on every
crates/blit-core/src/remote/transfer/stall_guard.rs:176:                // Pending arm below trips. (h3b round 2: GPT review
crates/blit-core/src/remote/transfer/socket.rs:35:pub const DATA_PLANE_ACCEPT_TIMEOUT: Duration = Duration::from_secs(30);
crates/blit-core/src/remote/transfer/socket.rs:48:/// (an armed resize slot, a stream waiting for work while siblings
crates/blit-core/src/remote/transfer/socket.rs:109:/// bounded by [`DATA_PLANE_ACCEPT_TIMEOUT`] (the audit-2 wave bounded
crates/blit-core/src/remote/transfer/socket.rs:132:        DATA_PLANE_ACCEPT_TIMEOUT,
crates/blit-core/src/remote/transfer/socket.rs:301:    /// portably; the stalled write exercises the same timeout arm).
crates/blit-core/src/remote/transfer/socket.rs:341:    /// black-hole it (timeout arm — assert the TimedOut chain), some
crates/blit-core/src/remote/transfer/socket.rs:362:        // Only the black-hole arm produces our TimedOut shape; a fast
crates/blit-core/src/remote/transfer/pipeline.rs:304:            // ready Add must be processed before the join arm can
crates/blit-core/src/remote/transfer/mod.rs:18:    generate_sub_token, receive_stream_double_buffered, DataPlaneSession, CONTROL_PLANE_CHUNK_SIZE,
crates/blit-core/src/remote/transfer/mod.rs:40:pub use socket::{configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT};
crates/blit-core/src/remote/transfer/data_plane.rs:24:/// (`DataTransferNegotiation.epoch0_sub_token` for the initial
crates/blit-core/src/remote/transfer/data_plane.rs:25:/// sockets, `DataPlaneResize.sub_token` for an ADD epoch's socket).
crates/blit-core/src/remote/transfer/data_plane.rs:31:pub fn generate_sub_token() -> eyre::Result<Vec<u8>> {
crates/blit-core/src/remote/transfer/sink.rs:456:    // R47-F1: the FsTransferSink::write_payload arm for
crates/blit-core/src/remote/transfer/sink.rs:2076:    /// R47-F1 regression: the `write_payload` arm for
crates/blit-core/src/remote/transfer/sink.rs:2134:    /// R47-F1 regression: the `write_payload` arm for

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/data_plane.rs | sed -n '260,460p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   260	    /// optional capacity offer, and a stray or hostile dial must not
   261	    /// kill a healthy stream set.
   262	    Armed(Arc<std::sync::Mutex<Vec<ArmedEpoch>>>),
   263	}
   264	
   265	/// `ue-r2-2`: the resize-enabled variant of
   266	/// [`accept_data_connection_stream`]. Epoch 0 behaves exactly like the
   267	/// fixed path (bounded sequential accepts, parallel handshakes,
   268	/// failures fatal); afterwards the listener stays alive for the whole
   269	/// transfer but only accepts while a live armed slot exists — an
   270	/// unarmed listener leaves stray dials in the OS backlog, so the
   271	/// 1g-era "accept phase is bounded" reasoning keeps holding in spirit
   272	/// (every accept is credential-gated and TTL-bounded). Ends when every
   273	/// worker — initial and added — has finished.
   274	pub(crate) async fn accept_data_connection_stream_resizable(
   275	    listener: TcpListener,
   276	    expected_token: Vec<u8>,
   277	    epoch0_sub_token: Vec<u8>,
   278	    module: ModuleConfig,
   279	    stream_count: u32,
   280	    mut arm_rx: tokio::sync::mpsc::UnboundedReceiver<ResizeArm>,
   281	) -> Result<TransferStats, Status> {
   282	    let start = Instant::now();
   283	    let streams = stream_count.max(1) as usize;
   284	    // NOTE: unlike the fixed path's bare JoinHandle Vec, dropping a
   285	    // JoinSet aborts every remaining worker — a first-error return no
   286	    // longer detaches the survivors (a strict improvement on the
   287	    // design-2 shape for this path; w4-1 still owns the family).
   288	    let mut join_set: tokio::task::JoinSet<Result<Option<TransferStats>, Status>> =
   289	        tokio::task::JoinSet::new();
   290	
   291	    for idx in 0..streams {
   292	        let (accepted, addr) =
   293	            match tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await {
   294	                Ok(Ok(pair)) => pair,
   295	                Ok(Err(err)) => {
   296	                    return Err(Status::internal(format!(
   297	                        "data plane accept failed: {}",
   298	                        err
   299	                    )));
   300	                }
   301	                Err(_elapsed) => {
   302	                    return Err(Status::deadline_exceeded(format!(
   303	                        "data plane accept timed out after {:?} waiting for stream {}/{}",
   304	                        DATA_PLANE_ACCEPT_TIMEOUT,
   305	                        idx + 1,
   306	                        streams
   307	                    )));
   308	                }
   309	            };
   310	        configure_data_socket(&accepted, None)
   311	            .map_err(|err| Status::internal(format!("configuring data socket: {err}")))?;
   312	        let socket = accepted;
   313	        eprintln!(
   314	            "blitd: push data plane: accepted connection {} from {}",
   315	            idx, addr
   316	        );
   317	        let token = expected_token.clone();
   318	        let sub = epoch0_sub_token.clone();
   319	        let module = module.clone();
   320	        join_set.spawn(async move {
   321	            handle_resizable_stream(socket, token, StreamCredential::Epoch0(sub), module).await
   322	        });
   323	    }
   324	
   325	    let armed: Arc<std::sync::Mutex<Vec<ArmedEpoch>>> = Arc::default();
   326	    let mut arm_open = true;
   327	    let mut total = TransferStats::default();
   328	    loop {
   329	        // Lapse expired offers so `has_armed` (the accept gate) is
   330	        // honest; a socket that raced in anyway is dropped by the
   331	        // worker's consume-time expiry check.
   332	        let (has_armed, earliest_expiry) = {
   333	            let mut slots = armed.lock().expect("armed registry poisoned");
   334	            let now = tokio::time::Instant::now();
   335	            slots.retain(|slot| {
   336	                if slot.expires > now {
   337	                    true
   338	                } else {
   339	                    log::warn!(
   340	                        "push data plane: resize ADD epoch {} expired unclaimed",
   341	                        slot.epoch
   342	                    );
   343	                    false
   344	                }
   345	            });
   346	            (
   347	                !slots.is_empty(),
   348	                slots.iter().map(|slot| slot.expires).min(),
   349	            )
   350	        };
   351	        tokio::select! {
   352	            joined = join_set.join_next() => match joined {
   353	                None => break,
   354	                Some(Ok(Ok(Some(stats)))) => accumulate_transfer_stats(&mut total, &stats),
   355	                Some(Ok(Ok(None))) => {} // dropped epoch-N socket, non-fatal
   356	                Some(Ok(Err(status))) => return Err(status),
   357	                Some(Err(_)) => {
   358	                    return Err(Status::internal("data plane worker cancelled"));
   359	                }
   360	            },
   361	            arm = arm_rx.recv(), if arm_open => match arm {
   362	                Some(arm) => {
   363	                    armed.lock().expect("armed registry poisoned").push(ArmedEpoch {
   364	                        epoch: arm.epoch,
   365	                        sub_token: arm.sub_token,
   366	                        expires: tokio::time::Instant::now() + RESIZE_ARM_TTL,
   367	                    });
   368	                }
   369	                None => arm_open = false,
   370	            },
   371	            // ue-r2-2 review (panel F4): wake at the earliest slot
   372	            // expiry so a lapsed offer closes the accept gate on time
   373	            // instead of at the next unrelated event.
   374	            _ = async {
   375	                tokio::time::sleep_until(earliest_expiry.expect("gated on has_armed")).await
   376	            }, if earliest_expiry.is_some() => {}
   377	            accepted = listener.accept(), if has_armed => match accepted {
   378	                Ok((socket, addr)) => match configure_data_socket(&socket, None) {
   379	                    Ok(()) => {
   380	                        eprintln!(
   381	                            "blitd: push data plane: accepted resize connection from {}",
   382	                            addr
   383	                        );
   384	                        let token = expected_token.clone();
   385	                        let registry = Arc::clone(&armed);
   386	                        let module = module.clone();
   387	                        join_set.spawn(async move {
   388	                            handle_resizable_stream(
   389	                                socket,
   390	                                token,
   391	                                StreamCredential::Armed(registry),
   392	                                module,
   393	                            )
   394	                            .await
   395	                        });
   396	                    }
   397	                    Err(err) => {
   398	                        log::warn!("push data plane: resize socket setup failed: {err}");
   399	                    }
   400	                },
   401	                Err(err) => {
   402	                    log::warn!("push data plane: resize accept failed: {err}");
   403	                }
   404	            },
   405	        }
   406	    }
   407	
   408	    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
   409	    let gbps = (total.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
   410	    eprintln!(
   411	        "blitd: push data plane: aggregate throughput {:.2} Gbps ({} bytes in {:.2}s)",
   412	        gbps, total.bytes_transferred, elapsed
   413	    );
   414	    Ok(total)
   415	}
   416	
   417	/// Per-socket worker on the resizable path: 48-byte handshake
   418	/// (one-time token ‖ sub-token), then the shared receive tail.
   419	/// `Ok(None)` = socket dropped non-fatally (bad epoch-N credential);
   420	/// post-handshake receive errors are always fatal — an authorized
   421	/// live stream dying mid-transfer is data loss on any epoch.
   422	async fn handle_resizable_stream(
   423	    mut socket: TcpStream,
   424	    expected_token: Vec<u8>,
   425	    credential: StreamCredential,
   426	    module: ModuleConfig,
   427	) -> Result<Option<TransferStats>, Status> {
   428	    fn refuse(fatal: bool, status: Status) -> Result<Option<TransferStats>, Status> {
   429	        if fatal {
   430	            Err(status)
   431	        } else {
   432	            log::warn!("push data plane: dropping resize socket: {status}");
   433	            Ok(None)
   434	        }
   435	    }
   436	
   437	    let start = Instant::now();
   438	    let fatal = matches!(credential, StreamCredential::Epoch0(_));
   439	    let mut buf = vec![0u8; expected_token.len() + blit_core::remote::transfer::SUB_TOKEN_LEN];
   440	    match tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await {
   441	        Ok(Ok(_)) => {}
   442	        Ok(Err(err)) => {
   443	            return refuse(
   444	                fatal,
   445	                Status::internal(format!("failed to read data plane token: {}", err)),
   446	            );
   447	        }
   448	        Err(_elapsed) => {
   449	            return refuse(
   450	                fatal,
   451	                Status::deadline_exceeded(format!(
   452	                    "data plane token read timed out after {:?}",
   453	                    DATA_PLANE_TOKEN_TIMEOUT
   454	                )),
   455	            );
   456	        }
   457	    }
   458	    let (token, sub) = buf.split_at(expected_token.len());
   459	    if token != &expected_token[..] {
   460	        log::warn!("push data plane: invalid token");

codex
The late-ADD path is bounded daemon-side by the resize arm TTL and does not look like a StallGuard/cancellation regression. I’m doing one more pass for API/reporting and mixed-version surface before I write the verdict.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '250,330p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   250	            sink,
   251	            work_rx.clone(),
   252	            source.clone(),
   253	            progress.cloned(),
   254	            total.clone(),
   255	            cancelled.clone(),
   256	            retire_rx,
   257	        );
   258	    }
   259	
   260	    // Forwarder: move payloads from the incoming channel onto the shared
   261	    // work queue. `send_async` applies back-pressure (bounded queue); if
   262	    // every worker has gone away (e.g. all sinks errored) the send fails
   263	    // and we stop. It also bails as soon as a worker sets `cancelled`, so
   264	    // a single sink error halts intake promptly instead of waiting for
   265	    // every worker to drop. Dropping `work_tx` on end-of-stream (or on
   266	    // cancel) signals the workers. (The executor keeps a `work_rx` clone
   267	    // for late-added workers — flume disconnect is sender-driven, so the
   268	    // retained receiver does not keep the queue alive.)
   269	    let cancelled_fwd = cancelled.clone();
   270	    let forwarder = tokio::spawn(async move {
   271	        while let Some(payload) = payload_rx.recv().await {
   272	            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
   273	                // A worker errored — stop draining the producer and let
   274	                // the queue close so survivors finish and the error
   275	                // surfaces without delay.
   276	                return;
   277	            }
   278	            if work_tx.send_async(payload).await.is_err() {
   279	                // All workers dropped their receivers — nothing left to
   280	                // feed; treat as shutdown.
   281	                return;
   282	            }
   283	        }
   284	        // Dropping work_tx closes the queue → workers see Disconnected
   285	        // after draining and run finish().
   286	    });
   287	
   288	    // Supervise: join workers (first error wins) while servicing the
   289	    // resize control channel. `join_next() == None` means every worker
   290	    // — initial and added — has finished, which only happens once the
   291	    // queue closed and drained (or errored/retired), so control is
   292	    // moot beyond that point.
   293	    let mut control_rx = control_rx;
   294	    let mut first_err: Option<eyre::Report> = None;
   295	    loop {
   296	        let control_recv = async {
   297	            match control_rx.as_mut() {
   298	                Some(rx) => rx.recv().await,
   299	                None => std::future::pending().await,
   300	            }
   301	        };
   302	        tokio::select! {
   303	            // ue-r2-2 review (panel F2): biased, control FIRST — a
   304	            // ready Add must be processed before the join arm can
   305	            // observe an empty set and break, or an already-authorized
   306	            // socket would drop without its END record (fatal on the
   307	            // peer). Processing a control command is always cheap and
   308	            // never starves joins.
   309	            biased;
   310	
   311	            cmd = control_recv => {
   312	                match cmd {
   313	                    Some(SinkControl::Add(sink)) => {
   314	                        if !cancelled.load(Ordering::Relaxed) {
   315	                            let (retire_tx, retire_rx) = tokio::sync::watch::channel(false);
   316	                            let slot = next_slot;
   317	                            next_slot += 1;
   318	                            retire_flags.push((slot, retire_tx));
   319	                            spawn_sink_worker(
   320	                                &mut join_set,
   321	                                slot,
   322	                                sink,
   323	                                work_rx.clone(),
   324	                                source.clone(),
   325	                                progress.cloned(),
   326	                                total.clone(),
   327	                                cancelled.clone(),
   328	                                retire_rx,
   329	                            );
   330	                        }

exec
/usr/bin/zsh -lc "rg -n \"RemotePushReport \\{|data_plane_streams|files_requested:|first_payload_elapsed:\" crates/blit-core crates/blit-cli crates/blit-app crates/blit-tui -g'*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/engine/streaming_plan.rs:110:    pub first_payload_elapsed: Option<Duration>,
crates/blit-core/src/remote/pull.rs:331:            receive_data_plane_streams_owned(
crates/blit-core/src/remote/pull.rs:1672:async fn receive_data_plane_streams_owned(
crates/blit-core/src/remote/pull.rs:2034:    //! `receive_data_plane_streams_owned` — the machinery the PullSync
crates/blit-core/src/remote/pull.rs:2047:    use super::receive_data_plane_streams_owned;
crates/blit-core/src/remote/pull.rs:2093:            receive_data_plane_streams_owned(
crates/blit-core/src/remote/pull.rs:2202:        let guard = super::AbortOnDrop::new(tokio::spawn(receive_data_plane_streams_owned(
crates/blit-core/src/remote/pull.rs:2252:            receive_data_plane_streams_owned(
crates/blit-core/src/remote/push/client/mod.rs:632:        let mut first_payload_elapsed: Option<Duration> = None;
crates/blit-core/src/remote/push/client/mod.rs:708:        let mut files_requested: Vec<String> = Vec::new();
crates/blit-core/src/remote/push/client/mod.rs:1511:        Ok(RemotePushReport {
crates/blit-core/src/remote/push/client/mod.rs:1517:            data_plane_streams: match (&dial, data_port) {
crates/blit-core/src/remote/push/client/types.rs:6:pub struct RemotePushReport {
crates/blit-core/src/remote/push/client/types.rs:7:    pub files_requested: Vec<String>,
crates/blit-core/src/remote/push/client/types.rs:11:    pub first_payload_elapsed: Option<Duration>,
crates/blit-core/src/remote/push/client/types.rs:16:    pub data_plane_streams: Option<usize>,

exec
/usr/bin/zsh -lc 'git status --short' in /home/michael/dev/Blit
 succeeded in 0ms:
?? .review/results/sf-2-shape-correction-resize.codex.md

exec
/usr/bin/zsh -lc "git show --name-only --format='%H%n%P%n%s' c70c2ac" in /home/michael/dev/Blit
 succeeded in 0ms:
c70c2ac64ebbd9ffb45633e8cc3a8e6075e272c5
84cd6bf3ae5dce78663707c3756339a948e027c3
sf-2: shape-correction stream resize for many-file pushes

.review/findings/sf-2-shape-correction-resize.md
crates/blit-core/src/engine/dial.rs
crates/blit-core/src/remote/push/client/mod.rs
crates/blit-core/src/remote/push/client/types.rs
crates/blit-daemon/src/service/push/mod.rs
crates/blit-daemon/src/service/push/shape_resize_e2e.rs

codex
- `crates/blit-core/src/remote/push/client/mod.rs:868` — **Medium** — Shape correction uses `requested_files.len()` as `need_count`, but `requested_files` is the pruning/outstanding set and is drained after payloads are queued. That is not the accumulated need-list count described by sf-2, so file-count tiers can be under-proposed after early batches drain. A 10k tiny push split as 128 + 9872 can pass the new `>1` e2e while missing the table’s 10k→8 target.

VERDICT: NEEDS FIXES. Static diff shows +4 tests and no removed test annotations, consistent with 1479→1483; I did not rerun the suite in the read-only sandbox.
tokens used
113,633
- `crates/blit-core/src/remote/push/client/mod.rs:868` — **Medium** — Shape correction uses `requested_files.len()` as `need_count`, but `requested_files` is the pruning/outstanding set and is drained after payloads are queued. That is not the accumulated need-list count described by sf-2, so file-count tiers can be under-proposed after early batches drain. A 10k tiny push split as 128 + 9872 can pass the new `>1` e2e while missing the table’s 10k→8 target.

VERDICT: NEEDS FIXES. Static diff shows +4 tests and no removed test annotations, consistent with 1479→1483; I did not rerun the suite in the read-only sandbox.
