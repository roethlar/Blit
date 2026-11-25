# cr-ecm40-4: the sender announced the bytes of the entries the destination refuses

**Severity**: MEDIUM — the log names a file it sent as its sibling. cr-ecm39-4's remaining half.
**Status**: Reviewed, found incomplete — `combined-r42` (Codex openreview, 2026-11-25) reviewed `b6e6fab9..e2c2e08d` — the `combined-r41` fixes, log-relay-cut and GitHub CI's seven — with `guard_confirmed: true`: "I would keep the held-directory walk, the identity check when it reopens a parent, and the change that marks a collapsed failure list incomplete. I found six corrections the approach still needs." "A `NeedBatch` removes the first header from the sender's map. A later byte-named sibling can then appear to be the first entry with that display text. With 8,192 intervening manifest entries, the receiver wrote only the UTF-8 file `dup�`, but the sender logged it as `dup\xff`. Emit the completion identity from the granted header or retain a consumed marker for the whole pass." The remaining work is cr-ecm41-3
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `d7d6d816`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r41` over `d7f5f5e9..b6e6fab9` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r41.codex.md`.

## Evidence
"In a push with 24 pairs of identically displayed names, 12 UTF-8 files landed. The sender logged all 12 as their rejected byte-named siblings. Associate live events with the granted manifest entry. The sender currently announces raw names for rejected entries too, so a per-pass map still misattributes them."

## Intake
Admitted as written.

## What
The sender keeps the first entry of each spelling, as the destination grants the first and refuses the rest (cr-ssc5-1). `keep_first_of_its_text` now says whether an entry was that first. The sender announces a file's bytes (`report_raw_name`) only for the entry it keeps. Announced for a second entry of that spelling, the bytes named the granted file in the pass's log.

## Guard proof
- `job_log_cli::a_push_logs_each_file_it_sent_by_its_own_name` (Unix):
  - The push has four pairs, made in alternating order, because which of a pair is listed first is the filesystem's: creation order on Btrfs, newest first on tmpfs.
  - Every file lands; the refused one of each pair lands on the retry (cr-ecm13-3).
  - The log's `file-sent` events name each by its own name.
- Mutation, restored byte for byte, on tmpfs and on Btrfs: every byte-named header announced → red on both. With one pair, it was red on tmpfs only.
- Checks:
  - native, Windows and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the workspace passes on Linux, with the round's seven fixes: 2,146 tests over 57 binaries.

## Files changed
- `crates/blit-core/src/transfer_session/mod.rs` — `keep_first_of_its_text`; the announcement.
- `crates/blit-cli/tests/job_log_cli.rs` — guard.

## Known gaps
- A filesystem that lists every pair bytes-first cannot show the fault; the guard is green there either way.
