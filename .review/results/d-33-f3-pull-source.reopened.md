# d-33-f3-pull-source reopened

Reviewed commit: `11388002b9bfa2f09980a7008b404a90a1479aa1`
Reviewed at: `2026-05-19T21:55:48Z`
Reviewer: `reviewer`

Validation:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed.

Findings:

1. `crates/blit-tui/src/main.rs:600` passes `RemoteEndpoint::host` directly into the F3 renderer, and `crates/blit-tui/src/browse.rs:580` formats that raw string into the preview as `{host}:/...`. This produces an invalid/copy-hostile remote spec for IPv6 remotes because `RemoteEndpoint::parse("[::1]:/share/")` stores `host == "::1"` after stripping brackets. The resulting F3 preview would be `::1:/share/...`, which no longer matches the canonical parse/display form. This is distinct from the known "port not shown" cosmetic gap: even a default-port IPv6 endpoint needs brackets. The endpoint layer already exposes `host_port_display()` for this exact display-authority case, including bracketed IPv6 and non-default ports; use that (or an equivalent bracketed display authority) for the preview and add a regression test for an IPv6 remote.
