# audit-7e-cleanup: remove tracked AppleDouble + empty npm stubs

**Severity**: Style / Workspace hygiene
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `16a92ce`
**Parent finding**: `audit-7-code-health`.

## What

The repo tracked 33 macOS AppleDouble sidecar files (`._<name>`
resource-fork stubs created when the tree was copied onto a non-native
filesystem) scattered across `crates/**` and `docs/**`, plus two empty
no-op npm artifacts (`package.json` = `{}`, `package-lock.json` with an
empty `packages` map). This is a Rust-only workspace with no Node tooling.

## Approach

- `git rm` all 33 `._*` files and `package.json` / `package-lock.json`.
- Added `._*` to `.gitignore` (under the OS-specific section) so the
  AppleDouble sidecars can't return. Per the finding's scope, only the
  `._*` pattern is gitignored — the npm files are simply removed (if Node
  tooling is ever legitimately added, a real `package.json` can be
  committed then).

## Safety

No build/test impact: none of the removed files were compiled or
referenced. Verified `git grep` finds no dependency on `package.json`
(the only matches were unrelated `.serena` monorepo-config comments and
this audit's own notes). `cargo fmt --check`, `cargo build --workspace`,
and `cargo test --workspace` all pass post-removal.

## Files changed

- Deleted: 33 `crates/**` + `docs/**` `._*` files; `package.json`;
  `package-lock.json`.
- `.gitignore`: added `._*`.

## Scope

One sub-item of audit-7. Remaining: 7b (dead code), 7c
(ARCHITECTURE/README), 7d (main.rs refactor). 7-cargo-lock verified.

## Reviewer comments

(empty — pending review)
