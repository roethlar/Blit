# cr-jl4-2: a retry names its files by text, losing non-UTF-8 names

**Severity**: HIGH (reviewer: HIGH)
**Status**: Open
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
