# cr-win-1: Copy/mirror push retries can falsely clear a named-stream failure

**Severity**: HIGH — a retry pass can skip a file whose first-pass failure left the main bytes intact (e.g. a rejected NTFS stream), clear the failure, and exit 0 with an incomplete backup
**Status**: Fixed — portable guard red/green on macOS; Windows guard run pending
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `501c408d`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 5eeff4ac..8a1f04cb (Windows fixes win-1..5), record .review/results/ssc-win-range.codex.json)

## Evidence
crates/blit-cli/src/transfers/mod.rs:417 — retry passes now invoke the deferred push with move_verb=false, selecting the ordinary SizeMtime/SizeOnly comparison at crates/blit-cli/src/transfers/remote.rs:493; that comparison explicitly ignores named streams at crates/blit-core/src/transfer_session/mod.rs:5929.

Context: before win-4 (`44442e54`) every push — main pass and retries — compared with the move rule (IgnoreTimes); win-4 restored the copy rule for the main pass and, with it, for the retry passes too.

## Predicted observable failure
On a Windows push using --size-only, if the destination writes the main file but rejects an NTFS alternate data stream, the first pass records a per-file failure. The retry sees the matching main-file size, skips the file without checking streams, replaces the failure set with an empty result, and exits 0 although the stream is missing—silently producing an incomplete backup.

## Reviewer's suggested approach
Use the ordinary copy comparison only for the main pass. Retry-only entries should be transferred unconditionally or verified with an exhaustive comparison that includes named streams; add a Windows guard where an ADS-tail failure remains reported until the stream is successfully applied.

## What
`run_retry_passes` (crates/blit-cli/src/transfers/retry.rs) sets `ignore_times` on every retry pass's arguments. Every route gives it top precedence (local `compare_mode`, push/pull `comparison_mode`, delegated `delegated_pull_options`), so a retry pass re-sends exactly its set; the main pass keeps the user's compare. Move validation runs before the loop and is never re-entered; a move with `--checksum` keeps Checksum on the local, push and pull routes (move's mapping reads checksum first), whose verdict covers named streams.

Scope found while fixing: not only `--size-only`. When a pushed file's stream tail fails, the streamed sink leaves the bytes in place (sink.rs `commit`: "a failed metadata tail leaves the written bytes in place") and never stamps the mtime, so the destination is the same size and NEWER — the default SizeMtime compare skips it too (`manifest.rs` `compare_file`: "Target is same age or newer - skip"). The local route is not exposed this way (its `PartialTarget` removes the file on a failed tail), but the fix sits in the shared loop, so every route behaves the same.

## Guard proof
Portable: `retry_pass::a_retry_pass_re_sends_a_file_whose_destination_looks_current` (ungated; local + push). The pfc blocked-directory fixture fails `blocked.txt` on the main pass; during the 4 s retry wait the test replaces the blocking directory with a file of the source's size and mtime but other bytes (what a failed tail leaves, made portable). Exit 0 must mean the source's bytes landed. Mutation: `pass_args.ignore_times = true;` commented out → FAILED on the local half (exit 0, destination still the 14 `x` lookalike bytes); with the local half temporarily disabled → FAILED on the push half (same, daemon log shows the main-pass `Is a directory` failure). Restored byte-identical → green; `retry_pass` 12/12.

Windows: `retry_pass::windows_a_rejected_named_stream_is_re_sent_on_the_retry_pass` (cfg(windows), push). The destination `tagged.bin` carries a stale `meta` stream held open with share mode 0, so `replace_streams` fails after the bytes land; the test asserts the bytes landed, frees the handle during the wait, and requires exit 0 with the source's stream content. Compile-checked by Windows cross-clippy only; its red/green run on Windows is PENDING (VM unreachable 2026-10-04).

Gate (macOS, at `501c408d`): fmt clean; clippy `-D warnings` clean native, `x86_64-unknown-linux-gnu`, and `x86_64-pc-windows-msvc` (`--features blake3/pure`); `cargo test --workspace --no-fail-fast` 1344 → 1345 passed / 0 failed / 2 ignored.

## Known gaps
- `--ignore-existing` is the user's orthogonal axis and is left on: under it a retried file whose bytes landed before its stream failed exists, so the retry skips it and clears the failure. Turning it off on retry would overwrite pre-existing destination files that failed only at the source scan (never compared), which the flag promises to keep.
- A retried file that failed before any compare (e.g. a source locked at scan, win-2) is re-sent even if the destination is current, and overwrites a same-size NEWER destination that the default compare would have kept — the robocopy-shaped meaning of a retry (re-copy).
- Outside this finding: a plain re-run (not a retry pass) under the default compare still skips a file left by a failed stream tail (ls-6, D-2026-08-01-4: the default compare never interrogates streams); `--checksum` is the converging path. The sink comment "re-run converges it" holds only for `--checksum`. Raised with the owner, not filed.
- No Windows run and no CI yet for this fix.
