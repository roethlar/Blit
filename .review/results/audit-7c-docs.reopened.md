Reviewed sha: `43087f5d6660592be3a4f8bbafd8503c1e2b9083`

Reopened.

Findings:

1. `docs/ARCHITECTURE.md:120` says `blit-app` sits between only `blit-cli` / `blit-tui` and `blit-core`, then line 127 says only the CLI and TUI are thin shells over it. Current code has `blit-prometheus-bridge` depending on `blit-app` and importing `blit_app::admin::jobs` (`crates/blit-prometheus-bridge/Cargo.toml:11`, `crates/blit-prometheus-bridge/src/server.rs:16`). Since the diagram places the bridge over `blit-app`, the prose needs to include the bridge too.

2. `docs/ARCHITECTURE.md:123-127` omits live `blit-app` public modules from the crate-surface summary. `crates/blit-app/src/lib.rs:16-24` exports `check` and `scan`, but the new text lists endpoint/transfers/client/admin/diagnostics/profile only. This is the same "full crate surface" gap the doc slice is supposed to close.

3. `docs/ARCHITECTURE.md:132-133` describes F4 as just "profile". Current F4 also contains Verify and Diagnostics (`crates/blit-tui/src/screens/f4.rs` renders Profile, Verify, and Diagnostics blocks). Please document it as Profile/Verify/Diagnostics or similar so the architecture doc matches the live TUI.

I did not run cargo gates because this is a docs accuracy reopen found during inspection.
