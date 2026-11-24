# cr-ecm38-1: a retry scan's text failures recorded by their bytes lost what kind of entry each was

**Severity**: HIGH — a retry copies what the list never selected and exits 0. Brought in by cr-ecm37-2. To keep a retry scan's failure from being renamed from the manifest, cr-ecm37-2 recorded a text name by its text's bytes, as if it were a raw name. That erased the difference between three kinds of entry:
- a text path;
- a raw filename;
- a list entry that names no file.

With a single-file source and a `--files-from` list holding only `.`, the first pass rejected the entry. The two retries asked for `.` by its bytes, copied the source the list never selected, and exited 0. Off Linux, where no raw name is taken, a text name was dropped from the retry.
**Status**: Reviewed, found incomplete — `combined-r40` (Codex openreview, 2026-11-24) reviewed `86d69ee3..d7f5f5e9` — the `combined-r39` fixes and log-named-text — with `guard_confirmed: true`: "The held directory, explicit links from diagnostics to filenames, and preservation of named failures withheld from a retry are worth keeping. I would require the following corrections." "Two filenames had the same displayed spelling. One disappeared before the first retry, and the other became writable before the second. The base copied the available file. The head retried only the missing name and saved one identity for two failures. Track whether the identity is known on each failure. The current set keyed by display text applies one file's identity decision to its sibling." The remaining work is cr-ecm39-1
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `b1f6b6d5`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r39` over `6ee7ed5e..86d69ee3` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r39.codex.md`. It completes cr-ecm37-2 (`0e0b6d3c`).

## Evidence
"With a single-file source and a `--files-from` list containing only `.`, the first pass rejected the entry. With two retries, the command copied the unselected source and exited successfully. The scan converts text failures into raw-byte identities, bypassing list-entry handling on Linux. On Windows and macOS, that raw-name path drops even ordinary text names. Preserve the distinction between a text path, a raw filename and a list entry that names no file."

## Intake
Admitted as the reviewer put it.
- **The wire identity** of a scan's failure is once again exactly what the scan gave.
- **"Known, do not rename"** is held in the account instead: a structured mark, not a change of identity.

## What
- **`SinkOutcome::record_known_failure`** records a failure whose identity is known, as given:
  - its bytes where its name is not text;
  - otherwise its text, which is then marked as its identity (`text_known`).
- **`named_raw`**, naming failures recorded by text alone from the manifest, leaves the marked ones as they are.
- **`record_scan_failures`** (`transfer_session/mod.rs`) records the scan's failures this way, in place of their text's bytes. A text path, a raw name and a list entry that names no file stay what they were, and the retry filter's handling of each applies again.

## Guard proof
- `retry_pass::a_list_entry_naming_no_file_is_never_retried_into_the_source` (CLI, end to end): a single-file source with `--files-from` listing only `.` and two retries.
  - It exits nonzero.
  - The source is not copied.

  Red on the head before this fix, as the reviewer found.
- `transfer_session::tests::a_scan_failure_keeps_its_own_identity` (cr-ecm37-2's guard, now as the identity is): a vanished `dup` + U+FFFD keeps its text identity when a manifest maps that text to `dup` + byte `FF`, and no bytes are given it.
- Mutations, each restored byte for byte:
  - a known text named from the manifest → red;
  - a text recorded by its bytes, as cr-ecm37-2 had it → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,123 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` — `text_known`, `record_known_failure`, `named_raw`.
- `crates/blit-core/src/transfer_session/mod.rs` — `record_scan_failures`; the guard updated.
- `crates/blit-cli/tests/retry_pass.rs` — guard.

## Known gaps
- Within one session, a known text failure and a write-site failure of a raw-named entry recorded by that same text would share one identity. The account would then count one failure unnamed, and the retry would report it as not retried. The manifest grants one entry per text, so this needs a vanished text name and a failed byte-named sibling in one pass.
