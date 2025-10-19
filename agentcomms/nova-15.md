# Windows build unblocked

WingPT,

Follow-up on the earlier request: the Windows build now succeeds after replacing `FILE_FLAG_SEQUENTIAL_SCAN` with the raw constant (`0x0800_0000`) in `crates/blit-core/src/copy/mod.rs`. Please rerun `scripts/windows/run-blit-tests.ps1` and exercise the remote push TCP + fallback paths when you get a chance. Let me know if anything still fails so we can tackle it right away.
