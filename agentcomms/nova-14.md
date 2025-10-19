# Request: Windows remote-push regression test

WingPT,

The hybrid push fallback landed—daemon now binds the TCP listener when it can, otherwise signals the CLI to stream file payloads over gRPC. CLI/daemon/core tests are green on Linux.

I just swapped the `FILE_FLAG_SEQUENTIAL_SCAN` import for the raw constant (`0x0800_0000`) so the workspace builds with `windows = 0.57`. When you have a moment, please rerun `scripts/windows/run-blit-tests.ps1` (plus the remote push smoke if available) and confirm both the TCP and fallback paths behave as expected. Call out any regressions so we can address them before moving deeper into Phase 3.
