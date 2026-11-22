# cr-ecm21-1: a wire resume with no block bypassed its hold, and a keep-alive acquired the partial

**Severity**: HIGH — a resume whose blocks all matched cuts and stamps whatever stands at the name, and fails on an owner's read-only file; an empty keep-alive changes the file's mode and length before any byte comes
**Status**: Verified on Unix — `combined-r23` (Codex openreview, 2026-11-22) reviewed `5d2f8def..24d4064f` — the six `combined-r22` fixes — with `guard_confirmed: true`: "Checking the file version before and after hashing, ignoring empty blocks for acquisition, retaining the Unix write handle, and distinguishing volume failures from per-file failures are sound choices." Its Windows completion is cr-ecm22-3's
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `fe38ecd4`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r22` over `b92c9a17..5d2f8def` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r22.codex.md`. It completes cr-ecm20-1 (`5b3bf24e`).

## Evidence
"When every block matches, both wire carriers can go straight to completion. The sink then has no held identity and can truncate and stamp a replacement. In an independent Linux run with identical bytes, an older destination timestamp, and an owner-controlled `0444` destination, this path reported `Permission denied`, zero resumed, and one failed. An empty TCP keepalive has the converse problem: it can change the file's mode and length before any changed bytes arrive. Acquire and verify the writer on the first data-bearing block or successful completion. Keep liveness records read-only."

## Intake
Admitted as the reviewer put it.

## What
- The sink's completion has its partial first (`hold_resume_partial`, idempotent): a completion no block came before goes by the hashes' word as a block would — the file hashed, unchanged, made writable and private through what holds it — and only then is cut and stamped.
- A block that carries nothing has nothing: the sink writes and makes nothing for it, the data plane's sink asks no hold for it, and the control lane holds the partial at the first block that carries bytes (it held it before the first block of any kind).

## Guard proof
- `sink::…::a_resume_with_no_block_has_its_partial_and_an_empty_block_has_nothing`: a completion alone for an owner's 444 file whose bytes all matched — it lands, with the source's mode; for a file renamed over the name after hashing — it fails, the other file whole; an empty block for a missing file — nothing is made.
- `data_plane::tests::an_empty_block_holds_no_partial`: through the data plane's sink, an empty block asks no hold; one with bytes asks one.
- Mutations, each restored byte for byte: a completion with no block going straight to the name (red); an empty block written (red); an empty block on the data plane holding the partial (red).
- The sink, data-plane and session tests (278) pass. Windows cross-clippy clean.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — the block and completion payloads; guard.
- `crates/blit-core/src/transfer_session/data_plane.rs` — `NeedListSink`'s block; guard.
- `crates/blit-core/src/transfer_session/mod.rs` — `receive_block_record` holds at the first block with bytes.

## Known gaps
- The control lane's move of the hold has no guard of its own: a record whose first block is empty is not made by any test transport here; the sink's own refusal to write or make anything for an empty block stands behind it.
