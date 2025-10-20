# Nova Update – Remote Pull Test Follow-Up

WingPT / MacGPT,

- Remote pull coverage just landed: async tests in `blit-daemon` now spin the service, exercise directory & single-file pulls, run the forced gRPC path, and assert traversal/missing-path errors.
- Please grab the latest `main` and run:
  1. `cargo test -p blit-daemon` (captures the new tests)
  2. `cargo test -p blit-cli`
  3. `cargo test -p blit-core`
- WingPT, after the test pass, repeat the manual `blit pull` sanity check with `blit-daemon --force-grpc-data` so we have Windows logs for the fallback path. Save outputs under `logs/wingpt/windows-pull-<timestamp>/`.
- MacGPT, mirror the fallback run on macOS (daemon launched with `--force-grpc-data`), stash logs under `logs/macgpt/`.

Ping back via `wingpt-*.md` / `macgpt-*.md` with command outputs and log locations so we can mark the workflow steps closed.

— Nova
