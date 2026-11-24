# cr-ecm39-1: the known-identity mark was keyed by display text, so a sibling of the same spelling was never named

**Severity**: HIGH — two failures are saved as one, and the second file is never retried. Brought in by cr-ecm38-1, and its recorded known gap.

cr-ecm38-1 marked a retry scan's text failures as known by their text (`text_known`). A write site's failure of a byte-named sibling of the same spelling is recorded by that text too, so it was left alone by `named_raw` and took the known failure's identity.

The reviewer's run had two files of one spelling:
- one disappeared before the first retry;
- the other became writable before the second.

The base copied the available file. The head retried only the missing name and saved one identity for two failures.
**Status**: Reviewed, found incomplete — `combined-r41` (Codex openreview, 2026-11-24) reviewed `d7f5f5e9..b6e6fab9` — the `combined-r40` fixes — with `guard_confirmed: true`: "The per-failure identity and Linux `openat2` fast path are worth keeping. A 72-directory-deep resume completed with a 64-descriptor limit and produced the correct bytes. I would require the following changes." "Before manifest resolution, the public encoder returned one path and `truncated=false` for two failures sharing a display name. After resolution it returned both. Require resolved identities at the encoding boundary or mark the list incomplete when distinct failures collapse in the emitted set." The remaining work is cr-ecm40-6
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `deb7fa27`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r40` over `86d69ee3..d7f5f5e9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r40.codex.md`. It completes cr-ecm38-1 (`b1f6b6d5`).

## Evidence
"Two filenames had the same displayed spelling. One disappeared before the first retry, and the other became writable before the second. The base copied the available file. The head retried only the missing name and saved one identity for two failures. Track whether the identity is known on each failure. The current set keyed by display text applies one file's identity decision to its sibling."

## Intake
Admitted as the reviewer put it.

## What
In `SinkOutcome`, each failure says whether its identity was known when it was recorded.
- **Recording.** One function records every failure (`record_failure_as`), with a `known` argument.
  - An identity known when recorded is exact at once, in `failed_identities`: one with bytes, or a text known to be its name (`record_known_failure`).
  - A write site's failure recorded by its text alone waits apart, in `unnamed_texts`, to be named from the manifest.
- **Report entries.** Each carries an in-memory `identity_known` flag, never sent on the wire.
- **`named_raw`** names only report entries not known and identities waiting apart, so a known failure and a sibling of its spelling stay two.
- **The wire form** (`wire_failed_identities`) and the count of unnamed failures read the two sets together, so neither depends on the naming having run.
- The display-text set `text_known` is removed.

## Guard proof
- `sink::cr_win_1_tests::a_known_text_and_a_sibling_of_its_spelling_are_two_failures`: a known `dup` + U+FFFD and a write site's failure recorded by that text, named from a manifest that maps the text to `dup` + byte `FF`.
  - They are two failures, each by its own identity: text, and bytes.
  - Nothing is counted unnamed.
- `transfer_session::tests::a_scan_failure_keeps_its_own_identity` (cr-ecm38-1's guard) still holds.
- Mutations, each restored byte for byte:
  - a write site's text taken for a known identity → red;
  - a known failure named from the manifest → red.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux: 2,139 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs`:
  - `FileFailure.identity_known`;
  - `SinkOutcome.unnamed_texts`;
  - `record_failure_as`, `record_known_failure`, `named_raw`, `has_unnamed_failures`, `merge_failures`, `wire_failed_identities`;
  - guard.
- Literals elsewhere: `payload.rs`, `run_log.rs`, `transfers/failures.rs`, the CLI's `run_log.rs`, `failures.rs` and `retry.rs`.

## Known gaps
- None known.
