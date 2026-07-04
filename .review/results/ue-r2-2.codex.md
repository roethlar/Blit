# codex raw review output — ue-r2-2

Invocation: `codex exec -s read-only -c 'plugins."superpowers@openai-curated".enabled=false' "<review prompt over commits 042ca4b..0788e83>"`
(stdin closed; this codex build streams its full session transcript to
stdout — per the established record format only the final findings
message is preserved below, verbatim. tokens used: 306,188.)

---

codex
- `crates/blit-daemon/src/service/push/control.rs:476` / `crates/blit-core/src/remote/pull.rs:1054` — **High** — ADD acceptance is target-only, not live-count bounded. Replaying `ADD target_stream_count=2` with fresh 16-byte tokens can arm/spawn more streams than `CapacityProfile.max_streams` on both receiver sides. This violates the weak-end capacity protection and credential/resource model.

- `crates/blit-daemon/src/service/pull_sync.rs:928` / `crates/blit-daemon/src/service/pull_sync.rs:1017` — **Medium** — The pull daemon validates an epoch-N socket inline inside the controller `select`. A stray armed connection that stalls the token read blocks polling of the data pipeline and control stream for up to `PULL_TOKEN_TIMEOUT` instead of being isolated like push’s spawned handshake workers.

- `crates/blit-daemon/src/service/pull_sync.rs:954` — **Medium** — After accepting an ADD socket, the controller ignores `ctl_tx.send(SinkControl::Add(sink))` and still settles the epoch accepted at line 955. If the elastic pipeline has already completed/closed, the authorized socket is dropped without an END record and the dial state lies about the live stream count.

VERDICT: NEEDS FIXES.
