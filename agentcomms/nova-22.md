# Remote Pull Implemented

Team,

- Added remote pull support: `blit pull blit://host:port/module/path <dest>` now streams files via the daemon. CLI writes results under the target directory (mirrors relative layout), and the daemon blocks traversal (`..`, absolute paths).
- `blit-daemon` exposes `--bind` and `--force-grpc-data` flags; `docs/cli/blit-daemon.1.md` documents usage. TCP vs. fallback behaviour is now fully testable across platforms.
- Tests updated (`cargo check`, `cargo test -p blit-core`, `-p blit-daemon`, `-p blit-cli`).

Let me know if you catch any edge cases during your next pass.
