# cr-ecm1-1: Two slice-1 guards do not compile on Windows

**Severity**: HIGH — `cargo test` cannot build `blit-core`'s tests on Windows, so the whole Windows leg fails; found before any push reached CI
**Status**: Fixed, awaiting re-review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: the commit carrying this record ("Fix cr-ecm1-1: …")
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, review `exclude-slice1-r1` over `3bc3ee47..66d1338a`. That run FAILED before a verdict (provider 503 at 2026-11-20T20:12Z); this finding is read from its event stream, where the reviewer's own commands and their output are recorded (`.review/results/exclude-slice1-r1.codex.events.jsonl`, events 102, 112, 143).

## Evidence
`crates/blit-core/src/transfers/filter.rs` — the tests `the_deletion_scope_ignores_case_on_any_filesystem` and `the_deletion_scope_ignores_case_after_a_component_the_disk_pairs` (added by slice 1, `f527c079`) call the fixture helper `files`, which was declared `#[cfg(unix)]`. Neither test is gated. The reviewer ran `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` in its export of the head and got `error[E0425]: cannot find function `files` in this scope` at both call sites; with the `#[cfg(unix)]` removed from the helper in its export, the same command exited 0.

## Predicted observable failure
On Windows, `cargo test --workspace` fails to compile `blit-core` (lib test): no test of that crate runs. CI's Windows leg would have gone red on the first push to GitHub.

## Intake
Admitted. The slice's own record said cross-target clippy "cannot run from this host"; that was wrong. Only the `x86_64-pc-windows-msvc` and `aarch64-apple-darwin` targets fail here (in `blake3`'s build script, which wants each target's C compiler); `x86_64-pc-windows-gnu` builds, as the reviewer's run shows. The slice was gated on Linux alone, which is how an ungated test came to lean on a Unix-only helper unnoticed.

## What
The helper `files` is no longer `#[cfg(unix)]`. It only creates directories and writes files, and ungated tests now use it on every OS.

## Guard proof
- Red: the reviewer's run at the unfixed head, above (events 102 and 112).
- Green: `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0 at the fix (Linux host `continuum`, rustc 1.100.0), as does the native command.
- This is a compile proof, not a run: the two tests have not executed on Windows. That needs CI.

## Files changed
- `crates/blit-core/src/transfers/filter.rs` — the `files` test helper loses its `#[cfg(unix)]`.

## Known gaps
- `aarch64-apple-darwin` still cannot be cross-checked from this host.
- The gate for every later slice now includes the `x86_64-pc-windows-gnu` cross-clippy; `docs/STATE.md` and the slice-1 record are corrected to say so.
