# cr-ecm36-3: a failure's display-shortened path was taken for another file

**Severity**: HIGH — a saved retry names a file that is not one. A failure's path is shortened on the wire to fit the report: a 4,618-byte path became a 4,096-byte `[truncated]…` rendering. Read back, that rendering was taken for a file's name. For one missing long path the CLI saved both the full name, from the failed-path set, and the rendering, called the list complete, and `blit jobs retry` reported two failures.
**Status**: Fixed; awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm36-3`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r37` over `d2494965..2c55cb3a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r37.codex.md`. It completes cr-ecm35-3 (`4146e1fa`) in part.

## Evidence
"For one missing 4,618-byte path, the CLI saved both the full name and its 4,096-byte `[truncated]…` rendering. It marked the list complete. `jobs retry` then reported two failures. Attach each bounded diagnostic to its exact path identity."

## Intake
Admitted as the reviewer put it. The shortened rendering is for people: per D-2026-11-24-1, nothing that names files may read it.

## What
- **The wire says so.** `FileFailure` gains `bool path_shortened = 5`, so the contract goes to v31. `FileFailure::to_wire` sets it when the bound shortened the path, and `from_wire` carries it into the in-memory `FileFailure.path_shortened`.
- **Nothing that names files reads it.**
  - The summary's join (`RunTotals::failed_files`) passes over a shortened failure. The exact name is in the failed-path set, and it is the file. Its reason is not joined to that file; the report's block still shows it.
  - The retry set's fallback to the report (`PassFailures::retry_set`, when the failed-path set was cut short) names no file by a shortened path.
- **Elsewhere** every failure made in memory carries `false`: a path is shortened only on the way out.

## Guard proof
- `sink::tests::a_shortened_path_says_it_was_shortened`: a failure past the path bound is shortened on the wire and says so, and reads back saying so; a short one does not.
- `run_log::tests::a_shortened_path_is_no_file`: a summary of one failed long path, its report entry shortened, names one file — the long one.
- Mutations, each restored byte for byte:
  - the shortening not said → red;
  - a shortened path taken for a name → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,104 tests over 57 binaries.

## Files changed
- `crates/blit-core/proto/blit.proto` — `FileFailure.path_shortened`.
- `crates/blit-core/src/transfer_session/mod.rs` — contract v31; wire literals.
- `crates/blit-core/src/remote/transfer/sink.rs` — `FileFailure.path_shortened`, `to_wire`/`from_wire`; guard.
- `crates/blit-core/src/run_log.rs` — the join passes over it; guard.
- `crates/blit-cli/src/transfers/retry.rs` — the retry set's fallback passes over it.
- Literals elsewhere: `payload.rs`, `transfers/failures.rs`, the CLI's `run_log.rs` and `failures.rs`, `tests/source_side_containment.rs`.

## Known gaps
- A shortened failure's reason does not reach the saved record for its file: the record names the file with no reason. The report's block shows the reason under the shortened rendering.
