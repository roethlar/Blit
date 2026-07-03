# Windows Test Harness Notes (2025-10-17)

- Added `scripts/windows/run-blit-tests.ps1` to run `cargo fmt -- --check`, `cargo check`, `cargo test -p blit-core`, and full `cargo test`.
- Each command tee’s output into `logs/<step>-<timestamp>.log` for later inspection.
- PowerShell script now checks `$LASTEXITCODE` to avoid treating warnings as failures.
- Used to validate streaming orchestrator changes on Windows (unit tests passing).
