# 2025-10-28 – Remote push throughput debugging

## Context
- CLI now streams tar shards as soon as the daemon sends the first need-list batch (shared buffered payload stream between control-plane and TCP data plane).
- Daemon batches FilesToUpload responses (size/entry/time thresholds) and logs `[data-plane] …` errors around every I/O path.
- Current blocker: TCP connection resets mid-transfer with `upload_tx send failed` on the daemon; need to trace why the upload channel closes after the client starts writing tar shards.

## Next steps
- Capture full `/tmp/blitd.log` with the new diagnostics while rerunning `blit-cli mirror -v -p ~/ skippy://elphaba/home` to identify the failing operation.
- Once the server-side cause is known (e.g., disk error, metadata mismatch, queue overflow) implement the fix and re-test throughput.
