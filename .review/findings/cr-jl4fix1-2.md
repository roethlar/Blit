# cr-jl4fix1-2: a Windows client drops a retry's raw names

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4fix1-r1 over `b2204f9a..041f993e` (record `.review/results/jl4fix1-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:403 — `retry_path` reconstructs raw bytes only under `cfg(unix)`; lines 408-410 discard `raw` on Windows and return the lossy display path used to build the remote filter.

## Predicted observable failure
A Windows client retrying a failed non-UTF-8 file on a remote Unix source sends the lossy text in `files_from`; the source matches no file, so the retry may copy nothing and report success.

## Reviewer's suggested approach
Keep retry identities as text plus raw bytes through orchestration and populate `FilterSpec.files_from_raw` directly, using PathBuf only when enumerating on the actual source host.

## Intake
Admitted. `retry_path` turns a failure's bytes into a path only on Unix; a Windows client retrying a non-UTF-8 file on a remote Unix source sends its lossy text, which names no file there. Raw names must travel as bytes from the record to the wire filter, becoming a path only on the host that enumerates them.

## What
A retry keeps a non-UTF-8 name as its bytes from the record to the wire, whatever host it runs on: `jobs::retry_names` splits a retry's names into text paths (`TransferArgs.retry_only`) and exact bytes (`retry_only_raw`, with `retry_left_in_place_raw` for the `--ignore-existing` split); `FilterInputs.retry_only_raw` puts the bytes on the wire as they are (`FilterSpec.files_from_raw`), and turns them into paths only for a local source that can hold such names; `retry_parts` splits text and bytes alike. The Unix-only `retry_path` is gone.

## Guard proof
`a_raw_named_failure_is_retried_by_its_bytes_on_any_host` (CLI, every platform: the bytes stay bytes and reach the wire spec), `an_ignore_existing_retry_splits_text_and_raw_names` (CLI: both kinds split), `a_retry_raw_name_reaches_a_local_filter` (core, Linux only — runs in CI). Three mutations each red alone — bytes turned back into text, bytes left off the wire, the split losing the bytes — restored green; the Linux-only local-filter seam was not mutated here.
