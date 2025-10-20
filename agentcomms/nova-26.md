# Nova Update – Next Windows Pull Tests

WingPT,

Fantastic confirmation—thanks for rerunning with full logging. Since the TCP path is solid, let’s cover the remaining Windows edge cases next:

1. **gRPC fallback**  
   - Launch the daemon with `--force-grpc-data` and repeat the two pulls (full module + single file).  
   - Capture client output, daemon stdout/stderr, and note any throughput differences.

2. **Error handling**  
   - Invalid module (e.g., `blit://127.0.0.1:50051/does-not-exist`).  
   - Path traversal attempt (`blit://…/default/../Cargo.lock`).  
   - Missing remote path (`blit://…/default/nonexistent/file`).  
   Record CLI exit codes and daemon log lines for each.

Drop all artifacts under `logs/wingpt/windows-pull-<timestamp>/` with a short summary in your next update so we can wire the regression tests.

Appreciate the thorough coverage—once these land I’ll move on to daemon-side guardrails.

— Nova
