# audit-7-code-health: Code organization, dead code, and documentation gaps

**Severity**: Style
**Status**: Open
**Branch**: (none yet)

## What

Ground-up audit identified structural code health issues:

### Monolithic files

1. **`crates/blit-tui/src/main.rs`** (11,114 lines) — contains app event loop,
   screen orchestration, key dispatch, mode management, RPC fetching, and
   tab-strip rendering in a single file. ~10 responsibilities in one module.

2. **`crates/blit-core/src/orchestrator/orchestrator.rs`** (2,466 lines)
3. **`crates/blit-daemon/src/service/core.rs`** (2,444 lines)
4. **`crates/blit-core/src/remote/transfer/sink.rs`** (2,071 lines)

### Documentation gaps

5. **`docs/ARCHITECTURE.md`** — omits `blit-tui`, `blit-prometheus-bridge`,
   and `blit-app` crates. Proto service block shows only 6 of 12 RPCs.
   README structure tree shows only 3 of 6 crates.

6. **`README.md:74`** — clone URL uses template placeholder
   `github.com/your_org/blit.git` instead of actual org.

### Dead code and artifacts

7. **28 AppleDouble `._*` files** across `crates/`, `docs/`, `docs/cli/` —
   macOS resource fork artifacts that pollute the source tree. Should be
   removed and `.gitignore` updated to block them.

8. **`#[allow(dead_code)]`** on production code at 5 sites:
   - `crates/blit-core/src/fs_enum.rs:56-60` (const fields)
   - `crates/blit-core/src/copy/compare.rs:130,137` (genuinely unused functions)
   - `crates/blit-tui/src/diagnostics.rs:23`

9. **Empty files that are declared as modules:**
   - `crates/blit-app/src/progress.rs` — comment-only placeholder
   - `crates/blit-app/src/transfers/remote_remote_direct.rs` — comment-only

10. **`Cargo.lock` is in `.gitignore`** — workspace produces 4 binaries
    (`blit`, `blit-daemon`, `blit-tui`, `blit-prometheus-bridge`).
    Missing lockfile means non-deterministic builds and potential "works
    on my machine" failures.

11. **Empty `package.json` + `package-lock.json`** in project root — leftover
    from prototyping, serves no purpose in pure Rust project.

## Approach

- Split `main.rs` into event loop, key dispatch, render orchestration modules
- Update ARCHITECTURE.md and README.md to reflect current crate structure
- Remove `._*` files and add pattern to `.gitignore`
- Remove dead code or guard behind appropriate feature flags
- Track `Cargo.lock` in git (remove from `.gitignore`)
- Clean up empty placeholder modules and npm artifacts

## Files changed

TBD by coder. Multiple files across the workspace.

## Known gaps

- TUI main.rs refactor is the highest-effort item and should be done carefully
  to avoid regressions. Consider doing it as a separate slice with its own
  review.
- Architecture doc updates should be verified against actual crate structure
  before committing.
