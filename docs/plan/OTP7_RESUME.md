# otp-7 — resume block phase (design)

**Status**: Draft
**Created**: 2026-07-07
**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-7.
**Contract**: `docs/TRANSFER_SESSION.md` (resume exception + frame table, pinned otp-1).
**Governs**: no code until the owner flips this to `**Status**: Active`
(AGENTS.md; `.agents/repo-guidance.md` plan operator). Per D-2026-07-04-1 this
plan change also goes through the codex loop.

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
  `SessionFault` (peer-notified) and aborts; the partial is left partially patched,
  and the NEXT resume re-syncs via a fresh block-hash exchange (the partial's new
  hashes reflect whatever landed). The pin asserts the fault surfaces cleanly and no
  file is falsely counted `files_resumed`. (No stronger atomicity than the code we
  are replacing — called out as a Known gap, not a regression.)
- **D5 — block size**: `ResumeSettings.block_size` clamped to `MAX_BLOCK_SIZE`, `0` ⇒
  `DEFAULT_BLOCK_SIZE`. The DEST chooses (it hashes first); the SOURCE reads the size
  from the `BlockHashList`, so the two never disagree.
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
- **otp-7b — resume over the TCP data plane.** Port the block records onto the data
  plane (`data_plane.rs::send_block`/`send_block_complete` already exist) with the
  same choreography; e2e in the daemon harness. Follows 7a exactly as otp-4b-1→4b-2
  and otp-5b-1→5b-2 did.

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

## Open questions for the owner

- **Q1**: D1 (graceful stale fallback) reconciles the old data-plane hard-error
  against the contract. Confirm the contract wins (agent rec: yes — it is the pinned
  wire behavior and the safer one).
- **Q2**: D4 keeps the old in-place-patch failure model (no temp+rename atomicity).
  Acceptable as-is for otp-7, or do you want atomic partial-file handling as a
  follow-up item? (agent rec: keep parity now, file a follow-up if wanted.)
- **Q3**: Staging — 7a (in-stream) then 7b (data-plane), per the AskUserQuestion
  answer's default. Confirm, or collapse into one.

## Verification (when Active)

- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo test --workspace` (count must not drop).
- Windows parity after touching `win_fs`/sink paths: `scripts/windows/run-blit-tests.ps1`.
- Each sub-slice through the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`), guard proof
  per pin above.
