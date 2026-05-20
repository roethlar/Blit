# d-61-f1-trigger-push reopened

Reviewed commit: `68f53897d61051bd99b59b015bb1a57436a09964`
Reviewed at: `2026-05-20T23:45:01Z`
Reviewer: `claude-reviewer`

Validation:
- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed (538 tests).

## Finding

### 1. Malformed remote-shaped sources are misclassified as local push sources

Severity: Medium

The d-61 push branch is entered whenever `RemoteEndpoint::parse(&src)` fails and the kind is `Copy` (`crates/blit-tui/src/main.rs:4001`). That treats every parse error as "source is local". But this parser also fails for malformed remote-shaped inputs, not only local paths. For example, `nas:9031:/home` looks like a module-root remote source but is invalid because module-root syntax requires the trailing slash; the parser rejects that shape at `crates/blit-core/src/remote/endpoint.rs:67`.

With the current handler, a trigger commit like:

```text
src: nas:9031:/home
dst: other:9031:/backup/
kind: copy
```

falls through to the local→remote push path if the destination parses, launching `spawn_f1_push` with `PathBuf::from("nas:9031:/home")` (`crates/blit-tui/src/main.rs:4013`). That is the same footgun the transfer parser avoids: remote-shaped typos must not silently become local filesystem paths.

Expected fix: before entering the push branch, distinguish a genuinely local source from a malformed remote-shaped source. At minimum, reject/drop source strings containing remote syntax such as `:/` or `://` when `RemoteEndpoint::parse` fails, or reuse/extend the strict transfer endpoint parser semantics. Add a regression test where `src = "nas:9031:/home"` and `dst = "other:9031:/backup/"` does not start `f1_push` and does not start `f3_pull`.
