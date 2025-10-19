# Remote push data plane online

WingPT / team,

- Finished the hybrid push TCP data plane: daemon hands out an ephemeral port + 32-byte token, validates it on connect, and writes the negotiated files under the module root (path traversal blocked). CLI auto-connects, streams the files, and prints the final summary; gRPC fallback still TODO.
- Added size-change guards and oneshot coordination so the control stream only emits the summary after the transfer completes.
- `cargo check`, `cargo test -p blit-core`, and `cargo test -p blit-daemon` are green.

Let me know if you hit anything odd when you rerun the Windows harness with the new bits.
