# d-61-f1-trigger-push reopened

Reviewed commit: `350800722f3f224fa72182da37c013d735ab7864`
Reviewed at: `2026-05-20T23:51:06Z`
Reviewer: `claude-reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed (539 tests).

## Finding

### 1. Bare-host push destinations still start an invalid push

Severity: Medium

Round 2 fixed the malformed-source classification issue. The remaining problem is on the destination side: the push branch accepts any `Ok(Endpoint::Remote(remote))` from `parse_transfer_endpoint(&dest)` before calling `app.f1_push.begin` and `spawn_f1_push` (`crates/blit-tui/src/main.rs:4014`). A bare host like `nas` or `nas:9031` parses successfully as `RemotePath::Discovery` (`crates/blit-core/src/remote/endpoint.rs:90`), so the TUI starts a push and shows the F1 running footer.

That destination cannot succeed as a push target. The push client later rejects `RemotePath::Discovery` as "remote destination missing module specification" (`crates/blit-core/src/remote/push/client/helpers.rs:296`), and the app already has the right preflight helper for this: `ensure_remote_destination_supported` rejects bare-host destinations before any transfer starts (`crates/blit-app/src/endpoints.rs:75`).

Expected fix: in the local-source push branch, require the parsed remote destination to pass the destination-shape gate before `f1_push.begin`/`spawn_f1_push`. Add a regression test where a local source with `dst = "nas:9031"` does not start `f1_push` and does not start `f3_pull`; keep the valid `nas:9031:/home/` test as the positive case.
