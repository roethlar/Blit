# audit-7-cargo-lock: track Cargo.lock for reproducible builds

**Severity**: Style / Build hygiene
**Status**: In progress / pending review
**Branch**: `phase5/a1`
**Commit**: `dfaecfe`
**Parent finding**: `audit-7-code-health` (item 10). Other audit-7 items
(AppleDouble cleanup, dead code, doc updates, main.rs refactor) ship as
separate audit-7* slices.

## What

`Cargo.lock` was `.gitignore`d. This workspace builds **four binaries**
(`blit`, `blit-daemon`, `blit-tui`, `blit-prometheus-bridge`), so an
untracked lockfile means contributors and CI can resolve different
dependency versions — non-deterministic builds and "works on my machine"
drift. Rust's official guidance is to commit `Cargo.lock` for
binary/application crates (only libraries omit it).

## Owner decision

The owner authorized tracking the lockfile (2026-05-23, "do the right
thing"). This **supersedes** the prior project rule that `Cargo.lock` is
gitignored and must never be `git add`ed — that rule is lifted *only*
for the lockfile itself. (Recorded in memory `audit-owner-decisions`.)

## Approach

- Remove `Cargo.lock` from `.gitignore` (with an inline note explaining
  why it's now tracked).
- Commit the current workspace lockfile. Verified consistent with the
  manifests via `cargo check --workspace --locked` before committing
  (a stale lock would have failed `--locked`).

## Files changed

- `.gitignore`: drop the `Cargo.lock` line + explanatory comment.
- `Cargo.lock`: now tracked (workspace root lockfile, format version 4).

## Tests

None applicable — this is a build-reproducibility / repo-policy change,
not code. Build integrity is asserted by `cargo check --workspace
--locked` succeeding against the committed lock.

## Note for future slices

This is the **only** slice that intentionally commits `Cargo.lock`.
Every other slice continues to keep generated lock churn out of its
commits (stage named source files explicitly).

## Reviewer comments

(empty — pending review)
