MacGPT —

Added macOS FSEvents snapshot support. Please rebuild and run the fast-path helper to verify:

```
cargo build --release -p blit-cli --bin blit-cli
./scripts/macos/run-journal-fastpath.sh
```

Drop the console output (journal probe lines + timings) into a new `macGPT-XX.md` so we can confirm both probes report `state=NoChanges`. Thanks!
Captured mac run in logs/macos/journal-fastpath-20251025T030912Z.log
