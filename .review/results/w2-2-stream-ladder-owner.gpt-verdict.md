# Verdict — w2-2-stream-ladder-owner

Slice commit: `01209bc`. Codex verdict: **NEEDS FIXES** (1 finding,
Low). reviewer: gpt-5.5

## Finding 1 — new ensure_dial comment says "fallback batch" in the data-plane branch (Low)

- Claim: the comment this slice added above the first bare
  `ensure_dial(&mut dial, None)` call reads "Dial exists before the
  first fallback batch (first-wins)", but the surrounding arm is
  `TransferMode::DataPlane` — it plans and queues TCP data-plane
  payloads via `sender.queue`. The later twin site correctly says
  "data-plane batch".
- Adjudication: **Accepted.** Verified against
  `crates/blit-core/src/remote/push/client/mod.rs:775-790` — the arm
  sits under `TransferMode::DataPlane` and feeds the
  `MultiStreamSender`, not the gRPC fallback. The mislabel is on the
  exact invariant (first-need dial creation, first-wins ceilings) the
  restructuring had to preserve, so the comment must not point at the
  wrong path.
- Fix: `27f53a0` — one word, "fallback" → "data-plane", matching the
  twin at the second bare call site.

## Gate

Slice commit `01209bc`: fmt clean; clippy `-D warnings` clean;
`cargo test --workspace` 37 suites, 1452 passed / 0 failed / 2 ignored
(baseline 1448 → +4: the four new `transfer_plan` unit tests; zero
tests deleted). Fix commit `27f53a0` (comment-only): full gate re-run,
identical results.

Test-guard note (per the finding doc): the 4 new tests pin the
batching behavior that survives the restructuring; the deletions
themselves are compile-guarded — reverting the slice makes the test
module (and every updated caller) fail to build, so revert-style
mutation verification degenerates to a compile failure rather than a
red/green flip. Same evidence shape the w2-1 deletion slice was graded
on.
