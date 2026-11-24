# cr-ecm39-4: the live log named a file by the bytes any pass had heard for its spelling

**Severity**: MEDIUM — the log names a failure as the wrong file. log-named-text's recorded known gap. The run log named each live event's file by the bytes heard for its spelling, from a map of `RawName` events kept for the whole run. Within one pass the manifest grants one entry per spelling, so that pass's own names are exact. A later pass's text-named file was given an earlier pass's byte-named sibling.

The reviewer's log:
- recorded a raw-named file as copied;
- then attributed its text-named sibling's write failure to that same raw file.

The closing summary added the correct failure, leaving both in the log.
**Status**: Reviewed, found incomplete — `combined-r41` (Codex openreview, 2026-11-24) reviewed `d7f5f5e9..b6e6fab9` — the `combined-r40` fixes — with `guard_confirmed: true`: "The per-failure identity and Linux `openat2` fast path are worth keeping. A 72-directory-deep resume completed with a 64-descriptor limit and produced the correct bytes. I would require the following changes." "In a push with 24 pairs of identically displayed names, 12 UTF-8 files landed. The sender logged all 12 as their rejected byte-named siblings. Associate live events with the granted manifest entry. The sender currently announces raw names for rejected entries too, so a per-pass map still misattributes them." The remaining work is cr-ecm40-4
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `9a85aa5e`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r40` over `86d69ee3..d7f5f5e9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r40.codex.md`. It completes log-named-text (`f62df807`).

## Evidence
"The live log recorded a raw-named file as copied, then attributed its text-named sibling's write failure to that same raw file. The closing summary added the correct failure, leaving both in the log. Carry the exact identity on live progress events. The run-wide lookup by display text can return an earlier pass's filename."

## Intake
Admitted. The fault is the run-wide lookup; the means differs from the reviewer's in one respect.

The live events are made at many places that know only a path, such as a tar member's name or a path read from the wire. Carrying bytes on each event would reach into all of them.

Each pass has its own audit lane, and the lanes are handled in order. So the names are kept per lane: a file is named by the bytes its own pass heard, which its pass's manifest makes exact.

## What
- **The run log's audit lane** (`RunLog::lane`) keeps the names heard on it: one pass's `RawName` events, first for each text.
- **`observe_from`** names a lane's events from that lane's names, not the run's. A log observed directly (`observe`, one session's, as the daemon's is) keeps its own names, as before.

## Guard proof
- `run_log::tests::each_pass_names_its_own_files`: one log, two passes, each with its own lane.
  - The first hears `dup` + byte `FF` for the spelling `dup` + U+FFFD and copies it.
  - The second fails a file of that spelling.

  The copy is logged with the bytes; the failure as the text name, with none.
- Mutation, restored byte for byte: a file named by every pass's names → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,142 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/run_log.rs` — `lane`, `observe`, `observe_from`; guard.

## Known gaps
- Within one pass, two sessions on one lane would share its names. One pass's sessions write one destination from one manifest.
