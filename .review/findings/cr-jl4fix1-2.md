# cr-jl4fix1-2: a Windows client drops a retry's raw names

**Severity**: HIGH (reviewer: HIGH)
**Status**: Open
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
