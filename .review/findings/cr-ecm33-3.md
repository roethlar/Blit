# cr-ecm33-3: a wire resume whose destination's directory could not be established was still written

**Severity**: HIGH — cr-ecm32-1 let a resume carry no place when the name's directory was missing at its hashing, taking that for a directory not yet made. Every resume hashed is of a file the diff saw, so a missing directory had gone since. If it came back as a link to another directory in the destination, an empty name there received the file. Off Unix the receiver discards even `Unjustified`; that half is cr-ecm32-4's, blocked.
**Status**: Verified — `combined-r35` (Codex openreview, 2026-11-24) reviewed `0f912b06..dc14afc4` — the `combined-r34` fixes and the three prose fixes — with `guard_confirmed: true`: "The ordinary Linux copy now succeeds through a valid destination symlink and retains its writable handle. A Unix resume refuses a write when it can no longer establish the destination directory. The typed endpoint and resume-error classifications also fit the goal. The remaining problems are at identifiable points where the implementation drops a handle, a file identity, or a known cleanup result." Its Windows half stays cr-ecm32-4's, blocked.
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `03bafda2`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r34` over `70efef8a..0f912b06` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r34.codex.md`. It completes cr-ecm32-1 (`f22af07f`).

## Evidence
"On Unix, if a previously present partial's parent vanishes before hashing, `place` becomes `None`. If the parent returns as a symlink to another in-root directory, an empty name there can receive the file. Fail that pass when the parent cannot be established. On Windows the receiver discards even `Unjustified`, so a failed comparison can still authorize a later write. Enforce that refusal on every platform and retain the validated Windows handle for writes."

## Intake
Admitted. The Unix half is fixed here. The Windows half, honouring `Unjustified` and `WholeOf` off Unix with a validated handle, is Windows-only code. It joins cr-ecm32-4, blocked.

## What
`compute_resume_block_hashes` no longer takes a missing directory for one not yet made. On Unix, a place that cannot be established — the directory gone, or any other failure of the resolution — sends the file `Unjustified`, which fails it at its hold. Its retry then compares afresh (D-2026-11-23-2). The note on `ResumeBaseline::WholeOf` says so.

## Guard proof
- `transfer_session::tests::a_resume_whose_directory_is_gone_justifies_no_write` (Unix): a resume of `gone/r.bin`, with `gone` absent, is sent `Unjustified`.
- Mutation, restored byte for byte: a directory gone taken for one not made yet → red.
- All 1,163 `blit-core` tests pass. Native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `compute_resume_block_hashes`; guard.
- `crates/blit-core/src/remote/transfer/sink.rs` — `ResumeBaseline::WholeOf`'s note.

## Known gaps
- Windows: the receiver discards the baseline, `Unjustified` included (cr-ecm32-4, blocked).
