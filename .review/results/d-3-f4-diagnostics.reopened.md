# d-3-f4-diagnostics reopened

Reviewed sha: `bb93f8c7f19bd6ac4ed941bbac257a3dc0624ef3`

## Verdict

Reopened.

## Findings

1. **Medium — TUI diagnostics dump does not match the CLI diagnostics JSON contract.**

   The finding doc says the TUI dump "matches `blit diagnostics dump --json` output", but `run_diagnostics_dump` builds a different object: it emits only `blit_version`, `source`, `destination`, display fields, and `same_device` (`crates/blit-tui/src/main.rs:1251`). The CLI JSON includes `invocation` and an `rsync_resolution` block, and computes `destination` / `same_device` against the resolved destination (`crates/blit-cli/src/diagnostics.rs:176`). The TUI version also snapshots `dst_endpoint` directly (`crates/blit-tui/src/main.rs:1254`) instead of applying `resolve_destination`, so a source directory copied into a destination container will report the wrong effective destination and wrong `same_device` context.

   This matters because the whole point of this action is bug-report parity with `blit diagnostics dump --json`; any consumer or operator comparing the TUI file with CLI output will see missing fields and potentially incorrect destination resolution.

   Fix direction: share or mirror the CLI's dump assembly: parse source + raw destination, call `resolve_destination`, compute `source_is_contents`, `dest_is_container`, `pre_resolve_destination`, `resolved_destination`, `resolution_changed`, and `same_device` against the resolved destination. Add a unit test or integration-level helper test that asserts the TUI dump JSON has the same top-level shape and `rsync_resolution` behavior as the CLI for a container-destination case.

## Gates

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test -p blit-tui` passed: 114 tests.
- `cargo test --workspace` passed.
