# cr-ecm36-8: the forty-link test expected Linux's limit on every Unix

**Severity**: MEDIUM — a test that fails where it should pass, and a resolver bound that is Linux's. cr-ecm35-1's guard required a chain of 40 links to nothing to have its file made on every Unix. macOS (XNU) follows at most 32 links in a path (`MAXSYMLINKS`), so there the opener's own open fails with `ELOOP` before the resolver is reached. The resolver's bound of 40 was likewise Linux's alone.
**Status**: Verified — `combined-r38` (Codex openreview, 2026-11-24) reviewed `2c55cb3a..6ee7ed5e` — the `combined-r37` fixes — with `guard_confirmed: true`: the review covered it and required no change
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `5a570d02`
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli headless one-shot, openreview `combined-r37` over `d2494965..2c55cb3a` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r37.codex.md`. It completes cr-ecm35-1 (`be844676`) in part.

## Evidence
"The new test requires a 40-link chain to succeed on every Unix. Apple XNU limits pathname resolution to 32 links, so I expect the macOS test to fail with `ELOOP`. Keep the Linux boundary check and test macOS against its native behavior."

## Intake
Admitted as the reviewer put it.

## What
- **The host's limit.** `resume::MAX_FOLLOWED_LINKS` is the host's own limit on the links one resolution follows: 40 on Linux, and 32 on macOS and the BSDs (`MAXSYMLINKS`).
- **The resolver** (`resolve_destination`) follows at most that many.
- **The guard** builds a chain of exactly that length, and expects the file made at its end.

## Guard proof
- `file_copy::mode_tests::a_chain_of_forty_links_to_nothing_has_its_file_made`: on Linux, 40 links.
- Mutation, restored byte for byte: one link fewer than the system follows → red.
- Checks:
  - native and macOS (`CARGO_FEATURE_PURE=1`) clippy are clean;
  - the test passes on Linux.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `MAX_FOLLOWED_LINKS`; the resolver's bound.
- `crates/blit-core/src/copy/file_copy/mod.rs` — the guard by the host's limit.

## Known gaps
- No Mac is reachable: the 32-link chain is compiled for macOS, never run there.
