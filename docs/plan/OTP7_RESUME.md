# otp-7 — resume block phase (design)

**Status**: Active (owner Q1–Q3 answered + "confirmed", 2026-07-09; D-2026-07-09-1)
**Created**: 2026-07-07
**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-7.
**Contract**: `docs/TRANSFER_SESSION.md` (resume exception + frame table, pinned otp-1).
**Governs**: implementation proceeds 7a → 7b, one slice per codex loop pass
(D-2026-07-04-1). Owner's deciding principle, quoted: "FAST, SIMPLE, RELIABLE
file transfer. if we abort the whole thing when we could have fixed or
surfaced a single error, we are violating all of those."

## Why this doc

otp-7 is the plan's **explicit RELIABLE exception**: resumed files use a
strictly-ordered block-hash exchange, and the choreography is novel (unlike the
mechanical carrier splits of otp-4b/5b/6). The owner asked for the design on paper
before the intricate code. This doc records the choreography, the reuse map, the
design decisions (most already settled by the contract), the staging, and the
guard-proof targets — so implementation is a transcription, not a discovery.

## What resume is (contract, already pinned in otp-1)

A `NeedEntry` may be flagged `resume=true`. For such a file the DESTINATION sends
its `BlockHashList` (Blake3 per block of the existing partial) and the SOURCE
**must not send any byte of that file until it has received that list**. The SOURCE
then transfers only the blocks whose hashes differ (or that the dest lacks), as
`BlockTransfer` records, ending with `BlockTransferComplete{total_bytes}`. Stale or
mismatched partials fall back to full-file transfer.

Frames (field numbers frozen, `TRANSFER_SESSION.md`): `8 BlockHashList` (DEST),
`14 BlockTransfer` (SOURCE), `15 BlockTransferComplete` (SOURCE). `SessionOpen.resume`
carries `ResumeSettings{enabled, block_size}`. `NeedEntry.resume` is field 2.

## What already exists (reused verbatim — no reinvention)

- **Wire frames + payload enums**: `BlockHashList`/`BlockTransfer`/`BlockTransferComplete`
  frames and `PreparedPayload::{FileBlock, FileBlockComplete}` are defined and
  name-mapped in the session (`transfer_session/mod.rs:250,256,257`).
- **DEST apply (reassembly)**: `FsTransferSink::write_file_block_payload`
  (`sink.rs:641`, seek+write into the partial in place) and
  `write_file_block_complete` (`sink.rs:687`, `set_len` + fsync + stamp mtime/perms).
  In-place patch of the partial — the partial IS the destination; no temp+rename
  (matches the old pull client).
- **DEST block hashing**: `compute_block_hashes` (`remote/pull.rs:1139`) — streams
  the partial in `block_size` chunks, `blake3::hash`, returns 32-byte digests; an
  absent file returns an empty vec (the implicit full-file fallback).
- **Block-diff reference**: `resume_copy_file` (`copy/file_copy/resume.rs:52`) is the
  canonical block-compare (write a block iff beyond dst len, a partial tail, or
  hashes differ; truncate if dst longer). The SOURCE-side diff is the same logic.
- **Defaults**: `DEFAULT_BLOCK_SIZE` = 1 MiB, `MAX_BLOCK_SIZE` = 64 MiB
  (`copy/file_copy/resume.rs:16,19`). `ResumeSettings.block_size == 0` ⇒ default.

## What is new (the otp-7 work)

1. **Un-stub the four refusal sites**: both open validators (`mod.rs:362,401`), the
   source recv-half resume-need rejection (`mod.rs:799`), and the outbound-planner
   FileBlock bail (`mod.rs:1446`).
2. **The strict-ordering exchange choreography** in the session's source/dest halves.
3. **A home for the SOURCE-side block-diff** — today hand-rolled in `pull_sync.rs`,
   not on any trait (see Design decision D3).

## Choreography (strict ordering)

```
DESTINATION (diff loop)                     SOURCE (send half)
─────────────────────────                   ──────────────────
for each manifest entry:
  if resume-eligible (see D2):
     NeedEntry{path, resume=true} ───────►  recv: ResumeNeed(header)
     BlockHashList{path, bsz, hashes} ───►  recv: BlockHashes(path, hashes)
                                            (send half correlates the two;
                                             a resume need is HELD until its
                                             BlockHashList arrives — the
                                             RELIABLE ordering guarantee)
  else:
     NeedEntry{path, resume=false} ──────►  recv: Need(header)  (unchanged)

                                            for a held resume need + its hashes:
                                              read source file block-by-block,
                                              blake3 each; for block i where
                                              i >= hashes.len() OR hash != hashes[i]:
  recv BlockTransfer{path,off,bytes} ◄────    send BlockTransfer{path, off, block}
     sink.write_file_block_payload            (in-stream carrier: control-lane
                                               frames; data-plane: send_block, 7b)
  recv BlockTransferComplete{path,total} ◄─  send BlockTransferComplete{path,total}
     sink.write_file_block_complete
     files_resumed += 1
```

The source's per-file byte phase for a resume need is "send changed blocks then
complete", replacing the whole-file record it sends for a non-resume need. Ordering
is enforced on the SOURCE: it will not emit a block for a path before it holds that
path's `BlockHashList` (fail-fast if a block phase would start without one).

## Design decisions

- **D1 — stale/mismatched partial ⇒ graceful full-file fallback**, per the contract
  (`TRANSFER_SESSION.md:84`), NOT the hard `Status::internal` the old *data-plane*
  path uses (`pull_sync.rs:1377`) — that is a pre-cutover quirk the gRPC path already
  contradicts (`pull_sync.rs:1544`, graceful). An empty / short / all-mismatched
  hash list simply means "send all blocks" = full transfer. **Reconcile in favor of
  the contract.**
- **D2 — resume eligibility** (which needs get `resume=true`): the file exists at the
  dest as a non-empty partial AND `ResumeSettings.enabled` AND the compare says the
  file must transfer (changed). A missing/empty dest file is a normal full transfer
  (no resume flag, no BlockHashList). This mirrors the daemon's `effective_resume`
  set (`pull_sync.rs:262`) minus the mtime-only-touch special case, which the session
  already handles via SizeMtime skip.
- **D3 — SOURCE block-diff home**: a free helper in the session
  (`resume_block_diff(source, header, dest_hashes, block_size) -> stream of blocks`)
  rather than a new `TransferSource` trait method. Rationale: it needs only
  `source.open_file(header)` (already on the trait) + blake3, and keeping it out of
  the trait avoids every future `TransferSource` impl re-implementing it (the same
  reasoning that made `FilteredSource` the one filter chokepoint in otp-6a). Flag for
  codex: confirm the helper doesn't belong on the trait.
- **D4 — mid-resume-failure**: block writes patch the partial in place (no
  temp+rename, matching the old client). A fault mid-block-transfer surfaces as a
  `SessionFault` (peer-notified) and aborts **the session** — the same
  whole-session failure model every session payload record already has (a
  file record hitting EOF-short aborts the session today, `mod.rs`
  `send_payload_records`); resume adds no new abort semantics. Per-file
  continue-on-error (skip the bad file, keep transferring) is NOT in otp-7 —
  it would be a session-wide failure-model change needing its own owner
  decision (`docs/plan/LOCAL_ERROR_TELEMETRY.md` is the adjacent Draft).
  After the abort the partial is left partially patched, and the NEXT resume
  re-syncs via a fresh block-hash exchange (the partial's new hashes reflect
  whatever landed). The pin asserts the fault surfaces cleanly and no file is
  falsely counted `files_resumed`. (No stronger atomicity than the code we
  are replacing — called out as a Known gap, not a regression.)
  **Owner rider (2026-07-09, Q2)**: the fault must also appear in the CLI's
  **end-of-operation summary** — naming the affected file(s) and suggesting a
  re-run to converge — not only as a mid-stream line that scrolls away. Small
  CLI-layer deliverable, lands within otp-7 (the session already collects the
  per-file fault; this is about where it is reported). The full progress-display
  redesign it brushes against is a separate queued item (TODO.md "CLI transfer
  output redesign") and is NOT in otp-7 scope.
- **D5 — block size**: the DEST chooses (it hashes first); the SOURCE reads the size
  from the `BlockHashList`, so the two never disagree. `0` ⇒ `DEFAULT_BLOCK_SIZE`.
  **Amended by D-2026-07-10-1 (codex 7a F1)** — the original "clamp to
  `MAX_BLOCK_SIZE` (64 MiB)" was unsafe on the in-stream carrier: it rides the gRPC
  `Transfer` RPC when the daemon serves, where tonic's default 4 MiB frame limit
  applies. The DEST clamps into **[64 KiB, 2 MiB]** (`MIN_RESUME_BLOCK_SIZE`,
  `MAX_IN_STREAM_RESUME_BLOCK_SIZE`) — the floor kills the 32×-amplified hash lists
  of absurdly small blocks, the ceiling keeps one-block `BlockTransfer` frames under
  the limit — and caps any one `BlockHashList` at **65_536 hashes** (2 MiB); a
  partial with more blocks degrades to the empty list = the D1 full-transfer
  fallback. The SOURCE range-validates the wire value at arrival (same-build peers:
  a mismatch is a violation, not a negotiation). otp-7b revisits the ceiling for the
  data plane, whose binary block records have no protobuf envelope.
- **D6 — invariance**: resume runs identically whichever end initiated (the flag is
  in the open; the DEST computes hashes and applies; the SOURCE diffs and sends). The
  role suite runs both initiator assignments, as for every prior slice.

## Staging

- **otp-7a — resume over the in-stream carrier.** Fully exercisable in
  `transfer_session_roles.rs` (both initiator roles, in-stream). Un-stub the
  refusals; implement the choreography + block-diff helper + DEST hash-send + apply
  wiring over the control-lane `BlockTransfer`/`Complete` frames; `files_resumed`.
  Pins: happy-path partial, identical-file (zero blocks), stale-partial fallback,
  mid-resume-failure.
- **otp-7b — resume over the TCP data plane, plus the D4 owner rider.** Port the
  block records onto the data plane (`data_plane.rs::send_block`/
  `send_block_complete` already exist) with the same choreography; e2e in the
  daemon harness. Follows 7a exactly as otp-4b-1→4b-2 and otp-5b-1→5b-2 did.
  **7b also owns the CLI end-of-operation fault summary** (D-2026-07-09-1, Q2
  rider): a session fault reported by the CLI transfer commands must name the
  affected file(s) at the END of the operation output and suggest a re-run to
  converge, with a test pinning that the failed path appears in the final
  output. 7a cannot own it — 7a's surface is the in-process roles suite,
  which has no CLI layer. Carried into 7b from the 7a codex review:
  revisit the D-2026-07-10-1 block-size ceiling for the data-plane carrier
  (binary records, no protobuf envelope), and exercise cancel-during-resume
  in the daemon e2e harness (codex 7a F4 — the in-stream hash/block phases
  inherit the session's existing payload-phase cancel latency; the e2e pins
  that a `CancelJob` mid-resume tears down cleanly, as otp-4b-3 pinned for
  file records).

### 7b implementation map (surveyed 2026-07-10, before any 7b code)

Recorded so the implementing session starts from facts, not re-exploration:

- **Receive side is ALREADY DONE.** `remote/transfer/pipeline.rs::
  execute_receive_pipeline` (~:417) decodes the binary `BLOCK` (=2) and
  `BLOCK_COMPLETE` (=3) record tags (`remote/transfer/data_plane.rs:16-20`),
  enforces `MAX_WIRE_BLOCK_BYTES`, and dispatches
  `PreparedPayload::FileBlock{,Complete}` to `sink.write_payload` — which
  `FsTransferSink` already applies. No new decoding needed.
- **Senders exist but are unreached**: `DataPlaneSession::send_block` (~:536)
  and `send_block_complete` (~:592) in `remote/transfer/data_plane.rs`; the
  block-complete record carries mtime+perms, so zero-block completes stamp
  metadata (the wire `BlockTransferComplete` frame does not — the SOURCE has
  the manifest header and must supply them).
- **Un-stub sites (session)**: grant suppression `transfer_session/mod.rs`
  (`open.in_stream_bytes || resume_negotiated(&open)`); the send-loop bail
  ("resume block records ride the in-stream carrier until otp-7b");
  `NeedListSink::write_payload`'s FileBlock rejection
  (`transfer_session/data_plane.rs` ~:970, unit test ~:1030) — on the data
  plane the resume claim + `files_resumed` count must happen HERE (the
  control loop never sees block records), so `resume_headers`/the resumed
  counter need to be shared with the receive path like `outstanding` is;
  `DataPlaneSink::write_payload`'s relay rejection (`sink.rs` ~:799) routes
  to `send_block`/`send_block_complete` instead.
- **Source-side flow wrinkle**: `SourceDataPlane::queue` takes
  `Vec<TransferPayload>` and the pipeline calls `source.prepare_payload`,
  which BAILS on FileBlock ("cannot be prepared from a filesystem source",
  `payload.rs:61`) — the block payloads must either bypass the prepare stage
  or the session's block-diff must run inside the pipeline path; the 7a
  in-stream block-diff (`send_resume_block_records`) is the logic to share.
- **Session client**: `PushSessionOptions`/`PullSessionOptions`
  (`session_client.rs:40/:119`) have NO resume field and never set
  `SessionOpen.resume` — 7b wiring, needed by any e2e.
- **CLI tension (affects the D4 rider)**: NO CLI verb calls
  `run_push_session`/`run_pull_session` yet — the only callers are the
  daemon e2e/parity tests (`blit-daemon/src/service/transfer_session_e2e.rs`);
  CLI verbs ride the OLD paths until the otp-10 cutover. The CLI's `--resume`
  flag (`blit-cli/src/cli.rs:267`) flows only into the old paths. So the
  end-of-op fault summary's CLI print integration cannot be exercised
  through a session-driven CLI verb until otp-10; 7b builds the mechanism at
  the layer that survives cutover (structured file identity on the fault +
  the summary formatting + pins at the session-client/e2e level) and the
  verb-level print lands with the otp-10 verb switch. `SessionFault`
  (`transfer_session/mod.rs` ~:176) has no structured path field today —
  message-only; add one rather than string-scraping.
- **e2e references**: harness `Daemon::start` + the otp-4b-3 cancel e2e
  (`mid_transfer_cancel_surfaces_cancelled_over_the_data_plane`,
  `transfer_session_e2e.rs` ~:294, `StuckAfterFirstChunkSource` shape) is the
  template for cancel-during-resume; old-path A/B references:
  `blit-cli/tests/remote_resume.rs` (real-binary `--resume` pulls) and
  `remote/pull.rs::wire_equivalence_resume_and_filter_and_force_grpc`.

## Guard-proof targets (the plan's mandate: "pins the stale-partial and
mid-resume-failure cases")

1. **Partial resume** — a multi-block file with some blocks already correct at the
   dest: only the changed blocks move (assert BlockTransfer count / bytes), final
   bytes identical, `files_resumed == 1`. Guard: neuter the block-diff so it sends
   all blocks ⇒ the "only changed blocks moved" assertion FAILS.
2. **Identical file** — zero blocks transferred, file untouched, still counted done.
3. **Stale-partial fallback** — a dest partial that shares no blocks with the source
   ⇒ full content lands, bytes identical, no hang/fault. Guard: force the source to
   trust the stale hashes ⇒ corrupt output.
4. **Mid-resume-failure** — inject a source fault mid-block-phase ⇒ a clean
   `SessionFault` surfaces to both ends, `files_resumed` not incremented for the
   aborted file, no deadlock.

## Open questions — RESOLVED (owner, 2026-07-09; D-2026-07-09-1)

- **Q1 — contract wins.** Stale/mismatched partial degrades gracefully to a
  full-file transfer, never an abort. Owner's principle (quoted in the header)
  is the rationale; D1 stands as written.
- **Q2 — keep in-place patch, surface at end of op.** No temp+rename atomicity
  for otp-7 (parity with the code being replaced). The owner's rider: the fault
  is surfaced in the end-of-operation summary with a re-run suggestion — see D4.
  No atomicity follow-up filed; convergence-on-retry is the reliability model.
- **Q3 — 7a then 7b, no collapse.** Owner: "confirmed. no collapse. keep the
  reviewloop codex playbook going slice by slice."

## Verification (when Active)

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo test --workspace` (count must not drop).
- Windows parity after touching `win_fs`/sink paths: `scripts/windows/run-blit-tests.ps1`.
- Each sub-slice through the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`), guard proof
  per pin above.
