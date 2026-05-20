# d-45-f3-delete reopened

Reviewed commit: `f0be497000c9947fdfc6d271faf35dc1b7ad9b75`
Reviewed at: `2026-05-20T19:25:55Z`
Reviewer: `reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. TUI delete does not use the CLI's canonical wire-path construction.

   `crates/blit-tui/src/main.rs:2327` derives the purge path with `rel_path.to_string_lossy().to_string()`. The CLI `rm` path intentionally does not do that: `crates/blit-cli/src/rm.rs:33` iterates path components and joins them with `/` before sending `PurgeRequest.paths_to_delete`. That distinction matters because F3 cursor endpoints are assembled with `PathBuf::push` in `crates/blit-tui/src/browse.rs:594` and `crates/blit-tui/src/browse.rs:600`; on Windows clients this can produce platform-shaped paths, while the purge wire contract is forward-slash relative paths. A Windows TUI deleting `photos/old.jpg` from a Unix daemon can therefore send a non-canonical path string and fail to delete the intended remote entry.

   Please share or duplicate the CLI's component-join behavior for `run_f3_del`, and add a small test that constructs a multi-component cursor path through `browse::pull_source_endpoint` / `run_f3_del`'s path conversion boundary so the purge path is pinned as `photos/old.jpg`, not a platform-rendered `PathBuf`.

2. Successful delete leaves the F3 browser listing stale.

   The delete reply branch at `crates/blit-tui/src/main.rs:1179` only applies `app.f3_del.apply_done(...)` on success. It does not invalidate `browse_last_fetched_view`, kick a browse refresh, or remove the deleted row from `BrowseState`. After a successful purge, the F3 table still shows the deleted path and the operator can act on the stale row until they manually refresh. That is a visible inconsistency for a browser action whose purpose is to delete the selected path.

   Please refresh or otherwise reconcile the current F3 listing after an accepted delete reply. The existing `handle_f3_refresh` / `kick_browse_fetch` path can likely be reused; add coverage that a successful delete reply invalidates the current browse view or removes the row so the stale cursor path cannot remain as an apparently live entry.
