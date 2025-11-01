## Change journal refactor – 2025-10-28
- Replaced single `change_journal.rs` with module tree: `change_journal/{mod,types,snapshot,tracker,util}.rs`.
- `types.rs` defines `ChangeState`, `ProbeToken`, snapshot structs, and `ChangeTracker` data fields.
- `tracker.rs` implements load/probe/refresh/reprobe/persist using helpers from `util.rs` and snapshot capture/comparison from `snapshot.rs`.
- Platform-specific snapshot capture now lives under `snapshot.rs` submodules (macOS/Linux/Windows), mirroring old logic.
- Orchestrator continues to `use crate::change_journal::{ChangeState, ChangeTracker, ProbeToken, StoredSnapshot}` via re-exports.
- Ran `cargo fmt`; `cargo check -p blit-core`.