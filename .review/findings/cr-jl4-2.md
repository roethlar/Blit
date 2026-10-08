# cr-jl4-2: a retry names its files by text, losing non-UTF-8 names

**Severity**: HIGH (reviewer: HIGH)
**Status**: Fixed — awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: —
**Reviewer**: codex / gpt-5.6-sol / xhigh / frontier (fallback grade), codex-cli 0.159.3; openreview jl4-r1 over `3913ab70..2dab5d21` (record `.review/results/jl4-r1.codex.json`); owner goal of 2026-10-07

## Evidence
crates/blit-cli/src/jobs.rs:345 — failed_paths distinguishes failures by (path, raw) while collecting them, then discards raw and constructs every retry PathBuf from the lossy text alone.

## Predicted observable failure
A failed non-UTF-8 filename on a Unix source will not match the source enumeration's exact OS path, so the retry can transfer nothing and report success; two raw names that collapse to the same display text can also be merged or both selected instead of retrying exactly one.

## Reviewer's suggested approach
Use a typed retry identity carrying display text plus reversible raw bytes, preserve it in the versioned record, and teach both local and remote retry filters to compare that exact identity.

## Intake
Admitted. The retry set is rebuilt from each failure's display text, which names no file when the name is not UTF-8; the failure's own bytes, kept since cr-jl3a-3/cr-jl3afix1-1, must name it.

## What
A retry names each failed file by its own bytes when the record kept them: `raw_name::unescape_raw` reads back what `escape_raw` wrote, and on Unix `jobs::retry_path` builds the path from those bytes (its text names no file); elsewhere, where raw names do not arise, the text. A local source's filter matches that path exactly. For a remote source the wire filter carries such names as their bytes (contract 7, `FilterSpec.files_from_raw = 8`, built from `raw_relative_bytes`), and the origin lists them by their bytes where it can hold such names (`operation_spec::raw_listed`), nothing elsewhere. Two failures whose names collapse to one text stay two paths.

## Guard proof
`escaped_bytes_read_back_exactly` (core: escape/unescape round trip; malformed escapes refused), `a_raw_named_failure_is_retried_by_its_bytes` (CLI, Unix: the retry path is the bytes; the wire spec lists it under `files_from_raw`, the UTF-8 one under `files_from`), `raw_listed_paths_name_files_by_their_bytes` (core, Unix: listed by bytes where storable, nothing where not), and `a_specs_raw_list_reaches_the_filter` (core, Linux only — compiled here by cross-clippy, runs in CI). Four mutations each red alone — the retry path ignoring the bytes, the wire putting them in the text list, the origin dropping them, unescaping misreading hex — restored green. Not run here: a non-UTF-8 file retried end to end (macOS cannot hold such names).
