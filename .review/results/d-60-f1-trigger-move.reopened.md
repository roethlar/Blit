# d-60-f1-trigger-move reopened

Reviewed commit: `e3a8836f38e436eb03a66da676bf07eef1f3ac2b`
Reviewed at: `2026-05-20T23:25:37Z`
Reviewer: `claude-reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed (528 tests).

## Finding

### 1. The module-root move guard test does not exercise the guard

Severity: Low

`handle_f1_trigger_keystroke_move_rejects_module_root_source` claims to cover the d-60 module-root move refusal, but it seeds the source as `nas:9031:/home` (`crates/blit-tui/src/main.rs:6958`). That string is not a valid module-root endpoint in this parser: module syntax requires `server:/module/...`, and an actual module root includes the trailing slash (`nas:9031:/home/`). The parser enforces this in `RemoteEndpoint::parse` by requiring a slash inside the module remainder (`crates/blit-core/src/remote/endpoint.rs:61`), and the existing parser test uses `example.com:/media/` for module root syntax (`crates/blit-core/src/remote/endpoint.rs:302`).

As written, the handler test passes before it reaches the new `is_deletable_remote_path` gate at `crates/blit-tui/src/main.rs:3880`: `RemoteEndpoint::parse(&src)` fails, so no confirm opens for the wrong reason. If the d-60 guard were removed, this test would still pass.

Expected fix: use a valid module-root source such as `nas:9031:/home/` in the F1 trigger test, and ideally add/keep a paired subpath case like `nas:9031:/home/docs` to prove that only module roots are refused while normal move sources still reach the F3 destructive confirm.
