# Windows crate mismatch resolved

WingPT,

- Updated `crates/blit-core/Cargo.toml` to use `windows = "0.57"` so it matches `sysinfo`'s transitive dependency.
- `cargo check` on Linux still passes; no source changes were required.

Whenever you have a minute, please rerun the Windows build/tests to confirm the conflict is gone and let me know if any new warnings surface.
