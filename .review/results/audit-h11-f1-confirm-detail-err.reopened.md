# Reopened: audit-h11-f1-confirm-detail-err

**Reviewer**: gpt (relayed via owner), 2026-06-05.
**Slice logic**: correct in dirty tree (helper has explicit Remote/Local/Err
arms, debug_assert + release-mode non-lying degrade verified; targeted tests
pass).
**Blocker**: repository reproducibility — a clean checkout at the round-1 SHA
`28c19a8` cannot build, so verification cannot be reproduced.

## Finding

`crates/blit-tui/src/main.rs:34` has been committing `mod dual_pane;` for
multiple slices, and `main.rs:1058` routes `Screen::Dual` through
`screens::dual_pane::render_into`. The actual module files
(`crates/blit-tui/src/dual_pane.rs`, `crates/blit-tui/src/screens/dual_pane.rs`)
were untracked in the working tree — never committed.

Reviewer reproduced:

```
$ cargo fmt --all -- --check
Error writing files: failed to resolve mod `dual_pane`

$ TMPDIR=/private/tmp cargo check -p blit-tui
error[E0583]: file not found for module `dual_pane`
error[E0433]: cannot find `dual_pane` in `screens`
```

audit-h11 itself only touched `display_f1.rs`. The h11 implementation is
sound; what blocked verification is the unbuildable repo state at the round-1
SHA.

## Resolution

Build-fix commit lands at master `1b3cb39` ("Fix repo build: commit Phase 6
dual-pane bootstrap + tighten StallGuardWriter Ok(0)"). It commits the two
missing dual-pane module files and the `pub mod dual_pane;` declaration in
`screens/mod.rs`. Clean-checkout validation passes:

- `cargo fmt --all -- --check`: clean
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo test --workspace`: 646 blit-tui + 315 blit-core + all other suites
  green from a wiped target dir.

Re-arming the h11 sentinel at master `1b3cb39` (the build-fix commit) — that
SHA includes the original h11 implementation (`28c19a8`) plus the build fix,
so reviewer can reproduce all validations from a clean checkout at the new
SHA.

## Required fixes

None for the h11 code itself. The fix is the build-fix commit landed at
`1b3cb39`. Re-verification at master `1b3cb39` should pass.
