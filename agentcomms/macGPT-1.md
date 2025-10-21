# macGPT Report — 2025-10-20

- Per nova-28, re-ran the targeted test suites: `cargo test -p blit-daemon`, `cargo test -p blit-cli`, and `cargo test -p blit-core` (all passed; existing warning remains at `crates/blit-core/src/fs_capability/macos.rs:162` about the unused `preserved` variable).
- Validated the pull fallback path by running `cargo run -p blit-daemon -- --force-grpc-data` and issuing two pulls:
  - `Cargo.toml` → `/tmp/blit-pull-file` (`logs/macgpt/macgpt-cli-pull-cargo-toml-fallback-20251020T010103Z.log`).
  - `crates/blit-cli/src` → `/tmp/blit-pull-dir` (`logs/macgpt/macgpt-cli-pull-cli-src-fallback-20251020T010103Z.log`).
  Daemon output: `logs/macgpt/macgpt-daemon-pull-fallback-20251020T010103Z.log`. Initial attempt against `README.md` returned the expected missing-path error; kept in `logs/macgpt/macgpt-cli-pull-readme-fallback-20251020T005739Z.log` for debugging history.
- Added `scripts/bench_local_mirror_macos.sh` (mac-specific wrapper) so we can bench to `target/macos` without touching shared artifacts; made it executable and left the original script untouched. Defaults now pin scratch workspaces under `/tmp`.
- Phase 2.5 macOS bench runs (RUNS=3, WARMUP=1, KEEP=1) now live under `logs/macos/`:
  - Size 0 MiB → `logs/macos/bench-local-mirror-size0-20251020T220501Z`: blit avg 0.355 s, rsync avg 0.020 s. Small tree still favors rsync’s startup costs.
  - Size 512 MiB → `logs/macos/bench-local-mirror-size512-20251020T220548Z`: blit avg 0.397 s vs rsync 1.234 s (~3.1× faster; ~1.97 GiB/s peak).
  - Size 2048 MiB → `logs/macos/bench-local-mirror-size2048-20251020T220703Z`: blit avg 1.597 s vs rsync 5.009 s (~3.1× faster; 1.58 GiB/s during measured passes).
