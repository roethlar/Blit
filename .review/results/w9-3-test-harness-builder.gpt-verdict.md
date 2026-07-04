# w9-3-test-harness-builder — codex adjudication

**Slice commit**: `f6e592e`
**Raw review**: `.review/results/w9-3-test-harness-builder.codex.md`
(codex verdict: **NEEDS FIXES**, 1 finding)
**reviewer: gpt-5.5**

## Finding 1 — fake-server port bypasses the claimed-port set

> `crates/blit-cli/tests/common/mod.rs:475` — **Medium** —
> `spawn_fake_blit_server()` still binds `127.0.0.1:0` outside the
> process-global claimed-port set. In the same libtest process, a
> parallel fake server can take a port already handed to
> `spawn_daemon()` during the probe-to-bind gap; the child-death poll
> then either flakes the daemon test or can mark the fake listener as
> daemon-ready. This leaves the port-collision fix incomplete for
> mixed fake/daemon binaries like `remote_remote` and `jobs_lifecycle`.

**Verdict: ACCEPTED.** Verified against source: the fake bound `:0`
directly and never consulted the set `pick_unused_port` maintains, so
the OS could assign it a port promised to a daemon whose own bind was
still pending — the daemon then dies on "address in use" (honest panic
but a flake), or in the worst interleaving the daemon test's readiness
connect succeeds against the fake's listener while the doomed child is
still alive, silently running the test against the wrong server. Same
class as the daemon-vs-daemon race the slice fixed; the fake path was
missed.

**Fix**: the claimed-set is factored into `claim_port(port) -> bool`,
shared by both paths. `spawn_fake_blit_server` now loops binding `:0`
until the OS assigns an *unclaimed* port and keeps that listener (the
fake path has no probe-to-bind gap at all — the bound socket is handed
to tonic via `from_std`), so fakes and daemons can never be issued the
same port within a process.

**Fix commit**: `8641bc6` — gate re-run green
(fmt ✓ / clippy -D warnings ✓ / workspace 1479 passed, 0 failed,
2 ignored across 37 suites).
