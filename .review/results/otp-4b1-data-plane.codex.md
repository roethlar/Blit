Reading additional input from stdin...
OpenAI Codex v0.142.5
--------
workdir: /home/michael/dev/Blit
model: gpt-5.5
provider: openai
approval: never
sandbox: read-only
reasoning effort: xhigh
reasoning summaries: none
session id: 019f3559-d2d7-7d53-a54c-2c5a85fc2c17
--------
user
Review the diff of commit 881d412 (run: git show 881d412). It implements otp-4b-1, the first sub-slice of otp-4b in docs/plan/ONE_TRANSFER_PATH.md: porting the TCP data plane onto the unified transfer_session (crates/blit-core/src/transfer_session/), replacing the in-stream carrier as the default. The wire contract is docs/TRANSFER_SESSION.md (frozen at otp-1; this slice only consumes DataPlaneGrant + the tokens). Scope: single epoch-0 stream, NO resize/multi-stream (that is otp-4b-2). The DESTINATION responder binds a TCP listener, mints session_token+epoch0_sub_token (16 bytes each), grants them in SessionAccept, and accepts+drains sockets via execute_receive_pipeline; the SOURCE initiator dials, authenticates (session_token then epoch0_sub_token), and sends payloads via DataPlaneSession over execute_sink_pipeline_streaming; in-stream stays as the requested fallback.

Check especially: (1) correctness of the concurrency — the DEST arms the accept+receive task before the SOURCE dials, the SOURCE dials before streaming the manifest, and the DEST joins the receive task at SourceDone; any deadlock, leak, or ordering hazard (AbortOnDrop usage, the bounded-accept timeout vs a slow manifest). (2) The completeness check on the DEST (files_written == needed_paths.len()) — is it correct for tar-shard bundling, 0-byte files, over/under delivery? (3) Socket auth: the exact token bytes and order match the contract; a mismatched socket faults. (4) Data safety: path containment on the data-plane receive path (the control-lane need-list check is bypassed for data-plane payloads — is the sink's containment still enforced?). (5) FAST/SIMPLE/RELIABLE, the REV4 invariants for touched transfer code (byte-identical results, StallGuard, cancellation, byte-accounting), and that the test count did not drop (1509 -> 1511). (6) Whether anything in transfer_session/data_plane.rs improperly depends on remote::push or the daemon push service (it must not — those drivers are deleted at otp-10).

Output a concise markdown findings list — each finding with file:line, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll inspect the target commit and the repo guidance/state that govern review scope, then trace the touched transfer paths against the frozen session contract. No skills invoked.
exec
/usr/bin/zsh -lc 'git show --stat --oneline --decorate --find-renames 881d412' in /home/michael/dev/Blit
 succeeded in 0ms:
881d412 (HEAD -> master) otp-4b-1: TCP data plane onto the session (single stream)
 .review/findings/otp-4b-data-plane.md              | 186 +++++++++++
 .../src/remote/transfer/session_client.rs          |  17 +-
 .../blit-core/src/transfer_session/data_plane.rs   | 339 +++++++++++++++++++++
 crates/blit-core/src/transfer_session/mod.rs       | 168 ++++++++--
 crates/blit-core/tests/transfer_session_roles.rs   |   7 +
 .../src/service/transfer_session_e2e.rs            |  54 +++-
 6 files changed, 737 insertions(+), 34 deletions(-)

exec
/usr/bin/zsh -lc "sed -n '1,220p' .agents/repo-guidance.md" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'git show --name-only --format=fuller 881d412' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 881d412b77512fd1d1e28c5932653b63e2cbf41a
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sun Jul 5 22:54:28 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sun Jul 5 22:54:28 2026 -0400

    otp-4b-1: TCP data plane onto the session (single stream)
    
    Port the TCP data plane onto the unified transfer session, replacing the
    in-stream carrier as the default. The DESTINATION responder binds a
    listener, mints session_token + epoch0_sub_token, and grants them in
    SessionAccept; the SOURCE initiator dials + authenticates the socket and
    sends payloads over the shared DataPlaneSession record framing while the
    control stream carries manifest/needs/summary. In-stream stays live as
    the requested fallback (--force-grpc-shaped).
    
    Single epoch-0 stream only; the zero-knowledge grant proposal is 1, so
    multi-stream is resize-only (otp-4b-2). Session-owned orchestration lives
    in transfer_session/data_plane.rs, reusing blit-core primitives
    (DataPlaneSession, execute_receive_pipeline, execute_sink_pipeline_
    streaming, dial_data_plane) — no call into remote::push or the daemon
    push service (those drivers die at otp-10).
    
    A/B parity vs old push over the data plane holds byte-identically.
    Suite 1509 -> 1511. [state: skip]
    
    Finding: .review/findings/otp-4b-data-plane.md
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>

.review/findings/otp-4b-data-plane.md
crates/blit-core/src/remote/transfer/session_client.rs
crates/blit-core/src/transfer_session/data_plane.rs
crates/blit-core/src/transfer_session/mod.rs
crates/blit-core/tests/transfer_session_roles.rs
crates/blit-daemon/src/service/transfer_session_e2e.rs

 succeeded in 0ms:
# Repo-Specific Guidance
<!-- Extends AGENTS.md; never overrides it. Rules and pointers only — state
     lives in .agents/state.md. -->

## Mission Detail

Blit is a high-performance, extensible file enumeration, planning, transfer,
and orchestration platform for local and remote backups, migration, and
cross-platform syncing, with CLI and daemon interfaces (`crates/blit-cli`,
`crates/blit-daemon`), async-aware planning, and Windows/Linux/macOS support.

## Reading Order

This repo predates the toolkit's `.agents/state.md` / `.agents/decisions.md`
convention and keeps its own canonical files at different paths; the
`.agents/` files below are pointer stubs, not duplicates. Read in this order:

1. `docs/STATE.md` — single entry point for current active work, queue, and
   blockers (the canonical equivalent of `.agents/state.md`; see
   `.agents/state.md` for why the path differs).
2. The active plan doc(s) `docs/STATE.md` names (under `docs/plan/`).
3. `REVIEW.md` + `.review/` — review-loop status for in-flight findings.
4. `docs/DECISIONS.md` — settled decisions and supersessions (the canonical
   equivalent of `.agents/decisions.md`).
5. `docs/agent/PROTOCOL.md` — the executable procedures behind the trigger
   vocabulary (`catchup`, `plan`, `decision`, `handoff`, `drift`, plus the
   repo-specific `slice` operator below).
6. Everything else in `docs/` — reference or historical; check its
   `**Status**:` header.
7. Code and tests are ground truth for behavior; plans are ground truth for
   intent. A mismatch is a drift finding, not permission to pick whichever is
   convenient.

`DEVLOG.md` is append-only history — write to it, never read it for current
state. `TODO.md` is the long-horizon backlog; the actionable queue lives in
`docs/STATE.md` and `REVIEW.md`. `.serena/memories/` and any tool-local
memory are scratch, never authoritative.

## Operator Vocabulary (repo-specific extension)

`AGENTS.md`'s Operator Requests section defines the toolkit's generic
vocabulary (`catchup`, `handoff`, `drift`, `decision`, `plan`, `playbook`).
In this repo every one of those words resolves to a procedure in
`docs/agent/PROTOCOL.md`, not to the generic `.agents/state.md`/
`.agents/decisions.md` files directly — read the matching section there and
execute it exactly:

- `catchup` → re-ground from `docs/STATE.md` + active docs; summarize
  now/next/blockers.
- `plan <topic>` → interview the owner, write `docs/plan/<NAME>.md`; no code
  until `**Status**: Active`.
- `decision <topic>` → record in `docs/DECISIONS.md`, propagate
  supersessions.
- `handoff` → update `docs/STATE.md` for the next session; prune to caps.
- `drift [scope]` → audit a doc against code; fix docs, file findings, raise
  questions.
- `slice` (repo-specific, no generic-template equivalent) → pick up the next
  review finding and run it through the codex review loop
  (`docs/agent/GPT_REVIEW_LOOP.md`).

**Review policy (D-2026-07-04-1): every code change and every plan change
goes through the codex review loop in `docs/agent/GPT_REVIEW_LOOP.md` — no
exceptions.** The `.review/README.md` async sentinel hand-off is retired;
its `findings/`/`results/` records and `REVIEW.md` remain the record store.

Claude Code exposes these as `/catchup`, `/plan`, … via `.claude/commands/`;
Antigravity exposes `catchup`/`handoff` as workspace skills in
`.agents/skills/`. This repo drafts `.agents/playbooks/reviewloop.md` as a template, but the codex review loop and `docs/agent/PROTOCOL.md` already cover that role for review-loop work.

## Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

- Test count may grow but never drop versus the prior baseline unless the
  removal is called out in the finding doc's Known gaps.
- Windows parity: after touching platform-specific code (`win_fs`, planners),
  run `scripts/windows/run-blit-tests.ps1`.
- Docs gate (CI): a push touching `crates/**` or `proto/**` must also touch
  `docs/STATE.md`, unless the commit message contains `[state: skip]`
  (reserved for mechanical changes). `scripts/agent/check-docs.sh` must pass;
  run it locally before pushing docs changes.
- Full command list and policy also live in `.agents/repo-map.json`.

## Remotes & Sync

- `origin` — `https://github.com/roethlar/Blit.git` (GitHub, canonical).
- `gitea` — `http://q:3000/michael/blit_v2.git` (LAN gitea mirror; pushed
  manually alongside or after `origin`, not auto-synced by any hook or CI
  job — it can lag GitHub by a commit or more at any given time).
- (Names verified against `git remote -v` 2026-07-04; an earlier revision
  of this doc called GitHub `github` and the mirror `origin` — that never
  matched the actual config and misread `origin/master` references.)
- Push policy: `.agents/push-policy.md` (ask). This repo's git-safety rules
  go well beyond a simple push policy — see Earned Practices below.

## Earned Practices

These are absolute; they exist because an unapproved `git merge -s ours`
octopus (commit `c793df2`) was pushed to `origin/master` without the owner's
consent (`docs/DECISIONS.md` D-2026-06-07-1).

- **No agent-created branches.** Agents never create git branches on their
  own decision. All work happens on `master` or the branch the owner already
  checked out.
- **Owner is the sole gate for git operations that publish, rewrite, or
  destroy.** No `push`, `push --force`/`--force-with-lease`,
  `reset --hard`, rebase or other history rewrite, `commit --amend` on
  pushed commits, or deletion of any branch/tag/ref (local or remote)
  without the owner approving that exact action in the current session.
  Working-tree edits, local commits, and read-only inspection
  (`status`/`log`/`diff`/`show`) need no special approval.
- **Branch deletion is by explicit name only** — the owner names the branch,
  the agent deletes that branch.
- **Before any push:** list the exact local refs, remote refs, and
  destination remotes, then stop and wait for approval.
- **`--merged`/`--no-merged` are unreliable in this repo.** The `-s ours`
  octopus made two now-abandoned branch tips ancestors of `master`, so
  `git branch --merged master` falsely lists them as merged and a plain
  `git merge` of those branches no-ops without landing any code
  (`docs/DECISIONS.md` D-2026-06-07-2). Verify content actually arrived
  (`git diff <branch> master`) before treating anything as landed or
  deleting it.
- **Checkpoints are owner-only.** Only an explicit owner message satisfies a
  checkpoint or verification step. Agents report observations; the owner
  declares pass/fail. Never self-certify a gate or continue a plan past one
  because the condition appears met. Approvals are single-use, step-specific,
  never carried across sessions. When the owner asks a question or thinks out
  loud, answer in plain English and stop — act only on an explicit decision.

## Style

- Rust edition 2021; format with rustfmt. Modules snake_case, types
  PascalCase, constants SHOUT_CASE; match existing names (`transfer_engine`,
  `TransferOrchestrator`, `PLAN_OPTIONS`).
- No blocking calls inside async contexts (use async send APIs in Tokio).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep
  tests deterministic; capture long logs under `logs/`.

## Project Map

- `crates/blit-core/` — core library (enumeration, planner, transfer engine,
  orchestrator); most logic and unit tests live here. New modules get
  re-exported in `crates/blit-core/src/lib.rs`.
- `crates/blit-cli/`, `crates/blit-daemon/` — CLI and daemon binaries; admin
  verbs (scan, ls, find, du, df, rm, completions, profile, list-modules) live
  in `blit-cli` alongside transfer commands.
- `crates/blit-app/`, `crates/blit-tui/` — TUI application layers.
- `crates/blit-prometheus-bridge/` — metrics bridge.
- `proto/blit.proto` — gRPC definitions; `blit-core`'s build script vendors
  protoc.
- Integration tests live per-crate (`crates/blit-cli/tests/`,
  `crates/blit-core/tests/`); the root `Cargo.toml` is a virtual workspace,
  so a root-level `tests/` dir would never be compiled. `scripts/` — helper
  tooling.
- `blit-utils` was intentionally removed; its admin verbs now live in
  `blit-cli`. Do not re-add it as a workspace member.

## Commit and Docs Hygiene

- Commit subject: short imperative ("Add streaming planner heartbeat").
  Review-loop commits: `Fix <id>: <one-line summary>` per `.review/README.md`.
- After meaningful work: append a `DEVLOG.md` entry (newest-first, ISO
  timestamp) and update `docs/STATE.md` — the `handoff` procedure does both.
- Every doc in `docs/plan/` carries a `**Status**:` header, one of:
  `Draft | Active | Shipped | Superseded | Historical`. Superseding a doc
  requires a `docs/DECISIONS.md` entry naming winner and loser, and an edit
  to the superseded text.
- `docs/STATE.md` stays ≤ 200 lines with ≤ 3 handoff entries; prune the
  overflow into `DEVLOG.md`.

exec
/usr/bin/zsh -lc "sed -n '1,220p' .agents/state.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Agent State (pointer)

Current state for this repo lives canonically in **`docs/STATE.md`** — read that
file, not this one. This stub exists only so tools expecting the standard
`.agents/` layout find their way.

Do not record state here. The path `docs/STATE.md` is load-bearing: the CI docs
gate (`.github/workflows/docs-gate.yml`), the doc lint
(`scripts/agent/check-docs.sh`), the session hooks (`scripts/agent/context.sh`),
and the `handoff` procedure in `docs/agent/PROTOCOL.md` all read or enforce it.

- Now / Next / Blockers: `docs/STATE.md`
- Verification commands: `.agents/repo-map.json` (mirrors `AGENTS.md` §5)
- How to update state: the `handoff` procedure in `docs/agent/PROTOCOL.md`

codex
The commit is small enough to review directly: it adds `transfer_session/data_plane.rs`, adjusts the session orchestration, and adds daemon/core tests. I’m now checking the contract and the active plan so the review is against the intended slice, not just the diff.
exec
/usr/bin/zsh -lc 'git show --find-renames --find-copies --unified=80 --no-ext-diff 881d412 -- crates/blit-core/src/transfer_session/data_plane.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 881d412b77512fd1d1e28c5932653b63e2cbf41a
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 22:54:28 2026 -0400

    otp-4b-1: TCP data plane onto the session (single stream)
    
    Port the TCP data plane onto the unified transfer session, replacing the
    in-stream carrier as the default. The DESTINATION responder binds a
    listener, mints session_token + epoch0_sub_token, and grants them in
    SessionAccept; the SOURCE initiator dials + authenticates the socket and
    sends payloads over the shared DataPlaneSession record framing while the
    control stream carries manifest/needs/summary. In-stream stays live as
    the requested fallback (--force-grpc-shaped).
    
    Single epoch-0 stream only; the zero-knowledge grant proposal is 1, so
    multi-stream is resize-only (otp-4b-2). Session-owned orchestration lives
    in transfer_session/data_plane.rs, reusing blit-core primitives
    (DataPlaneSession, execute_receive_pipeline, execute_sink_pipeline_
    streaming, dial_data_plane) — no call into remote::push or the daemon
    push service (those drivers die at otp-10).
    
    A/B parity vs old push over the data plane holds byte-identically.
    Suite 1509 -> 1511. [state: skip]
    
    Finding: .review/findings/otp-4b-data-plane.md
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>

diff --git a/crates/blit-core/src/transfer_session/data_plane.rs b/crates/blit-core/src/transfer_session/data_plane.rs
new file mode 100644
index 0000000..3ccde10
--- /dev/null
+++ b/crates/blit-core/src/transfer_session/data_plane.rs
@@ -0,0 +1,339 @@
+//! Session-side TCP data-plane orchestration (otp-4b).
+//!
+//! The unified session reuses blit-core's data-plane byte plumbing —
+//! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
+//! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
+//! its OWN choreography here. The push-specific bind/arm/accept loop
+//! (`blit-daemon` push service) and the multi-stream send driver
+//! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
+//! deletes at cutover (otp-10), so nothing in this file calls into them.
+//!
+//! otp-4b-1 scope: a single epoch-0 stream, no resize. The RESPONDER
+//! (whichever end is DESTINATION for otp-4/-5) binds a listener, mints
+//! the tokens, grants them in `SessionAccept`, and accepts + receives;
+//! the INITIATOR (SOURCE here) dials + authenticates + sends. Because
+//! the grant is issued before any manifest is seen,
+//! [`initial_stream_proposal`] with zero knowledge is 1 — the session
+//! data plane always starts single-stream and grows only via
+//! SOURCE-driven resize, which lands at otp-4b-2.
+
+use std::path::PathBuf;
+use std::sync::Arc;
+
+use eyre::Result;
+use tokio::io::AsyncReadExt;
+use tokio::net::{TcpListener, TcpStream};
+use tokio::sync::mpsc;
+use tokio::task::JoinSet;
+
+use crate::buffer::BufferPool;
+use crate::engine::{
+    initial_stream_proposal, local_receiver_capacity, DIAL_FLOOR_CHUNK_BYTES, DIAL_FLOOR_PREFETCH,
+};
+use crate::generated::{session_error::Code, DataPlaneGrant};
+use crate::remote::transfer::payload::TransferPayload;
+use crate::remote::transfer::pipeline::execute_receive_pipeline;
+use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
+use crate::remote::transfer::socket::{
+    configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
+};
+use crate::remote::transfer::source::TransferSource;
+use crate::remote::transfer::{
+    execute_sink_pipeline_streaming, generate_sub_token, AbortOnDrop, DataPlaneSession,
+};
+
+use super::SessionFault;
+
+/// Dial values for the session data plane. otp-4b-1 has no live dial
+/// tuner, so it runs at the engine floor — the conservative start the
+/// dial contract mandates (absent/0 capacity fields ⇒ conservative,
+/// never unlimited). A live dial + tuner is future work, not this slice.
+const SESSION_DP_CHUNK_BYTES: usize = DIAL_FLOOR_CHUNK_BYTES;
+const SESSION_DP_PREFETCH: usize = DIAL_FLOOR_PREFETCH;
+
+fn dp_fault(msg: impl Into<String>) -> eyre::Report {
+    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
+}
+
+// ---------------------------------------------------------------------------
+// Responder (DESTINATION) — bind, grant, accept, receive
+// ---------------------------------------------------------------------------
+
+/// A bound data-plane listener plus the credentials the responder
+/// advertises in its `SessionAccept`. Held by the responder driver
+/// across the handshake so the accept loop can run after establish.
+pub(super) struct ResponderDataPlane {
+    listener: TcpListener,
+    session_token: Vec<u8>,
+    epoch0_sub_token: Vec<u8>,
+    initial_streams: u32,
+    port: u16,
+}
+
+/// Bind a data-plane listener and mint credentials for the grant. Any
+/// failure (bind, addr, RNG) logs and returns `None` — the caller then
+/// issues a grant-less `SessionAccept` and the session falls back to the
+/// in-stream carrier (contract §Transport selection: a responder that
+/// cannot bind grants no data plane).
+pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
+    let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
+        Ok(listener) => listener,
+        Err(err) => {
+            log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
+            return None;
+        }
+    };
+    let port = match listener.local_addr() {
+        Ok(addr) => addr.port(),
+        Err(err) => {
+            log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
+            return None;
+        }
+    };
+    // Two independent 16-byte credentials (contract §Transport: a socket
+    // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
+    // is the fallible-RNG minter — a missing system RNG is an error, not
+    // a weaker credential.
+    let session_token = match generate_sub_token() {
+        Ok(token) => token,
+        Err(err) => {
+            log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
+            return None;
+        }
+    };
+    let epoch0_sub_token = match generate_sub_token() {
+        Ok(token) => token,
+        Err(err) => {
+            log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
+            return None;
+        }
+    };
+    // The grant is issued before any manifest is seen, so the proposal
+    // has zero knowledge: initial_streams == 1. All growth is via resize
+    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
+    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
+    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
+    Some(ResponderDataPlane {
+        listener,
+        session_token,
+        epoch0_sub_token,
+        initial_streams,
+        port,
+    })
+}
+
+impl ResponderDataPlane {
+    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
+    pub(super) fn grant(&self) -> DataPlaneGrant {
+        DataPlaneGrant {
+            tcp_port: self.port as u32,
+            session_token: self.session_token.clone(),
+            initial_streams: self.initial_streams,
+            epoch0_sub_token: self.epoch0_sub_token.clone(),
+        }
+    }
+
+    /// Accept exactly `initial_streams` authenticated data sockets and
+    /// drain each into `sink` via the shared receive pipeline, returning
+    /// the aggregated write outcome (the DESTINATION is the scorer). The
+    /// caller runs this concurrently with the control-stream diff loop
+    /// and joins it on `SourceDone`.
+    pub(super) async fn accept_and_receive(
+        self,
+        sink: Arc<dyn TransferSink>,
+    ) -> Result<SinkOutcome> {
+        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
+        let mut expected = self.session_token.clone();
+        expected.extend_from_slice(&self.epoch0_sub_token);
+
+        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
+        for _ in 0..self.initial_streams {
+            let mut socket = accept_authenticated(&self.listener, &expected).await?;
+            let sink = Arc::clone(&sink);
+            receives.spawn(async move { execute_receive_pipeline(&mut socket, sink, None).await });
+        }
+
+        let mut total = SinkOutcome::default();
+        while let Some(joined) = receives.join_next().await {
+            let outcome =
+                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
+            total.files_written += outcome.files_written;
+            total.bytes_written += outcome.bytes_written;
+        }
+        Ok(total)
+    }
+}
+
+/// Accept one data socket under the shared bounded-accept timeout, apply
+/// the data-plane socket policy, read the fixed-length credential under
+/// the shared bounded-read timeout, and verify it. A socket presenting
+/// anything else is a `DATA_PLANE_FAILED` fault (contract §Transport: a
+/// mismatched socket is closed without response — here the whole session
+/// faults, since otp-4b-1 arms exactly the sockets it dials).
+async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
+    let accept = tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await;
+    let socket = match accept {
+        Ok(Ok((socket, _peer))) => socket,
+        Ok(Err(err)) => return Err(dp_fault(format!("data-plane accept failed: {err}"))),
+        Err(_) => {
+            return Err(dp_fault(format!(
+            "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
+        )))
+        }
+    };
+    configure_data_socket(&socket, None)
+        .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
+
+    let mut socket = socket;
+    let mut buf = vec![0u8; expected.len()];
+    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
+    match read {
+        Ok(Ok(_)) => {}
+        Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
+        Err(_) => {
+            return Err(dp_fault(format!(
+                "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
+            )))
+        }
+    }
+    // Constant-time comparison is not required: the tokens are 16 random
+    // bytes read once per socket, single-session; a timing oracle buys
+    // nothing against per-transfer secrets (same posture as the old push
+    // acceptor's `token == expected_token`).
+    if buf != expected {
+        return Err(dp_fault(
+            "data-plane socket presented an invalid credential",
+        ));
+    }
+    Ok(socket)
+}
+
+// ---------------------------------------------------------------------------
+// Initiator (SOURCE) — dial, authenticate, send
+// ---------------------------------------------------------------------------
+
+/// A running source-side data plane: the dialed socket(s) wrapped as a
+/// sink pipeline. Planned payloads are fed via [`Self::queue`]; closing
+/// via [`Self::finish`] drains the pipeline, emits each socket's END
+/// record, and returns the bytes this end sent.
+pub(super) struct SourceDataPlane {
+    payload_tx: Option<mpsc::Sender<TransferPayload>>,
+    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
+    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
+    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
+}
+
+/// Dial the granted data plane and start the send pipeline. `host` is
+/// the responder's host (the initiator connected the control plane to
+/// it; the data plane rides the same host on the granted port —
+/// contract §Transport: the initiator always dials).
+pub(super) async fn dial_source_data_plane(
+    host: &str,
+    grant: &DataPlaneGrant,
+    source: Arc<dyn TransferSource>,
+) -> Result<SourceDataPlane> {
+    let streams = grant.initial_streams.max(1) as usize;
+    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
+    let mut handshake = grant.session_token.clone();
+    handshake.extend_from_slice(&grant.epoch0_sub_token);
+
+    let pool = Arc::new(BufferPool::for_data_plane(SESSION_DP_CHUNK_BYTES, streams));
+    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(streams);
+    for _ in 0..streams {
+        let session = DataPlaneSession::connect(
+            host,
+            grant.tcp_port,
+            &handshake,
+            SESSION_DP_CHUNK_BYTES,
+            SESSION_DP_PREFETCH,
+            false,
+            None,
+            Arc::clone(&pool),
+        )
+        .await
+        .map_err(|err| dp_fault(format!("dialing session data plane: {err:#}")))?;
+        // The source-side sink never reads its dst_root (it only sends);
+        // `root()` is consulted by the relay/receive case, not here.
+        sinks.push(Arc::new(DataPlaneSink::new(
+            session,
+            Arc::clone(&source),
+            PathBuf::new(),
+        )));
+    }
+
+    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(SESSION_DP_PREFETCH.max(1));
+    // Bounded by AbortOnDrop: a fault on the control lane that drops the
+    // SourceDataPlane aborts the pipeline task instead of leaking it.
+    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
+        execute_sink_pipeline_streaming(source, sinks, payload_rx, SESSION_DP_PREFETCH, None).await
+    }));
+    Ok(SourceDataPlane {
+        payload_tx: Some(payload_tx),
+        pipeline: Some(pipeline),
+    })
+}
+
+impl SourceDataPlane {
+    /// Feed one planned batch into the send pipeline. The pipeline
+    /// prepares each payload (tar-shard/file) and writes it through the
+    /// data-plane record framing across the live socket(s).
+    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
+        let tx = self.payload_tx.as_ref().ok_or_else(|| {
+            eyre::Report::new(SessionFault::internal("data plane already finished"))
+        })?;
+        for payload in payloads {
+            tx.send(payload).await.map_err(|_| {
+                dp_fault("data-plane send pipeline closed before all payloads sent")
+            })?;
+        }
+        Ok(())
+    }
+
+    /// Signal end-of-stream, drain the pipeline (each worker emits its
+    /// socket's END record on drain), and return the bytes sent. Must be
+    /// awaited before `SourceDone` goes out so the destination's receive
+    /// pipeline sees END and completes.
+    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
+        // Drop the sender: workers observe the closed queue, drain what
+        // is left, then `finish()` (END record) and exit.
+        self.payload_tx = None;
+        let pipeline = self
+            .pipeline
+            .take()
+            .expect("SourceDataPlane::finish called once");
+        pipeline
+            .join()
+            .await
+            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))?
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::remote::transfer::SUB_TOKEN_LEN;
+
+    /// The otp-4b-1 grant invariant: the responder always grants a
+    /// single epoch-0 stream (the zero-knowledge proposal — no manifest
+    /// has been seen when SessionAccept goes out) with two independent
+    /// 16-byte credentials on a real port. Multi-stream is resize-only
+    /// (otp-4b-2).
+    #[tokio::test]
+    async fn responder_grant_is_single_stream_with_16_byte_tokens() {
+        let rdp = prepare_responder_data_plane()
+            .await
+            .expect("bind loopback data plane");
+        let grant = rdp.grant();
+        assert_eq!(
+            grant.initial_streams, 1,
+            "zero-knowledge grant starts single-stream (otp-4b-1)"
+        );
+        assert_eq!(grant.session_token.len(), SUB_TOKEN_LEN);
+        assert_eq!(grant.epoch0_sub_token.len(), SUB_TOKEN_LEN);
+        assert_ne!(
+            grant.session_token, grant.epoch0_sub_token,
+            "session token and epoch-0 sub-token are independent credentials"
+        );
+        assert_ne!(grant.tcp_port, 0, "a real ephemeral port is granted");
+    }
+}

exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/TRANSFER_SESSION.md" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'git show --find-renames --find-copies --unified=80 --no-ext-diff 881d412 -- crates/blit-core/src/transfer_session/mod.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
# TransferSession wire + session contract (otp-1)

**Status**: Active (contract; implementation lands otp-3..otp-10)
**Created**: 2026-07-05
**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4)
**Decision refs**: D-2026-07-05-1 (one path), D-2026-07-05-2
(same-build only), D-2026-06-20-1/-2 (bounded-unilateral dial)

This document is the authoritative contract for the single `Transfer`
RPC that replaces `Push` and `PullSync` at cutover (otp-10). Proto
truth lives in `proto/blit.proto` under "ONE_TRANSFER_PATH unified
session"; this doc explains the state machine the proto cannot.

## Invariants

1. **One vocabulary, role-tagged.** Both wire directions carry the
   same frame type (`TransferFrame`). Which frames an end may send is
   determined by its ROLE (SOURCE or DESTINATION), never by whether
   it is the gRPC client or server. This is the structural form of
   the owner's invariant: there is no push-shaped or pull-shaped
   message set to diverge.
2. **Same build only (D-2026-07-05-2).** The first frame each way is
   `SessionHello{build_id, contract_version}`. Both ends compare for
   EXACT equality; any mismatch → `SessionError{BUILD_MISMATCH}`
   naming both ids, then stream close. No negotiate-down, no advisory
   fields, no feature-capability bits — same build implies same
   features. `build_id` = `<crate version>+<git commit hash>`
   composed at compile time; `contract_version` is a belt-and-braces
   integer bumped on any wire-shape change (exact match required).
   Imprecise identities never false-match (otp-3 codex F1): a dirty
   tree composes `<sha>.dirty.<content hash>` (deterministic — only
   byte-identical dirty trees match), and a build without git
   identity composes `unknown.<per-compilation entropy>` (only the
   selfsame binary matches itself).
3. **Roles.** The initiator (the end that opened the RPC — a CLI
   client, or a daemon acting as delegated initiator) declares in
   `SessionOpen` whether it is SOURCE or DESTINATION; the responder
   (always a daemon) takes the other role. All four
   initiator/role combinations run the identical state machine.
4. **Diff owner = DESTINATION, always.** SOURCE streams its manifest
   from live enumeration (immediate start — no buffered-enumeration
   phase in any direction). DESTINATION diffs incrementally against
   its own filesystem and streams need batches back. DESTINATION is
   authoritative for what it has; SOURCE is authoritative for what
   exists to send.
5. **Dial contract carries (D-2026-06-20-1/-2).** The byte RECEIVER
   (whichever end holds DESTINATION) advertises its
   `CapacityProfile` at session open — in `SessionOpen` when the
   initiator is DESTINATION, in `SessionAccept` when the responder
   is. The byte SENDER (SOURCE) owns the live dial bounded by that
   profile. Absent/0 profile fields mean "unknown hardware value" —
   conservative defaults, never unlimited, and NEVER "old peer"
   (there are no old peers).
6. **One stream policy.** The data plane opens at the dial floor
   immediately; SOURCE shape-corrects the stream count upward via
   resize as the need list accumulates (the sf-2 mechanism —
   `TransferDial::propose_shape_resize` — now the only policy).
   SOURCE is the resize controller in every session.

## Phase state machine

```
INITIATOR                                RESPONDER
  |-- SessionHello ----------------------->|   (phase: HELLO)
  |<------------------------ SessionHello--|
  |     both verify build_id exact match; mismatch => SessionError + close
  |-- SessionOpen ------------------------>|   (phase: OPEN)
  |<---------------------- SessionAccept --|
  |     responder validates module/path/read-only/gate here;
  |     refusal is a SessionError, never a silent close
  |                                        |
  |==== from here the lanes are ROLES, not initiator/responder ====|
  |  (whichever end holds SOURCE sends source-lane frames,          |
  |   regardless of which end opened the RPC)                       |
  |                                                                 |
  |  SOURCE streams:  ManifestEntry* ... ManifestComplete          |
  |  DEST streams:    NeedBatch* ... NeedComplete                  |
  |  SOURCE streams:  payload (data plane sockets, or in-stream    |
  |                   frames when the in-stream carrier is chosen) |
  |  SOURCE resize:   ResizeRequest -> DEST ResizeAck (per epoch)  |
  |                                                                 |
  |  resume exception (RELIABLE): a NeedBatch entry flagged         |
  |  `resume=true` is followed by DEST's BlockHashList for that     |
  |  file BEFORE SOURCE may send any byte of that file; stale or    |
  |  mismatched partials fall back to full-file transfer.           |
  |                                                                 |
  |  mirror: DEST computes deletions LOCALLY from the completed     |
  |  source manifest (filter-scoped, scan-complete-guarded) and     |
  |  executes them itself. No delete list crosses the wire.         |
  |                                                                 |
  |  CLOSING (role-directed, both initiator layouts):               |
  |    SOURCE -> DEST:  SourceDone (all requested payloads flushed) |
  |    DEST -> SOURCE:  TransferSummary (DEST is the scorer)        |
  |  then the INITIATOR closes the RPC stream.                      |
```

- Phase violations (a frame arriving in a phase where its role may
  not send it) are `SessionError{PROTOCOL_VIOLATION}` + close —
  fail-fast, no tolerant parsing.
- `NeedComplete` is DESTINATION's promise that no further need
  batches follow (SOURCE may finish after flushing what was asked).
  It may be sent only after BOTH: the source's `ManifestComplete`
  has been received AND the destination has finished diffing every
  received manifest entry. Mirror deletions additionally require the
  scan-complete guard, as above.
- **Flow control is the transport's, deliberately:** manifest, need,
  and in-stream payload frames ride gRPC/HTTP-2 stream flow control;
  each end holds only bounded internal queues (the engine's existing
  batching — 128-entry manifest check chunks, need-list batcher).
  Nothing in the contract requires unbounded buffering of the peer's
  stream, and implementations must not introduce it.
- `TransferSummary` always travels DESTINATION → SOURCE (the end
  that wrote bytes and executed deletes is the end that can attest
  to them), then the initiator surfaces it to the operator.

## Frame set and field numbers

`rpc Transfer(stream TransferFrame) returns (stream TransferFrame)`

`TransferFrame.frame` oneof (field numbers frozen by this doc):

| # | frame | sender | phase |
|---|-------|--------|-------|
| 1 | `SessionHello` | both, first frame | HELLO |
| 2 | `SessionOpen` | initiator | OPEN |
| 3 | `SessionAccept` | responder | OPEN |
| 4 | `FileHeader manifest_entry` | SOURCE | streaming |
| 5 | `ManifestComplete manifest_complete` | SOURCE | streaming |
| 6 | `NeedBatch need_batch` | DESTINATION | streaming |
| 7 | `NeedComplete need_complete` | DESTINATION | streaming |
| 8 | `BlockHashList block_hashes` | DESTINATION | resume, per flagged file |
| 9 | `FileHeader file_begin` | SOURCE | in-stream carrier |
| 10 | `FileData file_data` | SOURCE | in-stream carrier |
| 11 | `TarShardHeader tar_shard_header` | SOURCE | in-stream carrier |
| 12 | `TarShardChunk tar_shard_chunk` | SOURCE | in-stream carrier |
| 13 | `TarShardComplete tar_shard_complete` | SOURCE | in-stream carrier |
| 14 | `BlockTransfer block` | SOURCE | resume |
| 15 | `BlockTransferComplete block_complete` | SOURCE | resume |
| 16 | `DataPlaneResize resize` | SOURCE | any (post-accept) |
| 17 | `DataPlaneResizeAck resize_ack` | DESTINATION | any (post-accept) |
| 18 | `SourceDone source_done` | SOURCE | closing |
| 19 | `TransferSummary summary` | DESTINATION | closing |
| 20 | `SessionError error` | both | any |

Reused messages (`FileHeader`, `FileData`, `TarShard*`,
`BlockTransfer*`, `BlockHashList`, `ManifestComplete`,
`DataPlaneResize`/`Ack`, `FilterSpec`, `ComparisonMode`,
`MirrorMode`, `ResumeSettings`, `CapacityProfile`) keep their
existing shapes — the session reuses the engine's payload vocabulary
verbatim. New messages (`SessionHello`, `SessionOpen`,
`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
`NeedComplete`, `SourceDone`, `TransferSummary`, `SessionError`) are
defined in the proto with their field numbers.

Deliberately absent: `PeerCapabilities` (same build = same
features), `spec_version` negotiation (the hello's exact match
replaces it), any delete list (mirror is destination-local), any
push/pull-specific message.

## Transport selection

- **TCP data plane (default):** the RESPONDER binds the listener and
  issues `DataPlaneGrant{tcp_port, session_token, initial_streams,
  epoch0_sub_token}` inside `SessionAccept`; the INITIATOR always
  dials (NAT/firewall reality — connection topology, not
  choreography). Byte direction on the sockets is set by role:
  SOURCE writes, DESTINATION reads.
  **`initial_streams` is an ACCEPT ceiling, not a dial order**
  (D-2026-06-20-1/-2 preserved): it is the number of epoch-0 accept
  slots the responder arms, computed as min(engine dial floor,
  DESTINATION's capacity ceiling). SOURCE — wherever it sits — owns
  the dial and may use fewer epoch-0 sockets than armed; unclaimed
  slots expire harmlessly. Growth beyond epoch 0 happens only via
  SOURCE-initiated resize (sf-2 shape correction / tuner), one armed
  accept per ADD epoch, exactly as ue-r2-2 built.
  **Socket auth, exact:** every epoch-0 socket opens with
  `session_token` (16 bytes) immediately followed by
  `epoch0_sub_token` (16 bytes); every resize-ADD socket opens with
  `session_token` followed by that epoch's `sub_token` from the
  `DataPlaneResize` frame. Tokens are single-session; each armed
  accept slot admits exactly one socket (no replay within a
  session); armed slots that go unclaimed expire, as today's resize
  wiring already does. A socket presenting anything else is closed
  without response.
- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
  (operator `--force-grpc` diagnostics) or granted by the responder
  when it cannot bind a data plane (`SessionAccept` with no grant).
  Payload frames 9-15 ride the RPC itself. Same choreography, same
  planner decisions, different byte carrier.
  **Record grammar (fail-fast):** payload records on the
  source-lane are STRICTLY SERIALIZED — after `file_begin(header)`,
  only `file_data` frames for that file may follow on the lane until
  the record completes; completion is inferred at exactly
  `header.size` cumulative bytes (a `file_begin`/`tar_shard_header`/
  `block` arriving early, or bytes overrunning `size`, is
  `PROTOCOL_VIOLATION`). Tar-shard records run
  `tar_shard_header … tar_shard_chunk* … tar_shard_complete`; block
  records complete with `block_complete`. Payload records may begin
  only AFTER the source's `ManifestComplete` — this per-transport
  ordering rule applies identically to both roles and mirrors the
  design-4-proven fallback ordering, so manifest frames and payload
  records never interleave. DESTINATION-lane frames (need batches,
  acks, summary) are unaffected — they travel the other direction.
- **Local (in-process):** the identical session state machine runs
  with both roles in one process over an in-process frame channel —
  no RPC, no sockets (otp-11). Strategy selection (tar-shard vs
  file vs block) is planner-owned and reads workload shape +
  capability, never role/initiator/transport.

## Errors, cancel, stall

- `SessionError{code, message}` codes (plus both build ids on
  BUILD_MISMATCH):
  `BUILD_MISMATCH`, `MODULE_UNKNOWN`, `READ_ONLY`,
  `DELEGATION_REFUSED`, `SCAN_INCOMPLETE`, `PROTOCOL_VIOLATION`,
  `DATA_PLANE_FAILED`, `CANCELLED`, `INTERNAL`. An end that refuses
  or aborts says why before closing; operators never diagnose from a
  bare stream reset.
- `CancelJob` interop: the responder registers the session in
  `ActiveJobs` at OPEN (same transfer_id contract as today); the
  cancel token races the session exactly as w4-3 wired, and the
  peer receives `SessionError{CANCELLED}`.
- StallGuard, byte-accounting, and progress events (w6-1 contract)
  attach at the same boundaries they do today; the session emits the
  existing `DaemonEvent` payloads.

## What this replaces

At cutover (otp-10): `Push`, `PullSync`, and their message
choreographies are deleted from the proto and the tree; the four
per-direction drivers die with them; `DelegatedPull` shrinks to
trigger + progress relay (no payload bytes). Until then the old paths
stay live alongside the session as migration scaffolding.

Progress: otp-3 landed the role-parameterized drivers over the
in-process transport; **otp-4a** made the daemon serve `Transfer` for
real (runs `run_destination` as Responder; a client `run_source`s as
SOURCE initiator over gRPC, in-stream carrier) — the RPC no longer
returns `UNIMPLEMENTED`. The TCP data plane grant + resize land at
otp-4b; the daemon-as-SOURCE (pull-equivalent) layout at otp-5.

 succeeded in 0ms:
commit 881d412b77512fd1d1e28c5932653b63e2cbf41a
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 22:54:28 2026 -0400

    otp-4b-1: TCP data plane onto the session (single stream)
    
    Port the TCP data plane onto the unified transfer session, replacing the
    in-stream carrier as the default. The DESTINATION responder binds a
    listener, mints session_token + epoch0_sub_token, and grants them in
    SessionAccept; the SOURCE initiator dials + authenticates the socket and
    sends payloads over the shared DataPlaneSession record framing while the
    control stream carries manifest/needs/summary. In-stream stays live as
    the requested fallback (--force-grpc-shaped).
    
    Single epoch-0 stream only; the zero-knowledge grant proposal is 1, so
    multi-stream is resize-only (otp-4b-2). Session-owned orchestration lives
    in transfer_session/data_plane.rs, reusing blit-core primitives
    (DataPlaneSession, execute_receive_pipeline, execute_sink_pipeline_
    streaming, dial_data_plane) — no call into remote::push or the daemon
    push service (those drivers die at otp-10).
    
    A/B parity vs old push over the data plane holds byte-identically.
    Suite 1509 -> 1511. [state: skip]
    
    Finding: .review/findings/otp-4b-data-plane.md
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>

diff --git a/crates/blit-core/src/transfer_session/mod.rs b/crates/blit-core/src/transfer_session/mod.rs
index b2fb190..56b9ab2 100644
--- a/crates/blit-core/src/transfer_session/mod.rs
+++ b/crates/blit-core/src/transfer_session/mod.rs
@@ -1,197 +1,205 @@
 //! Unified transfer session — the ONE block of transfer code
 //! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
 //!
 //! A transfer has a SOURCE role and a DESTINATION role; which end
 //! initiated and which CLI verb was used select roles, never code.
 //! Both roles run the drivers below over a [`transport::FrameTransport`];
 //! the wire contract they implement — phases, frame table, record
 //! grammar, error semantics — is `docs/TRANSFER_SESSION.md` (otp-1).
 //!
 //! otp-3 scope: the role-parameterized state machine over the existing
 //! engine with the in-process transport and the in-stream byte
 //! carrier. The TCP data plane, daemon serving, ActiveJobs/cancel and
 //! progress wiring land at otp-4; mirror otp-6; resume otp-7;
 //! delegated otp-9 (see the slice list in the plan).
 
+mod data_plane;
 pub mod transport;
 
 use std::collections::{HashMap, HashSet};
 use std::fmt;
 use std::future::Future;
 use std::path::{Path, PathBuf};
 use std::pin::Pin;
 use std::sync::atomic::{AtomicBool, Ordering};
 use std::sync::{Arc, Mutex as StdMutex};
 
 use eyre::Result;
 use tokio::io::{AsyncReadExt, AsyncWriteExt};
 use tokio::sync::mpsc;
 
 use crate::generated::transfer_frame::Frame;
 use crate::generated::{
     session_error, ComparisonMode, FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch,
     NeedComplete, NeedEntry, SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone,
     TarShardComplete, TarShardHeader, TransferFrame, TransferRole, TransferSummary,
 };
 use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
 use crate::remote::transfer::diff_planner;
 use crate::remote::transfer::payload::PreparedPayload;
 use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
 use crate::remote::transfer::source::TransferSource;
 use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
 use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
 use crate::transfer_plan::PlanOptions;
 use transport::{FrameRx, FrameTransport, FrameTx};
 
 /// Belt-and-braces wire-shape version, bumped on any change to the
 /// frame set or grammar. Exchanged (and exact-matched) in
 /// `SessionHello` alongside the build id (D-2026-07-05-2).
 pub const CONTRACT_VERSION: u32 = 1;
 
 /// Payload chunk size on the in-stream carrier. Same unit the gRPC
 /// control plane uses today; the data plane (otp-4) has its own.
 const IN_STREAM_CHUNK: usize = CONTROL_PLANE_CHUNK_SIZE;
 
 /// Manifest entries buffered per destination diff batch. Mirrors the
 /// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
 /// per-entry check is 2+ blocking syscalls, so it runs chunked on the
 /// blocking pool instead of inline per entry.
 const DEST_DIFF_CHUNK: usize = 128;
 
 /// Buffer of the in-memory pipe that feeds wire file-record bytes
 /// into `FsTransferSink::write_file_stream`. Bounds destination-side
 /// buffering per file record.
 const FILE_RECORD_PIPE_BYTES: usize = 256 * 1024;
 
 /// This build's session identity: `<crate version>+<git sha>[.dirty]`
 /// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
 /// "unknown" when git was unavailable at compile time.
 pub fn session_build_id() -> &'static str {
     concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
 }
 
 /// The identity this end presents in `SessionHello`. Defaults to the
 /// real compile-time identity; tests inject mismatches.
 #[derive(Debug, Clone)]
 pub struct HelloConfig {
     pub build_id: String,
     pub contract_version: u32,
 }
 
 impl Default for HelloConfig {
     fn default() -> Self {
         Self {
             build_id: session_build_id().to_string(),
             contract_version: CONTRACT_VERSION,
         }
     }
 }
 
 /// Which handshake part this end plays. Orthogonal to role: all four
 /// initiator/role combinations run the same state machine (contract
 /// §Invariants 3).
 pub enum SessionEndpoint {
     /// This end opened the transport; it sends `SessionOpen`.
     /// (Boxed: `SessionOpen` dwarfs the bare `Responder` variant.)
     Initiator { open: Box<SessionOpen> },
     /// This end answers `SessionOpen` with `SessionAccept`. Daemon
     /// module/path/read-only validation attaches here at otp-4.
     Responder,
 }
 
 impl SessionEndpoint {
     /// Convenience constructor so callers don't spell the `Box`.
     pub fn initiator(open: SessionOpen) -> Self {
         SessionEndpoint::Initiator {
             open: Box::new(open),
         }
     }
 }
 
 pub struct SourceSessionConfig {
     pub hello: HelloConfig,
     pub endpoint: SessionEndpoint,
     /// Engine planner knobs (tar/large/raw thresholds). Local to the
     /// source end — strategy selection is planner-owned and never
     /// crosses the wire (contract §Transport selection).
     pub plan_options: PlanOptions,
+    /// Host to dial the granted TCP data plane on (otp-4b). The
+    /// initiator connected the control plane to this host; the data
+    /// plane rides the same host on the granted port (contract
+    /// §Transport: the initiator always dials). `None` disables the
+    /// data plane at this end — a grant then faults, since the responder
+    /// is waiting to accept sockets that would never arrive.
+    pub data_plane_host: Option<String>,
 }
 
 pub struct DestinationSessionConfig {
     pub hello: HelloConfig,
     pub endpoint: SessionEndpoint,
 }
 
 /// A session-terminating fault: either end refusing, aborting, or
 /// catching the peer in a protocol violation. Carried as the error
 /// payload of the drivers' `eyre::Report`s — downcast to inspect the
 /// wire code.
 #[derive(Debug, Clone)]
 pub struct SessionFault {
     pub code: session_error::Code,
     pub message: String,
     /// Both build ids on BUILD_MISMATCH so the operator sees exactly
     /// which end is stale (contract §Errors).
     pub local_build_id: String,
     pub peer_build_id: String,
     /// True when the peer already knows about this fault — it sent
     /// the `SessionError` frame itself, or this end already emitted
     /// one. Drivers must not send another.
     pub peer_notified: bool,
 }
 
 impl SessionFault {
     fn new(code: session_error::Code, message: impl Into<String>) -> Self {
         Self {
             code,
             message: message.into(),
             local_build_id: String::new(),
             peer_build_id: String::new(),
             peer_notified: false,
         }
     }
 
     fn protocol_violation(message: impl Into<String>) -> Self {
         Self::new(session_error::Code::ProtocolViolation, message)
     }
 
     fn internal(message: impl Into<String>) -> Self {
         Self::new(session_error::Code::Internal, message)
     }
 
     fn read_only(message: impl Into<String>) -> Self {
         Self::new(session_error::Code::ReadOnly, message)
     }
 
     /// Public constructor for a caller-side refusal (e.g. the daemon's
     /// [`OpenResolver`] mapping a `tonic::Status` to a `SessionError`
     /// code). blit-core stays free of `tonic::Status`, so the caller
     /// picks the wire code.
     pub fn refusal(code: session_error::Code, message: impl Into<String>) -> Self {
         Self::new(code, message)
     }
 
     fn from_wire(err: SessionError) -> Self {
         Self {
             code: session_error::Code::try_from(err.code)
                 .unwrap_or(session_error::Code::SessionErrorUnspecified),
             message: err.message,
             // The peer reports its view: its "local" is our peer.
             local_build_id: err.peer_build_id,
             peer_build_id: err.local_build_id,
             peer_notified: true,
         }
     }
 
     fn to_wire(&self) -> SessionError {
         SessionError {
             code: self.code as i32,
             message: self.message.clone(),
             local_build_id: self.local_build_id.clone(),
             peer_build_id: self.peer_build_id.clone(),
         }
     }
 }
 
 impl fmt::Display for SessionFault {
     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
@@ -273,323 +281,343 @@ pub fn session_error_frame(code: session_error::Code, message: impl Into<String>
 /// Per-role capability check of the operation a `SessionOpen`
 /// describes. otp-3 refuses what later slices implement rather than
 /// silently ignoring it (fail-fast; contract §Errors).
 type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;
 
 /// The local endpoint a Responder resolves a received `SessionOpen`
 /// to. The daemon maps the wire module name + path here; a test can
 /// hand a fixed root with no module semantics via
 /// [`DestinationTarget::Fixed`] instead.
 #[derive(Debug, Clone)]
 pub struct ResolvedEndpoint {
     /// Absolute local root this end targets.
     pub root: PathBuf,
     /// Whether the resolved module forbids writes. A DESTINATION
     /// responder refuses `READ_ONLY`; a SOURCE responder (otp-5,
     /// daemon-send) does not care — reading a read-only module is fine.
     pub read_only: bool,
 }
 
 /// Async callback a Responder uses to turn a received (and
 /// capability-validated) `SessionOpen` into its local endpoint. It
 /// lives caller-side — the daemon resolves modules and maps its own
 /// `tonic::Status` errors to [`SessionFault`], so blit-core stays free
 /// of module/Status types. A returned fault (unknown module,
 /// containment failure) becomes a `SessionError` at OPEN, never a
 /// silent close (contract §Phase state machine).
 pub type OpenResolver = dyn Fn(
         &SessionOpen,
     )
         -> Pin<Box<dyn Future<Output = std::result::Result<ResolvedEndpoint, SessionFault>> + Send>>
     + Send
     + Sync;
 
 /// Where a DESTINATION driver writes. `Fixed` is a root known up front
 /// (an initiator's own local root, or a test's temp dir). `Resolve`
 /// defers to a caller callback that maps the received `SessionOpen` to
 /// a local root — the daemon path, where the root depends on the wire
 /// module name and so can only be resolved mid-handshake (after HELLO,
 /// before SessionAccept). A `Resolve` target is meaningful only on a
 /// Responder; an Initiator always knows its own root.
 pub enum DestinationTarget {
     Fixed(PathBuf),
     Resolve(Box<OpenResolver>),
 }
 
 fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
     if open.resume.as_ref().is_some_and(|r| r.enabled) {
         return Err(SessionFault::internal(
             "resume is not implemented on the unified session yet (otp-7)",
         ));
     }
     if open
         .filter
         .as_ref()
         .is_some_and(|f| *f != FilterSpec::default())
     {
         return Err(SessionFault::internal(
             "filters are not implemented on the unified session yet (otp-6)",
         ));
     }
     Ok(())
 }
 
 fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
     if open.mirror_enabled {
         return Err(SessionFault::internal(
             "mirror is not implemented on the unified session yet (otp-6)",
         ));
     }
     if open.resume.as_ref().is_some_and(|r| r.enabled) {
         return Err(SessionFault::internal(
             "resume is not implemented on the unified session yet (otp-7)",
         ));
     }
     Ok(())
 }
 
 /// Outcome of the HELLO + OPEN phases.
 struct Negotiated {
     open: SessionOpen,
-    #[allow(dead_code)] // capacity/grant consumed from otp-4b (data plane) on
+    /// The responder's reply. The SOURCE initiator reads
+    /// `accept.data_plane` to decide dial-vs-in-stream (otp-4b).
     accept: SessionAccept,
     /// The write root a Responder's [`OpenResolver`] produced from the
     /// received open, if one was supplied; `None` for an Initiator or a
     /// fixed-root Responder (the caller supplies the root then).
     resolved_root: Option<PathBuf>,
+    /// The bound data-plane listener + credentials a DESTINATION
+    /// Responder prepared before its `SessionAccept` (otp-4b). `None`
+    /// on an Initiator, or when the responder granted no data plane
+    /// (in-stream carrier). Consumed by the DESTINATION accept loop.
+    responder_data_plane: Option<data_plane::ResponderDataPlane>,
 }
 
 /// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
 /// scoping requirement). Sends the refusal `SessionError` itself when
 /// it detects the fault locally; returned faults are `peer_notified`.
 async fn establish(
     transport: &mut FrameTransport,
     hello: &HelloConfig,
     endpoint: &SessionEndpoint,
     local_role: TransferRole,
     validate_open: &OpenValidator,
     // Consulted only on the Responder branch, after the received open
     // passes `validate_open` and before SessionAccept. `None` = the
     // caller supplies the root itself (Initiator, or fixed-root test).
     resolve_open: Option<&OpenResolver>,
 ) -> Result<Negotiated> {
     // HELLO both ways, exact match (D-2026-07-05-2). First frame each
     // direction; no ordering between the two directions.
     transport
         .send(frame(Frame::Hello(SessionHello {
             build_id: hello.build_id.clone(),
             contract_version: hello.contract_version,
         })))
         .await?;
 
     let peer_hello = match expect_frame(transport).await? {
         Frame::Hello(h) => h,
         other => {
             return Err(notify_and_wrap(
                 transport,
                 SessionFault::protocol_violation(format!(
                     "expected SessionHello, got {}",
                     frame_name(&Some(other))
                 )),
             )
             .await)
         }
     };
 
     if peer_hello.build_id != hello.build_id
         || peer_hello.contract_version != hello.contract_version
     {
         let fault = SessionFault {
             code: session_error::Code::BuildMismatch,
             message: format!(
                 "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
                 hello.build_id, hello.contract_version,
                 peer_hello.build_id, peer_hello.contract_version,
             ),
             local_build_id: hello.build_id.clone(),
             peer_build_id: peer_hello.build_id.clone(),
             peer_notified: false,
         };
         return Err(notify_and_wrap(transport, fault).await);
     }
 
     match endpoint {
         SessionEndpoint::Initiator { open } => {
             let open = open.as_ref().clone();
             transport.send(frame(Frame::Open(open.clone()))).await?;
             let accept = match expect_frame(transport).await? {
                 Frame::Accept(a) => a,
                 other => {
                     return Err(notify_and_wrap(
                         transport,
                         SessionFault::protocol_violation(format!(
                             "expected SessionAccept, got {}",
                             frame_name(&Some(other))
                         )),
                     )
                     .await)
                 }
             };
             Ok(Negotiated {
                 open,
                 accept,
                 resolved_root: None,
+                responder_data_plane: None,
             })
         }
         SessionEndpoint::Responder => {
             let open = match expect_frame(transport).await? {
                 Frame::Open(o) => o,
                 other => {
                     return Err(notify_and_wrap(
                         transport,
                         SessionFault::protocol_violation(format!(
                             "expected SessionOpen, got {}",
                             frame_name(&Some(other))
                         )),
                     )
                     .await)
                 }
             };
             // The initiator declares ITS role; this responder end must
             // hold the complement.
             let declared =
                 TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
             if declared != complement(local_role) {
                 return Err(notify_and_wrap(
                     transport,
                     SessionFault::protocol_violation(format!(
                         "initiator declared role {} but this responder is {}",
                         declared.as_str_name(),
                         local_role.as_str_name()
                     )),
                 )
                 .await);
             }
             if let Err(fault) = validate_open(&open) {
                 // Refusal is a SessionError instead of SessionAccept,
                 // never a silent close (contract §Phase state machine).
                 return Err(notify_and_wrap(transport, fault).await);
             }
             // Responder endpoint resolution (otp-4): map the wire
             // module/path to a local root and enforce read-only, both
             // BEFORE SessionAccept so a refusal replaces the accept
             // (never follows it). The resolver is caller-supplied
             // (daemon module lookup); a fixed-root responder passes
             // None and resolves nothing here.
             let resolved_root = match resolve_open {
                 Some(resolve) => match resolve(&open).await {
                     Ok(resolved) => {
                         // A read-only module is fatal only for a
                         // DESTINATION (it would write); a SOURCE
                         // responder (otp-5, daemon-send) reads happily.
                         if local_role == TransferRole::Destination && resolved.read_only {
                             return Err(notify_and_wrap(
                                 transport,
                                 SessionFault::read_only(
                                     "destination module is read-only".to_string(),
                                 ),
                             )
                             .await);
                         }
                         Some(resolved.root)
                     }
                     Err(fault) => return Err(notify_and_wrap(transport, fault).await),
                 },
                 None => None,
             };
+            // Data plane (otp-4b): a DESTINATION responder binds a TCP
+            // listener and grants it, unless the initiator requested the
+            // in-stream carrier or the bind fails (grant-less accept ⇒
+            // in-stream fallback). A SOURCE responder (otp-5,
+            // daemon-send) will bind on its own branch later; otp-4b's
+            // responder is always the DESTINATION.
+            let responder_data_plane =
+                if local_role == TransferRole::Destination && !open.in_stream_bytes {
+                    data_plane::prepare_responder_data_plane().await
+                } else {
+                    None
+                };
             let accept = SessionAccept {
                 // The byte RECEIVER advertises capacity at session
                 // open (D-2026-06-20-1/-2); consumed by the dial when
                 // the data plane lands (otp-4b).
                 receiver_capacity: if local_role == TransferRole::Destination {
                     Some(crate::engine::local_receiver_capacity())
                 } else {
                     None
                 },
-                // No grant = in-stream byte carrier, otp-4a's only one.
-                data_plane: None,
+                // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
+                data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
             };
             transport.send(frame(Frame::Accept(accept.clone()))).await?;
             Ok(Negotiated {
                 open,
                 accept,
                 resolved_root,
+                responder_data_plane,
             })
         }
     }
 }
 
 /// Receive one frame during establish; peer errors and closes become
 /// terminal faults.
 async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
     match transport.recv().await? {
         Some(TransferFrame {
             frame: Some(Frame::Error(err)),
         }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
         Some(TransferFrame { frame: Some(f) }) => Ok(f),
         Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
             SessionFault::protocol_violation("frame with empty oneof"),
         )),
         None => Err(eyre::Report::new(SessionFault::internal(
             "peer closed during session establish",
         ))),
     }
 }
 
 /// Send the fault to the peer (best effort), mark it notified, and
 /// wrap it for return.
 async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
     let _ = transport.send(error_frame(&fault)).await;
     fault.peer_notified = true;
     eyre::Report::new(fault)
 }
 
 // ---------------------------------------------------------------------------
 // SOURCE driver
 // ---------------------------------------------------------------------------
 
 /// Events the source's receive half forwards to its send half. The
 /// channel is unbounded but bounded by construction: every `Need`
 /// consumes a distinct sent-manifest entry (unknown or repeated paths
 /// fault the session), so the queue never exceeds the source's own
 /// manifest size — the contract's bounded-buffering rule holds.
 enum SourceEvent {
     Need(FileHeader),
     NeedComplete,
     Summary(TransferSummary),
     Fault(SessionFault),
 }
 
 /// Run the SOURCE role of one transfer session over `transport`.
 /// Returns the destination-computed `TransferSummary` (contract: the
 /// end that wrote the bytes is the end that attests to them).
 pub async fn run_source(
     cfg: SourceSessionConfig,
     transport: FrameTransport,
     source: Arc<dyn TransferSource>,
 ) -> Result<TransferSummary> {
     let mut transport = transport;
     if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
         // Own-config coherence: a source initiator declares SOURCE.
         let declared = TransferRole::try_from(open.initiator_role);
         if declared != Ok(TransferRole::Source) {
             eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
         }
         if let Err(fault) = source_open_validator(open) {
             eyre::bail!("run_source initiator config unsupported: {fault}");
         }
     }
 
     let negotiated = establish(
         &mut transport,
         &cfg.hello,
         &cfg.endpoint,
         TransferRole::Source,
         &source_open_validator,
         // A SOURCE responder's endpoint resolution (module→root for a
         // daemon-send) lands with otp-5; otp-4a's daemon is always the
         // DESTINATION responder, so the source never resolves here.
         None,
     )
     .await?;
 
     let (mut tx, rx) = transport.split();
@@ -657,216 +685,259 @@ async fn source_recv_half(
                     "transport receive failed: {err:#}"
                 ))));
                 return;
             }
         };
         match received.frame {
             Some(Frame::NeedBatch(batch)) => {
                 for entry in batch.entries {
                     if entry.resume {
                         let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                             format!(
                                 "resume-flagged need for '{}' in a session opened without resume",
                                 entry.relative_path
                             ),
                         )));
                         return;
                     }
                     let header = sent
                         .lock()
                         .expect("sent-manifest lock poisoned")
                         .remove(&entry.relative_path);
                     match header {
                         Some(h) => {
                             let _ = events.send(SourceEvent::Need(h));
                         }
                         None => {
                             let _ = events.send(SourceEvent::Fault(
                                 SessionFault::protocol_violation(format!(
                                     "need for unknown or already-needed path '{}'",
                                     entry.relative_path
                                 )),
                             ));
                             return;
                         }
                     }
                 }
             }
             Some(Frame::NeedComplete(_)) => {
                 if !manifest_sent.load(Ordering::Acquire) {
                     // Fail fast at arrival time (otp-3 codex F2): the
                     // event queue would otherwise let an early
                     // NeedComplete be processed late and pass as
                     // legitimate.
                     let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                         "NeedComplete before the source's ManifestComplete",
                     )));
                     return;
                 }
                 let _ = events.send(SourceEvent::NeedComplete);
             }
             Some(Frame::Summary(summary)) => {
                 let _ = events.send(SourceEvent::Summary(summary));
                 return;
             }
             Some(Frame::Error(err)) => {
                 let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
                 return;
             }
             other => {
                 let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
                     format!("{} on the source's receive lane", frame_name(&other)),
                 )));
                 return;
             }
         }
     }
 }
 
 async fn source_send_half(
     cfg: &SourceSessionConfig,
     negotiated: &Negotiated,
     tx: &mut Box<dyn FrameTx>,
     source: Arc<dyn TransferSource>,
     sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
     manifest_sent: &AtomicBool,
     mut events: mpsc::UnboundedReceiver<SourceEvent>,
 ) -> Result<TransferSummary> {
     let mut pending: Vec<FileHeader> = Vec::new();
     let mut need_complete = false;
 
+    // Data plane (otp-4b): dial the granted TCP sockets up front —
+    // BEFORE streaming the manifest — so the destination's accept loop
+    // (armed the moment it sent SessionAccept) sees the connections
+    // promptly rather than waiting out its bounded-accept timeout while
+    // a long manifest streams. The sockets sit idle (keepalive covers
+    // that) until payloads are queued below. `None` = the in-stream
+    // carrier (fallback), which needs no early setup.
+    let mut data_plane = match &negotiated.accept.data_plane {
+        Some(grant) => {
+            let host = cfg.data_plane_host.as_deref().ok_or_else(|| {
+                eyre::Report::new(SessionFault::internal(
+                    "responder granted a TCP data plane but this initiator has no host to dial",
+                ))
+            })?;
+            Some(data_plane::dial_source_data_plane(host, grant, Arc::clone(&source)).await?)
+        }
+        None => None,
+    };
+
     // Streaming manifest: entries go out as enumeration produces them
     // (immediate start in every direction — plan §Design 2). The open
     // carries no source path: the source end owns its local endpoint.
     let _ = &negotiated.open;
     let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
     let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
     while let Some(header) = header_rx.recv().await {
         sent.lock()
             .expect("sent-manifest lock poisoned")
             .insert(header.relative_path.clone(), header.clone());
         tx.send(frame(Frame::ManifestEntry(header))).await?;
         // Faults detected by the receive half abort the stream now,
         // not after the full scan; needs just accumulate.
         drain_source_events(&mut events, &mut pending, &mut need_complete)?;
     }
     let scanned = scan_handle
         .await
         .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
     let scan_complete = unreadable
         .lock()
         .expect("unreadable list lock poisoned")
         .is_empty();
     log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
     tx.send(frame(Frame::ManifestComplete(ManifestComplete {
         scan_complete,
     })))
     .await?;
     manifest_sent.store(true, Ordering::Release);
 
-    // Payload phase. In-stream record grammar: payload records only
-    // after ManifestComplete, strictly serialized per record
-    // (contract §Transport selection). Needs accumulated while a
-    // record batch was being sent become the next planner batch.
-    let mut read_buf = vec![0u8; IN_STREAM_CHUNK];
+    // Payload phase. The byte carrier is either the TCP data plane
+    // (dialed above) or the in-stream record grammar (fallback). Needs
+    // accumulated while a batch was being sent become the next planner
+    // batch (contract §Transport selection); payloads only flow after
+    // ManifestComplete.
+    // The in-stream carrier reuses one read buffer across records; the
+    // data plane owns its own pooled buffers, so skip that allocation.
+    let mut read_buf = if data_plane.is_none() {
+        vec![0u8; IN_STREAM_CHUNK]
+    } else {
+        Vec::new()
+    };
     loop {
         drain_source_events(&mut events, &mut pending, &mut need_complete)?;
         if !pending.is_empty() {
             let batch = std::mem::take(&mut pending);
-            send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf).await?;
+            match &mut data_plane {
+                Some(dp) => {
+                    let payloads =
+                        diff_planner::plan_push_payloads(batch, source.root(), cfg.plan_options)?;
+                    dp.queue(payloads).await?;
+                }
+                None => {
+                    send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf)
+                        .await?;
+                }
+            }
             continue;
         }
         if need_complete {
             break;
         }
         match events.recv().await {
             Some(event) => {
                 handle_source_event(event, &mut pending, &mut need_complete)?;
             }
             None => {
                 return Err(eyre::Report::new(SessionFault::internal(
                     "source receive half ended before NeedComplete",
                 )))
             }
         }
     }
 
+    // Close the data plane BEFORE SourceDone so the destination's receive
+    // pipeline sees each socket's END record and completes; SourceDone on
+    // the control lane then lets the destination score and summarize.
+    if let Some(dp) = data_plane.take() {
+        dp.finish().await?;
+    }
+
     tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
 
     // CLOSING: the destination is the scorer; the next event must be
     // its summary (the receive half ends after forwarding it).
     match events.recv().await {
         Some(SourceEvent::Summary(summary)) => Ok(summary),
         Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
         Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
             format!("need for '{}' after NeedComplete", h.relative_path),
         ))),
         Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
             SessionFault::protocol_violation("duplicate NeedComplete"),
         )),
         None => Err(eyre::Report::new(SessionFault::internal(
             "source receive half ended before TransferSummary",
         ))),
     }
 }
 
 fn drain_source_events(
     events: &mut mpsc::UnboundedReceiver<SourceEvent>,
     pending: &mut Vec<FileHeader>,
     need_complete: &mut bool,
 ) -> Result<()> {
     while let Ok(event) = events.try_recv() {
         handle_source_event(event, pending, need_complete)?;
     }
     Ok(())
 }
 
 fn handle_source_event(
     event: SourceEvent,
     pending: &mut Vec<FileHeader>,
     need_complete: &mut bool,
 ) -> Result<()> {
     match event {
         SourceEvent::Need(header) => {
             if *need_complete {
                 return Err(eyre::Report::new(SessionFault::protocol_violation(
                     format!("need for '{}' after NeedComplete", header.relative_path),
                 )));
             }
             pending.push(header);
             Ok(())
         }
         SourceEvent::NeedComplete => {
             if *need_complete {
                 return Err(eyre::Report::new(SessionFault::protocol_violation(
                     "duplicate NeedComplete",
                 )));
             }
             *need_complete = true;
             Ok(())
         }
         SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
             "TransferSummary before SourceDone",
         ))),
         SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
     }
 }
 
 /// Plan one batch of needed headers with the engine planner and emit
 /// the resulting payload records per the in-stream grammar.
 async fn send_payload_records(
     tx: &mut Box<dyn FrameTx>,
     source: &Arc<dyn TransferSource>,
     plan_options: PlanOptions,
     batch: Vec<FileHeader>,
     read_buf: &mut [u8],
 ) -> Result<()> {
     let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
     for payload in payloads {
         match source.prepare_payload(payload).await? {
             PreparedPayload::File(header) => {
                 tx.send(frame(Frame::FileBegin(header.clone()))).await?;
                 if header.size == 0 {
                     continue; // record complete at 0 cumulative bytes
                 }
                 let mut reader = source.open_file(&header).await?;
                 let mut remaining = header.size;
@@ -926,319 +997,374 @@ async fn send_payload_records(
 pub struct DestinationOutcome {
     /// The summary this end computed and sent (contract: DESTINATION
     /// is the scorer).
     pub summary: TransferSummary,
     /// Paths this end put on the need list, in emission order. The
     /// role suite pins these identical across role assignments — the
     /// executable form of the owner's invariance requirement.
     pub needed_paths: Vec<String>,
 }
 
 /// Run the DESTINATION role of one transfer session over `transport`,
 /// writing under the root named by `target`. Diffs the streamed
 /// manifest against its own filesystem (the destination is the one
 /// diff owner — plan §Design 3), returns the summary it computed and
 /// sent.
 ///
 /// `target` is [`DestinationTarget::Fixed`] when the root is known up
 /// front (an Initiator's own local root, or a test), or
 /// [`DestinationTarget::Resolve`] when the root must be resolved from
 /// the received `SessionOpen` mid-handshake (the daemon Responder,
 /// where the wire module name selects the root).
 pub async fn run_destination(
     cfg: DestinationSessionConfig,
     transport: FrameTransport,
     target: DestinationTarget,
 ) -> Result<DestinationOutcome> {
     let mut transport = transport;
     let endpoint = match cfg.endpoint {
         SessionEndpoint::Initiator { mut open } => {
             let declared = TransferRole::try_from(open.initiator_role);
             if declared != Ok(TransferRole::Destination) {
                 eyre::bail!(
                     "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
                 );
             }
             if let Err(fault) = destination_open_validator(&open) {
                 eyre::bail!("run_destination initiator config unsupported: {fault}");
             }
             // Dial contract: the byte receiver advertises capacity in
             // its open when it is the initiator (contract §Invariants 5).
             if open.receiver_capacity.is_none() {
                 open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
             }
             SessionEndpoint::Initiator { open }
         }
         SessionEndpoint::Responder => SessionEndpoint::Responder,
     };
 
     let resolve_open: Option<&OpenResolver> = match &target {
         DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
         DestinationTarget::Fixed(_) => None,
     };
 
     let negotiated = establish(
         &mut transport,
         &cfg.hello,
         &endpoint,
         TransferRole::Destination,
         &destination_open_validator,
         resolve_open,
     )
     .await?;
 
     // The resolver's root (Responder + Resolve) wins; otherwise the
     // caller-supplied Fixed root.
     let dst_root = match negotiated.resolved_root.clone() {
         Some(root) => root,
         None => match &target {
             DestinationTarget::Fixed(root) => root.clone(),
             // Unreachable: a Resolve target always yields a root on the
             // Responder branch, and establish only skips resolution on
             // the Initiator branch (which pairs with a Fixed root).
             DestinationTarget::Resolve(_) => {
                 return Err(eyre::Report::new(SessionFault::internal(
                     "resolver target produced no destination root",
                 )));
             }
         },
     };
 
-    match destination_session(&mut transport, &negotiated, &dst_root).await {
+    match destination_session(&mut transport, negotiated, &dst_root).await {
         Ok(outcome) => Ok(outcome),
         Err(report) => {
             let mut fault = fault_from_report(report);
             if !fault.peer_notified {
                 let _ = transport.send(error_frame(&fault)).await;
                 fault.peer_notified = true;
             }
             Err(eyre::Report::new(fault))
         }
     }
 }
 
 fn violation(message: String) -> eyre::Report {
     eyre::Report::new(SessionFault::protocol_violation(message))
 }
 
 async fn destination_session(
     transport: &mut FrameTransport,
-    negotiated: &Negotiated,
+    negotiated: Negotiated,
     dst_root: &Path,
 ) -> Result<DestinationOutcome> {
     let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
         .unwrap_or(ComparisonMode::Unspecified);
     let compare_opts = CompareOptions {
         mode: compare_mode.into(),
         ignore_existing: negotiated.open.ignore_existing,
         include_deletions: false, // mirror lands at otp-6
     };
     // src_root is only consumed by local File payloads, which never
     // occur on a session destination (payload bytes arrive as records
-    // and go through the stream/tar write paths).
-    let sink = FsTransferSink::new(
+    // and go through the stream/tar write paths). `Arc` so the data-plane
+    // receive task (otp-4b) can share the one sink across sockets.
+    let sink = Arc::new(FsTransferSink::new(
         PathBuf::new(),
         dst_root.to_path_buf(),
         FsSinkConfig {
             preserve_times: true,
             dry_run: false,
             checksum: None,
             resume: false,
             compare_mode,
         },
-    );
+    ));
     // Same canonical-containment chokepoint the sink write paths use
     // (R46-F3), applied to diff stats so a hostile manifest path can't
     // make the destination stat outside its root.
     let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
 
+    // Data plane (otp-4b): when the responder granted a TCP data plane,
+    // payload bytes arrive on sockets (not the control lane). Arm the
+    // accept+receive task NOW — concurrent with the diff loop below, and
+    // before the source dials — so the connections are accepted promptly.
+    // AbortOnDrop bounds it to this future: a control-lane fault that
+    // returns from this fn aborts the receive task instead of leaking it.
+    let mut data_plane_recv = negotiated.responder_data_plane.map(|rdp| {
+        let sink: Arc<dyn TransferSink> = Arc::clone(&sink) as Arc<dyn TransferSink>;
+        AbortOnDrop::new(tokio::spawn(rdp.accept_and_receive(sink)))
+    });
+
     let mut pending: Vec<FileHeader> = Vec::new();
     let mut outstanding: HashSet<String> = HashSet::new();
     let mut needed_paths: Vec<String> = Vec::new();
     let mut manifest_complete = false;
     let mut files_written: u64 = 0;
     let mut bytes_written: u64 = 0;
 
     loop {
         let received = match transport.recv().await? {
             Some(f) => f,
             None => {
                 return Err(eyre::Report::new(SessionFault::internal(
                     "peer closed mid-session",
                 )))
             }
         };
         match received.frame {
             Some(Frame::ManifestEntry(header)) => {
                 if manifest_complete {
                     return Err(violation(format!(
                         "manifest entry '{}' after ManifestComplete",
                         header.relative_path
                     )));
                 }
                 pending.push(header);
                 if pending.len() >= DEST_DIFF_CHUNK {
                     let chunk = std::mem::take(&mut pending);
                     diff_chunk_and_send_needs(
                         transport,
                         chunk,
                         dst_root,
                         canonical_dst_root.as_deref(),
                         &compare_opts,
                         &mut outstanding,
                         &mut needed_paths,
                     )
                     .await?;
                 }
             }
             Some(Frame::ManifestComplete(_complete)) => {
                 if manifest_complete {
                     return Err(violation("duplicate ManifestComplete".into()));
                 }
                 // (scan_complete gates mirror purges from otp-6 on;
                 // nothing consumes it in otp-3.)
                 let chunk = std::mem::take(&mut pending);
                 diff_chunk_and_send_needs(
                     transport,
                     chunk,
                     dst_root,
                     canonical_dst_root.as_deref(),
                     &compare_opts,
                     &mut outstanding,
                     &mut needed_paths,
                 )
                 .await?;
                 // NeedComplete only after ManifestComplete received
                 // AND every entry diffed — both true here.
                 transport
                     .send(frame(Frame::NeedComplete(NeedComplete {})))
                     .await?;
                 manifest_complete = true;
             }
             Some(Frame::FileBegin(header)) => {
+                // Payload records ride the control lane only under the
+                // in-stream carrier; with a TCP data plane active they
+                // flow over the sockets, so one here is a violation.
+                if data_plane_recv.is_some() {
+                    return Err(violation(format!(
+                        "file record '{}' on the control lane while a TCP data plane is active",
+                        header.relative_path
+                    )));
+                }
                 if !manifest_complete {
                     return Err(violation(format!(
                         "payload record for '{}' before ManifestComplete",
                         header.relative_path
                     )));
                 }
                 if !outstanding.remove(&header.relative_path) {
                     return Err(violation(format!(
                         "payload for '{}' which is not on the need list",
                         header.relative_path
                     )));
                 }
                 let outcome = receive_file_record(transport, &sink, &header).await?;
                 files_written += outcome.files_written as u64;
                 bytes_written += outcome.bytes_written;
             }
             Some(Frame::TarShardHeader(shard)) => {
+                if data_plane_recv.is_some() {
+                    return Err(violation(
+                        "tar shard record on the control lane while a TCP data plane is active"
+                            .into(),
+                    ));
+                }
                 if !manifest_complete {
                     return Err(violation("tar shard record before ManifestComplete".into()));
                 }
                 for h in &shard.files {
                     if !outstanding.remove(&h.relative_path) {
                         return Err(violation(format!(
                             "tar shard entry '{}' which is not on the need list",
                             h.relative_path
                         )));
                     }
                 }
                 let outcome = receive_tar_record(transport, &sink, shard).await?;
                 files_written += outcome.files_written as u64;
                 bytes_written += outcome.bytes_written;
             }
             Some(Frame::SourceDone(_)) => {
                 if !manifest_complete {
                     return Err(violation("SourceDone before ManifestComplete".into()));
                 }
-                if !outstanding.is_empty() {
-                    return Err(violation(format!(
-                        "SourceDone with {} needed file(s) never sent",
-                        outstanding.len()
-                    )));
-                }
+                // Carrier-specific completion. In-stream: every payload
+                // was consumed inline, so the need set must be fully
+                // drained. Data plane: payloads rode the sockets (the
+                // control lane never removed them from `outstanding`), so
+                // join the receive task for the authoritative counts and
+                // verify it delivered exactly the need list.
+                let in_stream_carrier_used = match data_plane_recv.take() {
+                    Some(recv) => {
+                        let outcome = recv.join().await.map_err(|err| {
+                            eyre::Report::new(SessionFault::internal(format!(
+                                "data-plane receive task panicked: {err}"
+                            )))
+                        })??;
+                        files_written = outcome.files_written as u64;
+                        bytes_written = outcome.bytes_written;
+                        if files_written != needed_paths.len() as u64 {
+                            return Err(violation(format!(
+                                "data plane delivered {} of {} needed file(s) before SourceDone",
+                                files_written,
+                                needed_paths.len()
+                            )));
+                        }
+                        false
+                    }
+                    None => {
+                        if !outstanding.is_empty() {
+                            return Err(violation(format!(
+                                "SourceDone with {} needed file(s) never sent",
+                                outstanding.len()
+                            )));
+                        }
+                        true
+                    }
+                };
                 let summary = TransferSummary {
                     files_transferred: files_written,
                     bytes_transferred: bytes_written,
                     entries_deleted: 0, // mirror lands at otp-6
-                    in_stream_carrier_used: true,
+                    in_stream_carrier_used,
                     files_resumed: 0, // resume lands at otp-7
                 };
                 transport.send(frame(Frame::Summary(summary))).await?;
                 return Ok(DestinationOutcome {
                     summary,
                     needed_paths,
                 });
             }
             Some(Frame::Error(err)) => {
                 return Err(eyre::Report::new(SessionFault::from_wire(err)));
             }
             other => {
                 // Everything else is off-lane or off-phase here:
                 // destination-lane frames echoed back, resume frames
                 // in a non-resume session (otp-7), resize with no
                 // data plane to resize (otp-4), stray handshake
                 // frames, bare FileData/TarShardChunk outside a
                 // record. Fail fast, no tolerant parsing.
                 return Err(violation(format!(
                     "{} not valid on the destination's receive lane in this phase",
                     frame_name(&other)
                 )));
             }
         }
     }
 }
 
 /// Stat-and-compare one chunk of manifest entries on the blocking
 /// pool (2+ syscalls per entry — same rationale as the daemon's
 /// w4-4 chunked checks), then stream the resulting need batch.
 async fn diff_chunk_and_send_needs(
     transport: &mut FrameTransport,
     chunk: Vec<FileHeader>,
     dst_root: &Path,
     canonical_dst_root: Option<&Path>,
     compare_opts: &CompareOptions,
     outstanding: &mut HashSet<String>,
     needed_paths: &mut Vec<String>,
 ) -> Result<()> {
     if chunk.is_empty() {
         return Ok(());
     }
     let dst_root = dst_root.to_path_buf();
     let canonical = canonical_dst_root.map(Path::to_path_buf);
     let opts = compare_opts.clone();
     let needed: Vec<String> = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
         let mut needed = Vec::new();
         for header in &chunk {
             if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
                 needed.push(header.relative_path.clone());
             }
         }
         Ok(needed)
     })
     .await
     .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;
 
     let entries: Vec<NeedEntry> = needed
         .into_iter()
         // A path the source manifests twice is diffed twice but
         // needed at most once.
         .filter(|path| outstanding.insert(path.clone()))
         .map(|relative_path| {
             needed_paths.push(relative_path.clone());
             NeedEntry {
                 relative_path,
                 resume: false, // resume lands at otp-7
             }
         })
         .collect();
     if entries.is_empty() {
         return Ok(());
     }
     transport
         .send(frame(Frame::NeedBatch(NeedBatch { entries })))
         .await?;
     Ok(())
 }
 
 /// Does the destination need this manifest entry? Stats its own file

exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/STATE.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-05 (**otp-4a landed + graded** — the daemon now
SERVES the unified `Transfer` RPC and a client pushes through it over
gRPC, byte-identical to old push; ONE_TRANSFER_PATH otp-1 + otp-3 +
otp-4a `[x]`, current slice otp-4b. SMALL_FILE_CEILING stays paused,
D-2026-07-05-1.)
**Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
local on top, unpushed — windows-latest CI check rides the next push.

Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Now (active work)

- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
  D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
  invariant (plan doc, verbatim): ONE block of transfer code;
  direction/initiator/verb can NEVER affect wall time by blit's doing
  — impossible by construction because the per-direction drivers and
  `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
  cells. **D-2026-07-05-2: same-build peers only, refusal at session
  open.** Progress (each through the codex loop):
  - **otp-1 `[x]`** (`a3e2acb`+`f861579`) — wire+session contract
    `docs/TRANSFER_SESSION.md`.
  - **otp-3 `[x]`** (`ef9ffa1`+`d5796a1`, codex 2/2) — role-param
    drivers over the in-process transport; the role suite pins
    identical need sets/summaries/byte-identical trees under both
    initiator layouts (the owner's invariance property, executable).
  - **otp-4a `[x]`** (`4b07bbb`+`25f538b`, codex 1/1) — daemon SERVES
    `Transfer` (runs `run_destination` as Responder, no longer
    UNIMPLEMENTED); client `run_source`s as SOURCE initiator over a
    gRPC `FrameTransport` (in-stream carrier); A/B parity byte-
    identical vs old push; SizeMtime = data-safe skip (owner-ack
    open question). Suite 1484 → **1509/0**.
  - Current: **otp-4b** — port the TCP data plane onto the session +
    resize + the sf-2 pin + the mid-transfer cancel e2e. (otp-2
    symmetric baseline is rig-gated; must land before otp-10.)
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+
  blocked** until ONE_TRANSFER_PATH ships, then resume/re-derive on
  the unified baseline. Principle stands: ceiling-driven, never
  competitor-relative (D-2026-07-04-4; a ≥25% margin answer was
  retracted — do not re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
- **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete +
  measurement gates DATA-COMPLETE (push/pull ≈ 9.5 of 9.88 Gbit/s,
  ue-1 band holds, no organic resize → ue-2 call; owner declarations
  pending in Blocked); 10 GbE session done; w9-3 + eleven review-queue
  rows landed. Details: DEVLOG 2026-07-04/05, commit map in REVIEW.md.
  Codex loop governs all code + plan changes (D-2026-07-04-1); REVIEW.md
  is the queue/status index.

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a
   `[x]`. Current: **otp-4b** — port the TCP data plane onto the
   session (responder binds + grants tcp_port/tokens in SessionAccept;
   source dials + authenticates; `maybe_shape_resize` controller on
   frames 16/17), port the sf-2 10k-file >1-stream pin to the session,
   add the deterministic mid-transfer cancel e2e. Then otp-5
   (daemon-as-SOURCE / pull-equivalent). otp-2 (symmetric baseline) is
   RIG-GATED — runs when the 10 GbE rig is available, must land before
   otp-10 cutover.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2,
   REV4 → Shipped (zero-copy resolved — D-2026-07-05-3). Optional
   owner-gated measurement follow-ups (Win 11 bare-metal datapoint;
   disk-path variants; >ARC-size push) — note the disk-path items
   are largely absorbed by otp-2/otp-12's symmetric-rig matrices. Env: bench
   binaries staged at `skippy:/mnt/generic-pool/video/blit-bin/`
   (/tmp and /home on skippy are noexec).
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
   open row; filed w6-2a/b/c + relay-1) — same directive; note w7-1
   (mirror-executor consolidation) likely lands for free inside
   otp-6's one-delete-rule slice; re-check before picking it up.
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + the aarch64-musl static build recipe: DEVLOG
   2026-07-05 10:00. **Standing owner safety rule**: ALL activity on
   rig `zoey` is confined to its `…/blit-temp/` folder — module roots,
   test data, everything; nothing written outside it, ever. Zero-copy
   is pre-authorized to be tested there when the post-cutover slice set
   reaches it; no daemon runs on zoey before then without a fresh go.
6. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
   `derive_local_plan_tuning` fold-or-retire; receive-side dial
   tuning residue (w3-1 scoped it out).

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**.
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
  (history only).
- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active) — the codex loop
  for **all code and plan changes** (D-2026-07-04-1); `.review/README.md`
  is retired as the grading mechanism (its `findings/`/`results/`
  records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
  rows) + `.review/findings/` + `.review/results/`.
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked
  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
  (gated on Round 1),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (all owner declarations; checkpoints are owner-only)

- **Three 10 GbE gate declarations**: ue-1 pass/fail (evidence: band
  holds), ue-2 pass/fail or re-scope (no organic resize at 10 GbE),
  REV4 → Shipped. (The zero-copy revisit verdict and the a/b/c
  question are RESOLVED — D-2026-07-05-3, unparked; measured skippy
  data 1.43 cores daemon-receive / 0.45 client at 9.5 Gbit/s stays
  recorded in DEVLOG + DIAGNOSIS.md.)
- **Push go**: local commits `f6e592e`..HEAD await the ref-listing +
  approval flow; windows-latest CI on the w9-3 harness fix rides it.
- `Cargo.lock`: dependency-refresh drift committed at `04c9c6d` (was
  unavoidable — blit-core gained `rand`); revert selectively if
  unwanted, otherwise settled.

## Open questions

- **(OPEN — owner ack requested, new 2026-07-05, otp-4a)** Unified
  SizeMtime semantic: old push and old pull DISAGREE on same-size +
  destination-NEWER — push re-transfers (clobbers the newer dest with
  older source), pull/session safely SKIP. The unified session adopts
  the **data-safe SKIP** (converge-up: pick the better direction;
  shared arm untouched so live pull_sync is unchanged; no test pinned
  push's clobber). This means the plan's "byte-identical trees vs old
  push" criterion is NOT literally achievable in that one cell —
  intentional. `--force` still overwrites. Agent rec: keep the safe
  skip (pinned by `same_size_newer_destination_is_skipped_not_clobbered`).
  Owner: confirm, or say you want old-push clobber as the unified
  default (a one-line compare change). Full reasoning:
  `.review/findings/otp-4-daemon-serves-transfer.md` compare section.
- **(OPEN)** Historical docs embed `/Users/...` paths — agent rec: leave.
- **(OPEN, new 2026-07-04)** `725aa07` tracked a 236-file stale
  worktree snapshot (`.claude/worktrees/vigilant-mayer/`, incl. a
  full `crates/` copy). Keep or `git rm -r`? Agent rec: remove;
  deletion awaits an owner go.
- **(OPEN, new 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still
  describe `determine_remote_tuning`/`TuningParams` (stale since
  ue-r2-1e, `TuningParams` now deleted) — fold into w10-docs-batch or
  rewrite sooner? Agent rec: w10.
- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: the 10 GbE
  session delivered the measurement evidence; flip awaits the three
  declarations in Blocked (was four — zero-copy resolved,
  D-2026-07-05-3).
- **(OPEN, new 2026-07-05)** CLI foot-gun found during the session:
  `blit copy src_large dst` with an existing local dir, no `./`,
  parses the bare name as an mDNS discovery endpoint and errors
  "remote source must include a module or root"
  (blit-app endpoints.rs). Should local-path existence win over the
  discovery interpretation, or at least improve the error? Candidate
  review-queue row; owner to slot.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
  locally across three sessions (clippy baseline + win-1 fixed). The
  daemon-spawn e2e load-flakiness is now root-caused and fixed on
  Linux (w9-3: port-TOCTOU wrong-daemon race + cargo-lock contention;
  claimed-port set + OnceLock build + child-death check). Remaining
  check: windows-latest CI on the next push (10d89e0 predates the
  w9-3 fix, so daemon-spawn flakes there would not be news).

## Handoff log (newest first, keep ≤ 3)

- **2026-07-05 (27th)** @ `fe4ad6d` — **otp-4a landed and graded**:
  daemon serves `Transfer` (runs `run_destination` as Responder — no
  longer UNIMPLEMENTED); client `run_source`s as SOURCE initiator over
  a gRPC `FrameTransport`; `run_destination` gained `DestinationTarget`
  + an async `OpenResolver` (daemon resolves module→root mid-handshake,
  before SessionAccept). Details: Now bullet 1, DEVLOG 21:30, finding
  doc. Codex FAIL 1/1 accepted+fixed (`4b07bbb`, fix `25f538b`): cancel
  emits a framed `SessionError{CANCELLED}` (guard proven by revert).
  A/B parity byte-identical vs old push. SizeMtime = safe-skip; **new
  owner-ack open question** logged. Suite 1501 → **1509/0**. In-flight:
  none. **Exact first action next session**: otp-4b (port the TCP data
  plane onto the session — grant in SessionAccept, source dials +
  auth, `maybe_shape_resize` on frames 16/17 — port the sf-2
  10k->1-stream pin, add the deterministic mid-transfer cancel e2e)
  through the codex loop. otp-2 stays rig-gated (before otp-10). Owner
  declarations: three 10 GbE gates + push go remain in Blocked.
- **2026-07-05 (26th)** @ `85bf611` — otp-3 landed and graded (details:
  DEVLOG 18:30, finding doc). Codex FAIL 2/2 accepted+fixed (`ef9ffa1`,
  fix `d5796a1`). Suite 1501/0.
- **2026-07-05 (25th)** @ `cb96e91`+records — plan Active
  (D-2026-07-05-4) + otp-1 landed/graded (`a3e2acb` → `f861579`,
  contract `docs/TRANSFER_SESSION.md`); D-2026-07-05-2 (same-build
  only); D-2026-07-05-3 (zero-copy unparked; zoey rig + musl recipe:
  queue item 5). Details: DEVLOG 2026-07-05 10:00.
- (older entries pruned — see DEVLOG 2026-07-05 06:45 and earlier)

exec
/usr/bin/zsh -lc 'rg -n "otp-4b|4b|DataPlaneGrant|session_token|epoch0_sub_token|single epoch|multi-stream|force-grpc-shaped" docs/plan/ONE_TRANSFER_PATH.md docs/TRANSFER_SESSION.md' in /home/michael/dev/Blit
 succeeded in 0ms:
docs/TRANSFER_SESSION.md:151:`SessionAccept`, `DataPlaneGrant`, `NeedBatch`/`NeedEntry`,
docs/TRANSFER_SESSION.md:163:  issues `DataPlaneGrant{tcp_port, session_token, initial_streams,
docs/TRANSFER_SESSION.md:164:  epoch0_sub_token}` inside `SessionAccept`; the INITIATOR always
docs/TRANSFER_SESSION.md:177:  `session_token` (16 bytes) immediately followed by
docs/TRANSFER_SESSION.md:178:  `epoch0_sub_token` (16 bytes); every resize-ADD socket opens with
docs/TRANSFER_SESSION.md:179:  `session_token` followed by that epoch's `sub_token` from the
docs/TRANSFER_SESSION.md:240:otp-4b; the daemon-as-SOURCE (pull-equivalent) layout at otp-5.

codex
One contract detail stands out for the review: `initial_streams` is documented as armed accept slots, and SOURCE may use fewer epoch-0 sockets. I’m checking whether the implementation accidentally treats it as an exact dial count or exact receive count, because that affects both concurrency and completion.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Unified transfer session — the ONE block of transfer code
     2	//! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
     3	//!
     4	//! A transfer has a SOURCE role and a DESTINATION role; which end
     5	//! initiated and which CLI verb was used select roles, never code.
     6	//! Both roles run the drivers below over a [`transport::FrameTransport`];
     7	//! the wire contract they implement — phases, frame table, record
     8	//! grammar, error semantics — is `docs/TRANSFER_SESSION.md` (otp-1).
     9	//!
    10	//! otp-3 scope: the role-parameterized state machine over the existing
    11	//! engine with the in-process transport and the in-stream byte
    12	//! carrier. The TCP data plane, daemon serving, ActiveJobs/cancel and
    13	//! progress wiring land at otp-4; mirror otp-6; resume otp-7;
    14	//! delegated otp-9 (see the slice list in the plan).
    15	
    16	mod data_plane;
    17	pub mod transport;
    18	
    19	use std::collections::{HashMap, HashSet};
    20	use std::fmt;
    21	use std::future::Future;
    22	use std::path::{Path, PathBuf};
    23	use std::pin::Pin;
    24	use std::sync::atomic::{AtomicBool, Ordering};
    25	use std::sync::{Arc, Mutex as StdMutex};
    26	
    27	use eyre::Result;
    28	use tokio::io::{AsyncReadExt, AsyncWriteExt};
    29	use tokio::sync::mpsc;
    30	
    31	use crate::generated::transfer_frame::Frame;
    32	use crate::generated::{
    33	    session_error, ComparisonMode, FileData, FileHeader, FilterSpec, ManifestComplete, NeedBatch,
    34	    NeedComplete, NeedEntry, SessionAccept, SessionError, SessionHello, SessionOpen, SourceDone,
    35	    TarShardComplete, TarShardHeader, TransferFrame, TransferRole, TransferSummary,
    36	};
    37	use crate::manifest::{header_transfer_status, CompareOptions, FileStatus};
    38	use crate::remote::transfer::diff_planner;
    39	use crate::remote::transfer::payload::PreparedPayload;
    40	use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
    41	use crate::remote::transfer::source::TransferSource;
    42	use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
    43	use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
    44	use crate::transfer_plan::PlanOptions;
    45	use transport::{FrameRx, FrameTransport, FrameTx};
    46	
    47	/// Belt-and-braces wire-shape version, bumped on any change to the
    48	/// frame set or grammar. Exchanged (and exact-matched) in
    49	/// `SessionHello` alongside the build id (D-2026-07-05-2).
    50	pub const CONTRACT_VERSION: u32 = 1;
    51	
    52	/// Payload chunk size on the in-stream carrier. Same unit the gRPC
    53	/// control plane uses today; the data plane (otp-4) has its own.
    54	const IN_STREAM_CHUNK: usize = CONTROL_PLANE_CHUNK_SIZE;
    55	
    56	/// Manifest entries buffered per destination diff batch. Mirrors the
    57	/// daemon push handler's `MANIFEST_CHECK_CHUNK` rationale (w4-4): the
    58	/// per-entry check is 2+ blocking syscalls, so it runs chunked on the
    59	/// blocking pool instead of inline per entry.
    60	const DEST_DIFF_CHUNK: usize = 128;
    61	
    62	/// Buffer of the in-memory pipe that feeds wire file-record bytes
    63	/// into `FsTransferSink::write_file_stream`. Bounds destination-side
    64	/// buffering per file record.
    65	const FILE_RECORD_PIPE_BYTES: usize = 256 * 1024;
    66	
    67	/// This build's session identity: `<crate version>+<git sha>[.dirty]`
    68	/// (contract §Invariants 2). `BLIT_GIT_SHA` is emitted by build.rs;
    69	/// "unknown" when git was unavailable at compile time.
    70	pub fn session_build_id() -> &'static str {
    71	    concat!(env!("CARGO_PKG_VERSION"), "+", env!("BLIT_GIT_SHA"))
    72	}
    73	
    74	/// The identity this end presents in `SessionHello`. Defaults to the
    75	/// real compile-time identity; tests inject mismatches.
    76	#[derive(Debug, Clone)]
    77	pub struct HelloConfig {
    78	    pub build_id: String,
    79	    pub contract_version: u32,
    80	}
    81	
    82	impl Default for HelloConfig {
    83	    fn default() -> Self {
    84	        Self {
    85	            build_id: session_build_id().to_string(),
    86	            contract_version: CONTRACT_VERSION,
    87	        }
    88	    }
    89	}
    90	
    91	/// Which handshake part this end plays. Orthogonal to role: all four
    92	/// initiator/role combinations run the same state machine (contract
    93	/// §Invariants 3).
    94	pub enum SessionEndpoint {
    95	    /// This end opened the transport; it sends `SessionOpen`.
    96	    /// (Boxed: `SessionOpen` dwarfs the bare `Responder` variant.)
    97	    Initiator { open: Box<SessionOpen> },
    98	    /// This end answers `SessionOpen` with `SessionAccept`. Daemon
    99	    /// module/path/read-only validation attaches here at otp-4.
   100	    Responder,
   101	}
   102	
   103	impl SessionEndpoint {
   104	    /// Convenience constructor so callers don't spell the `Box`.
   105	    pub fn initiator(open: SessionOpen) -> Self {
   106	        SessionEndpoint::Initiator {
   107	            open: Box::new(open),
   108	        }
   109	    }
   110	}
   111	
   112	pub struct SourceSessionConfig {
   113	    pub hello: HelloConfig,
   114	    pub endpoint: SessionEndpoint,
   115	    /// Engine planner knobs (tar/large/raw thresholds). Local to the
   116	    /// source end — strategy selection is planner-owned and never
   117	    /// crosses the wire (contract §Transport selection).
   118	    pub plan_options: PlanOptions,
   119	    /// Host to dial the granted TCP data plane on (otp-4b). The
   120	    /// initiator connected the control plane to this host; the data
   121	    /// plane rides the same host on the granted port (contract
   122	    /// §Transport: the initiator always dials). `None` disables the
   123	    /// data plane at this end — a grant then faults, since the responder
   124	    /// is waiting to accept sockets that would never arrive.
   125	    pub data_plane_host: Option<String>,
   126	}
   127	
   128	pub struct DestinationSessionConfig {
   129	    pub hello: HelloConfig,
   130	    pub endpoint: SessionEndpoint,
   131	}
   132	
   133	/// A session-terminating fault: either end refusing, aborting, or
   134	/// catching the peer in a protocol violation. Carried as the error
   135	/// payload of the drivers' `eyre::Report`s — downcast to inspect the
   136	/// wire code.
   137	#[derive(Debug, Clone)]
   138	pub struct SessionFault {
   139	    pub code: session_error::Code,
   140	    pub message: String,
   141	    /// Both build ids on BUILD_MISMATCH so the operator sees exactly
   142	    /// which end is stale (contract §Errors).
   143	    pub local_build_id: String,
   144	    pub peer_build_id: String,
   145	    /// True when the peer already knows about this fault — it sent
   146	    /// the `SessionError` frame itself, or this end already emitted
   147	    /// one. Drivers must not send another.
   148	    pub peer_notified: bool,
   149	}
   150	
   151	impl SessionFault {
   152	    fn new(code: session_error::Code, message: impl Into<String>) -> Self {
   153	        Self {
   154	            code,
   155	            message: message.into(),
   156	            local_build_id: String::new(),
   157	            peer_build_id: String::new(),
   158	            peer_notified: false,
   159	        }
   160	    }
   161	
   162	    fn protocol_violation(message: impl Into<String>) -> Self {
   163	        Self::new(session_error::Code::ProtocolViolation, message)
   164	    }
   165	
   166	    fn internal(message: impl Into<String>) -> Self {
   167	        Self::new(session_error::Code::Internal, message)
   168	    }
   169	
   170	    fn read_only(message: impl Into<String>) -> Self {
   171	        Self::new(session_error::Code::ReadOnly, message)
   172	    }
   173	
   174	    /// Public constructor for a caller-side refusal (e.g. the daemon's
   175	    /// [`OpenResolver`] mapping a `tonic::Status` to a `SessionError`
   176	    /// code). blit-core stays free of `tonic::Status`, so the caller
   177	    /// picks the wire code.
   178	    pub fn refusal(code: session_error::Code, message: impl Into<String>) -> Self {
   179	        Self::new(code, message)
   180	    }
   181	
   182	    fn from_wire(err: SessionError) -> Self {
   183	        Self {
   184	            code: session_error::Code::try_from(err.code)
   185	                .unwrap_or(session_error::Code::SessionErrorUnspecified),
   186	            message: err.message,
   187	            // The peer reports its view: its "local" is our peer.
   188	            local_build_id: err.peer_build_id,
   189	            peer_build_id: err.local_build_id,
   190	            peer_notified: true,
   191	        }
   192	    }
   193	
   194	    fn to_wire(&self) -> SessionError {
   195	        SessionError {
   196	            code: self.code as i32,
   197	            message: self.message.clone(),
   198	            local_build_id: self.local_build_id.clone(),
   199	            peer_build_id: self.peer_build_id.clone(),
   200	        }
   201	    }
   202	}
   203	
   204	impl fmt::Display for SessionFault {
   205	    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
   206	        write!(f, "session {}: {}", self.code.as_str_name(), self.message)
   207	    }
   208	}
   209	
   210	impl std::error::Error for SessionFault {}
   211	
   212	/// Downcast a driver-internal error back to its fault, wrapping
   213	/// non-fault failures (fs errors, planner errors, transport failures)
   214	/// as INTERNAL — an end that aborts says why before closing.
   215	fn fault_from_report(report: eyre::Report) -> SessionFault {
   216	    match report.downcast::<SessionFault>() {
   217	        Ok(fault) => fault,
   218	        Err(other) => SessionFault::internal(format!("{other:#}")),
   219	    }
   220	}
   221	
   222	fn frame(f: Frame) -> TransferFrame {
   223	    TransferFrame { frame: Some(f) }
   224	}
   225	
   226	fn error_frame(fault: &SessionFault) -> TransferFrame {
   227	    frame(Frame::Error(fault.to_wire()))
   228	}
   229	
   230	/// Short frame identifier for protocol-violation messages.
   231	fn frame_name(f: &Option<Frame>) -> &'static str {
   232	    match f {
   233	        Some(Frame::Hello(_)) => "SessionHello",
   234	        Some(Frame::Open(_)) => "SessionOpen",
   235	        Some(Frame::Accept(_)) => "SessionAccept",
   236	        Some(Frame::ManifestEntry(_)) => "ManifestEntry",
   237	        Some(Frame::ManifestComplete(_)) => "ManifestComplete",
   238	        Some(Frame::NeedBatch(_)) => "NeedBatch",
   239	        Some(Frame::NeedComplete(_)) => "NeedComplete",
   240	        Some(Frame::BlockHashes(_)) => "BlockHashList",
   241	        Some(Frame::FileBegin(_)) => "FileBegin",
   242	        Some(Frame::FileData(_)) => "FileData",
   243	        Some(Frame::TarShardHeader(_)) => "TarShardHeader",
   244	        Some(Frame::TarShardChunk(_)) => "TarShardChunk",
   245	        Some(Frame::TarShardComplete(_)) => "TarShardComplete",
   246	        Some(Frame::Block(_)) => "BlockTransfer",
   247	        Some(Frame::BlockComplete(_)) => "BlockTransferComplete",
   248	        Some(Frame::Resize(_)) => "DataPlaneResize",
   249	        Some(Frame::ResizeAck(_)) => "DataPlaneResizeAck",
   250	        Some(Frame::SourceDone(_)) => "SourceDone",
   251	        Some(Frame::Summary(_)) => "TransferSummary",
   252	        Some(Frame::Error(_)) => "SessionError",
   253	        None => "empty frame",
   254	    }
   255	}
   256	
   257	fn complement(role: TransferRole) -> TransferRole {
   258	    match role {
   259	        TransferRole::Source => TransferRole::Destination,
   260	        TransferRole::Destination => TransferRole::Source,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '620,1120p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   620	    )
   621	    .await?;
   622	
   623	    let (mut tx, rx) = transport.split();
   624	    let sent: Arc<StdMutex<HashMap<String, FileHeader>>> = Arc::default();
   625	    // Set by the send half the moment ManifestComplete goes out. On
   626	    // an ordered transport, a NeedComplete arriving while this is
   627	    // still false is provably premature — the peer cannot have
   628	    // received what we have not sent (contract: NeedComplete only
   629	    // after ManifestComplete received + all entries diffed).
   630	    let manifest_sent = Arc::new(AtomicBool::new(false));
   631	    let (event_tx, event_rx) = mpsc::unbounded_channel();
   632	    // AbortOnDrop: an early error return below must abort the receive
   633	    // half instead of leaking it (same rationale as design-2 / w4-1).
   634	    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
   635	        rx,
   636	        Arc::clone(&sent),
   637	        Arc::clone(&manifest_sent),
   638	        event_tx,
   639	    )));
   640	
   641	    match source_send_half(
   642	        &cfg,
   643	        &negotiated,
   644	        &mut tx,
   645	        source,
   646	        sent,
   647	        &manifest_sent,
   648	        event_rx,
   649	    )
   650	    .await
   651	    {
   652	        Ok(summary) => Ok(summary),
   653	        Err(report) => {
   654	            let mut fault = fault_from_report(report);
   655	            if !fault.peer_notified {
   656	                let _ = tx.send(error_frame(&fault)).await;
   657	                fault.peer_notified = true;
   658	            }
   659	            Err(eyre::Report::new(fault))
   660	        }
   661	    }
   662	}
   663	
   664	/// Receive half of the source driver: drains the transport for the
   665	/// whole session so destination sends can never deadlock against a
   666	/// blocked source send, and routes the destination lane to the send
   667	/// half. Terminates on summary, error, close, or violation.
   668	async fn source_recv_half(
   669	    mut rx: Box<dyn FrameRx>,
   670	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   671	    manifest_sent: Arc<AtomicBool>,
   672	    events: mpsc::UnboundedSender<SourceEvent>,
   673	) {
   674	    loop {
   675	        let received = match rx.recv().await {
   676	            Ok(Some(f)) => f,
   677	            Ok(None) => {
   678	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(
   679	                    "peer closed before TransferSummary",
   680	                )));
   681	                return;
   682	            }
   683	            Err(err) => {
   684	                let _ = events.send(SourceEvent::Fault(SessionFault::internal(format!(
   685	                    "transport receive failed: {err:#}"
   686	                ))));
   687	                return;
   688	            }
   689	        };
   690	        match received.frame {
   691	            Some(Frame::NeedBatch(batch)) => {
   692	                for entry in batch.entries {
   693	                    if entry.resume {
   694	                        let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   695	                            format!(
   696	                                "resume-flagged need for '{}' in a session opened without resume",
   697	                                entry.relative_path
   698	                            ),
   699	                        )));
   700	                        return;
   701	                    }
   702	                    let header = sent
   703	                        .lock()
   704	                        .expect("sent-manifest lock poisoned")
   705	                        .remove(&entry.relative_path);
   706	                    match header {
   707	                        Some(h) => {
   708	                            let _ = events.send(SourceEvent::Need(h));
   709	                        }
   710	                        None => {
   711	                            let _ = events.send(SourceEvent::Fault(
   712	                                SessionFault::protocol_violation(format!(
   713	                                    "need for unknown or already-needed path '{}'",
   714	                                    entry.relative_path
   715	                                )),
   716	                            ));
   717	                            return;
   718	                        }
   719	                    }
   720	                }
   721	            }
   722	            Some(Frame::NeedComplete(_)) => {
   723	                if !manifest_sent.load(Ordering::Acquire) {
   724	                    // Fail fast at arrival time (otp-3 codex F2): the
   725	                    // event queue would otherwise let an early
   726	                    // NeedComplete be processed late and pass as
   727	                    // legitimate.
   728	                    let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   729	                        "NeedComplete before the source's ManifestComplete",
   730	                    )));
   731	                    return;
   732	                }
   733	                let _ = events.send(SourceEvent::NeedComplete);
   734	            }
   735	            Some(Frame::Summary(summary)) => {
   736	                let _ = events.send(SourceEvent::Summary(summary));
   737	                return;
   738	            }
   739	            Some(Frame::Error(err)) => {
   740	                let _ = events.send(SourceEvent::Fault(SessionFault::from_wire(err)));
   741	                return;
   742	            }
   743	            other => {
   744	                let _ = events.send(SourceEvent::Fault(SessionFault::protocol_violation(
   745	                    format!("{} on the source's receive lane", frame_name(&other)),
   746	                )));
   747	                return;
   748	            }
   749	        }
   750	    }
   751	}
   752	
   753	async fn source_send_half(
   754	    cfg: &SourceSessionConfig,
   755	    negotiated: &Negotiated,
   756	    tx: &mut Box<dyn FrameTx>,
   757	    source: Arc<dyn TransferSource>,
   758	    sent: Arc<StdMutex<HashMap<String, FileHeader>>>,
   759	    manifest_sent: &AtomicBool,
   760	    mut events: mpsc::UnboundedReceiver<SourceEvent>,
   761	) -> Result<TransferSummary> {
   762	    let mut pending: Vec<FileHeader> = Vec::new();
   763	    let mut need_complete = false;
   764	
   765	    // Data plane (otp-4b): dial the granted TCP sockets up front —
   766	    // BEFORE streaming the manifest — so the destination's accept loop
   767	    // (armed the moment it sent SessionAccept) sees the connections
   768	    // promptly rather than waiting out its bounded-accept timeout while
   769	    // a long manifest streams. The sockets sit idle (keepalive covers
   770	    // that) until payloads are queued below. `None` = the in-stream
   771	    // carrier (fallback), which needs no early setup.
   772	    let mut data_plane = match &negotiated.accept.data_plane {
   773	        Some(grant) => {
   774	            let host = cfg.data_plane_host.as_deref().ok_or_else(|| {
   775	                eyre::Report::new(SessionFault::internal(
   776	                    "responder granted a TCP data plane but this initiator has no host to dial",
   777	                ))
   778	            })?;
   779	            Some(data_plane::dial_source_data_plane(host, grant, Arc::clone(&source)).await?)
   780	        }
   781	        None => None,
   782	    };
   783	
   784	    // Streaming manifest: entries go out as enumeration produces them
   785	    // (immediate start in every direction — plan §Design 2). The open
   786	    // carries no source path: the source end owns its local endpoint.
   787	    let _ = &negotiated.open;
   788	    let unreadable: Arc<StdMutex<Vec<String>>> = Arc::default();
   789	    let (mut header_rx, scan_handle) = source.scan(None, Arc::clone(&unreadable));
   790	    while let Some(header) = header_rx.recv().await {
   791	        sent.lock()
   792	            .expect("sent-manifest lock poisoned")
   793	            .insert(header.relative_path.clone(), header.clone());
   794	        tx.send(frame(Frame::ManifestEntry(header))).await?;
   795	        // Faults detected by the receive half abort the stream now,
   796	        // not after the full scan; needs just accumulate.
   797	        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
   798	    }
   799	    let scanned = scan_handle
   800	        .await
   801	        .map_err(|err| eyre::eyre!("manifest scan task panicked: {err}"))??;
   802	    let scan_complete = unreadable
   803	        .lock()
   804	        .expect("unreadable list lock poisoned")
   805	        .is_empty();
   806	    log::debug!("session source manifest complete: {scanned} entries, complete={scan_complete}");
   807	    tx.send(frame(Frame::ManifestComplete(ManifestComplete {
   808	        scan_complete,
   809	    })))
   810	    .await?;
   811	    manifest_sent.store(true, Ordering::Release);
   812	
   813	    // Payload phase. The byte carrier is either the TCP data plane
   814	    // (dialed above) or the in-stream record grammar (fallback). Needs
   815	    // accumulated while a batch was being sent become the next planner
   816	    // batch (contract §Transport selection); payloads only flow after
   817	    // ManifestComplete.
   818	    // The in-stream carrier reuses one read buffer across records; the
   819	    // data plane owns its own pooled buffers, so skip that allocation.
   820	    let mut read_buf = if data_plane.is_none() {
   821	        vec![0u8; IN_STREAM_CHUNK]
   822	    } else {
   823	        Vec::new()
   824	    };
   825	    loop {
   826	        drain_source_events(&mut events, &mut pending, &mut need_complete)?;
   827	        if !pending.is_empty() {
   828	            let batch = std::mem::take(&mut pending);
   829	            match &mut data_plane {
   830	                Some(dp) => {
   831	                    let payloads =
   832	                        diff_planner::plan_push_payloads(batch, source.root(), cfg.plan_options)?;
   833	                    dp.queue(payloads).await?;
   834	                }
   835	                None => {
   836	                    send_payload_records(tx, &source, cfg.plan_options, batch, &mut read_buf)
   837	                        .await?;
   838	                }
   839	            }
   840	            continue;
   841	        }
   842	        if need_complete {
   843	            break;
   844	        }
   845	        match events.recv().await {
   846	            Some(event) => {
   847	                handle_source_event(event, &mut pending, &mut need_complete)?;
   848	            }
   849	            None => {
   850	                return Err(eyre::Report::new(SessionFault::internal(
   851	                    "source receive half ended before NeedComplete",
   852	                )))
   853	            }
   854	        }
   855	    }
   856	
   857	    // Close the data plane BEFORE SourceDone so the destination's receive
   858	    // pipeline sees each socket's END record and completes; SourceDone on
   859	    // the control lane then lets the destination score and summarize.
   860	    if let Some(dp) = data_plane.take() {
   861	        dp.finish().await?;
   862	    }
   863	
   864	    tx.send(frame(Frame::SourceDone(SourceDone {}))).await?;
   865	
   866	    // CLOSING: the destination is the scorer; the next event must be
   867	    // its summary (the receive half ends after forwarding it).
   868	    match events.recv().await {
   869	        Some(SourceEvent::Summary(summary)) => Ok(summary),
   870	        Some(SourceEvent::Fault(fault)) => Err(eyre::Report::new(fault)),
   871	        Some(SourceEvent::Need(h)) => Err(eyre::Report::new(SessionFault::protocol_violation(
   872	            format!("need for '{}' after NeedComplete", h.relative_path),
   873	        ))),
   874	        Some(SourceEvent::NeedComplete) => Err(eyre::Report::new(
   875	            SessionFault::protocol_violation("duplicate NeedComplete"),
   876	        )),
   877	        None => Err(eyre::Report::new(SessionFault::internal(
   878	            "source receive half ended before TransferSummary",
   879	        ))),
   880	    }
   881	}
   882	
   883	fn drain_source_events(
   884	    events: &mut mpsc::UnboundedReceiver<SourceEvent>,
   885	    pending: &mut Vec<FileHeader>,
   886	    need_complete: &mut bool,
   887	) -> Result<()> {
   888	    while let Ok(event) = events.try_recv() {
   889	        handle_source_event(event, pending, need_complete)?;
   890	    }
   891	    Ok(())
   892	}
   893	
   894	fn handle_source_event(
   895	    event: SourceEvent,
   896	    pending: &mut Vec<FileHeader>,
   897	    need_complete: &mut bool,
   898	) -> Result<()> {
   899	    match event {
   900	        SourceEvent::Need(header) => {
   901	            if *need_complete {
   902	                return Err(eyre::Report::new(SessionFault::protocol_violation(
   903	                    format!("need for '{}' after NeedComplete", header.relative_path),
   904	                )));
   905	            }
   906	            pending.push(header);
   907	            Ok(())
   908	        }
   909	        SourceEvent::NeedComplete => {
   910	            if *need_complete {
   911	                return Err(eyre::Report::new(SessionFault::protocol_violation(
   912	                    "duplicate NeedComplete",
   913	                )));
   914	            }
   915	            *need_complete = true;
   916	            Ok(())
   917	        }
   918	        SourceEvent::Summary(_) => Err(eyre::Report::new(SessionFault::protocol_violation(
   919	            "TransferSummary before SourceDone",
   920	        ))),
   921	        SourceEvent::Fault(fault) => Err(eyre::Report::new(fault)),
   922	    }
   923	}
   924	
   925	/// Plan one batch of needed headers with the engine planner and emit
   926	/// the resulting payload records per the in-stream grammar.
   927	async fn send_payload_records(
   928	    tx: &mut Box<dyn FrameTx>,
   929	    source: &Arc<dyn TransferSource>,
   930	    plan_options: PlanOptions,
   931	    batch: Vec<FileHeader>,
   932	    read_buf: &mut [u8],
   933	) -> Result<()> {
   934	    let payloads = diff_planner::plan_push_payloads(batch, source.root(), plan_options)?;
   935	    for payload in payloads {
   936	        match source.prepare_payload(payload).await? {
   937	            PreparedPayload::File(header) => {
   938	                tx.send(frame(Frame::FileBegin(header.clone()))).await?;
   939	                if header.size == 0 {
   940	                    continue; // record complete at 0 cumulative bytes
   941	                }
   942	                let mut reader = source.open_file(&header).await?;
   943	                let mut remaining = header.size;
   944	                while remaining > 0 {
   945	                    let want = read_buf.len().min(remaining as usize);
   946	                    let got = reader.read(&mut read_buf[..want]).await?;
   947	                    if got == 0 {
   948	                        // Shorter on disk than the manifest promised —
   949	                        // the record can no longer complete at
   950	                        // header.size; abort rather than pad.
   951	                        eyre::bail!(
   952	                            "'{}' hit EOF with {} bytes still promised",
   953	                            header.relative_path,
   954	                            remaining
   955	                        );
   956	                    }
   957	                    tx.send(frame(Frame::FileData(FileData {
   958	                        content: read_buf[..got].to_vec(),
   959	                    })))
   960	                    .await?;
   961	                    remaining -= got as u64;
   962	                }
   963	            }
   964	            PreparedPayload::TarShard { headers, data } => {
   965	                tx.send(frame(Frame::TarShardHeader(TarShardHeader {
   966	                    files: headers,
   967	                    archive_size: data.len() as u64,
   968	                })))
   969	                .await?;
   970	                for chunk in data.chunks(IN_STREAM_CHUNK) {
   971	                    tx.send(frame(Frame::TarShardChunk(
   972	                        crate::generated::TarShardChunk {
   973	                            content: chunk.to_vec(),
   974	                        },
   975	                    )))
   976	                    .await?;
   977	                }
   978	                tx.send(frame(Frame::TarShardComplete(TarShardComplete {})))
   979	                    .await?;
   980	            }
   981	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   982	                // The outbound planner never emits these (resume is
   983	                // receive-originated and lands at otp-7).
   984	                eyre::bail!("resume payload planned in a non-resume session");
   985	            }
   986	        }
   987	    }
   988	    Ok(())
   989	}
   990	
   991	// ---------------------------------------------------------------------------
   992	// DESTINATION driver
   993	// ---------------------------------------------------------------------------
   994	
   995	/// What the destination end can report after a completed session.
   996	#[derive(Debug, Clone)]
   997	pub struct DestinationOutcome {
   998	    /// The summary this end computed and sent (contract: DESTINATION
   999	    /// is the scorer).
  1000	    pub summary: TransferSummary,
  1001	    /// Paths this end put on the need list, in emission order. The
  1002	    /// role suite pins these identical across role assignments — the
  1003	    /// executable form of the owner's invariance requirement.
  1004	    pub needed_paths: Vec<String>,
  1005	}
  1006	
  1007	/// Run the DESTINATION role of one transfer session over `transport`,
  1008	/// writing under the root named by `target`. Diffs the streamed
  1009	/// manifest against its own filesystem (the destination is the one
  1010	/// diff owner — plan §Design 3), returns the summary it computed and
  1011	/// sent.
  1012	///
  1013	/// `target` is [`DestinationTarget::Fixed`] when the root is known up
  1014	/// front (an Initiator's own local root, or a test), or
  1015	/// [`DestinationTarget::Resolve`] when the root must be resolved from
  1016	/// the received `SessionOpen` mid-handshake (the daemon Responder,
  1017	/// where the wire module name selects the root).
  1018	pub async fn run_destination(
  1019	    cfg: DestinationSessionConfig,
  1020	    transport: FrameTransport,
  1021	    target: DestinationTarget,
  1022	) -> Result<DestinationOutcome> {
  1023	    let mut transport = transport;
  1024	    let endpoint = match cfg.endpoint {
  1025	        SessionEndpoint::Initiator { mut open } => {
  1026	            let declared = TransferRole::try_from(open.initiator_role);
  1027	            if declared != Ok(TransferRole::Destination) {
  1028	                eyre::bail!(
  1029	                    "run_destination initiator must declare TRANSFER_ROLE_DESTINATION in SessionOpen"
  1030	                );
  1031	            }
  1032	            if let Err(fault) = destination_open_validator(&open) {
  1033	                eyre::bail!("run_destination initiator config unsupported: {fault}");
  1034	            }
  1035	            // Dial contract: the byte receiver advertises capacity in
  1036	            // its open when it is the initiator (contract §Invariants 5).
  1037	            if open.receiver_capacity.is_none() {
  1038	                open.receiver_capacity = Some(crate::engine::local_receiver_capacity());
  1039	            }
  1040	            SessionEndpoint::Initiator { open }
  1041	        }
  1042	        SessionEndpoint::Responder => SessionEndpoint::Responder,
  1043	    };
  1044	
  1045	    let resolve_open: Option<&OpenResolver> = match &target {
  1046	        DestinationTarget::Resolve(resolver) => Some(resolver.as_ref()),
  1047	        DestinationTarget::Fixed(_) => None,
  1048	    };
  1049	
  1050	    let negotiated = establish(
  1051	        &mut transport,
  1052	        &cfg.hello,
  1053	        &endpoint,
  1054	        TransferRole::Destination,
  1055	        &destination_open_validator,
  1056	        resolve_open,
  1057	    )
  1058	    .await?;
  1059	
  1060	    // The resolver's root (Responder + Resolve) wins; otherwise the
  1061	    // caller-supplied Fixed root.
  1062	    let dst_root = match negotiated.resolved_root.clone() {
  1063	        Some(root) => root,
  1064	        None => match &target {
  1065	            DestinationTarget::Fixed(root) => root.clone(),
  1066	            // Unreachable: a Resolve target always yields a root on the
  1067	            // Responder branch, and establish only skips resolution on
  1068	            // the Initiator branch (which pairs with a Fixed root).
  1069	            DestinationTarget::Resolve(_) => {
  1070	                return Err(eyre::Report::new(SessionFault::internal(
  1071	                    "resolver target produced no destination root",
  1072	                )));
  1073	            }
  1074	        },
  1075	    };
  1076	
  1077	    match destination_session(&mut transport, negotiated, &dst_root).await {
  1078	        Ok(outcome) => Ok(outcome),
  1079	        Err(report) => {
  1080	            let mut fault = fault_from_report(report);
  1081	            if !fault.peer_notified {
  1082	                let _ = transport.send(error_frame(&fault)).await;
  1083	                fault.peer_notified = true;
  1084	            }
  1085	            Err(eyre::Report::new(fault))
  1086	        }
  1087	    }
  1088	}
  1089	
  1090	fn violation(message: String) -> eyre::Report {
  1091	    eyre::Report::new(SessionFault::protocol_violation(message))
  1092	}
  1093	
  1094	async fn destination_session(
  1095	    transport: &mut FrameTransport,
  1096	    negotiated: Negotiated,
  1097	    dst_root: &Path,
  1098	) -> Result<DestinationOutcome> {
  1099	    let compare_mode = ComparisonMode::try_from(negotiated.open.compare_mode)
  1100	        .unwrap_or(ComparisonMode::Unspecified);
  1101	    let compare_opts = CompareOptions {
  1102	        mode: compare_mode.into(),
  1103	        ignore_existing: negotiated.open.ignore_existing,
  1104	        include_deletions: false, // mirror lands at otp-6
  1105	    };
  1106	    // src_root is only consumed by local File payloads, which never
  1107	    // occur on a session destination (payload bytes arrive as records
  1108	    // and go through the stream/tar write paths). `Arc` so the data-plane
  1109	    // receive task (otp-4b) can share the one sink across sockets.
  1110	    let sink = Arc::new(FsTransferSink::new(
  1111	        PathBuf::new(),
  1112	        dst_root.to_path_buf(),
  1113	        FsSinkConfig {
  1114	            preserve_times: true,
  1115	            dry_run: false,
  1116	            checksum: None,
  1117	            resume: false,
  1118	            compare_mode,
  1119	        },
  1120	    ));

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '1,380p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '260,620p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Session-side TCP data-plane orchestration (otp-4b).
     2	//!
     3	//! The unified session reuses blit-core's data-plane byte plumbing —
     4	//! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
     5	//! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
     6	//! its OWN choreography here. The push-specific bind/arm/accept loop
     7	//! (`blit-daemon` push service) and the multi-stream send driver
     8	//! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
     9	//! deletes at cutover (otp-10), so nothing in this file calls into them.
    10	//!
    11	//! otp-4b-1 scope: a single epoch-0 stream, no resize. The RESPONDER
    12	//! (whichever end is DESTINATION for otp-4/-5) binds a listener, mints
    13	//! the tokens, grants them in `SessionAccept`, and accepts + receives;
    14	//! the INITIATOR (SOURCE here) dials + authenticates + sends. Because
    15	//! the grant is issued before any manifest is seen,
    16	//! [`initial_stream_proposal`] with zero knowledge is 1 — the session
    17	//! data plane always starts single-stream and grows only via
    18	//! SOURCE-driven resize, which lands at otp-4b-2.
    19	
    20	use std::path::PathBuf;
    21	use std::sync::Arc;
    22	
    23	use eyre::Result;
    24	use tokio::io::AsyncReadExt;
    25	use tokio::net::{TcpListener, TcpStream};
    26	use tokio::sync::mpsc;
    27	use tokio::task::JoinSet;
    28	
    29	use crate::buffer::BufferPool;
    30	use crate::engine::{
    31	    initial_stream_proposal, local_receiver_capacity, DIAL_FLOOR_CHUNK_BYTES, DIAL_FLOOR_PREFETCH,
    32	};
    33	use crate::generated::{session_error::Code, DataPlaneGrant};
    34	use crate::remote::transfer::payload::TransferPayload;
    35	use crate::remote::transfer::pipeline::execute_receive_pipeline;
    36	use crate::remote::transfer::sink::{DataPlaneSink, SinkOutcome, TransferSink};
    37	use crate::remote::transfer::socket::{
    38	    configure_data_socket, DATA_PLANE_ACCEPT_TIMEOUT, DATA_PLANE_TOKEN_TIMEOUT,
    39	};
    40	use crate::remote::transfer::source::TransferSource;
    41	use crate::remote::transfer::{
    42	    execute_sink_pipeline_streaming, generate_sub_token, AbortOnDrop, DataPlaneSession,
    43	};
    44	
    45	use super::SessionFault;
    46	
    47	/// Dial values for the session data plane. otp-4b-1 has no live dial
    48	/// tuner, so it runs at the engine floor — the conservative start the
    49	/// dial contract mandates (absent/0 capacity fields ⇒ conservative,
    50	/// never unlimited). A live dial + tuner is future work, not this slice.
    51	const SESSION_DP_CHUNK_BYTES: usize = DIAL_FLOOR_CHUNK_BYTES;
    52	const SESSION_DP_PREFETCH: usize = DIAL_FLOOR_PREFETCH;
    53	
    54	fn dp_fault(msg: impl Into<String>) -> eyre::Report {
    55	    eyre::Report::new(SessionFault::refusal(Code::DataPlaneFailed, msg))
    56	}
    57	
    58	// ---------------------------------------------------------------------------
    59	// Responder (DESTINATION) — bind, grant, accept, receive
    60	// ---------------------------------------------------------------------------
    61	
    62	/// A bound data-plane listener plus the credentials the responder
    63	/// advertises in its `SessionAccept`. Held by the responder driver
    64	/// across the handshake so the accept loop can run after establish.
    65	pub(super) struct ResponderDataPlane {
    66	    listener: TcpListener,
    67	    session_token: Vec<u8>,
    68	    epoch0_sub_token: Vec<u8>,
    69	    initial_streams: u32,
    70	    port: u16,
    71	}
    72	
    73	/// Bind a data-plane listener and mint credentials for the grant. Any
    74	/// failure (bind, addr, RNG) logs and returns `None` — the caller then
    75	/// issues a grant-less `SessionAccept` and the session falls back to the
    76	/// in-stream carrier (contract §Transport selection: a responder that
    77	/// cannot bind grants no data plane).
    78	pub(super) async fn prepare_responder_data_plane() -> Option<ResponderDataPlane> {
    79	    let listener = match TcpListener::bind(("0.0.0.0", 0)).await {
    80	        Ok(listener) => listener,
    81	        Err(err) => {
    82	            log::warn!("session data-plane bind failed, using in-stream carrier: {err:#}");
    83	            return None;
    84	        }
    85	    };
    86	    let port = match listener.local_addr() {
    87	        Ok(addr) => addr.port(),
    88	        Err(err) => {
    89	            log::warn!("session data-plane local_addr failed, using in-stream carrier: {err:#}");
    90	            return None;
    91	        }
    92	    };
    93	    // Two independent 16-byte credentials (contract §Transport: a socket
    94	    // opens with session_token ‖ epoch0_sub_token). `generate_sub_token`
    95	    // is the fallible-RNG minter — a missing system RNG is an error, not
    96	    // a weaker credential.
    97	    let session_token = match generate_sub_token() {
    98	        Ok(token) => token,
    99	        Err(err) => {
   100	            log::warn!("session data-plane token RNG failed, using in-stream carrier: {err:#}");
   101	            return None;
   102	        }
   103	    };
   104	    let epoch0_sub_token = match generate_sub_token() {
   105	        Ok(token) => token,
   106	        Err(err) => {
   107	            log::warn!("session data-plane sub-token RNG failed, using in-stream carrier: {err:#}");
   108	            return None;
   109	        }
   110	    };
   111	    // The grant is issued before any manifest is seen, so the proposal
   112	    // has zero knowledge: initial_streams == 1. All growth is via resize
   113	    // (otp-4b-2). The ceiling is this end's own advertised max_streams.
   114	    let ceiling = local_receiver_capacity().max_streams.max(1) as usize;
   115	    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
   116	    Some(ResponderDataPlane {
   117	        listener,
   118	        session_token,
   119	        epoch0_sub_token,
   120	        initial_streams,
   121	        port,
   122	    })
   123	}
   124	
   125	impl ResponderDataPlane {
   126	    /// The `DataPlaneGrant` this responder advertises in `SessionAccept`.
   127	    pub(super) fn grant(&self) -> DataPlaneGrant {
   128	        DataPlaneGrant {
   129	            tcp_port: self.port as u32,
   130	            session_token: self.session_token.clone(),
   131	            initial_streams: self.initial_streams,
   132	            epoch0_sub_token: self.epoch0_sub_token.clone(),
   133	        }
   134	    }
   135	
   136	    /// Accept exactly `initial_streams` authenticated data sockets and
   137	    /// drain each into `sink` via the shared receive pipeline, returning
   138	    /// the aggregated write outcome (the DESTINATION is the scorer). The
   139	    /// caller runs this concurrently with the control-stream diff loop
   140	    /// and joins it on `SourceDone`.
   141	    pub(super) async fn accept_and_receive(
   142	        self,
   143	        sink: Arc<dyn TransferSink>,
   144	    ) -> Result<SinkOutcome> {
   145	        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
   146	        let mut expected = self.session_token.clone();
   147	        expected.extend_from_slice(&self.epoch0_sub_token);
   148	
   149	        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   150	        for _ in 0..self.initial_streams {
   151	            let mut socket = accept_authenticated(&self.listener, &expected).await?;
   152	            let sink = Arc::clone(&sink);
   153	            receives.spawn(async move { execute_receive_pipeline(&mut socket, sink, None).await });
   154	        }
   155	
   156	        let mut total = SinkOutcome::default();
   157	        while let Some(joined) = receives.join_next().await {
   158	            let outcome =
   159	                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   160	            total.files_written += outcome.files_written;
   161	            total.bytes_written += outcome.bytes_written;
   162	        }
   163	        Ok(total)
   164	    }
   165	}
   166	
   167	/// Accept one data socket under the shared bounded-accept timeout, apply
   168	/// the data-plane socket policy, read the fixed-length credential under
   169	/// the shared bounded-read timeout, and verify it. A socket presenting
   170	/// anything else is a `DATA_PLANE_FAILED` fault (contract §Transport: a
   171	/// mismatched socket is closed without response — here the whole session
   172	/// faults, since otp-4b-1 arms exactly the sockets it dials).
   173	async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
   174	    let accept = tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await;
   175	    let socket = match accept {
   176	        Ok(Ok((socket, _peer))) => socket,
   177	        Ok(Err(err)) => return Err(dp_fault(format!("data-plane accept failed: {err}"))),
   178	        Err(_) => {
   179	            return Err(dp_fault(format!(
   180	            "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
   181	        )))
   182	        }
   183	    };
   184	    configure_data_socket(&socket, None)
   185	        .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
   186	
   187	    let mut socket = socket;
   188	    let mut buf = vec![0u8; expected.len()];
   189	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   190	    match read {
   191	        Ok(Ok(_)) => {}
   192	        Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
   193	        Err(_) => {
   194	            return Err(dp_fault(format!(
   195	                "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   196	            )))
   197	        }
   198	    }
   199	    // Constant-time comparison is not required: the tokens are 16 random
   200	    // bytes read once per socket, single-session; a timing oracle buys
   201	    // nothing against per-transfer secrets (same posture as the old push
   202	    // acceptor's `token == expected_token`).
   203	    if buf != expected {
   204	        return Err(dp_fault(
   205	            "data-plane socket presented an invalid credential",
   206	        ));
   207	    }
   208	    Ok(socket)
   209	}
   210	
   211	// ---------------------------------------------------------------------------
   212	// Initiator (SOURCE) — dial, authenticate, send
   213	// ---------------------------------------------------------------------------
   214	
   215	/// A running source-side data plane: the dialed socket(s) wrapped as a
   216	/// sink pipeline. Planned payloads are fed via [`Self::queue`]; closing
   217	/// via [`Self::finish`] drains the pipeline, emits each socket's END
   218	/// record, and returns the bytes this end sent.
   219	pub(super) struct SourceDataPlane {
   220	    payload_tx: Option<mpsc::Sender<TransferPayload>>,
   221	    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
   222	    // `Result<SinkOutcome>`, so `T` is that (not the JoinHandle).
   223	    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
   224	}
   225	
   226	/// Dial the granted data plane and start the send pipeline. `host` is
   227	/// the responder's host (the initiator connected the control plane to
   228	/// it; the data plane rides the same host on the granted port —
   229	/// contract §Transport: the initiator always dials).
   230	pub(super) async fn dial_source_data_plane(
   231	    host: &str,
   232	    grant: &DataPlaneGrant,
   233	    source: Arc<dyn TransferSource>,
   234	) -> Result<SourceDataPlane> {
   235	    let streams = grant.initial_streams.max(1) as usize;
   236	    // Epoch-0 handshake: session_token ‖ epoch0_sub_token.
   237	    let mut handshake = grant.session_token.clone();
   238	    handshake.extend_from_slice(&grant.epoch0_sub_token);
   239	
   240	    let pool = Arc::new(BufferPool::for_data_plane(SESSION_DP_CHUNK_BYTES, streams));
   241	    let mut sinks: Vec<Arc<dyn TransferSink>> = Vec::with_capacity(streams);
   242	    for _ in 0..streams {
   243	        let session = DataPlaneSession::connect(
   244	            host,
   245	            grant.tcp_port,
   246	            &handshake,
   247	            SESSION_DP_CHUNK_BYTES,
   248	            SESSION_DP_PREFETCH,
   249	            false,
   250	            None,
   251	            Arc::clone(&pool),
   252	        )
   253	        .await
   254	        .map_err(|err| dp_fault(format!("dialing session data plane: {err:#}")))?;
   255	        // The source-side sink never reads its dst_root (it only sends);
   256	        // `root()` is consulted by the relay/receive case, not here.
   257	        sinks.push(Arc::new(DataPlaneSink::new(
   258	            session,
   259	            Arc::clone(&source),
   260	            PathBuf::new(),
   261	        )));
   262	    }
   263	
   264	    let (payload_tx, payload_rx) = mpsc::channel::<TransferPayload>(SESSION_DP_PREFETCH.max(1));
   265	    // Bounded by AbortOnDrop: a fault on the control lane that drops the
   266	    // SourceDataPlane aborts the pipeline task instead of leaking it.
   267	    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
   268	        execute_sink_pipeline_streaming(source, sinks, payload_rx, SESSION_DP_PREFETCH, None).await
   269	    }));
   270	    Ok(SourceDataPlane {
   271	        payload_tx: Some(payload_tx),
   272	        pipeline: Some(pipeline),
   273	    })
   274	}
   275	
   276	impl SourceDataPlane {
   277	    /// Feed one planned batch into the send pipeline. The pipeline
   278	    /// prepares each payload (tar-shard/file) and writes it through the
   279	    /// data-plane record framing across the live socket(s).
   280	    pub(super) async fn queue(&mut self, payloads: Vec<TransferPayload>) -> Result<()> {
   281	        let tx = self.payload_tx.as_ref().ok_or_else(|| {
   282	            eyre::Report::new(SessionFault::internal("data plane already finished"))
   283	        })?;
   284	        for payload in payloads {
   285	            tx.send(payload).await.map_err(|_| {
   286	                dp_fault("data-plane send pipeline closed before all payloads sent")
   287	            })?;
   288	        }
   289	        Ok(())
   290	    }
   291	
   292	    /// Signal end-of-stream, drain the pipeline (each worker emits its
   293	    /// socket's END record on drain), and return the bytes sent. Must be
   294	    /// awaited before `SourceDone` goes out so the destination's receive
   295	    /// pipeline sees END and completes.
   296	    pub(super) async fn finish(mut self) -> Result<SinkOutcome> {
   297	        // Drop the sender: workers observe the closed queue, drain what
   298	        // is left, then `finish()` (END record) and exit.
   299	        self.payload_tx = None;
   300	        let pipeline = self
   301	            .pipeline
   302	            .take()
   303	            .expect("SourceDataPlane::finish called once");
   304	        pipeline
   305	            .join()
   306	            .await
   307	            .map_err(|err| dp_fault(format!("data-plane send pipeline panicked: {err}")))?
   308	    }
   309	}
   310	
   311	#[cfg(test)]
   312	mod tests {
   313	    use super::*;
   314	    use crate::remote::transfer::SUB_TOKEN_LEN;
   315	
   316	    /// The otp-4b-1 grant invariant: the responder always grants a
   317	    /// single epoch-0 stream (the zero-knowledge proposal — no manifest
   318	    /// has been seen when SessionAccept goes out) with two independent
   319	    /// 16-byte credentials on a real port. Multi-stream is resize-only
   320	    /// (otp-4b-2).
   321	    #[tokio::test]
   322	    async fn responder_grant_is_single_stream_with_16_byte_tokens() {
   323	        let rdp = prepare_responder_data_plane()
   324	            .await
   325	            .expect("bind loopback data plane");
   326	        let grant = rdp.grant();
   327	        assert_eq!(
   328	            grant.initial_streams, 1,
   329	            "zero-knowledge grant starts single-stream (otp-4b-1)"
   330	        );
   331	        assert_eq!(grant.session_token.len(), SUB_TOKEN_LEN);
   332	        assert_eq!(grant.epoch0_sub_token.len(), SUB_TOKEN_LEN);
   333	        assert_ne!(
   334	            grant.session_token, grant.epoch0_sub_token,
   335	            "session token and epoch-0 sub-token are independent credentials"
   336	        );
   337	        assert_ne!(grant.tcp_port, 0, "a real ephemeral port is granted");
   338	    }
   339	}

 succeeded in 0ms:
   260	        TransferRole::Destination => TransferRole::Source,
   261	        TransferRole::Unspecified => TransferRole::Unspecified,
   262	    }
   263	}
   264	
   265	/// Build a `SessionError` frame with the given code and message — the
   266	/// wire form an end sends to tell its peer why it is aborting. Public
   267	/// so the daemon dispatcher can emit `CANCELLED` when a `CancelJob`
   268	/// fires mid-session (the session future is aborted by the select and
   269	/// cannot send it itself — otp-4a codex F1); blit-core stays the one
   270	/// owner of the frame grammar. The build-id fields are left empty:
   271	/// they are only meaningful for `BUILD_MISMATCH`.
   272	pub fn session_error_frame(code: session_error::Code, message: impl Into<String>) -> TransferFrame {
   273	    frame(Frame::Error(SessionError {
   274	        code: code as i32,
   275	        message: message.into(),
   276	        local_build_id: String::new(),
   277	        peer_build_id: String::new(),
   278	    }))
   279	}
   280	
   281	/// Per-role capability check of the operation a `SessionOpen`
   282	/// describes. otp-3 refuses what later slices implement rather than
   283	/// silently ignoring it (fail-fast; contract §Errors).
   284	type OpenValidator = dyn Fn(&SessionOpen) -> std::result::Result<(), SessionFault> + Send + Sync;
   285	
   286	/// The local endpoint a Responder resolves a received `SessionOpen`
   287	/// to. The daemon maps the wire module name + path here; a test can
   288	/// hand a fixed root with no module semantics via
   289	/// [`DestinationTarget::Fixed`] instead.
   290	#[derive(Debug, Clone)]
   291	pub struct ResolvedEndpoint {
   292	    /// Absolute local root this end targets.
   293	    pub root: PathBuf,
   294	    /// Whether the resolved module forbids writes. A DESTINATION
   295	    /// responder refuses `READ_ONLY`; a SOURCE responder (otp-5,
   296	    /// daemon-send) does not care — reading a read-only module is fine.
   297	    pub read_only: bool,
   298	}
   299	
   300	/// Async callback a Responder uses to turn a received (and
   301	/// capability-validated) `SessionOpen` into its local endpoint. It
   302	/// lives caller-side — the daemon resolves modules and maps its own
   303	/// `tonic::Status` errors to [`SessionFault`], so blit-core stays free
   304	/// of module/Status types. A returned fault (unknown module,
   305	/// containment failure) becomes a `SessionError` at OPEN, never a
   306	/// silent close (contract §Phase state machine).
   307	pub type OpenResolver = dyn Fn(
   308	        &SessionOpen,
   309	    )
   310	        -> Pin<Box<dyn Future<Output = std::result::Result<ResolvedEndpoint, SessionFault>> + Send>>
   311	    + Send
   312	    + Sync;
   313	
   314	/// Where a DESTINATION driver writes. `Fixed` is a root known up front
   315	/// (an initiator's own local root, or a test's temp dir). `Resolve`
   316	/// defers to a caller callback that maps the received `SessionOpen` to
   317	/// a local root — the daemon path, where the root depends on the wire
   318	/// module name and so can only be resolved mid-handshake (after HELLO,
   319	/// before SessionAccept). A `Resolve` target is meaningful only on a
   320	/// Responder; an Initiator always knows its own root.
   321	pub enum DestinationTarget {
   322	    Fixed(PathBuf),
   323	    Resolve(Box<OpenResolver>),
   324	}
   325	
   326	fn source_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
   327	    if open.resume.as_ref().is_some_and(|r| r.enabled) {
   328	        return Err(SessionFault::internal(
   329	            "resume is not implemented on the unified session yet (otp-7)",
   330	        ));
   331	    }
   332	    if open
   333	        .filter
   334	        .as_ref()
   335	        .is_some_and(|f| *f != FilterSpec::default())
   336	    {
   337	        return Err(SessionFault::internal(
   338	            "filters are not implemented on the unified session yet (otp-6)",
   339	        ));
   340	    }
   341	    Ok(())
   342	}
   343	
   344	fn destination_open_validator(open: &SessionOpen) -> std::result::Result<(), SessionFault> {
   345	    if open.mirror_enabled {
   346	        return Err(SessionFault::internal(
   347	            "mirror is not implemented on the unified session yet (otp-6)",
   348	        ));
   349	    }
   350	    if open.resume.as_ref().is_some_and(|r| r.enabled) {
   351	        return Err(SessionFault::internal(
   352	            "resume is not implemented on the unified session yet (otp-7)",
   353	        ));
   354	    }
   355	    Ok(())
   356	}
   357	
   358	/// Outcome of the HELLO + OPEN phases.
   359	struct Negotiated {
   360	    open: SessionOpen,
   361	    /// The responder's reply. The SOURCE initiator reads
   362	    /// `accept.data_plane` to decide dial-vs-in-stream (otp-4b).
   363	    accept: SessionAccept,
   364	    /// The write root a Responder's [`OpenResolver`] produced from the
   365	    /// received open, if one was supplied; `None` for an Initiator or a
   366	    /// fixed-root Responder (the caller supplies the root then).
   367	    resolved_root: Option<PathBuf>,
   368	    /// The bound data-plane listener + credentials a DESTINATION
   369	    /// Responder prepared before its `SessionAccept` (otp-4b). `None`
   370	    /// on an Initiator, or when the responder granted no data plane
   371	    /// (in-stream carrier). Consumed by the DESTINATION accept loop.
   372	    responder_data_plane: Option<data_plane::ResponderDataPlane>,
   373	}
   374	
   375	/// HELLO + OPEN/ACCEPT, one implementation both roles call (otp-3
   376	/// scoping requirement). Sends the refusal `SessionError` itself when
   377	/// it detects the fault locally; returned faults are `peer_notified`.
   378	async fn establish(
   379	    transport: &mut FrameTransport,
   380	    hello: &HelloConfig,
   381	    endpoint: &SessionEndpoint,
   382	    local_role: TransferRole,
   383	    validate_open: &OpenValidator,
   384	    // Consulted only on the Responder branch, after the received open
   385	    // passes `validate_open` and before SessionAccept. `None` = the
   386	    // caller supplies the root itself (Initiator, or fixed-root test).
   387	    resolve_open: Option<&OpenResolver>,
   388	) -> Result<Negotiated> {
   389	    // HELLO both ways, exact match (D-2026-07-05-2). First frame each
   390	    // direction; no ordering between the two directions.
   391	    transport
   392	        .send(frame(Frame::Hello(SessionHello {
   393	            build_id: hello.build_id.clone(),
   394	            contract_version: hello.contract_version,
   395	        })))
   396	        .await?;
   397	
   398	    let peer_hello = match expect_frame(transport).await? {
   399	        Frame::Hello(h) => h,
   400	        other => {
   401	            return Err(notify_and_wrap(
   402	                transport,
   403	                SessionFault::protocol_violation(format!(
   404	                    "expected SessionHello, got {}",
   405	                    frame_name(&Some(other))
   406	                )),
   407	            )
   408	            .await)
   409	        }
   410	    };
   411	
   412	    if peer_hello.build_id != hello.build_id
   413	        || peer_hello.contract_version != hello.contract_version
   414	    {
   415	        let fault = SessionFault {
   416	            code: session_error::Code::BuildMismatch,
   417	            message: format!(
   418	                "same-build peers required (D-2026-07-05-2): local {} (contract v{}) vs peer {} (contract v{})",
   419	                hello.build_id, hello.contract_version,
   420	                peer_hello.build_id, peer_hello.contract_version,
   421	            ),
   422	            local_build_id: hello.build_id.clone(),
   423	            peer_build_id: peer_hello.build_id.clone(),
   424	            peer_notified: false,
   425	        };
   426	        return Err(notify_and_wrap(transport, fault).await);
   427	    }
   428	
   429	    match endpoint {
   430	        SessionEndpoint::Initiator { open } => {
   431	            let open = open.as_ref().clone();
   432	            transport.send(frame(Frame::Open(open.clone()))).await?;
   433	            let accept = match expect_frame(transport).await? {
   434	                Frame::Accept(a) => a,
   435	                other => {
   436	                    return Err(notify_and_wrap(
   437	                        transport,
   438	                        SessionFault::protocol_violation(format!(
   439	                            "expected SessionAccept, got {}",
   440	                            frame_name(&Some(other))
   441	                        )),
   442	                    )
   443	                    .await)
   444	                }
   445	            };
   446	            Ok(Negotiated {
   447	                open,
   448	                accept,
   449	                resolved_root: None,
   450	                responder_data_plane: None,
   451	            })
   452	        }
   453	        SessionEndpoint::Responder => {
   454	            let open = match expect_frame(transport).await? {
   455	                Frame::Open(o) => o,
   456	                other => {
   457	                    return Err(notify_and_wrap(
   458	                        transport,
   459	                        SessionFault::protocol_violation(format!(
   460	                            "expected SessionOpen, got {}",
   461	                            frame_name(&Some(other))
   462	                        )),
   463	                    )
   464	                    .await)
   465	                }
   466	            };
   467	            // The initiator declares ITS role; this responder end must
   468	            // hold the complement.
   469	            let declared =
   470	                TransferRole::try_from(open.initiator_role).unwrap_or(TransferRole::Unspecified);
   471	            if declared != complement(local_role) {
   472	                return Err(notify_and_wrap(
   473	                    transport,
   474	                    SessionFault::protocol_violation(format!(
   475	                        "initiator declared role {} but this responder is {}",
   476	                        declared.as_str_name(),
   477	                        local_role.as_str_name()
   478	                    )),
   479	                )
   480	                .await);
   481	            }
   482	            if let Err(fault) = validate_open(&open) {
   483	                // Refusal is a SessionError instead of SessionAccept,
   484	                // never a silent close (contract §Phase state machine).
   485	                return Err(notify_and_wrap(transport, fault).await);
   486	            }
   487	            // Responder endpoint resolution (otp-4): map the wire
   488	            // module/path to a local root and enforce read-only, both
   489	            // BEFORE SessionAccept so a refusal replaces the accept
   490	            // (never follows it). The resolver is caller-supplied
   491	            // (daemon module lookup); a fixed-root responder passes
   492	            // None and resolves nothing here.
   493	            let resolved_root = match resolve_open {
   494	                Some(resolve) => match resolve(&open).await {
   495	                    Ok(resolved) => {
   496	                        // A read-only module is fatal only for a
   497	                        // DESTINATION (it would write); a SOURCE
   498	                        // responder (otp-5, daemon-send) reads happily.
   499	                        if local_role == TransferRole::Destination && resolved.read_only {
   500	                            return Err(notify_and_wrap(
   501	                                transport,
   502	                                SessionFault::read_only(
   503	                                    "destination module is read-only".to_string(),
   504	                                ),
   505	                            )
   506	                            .await);
   507	                        }
   508	                        Some(resolved.root)
   509	                    }
   510	                    Err(fault) => return Err(notify_and_wrap(transport, fault).await),
   511	                },
   512	                None => None,
   513	            };
   514	            // Data plane (otp-4b): a DESTINATION responder binds a TCP
   515	            // listener and grants it, unless the initiator requested the
   516	            // in-stream carrier or the bind fails (grant-less accept ⇒
   517	            // in-stream fallback). A SOURCE responder (otp-5,
   518	            // daemon-send) will bind on its own branch later; otp-4b's
   519	            // responder is always the DESTINATION.
   520	            let responder_data_plane =
   521	                if local_role == TransferRole::Destination && !open.in_stream_bytes {
   522	                    data_plane::prepare_responder_data_plane().await
   523	                } else {
   524	                    None
   525	                };
   526	            let accept = SessionAccept {
   527	                // The byte RECEIVER advertises capacity at session
   528	                // open (D-2026-06-20-1/-2); consumed by the dial when
   529	                // the data plane lands (otp-4b).
   530	                receiver_capacity: if local_role == TransferRole::Destination {
   531	                    Some(crate::engine::local_receiver_capacity())
   532	                } else {
   533	                    None
   534	                },
   535	                // Grant present ⇒ TCP data plane; absent ⇒ in-stream.
   536	                data_plane: responder_data_plane.as_ref().map(|dp| dp.grant()),
   537	            };
   538	            transport.send(frame(Frame::Accept(accept.clone()))).await?;
   539	            Ok(Negotiated {
   540	                open,
   541	                accept,
   542	                resolved_root,
   543	                responder_data_plane,
   544	            })
   545	        }
   546	    }
   547	}
   548	
   549	/// Receive one frame during establish; peer errors and closes become
   550	/// terminal faults.
   551	async fn expect_frame(transport: &mut FrameTransport) -> Result<Frame> {
   552	    match transport.recv().await? {
   553	        Some(TransferFrame {
   554	            frame: Some(Frame::Error(err)),
   555	        }) => Err(eyre::Report::new(SessionFault::from_wire(err))),
   556	        Some(TransferFrame { frame: Some(f) }) => Ok(f),
   557	        Some(TransferFrame { frame: None }) => Err(eyre::Report::new(
   558	            SessionFault::protocol_violation("frame with empty oneof"),
   559	        )),
   560	        None => Err(eyre::Report::new(SessionFault::internal(
   561	            "peer closed during session establish",
   562	        ))),
   563	    }
   564	}
   565	
   566	/// Send the fault to the peer (best effort), mark it notified, and
   567	/// wrap it for return.
   568	async fn notify_and_wrap(transport: &mut FrameTransport, mut fault: SessionFault) -> eyre::Report {
   569	    let _ = transport.send(error_frame(&fault)).await;
   570	    fault.peer_notified = true;
   571	    eyre::Report::new(fault)
   572	}
   573	
   574	// ---------------------------------------------------------------------------
   575	// SOURCE driver
   576	// ---------------------------------------------------------------------------
   577	
   578	/// Events the source's receive half forwards to its send half. The
   579	/// channel is unbounded but bounded by construction: every `Need`
   580	/// consumes a distinct sent-manifest entry (unknown or repeated paths
   581	/// fault the session), so the queue never exceeds the source's own
   582	/// manifest size — the contract's bounded-buffering rule holds.
   583	enum SourceEvent {
   584	    Need(FileHeader),
   585	    NeedComplete,
   586	    Summary(TransferSummary),
   587	    Fault(SessionFault),
   588	}
   589	
   590	/// Run the SOURCE role of one transfer session over `transport`.
   591	/// Returns the destination-computed `TransferSummary` (contract: the
   592	/// end that wrote the bytes is the end that attests to them).
   593	pub async fn run_source(
   594	    cfg: SourceSessionConfig,
   595	    transport: FrameTransport,
   596	    source: Arc<dyn TransferSource>,
   597	) -> Result<TransferSummary> {
   598	    let mut transport = transport;
   599	    if let SessionEndpoint::Initiator { open } = &cfg.endpoint {
   600	        // Own-config coherence: a source initiator declares SOURCE.
   601	        let declared = TransferRole::try_from(open.initiator_role);
   602	        if declared != Ok(TransferRole::Source) {
   603	            eyre::bail!("run_source initiator must declare TRANSFER_ROLE_SOURCE in SessionOpen");
   604	        }
   605	        if let Err(fault) = source_open_validator(open) {
   606	            eyre::bail!("run_source initiator config unsupported: {fault}");
   607	        }
   608	    }
   609	
   610	    let negotiated = establish(
   611	        &mut transport,
   612	        &cfg.hello,
   613	        &cfg.endpoint,
   614	        TransferRole::Source,
   615	        &source_open_validator,
   616	        // A SOURCE responder's endpoint resolution (module→root for a
   617	        // daemon-send) lands with otp-5; otp-4a's daemon is always the
   618	        // DESTINATION responder, so the source never resolves here.
   619	        None,
   620	    )

exec
/usr/bin/zsh -lc 'git show --unified=80 881d412 -- crates/blit-core/src/remote/transfer/session_client.rs crates/blit-core/tests/transfer_session_roles.rs crates/blit-daemon/src/service/transfer_session_e2e.rs .review/findings/otp-4b-data-plane.md' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 881d412b77512fd1d1e28c5932653b63e2cbf41a
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 22:54:28 2026 -0400

    otp-4b-1: TCP data plane onto the session (single stream)
    
    Port the TCP data plane onto the unified transfer session, replacing the
    in-stream carrier as the default. The DESTINATION responder binds a
    listener, mints session_token + epoch0_sub_token, and grants them in
    SessionAccept; the SOURCE initiator dials + authenticates the socket and
    sends payloads over the shared DataPlaneSession record framing while the
    control stream carries manifest/needs/summary. In-stream stays live as
    the requested fallback (--force-grpc-shaped).
    
    Single epoch-0 stream only; the zero-knowledge grant proposal is 1, so
    multi-stream is resize-only (otp-4b-2). Session-owned orchestration lives
    in transfer_session/data_plane.rs, reusing blit-core primitives
    (DataPlaneSession, execute_receive_pipeline, execute_sink_pipeline_
    streaming, dial_data_plane) — no call into remote::push or the daemon
    push service (those drivers die at otp-10).
    
    A/B parity vs old push over the data plane holds byte-identically.
    Suite 1509 -> 1511. [state: skip]
    
    Finding: .review/findings/otp-4b-data-plane.md
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>

diff --git a/.review/findings/otp-4b-data-plane.md b/.review/findings/otp-4b-data-plane.md
new file mode 100644
index 0000000..7d5b973
--- /dev/null
+++ b/.review/findings/otp-4b-data-plane.md
@@ -0,0 +1,186 @@
+# otp-4b — TCP data plane onto the unified session
+
+**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-4.
+**Contract**: `docs/TRANSFER_SESSION.md` §Transport selection.
+**Builds on**: otp-4a (`4b07bbb`+`25f538b`) — daemon serves `Transfer`,
+client `run_source`s as SOURCE over the **in-stream** carrier.
+**Status**: 4b-1 (single-stream data plane) implemented + validated;
+codex review pending. 4b-2 (resize + sf-2) and 4b-3 (cancel e2e) queued.
+
+## Goal (this slice)
+
+Port the TCP data plane onto the unified session so a client push rides
+real data-plane sockets (not the in-stream gRPC carrier), byte-identical
+to old push, with the sf-2 shape-correction resize as the one and only
+stream-growth policy. The wire contract is already frozen at otp-1
+(`DataPlaneGrant` in `SessionAccept`, frames 16/17); this slice only
+*consumes* it — no proto change.
+
+## Key architectural facts (established by tracing the old push path)
+
+- The reusable **byte plumbing** all lives in `blit-core` and is the
+  plan's "kept" engine: `DataPlaneSession` (record framing, double
+  buffering, StallGuard — `remote/transfer/data_plane.rs`),
+  `socket::dial_data_plane`, `execute_sink_pipeline_elastic` +
+  `SinkControl::{Add,RetireOne}` and `execute_receive_pipeline`
+  (`remote/transfer/pipeline.rs`), `DataPlaneSink` (`sink.rs`),
+  `TransferDial::{conservative_within,propose_shape_resize,resize_settled,
+  live_streams,ceiling_max_streams}`, `initial_stream_proposal`,
+  `local_receiver_capacity`, `generate_sub_token` (16 bytes).
+- The **orchestration** (daemon bind/arm/accept loop; client
+  multi-stream send + resize driver) is push-specific code in
+  `blit-daemon/src/service/push/` and `blit-core/src/remote/push/client/`
+  — the per-direction drivers ONE_TRANSFER_PATH deletes at otp-10. The
+  session therefore grows its **own** orchestration in `transfer_session/`,
+  reusing the blit-core primitives above. Nothing here calls into
+  `remote::push` or the daemon push service.
+- **Streaming consequence**: the responder issues the grant inside
+  `SessionAccept` — *before* it has seen a single manifest entry. So
+  `initial_streams` is always the zero-knowledge floor
+  (`initial_stream_proposal(0,0,ceiling) == 1`). The session data plane
+  **always starts single-stream and grows only via SOURCE-driven resize**
+  (sf-2). This is why multi-stream lives entirely in 4b-2, not 4b-1.
+- **Token sizes (new contract, `docs/TRANSFER_SESSION.md` §Transport)**:
+  `session_token` = 16 bytes, `epoch0_sub_token` = 16 bytes; an epoch-0
+  socket opens with `session_token ‖ epoch0_sub_token` (32 bytes), a
+  resize-ADD socket with `session_token ‖ resize.sub_token`. (Old push
+  used a 32-byte session token; the session uses 16 per the otp-1
+  contract. Both minted by `generate_sub_token`.)
+
+## Staging (each sub-slice is one commit through the codex loop)
+
+- **otp-4b-1 (single-stream data plane)** — *this commit*. Responder
+  (DESTINATION) binds a listener, mints tokens, grants
+  `initial_streams = 1` in `SessionAccept`; SOURCE reads the grant,
+  dials one socket (`session_token ‖ epoch0_sub_token`), and sends every
+  payload over it via a `DataPlaneSink`; DESTINATION accepts the socket
+  and drains it with `execute_receive_pipeline` into the same
+  `FsTransferSink` the control loop already builds. No resize. Fallback
+  to the in-stream carrier when the responder cannot bind or the
+  initiator set `in_stream_bytes`. A/B parity vs old push **over the
+  data plane**.
+- **otp-4b-2 (resize + multi-stream + sf-2 pin)** — SOURCE drives
+  `TransferDial::propose_shape_resize` as the need list accumulates:
+  emits `DataPlaneResize{ADD, epoch, target, sub_token}` (frame 16) on
+  the control stream; DESTINATION arms a new accept slot and replies
+  `DataPlaneResizeAck` (frame 17); SOURCE dials the epoch-N socket and
+  hands its sink to the running elastic pipeline (`SinkControl::Add`).
+  Port the sf-2 10k-file `>1-stream` pin onto the session (assert the
+  session's settled `live_streams() > 1`).
+- **otp-4b-3 (mid-transfer cancel e2e)** — deterministic test that fires
+  `CancelJob` while bytes flow over the data plane and asserts the client
+  surfaces `SessionFault{CANCELLED}` and the daemon tears down cleanly.
+
+## otp-4b-1 design
+
+**Responder (DESTINATION) side — `run_destination` / `establish`:**
+- Before sending `SessionAccept`, if the initiator did not request
+  `in_stream_bytes`, the responder prepares a data plane: bind
+  `TcpListener` on `0.0.0.0:0`, mint `session_token` + `epoch0_sub_token`
+  (16 bytes each), compute `initial_streams = 1`, and put the resulting
+  `DataPlaneGrant{tcp_port, session_token, initial_streams,
+  epoch0_sub_token}` in the accept. A bind failure logs and falls back to
+  a grant-less accept (in-stream). `establish` returns the bound listener
+  + tokens to `run_destination` via `Negotiated` so the accept loop can
+  run after the handshake.
+- After establish, `destination_session` runs the control loop
+  (manifest→needs→SourceDone→summary) *concurrently* with a data-plane
+  accept task: accept exactly `initial_streams` socket(s) under the
+  shared bounded-accept timeout, verify `session_token ‖ epoch0_sub_token`,
+  then `execute_receive_pipeline(&mut socket, sink.clone(), None)` per
+  socket into the shared `FsTransferSink`. Payload records no longer
+  arrive on the control stream in data-plane mode; a `file_begin`/
+  `tar_shard_header` on the control lane there is a PROTOCOL_VIOLATION
+  (the in-stream grammar is the fallback carrier only). The DESTINATION
+  tallies files/bytes from the receive pipeline outcome(s), waits for
+  `SourceDone` + all receive tasks, then sends `TransferSummary`
+  (`in_stream_carrier_used = false`).
+
+**Initiator (SOURCE) side — `run_source` / `source_send_half`:**
+- After establish, inspect `negotiated.accept.data_plane`. If present,
+  the payload phase dials one socket via `DataPlaneSession::connect`
+  (handshake `session_token ‖ epoch0_sub_token`), wraps it in a
+  `DataPlaneSink`, and feeds planned `TransferPayload`s (from
+  `diff_planner::plan_push_payloads`) into `execute_sink_pipeline_streaming`
+  (single sink) instead of `send_payload_records`. On NeedComplete +
+  all needs flushed, `finish()` the sink (writes the END record) and send
+  `SourceDone` on the control stream. The manifest/need/summary
+  choreography on the control stream is unchanged from otp-4a.
+- If `data_plane` is absent, the in-stream path from otp-4a runs verbatim
+  (fallback carrier).
+
+**Why this is byte-identical to old push**: the record framing, the
+double-buffered send/receive, and the `FsTransferSink` write path are the
+exact same blit-core code old push uses; only the choreography around
+them is the unified session's. The A/B parity test proves it.
+
+## Files (planned, 4b-1)
+- `crates/blit-core/src/transfer_session/mod.rs` — grant prep on the
+  Responder, data-plane accept loop on DESTINATION, data-plane send on
+  SOURCE; `Negotiated` carries the responder listener/tokens.
+- `crates/blit-core/src/transfer_session/data_plane.rs` (new) — the
+  session-side data-plane orchestration helpers (accept+auth,
+  socket→sink send), reusing the blit-core primitives.
+- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — data-plane
+  parity + lands-bytes tests (drop `in_stream_bytes`).
+- `crates/blit-core/src/remote/transfer/session_client.rs` — the client
+  entry stops forcing `in_stream_bytes` (or gains an option).
+
+## Files (4b-1, as implemented)
+- `crates/blit-core/src/transfer_session/data_plane.rs` (new) — the
+  session-side data-plane orchestration: `prepare_responder_data_plane`
+  (bind + mint tokens + grant), `ResponderDataPlane::{grant,
+  accept_and_receive}`, `accept_authenticated`, `dial_source_data_plane`,
+  `SourceDataPlane::{queue, finish}`. Reuses the blit-core primitives;
+  no call into `remote::push` or the daemon push service.
+- `crates/blit-core/src/transfer_session/mod.rs` — `mod data_plane`;
+  `Negotiated` carries the responder listener/tokens; `establish`
+  Responder branch prepares + grants the data plane (DESTINATION, unless
+  `in_stream_bytes` or bind fails); `source_send_half` dials up front and
+  queues planned payloads to the data plane; `destination_session` (now
+  by-value) arms the accept+receive task, treats control-lane payload
+  frames as violations under a data plane, and joins the receive task at
+  `SourceDone` for the authoritative counts (completeness = files
+  received == need-list size).
+- `crates/blit-core/src/remote/transfer/session_client.rs` —
+  `PushSessionOptions.in_stream_bytes` (default `false` = data plane);
+  threads `data_plane_host`.
+- `crates/blit-daemon/src/service/transfer_session_e2e.rs` — data-plane
+  parity + in-stream fallback tests.
+- `crates/blit-core/tests/transfer_session_roles.rs` — `data_plane_host:
+  None` on the in-process configs (they ride the in-stream carrier).
+
+## Tests (4b-1)
+Suite 1509 → **1511** (+2: `session_lands_bytes_over_in_stream_carrier`
+e2e + `responder_grant_is_single_stream_with_16_byte_tokens` unit; the
+old `session_lands_bytes_and_scores_them` became
+`session_lands_bytes_over_the_data_plane`). New/changed:
+- `session_lands_bytes_over_the_data_plane` — default rides the TCP data
+  plane (`!in_stream_carrier_used`), byte-identical trees + counts.
+- `old_push_and_session_produce_identical_trees_and_counts` — **A/B
+  parity over the data plane**: old push and the session (both data
+  plane) yield byte-identical trees + equal shared counters.
+- `session_lands_bytes_over_in_stream_carrier` — the in-stream fallback
+  still lands bytes and reports `in_stream_carrier_used`.
+- `responder_grant_is_single_stream_with_16_byte_tokens` — grant shape.
+
+Gate: `cargo fmt --check` ✓, `clippy --workspace --all-targets
+-D warnings` ✓, `cargo test --workspace` **1511/0** ✓.
+
+## Guard proof (4b-1)
+`session_lands_bytes_over_the_data_plane` asserts
+`summary.in_stream_carrier_used == false` + byte-identical trees.
+**Proven**: forcing `prepare_responder_data_plane` to return `None`
+(grant-less accept ⇒ in-stream fallback) flips the flag and fails the
+assertion (`otp-4b default rides the TCP data plane, not the in-stream
+carrier`); restored, the suite is green. A/B parity vs old push guards
+the byte identity of the data-plane path.
+
+## Known gaps (carried)
+- Resize / multi-stream / sf-2 pin → otp-4b-2.
+- Mid-transfer cancel e2e → otp-4b-3.
+- Progress-byte threading (`with_byte_progress`) still deferred (session
+  rows report `bytes_completed=0`, as today's push rows).
+
+## Reviewer comments
+(filled after codex review per `docs/agent/GPT_REVIEW_LOOP.md`.)
diff --git a/crates/blit-core/src/remote/transfer/session_client.rs b/crates/blit-core/src/remote/transfer/session_client.rs
index 34c37eb..89f38e9 100644
--- a/crates/blit-core/src/remote/transfer/session_client.rs
+++ b/crates/blit-core/src/remote/transfer/session_client.rs
@@ -1,123 +1,132 @@
 //! Client-side entry for initiating a unified transfer session as the
 //! SOURCE role (otp-4a).
 //!
 //! Builds a gRPC-backed [`FrameTransport`] over `BlitClient::transfer`
 //! and runs [`run_source`], so a CLI push becomes "open the Transfer
 //! RPC, declare SOURCE, stream the manifest + payloads." This is the
 //! push-equivalent on the unified path; the daemon answers by running
 //! `run_destination` as the Responder.
 //!
 //! Not yet wired to CLI verbs — the verbs keep riding the old push
 //! path until the otp-10 cutover; today the parity tests drive this.
 //! otp-4a uses the in-stream byte carrier only (`in_stream_bytes`);
 //! the TCP data plane lands at otp-4b.
 
 use std::sync::Arc;
 use std::time::Duration;
 
 use eyre::{eyre, Result};
 use tokio::sync::mpsc;
 use tokio_stream::wrappers::ReceiverStream;
 use tonic::transport::{Channel, Endpoint};
 
 use crate::generated::blit_client::BlitClient;
 use crate::generated::{ComparisonMode, SessionOpen, TransferRole, TransferSummary};
 use crate::remote::endpoint::{RemoteEndpoint, RemotePath};
 use crate::remote::transfer::source::TransferSource;
 use crate::transfer_plan::PlanOptions;
 use crate::transfer_session::transport::{grpc_client_transport, GRPC_CHANNEL_FRAMES};
 use crate::transfer_session::{run_source, HelloConfig, SessionEndpoint, SourceSessionConfig};
 
-/// The push-shaped subset of session options otp-4a supports. Mirror,
+/// The push-shaped subset of session options otp-4a/4b supports. Mirror,
 /// filters, and resume are refused at OPEN until their slices land
 /// (otp-6/otp-7), so they are intentionally absent here.
 pub struct PushSessionOptions {
     pub compare_mode: ComparisonMode,
     pub ignore_existing: bool,
     pub require_complete_scan: bool,
     pub plan_options: PlanOptions,
+    /// Force the in-stream byte carrier instead of the TCP data plane
+    /// (otp-4b). Default `false` = the responder grants a data plane and
+    /// payloads ride TCP sockets; `true` is the diagnostics / unreachable
+    /// data-plane fallback (`--force-grpc`-shaped).
+    pub in_stream_bytes: bool,
 }
 
 impl Default for PushSessionOptions {
     fn default() -> Self {
         Self {
             compare_mode: ComparisonMode::SizeMtime,
             ignore_existing: false,
             require_complete_scan: false,
             plan_options: PlanOptions::default(),
+            in_stream_bytes: false,
         }
     }
 }
 
 /// Connect to `endpoint`'s daemon and run one SOURCE-role transfer
 /// session pushing `source`'s tree into the endpoint's module/path.
 /// Returns the destination-computed [`TransferSummary`] (contract:
 /// DESTINATION is the scorer).
 pub async fn run_push_session(
     endpoint: &RemoteEndpoint,
     source: Arc<dyn TransferSource>,
     options: PushSessionOptions,
 ) -> Result<TransferSummary> {
     // The responder resolves module→root; the initiator's own local
     // path never crosses the wire (contract §SessionOpen). Empty module
     // targets the daemon's default root export.
     let (module, path) = match &endpoint.path {
         RemotePath::Module { module, rel_path } => {
             (module.clone(), rel_path.to_string_lossy().into_owned())
         }
         RemotePath::Root { rel_path } => (String::new(), rel_path.to_string_lossy().into_owned()),
         RemotePath::Discovery => {
             return Err(eyre!(
                 "a transfer session needs a resolved module or root endpoint, not a discovery form"
             ));
         }
     };
 
     let mut client = connect_transfer_client(endpoint).await?;
 
     let open = SessionOpen {
         initiator_role: TransferRole::Source as i32,
         module,
         path,
         compare_mode: options.compare_mode as i32,
         ignore_existing: options.ignore_existing,
         require_complete_scan: options.require_complete_scan,
-        // otp-4a: in-stream byte carrier only; the TCP data plane
-        // grant lands at otp-4b.
-        in_stream_bytes: true,
+        // otp-4b: default to the TCP data plane; the responder grants it
+        // in SessionAccept unless this asks for the in-stream fallback.
+        in_stream_bytes: options.in_stream_bytes,
         ..Default::default()
     };
 
     // Open the bidi RPC: the request stream is fed by `out_tx`, the
     // response stream is the inbound half. The handler returns its
     // response stream immediately (it spawns the session), so this
     // await resolves before any frame flows — no deadlock.
     let (out_tx, out_rx) = mpsc::channel(GRPC_CHANNEL_FRAMES);
     let inbound = client
         .transfer(ReceiverStream::new(out_rx))
         .await
         .map_err(|status| eyre!("opening Transfer RPC: {}", status.message()))?
         .into_inner();
     let transport = grpc_client_transport(out_tx, inbound);
 
     let cfg = SourceSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::initiator(open),
         plan_options: options.plan_options,
+        // The initiator dials the data plane on the same host it reached
+        // the control plane on (contract §Transport: initiator dials).
+        data_plane_host: Some(endpoint.host.clone()),
     };
     run_source(cfg, transport, source).await
 }
 
 /// Build a `BlitClient` over `endpoint`'s control-plane URI with the
 /// same bounded-connect policy `RemotePushClient::connect` uses.
 async fn connect_transfer_client(endpoint: &RemoteEndpoint) -> Result<BlitClient<Channel>> {
     let uri = endpoint.control_plane_uri();
     let conn = Endpoint::from_shared(uri.clone())
         .map_err(|e| eyre!("invalid endpoint uri {uri}: {e}"))?
         .connect_timeout(Duration::from_secs(30));
     let channel = tokio::time::timeout(Duration::from_secs(30), conn.connect())
         .await
         .map_err(|_| eyre!("timed out connecting to {uri}"))?
         .map_err(|e| eyre!("connecting to {uri}: {e}"))?;
     Ok(BlitClient::new(channel))
 }
diff --git a/crates/blit-core/tests/transfer_session_roles.rs b/crates/blit-core/tests/transfer_session_roles.rs
index b786316..7c0089e 100644
--- a/crates/blit-core/tests/transfer_session_roles.rs
+++ b/crates/blit-core/tests/transfer_session_roles.rs
@@ -43,160 +43,161 @@ fn write_tree(root: &Path, files: &[FileSpec]) {
         }
         std::fs::write(&path, content).unwrap();
         filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(*mtime, 0)).unwrap();
     }
 }
 
 /// Every regular file under `root` as rel-path → bytes.
 fn collect_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
     fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
         for entry in std::fs::read_dir(dir).unwrap() {
             let entry = entry.unwrap();
             let path = entry.path();
             if path.is_dir() {
                 walk(root, &path, out);
             } else {
                 let rel = path
                     .strip_prefix(root)
                     .unwrap()
                     .to_string_lossy()
                     .replace('\\', "/");
                 out.insert(rel, std::fs::read(&path).unwrap());
             }
         }
     }
     let mut out = BTreeMap::new();
     if root.exists() {
         walk(root, root, &mut out);
     }
     out
 }
 
 fn assert_trees_identical(src: &Path, dst: &Path) {
     let src_tree = collect_tree(src);
     let dst_tree = collect_tree(dst);
     assert_eq!(
         src_tree.keys().collect::<Vec<_>>(),
         dst_tree.keys().collect::<Vec<_>>(),
         "path sets differ between {src:?} and {dst:?}"
     );
     for (rel, bytes) in &src_tree {
         assert_eq!(
             bytes, &dst_tree[rel],
             "content differs for '{rel}' between {src:?} and {dst:?}"
         );
     }
 }
 
 fn basic_open(initiator_role: TransferRole) -> SessionOpen {
     SessionOpen {
         initiator_role: initiator_role as i32,
         compare_mode: ComparisonMode::SizeMtime as i32,
         in_stream_bytes: true,
         ..Default::default()
     }
 }
 
 /// Drive one full session between `src_root` and `dst_root` with the
 /// given end acting as initiator. Data direction is FIXED
 /// (src_root → dst_root); the parameter only swaps which end opens
 /// the session — the thing the owner's invariant says must not
 /// matter.
 async fn run_session(
     initiator_role: TransferRole,
     src_root: &Path,
     dst_root: &Path,
     plan_options: PlanOptions,
 ) -> (
     eyre::Result<TransferSummary>,
     eyre::Result<DestinationOutcome>,
 ) {
     let open = basic_open(initiator_role);
     let (source_endpoint, dest_endpoint) = match initiator_role {
         TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
         TransferRole::Destination => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
         TransferRole::Unspecified => panic!("fixture must pick a role"),
     };
     let source_cfg = SourceSessionConfig {
         hello: HelloConfig::default(),
         endpoint: source_endpoint,
         plan_options,
+        data_plane_host: None,
     };
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: dest_endpoint,
     };
     let (a, b) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root.to_path_buf()));
     tokio::time::timeout(SUITE_TIMEOUT, async {
         tokio::join!(
             run_source(source_cfg, a, source),
             run_destination(
                 dest_cfg,
                 b,
                 DestinationTarget::Fixed(dst_root.to_path_buf())
             ),
         )
     })
     .await
     .expect("session run timed out")
 }
 
 /// Run the same fixture under both role assignments (fresh trees per
 /// run) and pin the invariance property: identical need sets,
 /// identical summaries, byte-identical destinations.
 async fn assert_invariant_across_roles(
     src_files: &[FileSpec],
     dst_files: &[FileSpec],
     plan_options: PlanOptions,
 ) -> (TransferSummary, Vec<String>) {
     let mut per_role: Vec<(TransferSummary, Vec<String>)> = Vec::new();
     for initiator_role in [TransferRole::Source, TransferRole::Destination] {
         let tmp = tempfile::tempdir().unwrap();
         let src_root = tmp.path().join("src");
         let dst_root = tmp.path().join("dst");
         std::fs::create_dir_all(&src_root).unwrap();
         std::fs::create_dir_all(&dst_root).unwrap();
         write_tree(&src_root, src_files);
         write_tree(&dst_root, dst_files);
 
         let (source_result, dest_result) =
             run_session(initiator_role, &src_root, &dst_root, plan_options).await;
         let source_summary = source_result
             .unwrap_or_else(|e| panic!("source failed under initiator {initiator_role:?}: {e:#}"));
         let dest_outcome = dest_result.unwrap_or_else(|e| {
             panic!("destination failed under initiator {initiator_role:?}: {e:#}")
         });
 
         assert_eq!(
             source_summary, dest_outcome.summary,
             "both ends must hold the same summary (initiator {initiator_role:?})"
         );
         assert!(
             source_summary.in_stream_carrier_used,
             "otp-3 sessions ride the in-stream carrier"
         );
         assert_trees_identical(&src_root, &dst_root);
 
         let mut needed = dest_outcome.needed_paths.clone();
         needed.sort();
         per_role.push((dest_outcome.summary, needed));
     }
 
     let (summary_a, needed_a) = per_role.remove(0);
     let (summary_b, needed_b) = per_role.remove(0);
     assert_eq!(
         needed_a, needed_b,
         "need-list set must be identical whichever end initiates"
     );
     assert_eq!(
         summary_a, summary_b,
         "summary must be identical whichever end initiates"
     );
     (summary_a, needed_a)
 }
 
 fn fault_of(err: &eyre::Report) -> &SessionFault {
     err.downcast_ref::<SessionFault>()
         .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
 }
 
@@ -294,488 +295,494 @@ async fn incremental_transfer_needs_only_missing_and_changed() {
         ("newer.txt", b"old-eight".to_vec(), 1_600_000_100),
     ];
     let (summary, needed) = assert_invariant_across_roles(&src, &dst, PlanOptions::default()).await;
     assert_eq!(
         needed,
         vec!["newer.txt".to_string(), "sub/missing.txt".to_string()],
         "need list must be exactly the changed + missing files"
     );
     assert_eq!(summary.files_transferred, 2);
     assert_eq!(summary.bytes_transferred, 9 + 5);
 }
 
 #[tokio::test]
 async fn preexisting_identical_tree_yields_empty_need_list() {
     let files: Vec<FileSpec> = vec![
         ("one.txt", b"matching".to_vec(), 1_600_000_400),
         ("nested/two.txt", b"also matching".to_vec(), 1_600_000_500),
     ];
     let (summary, needed) =
         assert_invariant_across_roles(&files, &files, PlanOptions::default()).await;
     assert!(needed.is_empty(), "identical trees must need nothing");
     assert_eq!(summary.files_transferred, 0);
     assert_eq!(summary.bytes_transferred, 0);
 }
 
 #[tokio::test]
 async fn preserves_mtime_on_streamed_files() {
     // Not part of the role matrix — pins that the file-record write
     // path applies the manifest mtime (parity with today's receive
     // paths, which the byte-identical asserts alone wouldn't catch).
     let tmp = tempfile::tempdir().unwrap();
     let src_root = tmp.path().join("src");
     let dst_root = tmp.path().join("dst");
     std::fs::create_dir_all(&src_root).unwrap();
     std::fs::create_dir_all(&dst_root).unwrap();
     write_tree(
         &src_root,
         &[("stamped.txt", b"stamp me".to_vec(), 1_555_555_555)],
     );
 
     let (source_result, dest_result) = run_session(
         TransferRole::Source,
         &src_root,
         &dst_root,
         PlanOptions::default(),
     )
     .await;
     source_result.unwrap();
     dest_result.unwrap();
 
     let meta = std::fs::metadata(dst_root.join("stamped.txt")).unwrap();
     let mtime = filetime::FileTime::from_last_modification_time(&meta);
     assert_eq!(mtime.unix_seconds(), 1_555_555_555);
 }
 
 // ---------------------------------------------------------------------------
 // Handshake refusals
 // ---------------------------------------------------------------------------
 
 #[tokio::test]
 async fn build_mismatch_refused_under_both_initiators() {
     for initiator_role in [TransferRole::Source, TransferRole::Destination] {
         let tmp = tempfile::tempdir().unwrap();
         let src_root = tmp.path().join("src");
         let dst_root = tmp.path().join("dst");
         std::fs::create_dir_all(&src_root).unwrap();
         std::fs::create_dir_all(&dst_root).unwrap();
 
         let open = basic_open(initiator_role);
         let (source_endpoint, dest_endpoint) = match initiator_role {
             TransferRole::Source => (SessionEndpoint::initiator(open), SessionEndpoint::Responder),
             _ => (SessionEndpoint::Responder, SessionEndpoint::initiator(open)),
         };
         let source_cfg = SourceSessionConfig {
             hello: HelloConfig {
                 build_id: "0.1.0+aaaaaaaaaaaa".into(),
                 contract_version: CONTRACT_VERSION,
             },
             endpoint: source_endpoint,
             plan_options: PlanOptions::default(),
+            data_plane_host: None,
         };
         let dest_cfg = DestinationSessionConfig {
             hello: HelloConfig {
                 build_id: "0.1.0+bbbbbbbbbbbb".into(),
                 contract_version: CONTRACT_VERSION,
             },
             endpoint: dest_endpoint,
         };
         let (a, b) = in_process_pair();
         let source = Arc::new(FsTransferSource::new(src_root.clone()));
         let (source_result, dest_result) = tokio::time::timeout(SUITE_TIMEOUT, async {
             tokio::join!(
                 run_source(source_cfg, a, source),
                 run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root.clone())),
             )
         })
         .await
         .unwrap();
 
         for (end, err) in [
             ("source", source_result.unwrap_err()),
             ("destination", dest_result.err().unwrap()),
         ] {
             let fault = fault_of(&err);
             assert_eq!(
                 fault.code,
                 session_error::Code::BuildMismatch,
                 "{end} must refuse with BUILD_MISMATCH (initiator {initiator_role:?})"
             );
             assert!(
                 fault.message.contains("aaaaaaaaaaaa") && fault.message.contains("bbbbbbbbbbbb"),
                 "{end} must name both build ids, got: {}",
                 fault.message
             );
         }
         assert!(
             collect_tree(&dst_root).is_empty(),
             "no bytes may move on a refused handshake"
         );
     }
 }
 
 #[tokio::test]
 async fn contract_version_mismatch_is_refused() {
     let tmp = tempfile::tempdir().unwrap();
     let src_root = tmp.path().join("src");
     let dst_root = tmp.path().join("dst");
     std::fs::create_dir_all(&src_root).unwrap();
     std::fs::create_dir_all(&dst_root).unwrap();
 
     let source_cfg = SourceSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
         plan_options: PlanOptions::default(),
+        data_plane_host: None,
     };
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig {
             build_id: HelloConfig::default().build_id,
             contract_version: CONTRACT_VERSION + 1,
         },
         endpoint: SessionEndpoint::Responder,
     };
     let (a, b) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root));
     let (source_result, dest_result) = tokio::join!(
         run_source(source_cfg, a, source),
         run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root)),
     );
     assert_eq!(
         fault_of(&source_result.unwrap_err()).code,
         session_error::Code::BuildMismatch
     );
     assert_eq!(
         fault_of(&dest_result.err().unwrap()).code,
         session_error::Code::BuildMismatch
     );
 }
 
 #[tokio::test]
 async fn mirror_request_is_refused_until_its_slice_lands() {
     // otp-3 refuses what it does not implement rather than silently
     // ignoring it: a mirror-enabled open must fail the session at the
     // OPEN phase, from the destination (the end that would execute
     // deletions).
     let tmp = tempfile::tempdir().unwrap();
     let src_root = tmp.path().join("src");
     let dst_root = tmp.path().join("dst");
     std::fs::create_dir_all(&src_root).unwrap();
     std::fs::create_dir_all(&dst_root).unwrap();
 
     let mut open = basic_open(TransferRole::Source);
     open.mirror_enabled = true;
     let source_cfg = SourceSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::initiator(open),
         plan_options: PlanOptions::default(),
+        data_plane_host: None,
     };
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::Responder,
     };
     let (a, b) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root));
     let (source_result, dest_result) = tokio::join!(
         run_source(source_cfg, a, source),
         run_destination(dest_cfg, b, DestinationTarget::Fixed(dst_root)),
     );
     let source_fault = fault_of(&source_result.unwrap_err()).clone();
     assert_eq!(source_fault.code, session_error::Code::Internal);
     assert!(
         source_fault.message.contains("otp-6"),
         "refusal must say when mirror lands, got: {}",
         source_fault.message
     );
     assert!(dest_result.is_err());
 }
 
 // ---------------------------------------------------------------------------
 // Protocol-violation fail-fast (scripted peer)
 // ---------------------------------------------------------------------------
 
 fn wire(frame: Frame) -> TransferFrame {
     TransferFrame { frame: Some(frame) }
 }
 
 async fn recv_or_panic(t: &mut FrameTransport) -> Frame {
     t.recv()
         .await
         .unwrap()
         .expect("peer closed unexpectedly")
         .frame
         .expect("empty frame")
 }
 
 fn hello_frame() -> TransferFrame {
     let hello = HelloConfig::default();
     wire(Frame::Hello(SessionHello {
         build_id: hello.build_id,
         contract_version: hello.contract_version,
     }))
 }
 
 #[tokio::test]
 async fn payload_record_before_manifest_complete_is_protocol_violation() {
     let tmp = tempfile::tempdir().unwrap();
     let dst_root = tmp.path().join("dst");
     std::fs::create_dir_all(&dst_root).unwrap();
 
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::Responder,
     };
     let (mut peer, dest_transport) = in_process_pair();
     let dest = tokio::spawn(run_destination(
         dest_cfg,
         dest_transport,
         DestinationTarget::Fixed(dst_root),
     ));
 
     // Scripted source peer: valid handshake, then a payload record
     // while its manifest is still open — the contract's example
     // violation ("payload records may begin only AFTER the source's
     // ManifestComplete").
     peer.send(hello_frame()).await.unwrap();
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
     peer.send(wire(Frame::Open(basic_open(TransferRole::Source))))
         .await
         .unwrap();
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
 
     let header = FileHeader {
         relative_path: "early.bin".into(),
         size: 4,
         mtime_seconds: 1_600_000_000,
         permissions: 0o644,
         checksum: vec![],
     };
     peer.send(wire(Frame::ManifestEntry(header.clone())))
         .await
         .unwrap();
     peer.send(wire(Frame::FileBegin(header))).await.unwrap();
 
     // The destination must answer with a SessionError frame naming
     // the violation...
     let refusal = loop {
         match recv_or_panic(&mut peer).await {
             Frame::Error(e) => break e,
             // need batches may legitimately arrive first
             Frame::NeedBatch(_) | Frame::NeedComplete(_) => continue,
             other => panic!("expected SessionError, got {other:?}"),
         }
     };
     assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
 
     // ...and its driver must fail with the same fault.
     let dest_err = dest.await.unwrap().unwrap_err();
     assert_eq!(
         fault_of(&dest_err).code,
         session_error::Code::ProtocolViolation
     );
     assert!(
         collect_tree(tmp.path()).is_empty(),
         "no bytes may land from a violating record"
     );
 }
 
 #[tokio::test]
 async fn need_for_unknown_path_faults_the_source() {
     let tmp = tempfile::tempdir().unwrap();
     let src_root = tmp.path().join("src");
     std::fs::create_dir_all(&src_root).unwrap();
     write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_000_000)]);
 
     let source_cfg = SourceSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
         plan_options: PlanOptions::default(),
+        data_plane_host: None,
     };
     let (source_transport, mut peer) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root));
     let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
 
     // Scripted destination peer: valid handshake, then a need for a
     // path that was never manifested.
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
     peer.send(hello_frame()).await.unwrap();
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
     peer.send(wire(Frame::Accept(Default::default())))
         .await
         .unwrap();
     loop {
         match recv_or_panic(&mut peer).await {
             Frame::ManifestEntry(_) => continue,
             Frame::ManifestComplete(_) => break,
             other => panic!("expected manifest stream, got {other:?}"),
         }
     }
     peer.send(wire(Frame::NeedBatch(NeedBatch {
         entries: vec![NeedEntry {
             relative_path: "never-manifested.txt".into(),
             resume: false,
         }],
     })))
     .await
     .unwrap();
 
     let source_err = source_task.await.unwrap().unwrap_err();
     let fault = fault_of(&source_err);
     assert_eq!(fault.code, session_error::Code::ProtocolViolation);
     assert!(fault.message.contains("never-manifested.txt"));
 
     // The source must have told the peer why before aborting.
     let refusal = match recv_or_panic(&mut peer).await {
         Frame::Error(e) => e,
         other => panic!("expected SessionError, got {other:?}"),
     };
     assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
 }
 
 #[tokio::test]
 async fn resume_flagged_need_is_refused_in_non_resume_session() {
     let tmp = tempfile::tempdir().unwrap();
     let src_root = tmp.path().join("src");
     std::fs::create_dir_all(&src_root).unwrap();
     write_tree(&src_root, &[("real.txt", b"real".to_vec(), 1_600_000_000)]);
 
     let source_cfg = SourceSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
         plan_options: PlanOptions::default(),
+        data_plane_host: None,
     };
     let (source_transport, mut peer) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root));
     let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
 
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
     peer.send(hello_frame()).await.unwrap();
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
     peer.send(wire(Frame::Accept(Default::default())))
         .await
         .unwrap();
     loop {
         match recv_or_panic(&mut peer).await {
             Frame::ManifestEntry(_) => continue,
             Frame::ManifestComplete(_) => break,
             other => panic!("expected manifest stream, got {other:?}"),
         }
     }
     peer.send(wire(Frame::NeedBatch(NeedBatch {
         entries: vec![NeedEntry {
             relative_path: "real.txt".into(),
             resume: true,
         }],
     })))
     .await
     .unwrap();
 
     let source_err = source_task.await.unwrap().unwrap_err();
     assert_eq!(
         fault_of(&source_err).code,
         session_error::Code::ProtocolViolation
     );
 }
 
 #[tokio::test]
 async fn need_complete_before_manifest_complete_faults_the_source() {
     // codex otp-3 F2: NeedComplete is only legal after the source's
     // ManifestComplete has been received (contract §Phase state
     // machine). A peer promising "nothing further needed" before it
     // could have seen the full manifest must fail the session fast,
     // not end it as an empty transfer. The 500-entry manifest plus a
     // peer that reads nothing until after its early NeedComplete
     // keeps the source provably mid-manifest (64-frame transport
     // cap) when the violation is processed.
     let tmp = tempfile::tempdir().unwrap();
     let src_root = tmp.path().join("src");
     std::fs::create_dir_all(&src_root).unwrap();
     let mut files: Vec<FileSpec> = Vec::new();
     for i in 0..500 {
         let name: &'static str = Box::leak(format!("f{i:03}.txt").into_boxed_str());
         files.push((name, b"x".to_vec(), 1_600_000_000 + i as i64));
     }
     write_tree(&src_root, &files);
 
     let source_cfg = SourceSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::initiator(basic_open(TransferRole::Source)),
         plan_options: PlanOptions::default(),
+        data_plane_host: None,
     };
     let (source_transport, mut peer) = in_process_pair();
     let source = Arc::new(FsTransferSource::new(src_root));
     let source_task = tokio::spawn(run_source(source_cfg, source_transport, source));
 
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
     peer.send(hello_frame()).await.unwrap();
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Open(_)));
     peer.send(wire(Frame::Accept(Default::default())))
         .await
         .unwrap();
     // The violation: promise need-completion before reading a single
     // manifest frame.
     peer.send(wire(Frame::NeedComplete(NeedComplete {})))
         .await
         .unwrap();
 
     // The source must abort with a SessionError before its manifest
     // completes — never treat the early promise as a clean empty
     // transfer.
     let refusal = loop {
         match recv_or_panic(&mut peer).await {
             Frame::ManifestEntry(_) => continue,
             Frame::Error(e) => break e,
             Frame::ManifestComplete(_) => {
                 panic!("source completed its manifest instead of failing fast")
             }
             Frame::SourceDone(_) => panic!("source treated early NeedComplete as legitimate"),
             other => panic!("expected SessionError, got {other:?}"),
         }
     };
     assert_eq!(refusal.code, session_error::Code::ProtocolViolation as i32);
 
     let source_err = source_task.await.unwrap().unwrap_err();
     let fault = fault_of(&source_err);
     assert_eq!(fault.code, session_error::Code::ProtocolViolation);
     assert!(
         fault.message.contains("ManifestComplete"),
         "fault must name the ordering rule, got: {}",
         fault.message
     );
 }
 
 #[tokio::test]
 async fn manifest_entry_after_manifest_complete_is_protocol_violation() {
     let tmp = tempfile::tempdir().unwrap();
     let dst_root = tmp.path().join("dst");
     std::fs::create_dir_all(&dst_root).unwrap();
 
     let dest_cfg = DestinationSessionConfig {
         hello: HelloConfig::default(),
         endpoint: SessionEndpoint::Responder,
     };
     let (mut peer, dest_transport) = in_process_pair();
     let dest = tokio::spawn(run_destination(
         dest_cfg,
         dest_transport,
         DestinationTarget::Fixed(dst_root),
     ));
 
     peer.send(hello_frame()).await.unwrap();
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Hello(_)));
     peer.send(wire(Frame::Open(basic_open(TransferRole::Source))))
         .await
         .unwrap();
     assert!(matches!(recv_or_panic(&mut peer).await, Frame::Accept(_)));
 
     peer.send(wire(Frame::ManifestComplete(ManifestComplete {
         scan_complete: true,
     })))
     .await
     .unwrap();
     peer.send(wire(Frame::ManifestEntry(FileHeader {
         relative_path: "late.txt".into(),
         size: 1,
         mtime_seconds: 1,
         permissions: 0o644,
         checksum: vec![],
     })))
     .await
diff --git a/crates/blit-daemon/src/service/transfer_session_e2e.rs b/crates/blit-daemon/src/service/transfer_session_e2e.rs
index f93fa63..6ac93c3 100644
--- a/crates/blit-daemon/src/service/transfer_session_e2e.rs
+++ b/crates/blit-daemon/src/service/transfer_session_e2e.rs
@@ -1,89 +1,93 @@
-//! ONE_TRANSFER_PATH otp-4a loopback e2e: the daemon serves the unified
-//! `Transfer` session and a real client initiates it as SOURCE over
-//! gRPC (in-stream carrier). These tests replace the otp-1 UNIMPLEMENTED
-//! pin — the RPC now serves — and pin the push-equivalent behavior:
+//! ONE_TRANSFER_PATH otp-4a/4b loopback e2e: the daemon serves the
+//! unified `Transfer` session and a real client initiates it as SOURCE
+//! over gRPC. otp-4b makes the default carrier the **TCP data plane**
+//! (the responder grants it in `SessionAccept`, the client dials +
+//! authenticates + sends payloads over sockets); the in-stream carrier
+//! stays live as the requested fallback. These tests pin the
+//! push-equivalent behavior over both carriers:
 //!
-//! - a session lands bytes byte-identically and scores them correctly;
+//! - a session lands bytes byte-identically and scores them correctly,
+//!   over the data plane and over the in-stream fallback;
 //! - **A/B parity**: the same fixture through OLD push and the NEW
-//!   session yields byte-identical destination trees + equal shared
-//!   summary counters (the converge-up bar, in-stream);
+//!   session (data plane) yields byte-identical destination trees +
+//!   equal shared summary counters (the converge-up bar);
 //! - responder refusals (read-only module, unknown module) arrive as
 //!   `SessionError` frames, surfaced to the client as faults;
 //! - the unified SizeMtime semantic: a same-size destination file that
 //!   is NEWER than the source is SKIPPED (the data-safe, pull-style
 //!   converged behavior — see the finding doc's compare decision).
 //!
 //! Harness mirrors `push/shape_resize_e2e.rs`: a real in-process
 //! `BlitService` on loopback + a real client. Only in-crate tests can
 //! build `ModuleConfig`/`BlitService::with_modules`, so this lives in
 //! blit-daemon.
 
 use std::collections::{BTreeMap, HashMap};
 use std::path::{Path, PathBuf};
 use std::sync::Arc;
 
 use blit_core::fs_enum::FileFilter;
 use blit_core::generated::blit_server::BlitServer;
 use blit_core::generated::{session_error, MirrorMode};
 use blit_core::remote::transfer::session_client::{run_push_session, PushSessionOptions};
 use blit_core::remote::transfer::source::FsTransferSource;
 use blit_core::remote::{RemoteEndpoint, RemotePath, RemotePushClient};
 use blit_core::transfer_session::SessionFault;
 use tokio::sync::oneshot;
 
 use crate::runtime::ModuleConfig;
 use crate::service::BlitService;
 
 // ---------------------------------------------------------------------------
 // Harness
 // ---------------------------------------------------------------------------
 
 /// A running in-process daemon exposing module "test" over a writable
 /// (or read-only) temp dir, and the loopback endpoint targeting it.
 struct Daemon {
     endpoint: RemoteEndpoint,
     shutdown: Option<oneshot::Sender<()>>,
     server: Option<tokio::task::JoinHandle<()>>,
     _dest: tempfile::TempDir,
     dest_root: PathBuf,
 }
 
 impl Daemon {
     async fn start(read_only: bool) -> Self {
         let dest = tempfile::tempdir().expect("dest dir");
         let canonical = dest.path().canonicalize().expect("canonical dest");
         let mut modules = HashMap::new();
         modules.insert(
             "test".to_string(),
             ModuleConfig {
                 name: "test".into(),
                 path: canonical.clone(),
                 canonical_root: canonical.clone(),
                 read_only,
                 _comment: None,
                 delegation_allowed: true,
             },
         );
         let service = BlitService::with_modules(modules, false);
         let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
             .await
             .expect("bind loopback listener");
         let port = listener.local_addr().expect("listener addr").port();
         let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
         let server = tokio::spawn(async move {
             blit_core::remote::grpc_server::production_server_builder()
                 .add_service(BlitServer::new(service))
                 .serve_with_incoming_shutdown(
                     tokio_stream::wrappers::TcpListenerStream::new(listener),
                     async {
                         let _ = shutdown_rx.await;
                     },
                 )
                 .await
                 .expect("in-process daemon serves");
         });
         let endpoint = RemoteEndpoint {
             host: "127.0.0.1".into(),
             port,
             path: RemotePath::Module {
                 module: "test".into(),
@@ -116,181 +120,213 @@ impl Daemon {
             let _ = tx.send(());
         }
         if let Some(server) = self.server.take() {
             server.await.expect("server task joins");
         }
     }
 }
 
 type FileSpec = (&'static str, &'static [u8], i64);
 
 fn write_tree(root: &Path, files: &[FileSpec]) {
     for (rel, content, mtime) in files {
         let path = root.join(rel);
         if let Some(parent) = path.parent() {
             std::fs::create_dir_all(parent).unwrap();
         }
         std::fs::write(&path, content).unwrap();
         filetime::set_file_mtime(&path, filetime::FileTime::from_unix_time(*mtime, 0)).unwrap();
     }
 }
 
 /// rel-path → bytes for every regular file under `root`. Content only
 /// (byte-identical), copied from the role suite — no shared test util
 /// exists across crates yet.
 fn collect_tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
     fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
         for entry in std::fs::read_dir(dir).unwrap() {
             let entry = entry.unwrap();
             let path = entry.path();
             if path.is_dir() {
                 walk(root, &path, out);
             } else {
                 let rel = path
                     .strip_prefix(root)
                     .unwrap()
                     .to_string_lossy()
                     .replace('\\', "/");
                 out.insert(rel, std::fs::read(&path).unwrap());
             }
         }
     }
     let mut out = BTreeMap::new();
     if root.exists() {
         walk(root, root, &mut out);
     }
     out
 }
 
 fn assert_trees_identical(a: &Path, b: &Path) {
     let ta = collect_tree(a);
     let tb = collect_tree(b);
     assert_eq!(
         ta.keys().collect::<Vec<_>>(),
         tb.keys().collect::<Vec<_>>(),
         "path sets differ between {a:?} and {b:?}"
     );
     for (rel, bytes) in &ta {
         assert_eq!(bytes, &tb[rel], "content differs for '{rel}'");
     }
 }
 
 fn small_tree() -> Vec<FileSpec> {
     vec![
         ("a.txt", b"alpha", 1_600_000_001),
         ("empty.bin", b"", 1_600_000_002),
         ("dir one/b.log", b"beta beta beta", 1_600_000_003),
         ("dir one/deeper/c.dat", b"gamma-content", 1_600_000_004),
     ]
 }
 
 fn fault_of(err: &eyre::Report) -> &SessionFault {
     err.downcast_ref::<SessionFault>()
         .unwrap_or_else(|| panic!("expected a SessionFault, got: {err:#}"))
 }
 
 // ---------------------------------------------------------------------------
 // Tests
 // ---------------------------------------------------------------------------
 
 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
-async fn session_lands_bytes_and_scores_them() {
+async fn session_lands_bytes_over_the_data_plane() {
     let daemon = Daemon::start(false).await;
     let src = tempfile::tempdir().unwrap();
     write_tree(src.path(), &small_tree());
 
+    // Default options ⇒ TCP data plane: the responder grants it and the
+    // client dials + sends payloads over sockets (otp-4b).
     let source = Arc::new(FsTransferSource::new(src.path().to_path_buf()));
     let summary = run_push_session(&daemon.endpoint, source, PushSessionOptions::default())
         .await
         .expect("session push succeeds");
 
     assert_eq!(summary.files_transferred, small_tree().len() as u64);
     assert_eq!(
         summary.bytes_transferred,
         small_tree()
             .iter()
             .map(|(_, c, _)| c.len() as u64)
             .sum::<u64>()
     );
+    assert!(
+        !summary.in_stream_carrier_used,
+        "otp-4b default rides the TCP data plane, not the in-stream carrier"
+    );
+    assert_trees_identical(src.path(), &daemon.dest_root);
+    daemon.stop().await;
+}
+
+#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
+async fn session_lands_bytes_over_in_stream_carrier() {
+    // The in-stream carrier is the fallback (diagnostics / unreachable
+    // data plane). Requesting it must still land bytes byte-identically
+    // and score them — the otp-4a path stays live under otp-4b.
+    let daemon = Daemon::start(false).await;
+    let src = tempfile::tempdir().unwrap();
+    write_tree(src.path(), &small_tree());
+
+    let source = Arc::new(FsTransferSource::new(src.path().to_path_buf()));
+    let summary = run_push_session(
+        &daemon.endpoint,
+        source,
+        PushSessionOptions {
+            in_stream_bytes: true,
+            ..PushSessionOptions::default()
+        },
+    )
+    .await
+    .expect("in-stream session push succeeds");
+
+    assert_eq!(summary.files_transferred, small_tree().len() as u64);
     assert!(
         summary.in_stream_carrier_used,
-        "otp-4a rides the in-stream carrier"
+        "an in_stream_bytes request rides the in-stream carrier"
     );
     assert_trees_identical(src.path(), &daemon.dest_root);
     daemon.stop().await;
 }
 
 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
 async fn old_push_and_session_produce_identical_trees_and_counts() {
     let src = tempfile::tempdir().unwrap();
     write_tree(src.path(), &small_tree());
 
     // Arm A: OLD push.
     let daemon_a = Daemon::start(false).await;
     let mut push_client = RemotePushClient::connect(daemon_a.endpoint.clone())
         .await
         .expect("old push client connects");
     let report = push_client
         .push(
             Arc::new(FsTransferSource::new(src.path().to_path_buf())),
             &FileFilter::default(),
             false,
             MirrorMode::FilteredSubset,
             false,
             false,
             None,
             false,
         )
         .await
         .expect("old push succeeds");
 
     // Arm B: NEW session.
     let daemon_b = Daemon::start(false).await;
     let summary = run_push_session(
         &daemon_b.endpoint,
         Arc::new(FsTransferSource::new(src.path().to_path_buf())),
         PushSessionOptions::default(),
     )
     .await
     .expect("session push succeeds");
 
     // Both destinations equal the source and each other.
     assert_trees_identical(src.path(), &daemon_a.dest_root);
     assert_trees_identical(src.path(), &daemon_b.dest_root);
     assert_trees_identical(&daemon_a.dest_root, &daemon_b.dest_root);
 
     // Shared summary counters agree (transport-specific fields —
     // tcp_fallback_used/bytes_zero_copy vs in_stream_carrier_used — have
     // no cross analog and are not compared).
     assert_eq!(report.summary.files_transferred, summary.files_transferred);
     assert_eq!(report.summary.bytes_transferred, summary.bytes_transferred);
     assert_eq!(report.summary.entries_deleted, summary.entries_deleted);
 
     daemon_a.stop().await;
     daemon_b.stop().await;
 }
 
 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
 async fn read_only_module_refuses_the_session() {
     let daemon = Daemon::start(true).await; // read-only
     let src = tempfile::tempdir().unwrap();
     write_tree(src.path(), &[("a.txt", b"alpha", 1_600_000_001)]);
 
     let err = run_push_session(
         &daemon.endpoint,
         Arc::new(FsTransferSource::new(src.path().to_path_buf())),
         PushSessionOptions::default(),
     )
     .await
     .expect_err("read-only module must refuse the session");
     assert_eq!(fault_of(&err).code, session_error::Code::ReadOnly);
     assert!(
         collect_tree(&daemon.dest_root).is_empty(),
         "no bytes may land on a refused session"
     );
     daemon.stop().await;
 }
 
 #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
 async fn unknown_module_refuses_the_session() {
     let daemon = Daemon::start(false).await;
     let src = tempfile::tempdir().unwrap();

codex
The main receive path does join the data-plane task at `SourceDone`, and the source does finish the socket before sending `SourceDone`. I’m now tracing the reused receive/sink code, because the completion check and path containment depend on what `SinkOutcome.files_written` actually counts.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1120,1500p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1120	    ));
  1121	    // Same canonical-containment chokepoint the sink write paths use
  1122	    // (R46-F3), applied to diff stats so a hostile manifest path can't
  1123	    // make the destination stat outside its root.
  1124	    let canonical_dst_root = crate::path_safety::canonical_dest_root(dst_root).ok();
  1125	
  1126	    // Data plane (otp-4b): when the responder granted a TCP data plane,
  1127	    // payload bytes arrive on sockets (not the control lane). Arm the
  1128	    // accept+receive task NOW — concurrent with the diff loop below, and
  1129	    // before the source dials — so the connections are accepted promptly.
  1130	    // AbortOnDrop bounds it to this future: a control-lane fault that
  1131	    // returns from this fn aborts the receive task instead of leaking it.
  1132	    let mut data_plane_recv = negotiated.responder_data_plane.map(|rdp| {
  1133	        let sink: Arc<dyn TransferSink> = Arc::clone(&sink) as Arc<dyn TransferSink>;
  1134	        AbortOnDrop::new(tokio::spawn(rdp.accept_and_receive(sink)))
  1135	    });
  1136	
  1137	    let mut pending: Vec<FileHeader> = Vec::new();
  1138	    let mut outstanding: HashSet<String> = HashSet::new();
  1139	    let mut needed_paths: Vec<String> = Vec::new();
  1140	    let mut manifest_complete = false;
  1141	    let mut files_written: u64 = 0;
  1142	    let mut bytes_written: u64 = 0;
  1143	
  1144	    loop {
  1145	        let received = match transport.recv().await? {
  1146	            Some(f) => f,
  1147	            None => {
  1148	                return Err(eyre::Report::new(SessionFault::internal(
  1149	                    "peer closed mid-session",
  1150	                )))
  1151	            }
  1152	        };
  1153	        match received.frame {
  1154	            Some(Frame::ManifestEntry(header)) => {
  1155	                if manifest_complete {
  1156	                    return Err(violation(format!(
  1157	                        "manifest entry '{}' after ManifestComplete",
  1158	                        header.relative_path
  1159	                    )));
  1160	                }
  1161	                pending.push(header);
  1162	                if pending.len() >= DEST_DIFF_CHUNK {
  1163	                    let chunk = std::mem::take(&mut pending);
  1164	                    diff_chunk_and_send_needs(
  1165	                        transport,
  1166	                        chunk,
  1167	                        dst_root,
  1168	                        canonical_dst_root.as_deref(),
  1169	                        &compare_opts,
  1170	                        &mut outstanding,
  1171	                        &mut needed_paths,
  1172	                    )
  1173	                    .await?;
  1174	                }
  1175	            }
  1176	            Some(Frame::ManifestComplete(_complete)) => {
  1177	                if manifest_complete {
  1178	                    return Err(violation("duplicate ManifestComplete".into()));
  1179	                }
  1180	                // (scan_complete gates mirror purges from otp-6 on;
  1181	                // nothing consumes it in otp-3.)
  1182	                let chunk = std::mem::take(&mut pending);
  1183	                diff_chunk_and_send_needs(
  1184	                    transport,
  1185	                    chunk,
  1186	                    dst_root,
  1187	                    canonical_dst_root.as_deref(),
  1188	                    &compare_opts,
  1189	                    &mut outstanding,
  1190	                    &mut needed_paths,
  1191	                )
  1192	                .await?;
  1193	                // NeedComplete only after ManifestComplete received
  1194	                // AND every entry diffed — both true here.
  1195	                transport
  1196	                    .send(frame(Frame::NeedComplete(NeedComplete {})))
  1197	                    .await?;
  1198	                manifest_complete = true;
  1199	            }
  1200	            Some(Frame::FileBegin(header)) => {
  1201	                // Payload records ride the control lane only under the
  1202	                // in-stream carrier; with a TCP data plane active they
  1203	                // flow over the sockets, so one here is a violation.
  1204	                if data_plane_recv.is_some() {
  1205	                    return Err(violation(format!(
  1206	                        "file record '{}' on the control lane while a TCP data plane is active",
  1207	                        header.relative_path
  1208	                    )));
  1209	                }
  1210	                if !manifest_complete {
  1211	                    return Err(violation(format!(
  1212	                        "payload record for '{}' before ManifestComplete",
  1213	                        header.relative_path
  1214	                    )));
  1215	                }
  1216	                if !outstanding.remove(&header.relative_path) {
  1217	                    return Err(violation(format!(
  1218	                        "payload for '{}' which is not on the need list",
  1219	                        header.relative_path
  1220	                    )));
  1221	                }
  1222	                let outcome = receive_file_record(transport, &sink, &header).await?;
  1223	                files_written += outcome.files_written as u64;
  1224	                bytes_written += outcome.bytes_written;
  1225	            }
  1226	            Some(Frame::TarShardHeader(shard)) => {
  1227	                if data_plane_recv.is_some() {
  1228	                    return Err(violation(
  1229	                        "tar shard record on the control lane while a TCP data plane is active"
  1230	                            .into(),
  1231	                    ));
  1232	                }
  1233	                if !manifest_complete {
  1234	                    return Err(violation("tar shard record before ManifestComplete".into()));
  1235	                }
  1236	                for h in &shard.files {
  1237	                    if !outstanding.remove(&h.relative_path) {
  1238	                        return Err(violation(format!(
  1239	                            "tar shard entry '{}' which is not on the need list",
  1240	                            h.relative_path
  1241	                        )));
  1242	                    }
  1243	                }
  1244	                let outcome = receive_tar_record(transport, &sink, shard).await?;
  1245	                files_written += outcome.files_written as u64;
  1246	                bytes_written += outcome.bytes_written;
  1247	            }
  1248	            Some(Frame::SourceDone(_)) => {
  1249	                if !manifest_complete {
  1250	                    return Err(violation("SourceDone before ManifestComplete".into()));
  1251	                }
  1252	                // Carrier-specific completion. In-stream: every payload
  1253	                // was consumed inline, so the need set must be fully
  1254	                // drained. Data plane: payloads rode the sockets (the
  1255	                // control lane never removed them from `outstanding`), so
  1256	                // join the receive task for the authoritative counts and
  1257	                // verify it delivered exactly the need list.
  1258	                let in_stream_carrier_used = match data_plane_recv.take() {
  1259	                    Some(recv) => {
  1260	                        let outcome = recv.join().await.map_err(|err| {
  1261	                            eyre::Report::new(SessionFault::internal(format!(
  1262	                                "data-plane receive task panicked: {err}"
  1263	                            )))
  1264	                        })??;
  1265	                        files_written = outcome.files_written as u64;
  1266	                        bytes_written = outcome.bytes_written;
  1267	                        if files_written != needed_paths.len() as u64 {
  1268	                            return Err(violation(format!(
  1269	                                "data plane delivered {} of {} needed file(s) before SourceDone",
  1270	                                files_written,
  1271	                                needed_paths.len()
  1272	                            )));
  1273	                        }
  1274	                        false
  1275	                    }
  1276	                    None => {
  1277	                        if !outstanding.is_empty() {
  1278	                            return Err(violation(format!(
  1279	                                "SourceDone with {} needed file(s) never sent",
  1280	                                outstanding.len()
  1281	                            )));
  1282	                        }
  1283	                        true
  1284	                    }
  1285	                };
  1286	                let summary = TransferSummary {
  1287	                    files_transferred: files_written,
  1288	                    bytes_transferred: bytes_written,
  1289	                    entries_deleted: 0, // mirror lands at otp-6
  1290	                    in_stream_carrier_used,
  1291	                    files_resumed: 0, // resume lands at otp-7
  1292	                };
  1293	                transport.send(frame(Frame::Summary(summary))).await?;
  1294	                return Ok(DestinationOutcome {
  1295	                    summary,
  1296	                    needed_paths,
  1297	                });
  1298	            }
  1299	            Some(Frame::Error(err)) => {
  1300	                return Err(eyre::Report::new(SessionFault::from_wire(err)));
  1301	            }
  1302	            other => {
  1303	                // Everything else is off-lane or off-phase here:
  1304	                // destination-lane frames echoed back, resume frames
  1305	                // in a non-resume session (otp-7), resize with no
  1306	                // data plane to resize (otp-4), stray handshake
  1307	                // frames, bare FileData/TarShardChunk outside a
  1308	                // record. Fail fast, no tolerant parsing.
  1309	                return Err(violation(format!(
  1310	                    "{} not valid on the destination's receive lane in this phase",
  1311	                    frame_name(&other)
  1312	                )));
  1313	            }
  1314	        }
  1315	    }
  1316	}
  1317	
  1318	/// Stat-and-compare one chunk of manifest entries on the blocking
  1319	/// pool (2+ syscalls per entry — same rationale as the daemon's
  1320	/// w4-4 chunked checks), then stream the resulting need batch.
  1321	async fn diff_chunk_and_send_needs(
  1322	    transport: &mut FrameTransport,
  1323	    chunk: Vec<FileHeader>,
  1324	    dst_root: &Path,
  1325	    canonical_dst_root: Option<&Path>,
  1326	    compare_opts: &CompareOptions,
  1327	    outstanding: &mut HashSet<String>,
  1328	    needed_paths: &mut Vec<String>,
  1329	) -> Result<()> {
  1330	    if chunk.is_empty() {
  1331	        return Ok(());
  1332	    }
  1333	    let dst_root = dst_root.to_path_buf();
  1334	    let canonical = canonical_dst_root.map(Path::to_path_buf);
  1335	    let opts = compare_opts.clone();
  1336	    let needed: Vec<String> = tokio::task::spawn_blocking(move || -> Result<Vec<String>> {
  1337	        let mut needed = Vec::new();
  1338	        for header in &chunk {
  1339	            if destination_needs(header, &dst_root, canonical.as_deref(), &opts)? {
  1340	                needed.push(header.relative_path.clone());
  1341	            }
  1342	        }
  1343	        Ok(needed)
  1344	    })
  1345	    .await
  1346	    .map_err(|err| eyre::eyre!("destination diff task panicked: {err}"))??;
  1347	
  1348	    let entries: Vec<NeedEntry> = needed
  1349	        .into_iter()
  1350	        // A path the source manifests twice is diffed twice but
  1351	        // needed at most once.
  1352	        .filter(|path| outstanding.insert(path.clone()))
  1353	        .map(|relative_path| {
  1354	            needed_paths.push(relative_path.clone());
  1355	            NeedEntry {
  1356	                relative_path,
  1357	                resume: false, // resume lands at otp-7
  1358	            }
  1359	        })
  1360	        .collect();
  1361	    if entries.is_empty() {
  1362	        return Ok(());
  1363	    }
  1364	    transport
  1365	        .send(frame(Frame::NeedBatch(NeedBatch { entries })))
  1366	        .await?;
  1367	    Ok(())
  1368	}
  1369	
  1370	/// Does the destination need this manifest entry? Stats its own file
  1371	/// and delegates the verdict to `manifest::header_transfer_status` —
  1372	/// the same mode-aware owner `compare_manifests` uses, fed from a
  1373	/// live stat instead of a materialized target manifest.
  1374	fn destination_needs(
  1375	    header: &FileHeader,
  1376	    dst_root: &Path,
  1377	    canonical_dst_root: Option<&Path>,
  1378	    opts: &CompareOptions,
  1379	) -> Result<bool> {
  1380	    let dst = match canonical_dst_root {
  1381	        Some(canonical) => {
  1382	            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
  1383	        }
  1384	        None => crate::path_safety::safe_join(dst_root, &header.relative_path),
  1385	    }
  1386	    .map_err(|err| {
  1387	        SessionFault::protocol_violation(format!(
  1388	            "manifest path '{}' escapes the destination root: {err:#}",
  1389	            header.relative_path
  1390	        ))
  1391	    })?;
  1392	
  1393	    let target = match std::fs::metadata(&dst) {
  1394	        Ok(meta) if meta.is_file() => {
  1395	            let mtime = match meta.modified() {
  1396	                Ok(t) => match t.duration_since(std::time::UNIX_EPOCH) {
  1397	                    Ok(d) => d.as_secs() as i64,
  1398	                    Err(e) => -(e.duration().as_secs() as i64),
  1399	                },
  1400	                Err(_) => 0,
  1401	            };
  1402	            Some((meta.len(), mtime))
  1403	        }
  1404	        // Absent — or present as a directory/other, which a file
  1405	        // write must replace: both diff as "target does not have it"
  1406	        // (matches the push daemon's file_requires_upload).
  1407	        _ => None,
  1408	    };
  1409	    let status = header_transfer_status(
  1410	        header,
  1411	        // Destination-side checksums are never precomputed; Checksum
  1412	        // mode therefore transfers (the conservative arm of
  1413	        // compare_file), matching what push does today.
  1414	        target.map(|(size, mtime)| (size, mtime, &[] as &[u8])),
  1415	        opts,
  1416	    );
  1417	    Ok(matches!(status, FileStatus::New | FileStatus::Modified))
  1418	}
  1419	
  1420	/// Receive one strictly-serialized file record (`file_begin` already
  1421	/// consumed) and stream its bytes into the sink through a bounded
  1422	/// in-memory pipe — record completion is exactly `header.size`
  1423	/// cumulative bytes (contract §Transport selection).
  1424	async fn receive_file_record(
  1425	    transport: &mut FrameTransport,
  1426	    sink: &FsTransferSink,
  1427	    header: &FileHeader,
  1428	) -> Result<crate::remote::transfer::SinkOutcome> {
  1429	    let (mut pipe_wr, mut pipe_rd) = tokio::io::duplex(FILE_RECORD_PIPE_BYTES);
  1430	    let write = sink.write_file_stream(header, &mut pipe_rd);
  1431	    let feed = async {
  1432	        let mut remaining = header.size;
  1433	        while remaining > 0 {
  1434	            let received = match transport.recv().await? {
  1435	                Some(f) => f,
  1436	                None => {
  1437	                    return Err(eyre::Report::new(SessionFault::internal(format!(
  1438	                        "peer closed inside file record '{}'",
  1439	                        header.relative_path
  1440	                    ))))
  1441	                }
  1442	            };
  1443	            match received.frame {
  1444	                Some(Frame::FileData(data)) => {
  1445	                    let len = data.content.len() as u64;
  1446	                    if len > remaining {
  1447	                        return Err(violation(format!(
  1448	                            "file record '{}' overran its size by {} byte(s)",
  1449	                            header.relative_path,
  1450	                            len - remaining
  1451	                        )));
  1452	                    }
  1453	                    pipe_wr.write_all(&data.content).await?;
  1454	                    remaining -= len;
  1455	                }
  1456	                other => {
  1457	                    // Strict serialization: nothing may interleave
  1458	                    // with an open record on the source lane.
  1459	                    return Err(violation(format!(
  1460	                        "{} inside file record '{}' ({} byte(s) short)",
  1461	                        frame_name(&other),
  1462	                        header.relative_path,
  1463	                        remaining
  1464	                    )));
  1465	                }
  1466	            }
  1467	        }
  1468	        pipe_wr.shutdown().await?;
  1469	        Ok(())
  1470	    };
  1471	    let (outcome, ()) = tokio::try_join!(write, feed)?;
  1472	    Ok(outcome)
  1473	}
  1474	
  1475	/// Receive one tar-shard record (`tar_shard_header` already consumed):
  1476	/// buffer to exactly `archive_size` (bounded by the shared tar cap)
  1477	/// and hand the archive to the sink's tar-safety unpack path.
  1478	async fn receive_tar_record(
  1479	    transport: &mut FrameTransport,
  1480	    sink: &FsTransferSink,
  1481	    shard: TarShardHeader,
  1482	) -> Result<crate::remote::transfer::SinkOutcome> {
  1483	    if shard.archive_size > MAX_TAR_SHARD_BYTES {
  1484	        return Err(violation(format!(
  1485	            "tar shard of {} bytes exceeds the {} byte cap",
  1486	            shard.archive_size, MAX_TAR_SHARD_BYTES
  1487	        )));
  1488	    }
  1489	    let mut data: Vec<u8> = Vec::new();
  1490	    data.try_reserve_exact(shard.archive_size as usize)
  1491	        .map_err(|err| eyre::eyre!("allocating {} byte tar shard: {err}", shard.archive_size))?;
  1492	    loop {
  1493	        let received = match transport.recv().await? {
  1494	            Some(f) => f,
  1495	            None => {
  1496	                return Err(eyre::Report::new(SessionFault::internal(
  1497	                    "peer closed inside tar shard record",
  1498	                )))
  1499	            }
  1500	        };

exec
/usr/bin/zsh -lc "rg -n \"struct SinkOutcome|files_written|write_tar_shard|write_file_stream|TransferSink|DataPlaneSink|root\\(\" crates/blit-core/src/remote/transfer/sink.rs crates/blit-core/src/remote/transfer/pipeline.rs crates/blit-core/src/remote/transfer/data_plane.rs" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/data_plane.rs:265:    /// reader (not a local file). Used by `DataPlaneSink` for the
crates/blit-core/src/remote/transfer/data_plane.rs:303:        // FsTransferSink without consulting an out-of-band manifest cache.
crates/blit-core/src/remote/transfer/pipeline.rs:16:use super::sink::{SinkOutcome, TransferSink};
crates/blit-core/src/remote/transfer/pipeline.rs:26:    sinks: Vec<Arc<dyn TransferSink>>,
crates/blit-core/src/remote/transfer/pipeline.rs:79:    sinks: Vec<Arc<dyn TransferSink>>,
crates/blit-core/src/remote/transfer/pipeline.rs:93:    Add(Arc<dyn TransferSink>),
crates/blit-core/src/remote/transfer/pipeline.rs:112:    sinks: Vec<Arc<dyn TransferSink>>,
crates/blit-core/src/remote/transfer/pipeline.rs:158:        sink: Arc<dyn TransferSink>,
crates/blit-core/src/remote/transfer/pipeline.rs:401:/// Drive a `TransferSink` from a TCP wire stream.
crates/blit-core/src/remote/transfer/pipeline.rs:412:/// Both directions converge on `TransferSink::write_payload`: file data
crates/blit-core/src/remote/transfer/pipeline.rs:413:/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
crates/blit-core/src/remote/transfer/pipeline.rs:419:    sink: Arc<dyn TransferSink>,
crates/blit-core/src/remote/transfer/pipeline.rs:447:                    .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/pipeline.rs:654:    use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
crates/blit-core/src/remote/transfer/pipeline.rs:673:    impl TransferSink for FailingSink {
crates/blit-core/src/remote/transfer/pipeline.rs:677:        fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/pipeline.rs:695:        let sink = Arc::new(FsTransferSink::new(
crates/blit-core/src/remote/transfer/pipeline.rs:717:            source.root(),
crates/blit-core/src/remote/transfer/pipeline.rs:726:        assert_eq!(outcome.files_written, 3);
crates/blit-core/src/remote/transfer/pipeline.rs:743:        let sink = Arc::new(FsTransferSink::new(
crates/blit-core/src/remote/transfer/pipeline.rs:765:            source.root(),
crates/blit-core/src/remote/transfer/pipeline.rs:786:        assert_eq!(outcome.files_written, 5);
crates/blit-core/src/remote/transfer/pipeline.rs:807:            Arc::new(FsTransferSink::new(
crates/blit-core/src/remote/transfer/pipeline.rs:817:            )) as Arc<dyn TransferSink>
crates/blit-core/src/remote/transfer/pipeline.rs:830:            source.root(),
crates/blit-core/src/remote/transfer/pipeline.rs:839:        assert_eq!(outcome.files_written, 8);
crates/blit-core/src/remote/transfer/pipeline.rs:855:        // Build a minimal FsTransferSink that writes to a temp dir.
crates/blit-core/src/remote/transfer/pipeline.rs:858:        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
crates/blit-core/src/remote/transfer/pipeline.rs:1106:    impl TransferSink for RecordingSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1108:            let (files_written, bytes_written) = match &payload {
crates/blit-core/src/remote/transfer/pipeline.rs:1115:                files_written,
crates/blit-core/src/remote/transfer/pipeline.rs:1120:        async fn write_file_stream(
crates/blit-core/src/remote/transfer/pipeline.rs:1128:                files_written: 1,
crates/blit-core/src/remote/transfer/pipeline.rs:1133:        fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/pipeline.rs:1139:        Arc<dyn TransferSink>,
crates/blit-core/src/remote/transfer/pipeline.rs:1143:        let sink: Arc<dyn TransferSink> = Arc::new(RecordingSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1276:        let sink = Arc::new(FsTransferSink::new(
crates/blit-core/src/remote/transfer/pipeline.rs:1297:            source.root(),
crates/blit-core/src/remote/transfer/pipeline.rs:1342:        let failing: Arc<dyn TransferSink> = Arc::new(FailingSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1357:            source.root(),
crates/blit-core/src/remote/transfer/pipeline.rs:1401:        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
crates/blit-core/src/remote/transfer/pipeline.rs:1436:    use crate::remote::transfer::sink::{SinkOutcome, TransferSink};
crates/blit-core/src/remote/transfer/pipeline.rs:1454:    impl TransferSink for CountingSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1461:                files_written: 1,
crates/blit-core/src/remote/transfer/pipeline.rs:1465:        fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/pipeline.rs:1495:        let fast: Arc<dyn TransferSink> = Arc::new(CountingSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1500:        let slow: Arc<dyn TransferSink> = Arc::new(CountingSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1521:        assert_eq!(outcome.files_written, n, "every payload written once");
crates/blit-core/src/remote/transfer/pipeline.rs:1548:    impl TransferSink for ErrSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1552:        fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/pipeline.rs:1568:    impl TransferSink for GatedSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1576:                files_written: 1,
crates/blit-core/src/remote/transfer/pipeline.rs:1584:        fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/pipeline.rs:1621:        let first: Arc<dyn TransferSink> = Arc::new(GatedSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1629:        let second: Arc<dyn TransferSink> = Arc::new(GatedSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1674:        assert_eq!(outcome.files_written, 2, "exactly-once across both workers");
crates/blit-core/src/remote/transfer/pipeline.rs:1688:        let keep: Arc<dyn TransferSink> = Arc::new(GatedSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1696:        let victim: Arc<dyn TransferSink> = Arc::new(GatedSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1729:        assert_eq!(outcome.files_written, n, "no payload lost on retire");
crates/blit-core/src/remote/transfer/pipeline.rs:1756:        let only: Arc<dyn TransferSink> = Arc::new(GatedSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1779:        assert_eq!(outcome.files_written, n, "retire floor held at one worker");
crates/blit-core/src/remote/transfer/pipeline.rs:1796:        let first: Arc<dyn TransferSink> = Arc::new(GatedSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1804:        let late: Arc<dyn TransferSink> = Arc::new(GatedSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1846:        assert_eq!(outcome.files_written, 1);
crates/blit-core/src/remote/transfer/pipeline.rs:1869:        let sink: Arc<dyn TransferSink> = Arc::new(ErrSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1911:    impl TransferSink for ByteSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1928:                files_written: files,
crates/blit-core/src/remote/transfer/pipeline.rs:1932:        fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/pipeline.rs:1968:        let a: Arc<dyn TransferSink> = Arc::new(ByteSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1973:        let b: Arc<dyn TransferSink> = Arc::new(ByteSink {
crates/blit-core/src/remote/transfer/pipeline.rs:1992:        assert_eq!(outcome.files_written, n, "file total");
crates/blit-core/src/remote/transfer/pipeline.rs:2040:        let sink: Arc<dyn TransferSink> = Arc::new(CountingSink {
crates/blit-core/src/remote/transfer/pipeline.rs:2068:            outcome.files_written, 5,
crates/blit-core/src/remote/transfer/pipeline.rs:2100:        let err: Arc<dyn TransferSink> = Arc::new(ErrSink {
crates/blit-core/src/remote/transfer/pipeline.rs:2103:        let slow: Arc<dyn TransferSink> = Arc::new(CountingSink {
crates/blit-core/src/remote/transfer/sink.rs:3://! Every src→dst combination flows through `TransferSource → plan → prepare → TransferSink`.
crates/blit-core/src/remote/transfer/sink.rs:27:pub struct SinkOutcome {
crates/blit-core/src/remote/transfer/sink.rs:28:    pub files_written: usize,
crates/blit-core/src/remote/transfer/sink.rs:34:        self.files_written += other.files_written;
crates/blit-core/src/remote/transfer/sink.rs:44:pub trait TransferSink: Send + Sync {
crates/blit-core/src/remote/transfer/sink.rs:55:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:61:            "{} does not support write_file_stream (called for {})",
crates/blit-core/src/remote/transfer/sink.rs:74:    fn root(&self) -> &Path;
crates/blit-core/src/remote/transfer/sink.rs:78:// FsTransferSink — local filesystem writer
crates/blit-core/src/remote/transfer/sink.rs:114:pub struct FsTransferSink {
crates/blit-core/src/remote/transfer/sink.rs:130:    /// `write_payload`/`write_file_stream` pushes its `relative_path`.
crates/blit-core/src/remote/transfer/sink.rs:133:    /// `write_file_stream` passes it into
crates/blit-core/src/remote/transfer/sink.rs:138:    /// [`FsTransferSink::with_byte_progress`] from
crates/blit-core/src/remote/transfer/sink.rs:143:impl FsTransferSink {
crates/blit-core/src/remote/transfer/sink.rs:153:        let canonical_dst_root = crate::path_safety::canonical_dest_root(&dst_root).ok();
crates/blit-core/src/remote/transfer/sink.rs:174:    /// `write_file_stream` reports every chunk the data plane
crates/blit-core/src/remote/transfer/sink.rs:197:                    "FsTransferSink at '{}' has no canonical root; \
crates/blit-core/src/remote/transfer/sink.rs:217:impl TransferSink for FsTransferSink {
crates/blit-core/src/remote/transfer/sink.rs:253:                if outcome.files_written > 0 {
crates/blit-core/src/remote/transfer/sink.rs:280:                    PreparedPayload::TarShard { headers, data } => write_tar_shard_payload(
crates/blit-core/src/remote/transfer/sink.rs:291:                if outcome.files_written > 0 {
crates/blit-core/src/remote/transfer/sink.rs:300:        // write_payload, not write_file_stream, so the chunk-
crates/blit-core/src/remote/transfer/sink.rs:307:        // `write_tar_shard_payload`'s dry-run early returns), so
crates/blit-core/src/remote/transfer/sink.rs:309:        // `write_file_stream`'s dry-run branch.
crates/blit-core/src/remote/transfer/sink.rs:318:    /// is what makes push and pull receive symmetric on the FsTransferSink.
crates/blit-core/src/remote/transfer/sink.rs:319:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:361:                files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:437:            files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:442:    fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/sink.rs:456:    // R47-F1: the FsTransferSink::write_payload arm for
crates/blit-core/src/remote/transfer/sink.rs:462:    // write_file_stream uses.
crates/blit-core/src/remote/transfer/sink.rs:488:            files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:526:        files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:532:fn write_tar_shard_payload(
crates/blit-core/src/remote/transfer/sink.rs:541:            files_written: headers.len(),
crates/blit-core/src/remote/transfer/sink.rs:565:    // R47-F1: tar shards arriving on FsTransferSink::write_payload
crates/blit-core/src/remote/transfer/sink.rs:580:            "write_tar_shard_payload at '{}' has no canonical root; \
crates/blit-core/src/remote/transfer/sink.rs:589:    // either way (matches the historical FsTransferSink policy).
crates/blit-core/src/remote/transfer/sink.rs:627:    let mut files_written = 0usize;
crates/blit-core/src/remote/transfer/sink.rs:631:        files_written += 1;
crates/blit-core/src/remote/transfer/sink.rs:635:        files_written,
crates/blit-core/src/remote/transfer/sink.rs:677:        files_written: 0, // Resume blocks patch in-place; finalization counts the file.
crates/blit-core/src/remote/transfer/sink.rs:718:    // dance as write_file_stream — see commit 946bd77).
crates/blit-core/src/remote/transfer/sink.rs:736:        files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:742:// DataPlaneSink — TCP data plane writer
crates/blit-core/src/remote/transfer/sink.rs:748:/// transfers, the pipeline executor creates multiple DataPlaneSink instances.
crates/blit-core/src/remote/transfer/sink.rs:749:pub struct DataPlaneSink<P: Probe = NoProbe> {
crates/blit-core/src/remote/transfer/sink.rs:755:impl<P: Probe> DataPlaneSink<P> {
crates/blit-core/src/remote/transfer/sink.rs:770:impl<P: Probe> TransferSink for DataPlaneSink<P> {
crates/blit-core/src/remote/transfer/sink.rs:781:                    files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:793:                    files_written: count,
crates/blit-core/src/remote/transfer/sink.rs:800:                eyre::bail!("DataPlaneSink does not relay resume-block payloads")
crates/blit-core/src/remote/transfer/sink.rs:807:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:819:            files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:829:    fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/sink.rs:862:impl TransferSink for NullSink {
crates/blit-core/src/remote/transfer/sink.rs:866:                files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:870:                files_written: headers.len(),
crates/blit-core/src/remote/transfer/sink.rs:874:                files_written: 0,
crates/blit-core/src/remote/transfer/sink.rs:884:    async fn write_file_stream(
crates/blit-core/src/remote/transfer/sink.rs:896:        // FsTransferSink.
crates/blit-core/src/remote/transfer/sink.rs:907:            files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:912:    fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/sink.rs:924:/// negotiation fails. Slower than `DataPlaneSink` but works in restrictive
crates/blit-core/src/remote/transfer/sink.rs:950:impl TransferSink for GrpcFallbackSink {
crates/blit-core/src/remote/transfer/sink.rs:1022:                    files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:1059:                    files_written: count,
crates/blit-core/src/remote/transfer/sink.rs:1084:    fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/sink.rs:1128:impl TransferSink for GrpcServerStreamingSink {
crates/blit-core/src/remote/transfer/sink.rs:1193:                    files_written: 1,
crates/blit-core/src/remote/transfer/sink.rs:1230:                    files_written: count,
crates/blit-core/src/remote/transfer/sink.rs:1249:    fn root(&self) -> &Path {
crates/blit-core/src/remote/transfer/sink.rs:1280:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:1298:        assert_eq!(outcome.files_written, 1);
crates/blit-core/src/remote/transfer/sink.rs:1313:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:1331:        assert_eq!(outcome.files_written, 1);
crates/blit-core/src/remote/transfer/sink.rs:1350:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:1375:    /// R58-F4 regression for the streaming receive path. `write_file_stream`
crates/blit-core/src/remote/transfer/sink.rs:1380:    async fn fs_sink_dry_run_write_file_stream_does_not_create_dirs() {
crates/blit-core/src/remote/transfer/sink.rs:1387:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:1401:        let outcome = sink.write_file_stream(&header, &mut reader).await.unwrap();
crates/blit-core/src/remote/transfer/sink.rs:1403:        assert_eq!(outcome.files_written, 1);
crates/blit-core/src/remote/transfer/sink.rs:1423:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:1441:        assert_eq!(outcome.files_written, 1);
crates/blit-core/src/remote/transfer/sink.rs:1480:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:1500:        assert_eq!(outcome.files_written, 2);
crates/blit-core/src/remote/transfer/sink.rs:1515:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:1544:        assert_eq!(outcome.files_written, 1);
crates/blit-core/src/remote/transfer/sink.rs:1563:        assert_eq!(outcome.files_written, 3);
crates/blit-core/src/remote/transfer/sink.rs:1570:        assert_eq!(sink.root(), Path::new("/dev/null"));
crates/blit-core/src/remote/transfer/sink.rs:1592:        assert_eq!(outcome.files_written, 1);
crates/blit-core/src/remote/transfer/sink.rs:1875:    // validator's surface. These tests exercise the FsTransferSink end of
crates/blit-core/src/remote/transfer/sink.rs:1887:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:1899:        // Use write_file_stream so we exercise the sink.rs:218 site that
crates/blit-core/src/remote/transfer/sink.rs:1903:        let result = sink.write_file_stream(&header, &mut empty).await;
crates/blit-core/src/remote/transfer/sink.rs:1961:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:1977:            .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/sink.rs:1981:        assert_eq!(outcome.files_written, 1);
crates/blit-core/src/remote/transfer/sink.rs:1997:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:2013:            .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/sink.rs:2045:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:2062:            .write_file_stream(&header, &mut reader)
crates/blit-core/src/remote/transfer/sink.rs:2105:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:2137:    /// existing dst escape symlink. Pre-fix `write_tar_shard_payload`
crates/blit-core/src/remote/transfer/sink.rs:2176:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:2208:    /// not `write_file_stream`, so the chunk-granular byte hook
crates/blit-core/src/remote/transfer/sink.rs:2250:        let sink = FsTransferSink::new(
crates/blit-core/src/remote/transfer/sink.rs:2271:        assert_eq!(outcome.files_written, 2);
crates/blit-core/src/remote/transfer/sink.rs:2297:        let sink = FsTransferSink::new(

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/sink.rs | sed -n '1,360p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '1,380p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Pluggable write backends for the transfer pipeline.
     2	//!
     3	//! Every src→dst combination flows through `TransferSource → plan → prepare → TransferSink`.
     4	//! Implementations handle the actual write: local filesystem, TCP data plane, etc.
     5	
     6	use std::path::{Path, PathBuf};
     7	use std::sync::Arc;
     8	
     9	use async_trait::async_trait;
    10	use eyre::{Context, Result};
    11	use filetime::FileTime;
    12	
    13	use crate::buffer::BufferSizer;
    14	use crate::checksum::ChecksumType;
    15	use crate::copy::{copy_file, resume_copy_file};
    16	use crate::generated::{ComparisonMode, FileHeader};
    17	use crate::logger::NoopLogger;
    18	use crate::remote::transfer::payload::PreparedPayload;
    19	use crate::remote::transfer::progress::{ByteProgressSink, NoProbe, Probe};
    20	use crate::remote::transfer::source::TransferSource;
    21	
    22	// Re-export for consumers.
    23	pub use super::data_plane::DataPlaneSession;
    24	
    25	/// Outcome of writing payload(s) to a sink.
    26	#[derive(Debug, Default, Clone)]
    27	pub struct SinkOutcome {
    28	    pub files_written: usize,
    29	    pub bytes_written: u64,
    30	}
    31	
    32	impl SinkOutcome {
    33	    pub fn merge(&mut self, other: &SinkOutcome) {
    34	        self.files_written += other.files_written;
    35	        self.bytes_written += other.bytes_written;
    36	    }
    37	}
    38	
    39	/// A pluggable write backend for the transfer pipeline.
    40	///
    41	/// Implementations receive [`PreparedPayload`] items produced by a [`TransferSource`]
    42	/// and write them to a destination (local filesystem, TCP stream, etc.).
    43	#[async_trait]
    44	pub trait TransferSink: Send + Sync {
    45	    /// Write a single prepared payload to the destination.
    46	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome>;
    47	
    48	    /// Stream a file payload from a borrowed async reader.
    49	    ///
    50	    /// Used by the receive pipeline so file bytes that arrive on a TCP
    51	    /// wire can be written through the same sink as local copies — no
    52	    /// double-buffering into a `'static` reader. Sinks that don't
    53	    /// support inbound streaming (e.g. `GrpcFallbackSink`) inherit the
    54	    /// default error implementation.
    55	    async fn write_file_stream(
    56	        &self,
    57	        header: &FileHeader,
    58	        _reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
    59	    ) -> Result<SinkOutcome> {
    60	        eyre::bail!(
    61	            "{} does not support write_file_stream (called for {})",
    62	            std::any::type_name::<Self>(),
    63	            header.relative_path
    64	        )
    65	    }
    66	
    67	    /// Signal that all payloads have been sent. Flushes buffers, sends terminators, etc.
    68	    /// Default implementation is a no-op.
    69	    async fn finish(&self) -> Result<()> {
    70	        Ok(())
    71	    }
    72	
    73	    /// Destination root path (if applicable).
    74	    fn root(&self) -> &Path;
    75	}
    76	
    77	// ---------------------------------------------------------------------------
    78	// FsTransferSink — local filesystem writer
    79	// ---------------------------------------------------------------------------
    80	
    81	/// Configuration for filesystem sink writes.
    82	#[derive(Debug, Clone)]
    83	pub struct FsSinkConfig {
    84	    pub preserve_times: bool,
    85	    pub dry_run: bool,
    86	    pub checksum: Option<ChecksumType>,
    87	    pub resume: bool,
    88	    /// R58-followup: comparison policy the sink uses when deciding
    89	    /// whether to copy a `PreparedPayload::File`. The diff_planner
    90	    /// upstream already filters by `compare_mode`, but
    91	    /// `write_file_payload` re-checks before copying as a defense
    92	    /// layer; pre-fix it called `file_needs_copy_with_checksum_type`
    93	    /// which only knows SizeMtime + Checksum, so `Force` and
    94	    /// `IgnoreTimes` were silently downgraded to SizeMtime and
    95	    /// dropped at the sink layer. The default `SizeMtime` keeps
    96	    /// pre-fix behavior for callers that haven't migrated.
    97	    pub compare_mode: ComparisonMode,
    98	}
    99	
   100	impl Default for FsSinkConfig {
   101	    fn default() -> Self {
   102	        Self {
   103	            preserve_times: true,
   104	            dry_run: false,
   105	            checksum: None,
   106	            resume: false,
   107	            compare_mode: ComparisonMode::SizeMtime,
   108	        }
   109	    }
   110	}
   111	
   112	/// Writes files directly to a local filesystem using zero-copy primitives
   113	/// (copy_file_range, sendfile, clonefile, block clone) where available.
   114	pub struct FsTransferSink {
   115	    src_root: PathBuf,
   116	    dst_root: PathBuf,
   117	    /// Canonical form of `dst_root` (or its deepest existing
   118	    /// ancestor) captured once at sink construction time. Every
   119	    /// per-entry write resolves the lexical path under `dst_root`
   120	    /// and then verifies it stays inside `canonical_dst_root`
   121	    /// post-symlink. R46-F3: pre-fix the sink only ran lexical
   122	    /// `safe_join`, so a peer-controlled relative path joined under
   123	    /// a `dst_root/link → /outside` symlink would write outside
   124	    /// the destination root.
   125	    canonical_dst_root: Option<PathBuf>,
   126	    config: FsSinkConfig,
   127	    /// Optional collector for relative paths of successfully-written
   128	    /// files. Used by remote pull's mirror flow to know which files to
   129	    /// keep when purging extraneous local entries. Each successful
   130	    /// `write_payload`/`write_file_stream` pushes its `relative_path`.
   131	    path_tracker: Option<Arc<std::sync::Mutex<Vec<PathBuf>>>>,
   132	    /// Optional byte-level progress sink. When set,
   133	    /// `write_file_stream` passes it into
   134	    /// `receive_stream_double_buffered` so chunk-granularity
   135	    /// writes report cumulative byte progress against the
   136	    /// daemon's per-transfer counter (c-1a). Unset on the CLI
   137	    /// side; the daemon side sets it via
   138	    /// [`FsTransferSink::with_byte_progress`] from
   139	    /// `ActiveJobGuard::bytes_counter()`.
   140	    byte_progress: Option<ByteProgressSink>,
   141	}
   142	
   143	impl FsTransferSink {
   144	    pub fn new(src_root: PathBuf, dst_root: PathBuf, config: FsSinkConfig) -> Self {
   145	        // Best-effort canonical root capture. We don't fail
   146	        // construction if canonicalize fails (e.g. dst_root is a
   147	        // not-yet-created path under a deeply unusual filesystem) —
   148	        // instead we leave canonical_dst_root as None and the
   149	        // per-write check degrades to lexical-only with a warn.
   150	        // R46-F3: in the common case (dst_root or its ancestor
   151	        // exists) this captures the canonical form needed for
   152	        // symlink-escape rejection.
   153	        let canonical_dst_root = crate::path_safety::canonical_dest_root(&dst_root).ok();
   154	        Self {
   155	            src_root,
   156	            dst_root,
   157	            canonical_dst_root,
   158	            config,
   159	            path_tracker: None,
   160	            byte_progress: None,
   161	        }
   162	    }
   163	
   164	    /// Enable path tracking. After each successful write, the relative
   165	    /// path of the written file is pushed onto the supplied collector.
   166	    /// Lets receive callers (e.g. mirror) discover which files survived
   167	    /// without re-implementing the record dispatch loop.
   168	    pub fn with_path_tracker(mut self, tracker: Arc<std::sync::Mutex<Vec<PathBuf>>>) -> Self {
   169	        self.path_tracker = Some(tracker);
   170	        self
   171	    }
   172	
   173	    /// Attach a byte-level progress sink. When set,
   174	    /// `write_file_stream` reports every chunk the data plane
   175	    /// writes against this sink. Used by the daemon side of
   176	    /// remote→remote transfers so `GetState.active[].bytes_completed`
   177	    /// tracks live progress; CLI-side callers omit it.
   178	    pub fn with_byte_progress(mut self, sink: ByteProgressSink) -> Self {
   179	        self.byte_progress = Some(sink);
   180	        self
   181	    }
   182	
   183	    /// R46-F3: lexical resolve + canonical containment check in one
   184	    /// call. Used by every per-entry write site on this sink so a
   185	    /// peer-controlled relative path can't escape the destination
   186	    /// root via a pre-existing symlink. Falls back to lexical-only
   187	    /// (with a warn) if `canonical_dst_root` was None at
   188	    /// construction time — that path remains exposed but is
   189	    /// extremely unusual in practice.
   190	    fn resolve_destination(&self, wire_path: &str) -> Result<PathBuf> {
   191	        match self.canonical_dst_root.as_ref() {
   192	            Some(canonical) => {
   193	                crate::path_safety::safe_join_contained(canonical, &self.dst_root, wire_path)
   194	            }
   195	            None => {
   196	                log::warn!(
   197	                    "FsTransferSink at '{}' has no canonical root; \
   198	                     receive falls back to lexical-only path check \
   199	                     (R46-F3 escape protection unavailable)",
   200	                    self.dst_root.display()
   201	                );
   202	                crate::path_safety::safe_join(&self.dst_root, wire_path)
   203	            }
   204	        }
   205	    }
   206	
   207	    fn track(&self, rel: &str) {
   208	        if let Some(tracker) = &self.path_tracker {
   209	            if let Ok(mut guard) = tracker.lock() {
   210	                guard.push(PathBuf::from(rel));
   211	            }
   212	        }
   213	    }
   214	}
   215	
   216	#[async_trait]
   217	impl TransferSink for FsTransferSink {
   218	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   219	        // Resume payloads need async I/O (file open + seek + write
   220	        // through tokio). Local-source payloads (File / TarShard) stay
   221	        // on a blocking thread so the zero-copy cascade and tar
   222	        // extraction can use std::fs.
   223	        let outcome = match payload {
   224	            PreparedPayload::FileBlock {
   225	                relative_path,
   226	                offset,
   227	                bytes,
   228	            } => {
   229	                write_file_block_payload(
   230	                    &self.dst_root,
   231	                    self.canonical_dst_root.as_deref(),
   232	                    &relative_path,
   233	                    offset,
   234	                    bytes,
   235	                )
   236	                .await?
   237	            }
   238	            PreparedPayload::FileBlockComplete {
   239	                relative_path,
   240	                total_size,
   241	                mtime_seconds,
   242	                permissions,
   243	            } => {
   244	                let outcome = write_file_block_complete(
   245	                    &self.dst_root,
   246	                    self.canonical_dst_root.as_deref(),
   247	                    &relative_path,
   248	                    total_size,
   249	                    mtime_seconds,
   250	                    permissions,
   251	                )
   252	                .await?;
   253	                if outcome.files_written > 0 {
   254	                    self.track(&relative_path);
   255	                }
   256	                outcome
   257	            }
   258	            PreparedPayload::File(_) | PreparedPayload::TarShard { .. } => {
   259	                // Capture paths for tracking before payload moves into
   260	                // the spawn_blocking closure.
   261	                let tracked_paths: Vec<String> = match &payload {
   262	                    PreparedPayload::File(h) => vec![h.relative_path.clone()],
   263	                    PreparedPayload::TarShard { headers, .. } => {
   264	                        headers.iter().map(|h| h.relative_path.clone()).collect()
   265	                    }
   266	                    _ => Vec::new(),
   267	                };
   268	                let src_root = self.src_root.clone();
   269	                let dst_root = self.dst_root.clone();
   270	                let canonical_dst_root = self.canonical_dst_root.clone();
   271	                let config = self.config.clone();
   272	                let outcome = tokio::task::spawn_blocking(move || match payload {
   273	                    PreparedPayload::File(header) => write_file_payload(
   274	                        &src_root,
   275	                        &dst_root,
   276	                        canonical_dst_root.as_deref(),
   277	                        &header,
   278	                        &config,
   279	                    ),
   280	                    PreparedPayload::TarShard { headers, data } => write_tar_shard_payload(
   281	                        &dst_root,
   282	                        canonical_dst_root.as_deref(),
   283	                        &headers,
   284	                        &data,
   285	                        &config,
   286	                    ),
   287	                    _ => unreachable!("outer match guarantees File or TarShard"),
   288	                })
   289	                .await
   290	                .context("sink worker panicked")??;
   291	                if outcome.files_written > 0 {
   292	                    for path in tracked_paths {
   293	                        self.track(&path);
   294	                    }
   295	                }
   296	                outcome
   297	            }
   298	        };
   299	        // c-1b round 2: tar shards and resume blocks land via
   300	        // write_payload, not write_file_stream, so the chunk-
   301	        // granular `receive_stream_double_buffered` hook never
   302	        // fires for them. Report `outcome.bytes_written` here so
   303	        // `GetState.active[].bytes_completed` reflects bytes
   304	        // landed on disk for ALL payload shapes, not just
   305	        // streamed files. Dry-run write paths return
   306	        // `bytes_written: 0` (see `write_file_payload` and
   307	        // `write_tar_shard_payload`'s dry-run early returns), so
   308	        // adding 0 is a no-op for previews — same semantics as
   309	        // `write_file_stream`'s dry-run branch.
   310	        if let Some(bp) = &self.byte_progress {
   311	            bp.report(outcome.bytes_written);
   312	        }
   313	        Ok(outcome)
   314	    }
   315	
   316	    /// Stream file bytes from the wire to the destination filesystem
   317	    /// using the same double-buffered helper the send side uses. This
   318	    /// is what makes push and pull receive symmetric on the FsTransferSink.
   319	    async fn write_file_stream(
   320	        &self,
   321	        header: &FileHeader,
   322	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   323	    ) -> Result<SinkOutcome> {
   324	        use crate::remote::transfer::data_plane::{
   325	            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
   326	        };
   327	
   328	        // R46-F3: lexical resolve + canonical containment check via
   329	        // resolve_destination. Pre-fix this was a bare safe_join,
   330	        // which rejected lexical traversal (`../`) but didn't catch
   331	        // the case where dst_root contained a pre-existing symlink
   332	        // pointing outside (`dst_root/link → /outside`); a peer-
   333	        // controlled relative path `link/file` would then write to
   334	        // `/outside/file`.
   335	        let dst = self
   336	            .resolve_destination(&header.relative_path)
   337	            .with_context(|| format!("validating receive path {:?}", header.relative_path))?;
   338	
   339	        // R58-F4: dry-run must be side-effect-free. Drain the wire
   340	        // for protocol-stream alignment, but skip the parent-mkdir
   341	        // and the file write. Pre-fix the parent-mkdir ran before
   342	        // the dry-run check below, so `--dry-run` over a remote
   343	        // transfer would create destination directories.
   344	        if self.config.dry_run {
   345	            let mut sink = tokio::io::sink();
   346	            // Dry-run: drain wire bytes for protocol alignment.
   347	            // Do NOT report against `byte_progress` — by contract
   348	            // dry-run is side-effect-free and these bytes never
   349	            // hit user disk; we don't want a daemon-side bytes_completed
   350	            // counter to advance for an aborted preview.
   351	            receive_stream_double_buffered(
   352	                reader,
   353	                &mut sink,
   354	                header.size,
   355	                RECEIVE_CHUNK_SIZE,
   356	                None,
   357	            )
   358	            .await
   359	            .with_context(|| format!("draining {} (dry-run)", header.relative_path))?;
   360	            return Ok(SinkOutcome {

 succeeded in 0ms:
     1	//! Unified transfer pipeline: source → prepare → sink(s).
     2	//!
     3	//! All transfer paths (local→local, local→remote push, remote→local pull,
     4	//! remote→remote) route through the same executor. Payloads can be supplied
     5	//! either upfront ([`execute_sink_pipeline`]) or incrementally as they are
     6	//! produced ([`execute_sink_pipeline_streaming`]). The one-shot form is a
     7	//! thin wrapper that sends every payload on a channel and delegates.
     8	
     9	use std::sync::Arc;
    10	
    11	use eyre::{Context, Result};
    12	use tokio::sync::mpsc;
    13	
    14	use super::payload::{PreparedPayload, TransferPayload};
    15	use super::progress::RemoteTransferProgress;
    16	use super::sink::{SinkOutcome, TransferSink};
    17	use super::source::TransferSource;
    18	
    19	/// Execute a transfer pipeline with all payloads known upfront.
    20	///
    21	/// This is a convenience wrapper around [`execute_sink_pipeline_streaming`]
    22	/// that spawns a task to send every payload into the channel and then drops
    23	/// the sender, signalling end-of-stream.
    24	pub async fn execute_sink_pipeline(
    25	    source: Arc<dyn TransferSource>,
    26	    sinks: Vec<Arc<dyn TransferSink>>,
    27	    payloads: Vec<TransferPayload>,
    28	    prefetch: usize,
    29	    progress: Option<&RemoteTransferProgress>,
    30	) -> Result<SinkOutcome> {
    31	    if sinks.is_empty() {
    32	        return Ok(SinkOutcome::default());
    33	    }
    34	    if payloads.is_empty() {
    35	        for sink in &sinks {
    36	            sink.finish().await?;
    37	        }
    38	        return Ok(SinkOutcome::default());
    39	    }
    40	
    41	    let capacity = prefetch.max(1);
    42	    let (tx, rx) = mpsc::channel::<TransferPayload>(capacity);
    43	
    44	    // Feed payloads in a background task so the pipeline can start writing
    45	    // before the whole vec is queued (the channel provides back-pressure).
    46	    let feeder = tokio::spawn(async move {
    47	        for payload in payloads {
    48	            if tx.send(payload).await.is_err() {
    49	                break;
    50	            }
    51	        }
    52	        // Dropping tx closes the channel and signals end-of-stream.
    53	    });
    54	
    55	    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
    56	    let _ = feeder.await;
    57	    result
    58	}
    59	
    60	/// Execute a transfer pipeline with payloads arriving on a channel.
    61	///
    62	/// Payloads are distributed across `sinks` through a single shared
    63	/// **work-stealing** queue (a bounded `flume` MPMC channel): each sink
    64	/// runs as a tokio task that pulls the next available payload via
    65	/// `recv_async().await`, so a slow sink can never head-of-line-block the
    66	/// others (the failure mode of the previous round-robin per-sink
    67	/// channels). A forwarder task moves payloads from the incoming
    68	/// `payload_rx` onto the shared queue; dropping its sender on
    69	/// end-of-stream lets every worker observe `Disconnected` once the queue
    70	/// drains, at which point it calls `sink.finish()`. Errors from any
    71	/// worker propagate up (first error wins).
    72	///
    73	/// `prefetch` controls the per-sink preparation-in-flight limit; the
    74	/// shared queue is bounded at `prefetch * sinks.len()` so total
    75	/// in-flight capacity matches the previous per-sink-channel design
    76	/// (back-pressure preserved).
    77	pub async fn execute_sink_pipeline_streaming(
    78	    source: Arc<dyn TransferSource>,
    79	    sinks: Vec<Arc<dyn TransferSink>>,
    80	    payload_rx: mpsc::Receiver<TransferPayload>,
    81	    prefetch: usize,
    82	    progress: Option<&RemoteTransferProgress>,
    83	) -> Result<SinkOutcome> {
    84	    execute_sink_pipeline_elastic(source, sinks, payload_rx, prefetch, progress, None).await
    85	}
    86	
    87	/// Control commands for a RUNNING pipeline (`ue-r2-2` stream resize).
    88	pub enum SinkControl {
    89	    /// Spawn a worker for this sink, pulling from the shared work
    90	    /// queue like every other worker. Safe at any time: a worker added
    91	    /// after end-of-stream sees the closed queue immediately and just
    92	    /// runs `finish()`.
    93	    Add(Arc<dyn TransferSink>),
    94	    /// Retire one worker: it stops pulling new payloads at the next
    95	    /// payload boundary, emits its sink's per-stream END record via
    96	    /// `finish()`, and exits — the receiving end's worker terminates
    97	    /// normally on that END, so a REMOVE needs no receiver-side
    98	    /// coordination. Refused (no-op) when only one live worker
    99	    /// remains: with zero workers the forwarder's queue send fails and
   100	    /// it treats that as shutdown, silently dropping the rest of the
   101	    /// payload stream.
   102	    RetireOne,
   103	}
   104	
   105	/// `ue-r2-2`: [`execute_sink_pipeline_streaming`] plus a control
   106	/// channel that can grow or shrink the live worker set mid-run. The
   107	/// shared queue's capacity stays `prefetch * initial sink count`
   108	/// (added workers raise parallelism, not in-flight buffering — the
   109	/// bound is a back-pressure property, not a correctness one).
   110	pub async fn execute_sink_pipeline_elastic(
   111	    source: Arc<dyn TransferSource>,
   112	    sinks: Vec<Arc<dyn TransferSink>>,
   113	    mut payload_rx: mpsc::Receiver<TransferPayload>,
   114	    prefetch: usize,
   115	    progress: Option<&RemoteTransferProgress>,
   116	    control_rx: Option<mpsc::UnboundedReceiver<SinkControl>>,
   117	) -> Result<SinkOutcome> {
   118	    use std::sync::atomic::{AtomicBool, Ordering};
   119	
   120	    if sinks.is_empty() {
   121	        // Drain incoming channel so the producer isn't left dangling.
   122	        while payload_rx.recv().await.is_some() {}
   123	        return Ok(SinkOutcome::default());
   124	    }
   125	
   126	    let sink_count = sinks.len();
   127	    let capacity = prefetch.max(1) * sink_count;
   128	    let total = Arc::new(std::sync::Mutex::new(SinkOutcome::default()));
   129	
   130	    // Single shared work queue. Each worker owns exactly one sink but
   131	    // pulls payloads from the common queue, so work is stolen by
   132	    // whichever sink is free rather than pre-assigned round-robin.
   133	    let (work_tx, work_rx) = flume::bounded::<TransferPayload>(capacity);
   134	
   135	    // Cancellation flag set by the first worker that errors. Without it,
   136	    // one sink failing only drops that worker's `work_rx` clone; as long
   137	    // as any other worker is alive `send_async` keeps succeeding, so the
   138	    // forwarder would keep draining `payload_rx` and queueing payloads
   139	    // that can never complete — delaying first-error-wins propagation
   140	    // (Codex review, PR2). With it, the forwarder stops at the next
   141	    // payload boundary and closes the queue so the survivors drain and
   142	    // finish promptly.
   143	    let cancelled = Arc::new(AtomicBool::new(false));
   144	
   145	    // Dynamic worker membership (`ue-r2-2`): a JoinSet instead of a
   146	    // fixed Vec of handles, plus a per-worker retire flag so a REMOVE
   147	    // can drain exactly one worker. `retire_flags` holds the workers
   148	    // that are live and not yet asked to retire — its length is the
   149	    // count the retire floor checks.
   150	    let mut join_set: tokio::task::JoinSet<(usize, Result<()>)> = tokio::task::JoinSet::new();
   151	    let mut retire_flags: Vec<(usize, tokio::sync::watch::Sender<bool>)> = Vec::new();
   152	    let mut next_slot = 0usize;
   153	
   154	    #[allow(clippy::too_many_arguments)]
   155	    fn spawn_sink_worker(
   156	        join_set: &mut tokio::task::JoinSet<(usize, Result<()>)>,
   157	        slot: usize,
   158	        sink: Arc<dyn TransferSink>,
   159	        work_rx: flume::Receiver<TransferPayload>,
   160	        source: Arc<dyn TransferSource>,
   161	        progress: Option<RemoteTransferProgress>,
   162	        total: Arc<std::sync::Mutex<SinkOutcome>>,
   163	        cancelled: Arc<std::sync::atomic::AtomicBool>,
   164	        mut retire: tokio::sync::watch::Receiver<bool>,
   165	    ) {
   166	        use std::sync::atomic::Ordering;
   167	        join_set.spawn(async move {
   168	            // Wrap the body so any early-return error trips the shared
   169	            // cancel flag before the `?` unwinds the task.
   170	            let run = async {
   171	                loop {
   172	                    // Stop pulling queued work once a sibling worker has
   173	                    // errored: first-error-wins should surface without the
   174	                    // survivors draining the rest of the bounded queue.
   175	                    // Interrupting an in-flight prepare/write (true prompt
   176	                    // cancellation) is the AbortOnDrop family, w4-1.
   177	                    if cancelled.load(Ordering::Relaxed) {
   178	                        break;
   179	                    }
   180	                    // ue-r2-2: a retired worker stops at the same payload
   181	                    // boundary; queued payloads stay in the shared queue
   182	                    // for the survivors (dequeue = ownership, so
   183	                    // exactly-once is preserved — flume's RecvFut only
   184	                    // takes an item when it resolves, so racing it is
   185	                    // safe). The watch (not a flag) also frees a worker
   186	                    // parked on an IDLE queue. Its `finish()` below emits
   187	                    // the per-stream END record — the receiver-side
   188	                    // teardown signal.
   189	                    let payload = tokio::select! {
   190	                        biased;
   191	                        _ = retire.changed() => break,
   192	                        recv = work_rx.recv_async() => match recv {
   193	                            Ok(p) => p,
   194	                            Err(_) => break, // queue closed and drained
   195	                        },
   196	                    };
   197	                    let prepared = source
   198	                        .prepare_payload(payload)
   199	                        .await
   200	                        .context("preparing payload")?;
   201	                    let files: Vec<(String, u64)> = match &prepared {
   202	                        PreparedPayload::File(h) => vec![(h.relative_path.clone(), h.size)],
   203	                        PreparedPayload::TarShard { headers, .. } => headers
   204	                            .iter()
   205	                            .map(|h| (h.relative_path.clone(), h.size))
   206	                            .collect(),
   207	                        // Resume-block payloads patch existing files; no
   208	                        // file-completion event from one-block-at-a-time.
   209	                        PreparedPayload::FileBlock { .. }
   210	                        | PreparedPayload::FileBlockComplete { .. } => Vec::new(),
   211	                    };
   212	                    let outcome = sink
   213	                        .write_payload(prepared)
   214	                        .await
   215	                        .context("writing payload")?;
   216	                    if let Some(p) = &progress {
   217	                        // Contract (progress.rs): bytes ride Payload, one
   218	                        // FileComplete per file. `size` is the planned
   219	                        // manifest size — the value this lane has always
   220	                        // reported, now on the right variant.
   221	                        for (name, size) in &files {
   222	                            p.report_payload(0, *size);
   223	                            p.report_file_complete(name.clone());
   224	                        }
   225	                    }
   226	                    let mut t = total.lock().unwrap();
   227	                    t.merge(&outcome);
   228	                }
   229	                sink.finish().await?;
   230	                Ok::<(), eyre::Report>(())
   231	            }
   232	            .await;
   233	            if run.is_err() {
   234	                // Signal the forwarder (and implicitly the other workers,
   235	                // once the queue closes) to stop feeding new work.
   236	                cancelled.store(true, Ordering::Relaxed);
   237	            }
   238	            (slot, run)
   239	        });
   240	    }
   241	
   242	    for sink in sinks {
   243	        let (retire_tx, retire_rx) = tokio::sync::watch::channel(false);
   244	        let slot = next_slot;
   245	        next_slot += 1;
   246	        retire_flags.push((slot, retire_tx));
   247	        spawn_sink_worker(
   248	            &mut join_set,
   249	            slot,
   250	            sink,
   251	            work_rx.clone(),
   252	            source.clone(),
   253	            progress.cloned(),
   254	            total.clone(),
   255	            cancelled.clone(),
   256	            retire_rx,
   257	        );
   258	    }
   259	
   260	    // Forwarder: move payloads from the incoming channel onto the shared
   261	    // work queue. `send_async` applies back-pressure (bounded queue); if
   262	    // every worker has gone away (e.g. all sinks errored) the send fails
   263	    // and we stop. It also bails as soon as a worker sets `cancelled`, so
   264	    // a single sink error halts intake promptly instead of waiting for
   265	    // every worker to drop. Dropping `work_tx` on end-of-stream (or on
   266	    // cancel) signals the workers. (The executor keeps a `work_rx` clone
   267	    // for late-added workers — flume disconnect is sender-driven, so the
   268	    // retained receiver does not keep the queue alive.)
   269	    let cancelled_fwd = cancelled.clone();
   270	    let forwarder = tokio::spawn(async move {
   271	        while let Some(payload) = payload_rx.recv().await {
   272	            if cancelled_fwd.load(std::sync::atomic::Ordering::Relaxed) {
   273	                // A worker errored — stop draining the producer and let
   274	                // the queue close so survivors finish and the error
   275	                // surfaces without delay.
   276	                return;
   277	            }
   278	            if work_tx.send_async(payload).await.is_err() {
   279	                // All workers dropped their receivers — nothing left to
   280	                // feed; treat as shutdown.
   281	                return;
   282	            }
   283	        }
   284	        // Dropping work_tx closes the queue → workers see Disconnected
   285	        // after draining and run finish().
   286	    });
   287	
   288	    // Supervise: join workers (first error wins) while servicing the
   289	    // resize control channel. `join_next() == None` means every worker
   290	    // — initial and added — has finished, which only happens once the
   291	    // queue closed and drained (or errored/retired), so control is
   292	    // moot beyond that point.
   293	    let mut control_rx = control_rx;
   294	    let mut first_err: Option<eyre::Report> = None;
   295	    loop {
   296	        let control_recv = async {
   297	            match control_rx.as_mut() {
   298	                Some(rx) => rx.recv().await,
   299	                None => std::future::pending().await,
   300	            }
   301	        };
   302	        tokio::select! {
   303	            // ue-r2-2 review (panel F2): biased, control FIRST — a
   304	            // ready Add must be processed before the join arm can
   305	            // observe an empty set and break, or an already-authorized
   306	            // socket would drop without its END record (fatal on the
   307	            // peer). Processing a control command is always cheap and
   308	            // never starves joins.
   309	            biased;
   310	
   311	            cmd = control_recv => {
   312	                match cmd {
   313	                    Some(SinkControl::Add(sink)) => {
   314	                        if !cancelled.load(Ordering::Relaxed) {
   315	                            let (retire_tx, retire_rx) = tokio::sync::watch::channel(false);
   316	                            let slot = next_slot;
   317	                            next_slot += 1;
   318	                            retire_flags.push((slot, retire_tx));
   319	                            spawn_sink_worker(
   320	                                &mut join_set,
   321	                                slot,
   322	                                sink,
   323	                                work_rx.clone(),
   324	                                source.clone(),
   325	                                progress.cloned(),
   326	                                total.clone(),
   327	                                cancelled.clone(),
   328	                                retire_rx,
   329	                            );
   330	                        }
   331	                        // On a failing transfer the added sink is dropped
   332	                        // unused; its socket closes and the peer's worker
   333	                        // errors into the already-failing teardown.
   334	                    }
   335	                    Some(SinkControl::RetireOne) => {
   336	                        // Floor at one live worker (see SinkControl docs).
   337	                        if retire_flags.len() > 1 {
   338	                            if let Some((_, retire_tx)) = retire_flags.pop() {
   339	                                let _ = retire_tx.send(true);
   340	                            }
   341	                        }
   342	                    }
   343	                    None => control_rx = None, // controller gone; keep draining
   344	                }
   345	            }
   346	            joined = join_set.join_next() => {
   347	                match joined {
   348	                    None => break,
   349	                    Some(Ok((slot, res))) => {
   350	                        retire_flags.retain(|(s, _)| *s != slot);
   351	                        if let Err(e) = res {
   352	                            if first_err.is_none() {
   353	                                first_err = Some(e);
   354	                            }
   355	                        }
   356	                    }
   357	                    Some(Err(join)) => {
   358	                        if first_err.is_none() {
   359	                            first_err = Some(eyre::eyre!("sink worker panicked: {}", join));
   360	                        }
   361	                    }
   362	                }
   363	            }
   364	        }
   365	    }
   366	    // ue-r2-2 review (panel F2, second half): an Add can still be
   367	    // queued in the instant between the last join and the break.
   368	    // Close its sink cleanly — the END record is what keeps the
   369	    // already-authorized peer worker from dying on a reset.
   370	    if let Some(rx) = control_rx.as_mut() {
   371	        while let Ok(cmd) = rx.try_recv() {
   372	            if let SinkControl::Add(sink) = cmd {
   373	                let _ = sink.finish().await;
   374	            }
   375	        }
   376	    }
   377	    drop(work_rx);
   378	    let _ = forwarder.await;
   379	
   380	    if let Some(err) = first_err {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '1,420p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use eyre::{bail, Context, Result};
     2	use futures::StreamExt;
     3	use tokio::io::{AsyncReadExt, AsyncWriteExt};
     4	use tokio::net::TcpStream;
     5	
     6	use crate::buffer::BufferPool;
     7	use crate::generated::FileHeader;
     8	
     9	use super::payload::{prepared_payload_stream, PreparedPayload, TransferPayload};
    10	use super::progress::{NoProbe, Probe};
    11	use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
    12	use crate::remote::transfer::source::TransferSource;
    13	use std::sync::Arc;
    14	
    15	pub const CONTROL_PLANE_CHUNK_SIZE: usize = 1024 * 1024;
    16	pub const DATA_PLANE_RECORD_FILE: u8 = 0;
    17	pub const DATA_PLANE_RECORD_TAR_SHARD: u8 = 1;
    18	pub const DATA_PLANE_RECORD_BLOCK: u8 = 2;
    19	pub const DATA_PLANE_RECORD_BLOCK_COMPLETE: u8 = 3;
    20	pub const DATA_PLANE_RECORD_END: u8 = 0xFF;
    21	
    22	/// ue-r2-2: length of the per-epoch resize credential a data socket
    23	/// echoes after the one-time token when resize was negotiated
    24	/// (`DataTransferNegotiation.epoch0_sub_token` for the initial
    25	/// sockets, `DataPlaneResize.sub_token` for an ADD epoch's socket).
    26	pub const SUB_TOKEN_LEN: usize = 16;
    27	
    28	/// Generate one 16-byte resize sub-token. Same fallible-RNG posture
    29	/// as the daemon's one-time token (audit-3b): a missing system RNG is
    30	/// an error, never a weaker credential.
    31	pub fn generate_sub_token() -> eyre::Result<Vec<u8>> {
    32	    use rand::{rngs::SysRng, TryRng};
    33	    let mut buf = vec![0u8; SUB_TOKEN_LEN];
    34	    SysRng
    35	        .try_fill_bytes(&mut buf)
    36	        .map_err(|err| eyre::eyre!("system RNG unavailable: {err}"))?;
    37	    Ok(buf)
    38	}
    39	
    40	/// A single data-plane TCP stream and its send loop.
    41	///
    42	/// Generic over a [`Probe`] so the byte-copy hot path can carry
    43	/// per-stream telemetry under adaptive mode at **zero cost** when the
    44	/// probe is [`NoProbe`] (the default): the instrumented branches are
    45	/// gated on `P::ACTIVE`, a compile-time constant, so they fold away
    46	/// entirely for `DataPlaneSession<NoProbe>`. Existing callers name the
    47	/// bare type and get the `NoProbe` default; the adaptive controller
    48	/// constructs `DataPlaneSession<LiveProbe>` via
    49	/// [`from_stream_with_probe`](DataPlaneSession::from_stream_with_probe).
    50	///
    51	/// audit-h3b: writes go through [`StallGuardWriter`] so a stalled
    52	/// reader (TCP backpressure from a slow / wedged peer) trips after
    53	/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
    54	/// of pinning the worker for OS-level TCP retransmit exhaustion
    55	/// (15+ minutes). All existing `self.stream.write_all/.flush` call
    56	/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
    57	/// so no per-site change was needed.
    58	pub struct DataPlaneSession<P: Probe = NoProbe> {
    59	    stream: StallGuardWriter<TcpStream>,
    60	    pool: Arc<BufferPool>,
    61	    trace: bool,
    62	    chunk_bytes: usize,
    63	    payload_prefetch: usize,
    64	    bytes_sent: u64,
    65	    probe: P,
    66	}
    67	
    68	macro_rules! trace_client {
    69	    ($session:expr, $($arg:tt)*) => {
    70	        if $session.trace {
    71	            eprintln!("[data-plane-client] {}", format_args!($($arg)*));
    72	        }
    73	    };
    74	}
    75	
    76	impl DataPlaneSession<NoProbe> {
    77	    /// Create a session from an existing stream with buffer pooling.
    78	    ///
    79	    /// Produces the un-instrumented `NoProbe` variant — the default for
    80	    /// every non-adaptive caller. audit-h3b: the stream is wrapped in
    81	    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
    82	    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
    83	    /// observable write progress instead of pinning the worker for
    84	    /// OS-level TCP retransmit exhaustion. The production call sites
    85	    /// (`daemon/service/pull.rs`, `daemon/service/pull_sync.rs`, and the
    86	    /// resume path) inherit the guard without code changes.
    87	    pub async fn from_stream(
    88	        stream: TcpStream,
    89	        trace: bool,
    90	        chunk_bytes: usize,
    91	        payload_prefetch: usize,
    92	        pool: Arc<BufferPool>,
    93	    ) -> Self {
    94	        Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, NoProbe)
    95	            .await
    96	    }
    97	
    98	    /// Connect to a data plane endpoint with buffer pooling.
    99	    #[allow(clippy::too_many_arguments)]
   100	    pub async fn connect(
   101	        host: &str,
   102	        port: u32,
   103	        token: &[u8],
   104	        chunk_bytes: usize,
   105	        payload_prefetch: usize,
   106	        trace: bool,
   107	        tcp_buffer_size: Option<usize>,
   108	        pool: Arc<BufferPool>,
   109	    ) -> Result<Self> {
   110	        Self::connect_with_probe(
   111	            host,
   112	            port,
   113	            token,
   114	            chunk_bytes,
   115	            payload_prefetch,
   116	            trace,
   117	            tcp_buffer_size,
   118	            pool,
   119	            NoProbe,
   120	        )
   121	        .await
   122	    }
   123	}
   124	
   125	impl<P: Probe> DataPlaneSession<P> {
   126	    /// `connect` with an explicit probe (ue-r2-1e: the dial tuner
   127	    /// attaches `LiveProbe` telemetry to the push data plane; the
   128	    /// probe-free path monomorphizes to `NoProbe` and reads no clock).
   129	    #[allow(clippy::too_many_arguments)]
   130	    pub async fn connect_with_probe(
   131	        host: &str,
   132	        port: u32,
   133	        token: &[u8],
   134	        chunk_bytes: usize,
   135	        payload_prefetch: usize,
   136	        trace: bool,
   137	        tcp_buffer_size: Option<usize>,
   138	        pool: Arc<BufferPool>,
   139	        probe: P,
   140	    ) -> Result<Self> {
   141	        let addr = format!("{}:{}", host, port);
   142	        if trace {
   143	            eprintln!("[data-plane-client] connecting to {}", addr);
   144	        }
   145	        // design-3: bounded dial (connect + w1-2 socket policy +
   146	        // negotiation-token write) via the shared data-plane helper —
   147	        // one owner for every client-side data-plane dial, both
   148	        // directions.
   149	        let stream = super::socket::dial_data_plane(&addr, token, tcp_buffer_size)
   150	            .await
   151	            .context("dialing push data plane")?;
   152	
   153	        Ok(
   154	            Self::from_stream_with_probe(stream, trace, chunk_bytes, payload_prefetch, pool, probe)
   155	                .await,
   156	        )
   157	    }
   158	}
   159	
   160	impl<P: Probe> DataPlaneSession<P> {
   161	    /// Create a session carrying an arbitrary [`Probe`]. The generic
   162	    /// primitive behind [`from_stream`](DataPlaneSession::from_stream);
   163	    /// the adaptive controller calls this with a `LiveProbe` to enable
   164	    /// per-stream telemetry.
   165	    pub async fn from_stream_with_probe(
   166	        stream: TcpStream,
   167	        trace: bool,
   168	        chunk_bytes: usize,
   169	        payload_prefetch: usize,
   170	        pool: Arc<BufferPool>,
   171	        probe: P,
   172	    ) -> Self {
   173	        let payload_prefetch = payload_prefetch.max(1);
   174	        let chunk_bytes = chunk_bytes.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);
   175	        Self {
   176	            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
   177	            pool,
   178	            trace,
   179	            chunk_bytes,
   180	            payload_prefetch,
   181	            bytes_sent: 0,
   182	            probe,
   183	        }
   184	    }
   185	
   186	    pub async fn send_payloads(
   187	        &mut self,
   188	        source: Arc<dyn TransferSource>,
   189	        payloads: Vec<TransferPayload>,
   190	    ) -> Result<()> {
   191	        self.send_payloads_with_progress(source, payloads, None)
   192	            .await
   193	    }
   194	
   195	    pub async fn send_payloads_with_progress(
   196	        &mut self,
   197	        source: Arc<dyn TransferSource>,
   198	        payloads: Vec<TransferPayload>,
   199	        progress: Option<&super::progress::RemoteTransferProgress>,
   200	    ) -> Result<()> {
   201	        let mut stream = prepared_payload_stream(payloads, source.clone(), self.payload_prefetch);
   202	        while let Some(prepared) = stream.next().await {
   203	            match prepared? {
   204	                PreparedPayload::File(header) => {
   205	                    if let Err(err) = self.send_file(source.clone(), &header).await {
   206	                        return Err(err.wrap_err(format!("sending {}", header.relative_path)));
   207	                    }
   208	                    self.bytes_sent = self.bytes_sent.saturating_add(header.size);
   209	                    if let Some(progress) = progress {
   210	                        progress.report_payload(0, header.size);
   211	                        progress.report_file_complete(header.relative_path.clone());
   212	                    }
   213	                }
   214	                PreparedPayload::TarShard { headers, data } => {
   215	                    let shard_bytes: u64 = headers.iter().map(|h| h.size).sum();
   216	                    if let Err(err) = self.send_prepared_tar_shard(headers.clone(), &data).await {
   217	                        return Err(err.wrap_err("sending tar shard"));
   218	                    }
   219	                    self.bytes_sent = self.bytes_sent.saturating_add(shard_bytes);
   220	                    if let Some(progress) = progress {
   221	                        for header in &headers {
   222	                            progress.report_payload(0, header.size);
   223	                            progress.report_file_complete(header.relative_path.clone());
   224	                        }
   225	                    }
   226	                }
   227	                PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   228	                    bail!("DataPlaneSession::send_payloads does not handle resume payloads");
   229	                }
   230	            }
   231	        }
   232	
   233	        Ok(())
   234	    }
   235	
   236	    pub async fn finish(&mut self) -> Result<()> {
   237	        self.stream
   238	            .write_all(&[DATA_PLANE_RECORD_END])
   239	            .await
   240	            .context("writing transfer terminator")?;
   241	        self.stream
   242	            .flush()
   243	            .await
   244	            .context("flushing data plane stream")
   245	    }
   246	
   247	    pub fn bytes_sent(&self) -> u64 {
   248	        self.bytes_sent
   249	    }
   250	
   251	    pub async fn send_file(
   252	        &mut self,
   253	        source: Arc<dyn TransferSource>,
   254	        header: &FileHeader,
   255	    ) -> Result<()> {
   256	        let rel = &header.relative_path;
   257	        let mut file = source
   258	            .open_file(header)
   259	            .await
   260	            .with_context(|| format!("opening {}", rel))?;
   261	        self.send_file_from_reader(header, &mut file).await
   262	    }
   263	
   264	    /// Send a file payload whose bytes come from an arbitrary async
   265	    /// reader (not a local file). Used by `DataPlaneSink` for the
   266	    /// remote→remote relay case, where bytes arrive from an inbound
   267	    /// `DataPlaneSource` and need to be forwarded to the next hop.
   268	    ///
   269	    /// Same wire format and double-buffered loop as `send_file`.
   270	    pub async fn send_file_from_reader(
   271	        &mut self,
   272	        header: &FileHeader,
   273	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   274	    ) -> Result<()> {
   275	        let rel = &header.relative_path;
   276	        trace_client!(self, "sending file '{}' ({} bytes)", rel, header.size);
   277	
   278	        let path_bytes = rel.as_bytes();
   279	        if path_bytes.len() > u32::MAX as usize {
   280	            bail!("relative path too long for transfer: {}", rel);
   281	        }
   282	
   283	        self.stream
   284	            .write_all(&[DATA_PLANE_RECORD_FILE])
   285	            .await
   286	            .context("writing data-plane record tag")?;
   287	        self.stream
   288	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   289	            .await
   290	            .context("writing path length")?;
   291	        self.stream
   292	            .write_all(path_bytes)
   293	            .await
   294	            .context("writing path bytes")?;
   295	
   296	        self.stream
   297	            .write_all(&header.size.to_be_bytes())
   298	            .await
   299	            .context("writing file size")?;
   300	        // Wire-format extension (2026-05-01): include mtime + permissions
   301	        // inline so push and pull data plane records carry the same
   302	        // information. Lets the receive pipeline apply metadata via
   303	        // FsTransferSink without consulting an out-of-band manifest cache.
   304	        self.stream
   305	            .write_all(&header.mtime_seconds.to_be_bytes())
   306	            .await
   307	            .context("writing mtime")?;
   308	        self.stream
   309	            .write_all(&header.permissions.to_be_bytes())
   310	            .await
   311	            .context("writing permissions")?;
   312	
   313	        // Double-buffered I/O: overlaps source reads with network writes
   314	        self.send_file_double_buffered(reader, header, rel).await?;
   315	
   316	        trace_client!(self, "file '{}' sent ({} bytes)", rel, header.size);
   317	
   318	        Ok(())
   319	    }
   320	
   321	    /// Double-buffered file sending: overlaps disk reads with network writes.
   322	    /// Uses two buffers from the pool to enable concurrent I/O operations.
   323	    ///
   324	    /// Pattern: While buffer A is being written to network, buffer B is filled from disk.
   325	    /// This hides disk latency behind network latency for improved throughput.
   326	    async fn send_file_double_buffered(
   327	        &mut self,
   328	        file: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   329	        header: &FileHeader,
   330	        rel: &str,
   331	    ) -> Result<()> {
   332	        let mut remaining = header.size;
   333	        if remaining == 0 {
   334	            return Ok(());
   335	        }
   336	
   337	        // Acquire two buffers for double-buffering
   338	        let mut buf_a = self.pool.acquire().await;
   339	        let mut buf_b = self.pool.acquire().await;
   340	
   341	        // Initial read into buf_a
   342	        let mut bytes_a = file
   343	            .read(buf_a.as_mut_slice())
   344	            .await
   345	            .with_context(|| format!("reading {}", rel))?;
   346	
   347	        if bytes_a == 0 {
   348	            bail!(
   349	                "unexpected EOF while reading {} ({} bytes remaining)",
   350	                rel,
   351	                remaining
   352	            );
   353	        }
   354	        // Clamp to the declared size before subtracting. A source that
   355	        // returns more bytes than `header.size` — a file that grew after
   356	        // the manifest was computed, or a lying `TransferSource` — would
   357	        // otherwise underflow `remaining` (debug: panic; release: wrap to
   358	        // u64::MAX → runaway loop) and push undeclared bytes onto the
   359	        // framed stream. We send exactly `header.size` and ignore excess.
   360	        bytes_a = (bytes_a as u64).min(remaining) as usize;
   361	        remaining -= bytes_a as u64;
   362	
   363	        // Main loop: write buf_a while reading into buf_b
   364	        while remaining > 0 {
   365	            // Per-stream telemetry: time ONLY the socket write as the
   366	            // backpressure signal. ue-r2-1e (carried ue-r2-1a review
   367	            // finding): the old code timed the whole overlapped
   368	            // write+read join, so a slow disk READ inflated
   369	            // "write blocked" and would bias the dial tuner
   370	            // conservative. The async block's clock starts when the
   371	            // join first polls it and stops when write_all completes —
   372	            // the concurrent read neither extends nor shortens it.
   373	            // Gated on the compile-time `P::ACTIVE` constant so
   374	            // `DataPlaneSession<NoProbe>` reads no clock.
   375	            let write_slice = &buf_a.as_slice()[..bytes_a];
   376	            let stream = &mut self.stream;
   377	            let (write_outcome, read_result) = tokio::join!(
   378	                async {
   379	                    let started = if P::ACTIVE {
   380	                        Some(std::time::Instant::now())
   381	                    } else {
   382	                        None
   383	                    };
   384	                    let result = stream.write_all(write_slice).await;
   385	                    (result, started.map(|t| t.elapsed()))
   386	                },
   387	                file.read(buf_b.as_mut_slice())
   388	            );
   389	
   390	            let (write_result, write_elapsed) = write_outcome;
   391	            write_result.with_context(|| format!("sending {}", rel))?;
   392	            if let Some(elapsed) = write_elapsed {
   393	                self.probe.note_write_blocked(elapsed.as_nanos() as u64);
   394	            }
   395	            self.probe.record_bytes(bytes_a as u64);
   396	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   397	
   398	            let bytes_b = read_result.with_context(|| format!("reading {}", rel))?;
   399	
   400	            if bytes_b == 0 && remaining > 0 {
   401	                bail!(
   402	                    "unexpected EOF while reading {} ({} bytes remaining)",
   403	                    rel,
   404	                    remaining
   405	                );
   406	            }
   407	            // Same clamp as the initial read: never subtract more than
   408	            // `remaining`, so an over-returning reader can neither
   409	            // underflow the counter nor send undeclared bytes.
   410	            let bytes_b = (bytes_b as u64).min(remaining) as usize;
   411	            remaining -= bytes_b as u64;
   412	
   413	            // Swap roles: buf_b becomes the write buffer, buf_a becomes read buffer
   414	            std::mem::swap(&mut buf_a, &mut buf_b);
   415	            bytes_a = bytes_b;
   416	        }
   417	
   418	        // Final write: send the last chunk in buf_a. This is a pure
   419	        // write (no overlapped read), so the timing is cleanly
   420	        // attributable to socket-write backpressure.

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/sink.rs | sed -n '360,860p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '380,540p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   360	            return Ok(SinkOutcome {
   361	                files_written: 1,
   362	                bytes_written: 0,
   363	            });
   364	        }
   365	
   366	        if let Some(parent) = dst.parent() {
   367	            tokio::fs::create_dir_all(parent)
   368	                .await
   369	                .with_context(|| format!("creating directory {}", parent.display()))?;
   370	        }
   371	
   372	        {
   373	            use tokio::io::AsyncWriteExt as _;
   374	            let mut file = tokio::fs::File::create(&dst)
   375	                .await
   376	                .with_context(|| format!("creating {}", dst.display()))?;
   377	            receive_stream_double_buffered(
   378	                reader,
   379	                &mut file,
   380	                header.size,
   381	                RECEIVE_CHUNK_SIZE,
   382	                self.byte_progress.as_ref(),
   383	            )
   384	            .await
   385	            .with_context(|| format!("writing {}", dst.display()))?;
   386	            // Flush the tokio File's internal buffer state (does NOT
   387	            // fsync — just ensures user-space buffering is drained
   388	            // before we drop the handle and apply mtime). Without
   389	            // this, set_file_mtime races with deferred writes from
   390	            // tokio's blocking-thread pool: 5/8 of mtimes were
   391	            // observed silently bumped to "now" on the receive side.
   392	            //
   393	            // POST_REVIEW_FIXES §1.1: flush failure is a data-loss
   394	            // signal — the user believes the file is durable when it
   395	            // isn't. Propagate, don't swallow.
   396	            file.flush()
   397	                .await
   398	                .with_context(|| format!("flushing {}", dst.display()))?;
   399	        }
   400	        // Handle dropped → kernel close() complete → no further
   401	        // metadata churn from this file. Now safe to set mtime by path.
   402	
   403	        // Intentionally no sync_all: ZFS commits per fsync are
   404	        // multi-second on spinning rust and crater throughput
   405	        // (9.3 → 3.3 Gbps observed). The transfer's durability signal
   406	        // is its END marker plus the OS's own flush; matches rsync's
   407	        // default behavior. Add a config flag if a caller needs sync.
   408	
   409	        if self.config.preserve_times && header.mtime_seconds > 0 {
   410	            let ft = FileTime::from_unix_time(header.mtime_seconds, 0);
   411	            // Best-effort: cross-fs, root-owned, or ACL-protected
   412	            // destinations can refuse mtime updates. Surface via
   413	            // `log::warn!` so the failure is visible without making
   414	            // it a hard transfer error. POST_REVIEW_FIXES §1.1.
   415	            if let Err(e) = filetime::set_file_mtime(&dst, ft) {
   416	                log::warn!("set mtime on {}: {}", dst.display(), e);
   417	            }
   418	        }
   419	
   420	        // Permissions arrive on the wire (Unix mode bits). Apply best-
   421	        // effort; ignore failures (cross-fs, root-owned dst, etc.).
   422	        #[cfg(unix)]
   423	        if header.permissions != 0 {
   424	            use std::os::unix::fs::PermissionsExt;
   425	            if let Err(e) =
   426	                std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(header.permissions))
   427	            {
   428	                log::warn!("set permissions on {}: {}", dst.display(), e);
   429	            }
   430	        }
   431	        #[cfg(not(unix))]
   432	        let _ = header.permissions;
   433	
   434	        self.track(&header.relative_path);
   435	
   436	        Ok(SinkOutcome {
   437	            files_written: 1,
   438	            bytes_written: header.size,
   439	        })
   440	    }
   441	
   442	    fn root(&self) -> &Path {
   443	        &self.dst_root
   444	    }
   445	}
   446	
   447	/// Copy a single file using the zero-copy cascade in `copy::file_copy`.
   448	fn write_file_payload(
   449	    src_root: &Path,
   450	    dst_root: &Path,
   451	    canonical_dst_root: Option<&Path>,
   452	    header: &FileHeader,
   453	    config: &FsSinkConfig,
   454	) -> Result<SinkOutcome> {
   455	    let src = src_root.join(&header.relative_path);
   456	    // R47-F1: the FsTransferSink::write_payload arm for
   457	    // PreparedPayload::File hit this helper, which previously
   458	    // joined dst_root + header.relative_path lexically. A peer-
   459	    // controlled `link/file` with a pre-existing `dst/link →
   460	    // /outside` symlink would write outside the destination root.
   461	    // Route through the same canonical-containment chokepoint that
   462	    // write_file_stream uses.
   463	    let dst = match canonical_dst_root {
   464	        Some(canonical) => {
   465	            crate::path_safety::safe_join_contained(canonical, dst_root, &header.relative_path)
   466	                .with_context(|| {
   467	                    format!("validating file payload path {:?}", header.relative_path)
   468	                })?
   469	        }
   470	        None => {
   471	            log::warn!(
   472	                "write_file_payload at '{}' has no canonical root; \
   473	                 falls back to lexical-only path check (R47-F1 \
   474	                 escape protection unavailable)",
   475	                dst_root.display()
   476	            );
   477	            crate::path_safety::safe_join(dst_root, &header.relative_path).with_context(|| {
   478	                format!("validating file payload path {:?}", header.relative_path)
   479	            })?
   480	        }
   481	    };
   482	
   483	    // R58-F4: dry-run must be side-effect-free. Bail before the
   484	    // parent-mkdir so a dry-run doesn't create destination
   485	    // directories on disk.
   486	    if config.dry_run {
   487	        return Ok(SinkOutcome {
   488	            files_written: 1,
   489	            bytes_written: 0,
   490	        });
   491	    }
   492	
   493	    if let Some(parent) = dst.parent() {
   494	        std::fs::create_dir_all(parent)
   495	            .with_context(|| format!("creating directory {}", parent.display()))?;
   496	    }
   497	
   498	    let mut did_copy = false;
   499	    let mut clone_succeeded = false;
   500	
   501	    if config.resume {
   502	        let outcome = resume_copy_file(&src, &dst, 0)
   503	            .with_context(|| format!("resume copy {}", header.relative_path))?;
   504	        did_copy = outcome.bytes_transferred > 0;
   505	    } else if crate::copy::file_needs_copy_with_mode(&src, &dst, config.compare_mode)? {
   506	        let sizer = BufferSizer::default();
   507	        let logger = NoopLogger;
   508	        let outcome = copy_file(&src, &dst, &sizer, false, &logger)
   509	            .with_context(|| format!("copy {}", header.relative_path))?;
   510	        did_copy = true;
   511	        clone_succeeded = outcome.clone_succeeded;
   512	    }
   513	
   514	    if config.preserve_times && did_copy && !clone_succeeded {
   515	        if let Ok(meta) = std::fs::metadata(&src) {
   516	            if let Ok(modified) = meta.modified() {
   517	                let ft = FileTime::from_system_time(modified);
   518	                if let Err(e) = filetime::set_file_mtime(&dst, ft) {
   519	                    log::warn!("set mtime on {}: {}", dst.display(), e);
   520	                }
   521	            }
   522	        }
   523	    }
   524	
   525	    Ok(SinkOutcome {
   526	        files_written: 1,
   527	        bytes_written: if did_copy { header.size } else { 0 },
   528	    })
   529	}
   530	
   531	/// Extract an in-memory tar shard to the destination directory.
   532	fn write_tar_shard_payload(
   533	    dst_root: &Path,
   534	    canonical_dst_root: Option<&Path>,
   535	    headers: &[FileHeader],
   536	    data: &[u8],
   537	    config: &FsSinkConfig,
   538	) -> Result<SinkOutcome> {
   539	    if config.dry_run {
   540	        return Ok(SinkOutcome {
   541	            files_written: headers.len(),
   542	            bytes_written: 0,
   543	        });
   544	    }
   545	
   546	    // Two-phase extraction:
   547	    //   1. Validate + parse the tar serially via the shared
   548	    //      `tar_safety` helper. Tar is a sequential format — entries
   549	    //      can't be read in parallel out of one Archive — and this
   550	    //      is also where R5-F2 / R6-F1 / R6-F3 safety checks live.
   551	    //   2. Write files to disk in parallel via rayon. Inode creation
   552	    //      and write are the bottleneck for many-small-files shards;
   553	    //      4–8 worker cores can saturate ZFS' inode pipeline.
   554	    //
   555	    // Empirically, sequential extraction was ~62 MiB/s on ZFS-on-HDD
   556	    // for 10k × 4 KiB; parallel raises the disk's small-file ceiling
   557	    // toward CPU-or-fs limits.
   558	    use rayon::prelude::*;
   559	
   560	    use super::tar_safety::{safe_extract_tar_shard, ExtractedFile, TarShardExtractOptions};
   561	
   562	    let opts = TarShardExtractOptions::default();
   563	    let mut extracted = safe_extract_tar_shard(data, headers.to_vec(), dst_root, &opts)?;
   564	
   565	    // R47-F1: tar shards arriving on FsTransferSink::write_payload
   566	    // (push-receive on the daemon flows through here too) only had
   567	    // lexical safe_join inside safe_extract_tar_shard. A pre-
   568	    // existing dst/link → /outside escape symlink would let an
   569	    // entry path like `link/victim` write through the symlink.
   570	    // Verify each extracted entry's destination against the
   571	    // canonical root before writing.
   572	    if let Some(canonical) = canonical_dst_root {
   573	        for f in &extracted {
   574	            crate::path_safety::verify_contained(canonical, &f.dest_path).with_context(|| {
   575	                format!("tar shard entry {:?} escapes destination root", f.dest_path)
   576	            })?;
   577	        }
   578	    } else {
   579	        log::warn!(
   580	            "write_tar_shard_payload at '{}' has no canonical root; \
   581	             tar-shard receive falls back to lexical-only path \
   582	             checks (R47-F1 escape protection unavailable)",
   583	            dst_root.display()
   584	        );
   585	    }
   586	
   587	    // Honor the sink's preserve_times toggle by stripping mtimes that
   588	    // the helper would otherwise apply. Permissions are best-effort
   589	    // either way (matches the historical FsTransferSink policy).
   590	    if !config.preserve_times {
   591	        for f in &mut extracted {
   592	            f.mtime = None;
   593	        }
   594	    }
   595	
   596	    // Write in parallel. Each closure does its own create_dir_all +
   597	    // fs::write + best-effort mtime/permission application — same
   598	    // policy as `tar_safety::write_extracted_file` but inlined so we
   599	    // can return per-file byte counts for the SinkOutcome.
   600	    let results: Vec<Result<u64>> = extracted
   601	        .into_par_iter()
   602	        .map(|f: ExtractedFile| -> Result<u64> {
   603	            if let Some(parent) = f.dest_path.parent() {
   604	                std::fs::create_dir_all(parent)
   605	                    .with_context(|| format!("create dir {}", parent.display()))?;
   606	            }
   607	            std::fs::write(&f.dest_path, &f.contents)
   608	                .with_context(|| format!("write {}", f.dest_path.display()))?;
   609	            if let Some(ft) = f.mtime {
   610	                if let Err(e) = filetime::set_file_mtime(&f.dest_path, ft) {
   611	                    log::warn!("set mtime on {}: {}", f.dest_path.display(), e);
   612	                }
   613	            }
   614	            #[cfg(unix)]
   615	            if let Some(perms) = f.permissions {
   616	                use std::os::unix::fs::PermissionsExt;
   617	                if let Err(e) =
   618	                    std::fs::set_permissions(&f.dest_path, std::fs::Permissions::from_mode(perms))
   619	                {
   620	                    log::warn!("set permissions on {}: {}", f.dest_path.display(), e);
   621	                }
   622	            }
   623	            Ok(f.size)
   624	        })
   625	        .collect();
   626	
   627	    let mut files_written = 0usize;
   628	    let mut bytes_written = 0u64;
   629	    for r in results {
   630	        bytes_written += r?;
   631	        files_written += 1;
   632	    }
   633	
   634	    Ok(SinkOutcome {
   635	        files_written,
   636	        bytes_written,
   637	    })
   638	}
   639	
   640	/// Resume protocol: overwrite a block of an existing file at the given offset.
   641	async fn write_file_block_payload(
   642	    dst_root: &Path,
   643	    canonical_dst_root: Option<&Path>,
   644	    relative_path: &str,
   645	    offset: u64,
   646	    bytes: Vec<u8>,
   647	) -> Result<SinkOutcome> {
   648	    use tokio::io::{AsyncSeekExt, AsyncWriteExt};
   649	
   650	    // R46-F3: contained resolve when canonical root is available.
   651	    let dst = match canonical_dst_root {
   652	        Some(canonical) => {
   653	            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
   654	                .with_context(|| format!("validating block-write path {:?}", relative_path))?
   655	        }
   656	        None => crate::path_safety::safe_join(dst_root, relative_path)
   657	            .with_context(|| format!("validating block-write path {:?}", relative_path))?,
   658	    };
   659	    let bytes_len = bytes.len() as u64;
   660	    // Resume blocks patch existing files at offset; we want to create
   661	    // if missing but never truncate (subsequent block records share
   662	    // the file).
   663	    let mut file = tokio::fs::OpenOptions::new()
   664	        .write(true)
   665	        .create(true)
   666	        .truncate(false)
   667	        .open(&dst)
   668	        .await
   669	        .with_context(|| format!("opening {} for block write", dst.display()))?;
   670	    file.seek(std::io::SeekFrom::Start(offset))
   671	        .await
   672	        .with_context(|| format!("seeking {} to offset {}", dst.display(), offset))?;
   673	    file.write_all(&bytes)
   674	        .await
   675	        .with_context(|| format!("writing block to {}", dst.display()))?;
   676	    Ok(SinkOutcome {
   677	        files_written: 0, // Resume blocks patch in-place; finalization counts the file.
   678	        bytes_written: bytes_len,
   679	    })
   680	}
   681	
   682	/// Resume protocol: finalize a resumed file by truncating to total_size,
   683	/// then stamp mtime + perms from the wire. The mtime stamp is what makes
   684	/// the "mtime touched, content identical" mirror case correct — block-hash
   685	/// compare sends zero blocks, but BLOCK_COMPLETE still updates the dest
   686	/// mtime to match the source.
   687	async fn write_file_block_complete(
   688	    dst_root: &Path,
   689	    canonical_dst_root: Option<&Path>,
   690	    relative_path: &str,
   691	    total_size: u64,
   692	    mtime_seconds: i64,
   693	    permissions: u32,
   694	) -> Result<SinkOutcome> {
   695	    // R46-F3: contained resolve when canonical root is available.
   696	    let dst = match canonical_dst_root {
   697	        Some(canonical) => {
   698	            crate::path_safety::safe_join_contained(canonical, dst_root, relative_path)
   699	                .with_context(|| format!("validating block-complete path {:?}", relative_path))?
   700	        }
   701	        None => crate::path_safety::safe_join(dst_root, relative_path)
   702	            .with_context(|| format!("validating block-complete path {:?}", relative_path))?,
   703	    };
   704	    {
   705	        let file = tokio::fs::OpenOptions::new()
   706	            .write(true)
   707	            .open(&dst)
   708	            .await
   709	            .with_context(|| format!("opening {} for truncation", dst.display()))?;
   710	        file.set_len(total_size)
   711	            .await
   712	            .with_context(|| format!("truncating {} to {}", dst.display(), total_size))?;
   713	        file.sync_all()
   714	            .await
   715	            .with_context(|| format!("syncing {}", dst.display()))?;
   716	    }
   717	    // Stamp mtime + perms after the file handle is closed (same race
   718	    // dance as write_file_stream — see commit 946bd77).
   719	    if mtime_seconds > 0 {
   720	        let ft = FileTime::from_unix_time(mtime_seconds, 0);
   721	        if let Err(e) = filetime::set_file_mtime(&dst, ft) {
   722	            log::warn!("set mtime on {}: {}", dst.display(), e);
   723	        }
   724	    }
   725	    #[cfg(unix)]
   726	    if permissions != 0 {
   727	        use std::os::unix::fs::PermissionsExt;
   728	        if let Err(e) = std::fs::set_permissions(&dst, std::fs::Permissions::from_mode(permissions))
   729	        {
   730	            log::warn!("set permissions on {}: {}", dst.display(), e);
   731	        }
   732	    }
   733	    #[cfg(not(unix))]
   734	    let _ = permissions;
   735	    Ok(SinkOutcome {
   736	        files_written: 1,
   737	        bytes_written: 0,
   738	    })
   739	}
   740	
   741	// ---------------------------------------------------------------------------
   742	// DataPlaneSink — TCP data plane writer
   743	// ---------------------------------------------------------------------------
   744	
   745	/// Writes payloads to a remote daemon via the TCP data plane binary protocol.
   746	///
   747	/// Each instance wraps a single TCP stream (DataPlaneSession). For multi-stream
   748	/// transfers, the pipeline executor creates multiple DataPlaneSink instances.
   749	pub struct DataPlaneSink<P: Probe = NoProbe> {
   750	    session: tokio::sync::Mutex<DataPlaneSession<P>>,
   751	    source: Arc<dyn TransferSource>,
   752	    dst_root: PathBuf,
   753	}
   754	
   755	impl<P: Probe> DataPlaneSink<P> {
   756	    pub fn new(
   757	        session: DataPlaneSession<P>,
   758	        source: Arc<dyn TransferSource>,
   759	        dst_root: PathBuf,
   760	    ) -> Self {
   761	        Self {
   762	            session: tokio::sync::Mutex::new(session),
   763	            source,
   764	            dst_root,
   765	        }
   766	    }
   767	}
   768	
   769	#[async_trait]
   770	impl<P: Probe> TransferSink for DataPlaneSink<P> {
   771	    async fn write_payload(&self, payload: PreparedPayload) -> Result<SinkOutcome> {
   772	        let mut session = self.session.lock().await;
   773	        match payload {
   774	            PreparedPayload::File(header) => {
   775	                let size = header.size;
   776	                session
   777	                    .send_file(self.source.clone(), &header)
   778	                    .await
   779	                    .with_context(|| format!("sending {}", header.relative_path))?;
   780	                Ok(SinkOutcome {
   781	                    files_written: 1,
   782	                    bytes_written: size,
   783	                })
   784	            }
   785	            PreparedPayload::TarShard { headers, data } => {
   786	                let bytes: u64 = headers.iter().map(|h| h.size).sum();
   787	                let count = headers.len();
   788	                session
   789	                    .send_prepared_tar_shard(headers, &data)
   790	                    .await
   791	                    .context("sending tar shard")?;
   792	                Ok(SinkOutcome {
   793	                    files_written: count,
   794	                    bytes_written: bytes,
   795	                })
   796	            }
   797	            // Resume payloads can't be relayed without a reverse-resume
   798	            // protocol on the next hop. Reject explicitly.
   799	            PreparedPayload::FileBlock { .. } | PreparedPayload::FileBlockComplete { .. } => {
   800	                eyre::bail!("DataPlaneSink does not relay resume-block payloads")
   801	            }
   802	        }
   803	    }
   804	
   805	    /// Relay case: bytes arrive on `reader` (e.g. from a DataPlaneSource
   806	    /// during a remote→remote transfer) and forward to the next hop.
   807	    async fn write_file_stream(
   808	        &self,
   809	        header: &FileHeader,
   810	        reader: &mut (dyn tokio::io::AsyncRead + Unpin + Send),
   811	    ) -> Result<SinkOutcome> {
   812	        let size = header.size;
   813	        let mut session = self.session.lock().await;
   814	        session
   815	            .send_file_from_reader(header, reader)
   816	            .await
   817	            .with_context(|| format!("relaying {}", header.relative_path))?;
   818	        Ok(SinkOutcome {
   819	            files_written: 1,
   820	            bytes_written: size,
   821	        })
   822	    }
   823	
   824	    async fn finish(&self) -> Result<()> {
   825	        let mut session = self.session.lock().await;
   826	        session.finish().await
   827	    }
   828	
   829	    fn root(&self) -> &Path {
   830	        &self.dst_root
   831	    }
   832	}
   833	
   834	// ---------------------------------------------------------------------------
   835	// NullSink — discard data, count bytes (for benchmarking)
   836	// ---------------------------------------------------------------------------
   837	
   838	/// Discards all payload data, counting files and bytes.
   839	///
   840	/// Useful for benchmarking source + network throughput without destination
   841	/// I/O as a bottleneck. The pipeline still prepares payloads (reading source
   842	/// files, building tar shards) so this measures everything except the write.
   843	pub struct NullSink {
   844	    label: PathBuf,
   845	}
   846	
   847	impl Default for NullSink {
   848	    fn default() -> Self {
   849	        Self {
   850	            label: PathBuf::from("/dev/null"),
   851	        }
   852	    }
   853	}
   854	
   855	impl NullSink {
   856	    pub fn new() -> Self {
   857	        Self::default()
   858	    }
   859	}
   860	

 succeeded in 0ms:
   380	    if let Some(err) = first_err {
   381	        return Err(err);
   382	    }
   383	
   384	    let result = total.lock().unwrap().clone();
   385	    Ok(result)
   386	}
   387	
   388	// =====================================================================
   389	// Receive pipeline — symmetric counterpart of execute_sink_pipeline.
   390	// =====================================================================
   391	
   392	use crate::generated::FileHeader;
   393	use eyre::bail;
   394	use tokio::io::{AsyncRead, AsyncReadExt};
   395	
   396	use super::data_plane::{
   397	    DATA_PLANE_RECORD_BLOCK, DATA_PLANE_RECORD_BLOCK_COMPLETE, DATA_PLANE_RECORD_END,
   398	    DATA_PLANE_RECORD_FILE, DATA_PLANE_RECORD_TAR_SHARD,
   399	};
   400	
   401	/// Drive a `TransferSink` from a TCP wire stream.
   402	///
   403	/// This is the symmetric counterpart to [`execute_sink_pipeline_streaming`]:
   404	/// where the outbound executor takes a [`TransferSource`] and dispatches
   405	/// payloads round-robin across N sinks, this one consumes a single
   406	/// inbound wire (parsing record headers and producing
   407	/// [`PreparedPayload::FileStream`] / [`PreparedPayload::TarShard`] /
   408	/// [`PreparedPayload::FileBlock`] events) and feeds them to a single sink
   409	/// sequentially. Multi-stream parallelism comes from spawning N invocations,
   410	/// one per inbound TCP connection.
   411	///
   412	/// Both directions converge on `TransferSink::write_payload`: file data
   413	/// hits disk through `FsTransferSink::write_payload(FileStream { … })`,
   414	/// which uses the same `receive_stream_double_buffered` helper as the
   415	/// daemon's push receiver and the client's pull receiver — one path,
   416	/// one optimization surface.
   417	pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
   418	    socket: &mut R,
   419	    sink: Arc<dyn TransferSink>,
   420	    progress: Option<&RemoteTransferProgress>,
   421	) -> Result<SinkOutcome> {
   422	    let mut total = SinkOutcome::default();
   423	
   424	    loop {
   425	        let mut tag = [0u8; 1];
   426	        socket
   427	            .read_exact(&mut tag)
   428	            .await
   429	            .context("reading data-plane record tag")?;
   430	
   431	        match tag[0] {
   432	            DATA_PLANE_RECORD_END => break,
   433	            DATA_PLANE_RECORD_FILE => {
   434	                let mut header = read_file_header(socket).await?;
   435	                let file_size = read_u64(socket).await?;
   436	                let mtime = read_i64(socket).await?;
   437	                let perms = read_u32(socket).await?;
   438	                header.size = file_size;
   439	                header.mtime_seconds = mtime;
   440	                header.permissions = perms;
   441	                // Use AsyncReadExt::take to give the sink exactly
   442	                // file_size bytes of the wire. tokio's Take is the
   443	                // canonical way to limit a borrowed AsyncRead.
   444	                use tokio::io::AsyncReadExt;
   445	                let mut reader = (&mut *socket).take(file_size);
   446	                let outcome = sink
   447	                    .write_file_stream(&header, &mut reader)
   448	                    .await
   449	                    .with_context(|| format!("receiving {}", header.relative_path))?;
   450	                if let Some(p) = progress {
   451	                    p.report_payload(0, outcome.bytes_written);
   452	                    p.report_file_complete(header.relative_path.clone());
   453	                }
   454	                total.merge(&outcome);
   455	            }
   456	            DATA_PLANE_RECORD_TAR_SHARD => {
   457	                let (headers, data) = read_tar_shard(socket).await?;
   458	                let bytes = data.len() as u64;
   459	                // Capture member paths for the per-file lane before the
   460	                // payload takes ownership; skip the allocation when no
   461	                // one is listening (the daemon receive path).
   462	                let member_paths: Option<Vec<String>> =
   463	                    progress.map(|_| headers.iter().map(|h| h.relative_path.clone()).collect());
   464	                let payload = PreparedPayload::TarShard { headers, data };
   465	                let outcome = sink
   466	                    .write_payload(payload)
   467	                    .await
   468	                    .context("writing payload")?;
   469	                if let Some(p) = progress {
   470	                    p.report_payload(0, bytes);
   471	                    for path in member_paths.unwrap_or_default() {
   472	                        p.report_file_complete(path);
   473	                    }
   474	                }
   475	                total.merge(&outcome);
   476	            }
   477	            DATA_PLANE_RECORD_BLOCK => {
   478	                let path = read_string(socket).await?;
   479	                let offset = read_u64(socket).await?;
   480	                let len = read_u32(socket).await? as usize;
   481	                if len > MAX_WIRE_BLOCK_BYTES {
   482	                    bail!(
   483	                        "wire block payload {} bytes exceeds max {} (rejecting to avoid OOM)",
   484	                        len,
   485	                        MAX_WIRE_BLOCK_BYTES
   486	                    );
   487	                }
   488	                let mut bytes = vec![0u8; len];
   489	                socket
   490	                    .read_exact(&mut bytes)
   491	                    .await
   492	                    .context("reading block bytes")?;
   493	                let payload = PreparedPayload::FileBlock {
   494	                    relative_path: path,
   495	                    offset,
   496	                    bytes,
   497	                };
   498	                let outcome = sink
   499	                    .write_payload(payload)
   500	                    .await
   501	                    .context("writing payload")?;
   502	                if let Some(p) = progress {
   503	                    p.report_payload(0, outcome.bytes_written);
   504	                }
   505	                total.merge(&outcome);
   506	            }
   507	            DATA_PLANE_RECORD_BLOCK_COMPLETE => {
   508	                let path = read_string(socket).await?;
   509	                let total_size = read_u64(socket).await?;
   510	                let mtime = read_i64(socket).await?;
   511	                let perms = read_u32(socket).await?;
   512	                let path_for_progress = progress.map(|_| path.clone());
   513	                let payload = PreparedPayload::FileBlockComplete {
   514	                    relative_path: path,
   515	                    total_size,
   516	                    mtime_seconds: mtime,
   517	                    permissions: perms,
   518	                };
   519	                let outcome = sink
   520	                    .write_payload(payload)
   521	                    .await
   522	                    .context("writing payload")?;
   523	                if let Some(p) = progress {
   524	                    p.report_file_complete(path_for_progress.unwrap_or_default());
   525	                }
   526	                total.merge(&outcome);
   527	            }
   528	            other => bail!("unknown data-plane record tag: 0x{:02X}", other),
   529	        }
   530	    }
   531	
   532	    sink.finish().await.context("finalising sink")?;
   533	    Ok(total)
   534	}
   535	
   536	async fn read_u32<R: AsyncRead + Unpin>(socket: &mut R) -> Result<u32> {
   537	    let mut buf = [0u8; 4];
   538	    socket.read_exact(&mut buf).await.context("reading u32")?;
   539	    Ok(u32::from_be_bytes(buf))
   540	}

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/data_plane.rs | sed -n '420,760p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '540,660p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   540	}
   541	
   542	async fn read_u64<R: AsyncRead + Unpin>(socket: &mut R) -> Result<u64> {
   543	    let mut buf = [0u8; 8];
   544	    socket.read_exact(&mut buf).await.context("reading u64")?;
   545	    Ok(u64::from_be_bytes(buf))
   546	}
   547	
   548	async fn read_i64<R: AsyncRead + Unpin>(socket: &mut R) -> Result<i64> {
   549	    let mut buf = [0u8; 8];
   550	    socket.read_exact(&mut buf).await.context("reading i64")?;
   551	    Ok(i64::from_be_bytes(buf))
   552	}
   553	
   554	/// Maximum path length accepted from the wire. Higher than POSIX
   555	/// PATH_MAX (4096) but bounded so a hostile peer can't trigger a
   556	/// many-GB allocation by sending u32::MAX as a path length.
   557	const MAX_WIRE_PATH_LEN: usize = 64 * 1024;
   558	/// Maximum file count per tar shard. The planner targets up to a few
   559	/// thousand entries per shard; this bound prevents a wire-driven
   560	/// `Vec::with_capacity(u32::MAX)` allocation.
   561	const MAX_WIRE_TAR_SHARD_FILES: usize = 1_048_576;
   562	/// Maximum tar shard payload size (in bytes). Single source of truth
   563	/// is `tar_safety::MAX_TAR_SHARD_BYTES` so the wire-side reader
   564	/// rejects shards the receive-side helper would reject anyway.
   565	/// Previously inconsistent: wire was 1 GiB, helper was 256 MiB —
   566	/// closing F8 of `docs/reviews/codebase_review_2026-05-01.md`.
   567	const MAX_WIRE_TAR_SHARD_BYTES: usize =
   568	    crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES as usize;
   569	/// Maximum single-block payload size on the resume protocol. Aligns
   570	/// with `crate::copy::MAX_BLOCK_SIZE`.
   571	const MAX_WIRE_BLOCK_BYTES: usize = 64 * 1024 * 1024;
   572	
   573	async fn read_string<R: AsyncRead + Unpin>(socket: &mut R) -> Result<String> {
   574	    let len = read_u32(socket).await? as usize;
   575	    if len > MAX_WIRE_PATH_LEN {
   576	        bail!(
   577	            "wire path length {} exceeds max {} (rejecting to avoid OOM)",
   578	            len,
   579	            MAX_WIRE_PATH_LEN
   580	        );
   581	    }
   582	    let mut buf = vec![0u8; len];
   583	    socket
   584	        .read_exact(&mut buf)
   585	        .await
   586	        .context("reading string bytes")?;
   587	    String::from_utf8(buf).context("invalid UTF-8 in data-plane string")
   588	}
   589	
   590	async fn read_file_header<R: AsyncRead + Unpin>(socket: &mut R) -> Result<FileHeader> {
   591	    let path = read_string(socket).await?;
   592	    // Validate at the wire boundary — rejects ../, absolute paths,
   593	    // Windows drive prefixes, UNC, NUL bytes. Sinks re-validate via
   594	    // `safe_join` (defense in depth), but failing here keeps unsafe
   595	    // headers out of the FileHeader stream entirely.
   596	    crate::path_safety::validate_wire_path(&path)
   597	        .with_context(|| format!("rejecting wire file header path {:?}", path))?;
   598	    Ok(FileHeader {
   599	        relative_path: path,
   600	        size: 0, // populated by caller from the file_size field on the wire
   601	        mtime_seconds: 0,
   602	        permissions: 0,
   603	        checksum: vec![],
   604	    })
   605	}
   606	
   607	async fn read_tar_shard<R: AsyncRead + Unpin>(
   608	    socket: &mut R,
   609	) -> Result<(Vec<FileHeader>, Vec<u8>)> {
   610	    let count = read_u32(socket).await? as usize;
   611	    if count > MAX_WIRE_TAR_SHARD_FILES {
   612	        bail!(
   613	            "wire tar shard file count {} exceeds max {} (rejecting to avoid OOM)",
   614	            count,
   615	            MAX_WIRE_TAR_SHARD_FILES
   616	        );
   617	    }
   618	    let mut headers = Vec::with_capacity(count);
   619	    for _ in 0..count {
   620	        let path = read_string(socket).await?;
   621	        crate::path_safety::validate_wire_path(&path)
   622	            .with_context(|| format!("rejecting wire tar shard header path {:?}", path))?;
   623	        let size = read_u64(socket).await?;
   624	        let mtime = read_i64(socket).await?;
   625	        let permissions = read_u32(socket).await?;
   626	        headers.push(FileHeader {
   627	            relative_path: path,
   628	            size,
   629	            mtime_seconds: mtime,
   630	            permissions,
   631	            checksum: vec![],
   632	        });
   633	    }
   634	    let tar_size = read_u64(socket).await?;
   635	    if tar_size > MAX_WIRE_TAR_SHARD_BYTES as u64 {
   636	        bail!(
   637	            "wire tar shard payload {} bytes exceeds max {} (rejecting to avoid OOM)",
   638	            tar_size,
   639	            MAX_WIRE_TAR_SHARD_BYTES
   640	        );
   641	    }
   642	    let mut data = vec![0u8; tar_size as usize];
   643	    socket
   644	        .read_exact(&mut data)
   645	        .await
   646	        .context("reading tar shard bytes")?;
   647	    Ok((headers, data))
   648	}
   649	
   650	#[cfg(test)]
   651	mod tests {
   652	    use super::*;
   653	    use crate::generated::ComparisonMode;
   654	    use crate::remote::transfer::sink::{FsSinkConfig, FsTransferSink, TransferSink};
   655	    use crate::remote::transfer::source::FsTransferSource;
   656	    use std::path::{Path, PathBuf};
   657	    use std::sync::Mutex;
   658	    use tempfile::tempdir;
   659	
   660	    /// Sink that fails the first `write_payload` with a recognisable

 succeeded in 0ms:
   420	        // attributable to socket-write backpressure.
   421	        if bytes_a > 0 {
   422	            let tail_start = if P::ACTIVE {
   423	                Some(std::time::Instant::now())
   424	            } else {
   425	                None
   426	            };
   427	            self.stream
   428	                .write_all(&buf_a.as_slice()[..bytes_a])
   429	                .await
   430	                .with_context(|| format!("sending {}", rel))?;
   431	            if P::ACTIVE {
   432	                if let Some(t) = tail_start {
   433	                    self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   434	                }
   435	            }
   436	            self.probe.record_bytes(bytes_a as u64);
   437	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(bytes_a as u64);
   438	        }
   439	
   440	        // Buffers return to pool automatically on drop
   441	        Ok(())
   442	    }
   443	
   444	    pub async fn send_prepared_tar_shard(
   445	        &mut self,
   446	        headers: Vec<FileHeader>,
   447	        data: &[u8],
   448	    ) -> Result<()> {
   449	        let shard_len = headers.len();
   450	        let preview = headers
   451	            .first()
   452	            .map(|h| h.relative_path.as_str())
   453	            .unwrap_or("<empty>");
   454	        trace_client!(
   455	            self,
   456	            "sending tar shard with {} file(s), {} bytes (first='{}')",
   457	            shard_len,
   458	            data.len(),
   459	            preview
   460	        );
   461	        self.stream
   462	            .write_all(&[DATA_PLANE_RECORD_TAR_SHARD])
   463	            .await
   464	            .context("writing tar shard record tag")?;
   465	        self.stream
   466	            .write_all(&(headers.len() as u32).to_be_bytes())
   467	            .await
   468	            .context("writing tar shard count")?;
   469	
   470	        for header in headers {
   471	            let rel_bytes = header.relative_path.as_bytes();
   472	            if rel_bytes.len() > u32::MAX as usize {
   473	                bail!(
   474	                    "relative path too long for transfer: {}",
   475	                    header.relative_path
   476	                );
   477	            }
   478	            self.stream
   479	                .write_all(&(rel_bytes.len() as u32).to_be_bytes())
   480	                .await
   481	                .context("writing shard path length")?;
   482	            self.stream
   483	                .write_all(rel_bytes)
   484	                .await
   485	                .context("writing shard path bytes")?;
   486	            self.stream
   487	                .write_all(&header.size.to_be_bytes())
   488	                .await
   489	                .context("writing shard size")?;
   490	            self.stream
   491	                .write_all(&header.mtime_seconds.to_be_bytes())
   492	                .await
   493	                .context("writing shard mtime")?;
   494	            self.stream
   495	                .write_all(&header.permissions.to_be_bytes())
   496	                .await
   497	                .context("writing shard permissions")?;
   498	        }
   499	
   500	        self.stream
   501	            .write_all(&(data.len() as u64).to_be_bytes())
   502	            .await
   503	            .context("writing tar shard length")?;
   504	        for chunk in data.chunks(self.chunk_bytes.max(1)) {
   505	            // codex ue-r2-1e F3: shard writes carry the small-file
   506	            // workloads — without a blocked signal here the tuner sees
   507	            // a saturated link as a clean one. Same P::ACTIVE gating as
   508	            // the file loop: NoProbe reads no clock.
   509	            let started = if P::ACTIVE {
   510	                Some(std::time::Instant::now())
   511	            } else {
   512	                None
   513	            };
   514	            self.stream
   515	                .write_all(chunk)
   516	                .await
   517	                .context("writing tar shard payload")?;
   518	            if let Some(t) = started {
   519	                self.probe.note_write_blocked(t.elapsed().as_nanos() as u64);
   520	            }
   521	            self.probe.record_bytes(chunk.len() as u64);
   522	            crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(chunk.len() as u64);
   523	        }
   524	        trace_client!(
   525	            self,
   526	            "tar shard payload sent ({} file(s), {} bytes)",
   527	            shard_len,
   528	            data.len()
   529	        );
   530	
   531	        Ok(())
   532	    }
   533	
   534	    /// Send a single block for block-level resume.
   535	    /// Format: [type:1][path_len:4][path][offset:8][block_len:4][content]
   536	    pub async fn send_block(
   537	        &mut self,
   538	        relative_path: &str,
   539	        offset: u64,
   540	        content: &[u8],
   541	    ) -> Result<()> {
   542	        let path_bytes = relative_path.as_bytes();
   543	        if path_bytes.len() > u32::MAX as usize {
   544	            bail!("relative path too long for transfer: {}", relative_path);
   545	        }
   546	
   547	        trace_client!(
   548	            self,
   549	            "sending block for '{}' at offset {} ({} bytes)",
   550	            relative_path,
   551	            offset,
   552	            content.len()
   553	        );
   554	
   555	        self.stream
   556	            .write_all(&[DATA_PLANE_RECORD_BLOCK])
   557	            .await
   558	            .context("writing block record tag")?;
   559	        self.stream
   560	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   561	            .await
   562	            .context("writing path length")?;
   563	        self.stream
   564	            .write_all(path_bytes)
   565	            .await
   566	            .context("writing path bytes")?;
   567	        self.stream
   568	            .write_all(&offset.to_be_bytes())
   569	            .await
   570	            .context("writing block offset")?;
   571	        self.stream
   572	            .write_all(&(content.len() as u32).to_be_bytes())
   573	            .await
   574	            .context("writing block length")?;
   575	        self.stream
   576	            .write_all(content)
   577	            .await
   578	            .context("writing block content")?;
   579	        crate::remote::instrumentation::record_cli_data_plane_outbound_bytes(content.len() as u64);
   580	        self.probe.record_bytes(content.len() as u64);
   581	
   582	        self.bytes_sent += content.len() as u64;
   583	        Ok(())
   584	    }
   585	
   586	    /// Signal that block-level transfer for a file is complete.
   587	    /// Format: [type:1][path_len:4][path][total_size:8][mtime:8][perms:4]
   588	    ///
   589	    /// Carries mtime + perms inline so the receiver can stamp the
   590	    /// destination metadata even when zero blocks transferred (the
   591	    /// "mtime touched, content identical" case for mirror).
   592	    pub async fn send_block_complete(
   593	        &mut self,
   594	        relative_path: &str,
   595	        total_size: u64,
   596	        mtime_seconds: i64,
   597	        permissions: u32,
   598	    ) -> Result<()> {
   599	        let path_bytes = relative_path.as_bytes();
   600	        if path_bytes.len() > u32::MAX as usize {
   601	            bail!("relative path too long for transfer: {}", relative_path);
   602	        }
   603	
   604	        trace_client!(
   605	            self,
   606	            "sending block complete for '{}' ({} bytes total)",
   607	            relative_path,
   608	            total_size
   609	        );
   610	
   611	        self.stream
   612	            .write_all(&[DATA_PLANE_RECORD_BLOCK_COMPLETE])
   613	            .await
   614	            .context("writing block complete record tag")?;
   615	        self.stream
   616	            .write_all(&(path_bytes.len() as u32).to_be_bytes())
   617	            .await
   618	            .context("writing path length")?;
   619	        self.stream
   620	            .write_all(path_bytes)
   621	            .await
   622	            .context("writing path bytes")?;
   623	        self.stream
   624	            .write_all(&total_size.to_be_bytes())
   625	            .await
   626	            .context("writing total size")?;
   627	        self.stream
   628	            .write_all(&mtime_seconds.to_be_bytes())
   629	            .await
   630	            .context("writing mtime")?;
   631	        self.stream
   632	            .write_all(&permissions.to_be_bytes())
   633	            .await
   634	            .context("writing permissions")?;
   635	
   636	        Ok(())
   637	    }
   638	}
   639	
   640	/// Default buffer size for the receive path. Deliberately smaller than
   641	/// the send side's pooled buffers (the dial's chunk_bytes, 16–64 MiB):
   642	/// the wire carries no per-chunk framing, so the receiver is free to
   643	/// drain a 16 MiB send in 1 MiB slices — it only needs to be large
   644	/// enough that per-syscall overhead doesn't dominate at 10 GbE, and
   645	/// that ZFS-style transactional filesystems can amortize per-write
   646	/// costs. Tuning the receive side into the dial is future work (W3.1
   647	/// scoped it out; see the w3-1 finding doc's Known gaps).
   648	///
   649	/// Empirically, 8 KiB caps push throughput at ~1 Gbps on EPYC/ZFS even
   650	/// when the network can do 9.4 Gbps and the disk can do 14.76 Gbps.
   651	/// 1 MiB lets the receiver keep up with the sender's double-buffered
   652	/// pipeline.
   653	pub const RECEIVE_CHUNK_SIZE: usize = 1024 * 1024;
   654	
   655	/// Stream `expected` bytes from an async source into an async sink with
   656	/// double-buffered I/O — while one buffer drains to disk, the other is
   657	/// being filled from the wire. Symmetric counterpart of
   658	/// `DataPlaneSession::send_file_double_buffered`.
   659	///
   660	/// Both the daemon's push receiver (writing to disk from a TCP socket)
   661	/// and the client's pull receiver (same shape, opposite direction) call
   662	/// this so the receive side has the same throughput characteristics as
   663	/// the send side. Replacing this with `tokio::io::copy` (8 KiB internal
   664	/// buffer) caps real-world transfers at ~1 Gbps regardless of network
   665	/// or disk speed.
   666	///
   667	/// Returns the number of bytes copied. Errors on early EOF.
   668	///
   669	/// `byte_progress` (optional) gets a `report(delta)` call after
   670	/// each successful chunk write. Cadence matches the receive
   671	/// buffer size (`buffer_size`; clamped ≥ 64 KiB), so a 10 GiB
   672	/// transfer at the default 1 MiB chunk size emits ~10 000
   673	/// reports. Callers that don't need byte-level instrumentation
   674	/// pass `None` and pay nothing — the inner loop's
   675	/// `if let Some(p)` branch is a single predicted-taken jump.
   676	pub async fn receive_stream_double_buffered<R, W>(
   677	    src: &mut R,
   678	    dst: &mut W,
   679	    expected: u64,
   680	    buffer_size: usize,
   681	    byte_progress: Option<&crate::remote::transfer::progress::ByteProgressSink>,
   682	) -> Result<u64>
   683	where
   684	    R: tokio::io::AsyncRead + Unpin + ?Sized,
   685	    W: tokio::io::AsyncWrite + Unpin + ?Sized,
   686	{
   687	    if expected == 0 {
   688	        return Ok(0);
   689	    }
   690	
   691	    let cap = buffer_size.max(crate::buffer::DATA_PLANE_BUFFER_FLOOR);
   692	    let mut buf_a = vec![0u8; cap];
   693	    let mut buf_b = vec![0u8; cap];
   694	
   695	    // Initial fill of buf_a.
   696	    let mut bytes_a = read_up_to(src, &mut buf_a, expected).await?;
   697	    if bytes_a == 0 {
   698	        bail!("unexpected EOF: 0 bytes received, {} expected", expected);
   699	    }
   700	    let mut total: u64 = bytes_a as u64;
   701	
   702	    while total < expected {
   703	        let want_b = (expected - total).min(buf_b.len() as u64);
   704	        let (write_res, read_res) = tokio::join!(
   705	            dst.write_all(&buf_a[..bytes_a]),
   706	            read_up_to(src, &mut buf_b, want_b),
   707	        );
   708	        write_res.context("writing received bytes to disk")?;
   709	        // Report the bytes that just landed on disk. We report
   710	        // AFTER `write_all` succeeds so a `bytes_completed`
   711	        // observed by GetState never exceeds bytes actually
   712	        // written (mid-failure transfers stay accurate too —
   713	        // the post-Drop record holds the value at last success).
   714	        if let Some(progress) = byte_progress {
   715	            progress.report(bytes_a as u64);
   716	        }
   717	        let bytes_b = read_res?;
   718	        if bytes_b == 0 && total + bytes_a as u64 != expected {
   719	            bail!(
   720	                "unexpected EOF: {} bytes received, {} expected",
   721	                total + bytes_a as u64,
   722	                expected
   723	            );
   724	        }
   725	        total += bytes_b as u64;
   726	        std::mem::swap(&mut buf_a, &mut buf_b);
   727	        bytes_a = bytes_b;
   728	    }
   729	
   730	    if bytes_a > 0 {
   731	        dst.write_all(&buf_a[..bytes_a])
   732	            .await
   733	            .context("writing final chunk to disk")?;
   734	        if let Some(progress) = byte_progress {
   735	            progress.report(bytes_a as u64);
   736	        }
   737	    }
   738	
   739	    Ok(total)
   740	}
   741	
   742	/// Read up to `cap` bytes (clamped to the slice length) from `src`,
   743	/// returning how many were read. Returns 0 only on EOF or zero-cap.
   744	async fn read_up_to<R>(src: &mut R, buf: &mut [u8], cap: u64) -> Result<usize>
   745	where
   746	    R: tokio::io::AsyncRead + Unpin + ?Sized,
   747	{
   748	    let take = (buf.len() as u64).min(cap) as usize;
   749	    if take == 0 {
   750	        return Ok(0);
   751	    }
   752	    let n = src
   753	        .read(&mut buf[..take])
   754	        .await
   755	        .context("reading from data plane stream")?;
   756	    Ok(n)
   757	}
   758	
   759	#[cfg(test)]
   760	mod byte_progress_tests {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/socket.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Shared socket policy for data-plane TCP streams (w1-2).
     2	//!
     3	//! Every data-plane socket — the push client connect, the pull client
     4	//! connect, and all daemon accept paths (push epoch-0/resize,
     5	//! pull_sync epoch-0/resize/resume) — routes through
     6	//! [`configure_data_socket`], the single owner of the
     7	//! NODELAY/keepalive/tuned-buffer policy. Before this module the
     8	//! policy existed on push sockets only; the pull direction ran with
     9	//! Nagle enabled and the tuner's `tcp_buffer_bytes` was computed and
    10	//! discarded (design map §1.1, finding
    11	//! boundaries-pull-direction-bypasses-socket-policy).
    12	//!
    13	//! design-3 added [`dial_data_plane`]: the client-side dial (bounded
    14	//! connect + policy + bounded handshake write) lives here too, so
    15	//! both data-plane connect sites share one owner and neither can
    16	//! regress to an unbounded `TcpStream::connect`.
    17	
    18	use std::io;
    19	use std::time::Duration;
    20	
    21	use eyre::Context as _;
    22	use socket2::{SockRef, TcpKeepalive};
    23	use tokio::io::AsyncWriteExt;
    24	use tokio::net::TcpStream;
    25	
    26	/// Bounded wait for a data-plane accept (w1-4: one shared pair — this
    27	/// and [`DATA_PLANE_TOKEN_TIMEOUT`] — replacing three per-file
    28	/// declarations of the same two values). R46-F7 lineage: pre-fix the
    29	/// daemon called `listener.accept().await` with no timeout — a peer
    30	/// that opened the control connection but never opened the data
    31	/// connection (or hung mid-handshake) would pin the daemon's stream
    32	/// task indefinitely, holding the listener and the queued work. 30 s
    33	/// gives a generous margin for slow networks while still bounding the
    34	/// worst case.
    35	pub const DATA_PLANE_ACCEPT_TIMEOUT: Duration = Duration::from_secs(30);
    36	/// Bounded wait for the handshake-token bytes after a TCP accept.
    37	/// R46-F7: pre-fix `read_exact(&mut token_buf).await` had no timeout —
    38	/// a peer that opened the socket and stalled would hold the stream
    39	/// worker forever. 15 s is enough for a healthy peer to send a few
    40	/// dozen bytes; anything slower is a stuck or hostile peer.
    41	pub const DATA_PLANE_TOKEN_TIMEOUT: Duration = Duration::from_secs(15);
    42	
    43	/// Idle time before the first keepalive probe (w1-3). Before this the
    44	/// sockets ran `SO_KEEPALIVE` with OS-default timing (~2 h idle on
    45	/// every supported platform) — useless on transfer timescales, while
    46	/// the comments claimed it prevented idle stream timeouts. With
    47	/// 60 s + 5 probes at 10 s, a vanished peer on an idle data socket
    48	/// (an armed resize slot, a stream waiting for work while siblings
    49	/// transfer) is detected in ~2 minutes. The complementary case — a
    50	/// stalled peer with data in flight — is StallGuard's 30 s, not
    51	/// keepalive's.
    52	pub const TCP_KEEPALIVE_IDLE: Duration = Duration::from_secs(60);
    53	/// Interval between keepalive probes once idle has elapsed.
    54	pub const TCP_KEEPALIVE_INTERVAL: Duration = Duration::from_secs(10);
    55	/// Unanswered probes before the connection is declared dead.
    56	pub const TCP_KEEPALIVE_RETRIES: u32 = 5;
    57	
    58	/// Apply the data-plane socket policy to a connected or accepted
    59	/// stream, in place (no `into_std`/`from_std` round trip):
    60	///
    61	/// - `TCP_NODELAY` on — **hard error**. Nagle on a data-plane socket
    62	///   silently serializes small records behind ACKs; a socket we cannot
    63	///   configure is a socket we do not use.
    64	/// - `SO_KEEPALIVE` on with explicit timing
    65	///   ([`TCP_KEEPALIVE_IDLE`]/[`TCP_KEEPALIVE_INTERVAL`]/
    66	///   [`TCP_KEEPALIVE_RETRIES`]) — best-effort, logged. Detects a
    67	///   vanished peer on an idle data socket within ~2 minutes instead of
    68	///   the OS-default ~2 hours; the kernel can refuse on exotic socket
    69	///   types (POST_REVIEW_FIXES §1.1 lineage — failure is loud, never
    70	///   fatal).
    71	/// - Send/receive buffers sized to `tcp_buffer_size` when `Some` —
    72	///   best-effort, logged. The knobs are advisory (the kernel can
    73	///   clamp); a failure here should be visible to operators chasing a
    74	///   sysctl/rlimit mismatch, never fatal. `None` = kernel default —
    75	///   the value is a connect-time snapshot of
    76	///   [`TransferDial::tcp_buffer_bytes`](crate::engine::TransferDial::tcp_buffer_bytes)
    77	///   where a dial is in scope (epoch-0 sockets therefore run kernel
    78	///   defaults; resize-ADD sockets get the ramped size), and `None`
    79	///   where none is (the pull client and the daemon push receiver hold
    80	///   no dial).
    81	///
    82	/// Errors only if `TCP_NODELAY` cannot be set (or the fd/socket
    83	/// handle is unusable, which the same call surfaces).
    84	pub fn configure_data_socket(stream: &TcpStream, tcp_buffer_size: Option<usize>) -> io::Result<()> {
    85	    let socket = SockRef::from(stream);
    86	    socket.set_tcp_nodelay(true)?;
    87	    // `set_tcp_keepalive` also flips SO_KEEPALIVE on, so this is the
    88	    // whole keepalive story in one call.
    89	    let keepalive = TcpKeepalive::new()
    90	        .with_time(TCP_KEEPALIVE_IDLE)
    91	        .with_interval(TCP_KEEPALIVE_INTERVAL)
    92	        .with_retries(TCP_KEEPALIVE_RETRIES);
    93	    if let Err(e) = socket.set_tcp_keepalive(&keepalive) {
    94	        log::warn!("set TCP keepalive on data-plane socket: {}", e);
    95	    }
    96	    if let Some(size) = tcp_buffer_size {
    97	        if let Err(e) = socket.set_send_buffer_size(size) {
    98	            log::warn!("set TCP send buffer to {} bytes: {}", size, e);
    99	        }
   100	        if let Err(e) = socket.set_recv_buffer_size(size) {
   101	            log::warn!("set TCP recv buffer to {} bytes: {}", size, e);
   102	        }
   103	    }
   104	    Ok(())
   105	}
   106	
   107	/// design-3: dial a data-plane endpoint with the shared bounds — the
   108	/// client-side mirror of the daemon's bounded accept. Connect is
   109	/// bounded by [`DATA_PLANE_ACCEPT_TIMEOUT`] (the audit-2 wave bounded
   110	/// every control-plane connect at the same 30 s but never reached the
   111	/// TCP data plane: a firewalled or black-holed data port — the daemon
   112	/// advertises a fresh ephemeral port per transfer, and asymmetric
   113	/// firewalls that pass the control port but block ephemerals are
   114	/// common — hung for the kernel SYN timeout, 60–127 s, with no
   115	/// message). The handshake-token write is bounded by
   116	/// [`DATA_PLANE_TOKEN_TIMEOUT`], mirroring the acceptor's bounded
   117	/// token read. Applies [`configure_data_socket`] in between.
   118	///
   119	/// On timeout the error chain carries an `io::ErrorKind::TimedOut`
   120	/// source so `remote::retry::is_retryable` classifies it as a
   121	/// transient transport failure (`--retry` re-dials instead of giving
   122	/// up on a deterministic-looking error).
   123	pub async fn dial_data_plane(
   124	    addr: &str,
   125	    handshake: &[u8],
   126	    tcp_buffer_size: Option<usize>,
   127	) -> eyre::Result<TcpStream> {
   128	    dial_data_plane_with_timeouts(
   129	        addr,
   130	        handshake,
   131	        tcp_buffer_size,
   132	        DATA_PLANE_ACCEPT_TIMEOUT,
   133	        DATA_PLANE_TOKEN_TIMEOUT,
   134	    )
   135	    .await
   136	}
   137	
   138	/// Timeout-parameterized core of [`dial_data_plane`], so tests can pin
   139	/// the bounded-failure shape without waiting out the production 30 s.
   140	async fn dial_data_plane_with_timeouts(
   141	    addr: &str,
   142	    handshake: &[u8],
   143	    tcp_buffer_size: Option<usize>,
   144	    connect_timeout: Duration,
   145	    token_timeout: Duration,
   146	) -> eyre::Result<TcpStream> {
   147	    let mut stream = match tokio::time::timeout(connect_timeout, TcpStream::connect(addr)).await {
   148	        Ok(connected) => connected.with_context(|| format!("connecting data plane {addr}"))?,
   149	        Err(_) => {
   150	            return Err(eyre::Report::new(io::Error::new(
   151	                io::ErrorKind::TimedOut,
   152	                format!("connect did not complete within {connect_timeout:?}"),
   153	            ))
   154	            .wrap_err(format!(
   155	                "data-plane connect to {addr} timed out after {connect_timeout:?} — the \
   156	                 port is likely unreachable (the daemon advertises a fresh ephemeral \
   157	                 data port per transfer; a firewall that passes the control port but \
   158	                 blocks ephemeral ports produces exactly this failure)"
   159	            )));
   160	        }
   161	    };
   162	    configure_data_socket(&stream, tcp_buffer_size).context("setting TCP_NODELAY")?;
   163	    match tokio::time::timeout(token_timeout, stream.write_all(handshake)).await {
   164	        Ok(written) => {
   165	            written.with_context(|| format!("writing data-plane handshake token to {addr}"))?
   166	        }
   167	        Err(_) => {
   168	            return Err(eyre::Report::new(io::Error::new(
   169	                io::ErrorKind::TimedOut,
   170	                format!("handshake write did not complete within {token_timeout:?}"),
   171	            ))
   172	            .wrap_err(format!(
   173	                "data-plane handshake to {addr} stalled for {token_timeout:?} — the peer \
   174	                 accepted the connection but is not reading"
   175	            )));
   176	        }
   177	    }
   178	    Ok(stream)
   179	}
   180	
   181	#[cfg(test)]
   182	mod tests {
   183	    use super::*;
   184	    use tokio::net::TcpListener;
   185	
   186	    async fn loopback_pair() -> (TcpStream, TcpStream) {
   187	        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
   188	        let addr = listener.local_addr().expect("addr");
   189	        let (client, accepted) = tokio::join!(TcpStream::connect(addr), listener.accept());
   190	        let (server, _) = accepted.expect("accept");
   191	        (client.expect("connect"), server)
   192	    }
   193	
   194	    /// The full policy lands on the socket: nodelay and keepalive read
   195	    /// back true, and both buffer directions honor (at least) the
   196	    /// requested size — kernels may round up (Linux doubles), never
   197	    /// silently ignore a size this small.
   198	    #[tokio::test]
   199	    async fn applies_nodelay_keepalive_and_buffers() {
   200	        let (client, _server) = loopback_pair().await;
   201	        let requested = 256 * 1024;
   202	        configure_data_socket(&client, Some(requested)).expect("configure");
   203	
   204	        let sock = SockRef::from(&client);
   205	        assert!(
   206	            sock.tcp_nodelay().expect("read nodelay"),
   207	            "TCP_NODELAY must be on"
   208	        );
   209	        assert!(
   210	            sock.keepalive().expect("read keepalive"),
   211	            "SO_KEEPALIVE must be on"
   212	        );
   213	        assert!(
   214	            sock.send_buffer_size().expect("read sndbuf") >= requested,
   215	            "send buffer must be at least the requested size"
   216	        );
   217	        assert!(
   218	            sock.recv_buffer_size().expect("read rcvbuf") >= requested,
   219	            "recv buffer must be at least the requested size"
   220	        );
   221	    }
   222	
   223	    /// w1-3: the keepalive is configured with explicit timing, not
   224	    /// just switched on — OS-default timing (~2 h idle) is useless on
   225	    /// transfer timescales. Read back through the kernel so the test
   226	    /// pins what a peer actually experiences, not what we asked for.
   227	    /// The socket2 getters are unix-only; Windows exercises the set
   228	    /// path through every other test in this module.
   229	    #[cfg(unix)]
   230	    #[tokio::test]
   231	    async fn keepalive_timing_is_explicit() {
   232	        let (client, _server) = loopback_pair().await;
   233	        configure_data_socket(&client, None).expect("configure");
   234	
   235	        let sock = SockRef::from(&client);
   236	        assert_eq!(
   237	            sock.tcp_keepalive_time().expect("read keepalive time"),
   238	            TCP_KEEPALIVE_IDLE,
   239	            "idle time before the first probe must be the policy value"
   240	        );
   241	        assert_eq!(
   242	            sock.tcp_keepalive_interval()
   243	                .expect("read keepalive interval"),
   244	            TCP_KEEPALIVE_INTERVAL,
   245	            "probe interval must be the policy value"
   246	        );
   247	        assert_eq!(
   248	            sock.tcp_keepalive_retries()
   249	                .expect("read keepalive retries"),
   250	            TCP_KEEPALIVE_RETRIES,
   251	            "probe retry count must be the policy value"
   252	        );
   253	    }
   254	
   255	    // ── design-3: bounded dial ────────────────────────────────────
   256	
   257	    fn chain_has_timed_out(err: &eyre::Report) -> bool {
   258	        err.chain().any(|cause| {
   259	            cause
   260	                .downcast_ref::<io::Error>()

codex
The reused sink path enforces containment at both file-stream and tar-shard write sites, and zero-byte files are counted as files written. The remaining concern is completeness: the data-plane receive path accepts any framed payload with a safe path, but I need to verify whether it still proves that the payloads are exactly the requested need list.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1480,1585p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1480	    sink: &FsTransferSink,
  1481	    shard: TarShardHeader,
  1482	) -> Result<crate::remote::transfer::SinkOutcome> {
  1483	    if shard.archive_size > MAX_TAR_SHARD_BYTES {
  1484	        return Err(violation(format!(
  1485	            "tar shard of {} bytes exceeds the {} byte cap",
  1486	            shard.archive_size, MAX_TAR_SHARD_BYTES
  1487	        )));
  1488	    }
  1489	    let mut data: Vec<u8> = Vec::new();
  1490	    data.try_reserve_exact(shard.archive_size as usize)
  1491	        .map_err(|err| eyre::eyre!("allocating {} byte tar shard: {err}", shard.archive_size))?;
  1492	    loop {
  1493	        let received = match transport.recv().await? {
  1494	            Some(f) => f,
  1495	            None => {
  1496	                return Err(eyre::Report::new(SessionFault::internal(
  1497	                    "peer closed inside tar shard record",
  1498	                )))
  1499	            }
  1500	        };
  1501	        match received.frame {
  1502	            Some(Frame::TarShardChunk(chunk)) => {
  1503	                if data.len() as u64 + chunk.content.len() as u64 > shard.archive_size {
  1504	                    return Err(violation(format!(
  1505	                        "tar shard record overran its declared {} bytes",
  1506	                        shard.archive_size
  1507	                    )));
  1508	                }
  1509	                data.extend_from_slice(&chunk.content);
  1510	            }
  1511	            Some(Frame::TarShardComplete(_)) => {
  1512	                if data.len() as u64 != shard.archive_size {
  1513	                    return Err(violation(format!(
  1514	                        "tar shard record completed at {} of {} declared bytes",
  1515	                        data.len(),
  1516	                        shard.archive_size
  1517	                    )));
  1518	                }
  1519	                return sink
  1520	                    .write_payload(PreparedPayload::TarShard {
  1521	                        headers: shard.files,
  1522	                        data,
  1523	                    })
  1524	                    .await;
  1525	            }
  1526	            other => {
  1527	                return Err(violation(format!(
  1528	                    "{} inside tar shard record",
  1529	                    frame_name(&other)
  1530	                )));
  1531	            }
  1532	        }
  1533	    }
  1534	}
  1535	
  1536	#[cfg(test)]
  1537	mod tests {
  1538	    use super::*;
  1539	
  1540	    #[test]
  1541	    fn build_id_has_version_and_git_components() {
  1542	        let id = session_build_id();
  1543	        let (version, git) = id.split_once('+').expect("build id must be version+git");
  1544	        assert_eq!(version, env!("CARGO_PKG_VERSION"));
  1545	        assert!(!git.is_empty(), "git component must be non-empty");
  1546	    }
  1547	
  1548	    #[test]
  1549	    fn fault_round_trips_the_wire_shape() {
  1550	        let fault = SessionFault {
  1551	            code: session_error::Code::BuildMismatch,
  1552	            message: "boom".into(),
  1553	            local_build_id: "1.0+aaa".into(),
  1554	            peer_build_id: "1.0+bbb".into(),
  1555	            peer_notified: false,
  1556	        };
  1557	        let wire = fault.to_wire();
  1558	        let back = SessionFault::from_wire(wire);
  1559	        assert_eq!(back.code, session_error::Code::BuildMismatch);
  1560	        assert_eq!(back.message, "boom");
  1561	        // from_wire swaps perspective: the sender's local is our peer.
  1562	        assert_eq!(back.peer_build_id, "1.0+aaa");
  1563	        assert_eq!(back.local_build_id, "1.0+bbb");
  1564	        assert!(back.peer_notified);
  1565	    }
  1566	}

exec
/usr/bin/zsh -lc 'rg -n "plan_push_payloads|TransferPayload|enum PreparedPayload|tar|TarShard" crates/blit-core/src/remote/transfer/payload.rs crates/blit-core/src/remote/transfer/diff_planner.rs crates/blit-core/src/remote/transfer/source.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/remote/transfer/source.rs:12:use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
crates/blit-core/src/remote/transfer/source.rs:28:    /// Prepares a payload for transfer (e.g. opens a file or builds a tar shard).
crates/blit-core/src/remote/transfer/source.rs:29:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload>;
crates/blit-core/src/remote/transfer/source.rs:77:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:114:/// Validate `FileHeader` sizes for a remote-source tar shard against
crates/blit-core/src/remote/transfer/source.rs:116:/// size must stay within `tar_safety::MAX_TAR_SHARD_BYTES` so a
crates/blit-core/src/remote/transfer/source.rs:118:/// unbounded allocation while building the tar.
crates/blit-core/src/remote/transfer/source.rs:123:fn validate_remote_tar_shard_sizes(headers: &[FileHeader]) -> Result<()> {
crates/blit-core/src/remote/transfer/source.rs:124:    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
crates/blit-core/src/remote/transfer/source.rs:129:                "remote-source tar entry '{}' size {} exceeds local cap {} bytes",
crates/blit-core/src/remote/transfer/source.rs:139:        .ok_or_else(|| eyre::eyre!("remote-source tar shard size sum overflows u64"))?;
crates/blit-core/src/remote/transfer/source.rs:142:            "remote-source tar shard total size {} exceeds local cap {} bytes",
crates/blit-core/src/remote/transfer/source.rs:166:    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
crates/blit-core/src/remote/transfer/source.rs:263:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:265:            TransferPayload::File(header) => Ok(PreparedPayload::File(header)),
crates/blit-core/src/remote/transfer/source.rs:266:            TransferPayload::TarShard { headers } => {
crates/blit-core/src/remote/transfer/source.rs:273:                validate_remote_tar_shard_sizes(&headers)?;
crates/blit-core/src/remote/transfer/source.rs:275:                let mut builder = tar::Builder::new(Vec::new());
crates/blit-core/src/remote/transfer/source.rs:285:                    let mut tar_header = tar::Header::new_gnu();
crates/blit-core/src/remote/transfer/source.rs:286:                    tar_header.set_path(&header.relative_path)?;
crates/blit-core/src/remote/transfer/source.rs:287:                    tar_header.set_size(header.size);
crates/blit-core/src/remote/transfer/source.rs:288:                    tar_header.set_mode(header.permissions);
crates/blit-core/src/remote/transfer/source.rs:289:                    tar_header.set_mtime(header.mtime_seconds as u64);
crates/blit-core/src/remote/transfer/source.rs:290:                    tar_header.set_cksum();
crates/blit-core/src/remote/transfer/source.rs:292:                    builder.append_data(&mut tar_header, &header.relative_path, &data[..])?;
crates/blit-core/src/remote/transfer/source.rs:295:                Ok(PreparedPayload::TarShard { headers, data })
crates/blit-core/src/remote/transfer/source.rs:298:            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
crates/blit-core/src/remote/transfer/source.rs:381:    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:497:        async fn prepare_payload(&self, _: TransferPayload) -> Result<PreparedPayload> {
crates/blit-core/src/remote/transfer/source.rs:663:mod remote_tar_size_tests {
crates/blit-core/src/remote/transfer/source.rs:665:    use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
crates/blit-core/src/remote/transfer/source.rs:680:        validate_remote_tar_shard_sizes(&headers).unwrap();
crates/blit-core/src/remote/transfer/source.rs:687:        validate_remote_tar_shard_sizes(&headers).unwrap();
crates/blit-core/src/remote/transfer/source.rs:693:        let err = validate_remote_tar_shard_sizes(&headers).unwrap_err();
crates/blit-core/src/remote/transfer/source.rs:707:        let err = validate_remote_tar_shard_sizes(&headers).unwrap_err();
crates/blit-core/src/remote/transfer/source.rs:723:        let err = validate_remote_tar_shard_sizes(&headers).unwrap_err();
crates/blit-core/src/remote/transfer/source.rs:789:        use crate::remote::transfer::tar_safety::MAX_TAR_SHARD_BYTES;
crates/blit-core/src/remote/transfer/diff_planner.rs:8://!      transfer (against the target's destination state).
crates/blit-core/src/remote/transfer/diff_planner.rs:10://!      `File` payloads, batched `TarShard`, or — once step 4 lands —
crates/blit-core/src/remote/transfer/diff_planner.rs:30:use crate::remote::transfer::payload::{plan_transfer_payloads, TransferPayload};
crates/blit-core/src/remote/transfer/diff_planner.rs:46:pub fn plan_push_payloads(
crates/blit-core/src/remote/transfer/diff_planner.rs:50:) -> Result<Vec<TransferPayload>> {
crates/blit-core/src/remote/transfer/diff_planner.rs:54:/// Input bundle for the local-mirror diff stage. Origin and target
crates/blit-core/src/remote/transfer/diff_planner.rs:62:    /// joined under this to compare against existing target state.
crates/blit-core/src/remote/transfer/diff_planner.rs:64:    /// How to decide whether a target-existing file matches.
crates/blit-core/src/remote/transfer/diff_planner.rs:70:    /// Knobs for the tar / large / raw planner (unchanged from the
crates/blit-core/src/remote/transfer/diff_planner.rs:89:) -> Result<Vec<TransferPayload>> {
crates/blit-core/src/remote/transfer/diff_planner.rs:380:    fn plan_local_mirror_batches_many_small_files_into_tar_shard() {
crates/blit-core/src/remote/transfer/diff_planner.rs:381:        // R2-F4 tar-shard batching boundary: 50 tiny files in the
crates/blit-core/src/remote/transfer/diff_planner.rs:382:        // small bucket (<64KiB) should produce at least one TarShard
crates/blit-core/src/remote/transfer/diff_planner.rs:383:        // payload from the planner. We only assert that *some* tar
crates/blit-core/src/remote/transfer/diff_planner.rs:409:        let tar_shards = planned
crates/blit-core/src/remote/transfer/diff_planner.rs:411:            .filter(|p| matches!(p, TransferPayload::TarShard { .. }))
crates/blit-core/src/remote/transfer/diff_planner.rs:414:            tar_shards >= 1,
crates/blit-core/src/remote/transfer/diff_planner.rs:415:            "expected at least one TarShard payload for 50 small files, got {} payloads: {:?}",
crates/blit-core/src/remote/transfer/diff_planner.rs:422:    fn plan_local_mirror_force_tar_groups_even_a_few_files() {
crates/blit-core/src/remote/transfer/diff_planner.rs:423:        // PlanOptions::force_tar=true should always produce tar shards
crates/blit-core/src/remote/transfer/diff_planner.rs:438:            force_tar: true,
crates/blit-core/src/remote/transfer/diff_planner.rs:453:        let has_tar = planned
crates/blit-core/src/remote/transfer/diff_planner.rs:455:            .any(|p| matches!(p, TransferPayload::TarShard { .. }));
crates/blit-core/src/remote/transfer/diff_planner.rs:456:        assert!(has_tar, "force_tar must produce a TarShard payload");
crates/blit-core/src/remote/transfer/payload.rs:13:    ClientPushRequest, FileData, FileHeader, TarShardChunk, TarShardComplete, TarShardHeader,
crates/blit-core/src/remote/transfer/payload.rs:17:use tar::{Builder, EntryType, Header};
crates/blit-core/src/remote/transfer/payload.rs:25:pub enum TransferPayload {
crates/blit-core/src/remote/transfer/payload.rs:27:    TarShard {
crates/blit-core/src/remote/transfer/payload.rs:44:    payload: TransferPayload,
crates/blit-core/src/remote/transfer/payload.rs:48:        TransferPayload::File(header) => Ok(PreparedPayload::File(header)),
crates/blit-core/src/remote/transfer/payload.rs:49:        TransferPayload::TarShard { headers } => {
crates/blit-core/src/remote/transfer/payload.rs:53:                task::spawn_blocking(move || build_tar_shard(&source_root_clone, &headers_clone))
crates/blit-core/src/remote/transfer/payload.rs:55:                    .map_err(|err| eyre!("tar shard worker failed: {err}"))??;
crates/blit-core/src/remote/transfer/payload.rs:56:            Ok(PreparedPayload::TarShard { headers, data })
crates/blit-core/src/remote/transfer/payload.rs:61:        TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
crates/blit-core/src/remote/transfer/payload.rs:69:/// `File` and `TarShard` are used by both outbound and inbound paths
crates/blit-core/src/remote/transfer/payload.rs:78:pub enum PreparedPayload {
crates/blit-core/src/remote/transfer/payload.rs:82:    /// In-memory tar shard. Already buffered (bounded by the planner's
crates/blit-core/src/remote/transfer/payload.rs:84:    TarShard {
crates/blit-core/src/remote/transfer/payload.rs:114:) -> Result<Vec<TransferPayload>> {
crates/blit-core/src/remote/transfer/payload.rs:136:    let mut payloads: Vec<TransferPayload> = Vec::new();
crates/blit-core/src/remote/transfer/payload.rs:140:            TransferTask::TarShard(paths) => {
crates/blit-core/src/remote/transfer/payload.rs:149:                    payloads.push(TransferPayload::TarShard {
crates/blit-core/src/remote/transfer/payload.rs:158:                        payloads.push(TransferPayload::File(header));
crates/blit-core/src/remote/transfer/payload.rs:165:                    payloads.push(TransferPayload::File(header));
crates/blit-core/src/remote/transfer/payload.rs:172:        payloads.push(TransferPayload::File(header));
crates/blit-core/src/remote/transfer/payload.rs:175:    // Sort payloads: tar shards first (small, distribute well across streams),
crates/blit-core/src/remote/transfer/payload.rs:181:        TransferPayload::TarShard { .. } => (0, 0),
crates/blit-core/src/remote/transfer/payload.rs:182:        TransferPayload::File(h) => (1, h.size),
crates/blit-core/src/remote/transfer/payload.rs:183:        TransferPayload::FileBlock { size, .. } => (2, *size),
crates/blit-core/src/remote/transfer/payload.rs:184:        TransferPayload::FileBlockComplete { .. } => (3, 0),
crates/blit-core/src/remote/transfer/payload.rs:190:pub fn payload_file_count(payloads: &[TransferPayload]) -> usize {
crates/blit-core/src/remote/transfer/payload.rs:194:            TransferPayload::File(_) => 1,
crates/blit-core/src/remote/transfer/payload.rs:195:            TransferPayload::TarShard { headers } => headers.len(),
crates/blit-core/src/remote/transfer/payload.rs:198:            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => 0,
crates/blit-core/src/remote/transfer/payload.rs:211:    payloads: Vec<TransferPayload>,
crates/blit-core/src/remote/transfer/payload.rs:225:    payloads: Vec<TransferPayload>,
crates/blit-core/src/remote/transfer/payload.rs:234:    // function emits FileData / TarShardChunk over the same gRPC
crates/blit-core/src/remote/transfer/payload.rs:294:            PreparedPayload::TarShard { headers, data } => {
crates/blit-core/src/remote/transfer/payload.rs:297:                    ClientPayload::TarShardHeader(TarShardHeader {
crates/blit-core/src/remote/transfer/payload.rs:307:                        ClientPayload::TarShardChunk(TarShardChunk {
crates/blit-core/src/remote/transfer/payload.rs:317:                send_payload(tx, ClientPayload::TarShardComplete(TarShardComplete {})).await?;
crates/blit-core/src/remote/transfer/payload.rs:346:pub fn build_tar_shard(source_root: &Path, headers: &[FileHeader]) -> Result<Vec<u8>> {
crates/blit-core/src/remote/transfer/payload.rs:362:        let mut tar_header = Header::new_gnu();
crates/blit-core/src/remote/transfer/payload.rs:363:        tar_header.set_entry_type(EntryType::Regular);
crates/blit-core/src/remote/transfer/payload.rs:369:        tar_header.set_mode(mode);
crates/blit-core/src/remote/transfer/payload.rs:370:        tar_header.set_size(header.size);
crates/blit-core/src/remote/transfer/payload.rs:376:        tar_header.set_mtime(mtime);
crates/blit-core/src/remote/transfer/payload.rs:377:        tar_header.set_cksum();
crates/blit-core/src/remote/transfer/payload.rs:380:            .append_data(&mut tar_header, rel, &mut file)
crates/blit-core/src/remote/transfer/payload.rs:381:            .with_context(|| format!("adding {} to tar shard", full_path.display()))?;
crates/blit-core/src/remote/transfer/payload.rs:384:    builder.into_inner().context("finalizing tar shard")

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/diff_planner.rs | sed -n '1,300p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Unified diff + payload planning stage.
     2	//!
     3	//! Sits between `TransferSource::scan` (which emits headers from the
     4	//! origin's filesystem) and `execute_sink_pipeline_streaming` (which
     5	//! dispatches payloads to one or more sinks). Decides:
     6	//!
     7	//!   1. Which source headers represent files that genuinely need to
     8	//!      transfer (against the target's destination state).
     9	//!   2. What payload shapes the surviving files become (whole-file
    10	//!      `File` payloads, batched `TarShard`, or — once step 4 lands —
    11	//!      block-level resume `FileBlock` + `FileBlockComplete` pairs).
    12	//!
    13	//! Step 3a of `docs/plan/PIPELINE_UNIFICATION.md`. Today this module
    14	//! consolidates the local-mirror path that lived in `orchestrator.rs`
    15	//! (`filter_headers_for_copy` + the call to `plan_transfer_payloads`).
    16	//! Push and pull will adopt the same module in 3b and step 4.
    17	//!
    18	//! `ComparisonMode` in `proto/blit.proto` is the canonical input shape.
    19	//! As of R2-F1 (`docs/reviews/followup_review_2026-05-02.md`) we honor
    20	//! every variant with concrete semantics — no silent fall-through to
    21	//! size+mtime. This means callers passing `SizeOnly`, `IgnoreTimes`,
    22	//! or `Force` get the behavior the wire enum
    23	//! advertises, not whatever the historical default happened to do.
    24	
    25	use std::path::Path;
    26	
    27	use eyre::{Context, Result};
    28	
    29	use crate::generated::{ComparisonMode, FileHeader};
    30	use crate::remote::transfer::payload::{plan_transfer_payloads, TransferPayload};
    31	use crate::transfer_plan::PlanOptions;
    32	
    33	/// Push origins outsource the diff to the daemon: the client sends its
    34	/// source manifest, daemon returns a NeedList, client filters to the
    35	/// intersection. By the time we plan payloads, the headers are already
    36	/// filtered. This re-exports the existing payload planner under the
    37	/// diff_planner module so the push-client call site goes through the
    38	/// unified module — there's no separate comparison stage to consolidate
    39	/// (the comparison happens on the daemon, not the client).
    40	///
    41	/// When step 4 lands and the daemon-side diff moves into this module
    42	/// for the pull case, push could in principle use the same daemon-side
    43	/// helper instead of the round-trip-via-NeedList protocol. That would
    44	/// be a deeper protocol change tracked under remote→remote re-evaluation
    45	/// (step 5 of `docs/plan/PIPELINE_UNIFICATION.md`).
    46	pub fn plan_push_payloads(
    47	    headers: Vec<FileHeader>,
    48	    source_root: &Path,
    49	    plan_options: PlanOptions,
    50	) -> Result<Vec<TransferPayload>> {
    51	    plan_transfer_payloads(headers, source_root, plan_options).context("planning push payloads")
    52	}
    53	
    54	/// Input bundle for the local-mirror diff stage. Origin and target
    55	/// are co-located (both on the same filesystem), so the comparison
    56	/// can stat the destination directly without a wire roundtrip.
    57	pub struct LocalDiffInputs<'a> {
    58	    /// Source-rooted absolute path. Headers' `relative_path` is
    59	    /// joined under this to find the source bytes.
    60	    pub src_root: &'a Path,
    61	    /// Destination-rooted absolute path. Headers' `relative_path` is
    62	    /// joined under this to compare against existing target state.
    63	    pub dst_root: &'a Path,
    64	    /// How to decide whether a target-existing file matches.
    65	    pub compare_mode: ComparisonMode,
    66	    /// When true, skip any file the destination already has,
    67	    /// regardless of `compare_mode`. Orthogonal axis; matches the
    68	    /// `ignore_existing` field on `TransferOperationSpec`.
    69	    pub ignore_existing: bool,
    70	    /// Knobs for the tar / large / raw planner (unchanged from the
    71	    /// pre-extraction call site).
    72	    pub plan_options: PlanOptions,
    73	    /// When false, every source header passes the comparison stage —
    74	    /// equivalent to `--ignore-times`/`--force` in user-facing terms.
    75	    /// Used by the orchestrator when its `skip_unchanged` flag is off.
    76	    pub skip_unchanged: bool,
    77	}
    78	
    79	/// Filter source headers down to those that need transferring against
    80	/// a local destination, then plan the surviving headers into payloads.
    81	///
    82	/// This is the single entry point the local-mirror path uses. Future
    83	/// origin paths (push client, pull daemon) will gain their own entry
    84	/// points on this module — same diff + planning algorithm, different
    85	/// "where the destination lives" assumption.
    86	pub fn plan_local_mirror(
    87	    source_headers: Vec<FileHeader>,
    88	    inputs: LocalDiffInputs<'_>,
    89	) -> Result<Vec<TransferPayload>> {
    90	    let headers_to_copy = if inputs.skip_unchanged {
    91	        filter_unchanged(
    92	            &source_headers,
    93	            inputs.src_root,
    94	            inputs.dst_root,
    95	            inputs.compare_mode,
    96	            inputs.ignore_existing,
    97	        )
    98	    } else {
    99	        source_headers
   100	    };
   101	
   102	    plan_transfer_payloads(headers_to_copy, inputs.src_root, inputs.plan_options)
   103	        .context("planning payloads after diff stage")
   104	}
   105	
   106	/// Drop headers whose destination file already matches the source
   107	/// under the chosen comparison mode. Keeps headers that need transfer.
   108	///
   109	/// `ignore_existing` is the orthogonal "skip if dst exists" axis from
   110	/// `TransferOperationSpec`: when true, present destination files are
   111	/// dropped before `compare_mode` is consulted at all.
   112	///
   113	/// This is the local-mirror flavor: it stats the destination directly.
   114	/// Remote-source variants (where the destination manifest arrives over
   115	/// the wire) live in their own helpers — TBD step 4.
   116	///
   117	/// Every `ComparisonMode` variant is implemented (R2-F1). `Unspecified`
   118	/// behaves as `SizeMtime` (the historical default) — callers should fold
   119	/// `Unspecified` away via `NormalizedTransferOperation::from_spec`
   120	/// before reaching this function, but we accept it defensively.
   121	pub fn filter_unchanged(
   122	    headers: &[FileHeader],
   123	    src_root: &Path,
   124	    dst_root: &Path,
   125	    compare_mode: ComparisonMode,
   126	    ignore_existing: bool,
   127	) -> Vec<FileHeader> {
   128	    headers
   129	        .iter()
   130	        .filter(|h| {
   131	            let src = src_root.join(&h.relative_path);
   132	            let dst = dst_root.join(&h.relative_path);
   133	            if ignore_existing && dst.exists() {
   134	                return false;
   135	            }
   136	            local_needs_copy(&src, &dst, compare_mode).unwrap_or(true)
   137	        })
   138	        .cloned()
   139	        .collect()
   140	}
   141	
   142	/// Per-mode comparison predicate. Delegates to the centralized helper
   143	/// in `copy::compare` so the diff planner, the single-file copy path,
   144	/// and the sink all share one decision tree.
   145	fn local_needs_copy(src: &Path, dst: &Path, mode: ComparisonMode) -> Result<bool> {
   146	    crate::copy::file_needs_copy_with_mode(src, dst, mode)
   147	}
   148	
   149	#[cfg(test)]
   150	mod tests {
   151	    use super::*;
   152	
   153	    /// Build src+dst trees with the given (relative_path, content)
   154	    /// pairs on each side. Returns (src_root, dst_root, _tempdir).
   155	    fn make_trees(
   156	        src_files: &[(&str, &[u8])],
   157	        dst_files: &[(&str, &[u8])],
   158	    ) -> (std::path::PathBuf, std::path::PathBuf, tempfile::TempDir) {
   159	        let tmp = tempfile::tempdir().unwrap();
   160	        let src = tmp.path().join("src");
   161	        let dst = tmp.path().join("dst");
   162	        std::fs::create_dir_all(&src).unwrap();
   163	        std::fs::create_dir_all(&dst).unwrap();
   164	        for (path, content) in src_files {
   165	            let full = src.join(path);
   166	            if let Some(parent) = full.parent() {
   167	                std::fs::create_dir_all(parent).unwrap();
   168	            }
   169	            std::fs::write(full, content).unwrap();
   170	        }
   171	        for (path, content) in dst_files {
   172	            let full = dst.join(path);
   173	            if let Some(parent) = full.parent() {
   174	                std::fs::create_dir_all(parent).unwrap();
   175	            }
   176	            std::fs::write(full, content).unwrap();
   177	        }
   178	        (src, dst, tmp)
   179	    }
   180	
   181	    fn header(rel: &str, size: u64) -> FileHeader {
   182	        FileHeader {
   183	            relative_path: rel.into(),
   184	            size,
   185	            mtime_seconds: 0,
   186	            permissions: 0,
   187	            checksum: vec![],
   188	        }
   189	    }
   190	
   191	    fn sync_mtimes(src_root: &Path, dst_root: &Path, rel: &str) {
   192	        let src_mtime = std::fs::metadata(src_root.join(rel))
   193	            .unwrap()
   194	            .modified()
   195	            .unwrap();
   196	        let _ = filetime::set_file_mtime(
   197	            dst_root.join(rel),
   198	            filetime::FileTime::from_system_time(src_mtime),
   199	        );
   200	    }
   201	
   202	    fn kept_paths(kept: &[FileHeader]) -> Vec<String> {
   203	        let mut v: Vec<String> = kept.iter().map(|h| h.relative_path.clone()).collect();
   204	        v.sort();
   205	        v
   206	    }
   207	
   208	    #[test]
   209	    fn size_mtime_drops_matching_files() {
   210	        let (src, dst, _tmp) = make_trees(
   211	            &[("same.txt", b"matching content"), ("diff.txt", b"new")],
   212	            &[
   213	                ("same.txt", b"matching content"),
   214	                ("diff.txt", b"old content"),
   215	            ],
   216	        );
   217	        sync_mtimes(&src, &dst, "same.txt");
   218	
   219	        let headers = vec![header("same.txt", 16), header("diff.txt", 3)];
   220	        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeMtime, false);
   221	        assert_eq!(kept_paths(&kept), vec!["diff.txt"]);
   222	    }
   223	
   224	    #[test]
   225	    fn size_mtime_keeps_missing_dest() {
   226	        let (src, dst, _tmp) = make_trees(&[("only.txt", b"hi")], &[]);
   227	        let headers = vec![header("only.txt", 2)];
   228	        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeMtime, false);
   229	        assert_eq!(kept.len(), 1);
   230	    }
   231	
   232	    #[test]
   233	    fn size_only_ignores_mtime_when_sizes_match() {
   234	        let (src, dst, _tmp) = make_trees(&[("same.txt", b"abcdef")], &[("same.txt", b"abcdef")]);
   235	        // Don't sync mtimes — they'll differ. SizeOnly should still drop
   236	        // the entry because content sizes match.
   237	        let headers = vec![header("same.txt", 6)];
   238	        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeOnly, false);
   239	        assert!(
   240	            kept.is_empty(),
   241	            "SizeOnly must skip files with matching size regardless of mtime"
   242	        );
   243	    }
   244	
   245	    #[test]
   246	    fn size_only_keeps_size_mismatch() {
   247	        let (src, dst, _tmp) = make_trees(&[("file.txt", b"longer")], &[("file.txt", b"short")]);
   248	        let headers = vec![header("file.txt", 6)];
   249	        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeOnly, false);
   250	        assert_eq!(kept.len(), 1);
   251	    }
   252	
   253	    #[test]
   254	    fn ignore_times_always_copies() {
   255	        let (src, dst, _tmp) = make_trees(
   256	            &[("a.txt", b"x"), ("b.txt", b"y")],
   257	            &[("a.txt", b"x"), ("b.txt", b"y")],
   258	        );
   259	        sync_mtimes(&src, &dst, "a.txt");
   260	        sync_mtimes(&src, &dst, "b.txt");
   261	        let headers = vec![header("a.txt", 1), header("b.txt", 1)];
   262	        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::IgnoreTimes, false);
   263	        assert_eq!(kept.len(), 2, "IgnoreTimes must always copy");
   264	    }
   265	
   266	    #[test]
   267	    fn force_always_copies() {
   268	        let (src, dst, _tmp) = make_trees(&[("a.txt", b"x")], &[("a.txt", b"x")]);
   269	        sync_mtimes(&src, &dst, "a.txt");
   270	        let headers = vec![header("a.txt", 1)];
   271	        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::Force, false);
   272	        assert_eq!(kept.len(), 1);
   273	    }
   274	
   275	    #[test]
   276	    fn ignore_existing_skips_existing_regardless_of_mode() {
   277	        // ignore_existing is orthogonal to compare_mode: even Force,
   278	        // which would otherwise always copy, must respect it.
   279	        let (src, dst, _tmp) = make_trees(
   280	            &[("a.txt", b"new"), ("b.txt", b"only-on-src")],
   281	            &[("a.txt", b"old")],
   282	        );
   283	        let headers = vec![header("a.txt", 3), header("b.txt", 11)];
   284	        // Use SizeMtime as the mode (Force+ignore_existing is rejected
   285	        // at the spec normalizer); we still expect a.txt to be skipped.
   286	        let kept = filter_unchanged(&headers, &src, &dst, ComparisonMode::SizeMtime, true);
   287	        assert_eq!(
   288	            kept_paths(&kept),
   289	            vec!["b.txt"],
   290	            "ignore_existing keeps only files missing on dest"
   291	        );
   292	    }
   293	
   294	    #[test]
   295	    fn checksum_drops_byte_identical_files_with_diff_mtime() {
   296	        let (src, dst, _tmp) = make_trees(
   297	            &[("same.txt", b"identical bytes")],
   298	            &[("same.txt", b"identical bytes")],
   299	        );
   300	        // Don't sync mtimes — Checksum mode shouldn't care about mtime.

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/payload.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::collections::HashMap;
     2	use std::path::{Path, PathBuf};
     3	
     4	use eyre::{bail, eyre, Context, Result};
     5	use futures::{stream, StreamExt};
     6	use tokio::io::AsyncReadExt;
     7	use tokio::sync::mpsc;
     8	use tokio::task;
     9	
    10	use crate::fs_enum::FileEntry;
    11	use crate::generated::client_push_request::Payload as ClientPayload;
    12	use crate::generated::{
    13	    ClientPushRequest, FileData, FileHeader, TarShardChunk, TarShardComplete, TarShardHeader,
    14	    UploadComplete,
    15	};
    16	use crate::transfer_plan::{self, PlanOptions, TransferTask};
    17	use tar::{Builder, EntryType, Header};
    18	
    19	use super::data_plane::CONTROL_PLANE_CHUNK_SIZE;
    20	use super::progress::RemoteTransferProgress;
    21	use crate::remote::transfer::source::TransferSource;
    22	use std::sync::Arc;
    23	
    24	#[derive(Debug, Clone)]
    25	pub enum TransferPayload {
    26	    File(FileHeader),
    27	    TarShard {
    28	        headers: Vec<FileHeader>,
    29	    },
    30	    /// Resume protocol: overwrite a block of an existing file.
    31	    FileBlock {
    32	        relative_path: String,
    33	        offset: u64,
    34	        size: u64,
    35	    },
    36	    /// Resume protocol: finalize a resumed file (truncate to total_size).
    37	    FileBlockComplete {
    38	        relative_path: String,
    39	        total_size: u64,
    40	    },
    41	}
    42	
    43	pub async fn prepare_payload(
    44	    payload: TransferPayload,
    45	    source_root: PathBuf,
    46	) -> Result<PreparedPayload> {
    47	    match payload {
    48	        TransferPayload::File(header) => Ok(PreparedPayload::File(header)),
    49	        TransferPayload::TarShard { headers } => {
    50	            let headers_clone = headers.clone();
    51	            let source_root_clone = source_root.clone();
    52	            let data =
    53	                task::spawn_blocking(move || build_tar_shard(&source_root_clone, &headers_clone))
    54	                    .await
    55	                    .map_err(|err| eyre!("tar shard worker failed: {err}"))??;
    56	            Ok(PreparedPayload::TarShard { headers, data })
    57	        }
    58	        // Resume payloads can only originate on the receive side (parsed
    59	        // off the wire by DataPlaneSource); the file-system source never
    60	        // produces them.
    61	        TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => {
    62	            bail!("FileBlock payloads cannot be prepared from a filesystem source")
    63	        }
    64	    }
    65	}
    66	
    67	/// A payload ready for a sink to consume.
    68	///
    69	/// `File` and `TarShard` are used by both outbound and inbound paths
    70	/// (they carry self-contained data). The receive pipeline additionally
    71	/// uses `FileBlock` / `FileBlockComplete` for the resume protocol.
    72	///
    73	/// Streaming file bytes (4 GiB pulls, no point buffering) are NOT a
    74	/// payload variant — they go through `TransferSink::write_file_stream`
    75	/// directly so the receiver can hand the sink a borrowed reader without
    76	/// fighting `'static` trait-object lifetimes.
    77	#[derive(Debug)]
    78	pub enum PreparedPayload {
    79	    /// Whole file, source has it accessible by `src_root.join(relative_path)`.
    80	    /// The sink performs a (zero-copy when possible) local copy.
    81	    File(FileHeader),
    82	    /// In-memory tar shard. Already buffered (bounded by the planner's
    83	    /// shard threshold).
    84	    TarShard {
    85	        headers: Vec<FileHeader>,
    86	        data: Vec<u8>,
    87	    },
    88	    /// Resume: write `bytes` at `offset` into the existing file at
    89	    /// `dst_root.join(relative_path)`.
    90	    FileBlock {
    91	        relative_path: String,
    92	        offset: u64,
    93	        bytes: Vec<u8>,
    94	    },
    95	    /// Resume: finalize the file at `dst_root.join(relative_path)` by
    96	    /// truncating to `total_size` and stamping mtime + perms.
    97	    /// Metadata is carried inline so a "mtime touched, content
    98	    /// identical" mirror correctly updates the destination's mtime
    99	    /// even when zero blocks needed to be transferred.
   100	    FileBlockComplete {
   101	        relative_path: String,
   102	        total_size: u64,
   103	        mtime_seconds: i64,
   104	        permissions: u32,
   105	    },
   106	}
   107	
   108	pub const DEFAULT_PAYLOAD_PREFETCH: usize = 8;
   109	
   110	pub fn plan_transfer_payloads(
   111	    headers: Vec<FileHeader>,
   112	    source_root: &Path,
   113	    options: PlanOptions,
   114	) -> Result<Vec<TransferPayload>> {
   115	    if headers.is_empty() {
   116	        return Ok(Vec::new());
   117	    }
   118	
   119	    let mut entries: Vec<FileEntry> = Vec::with_capacity(headers.len());
   120	    for header in &headers {
   121	        let rel_path = Path::new(&header.relative_path);
   122	        let absolute = source_root.join(rel_path);
   123	        entries.push(FileEntry {
   124	            path: absolute,
   125	            size: header.size,
   126	            is_directory: false,
   127	        });
   128	    }
   129	
   130	    let mut header_map: HashMap<String, FileHeader> = headers
   131	        .into_iter()
   132	        .map(|header| (header.relative_path.clone(), header))
   133	        .collect();
   134	
   135	    let tasks = transfer_plan::build_plan(&entries, source_root, options);
   136	    let mut payloads: Vec<TransferPayload> = Vec::new();
   137	
   138	    for task in tasks {
   139	        match task {
   140	            TransferTask::TarShard(paths) => {
   141	                let mut shard_headers: Vec<FileHeader> = Vec::with_capacity(paths.len());
   142	                for path in paths {
   143	                    let rel = normalize_relative_path(&path);
   144	                    if let Some(header) = header_map.remove(&rel) {
   145	                        shard_headers.push(header);
   146	                    }
   147	                }
   148	                if !shard_headers.is_empty() {
   149	                    payloads.push(TransferPayload::TarShard {
   150	                        headers: shard_headers,
   151	                    });
   152	                }
   153	            }
   154	            TransferTask::RawBundle(paths) => {
   155	                for path in paths {
   156	                    let rel = normalize_relative_path(&path);
   157	                    if let Some(header) = header_map.remove(&rel) {
   158	                        payloads.push(TransferPayload::File(header));
   159	                    }
   160	                }
   161	            }
   162	            TransferTask::Large { path } => {
   163	                let rel = normalize_relative_path(&path);
   164	                if let Some(header) = header_map.remove(&rel) {
   165	                    payloads.push(TransferPayload::File(header));
   166	                }
   167	            }
   168	        }
   169	    }
   170	
   171	    for (_, header) in header_map.into_iter() {
   172	        payloads.push(TransferPayload::File(header));
   173	    }
   174	
   175	    // Sort payloads: tar shards first (small, distribute well across streams),
   176	    // then files ascending by size. This ensures all streams stay busy with
   177	    // small work before a single large file monopolizes one stream's tail.
   178	    // Resume variants (FileBlock / FileBlockComplete) are receive-only and
   179	    // never appear here — plan_transfer_payloads is the outbound planner.
   180	    payloads.sort_by_key(|p| match p {
   181	        TransferPayload::TarShard { .. } => (0, 0),
   182	        TransferPayload::File(h) => (1, h.size),
   183	        TransferPayload::FileBlock { size, .. } => (2, *size),
   184	        TransferPayload::FileBlockComplete { .. } => (3, 0),
   185	    });
   186	
   187	    Ok(payloads)
   188	}
   189	
   190	pub fn payload_file_count(payloads: &[TransferPayload]) -> usize {
   191	    payloads
   192	        .iter()
   193	        .map(|payload| match payload {
   194	            TransferPayload::File(_) => 1,
   195	            TransferPayload::TarShard { headers } => headers.len(),
   196	            // Resume payloads patch existing files in-place — they
   197	            // don't add to the "files transferred" count.
   198	            TransferPayload::FileBlock { .. } | TransferPayload::FileBlockComplete { .. } => 0,
   199	        })
   200	        .sum()
   201	}
   202	
   203	fn normalize_relative_path(path: &Path) -> String {
   204	    // Canonical POSIX form — see `crate::path_posix` for why a
   205	    // component-walk is correct on every platform and the historical
   206	    // string `replace('\\', "/")` was destructive on POSIX.
   207	    crate::path_posix::relative_path_to_posix(path)
   208	}
   209	
   210	pub fn prepared_payload_stream(
   211	    payloads: Vec<TransferPayload>,
   212	    source: Arc<dyn TransferSource>,
   213	    prefetch: usize,
   214	) -> impl futures::Stream<Item = Result<PreparedPayload>> {
   215	    let capacity = prefetch.max(1);
   216	    stream::iter(payloads.into_iter().map(move |payload| {
   217	        let source = source.clone();
   218	        async move { source.prepare_payload(payload).await }
   219	    }))
   220	    .buffered(capacity)
   221	}
   222	
   223	pub async fn transfer_payloads_via_control_plane(
   224	    source: Arc<dyn TransferSource>,
   225	    payloads: Vec<TransferPayload>,
   226	    tx: &mpsc::Sender<ClientPushRequest>,
   227	    finish: bool,
   228	    progress: Option<&RemoteTransferProgress>,
   229	    chunk_bytes: usize,
   230	    payload_prefetch: usize,
   231	) -> Result<()> {
   232	    // audit-h3c slice 1: clamp at the gRPC fallback ceiling for the
   233	    // same reason GrpcFallbackSink / GrpcServerStreamingSink do — this
   234	    // function emits FileData / TarShardChunk over the same gRPC
   235	    // control plane and must produce frames at observable cadence.
   236	    // No live caller today (grep returns zero matches), but the
   237	    // function is `pub` and re-exported, so any future caller would
   238	    // silently bypass the cap without this line.
   239	    let chunk_size =
   240	        super::grpc_fallback::clamp_fallback_chunk_size(chunk_bytes.max(CONTROL_PLANE_CHUNK_SIZE));
   241	    let mut buffer = vec![0u8; chunk_size];
   242	    let mut prepared_stream = prepared_payload_stream(payloads, source.clone(), payload_prefetch);
   243	
   244	    while let Some(prepared) = prepared_stream.next().await {
   245	        match prepared? {
   246	            PreparedPayload::File(header) => {
   247	                send_payload(tx, ClientPayload::FileManifest(header.clone())).await?;
   248	
   249	                if header.size == 0 {
   250	                    if let Some(progress) = progress {
   251	                        progress.report_file_complete(header.relative_path.clone());
   252	                    }
   253	                    continue;
   254	                }
   255	
   256	                let mut file = source
   257	                    .open_file(&header)
   258	                    .await
   259	                    .with_context(|| format!("opening {}", header.relative_path))?;
   260	

exec
/usr/bin/zsh -lc 'rg -n "force-grpc|force_grpc|grpc-shaped|in_stream|PushSessionOptions|run_push_session" crates docs | head -200' in /home/michael/dev/Blit
 succeeded in 0ms:
docs/TRANSFER_SESSION.md:185:- **In-stream carrier:** requested via `SessionOpen.in_stream_bytes`
docs/TRANSFER_SESSION.md:186:  (operator `--force-grpc` diagnostics) or granted by the responder
crates/blit-tui/src/exec_plan.rs:71:        force_grpc: false,
crates/blit-app/src/transfers/remote.rs:423:    pub force_grpc: bool,
crates/blit-app/src/transfers/remote.rs:498:            execution.force_grpc,
crates/blit-daemon/src/main.rs:108:        args.force_grpc_data,
crates/blit-daemon/src/runtime.rs:100:    pub(crate) force_grpc_data: bool,
crates/blit-daemon/src/runtime.rs:378:            force_grpc_data: false,
crates/blit-daemon/src/runtime.rs:464:            force_grpc_data: false,
crates/blit-daemon/src/runtime.rs:495:            force_grpc_data: false,
crates/blit-daemon/src/service/transfer_session_e2e.rs:32:use blit_core::remote::transfer::session_client::{run_push_session, PushSessionOptions};
crates/blit-daemon/src/service/transfer_session_e2e.rs:208:    let summary = run_push_session(&daemon.endpoint, source, PushSessionOptions::default())
crates/blit-daemon/src/service/transfer_session_e2e.rs:221:        !summary.in_stream_carrier_used,
crates/blit-daemon/src/service/transfer_session_e2e.rs:229:async fn session_lands_bytes_over_in_stream_carrier() {
crates/blit-daemon/src/service/transfer_session_e2e.rs:238:    let summary = run_push_session(
crates/blit-daemon/src/service/transfer_session_e2e.rs:241:        PushSessionOptions {
crates/blit-daemon/src/service/transfer_session_e2e.rs:242:            in_stream_bytes: true,
crates/blit-daemon/src/service/transfer_session_e2e.rs:243:            ..PushSessionOptions::default()
crates/blit-daemon/src/service/transfer_session_e2e.rs:251:        summary.in_stream_carrier_used,
crates/blit-daemon/src/service/transfer_session_e2e.rs:252:        "an in_stream_bytes request rides the in-stream carrier"
crates/blit-daemon/src/service/transfer_session_e2e.rs:284:    let summary = run_push_session(
crates/blit-daemon/src/service/transfer_session_e2e.rs:287:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:298:    // tcp_fallback_used/bytes_zero_copy vs in_stream_carrier_used — have
crates/blit-daemon/src/service/transfer_session_e2e.rs:314:    let err = run_push_session(
crates/blit-daemon/src/service/transfer_session_e2e.rs:317:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:335:    let err = run_push_session(
crates/blit-daemon/src/service/transfer_session_e2e.rs:338:        PushSessionOptions::default(),
crates/blit-daemon/src/service/transfer_session_e2e.rs:375:    let summary = run_push_session(
crates/blit-daemon/src/service/transfer_session_e2e.rs:378:        PushSessionOptions::default(),
crates/blit-daemon/src/service/core.rs:66:    force_grpc_data: bool,
crates/blit-daemon/src/service/core.rs:97:        force_grpc_data: bool,
crates/blit-daemon/src/service/core.rs:106:            force_grpc_data,
crates/blit-daemon/src/service/core.rs:120:        force_grpc_data: bool,
crates/blit-daemon/src/service/core.rs:125:            force_grpc_data,
crates/blit-daemon/src/service/core.rs:526:        let force_grpc_data = self.force_grpc_data;
crates/blit-daemon/src/service/core.rs:580:                    force_grpc_data,
crates/blit-daemon/src/service/core.rs:626:        let force_grpc_data = self.force_grpc_data;
crates/blit-daemon/src/service/core.rs:663:                    force_grpc_data,
crates/blit-daemon/src/service/pull_sync.rs:51:    force_grpc_override: bool,
crates/blit-daemon/src/service/pull_sync.rs:79:    let force_grpc = spec.force_grpc || force_grpc_override;
crates/blit-daemon/src/service/pull_sync.rs:301:    if force_grpc {
crates/blit-daemon/src/service/delegated_pull.rs:654:            force_grpc: false,
crates/blit-daemon/src/service/delegated_pull.rs:747:            force_grpc: true,
crates/blit-daemon/src/service/delegated_pull.rs:759:        let snapshot_force_grpc = spec_in.force_grpc;
crates/blit-daemon/src/service/delegated_pull.rs:769:        assert_eq!(spec_in.force_grpc, snapshot_force_grpc);
crates/blit-daemon/src/service/push/control.rs:66:    force_grpc_data: bool,
crates/blit-daemon/src/service/push/control.rs:74:    let mut force_grpc_client = false;
crates/blit-daemon/src/service/push/control.rs:95:    let mut force_grpc_effective = force_grpc_data;
crates/blit-daemon/src/service/push/control.rs:128:                force_grpc_client = header.force_grpc;
crates/blit-daemon/src/service/push/control.rs:129:                force_grpc_effective = force_grpc_data || force_grpc_client;
crates/blit-daemon/src/service/push/control.rs:243:                    if flushed && data_plane_handle.is_none() && !force_grpc_effective {
crates/blit-daemon/src/service/push/control.rs:252:                                    force_grpc_effective = true;
crates/blit-daemon/src/service/push/control.rs:379:    let force_grpc_effective = force_grpc_effective || force_grpc_client;
crates/blit-daemon/src/service/push/control.rs:383:    } else if force_grpc_effective {
crates/blit-core/tests/transfer_session_roles.rs:94:        in_stream_bytes: true,
crates/blit-core/tests/transfer_session_roles.rs:176:            source_summary.in_stream_carrier_used,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:256:        force_grpc: false,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:357:        force_grpc: true,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:727:        spec.force_grpc,
crates/blit-core/tests/pull_sync_with_spec_wire.rs:743:    // `metadata_only` and runs the full force_grpc fallback — headers
crates/blit-core/tests/pull_sync_with_spec_wire.rs:789:    // force_grpc is set, so a daemon steering the session onto a TCP
crates/blit-core/tests/pull_sync_with_spec_wire.rs:856:    assert!(spec.force_grpc);
crates/blit-core/tests/proto_wire_compat.rs:69:    force_grpc: bool,
crates/blit-core/tests/proto_wire_compat.rs:112:    force_grpc: bool,
crates/blit-core/tests/proto_wire_compat.rs:210:        force_grpc: false,
crates/blit-core/tests/proto_wire_compat.rs:263:        force_grpc: false,
crates/blit-core/tests/proto_wire_compat.rs:285:        force_grpc: false,
crates/blit-core/tests/proto_wire_compat.rs:347:        force_grpc: true,
crates/blit-core/tests/proto_wire_compat.rs:356:    assert!(old.force_grpc);
crates/blit-core/src/transfer_session/mod.rs:521:                if local_role == TransferRole::Destination && !open.in_stream_bytes {
crates/blit-core/src/transfer_session/mod.rs:1258:                let in_stream_carrier_used = match data_plane_recv.take() {
crates/blit-core/src/transfer_session/mod.rs:1290:                    in_stream_carrier_used,
crates/blit-core/src/remote/pull.rs:73:    pub force_grpc: bool,
crates/blit-core/src/remote/pull.rs:248:    /// mirror/resume, and always `force_grpc` — bytes, when any, ride
crates/blit-core/src/remote/pull.rs:278:            force_grpc: true,
crates/blit-core/src/remote/pull.rs:355:    /// `force_grpc` it streams every file's bytes over the control
crates/blit-core/src/remote/pull.rs:387:                // force_grpc was set, so a real TCP negotiation (as
crates/blit-core/src/remote/pull.rs:395:                         metadata-only scan (force_grpc was set)"
crates/blit-core/src/remote/pull.rs:409:    /// single-file `force_grpc` PullSync session — ue-r2-1h's port of
crates/blit-core/src/remote/pull.rs:503:            force_grpc: options.force_grpc,
crates/blit-core/src/remote/pull.rs:1280:                        // force_grpc was set — a real TCP negotiation
crates/blit-core/src/remote/pull.rs:1286:                                 force_grpc single-file session",
crates/blit-core/src/remote/pull.rs:2334:        assert!(!spec.force_grpc);
crates/blit-core/src/remote/pull.rs:2418:    fn wire_equivalence_resume_and_filter_and_force_grpc() {
crates/blit-core/src/remote/pull.rs:2433:            force_grpc: true,
crates/blit-core/src/remote/pull.rs:2445:        assert!(spec.force_grpc);
crates/blit-core/src/remote/pull.rs:2499:            force_grpc: false,
crates/blit-core/src/remote/push/client/mod.rs:628:        force_grpc: bool,
crates/blit-core/src/remote/push/client/mod.rs:688:                force_grpc,
crates/blit-core/src/remote/push/client/mod.rs:723:        let mut fallback_used = force_grpc;
crates/blit-core/src/remote/push/client/mod.rs:726:        let mut transfer_mode = if force_grpc {
crates/blit-core/src/remote/push/client/mod.rs:736:        // has seen ManifestComplete. Pre-fix, force_grpc initialized
crates/blit-core/src/remote/transfer/session_client.rs:12://! otp-4a uses the in-stream byte carrier only (`in_stream_bytes`);
crates/blit-core/src/remote/transfer/session_client.rs:34:pub struct PushSessionOptions {
crates/blit-core/src/remote/transfer/session_client.rs:42:    /// data-plane fallback (`--force-grpc`-shaped).
crates/blit-core/src/remote/transfer/session_client.rs:43:    pub in_stream_bytes: bool,
crates/blit-core/src/remote/transfer/session_client.rs:46:impl Default for PushSessionOptions {
crates/blit-core/src/remote/transfer/session_client.rs:53:            in_stream_bytes: false,
crates/blit-core/src/remote/transfer/session_client.rs:62:pub async fn run_push_session(
crates/blit-core/src/remote/transfer/session_client.rs:65:    options: PushSessionOptions,
crates/blit-core/src/remote/transfer/session_client.rs:93:        in_stream_bytes: options.in_stream_bytes,
crates/blit-core/src/remote/transfer/operation_spec.rs:77:    pub force_grpc: bool,
crates/blit-core/src/remote/transfer/operation_spec.rs:155:            force_grpc: spec.force_grpc,
crates/blit-core/src/remote/transfer/operation_spec.rs:242:            force_grpc: false,
crates/blit-core/src/remote/transfer/sink.rs:923:/// Used when the TCP data plane is unavailable (`--force-grpc`) or when
crates/blit-cli/src/cli.rs:319:    pub force_grpc: bool,
crates/blit-cli/src/transfers/remote.rs:241:        force_grpc: args.force_grpc,
crates/blit-cli/src/transfers/remote.rs:368:            force_grpc: args.force_grpc,
crates/blit-cli/src/transfers/mod.rs:702:            force_grpc: false,
crates/blit-cli/src/transfers/mod.rs:751:            force_grpc: false,
crates/blit-cli/src/transfers/mod.rs:814:            force_grpc: false,
crates/blit-cli/src/transfers/remote_remote_direct.rs:94:        force_grpc: args.force_grpc,
crates/blit-cli/tests/remote_tcp_fallback.rs:8:/// Daemon forced into gRPC data fallback (`--force-grpc-data`).
crates/blit-cli/tests/remote_tcp_fallback.rs:11:        .extra_daemon_args(["--force-grpc-data"])
crates/blit-cli/tests/remote_tcp_fallback.rs:32:        .arg("--force-grpc")
crates/blit-cli/tests/remote_tcp_fallback.rs:70:/// --force-grpc, assert success, and return how many landed.
crates/blit-cli/tests/remote_tcp_fallback.rs:93:        .arg("--force-grpc")
crates/blit-cli/tests/remote_tcp_fallback.rs:130:/// the mid-manifest early flush, and a force_grpc client started
crates/blit-cli/tests/remote_remote.rs:158:    // single-file force_grpc reads). A nested multi-file tree
crates/blit-cli/tests/remote_parity.rs:164:        .arg("--force-grpc")
crates/blit-cli/tests/remote_parity.rs:205:        .arg("--force-grpc")
crates/blit-cli/tests/remote_parity.rs:253:        .arg("--force-grpc")
crates/blit-cli/tests/remote_resume.rs:86:/// Test that --resume with --force-grpc also works (fallback path).
crates/blit-cli/tests/remote_resume.rs:108:        .arg("--force-grpc")
crates/blit-cli/tests/common/mod.rs:212:    /// `--force-grpc-data`).
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:538:  "pull is single-stream via force_grpc, not a third ladder" conflated
docs/cli/blit.1.md:121:- `--force-grpc`
docs/ARCHITECTURE.md:225:  when `--force-grpc` is set or TCP is unavailable.
docs/WHITEPAPER.md:35:  streams, optional `--force-grpc` fallback that pushes file bytes via
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:86:      single-stream today via the `force_grpc` single-file path, not a
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md:152:  (`force_grpc` single-file path).
docs/plan/REMOTE_TRANSFER_PARITY.md:22:- ✅ Pull path now mirrors the hybrid transport: negotiation + multi-stream TCP data plane (with `--force-grpc` fallback) and shared progress reporting are live in both CLI and daemon.
docs/plan/REMOTE_TRANSFER_PARITY.md:32:| 3. Daemon Pull Pipeline | Rebuild `crates/blit-daemon/src/service/pull.rs` so it enumerates manifests once, plans payloads via shared module, sends negotiation (unless `force_grpc`), and streams files/tar shards over TCP using the existing push data-plane listener. Keep gRPC fallback behind `force_grpc`, but increase data-plane buffers / enable zero-copy just as v1 does so each record can saturate 10 GbE. | ✅ `service/pull.rs` now streams via TCP negotiation (2025-11-10); fallback path still available when forced. |
docs/plan/REMOTE_TRANSFER_PARITY.md:57:   - Preserve `--force-grpc` path by toggling negotiation.
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:204:  // MirrorMode, ResumeSettings, force_grpc, ignore_existing, and
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:431:   `resume`, `client_capabilities`, `force_grpc`, `ignore_existing`
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md:536:  force_grpc, source identity). It MAY leave
docs/plan/RELEASE_PLAN_v2_2026-05-04.md:135:  plane with parallel streams, gRPC fallback, force-grpc flag.
docs/plan/greenfield_plan_v6.md:161:   - Automatic fallback to gRPC-streamed data when the negotiated TCP port cannot be reached (firewall/NAT); surface as a warning and continue. If a deterministic override is ever needed for locked-down environments it will be a CLI flag (e.g. `--force-grpc-data`, marked diagnostics-only and sparingly added). **No env-var form** — env vars are not used for app or diagnostic configuration (audit-l39, 2026-06-04).
docs/plan/greenfield_plan_v6.md:245:   - Implement automatic gRPC-stream fallback when TCP negotiation fails; emit warning and respect advanced `force-grpc` override.
docs/plan/greenfield_plan_v6.md:267:  - Ensure advanced options (`--max-threads`, `--force-grpc-data`) are documented in “Advanced / Niche” sections of help/man pages.
docs/plan/WORKFLOW_PHASE_3.md:54:| 3.3.5 | Handle gRPC fallback automatically when TCP negotiation fails; emit warning and continue. | ✅ 2025-10-25 – integration test (`remote_tcp_fallback`) forces client `--force-grpc` and asserts CLI reports `[gRPC fallback]` with files mirrored. |
docs/DAEMON_CONFIG.md:290:    --force-grpc-data        Force gRPC data plane (disable TCP)
docs/DAEMON_CONFIG.md:598:blit-daemon --force-grpc-data
docs/cli/blit-daemon.1.md:70:- `--force-grpc-data`  
docs/reviews/followup_review_2026-05-02.md:1503:destination path, `bool mirror_mode`, `bool force_grpc`, source locator, and
docs/reviews/followup_review_2026-05-02.md:1701:`bool force_grpc`) is gone. All of `MirrorMode`, `ComparisonMode`,
docs/API.md:88:  bool force_grpc = 4;         // Disable TCP data plane
docs/API.md:346:            false, // force_grpc
docs/ux-feedback-migrate-games-poc.md:159:--force-grpc, --resume, --null, --json
docs/ux-feedback-migrate-games-poc.md:162:Common-case flags (`-p`, `-v`, `--dry-run`) are interleaved with niche performance/debug flags (`--null`, `--force-grpc`, `--retries`). A new user reading top-to-bottom sees `--null` and `--force-grpc` before they see `--progress`.
docs/audit/inventory/code-tests-scripts.md:30:| `crates/blit-cli/tests/remote_tcp_fallback.rs` | 243 | --force-grpc-data daemon flag exercise |
docs/audit/inventory/code-tests-scripts.md:104:- **grpc-fallback-many-small-files** — `crates/blit-cli/tests/remote_parity.rs:170-220` — 50 small files via `--force-grpc`; verifies every file & bytes arrive. Step-4C parity guard. _(notes: tar-shard batching on fallback path)_
docs/audit/inventory/code-tests-scripts.md:105:- **force-grpc-data-daemon-flag** — `crates/blit-cli/tests/remote_tcp_fallback.rs:134-148` — daemon launched with `--force-grpc-data`; client expected to also emit `gRPC data fallback` even without `--force-grpc`. _(notes: daemon-side force flag; bench script also uses `--force-grpc-data` while tests prefer the CLI flag)_
docs/audit/inventory/code-tests-scripts.md:156:- **resume-flag-with-grpc-fallback** — `crates/blit-cli/tests/remote_resume.rs:91-132` — `--resume --force-grpc` exercises legacy fallback path; stdout must contain `[gRPC fallback]` string. _(notes: brittle string match again)_
docs/audit/inventory/plan-wire.md:339:**Source**: WHITEPAPER.md:§1; DAEMON_CONFIG.md:§Performance Tuning; blit.proto:TransferOperationSpec.force_grpc
docs/audit/inventory/plan-wire.md:342:Optional `--force-grpc` / `--force-grpc-data` fallback that pushes file bytes via the gRPC channel. Origin still honors filter/compare/mirror/resume; only byte transport changes.
docs/audit/inventory/plan-wire.md:384:`PushHeader`: `module` (1), `mirror_mode` (2, bool — master switch for purging), `destination_path` (3), `force_grpc` (4), `FilterSpec filter` (5), `MirrorMode mirror_kind` (6), `require_complete_scan` (7, bool).
docs/audit/inventory/plan-wire.md:408:`PullRequest`: `module` (1), `path` (2), `force_grpc` (3), `metadata_only` (4).
docs/audit/inventory/plan-wire.md:516:`TransferOperationSpec` (unified contract): `spec_version` (1), `module` (2), `source_path` (3), `FilterSpec filter` (4), `ComparisonMode compare_mode` (5), `MirrorMode mirror_mode` (6), `ResumeSettings resume` (7), `PeerCapabilities client_capabilities` (8), `force_grpc` (9), `ignore_existing` (10), `require_complete_scan` (11).
docs/audit/inventory/plan-wire.md:696:`blit-daemon` CLI: `--config <PATH>`, `--bind <ADDR>`, `--port <PORT>`, `--root <PATH>`, `--no-mdns`, `--mdns-name <NAME>`, `--force-grpc-data`, `--no-server-checksums`, `-h/--help`.
docs/audit/AUDIT_REPORT_2026-06-04.md:223:**Code does**: Daemon `--force-grpc-data` flag exists; client `--force-grpc` flag exists; perf-history opt-out is via `blit diagnostics perf --disable` writing `settings.json`. The two named env vars are dead text.
docs/audit/2026-05-04_roadmap_audit.md:169:| Daemon pull pipeline rebuilt to use TCP data plane with `--force-grpc` fallback | SHIPPING | `crates/blit-daemon/src/service/pull.rs:656` instantiates `BufferPool` and TCP listener; gRPC fallback retained. |
docs/audit/inventory/code-daemon.md:32:- **rpc-pull-dispatch** — `crates/blit-daemon/src/service/core.rs:548-599` — `pull` handler: synchronously resolves module (returns NotFound before spawn), increments `metrics.inc_pull()`. Registers `ActiveJob` with populated module/path (unlike push/pull_sync). Honors both `req.force_grpc` AND service `force_grpc_data`.
docs/audit/inventory/code-daemon.md:176:- **force-grpc-OR-semantics** — `crates/blit-daemon/src/service/core.rs:556` — `req.force_grpc OR self.force_grpc_data` — either side can force gRPC fallback.
docs/audit/inventory/plan-principles.md:212:#### iface-force-grpc-data-override
docs/audit/inventory/plan-principles.md:216:> Automatic fallback to gRPC-streamed data when the negotiated TCP port cannot be reached (firewall/NAT); surface as a warning and continue, with an advanced `--force-grpc-data`/`BLIT_FORCE_GRPC_DATA=1` override for locked-down environments.
docs/audit/inventory/plan-principles.md:628:> Push: control plane + bounded-channel manifest, NeedList, TCP data plane with parallel streams, gRPC fallback, force-grpc flag. Pull (PullSync): unified spec, filter parity, tar shards, delete list, checksum negotiation (F11/R15), gRPC fallback. Remote→remote delegation (`DelegatedPull`): Default direct path. `--relay-via-cli` operator escape hatch. Delegation gate (`[delegation]` config block, IDNA/CIDR/IP matching, R25-F3 special-range rule, DNS-rebinding mitigation, per-module override). No-silent-fallback CLI dispatch. Admin RPCs: ListModules, List, Find, DiskUsage, FilesystemStats, CompletePath, Purge.
docs/audit/inventory/code-core-transfer.md:118:- **scan-remote-files-force-grpc** — `crates/blit-core/src/remote/pull.rs:461-494` — Sets `force_grpc=true` + `metadata_only=true` so headers come back on the gRPC control stream. _(notes: comment claims "Force gRPC to get headers in the control stream")_
docs/audit/inventory/code-core-transfer.md:184:- **force-grpc-vs-data-plane-mode** — `crates/blit-core/src/remote/push/client/mod.rs:411-418` — `force_grpc=true` → `transfer_mode = TransferMode::Fallback` straight away.
docs/audit/inventory/code-core-transfer.md:185:- **force-grpc-scan-metadata-only** — `crates/blit-core/src/remote/pull.rs:470-473, 509-512` — `scan_remote_files` / `open_remote_file` always set `force_grpc=true`.
docs/audit/inventory/code-core-transfer.md:238:- **`force_grpc` semantics overloaded** — On pull this also implies `metadata_only=true` for scan, but the push call site (`mod.rs:411-418`) uses it as a strict mode lock. Two different meanings.
docs/audit/inventory/code-tui-display.md:148:- **build-f1-push-execution** — `crates/blit-tui/src/exec_plan.rs:50-76` — Constructs `PushExecution` with mirror_mode/mirror_kind/require_complete_scan derived from `kind == Mirror`. _(notes: `force_grpc: false`, `trace_data_plane: false` hardcoded; `remote_label` derived from `remote.display()`.)_
docs/audit/inventory/plan-cli.md:168:#### opt-force-grpc
docs/audit/inventory/plan-cli.md:172:`--force-grpc` — Bypass the TCP data plane negotiation and stream payloads over gRPC.
docs/audit/inventory/plan-cli.md:238:CHANGELOG lists `--workers` as a shipped CLI option alongside `--dry-run`, `--checksum`, `--force-grpc`.
docs/audit/inventory/plan-cli.md:644:Shipped CLI options: `--dry-run`, `--checksum`, `--force-grpc`, `--workers`, plus `--progress`, `--verbose`.
docs/audit/inventory/plan-cli.md:668:Daemon flags shipped: `--root` default export, `--no-mdns`, `--force-grpc-data`.
docs/audit/inventory/plan-phases.md:316:> "Handle gRPC fallback automatically when TCP negotiation fails; emit warning and continue." Integration test (`remote_tcp_fallback`) forces client `--force-grpc` and asserts CLI reports `[gRPC fallback]` with files mirrored.
docs/audit/inventory/plan-phases.md:604:> "2025-10-25 – integration test (`remote_tcp_fallback`) forces client `--force-grpc` and asserts CLI reports `[gRPC fallback]` with files mirrored."
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:82:**Mechanism**: rg for `.pull(` workspace-wide finds only three gRPC-stub calls inside blit-core/src/remote/pull.rs itself: line 305 (inside the deprecated `pull` method at :251), line 491 (scan_remote_files) and line 539 (open_remote_file). The latter two hardcode force_grpc:true (:485, :530), so the daemon's Pull handler always takes the gRPC/non-streaming branches (daemon pull.rs:64 single-file, :85 force_grpc||metadata_only) and never reaches stream_pull_streaming (:208) or the TCP accepts (accept_pull_data_connection :625, accept_pull_data_connection_streaming :841, enumerate_to_channel :764, pull_stream_count :915). The only code that could send force_grpc=false is the deprecated client method, which nothing calls — pull.rs's own test doc at :1855 calls it 'the deprecated `pull` method', and the daemon comment at pull.rs:694-696 calls its server half 'this deprecated-but-exposed Pull RPC path'. Meanwhile the live PullSync handler negotiates stream_count=1 at both of its negotiation sites (pull_sync.rs:567-568 with the comment 'multi-stream support lives in pull.rs', and :707), even though the blit-core client side can receive multiple streams (receive_data_plane_streams_owned, pull.rs:1600-1646). Net: production pull runs one TCP stream; the up-to-16-stream ladder lives only in dead code. Wire-compat caveat: proto/blit.proto:11 still declares `rpc Pull`, so out-of-repo older clients could reach the TCP branches — retiring the RPC is an owner decision.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:86:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:485 — scan_remote_files: force_grpc: true — 'Force gRPC to get headers in the control stream'; open_remote_file same at :530
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:198:**Mechanism**: control.rs:55 creates `mpsc::channel::<FileHeader>(FILE_UPLOAD_CHANNEL_CAPACITY)` (= 16*1024*16 = 262,144, :31); every manifest entry passing file_requires_upload is sent at :157 *before* the transfer-mode branch. In fallback mode (client --force-grpc, daemon force_grpc_data, or the automatic bind-failure fallback at :181-199) `upload_rx_opt.take()` is never executed (:214/:287 are TCP-only), so the receiver stays alive-but-unread in the local until the function returns: send #262,145 awaits forever. The daemon stops reading the request stream, gRPC flow control backpressures the client's manifest sends, and both sides wedge with no timeout in scope — HTTP/2 keepalive (main.rs:137-142) sees a healthy connection, StallGuard covers TCP data planes only. In TCP mode the consumer (data_plane.rs:89, :164) wraps the receiver in Arc<AsyncMutex>, each of N workers spawns a task (:200-206) whose only body is `while guard.recv().await.is_some() {}` (N-1 of them blocked on the mutex), and the companion `cache` is explicitly voided (:207) — the comment at control.rs:150-156 admits 'Only the gRPC fallback path uses this queue', which is false: the fallback path is the one that never reads it.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:206:**Proposed fix**: Delete the upload_tx/upload_rx + cache plumbing entirely (headers travel on the wire post-Phase-5; nothing consumes them); if any consumer is ever reintroduced it should own the Receiver directly, not share it through a mutex. Regression: force-grpc push with a synthetic >capacity manifest completes instead of hanging.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:692:**Mechanism**: tests/common/mod.rs::TestContext (lines 56-169) is the nominal shared harness, but remote_remote.rs (DualDaemonContext::spawn_daemon, line 119), remote_pull_mirror.rs (spawn_daemon, line 247), remote_checksum_negotiation.rs (spawn_daemon_harness, line 96), and remote_tcp_fallback.rs (inline clone, cargo build at line 104) each re-implement it because TestContext cannot express their one extra knob (delegation config, second daemon, extra daemon args like --no-server-checksums / --force-grpc-data). Drift is already observable: common's DaemonConfig lacks the delegation/delegation_allowed fields remote_remote.rs added (remote_remote.rs:17-42); remote_remote.rs::build_daemon (line 417) dropped the --target triple handling the other four copies carry (common/mod.rs:123-127), so it builds into the wrong directory under cross-target test runs; stderr policy differs (common pipes it, the others null it). cli_bin() is additionally pasted into single_file_copy.rs:32, local_move_semantics.rs:38, diagnostics_dump.rs:14, cli_arg_safety_gates.rs:40, and remote_pull_mirror.rs:329. Any daemon config-schema or harness fix must now land in 5 places, and the next one will miss at least one (the --target drift proves it already happened).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:698:- /home/michael/dev/Blit/crates/blit-cli/tests/remote_tcp_fallback.rs:104 — fifth full harness clone (map said four), differing only by the --force-grpc-data daemon arg
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1183:Scope: verified every entry in the design map's Part 2 Dead/abandoned lists for blit-core, blit-daemon, blit-cli, and blit-app, re-deriving each from code (rg caller searches + file reads) rather than trusting the map. VERIFIED-DEAD and reported: blit-core errors.rs, tar_stream.rs, zero_copy.rs, delete.rs, copy/parallel.rs+stats.rs, chunked_copy_file, fs_enum categorize_files/enumerate_symlinks/SymlinkEntry/enumerate_directory_deref_filtered, auto_tune warmup machinery, transfer_payloads_via_control_plane, RemotePullClient::pull + daemon legacy Pull TCP plane (with the pull_sync single-stream FAST consequence and the proto wire-compat owner decision), daemon push upload channel + drain task, daemon dead items behind allow(dead_code) (ModuleOptOut, resolve_contained_wire, acquire_buffer, ActiveJobs::cancel/as_str), CLI --interval-ms, CLI unused deps, blit-app empty remote_remote_direct stub + perf::query/PerfReport + stale WatchSnapshot allow. CHECKED AND FOUND LIVE (clean — valuable for Phase C): manifest.rs (consumed by daemon pull_sync); copy_file/copy_paths_blocking/resume_copy_file/mmap_copy_file (live local fast path via orchestrator.rs:355/:1263, local_worker.rs); scan_remote_files and open_remote_file (live, but force_grpc=true only); build_spec_from_options (live in blit-app, blit-tui, daemon); pull_sync client multi-stream receive machinery (pull.rs:1600-1646 — capable but never fed >1 stream by the daemon, folded into the Pull finding); WatchSnapshot, spawn_progress_ticker, active_jobs snapshot/recent/transfer_id/bytes_counter (live — only their allow annotations are stale); cancel_authorized (live at core.rs); push/data_plane.rs+push/payload.rs re-export shims (alive as indirection; judged low severity, dropped); blit-cli endpoints.rs wrapper module, DeferredPullState/DeferredDelegatedState aliases, rm.rs re-export (alive A.0 shims, low, dropped); tests/blit_utils.rs (runs and partially unique — overlap is a test-hygiene issue, judged low for this dimension); ls.rs defensive unreachable Discovery arm (intentional, low, dropped); buffer.rs BufferPool stats counters (vestigial but low, dropped). NOT RE-REPORTED per instructions: design-1/2/3 findings and all queued slice-2 transport items — notably I confirmed blit-app client::CONNECT_TIMEOUT has zero external consumers but folded that into the queued shared-channel-builder work instead of filing it. blit-tui: light pass only — confirmed it consumes RemotePullClient::build_spec_from_options and blit_app::profile (no TUI-internal dead-code findings filed per the Phase-6 rule). Not covered: blit-prometheus-bridge (map reports no dead list for it; did not independently sweep), Windows-only win_fs paths (cannot exercise; caller search only), and git-history dating of modules (read-only session, relied on map dates only for narrative, not for any claim).
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1187:Checked and found clean: (1) Local mirror deletion semantics — orchestrator.rs has a substantial cfg(test) suite (lines 1300+, 1971+) including mirror_still_deletes_truly_unrelated_destination_dirs (:1616), mirror_refuses_when_source_scan_incomplete (:1448), local_mirror_all_scope_deletes_through_filter (:1753), compare-mode and ignore-existing tests; diff_planner.rs has 14 unit tests covering plan_local_mirror; local_move_semantics.rs covers move-vs-mirror regression (R46-F1). (2) Force-gRPC paths ARE integration-tested (contrary to the dimension brief's suspicion): remote_parity.rs test_pull_grpc_fallback/:92 and test_push_grpc_fallback/:133, remote_tcp_fallback.rs --force-grpc-data/:137 and --force-grpc/:168, remote_resume.rs gRPC-fallback resume/:91 — but ALL unix-gated (folded into the cfg(unix) finding). (3) Remote pull-mirror purge semantics well covered on unix (remote_pull_mirror.rs:46,:342,:404 cover purge, FilteredSubset preservation, delete-scope-all). (4) Push mirror safety F1/F2 (incomplete-scan refusal, filtered enumeration) covered in remote_push_mirror_safety.rs. (5) Daemon unit-test density is good: active_jobs.rs 29, delegation_gate.rs 31, core.rs 25, pull_sync.rs 6, push/data_plane.rs 13. (6) Checksum negotiation ack flow covered (remote_checksum_negotiation.rs). Notable non-finding per instructions: multi-stream pull is NOT a test gap — the live PullSync path hard-codes stream_count=1 (pull_sync.rs:568, :707) and the multi-stream machinery lives only in the deprecated Pull client (blit-core pull.rs:251/1600) which has zero callers outside its own file (rg verified) — that is a Phase B dead-code question, already adjacent to the map's 'deprecated pull client path' dead-weight list. Map claim correction: 'all remote integration tests are cfg(unix)' is overstated — remote_move.rs, remote_pull_subpath.rs, admin_verbs.rs, and blit_utils.rs spawn daemons ungated and run on Windows CI; my cfg(unix) finding is re-derived around the files that are gated without unix APIs. Not covered: blit-tui test quality (light-pass rule; it has many in-crate tests, e.g. main.rs:8736+ purge-safety assertions, left to Phase 6), blit-prometheus-bridge tests, the .ps1 journal/USN scripts' coverage, and proto-level compatibility tests (no buf/breaking-change gate exists, but I could not size that without speculating). I did not run any cargo commands (read-only constraint), so test counts are from source inspection, not execution.
docs/audit/DESIGN_MAP_2026-06-11.md:1151:  - evidence: pull.rs:694-696 calls its own RPC 'this deprecated-but-exposed Pull RPC path'. Repo-wide rg for `.pull(` shows the only Pull-RPC clients are blit-core/src/remote/pull.rs:305 (RemotePullClient::pull — itself uncalled by blit-app/blit-cli/blit-tui/tests; rg for `RemotePullClient::pull`, `.pull(`, `pull_with_options` across those crates returns nothing) and :491/:539 (scan_remote_files/open_remote_file, both hardcoding force_grpc=true so they take only the gRPC branch at pull.rs:85-105). The TCP-negotiation branches are unreachable from any in-repo caller. pull_sync.rs:567-568 confirms the half-migration: 'Single stream for the resume path (multi-stream support lives in pull.rs)' — the PRODUCTION pull path runs one TCP stream while multi-stream support survives only in the dead RPC.
docs/audit/DESIGN_MAP_2026-06-11.md:1389:  - evidence: crates/blit-core/src/remote/transfer/data_plane.rs and grpc_fallback.rs exist; proto/blit.proto:159/241/472 force_grpc flags
docs/audit/DESIGN_MAP_2026-06-11.md:1435:- **[complete]** Automatic gRPC fallback when TCP fails, with diagnostics-only force-grpc CLI flag (no env-var form)
docs/audit/DESIGN_MAP_2026-06-11.md:1436:  - evidence: crates/blit-core/src/remote/transfer/grpc_fallback.rs; crates/blit-cli/src/cli.rs:319 `pub force_grpc: bool` (named --force-grpc rather than the doc's example --force-grpc-data)
docs/audit/DESIGN_MAP_2026-06-11.md:1451:- **[abandoned]** Advanced options `--max-threads` / `--force-grpc-data` documented in help/man pages (Phase 4 bullet)
docs/audit/DESIGN_MAP_2026-06-11.md:1611:- **[complete]** Pull path mirrors hybrid transport: TCP negotiation + multi-stream data plane with --force-grpc fallback, in both CLI and daemon
docs/audit/DESIGN_MAP_2026-06-11.md:1612:  - evidence: crates/blit-core/src/remote/pull.rs:439-455 (client decodes negotiation token/port/stream_count); crates/blit-daemon/src/service/pull.rs:158-178 and 268-289 (daemon binds listener via shared bind_data_plane_listener and sends negotiation); force_grpc at pull.rs:128,254
docs/audit/DESIGN_MAP_2026-06-11.md:1693:  - evidence: crates/blit-cli/tests/remote_tcp_fallback.rs exists; force_grpc in crates/blit-core/src/remote/pull.rs:128 and grpc_fallback.rs module
docs/audit/inventory/code-cli.md:9:- **transferargs-shape** — `crates/blit-cli/src/cli.rs:188-364` — `TransferArgs` defines every flag for Copy/Mirror/Move: comparison (`--checksum`/`--size-only`/`--ignore-times`/`--ignore-existing`/`--force`/`--delete-scope`), reliability (`--resume`/`--retry`/`--wait`), filtering (`--exclude`/`--include`/`--files-from`/`--min-size`/`--max-size`/`--min-age`/`--max-age`), performance (`--force-grpc`/`--relay-via-cli`/`--detach`/`--null`), and hidden (`--workers`/`--trace-data-plane`). _(notes: single struct shared across three verbs even when many combinations are illegal — gates live downstream in run_move/run_transfer)_
docs/audit/findings/drift-wire.md:39:**Plan says**: `docs/DAEMON_CONFIG.md:280-293` `--config`, `--bind`, `--port`, `--root`, `--no-mdns`, `--mdns-name`, `--force-grpc-data`, `--no-server-checksums`, `-h/--help`. Plan claim `iface-daemon-cli-flags` enumerates the same list.
docs/audit/findings/drift-perf.md:132:### force-grpc-and-relay-via-cli-as-tuning-knobs — Plan says "no user speed knobs" but CLI still has them
docs/audit/findings/drift-perf.md:135:- `--force-grpc` (cli.rs has it visible; not hidden)

exec
/usr/bin/zsh -lc "rg -n \"fn initial_stream_proposal|pub fn initial_stream_proposal|DIAL_FLOOR_CHUNK_BYTES|DIAL_FLOOR_PREFETCH|local_receiver_capacity\" crates/blit-core/src/engine.rs crates/blit-core/src/engine -g '*.rs'" in /home/michael/dev/Blit
 exited 2 in 0ms:
rg: crates/blit-core/src/engine.rs: No such file or directory (os error 2)
crates/blit-core/src/engine/dial.rs:37:pub const DIAL_FLOOR_CHUNK_BYTES: usize = 16 * MIB;
crates/blit-core/src/engine/dial.rs:38:pub const DIAL_FLOOR_PREFETCH: usize = 4;
crates/blit-core/src/engine/dial.rs:72:pub fn local_receiver_capacity() -> CapacityProfile {
crates/blit-core/src/engine/dial.rs:171:            chunk_bytes: AtomicUsize::new(DIAL_FLOOR_CHUNK_BYTES.min(ceiling_chunk)),
crates/blit-core/src/engine/dial.rs:172:            prefetch_count: AtomicUsize::new(DIAL_FLOOR_PREFETCH.min(ceiling_prefetch)),
crates/blit-core/src/engine/dial.rs:254:            <= DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes)
crates/blit-core/src/engine/dial.rs:256:                <= DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch).max(1)
crates/blit-core/src/engine/dial.rs:430:        let next = (chunk / 2).max(DIAL_FLOOR_CHUNK_BYTES.min(self.ceiling_chunk_bytes));
crates/blit-core/src/engine/dial.rs:437:            .max(DIAL_FLOOR_PREFETCH.min(self.ceiling_prefetch))
crates/blit-core/src/engine/dial.rs:474:pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
crates/blit-core/src/engine/dial.rs:687:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
crates/blit-core/src/engine/mod.rs:30:    initial_stream_proposal, local_receiver_capacity, spawn_dial_tuner,
crates/blit-core/src/engine/mod.rs:33:    DIAL_FLOOR_CHUNK_BYTES, DIAL_FLOOR_PREFETCH, DIAL_STEP_DOWN_BLOCKED_RATIO,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine.rs | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
nl: crates/blit-core/src/engine.rs: No such file or directory

exec
/usr/bin/zsh -lc 'rg -n "initial_stream_proposal" crates/blit-core/src crates/blit-core/tests' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/transfer_session/data_plane.rs:16://! [`initial_stream_proposal`] with zero knowledge is 1 — the session
crates/blit-core/src/transfer_session/data_plane.rs:31:    initial_stream_proposal, local_receiver_capacity, DIAL_FLOOR_CHUNK_BYTES, DIAL_FLOOR_PREFETCH,
crates/blit-core/src/transfer_session/data_plane.rs:115:    let initial_streams = initial_stream_proposal(0, 0, ceiling).max(1);
crates/blit-core/src/engine/dial.rs:338:    /// [`initial_stream_proposal`] assigns the full workload. As the
crates/blit-core/src/engine/dial.rs:474:pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
crates/blit-core/src/engine/dial.rs:687:    fn initial_stream_proposal_matches_the_retired_daemon_table() {
crates/blit-core/src/engine/dial.rs:691:        assert_eq!(initial_stream_proposal(0, 0, 32), 1);
crates/blit-core/src/engine/dial.rs:695:        assert_eq!(initial_stream_proposal(32 * MIB64 - 1, 10, 32), 1);
crates/blit-core/src/engine/dial.rs:696:        assert_eq!(initial_stream_proposal(32 * MIB64, 10, 32), 2);
crates/blit-core/src/engine/dial.rs:697:        assert_eq!(initial_stream_proposal(128 * MIB64 - 1, 10, 32), 2);
crates/blit-core/src/engine/dial.rs:698:        assert_eq!(initial_stream_proposal(128 * MIB64, 10, 32), 4);
crates/blit-core/src/engine/dial.rs:699:        assert_eq!(initial_stream_proposal(512 * MIB64 - 1, 10, 32), 4);
crates/blit-core/src/engine/dial.rs:700:        assert_eq!(initial_stream_proposal(512 * MIB64, 10, 32), 8);
crates/blit-core/src/engine/dial.rs:701:        assert_eq!(initial_stream_proposal(2 * GIB - 1, 10, 32), 8);
crates/blit-core/src/engine/dial.rs:702:        assert_eq!(initial_stream_proposal(2 * GIB, 10, 32), 10);
crates/blit-core/src/engine/dial.rs:703:        assert_eq!(initial_stream_proposal(8 * GIB - 1, 10, 32), 10);
crates/blit-core/src/engine/dial.rs:704:        assert_eq!(initial_stream_proposal(8 * GIB, 10, 32), 12);
crates/blit-core/src/engine/dial.rs:705:        assert_eq!(initial_stream_proposal(32 * GIB - 1, 10, 32), 12);
crates/blit-core/src/engine/dial.rs:706:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 32), 16);
crates/blit-core/src/engine/dial.rs:708:        assert_eq!(initial_stream_proposal(1, 256, 32), 2);
crates/blit-core/src/engine/dial.rs:709:        assert_eq!(initial_stream_proposal(1, 2_000, 32), 4);
crates/blit-core/src/engine/dial.rs:710:        assert_eq!(initial_stream_proposal(1, 10_000, 32), 8);
crates/blit-core/src/engine/dial.rs:711:        assert_eq!(initial_stream_proposal(1, 50_000, 32), 10);
crates/blit-core/src/engine/dial.rs:712:        assert_eq!(initial_stream_proposal(1, 80_000, 32), 12);
crates/blit-core/src/engine/dial.rs:713:        assert_eq!(initial_stream_proposal(1, 200_000, 32), 16);
crates/blit-core/src/engine/dial.rs:715:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 6), 6);
crates/blit-core/src/engine/dial.rs:716:        assert_eq!(initial_stream_proposal(32 * GIB, 10, 0), 1, "floor 1");
crates/blit-core/src/engine/dial.rs:936:        assert_eq!(initial_stream_proposal(10_000 * 4 * KIB, 10_000, 32), 8);
crates/blit-core/src/engine/dial.rs:938:        assert_eq!(initial_stream_proposal(GIB, 1, 32), 8);
crates/blit-core/src/engine/dial.rs:942:            initial_stream_proposal(512 * MIB64 + 5_000 * 2 * KIB, 5_001, 32),
crates/blit-core/src/engine/dial.rs:948:        assert_eq!(initial_stream_proposal(1_000 * 4 * KIB, 1_000, 32), 2);
crates/blit-core/src/engine/mod.rs:30:    initial_stream_proposal, local_receiver_capacity, spawn_dial_tuner,
crates/blit-core/src/remote/push/client/mod.rs:523:/// need list accumulates, re-run [`crate::engine::initial_stream_proposal`]
crates/blit-core/src/remote/push/client/mod.rs:545:        crate::engine::initial_stream_proposal(need_bytes, need_count, dial.ceiling_max_streams())

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/dial.rs | sed -n '60,125p;460,505p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    60	/// after the cheap dials are pinned at a bound and the signal has held
    61	/// for [`RESIZE_SUSTAIN_TICKS`] consecutive ticks, and never within
    62	/// [`RESIZE_COOLDOWN_TICKS`] of the previous settle. One stream per
    63	/// epoch (the wire carries one `sub_token` per ADD).
    64	pub const RESIZE_COOLDOWN_TICKS: u32 = 4;
    65	pub const RESIZE_SUSTAIN_TICKS: i32 = 2;
    66	
    67	/// The capacity profile this host advertises when it is the byte
    68	/// RECEIVER (ue-r2-1e: the first real sender of the ue-r2-1b wire
    69	/// fields). Honest system facts only — fields we cannot measure yet
    70	/// stay 0 (= unknown per the wire contract), never fabricated:
    71	/// ceilings mirror what today's receive paths actually accept.
    72	pub fn local_receiver_capacity() -> CapacityProfile {
    73	    CapacityProfile {
    74	        cpu_cores: num_cpus::get() as u32,
    75	        drain_class: 0,
    76	        load_percent: 0,
    77	        max_streams: DIAL_CEILING_MAX_STREAMS as u32,
    78	        drain_rate_bytes_per_sec: 0,
    79	        max_chunk_bytes: DIAL_CEILING_CHUNK_BYTES as u64,
    80	        max_inflight_bytes: (DIAL_CEILING_CHUNK_BYTES * DIAL_CEILING_PREFETCH) as u64,
    81	    }
    82	}
    83	
    84	/// The one mutable tuning object for a transfer.
    85	#[derive(Debug)]
    86	pub struct TransferDial {
    87	    chunk_bytes: AtomicUsize,
    88	    prefetch_count: AtomicUsize,
    89	    /// 0 = unset (kernel default), matching the old `Option<usize>`.
    90	    tcp_buffer_bytes: AtomicUsize,
    91	    initial_streams: AtomicUsize,
    92	    max_streams: AtomicUsize,
    93	    // ── ue-r2-2 resize state (all epochs are the wire's monotonic
    94	    // resize ids; 0 is reserved for the initial stream set) ──────────
    95	    /// Settled live stream count. Epoch-0 write is
    96	    /// `set_negotiated_streams`; later writes come from
    97	    /// `resize_settled` on an accepted epoch.
    98	    live_streams: AtomicUsize,
    99	    /// Last settled epoch (0 until the first accepted resize).
   100	    resize_epoch: AtomicU32,
   101	    /// In-flight proposal's epoch; 0 = none. While non-zero no new
   102	    /// proposal is produced (the wire is idempotent but overlapping
   103	    /// epochs would complicate sub-token registration).
   104	    pending_epoch: AtomicU32,
   105	    /// Resize-eligible ticks since the last settle (cooldown clock).
   106	    ticks_since_settle: AtomicU32,
   107	    /// Consecutive same-direction tick counter: positive = "pipe clean
   108	    /// AND cheap dials maxed" streak, negative = "blocked AND cheap
   109	    /// dials floored" streak. Any other tick resets it.
   110	    resize_sustain: AtomicI32,
   111	    // Profile-clamped bounds, fixed at construction.
   112	    ceiling_chunk_bytes: usize,
   113	    ceiling_prefetch: usize,
   114	    ceiling_max_streams: usize,
   115	    ceiling_tcp_buffer_bytes: usize,
   116	}
   117	
   118	/// One engine resize decision (`ue-r2-2`). The adapter that owns the
   119	/// control stream turns this into a wire `DataPlaneResize` (the engine
   120	/// stays wire-type-free here on purpose) and MUST eventually call
   121	/// [`TransferDial::resize_settled`] for the epoch — with what actually
   122	/// happened — or no further proposals are produced.
   123	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
   124	pub struct ResizeProposal {
   125	    /// The wire epoch for this change (`resize_epoch() + 1`).
   460	
   461	/// Workload-shape-aware initial stream proposal (`ue-r2-1f`): the
   462	/// end that KNOWS the workload shape proposes a starting stream
   463	/// count — file count matters as much as bytes (many small files
   464	/// parallelize on per-file overhead even at low byte totals). On push
   465	/// that is the receiving daemon (it has the manifest) clamped to its
   466	/// own advertised ceiling; on pull_sync it is the sending daemon (it
   467	/// enumerated the source) clamped to the CLIENT's advertised
   468	/// `receiver_capacity.max_streams` (`ue-r2-1g`) — either way the byte
   469	/// receiver's profile is the bound. Table carried over verbatim from
   470	/// the daemon push `desired_streams` ladder it retires (the ladder
   471	/// the old `tuning.rs` doc said "wins"), now engine-owned. The
   472	/// sender's dial clamps again on its side (`set_negotiated_streams`).
   473	/// Live mid-transfer stream changes arrive with `ue-r2-2` resize.
   474	pub fn initial_stream_proposal(total_bytes: u64, file_count: usize, ceiling: usize) -> u32 {
   475	    if file_count == 0 {
   476	        return 1;
   477	    }
   478	    let proposal: u32 = if total_bytes >= 32 * 1024 * 1024 * 1024 || file_count >= 200_000 {
   479	        16
   480	    } else if total_bytes >= 8 * 1024 * 1024 * 1024 || file_count >= 80_000 {
   481	        12
   482	    } else if total_bytes >= 2 * 1024 * 1024 * 1024 || file_count >= 50_000 {
   483	        10
   484	    } else if total_bytes >= 512 * 1024 * 1024 || file_count >= 10_000 {
   485	        8
   486	    } else if total_bytes >= 128 * 1024 * 1024 || file_count >= 2_000 {
   487	        4
   488	    } else if total_bytes >= 32 * 1024 * 1024 || file_count >= 256 {
   489	        2
   490	    } else {
   491	        1
   492	    };
   493	    proposal.min(ceiling.max(1) as u32)
   494	}
   495	
   496	/// Blocked-time ratio for one tuner tick: the share of the tick's
   497	/// wall-clock (× stream count) the senders spent inside socket writes.
   498	/// 0 streams or a zero-length tick reads as "no signal" (0.0 — the
   499	/// hysteresis band holds the dial still rather than guessing).
   500	pub(crate) fn blocked_ratio(
   501	    delta_blocked_nanos: u64,
   502	    elapsed: std::time::Duration,
   503	    streams: usize,
   504	) -> f64 {
   505	    let denom = elapsed.as_nanos().saturating_mul(streams as u128);

codex
So far I have one substantive correctness issue: the destination validates “how many” files arrived on the data plane, not “which” files. I’m doing one pass for dependency boundaries and line-level evidence before I write the verdict.
exec
/usr/bin/zsh -lc "rg -n \"pub struct AbortOnDrop|AbortOnDrop\" crates/blit-core/src -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/transfer_session/data_plane.rs:42:    execute_sink_pipeline_streaming, generate_sub_token, AbortOnDrop, DataPlaneSession,
crates/blit-core/src/transfer_session/data_plane.rs:221:    // `AbortOnDrop<T>` wraps a `JoinHandle<T>`; the task's output is
crates/blit-core/src/transfer_session/data_plane.rs:223:    pipeline: Option<AbortOnDrop<Result<SinkOutcome>>>,
crates/blit-core/src/transfer_session/data_plane.rs:265:    // Bounded by AbortOnDrop: a fault on the control lane that drops the
crates/blit-core/src/transfer_session/data_plane.rs:267:    let pipeline = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/transfer_session/mod.rs:43:use crate::remote::transfer::{AbortOnDrop, CONTROL_PLANE_CHUNK_SIZE};
crates/blit-core/src/transfer_session/mod.rs:632:    // AbortOnDrop: an early error return below must abort the receive
crates/blit-core/src/transfer_session/mod.rs:634:    let _recv_guard = AbortOnDrop::new(tokio::spawn(source_recv_half(
crates/blit-core/src/transfer_session/mod.rs:1130:    // AbortOnDrop bounds it to this future: a control-lane fault that
crates/blit-core/src/transfer_session/mod.rs:1134:        AbortOnDrop::new(tokio::spawn(rdp.accept_and_receive(sink)))
crates/blit-core/src/remote/pull.rs:21:use crate::remote::transfer::AbortOnDrop;
crates/blit-core/src/remote/pull.rs:617:        // R32-F2: AbortOnDrop so an outer cancellation aborts the
crates/blit-core/src/remote/pull.rs:623:        let manifest_send_task = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/pull.rs:660:        // R32-F2: wrap the data-plane handle in AbortOnDrop so an
crates/blit-core/src/remote/pull.rs:663:        let mut data_plane_handle: Option<AbortOnDrop<Result<DataPlaneResult>>> = None;
crates/blit-core/src/remote/pull.rs:871:                    data_plane_handle = Some(AbortOnDrop::new(handle));
crates/blit-core/src/remote/pull.rs:1100:        // `.join()` keeps the AbortOnDrop wrapper alive across the
crates/blit-core/src/remote/pull.rs:1729:    // AbortOnDrop set became a JoinSet, which aborts every remaining
crates/blit-core/src/remote/pull.rs:2040:    //! `AbortOnDrop`'s own contract (drop-without-consume aborts,
crates/blit-core/src/remote/pull.rs:2172:        // `pull_sync_with_spec`'s AbortOnDrop data-plane handle does
crates/blit-core/src/remote/pull.rs:2202:        let guard = super::AbortOnDrop::new(tokio::spawn(receive_data_plane_streams_owned(
crates/blit-core/src/remote/push/client/mod.rs:41:use crate::remote::transfer::AbortOnDrop;
crates/blit-core/src/remote/push/client/mod.rs:64:/// w4-1: takes `AbortOnDrop` (not a bare `JoinHandle`) and drains via
crates/blit-core/src/remote/push/client/mod.rs:67:async fn drain_pipeline_outcome(handle: AbortOnDrop<Result<SinkOutcome>>) -> Result<SinkOutcome> {
crates/blit-core/src/remote/push/client/mod.rs:87:async fn drain_pipeline_error(handle: AbortOnDrop<Result<SinkOutcome>>) -> eyre::Report {
crates/blit-core/src/remote/push/client/mod.rs:114:    /// w4-1: `AbortOnDrop`, not a bare `JoinHandle` — if `push()`
crates/blit-core/src/remote/push/client/mod.rs:119:    pipeline_handle: Option<AbortOnDrop<Result<SinkOutcome>>>,
crates/blit-core/src/remote/push/client/mod.rs:297:        let pipeline_handle = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/push/client/mod.rs:1602:    //! needs the `pipeline_handle` field wired through `AbortOnDrop`.
crates/blit-core/src/remote/push/client/mod.rs:1618:        let pipeline_handle = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/push/client/mod.rs:1673:        let handle = AbortOnDrop::new(tokio::spawn(async {
crates/blit-core/src/remote/push/client/mod.rs:1696:        let handle = AbortOnDrop::new(tokio::spawn(async {
crates/blit-core/src/remote/push/client/mod.rs:1721:        let handle = AbortOnDrop::new(tokio::spawn(async move { Ok(cloned) }));
crates/blit-core/src/remote/push/client/mod.rs:1733:        let handle = AbortOnDrop::new(tokio::spawn(async {
crates/blit-core/src/remote/push/client/mod.rs:1752:        let handle = AbortOnDrop::new(tokio::spawn(async {
crates/blit-core/src/remote/push/client/mod.rs:1773:        let handle = AbortOnDrop::new(tokio::spawn(async { Ok(SinkOutcome::default()) }));
crates/blit-core/src/remote/push/client/helpers.rs:17:use crate::remote::transfer::AbortOnDrop;
crates/blit-core/src/remote/push/client/helpers.rs:256:/// w4-1: the returned task is wrapped in `AbortOnDrop` so a caller
crates/blit-core/src/remote/push/client/helpers.rs:265:    AbortOnDrop<()>,
crates/blit-core/src/remote/push/client/helpers.rs:268:    let task = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/transfer/mod.rs:17:pub use abort_on_drop::AbortOnDrop;
crates/blit-core/src/remote/transfer/pipeline.rs:176:                    // cancellation) is the AbortOnDrop family, w4-1.
crates/blit-core/src/remote/transfer/pipeline.rs:2019:    /// AbortOnDrop family tracked under w4-1.
crates/blit-core/src/remote/transfer/abort_on_drop.rs:25:pub struct AbortOnDrop<T>(Option<JoinHandle<T>>);
crates/blit-core/src/remote/transfer/abort_on_drop.rs:27:impl<T> AbortOnDrop<T> {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:46:            .expect("AbortOnDrop already consumed (programming error)");
crates/blit-core/src/remote/transfer/abort_on_drop.rs:56:impl<T> Drop for AbortOnDrop<T> {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:66:    //! Regression tests for the `AbortOnDrop` wrapper that bounds
crates/blit-core/src/remote/transfer/abort_on_drop.rs:73:    use super::AbortOnDrop;
crates/blit-core/src/remote/transfer/abort_on_drop.rs:89:        // wrapping in AbortOnDrop and dropping immediately must
crates/blit-core/src/remote/transfer/abort_on_drop.rs:94:        let guard = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:110:            "task ran to completion despite AbortOnDrop being dropped"
crates/blit-core/src/remote/transfer/abort_on_drop.rs:122:        let guard = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:138:        let guard = AbortOnDrop::new(tokio::spawn(async {}));
crates/blit-core/src/remote/transfer/abort_on_drop.rs:162:        let guard = AbortOnDrop::new(tokio::spawn(async move {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:186:             AbortOnDrop is leaking the handle out before the await again"

exec
/usr/bin/zsh -lc 'rg -n "struct AbortOnDrop|impl.*AbortOnDrop|remote::push|crate::remote::push|blit_daemon|push::|DataPlaneSession::connect|dial_data_plane|execute_sink_pipeline_streaming|execute_receive_pipeline" crates/blit-core/src/remote/transfer crates/blit-core/src/transfer_session/data_plane.rs crates/blit-core/src/transfer_session/mod.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/transfer_session/data_plane.rs:4://! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
crates/blit-core/src/transfer_session/data_plane.rs:5://! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
crates/blit-core/src/transfer_session/data_plane.rs:8://! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
crates/blit-core/src/transfer_session/data_plane.rs:35:use crate::remote::transfer::pipeline::execute_receive_pipeline;
crates/blit-core/src/transfer_session/data_plane.rs:42:    execute_sink_pipeline_streaming, generate_sub_token, AbortOnDrop, DataPlaneSession,
crates/blit-core/src/transfer_session/data_plane.rs:153:            receives.spawn(async move { execute_receive_pipeline(&mut socket, sink, None).await });
crates/blit-core/src/transfer_session/data_plane.rs:243:        let session = DataPlaneSession::connect(
crates/blit-core/src/transfer_session/data_plane.rs:268:        execute_sink_pipeline_streaming(source, sinks, payload_rx, SESSION_DP_PREFETCH, None).await
crates/blit-core/src/remote/transfer/mod.rs:29:    execute_sink_pipeline, execute_sink_pipeline_elastic, execute_sink_pipeline_streaming,
crates/blit-core/src/remote/transfer/stall_guard.rs:60:/// - Daemon push-receive TCP (`daemon::service::push::data_plane`
crates/blit-core/src/remote/transfer/socket.rs:13://! design-3 added [`dial_data_plane`]: the client-side dial (bounded
crates/blit-core/src/remote/transfer/socket.rs:123:pub async fn dial_data_plane(
crates/blit-core/src/remote/transfer/socket.rs:128:    dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/socket.rs:138:/// Timeout-parameterized core of [`dial_data_plane`], so tests can pin
crates/blit-core/src/remote/transfer/socket.rs:140:async fn dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/socket.rs:273:            dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/socket.rs:313:            dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/socket.rs:347:        let result = dial_data_plane_with_timeouts(
crates/blit-core/src/remote/transfer/pipeline.rs:6://! produced ([`execute_sink_pipeline_streaming`]). The one-shot form is a
crates/blit-core/src/remote/transfer/pipeline.rs:21:/// This is a convenience wrapper around [`execute_sink_pipeline_streaming`]
crates/blit-core/src/remote/transfer/pipeline.rs:55:    let result = execute_sink_pipeline_streaming(source, sinks, rx, prefetch, progress).await;
crates/blit-core/src/remote/transfer/pipeline.rs:77:pub async fn execute_sink_pipeline_streaming(
crates/blit-core/src/remote/transfer/pipeline.rs:105:/// `ue-r2-2`: [`execute_sink_pipeline_streaming`] plus a control
crates/blit-core/src/remote/transfer/pipeline.rs:403:/// This is the symmetric counterpart to [`execute_sink_pipeline_streaming`]:
crates/blit-core/src/remote/transfer/pipeline.rs:417:pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
crates/blit-core/src/remote/transfer/pipeline.rs:662:    /// to confirm `execute_sink_pipeline_streaming` returns the
crates/blit-core/src/remote/transfer/pipeline.rs:780:        let outcome = execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None)
crates/blit-core/src/remote/transfer/pipeline.rs:999:            // execute_receive_pipeline takes &mut TcpStream. Use a real
crates/blit-core/src/remote/transfer/pipeline.rs:1021:            let result = execute_receive_pipeline(&mut reader, sink, None).await;
crates/blit-core/src/remote/transfer/pipeline.rs:1170:        let outcome = execute_receive_pipeline(&mut reader, sink, Some(&progress))
crates/blit-core/src/remote/transfer/pipeline.rs:1210:        execute_receive_pipeline(&mut reader, sink, Some(&progress))
crates/blit-core/src/remote/transfer/pipeline.rs:1242:        execute_receive_pipeline(&mut reader, sink, Some(&progress))
crates/blit-core/src/remote/transfer/pipeline.rs:1327:    /// pipeline, `execute_sink_pipeline_streaming` must return the
crates/blit-core/src/remote/transfer/pipeline.rs:1369:            execute_sink_pipeline_streaming(source_clone, vec![failing], payload_rx, 4, None).await
crates/blit-core/src/remote/transfer/pipeline.rs:1418:        let err = execute_receive_pipeline(&mut guarded, sink, None)
crates/blit-core/src/remote/transfer/pipeline.rs:1514:        let outcome = execute_sink_pipeline_streaming(source, vec![fast, slow], rx, 2, None)
crates/blit-core/src/remote/transfer/pipeline.rs:1890:        let result = execute_sink_pipeline_streaming(source, vec![sink], rx, 1, None).await;
crates/blit-core/src/remote/transfer/pipeline.rs:1987:        let outcome = execute_sink_pipeline_streaming(source, vec![a, b], rx, 2, None)
crates/blit-core/src/remote/transfer/pipeline.rs:2060:            execute_sink_pipeline_streaming(source, vec![sink], rx, 2, None),
crates/blit-core/src/remote/transfer/pipeline.rs:2119:            execute_sink_pipeline_streaming(source, vec![err, slow], rx, 2, None),
crates/blit-core/src/remote/transfer/diff_planner.rs:4://! origin's filesystem) and `execute_sink_pipeline_streaming` (which
crates/blit-core/src/remote/transfer/data_plane.rs:149:        let stream = super::socket::dial_data_plane(&addr, token, tcp_buffer_size)
crates/blit-core/src/remote/transfer/abort_on_drop.rs:25:pub struct AbortOnDrop<T>(Option<JoinHandle<T>>);
crates/blit-core/src/remote/transfer/abort_on_drop.rs:27:impl<T> AbortOnDrop<T> {
crates/blit-core/src/remote/transfer/abort_on_drop.rs:56:impl<T> Drop for AbortOnDrop<T> {
crates/blit-core/src/remote/transfer/sink.rs:2083:    /// helper via `execute_receive_pipeline`, so this also closes
crates/blit-core/src/remote/transfer/source.rs:69:        use crate::remote::push::client::helpers::spawn_manifest_task;
crates/blit-core/src/remote/transfer/source.rs:87:        use crate::remote::push::client::helpers::filter_readable_headers;

exec
/usr/bin/zsh -lc "git show --numstat --format='%H%n%s%n%B' 881d412" in /home/michael/dev/Blit
 succeeded in 0ms:
881d412b77512fd1d1e28c5932653b63e2cbf41a
otp-4b-1: TCP data plane onto the session (single stream)
otp-4b-1: TCP data plane onto the session (single stream)

Port the TCP data plane onto the unified transfer session, replacing the
in-stream carrier as the default. The DESTINATION responder binds a
listener, mints session_token + epoch0_sub_token, and grants them in
SessionAccept; the SOURCE initiator dials + authenticates the socket and
sends payloads over the shared DataPlaneSession record framing while the
control stream carries manifest/needs/summary. In-stream stays live as
the requested fallback (--force-grpc-shaped).

Single epoch-0 stream only; the zero-knowledge grant proposal is 1, so
multi-stream is resize-only (otp-4b-2). Session-owned orchestration lives
in transfer_session/data_plane.rs, reusing blit-core primitives
(DataPlaneSession, execute_receive_pipeline, execute_sink_pipeline_
streaming, dial_data_plane) — no call into remote::push or the daemon
push service (those drivers die at otp-10).

A/B parity vs old push over the data plane holds byte-identically.
Suite 1509 -> 1511. [state: skip]

Finding: .review/findings/otp-4b-data-plane.md

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>


186	0	.review/findings/otp-4b-data-plane.md
13	4	crates/blit-core/src/remote/transfer/session_client.rs
339	0	crates/blit-core/src/transfer_session/data_plane.rs
147	21	crates/blit-core/src/transfer_session/mod.rs
7	0	crates/blit-core/tests/transfer_session_roles.rs
45	9	crates/blit-daemon/src/service/transfer_session_e2e.rs

exec
/usr/bin/zsh -lc 'git show --check 881d412' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 881d412b77512fd1d1e28c5932653b63e2cbf41a
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 22:54:28 2026 -0400

    otp-4b-1: TCP data plane onto the session (single stream)
    
    Port the TCP data plane onto the unified transfer session, replacing the
    in-stream carrier as the default. The DESTINATION responder binds a
    listener, mints session_token + epoch0_sub_token, and grants them in
    SessionAccept; the SOURCE initiator dials + authenticates the socket and
    sends payloads over the shared DataPlaneSession record framing while the
    control stream carries manifest/needs/summary. In-stream stays live as
    the requested fallback (--force-grpc-shaped).
    
    Single epoch-0 stream only; the zero-knowledge grant proposal is 1, so
    multi-stream is resize-only (otp-4b-2). Session-owned orchestration lives
    in transfer_session/data_plane.rs, reusing blit-core primitives
    (DataPlaneSession, execute_receive_pipeline, execute_sink_pipeline_
    streaming, dial_data_plane) — no call into remote::push or the daemon
    push service (those drivers die at otp-10).
    
    A/B parity vs old push over the data plane holds byte-identically.
    Suite 1509 -> 1511. [state: skip]
    
    Finding: .review/findings/otp-4b-data-plane.md
    
    Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>


exec
/usr/bin/zsh -lc 'git diff 881d412''^ 881d412 -- crates/blit-core/src/transfer_session/data_plane.rs | rg -n "remote::push|push::|blit-daemon|daemon push|execute_receive_pipeline|execute_sink_pipeline_streaming|DataPlaneSession|dial_data_plane"' in /home/michael/dev/Blit
 succeeded in 0ms:
10:+//! [`DataPlaneSession`] record framing, [`execute_receive_pipeline`],
11:+//! [`execute_sink_pipeline_streaming`], [`dial_data_plane`] — but owns
13:+//! (`blit-daemon` push service) and the multi-stream send driver
14:+//! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
41:+use crate::remote::transfer::pipeline::execute_receive_pipeline;
48:+    execute_sink_pipeline_streaming, generate_sub_token, AbortOnDrop, DataPlaneSession,
159:+            receives.spawn(async move { execute_receive_pipeline(&mut socket, sink, None).await });
249:+        let session = DataPlaneSession::connect(
274:+        execute_sink_pipeline_streaming(source, sinks, payload_rx, SESSION_DP_PREFETCH, None).await

codex
There is a second edge case in the same area: after `SourceDone`, the destination awaits the socket reader before it can even run the count check. I’m checking whether the generic receive pipeline has a read-side stall bound; if not, a missing END can pin the session.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/stall_guard.rs | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! audit-1c / audit-h3: a `StallGuard<R>` `AsyncRead` adapter that turns
     2	//! a *stalled* transfer — no bytes received for `timeout` — into a clean
     3	//! `io::ErrorKind::TimedOut`, while leaving a steadily-progressing
     4	//! transfer untouched.
     5	//!
     6	//! Why an `AsyncRead` adapter and not a `tokio::time::timeout` around the
     7	//! receive call: the receive pipeline reads each wire frame through many
     8	//! separate socket awaits (record tag, file header, length-prefixed
     9	//! fields, file-data streaming, tar shards). A stall can happen at *any*
    10	//! of them, mid-frame. Sitting at the `AsyncRead` layer catches a stall
    11	//! at every read without touching the parsing logic, and — crucially —
    12	//! it is an **idle** timeout (re-armed on every read that makes progress)
    13	//! NOT a total-duration deadline, so a legitimate large transfer that
    14	//! keeps making progress is never aborted. (Owner decision, memory
    15	//! `audit-owner-decisions`: no-bytes-for-30s.)
    16	//!
    17	//! Scope:
    18	//! - audit-1c shipped [`StallGuard`] on the CLI pull-receive TCP path
    19	//!   (the original AsyncRead idle adapter).
    20	//! - audit-h3a extended [`StallGuard`] to the daemon push-receive socket
    21	//!   — another receive path.
    22	//! - audit-h3b adds [`StallGuardWriter`] (this slice), an AsyncWrite
    23	//!   adapter mirroring [`StallGuard`] for **write** progress. The
    24	//!   daemon-side pull data plane is a SENDER (daemon writes bytes to
    25	//!   the puller), so the stall surface is a slow / wedged reader
    26	//!   causing TCP write backpressure on the daemon. `StallGuardWriter`
    27	//!   trips after `TRANSFER_STALL_TIMEOUT` of no successful write
    28	//!   progress, with the same idle-vs-total-deadline semantics as the
    29	//!   read side. The earlier R2/R3 wording for h3b ("daemon pull-data-
    30	//!   plane accepts") was imprecise — the accept + token phases are
    31	//!   already bounded by the shared `DATA_PLANE_ACCEPT_TIMEOUT` /
    32	//!   `DATA_PLANE_TOKEN_TIMEOUT` pair (`remote::transfer::socket`);
    33	//!   the missing guard is daemon pull-data-plane **write progress
    34	//!   after token acceptance**, addressed here by wiring this writer
    35	//!   inside `DataPlaneSession`.
    36	//! - audit-h3c is the gRPC-fallback class, re-scoped 2026-06-05 to a
    37	//!   two-slice contract because message-granular timeouts can't be
    38	//!   reused from `StallGuard`'s byte-level model. **Slice 1 shipped**
    39	//!   (structural frame cap + unified receive helper at
    40	//!   `crates/blit-core/src/remote/transfer/grpc_fallback.rs`); **slice
    41	//!   2 pending** (dynamic progress watchdog + retryable `TimedOut`
    42	//!   error). See that module for details.
    43	
    44	use std::future::Future;
    45	use std::io;
    46	use std::pin::Pin;
    47	use std::task::{Context, Poll};
    48	use std::time::Duration;
    49	
    50	use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    51	use tokio::time::{Instant, Sleep};
    52	
    53	/// Idle/stall timeout applied to every data-plane transfer path: if no
    54	/// data-plane progress (read or write) is observable for this long, the
    55	/// transfer is aborted with `TimedOut` rather than pinning resources
    56	/// forever. Owner-decided 30s.
    57	///
    58	/// Applied by:
    59	/// - CLI pull-receive TCP (`remote::pull` — audit-1c) via [`StallGuard`].
    60	/// - Daemon push-receive TCP (`daemon::service::push::data_plane`
    61	///   — audit-h3a) via [`StallGuard`].
    62	/// - Daemon pull-data-plane **write progress after token acceptance**
    63	///   (`daemon::service::pull_sync` — audit-h3b; the `pull` service died
    64	///   at ue-r2-1h) via [`StallGuardWriter`] inside `DataPlaneSession`.
    65	///   The accept + token phases on those paths are separately bounded by
    66	///   the shared `DATA_PLANE_ACCEPT_TIMEOUT` / `DATA_PLANE_TOKEN_TIMEOUT`
    67	///   pair (`remote::transfer::socket`, w1-4).
    68	///
    69	/// The gRPC-fallback paths sit below `tonic::Streaming<T>` rather than
    70	/// `AsyncRead` / `AsyncWrite` and are covered separately (audit-h3c).
    71	pub const TRANSFER_STALL_TIMEOUT: Duration = Duration::from_secs(30);
    72	
    73	/// Wraps an `AsyncRead` so a read that makes no progress within `timeout`
    74	/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on
    75	/// every read that returns (data or clean EOF), so it is a per-gap idle
    76	/// timeout, not a cap on the whole transfer.
    77	pub struct StallGuard<R> {
    78	    inner: R,
    79	    timeout: Duration,
    80	    deadline: Pin<Box<Sleep>>,
    81	}
    82	
    83	impl<R> StallGuard<R> {
    84	    pub fn new(inner: R, timeout: Duration) -> Self {
    85	        Self {
    86	            inner,
    87	            timeout,
    88	            deadline: Box::pin(tokio::time::sleep(timeout)),
    89	        }
    90	    }
    91	}
    92	
    93	impl<R: AsyncRead + Unpin> AsyncRead for StallGuard<R> {
    94	    fn poll_read(
    95	        self: Pin<&mut Self>,
    96	        cx: &mut Context<'_>,
    97	        buf: &mut ReadBuf<'_>,
    98	    ) -> Poll<io::Result<()>> {
    99	        let this = self.get_mut();
   100	        match Pin::new(&mut this.inner).poll_read(cx, buf) {
   101	            Poll::Ready(Ok(())) => {
   102	                // The read completed (delivered bytes, or a clean EOF) —
   103	                // that's progress, so re-arm the idle deadline.
   104	                this.deadline.as_mut().reset(Instant::now() + this.timeout);
   105	                Poll::Ready(Ok(()))
   106	            }
   107	            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
   108	            Poll::Pending => {
   109	                // No data available yet. Trip only if the whole idle
   110	                // window has elapsed since the last progress; otherwise
   111	                // stay pending (the deadline poll registers our waker).
   112	                match this.deadline.as_mut().poll(cx) {
   113	                    Poll::Ready(()) => Poll::Ready(Err(io::Error::new(
   114	                        io::ErrorKind::TimedOut,
   115	                        format!("transfer stalled: no data received for {:?}", this.timeout),
   116	                    ))),
   117	                    Poll::Pending => Poll::Pending,
   118	                }
   119	            }
   120	        }
   121	    }
   122	}
   123	
   124	/// Wraps an `AsyncWrite` so a write that makes no progress within `timeout`
   125	/// resolves to `io::ErrorKind::TimedOut`. The deadline is re-armed on every
   126	/// successful `poll_write` (any byte count > 0 counts as progress), so it
   127	/// is a per-gap idle timeout, not a cap on the whole transfer.
   128	///
   129	/// audit-h3b: the daemon-side pull data plane writes bytes to the puller.
   130	/// If the puller stops reading mid-stream, TCP flow control fills the
   131	/// kernel send buffer and `write_all` blocks indefinitely (until OS-level
   132	/// TCP retransmit exhaustion, often 15+ minutes). Wrapping the inner
   133	/// stream in this adapter turns that into a clean
   134	/// `io::ErrorKind::TimedOut` after `TRANSFER_STALL_TIMEOUT` of no
   135	/// observable write progress.
   136	///
   137	/// Symmetric in spirit with [`StallGuard`] on the read side: same idle-
   138	/// timeout semantics, same load-bearing property that a steadily-
   139	/// progressing transfer (any non-trivial network at all) is never
   140	/// aborted.
   141	pub struct StallGuardWriter<W> {
   142	    inner: W,
   143	    timeout: Duration,
   144	    deadline: Pin<Box<Sleep>>,
   145	}
   146	
   147	impl<W> StallGuardWriter<W> {
   148	    pub fn new(inner: W, timeout: Duration) -> Self {
   149	        Self {
   150	            inner,
   151	            timeout,
   152	            deadline: Box::pin(tokio::time::sleep(timeout)),
   153	        }
   154	    }
   155	
   156	    pub fn into_inner(self) -> W {
   157	        self.inner
   158	    }
   159	}
   160	
   161	impl<W: AsyncWrite + Unpin> AsyncWrite for StallGuardWriter<W> {
   162	    fn poll_write(
   163	        self: Pin<&mut Self>,
   164	        cx: &mut Context<'_>,
   165	        buf: &[u8],
   166	    ) -> Poll<io::Result<usize>> {
   167	        let this = self.get_mut();
   168	        match Pin::new(&mut this.inner).poll_write(cx, buf) {
   169	            Poll::Ready(Ok(0)) => {
   170	                // Per the doc contract above, "no progress" means zero
   171	                // bytes accepted. A 0-byte poll_write does NOT reset
   172	                // the deadline — otherwise a peer that accepts zero
   173	                // bytes per poll would never trip the guard. The
   174	                // caller (write_all loop) will keep polling; if real
   175	                // progress doesn't show up within the window the
   176	                // Pending arm below trips. (h3b round 2: GPT review
   177	                // flagged Ok(0) as a doc/code mismatch.)
   178	                Poll::Ready(Ok(0))
   179	            }
   180	            Poll::Ready(Ok(n)) => {
   181	                // n > 0: real progress. Reset the idle deadline so a
   182	                // steadily-progressing transfer is never aborted.
   183	                this.deadline.as_mut().reset(Instant::now() + this.timeout);
   184	                Poll::Ready(Ok(n))
   185	            }
   186	            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
   187	            Poll::Pending => {
   188	                // The inner stream is back-pressuring (kernel send
   189	                // buffer full → no kernel space available). Trip only
   190	                // if the whole idle window has elapsed since the last
   191	                // progress; otherwise stay pending (the deadline poll
   192	                // registers our waker).
   193	                match this.deadline.as_mut().poll(cx) {
   194	                    Poll::Ready(()) => Poll::Ready(Err(io::Error::new(
   195	                        io::ErrorKind::TimedOut,
   196	                        format!("transfer stalled: no write progress for {:?}", this.timeout),
   197	                    ))),
   198	                    Poll::Pending => Poll::Pending,
   199	                }
   200	            }
   201	        }
   202	    }
   203	
   204	    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
   205	        // Flush is a no-op for most AsyncWrite impls; we don't gate it
   206	        // on the deadline because a stuck flush manifests as a stuck
   207	        // poll_write upstream, which IS gated. Pass through cleanly.
   208	        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
   209	    }
   210	
   211	    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
   212	        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
   213	    }
   214	}
   215	
   216	#[cfg(test)]
   217	mod tests {
   218	    use super::*;
   219	    use tokio::io::{AsyncReadExt, AsyncWriteExt};
   220	

exec
/usr/bin/zsh -lc 'rg -n "StallGuard|read.*timeout|TRANSFER_STALL_TIMEOUT|read_exact|receive_stream_double_buffered|DATA_PLANE_RECORD_END" crates/blit-core/src/remote/transfer crates/blit-daemon/src/service/push' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/service/push/data_plane.rs:9:use blit_core::remote::transfer::stall_guard::{StallGuard, TRANSFER_STALL_TIMEOUT};
crates/blit-daemon/src/service/push/data_plane.rs:147:    match tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut token_buf)).await {
crates/blit-daemon/src/service/push/data_plane.rs:185:    //   socket → StallGuard → execute_receive_pipeline → FsTransferSink → disk
crates/blit-daemon/src/service/push/data_plane.rs:195:    // bounds the token read). StallGuard turns that into a clean
crates/blit-daemon/src/service/push/data_plane.rs:196:    // TimedOut after TRANSFER_STALL_TIMEOUT of no progress.
crates/blit-daemon/src/service/push/data_plane.rs:440:    match tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await {
crates/blit-daemon/src/service/push/data_plane.rs:1072:/// audit-h3a: wrap the push-receive socket in a `StallGuard` so a peer
crates/blit-daemon/src/service/push/data_plane.rs:1074:/// by `TRANSFER_STALL_TIMEOUT` rather than holding the receive worker
crates/blit-daemon/src/service/push/data_plane.rs:1085:    let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
crates/blit-daemon/src/service/push/data_plane.rs:1113:    /// after the token must abort with the StallGuard's TimedOut, not
crates/blit-daemon/src/service/push/data_plane.rs:1119:    /// with the production `TRANSFER_STALL_TIMEOUT` — so a future
crates/blit-daemon/src/service/push/data_plane.rs:1120:    /// refactor that removes the StallGuard wrap from the daemon's
crates/blit-daemon/src/service/push/data_plane.rs:1142:        // perpetually Pending until the StallGuard fires.
crates/blit-daemon/src/service/push/data_plane.rs:1147:        // StallGuard's sleep wakes; poll_read returns TimedOut;
crates/blit-daemon/src/service/push/data_plane.rs:1149:        tokio::time::advance(TRANSFER_STALL_TIMEOUT + Duration::from_secs(1)).await;
crates/blit-daemon/src/service/push/data_plane.rs:1157:            "expected a StallGuard timeout in the error chain; got: {err:#}"
crates/blit-daemon/src/service/push/data_plane.rs:1481:        // DATA_PLANE_RECORD_END — an incomplete stream, which the
crates/blit-daemon/src/service/push/data_plane.rs:1483:        // next record tag (read_exact sees EOF, not a stall, so this
crates/blit-daemon/src/service/push/data_plane.rs:1484:        // resolves promptly rather than waiting on StallGuard).
crates/blit-core/src/remote/transfer/mod.rs:19:    generate_sub_token, receive_stream_double_buffered, DataPlaneSession, CONTROL_PLANE_CHUNK_SIZE,
crates/blit-core/src/remote/transfer/mod.rs:20:    DATA_PLANE_RECORD_BLOCK, DATA_PLANE_RECORD_BLOCK_COMPLETE, DATA_PLANE_RECORD_END,
crates/blit-core/src/remote/transfer/grpc_fallback.rs:4://! — every `poll_read` event resets the [`super::stall_guard::StallGuard`]
crates/blit-core/src/remote/transfer/grpc_fallback.rs:97:/// stream lets the read-side `StallGuard` observe every successful
crates/blit-core/src/remote/transfer/stall_guard.rs:1://! audit-1c / audit-h3: a `StallGuard<R>` `AsyncRead` adapter that turns
crates/blit-core/src/remote/transfer/stall_guard.rs:18://! - audit-1c shipped [`StallGuard`] on the CLI pull-receive TCP path
crates/blit-core/src/remote/transfer/stall_guard.rs:20://! - audit-h3a extended [`StallGuard`] to the daemon push-receive socket
crates/blit-core/src/remote/transfer/stall_guard.rs:22://! - audit-h3b adds [`StallGuardWriter`] (this slice), an AsyncWrite
crates/blit-core/src/remote/transfer/stall_guard.rs:23://!   adapter mirroring [`StallGuard`] for **write** progress. The
crates/blit-core/src/remote/transfer/stall_guard.rs:26://!   causing TCP write backpressure on the daemon. `StallGuardWriter`
crates/blit-core/src/remote/transfer/stall_guard.rs:27://!   trips after `TRANSFER_STALL_TIMEOUT` of no successful write
crates/blit-core/src/remote/transfer/stall_guard.rs:38://!   reused from `StallGuard`'s byte-level model. **Slice 1 shipped**
crates/blit-core/src/remote/transfer/stall_guard.rs:59:/// - CLI pull-receive TCP (`remote::pull` — audit-1c) via [`StallGuard`].
crates/blit-core/src/remote/transfer/stall_guard.rs:61:///   — audit-h3a) via [`StallGuard`].
crates/blit-core/src/remote/transfer/stall_guard.rs:64:///   at ue-r2-1h) via [`StallGuardWriter`] inside `DataPlaneSession`.
crates/blit-core/src/remote/transfer/stall_guard.rs:71:pub const TRANSFER_STALL_TIMEOUT: Duration = Duration::from_secs(30);
crates/blit-core/src/remote/transfer/stall_guard.rs:73:/// Wraps an `AsyncRead` so a read that makes no progress within `timeout`
crates/blit-core/src/remote/transfer/stall_guard.rs:77:pub struct StallGuard<R> {
crates/blit-core/src/remote/transfer/stall_guard.rs:83:impl<R> StallGuard<R> {
crates/blit-core/src/remote/transfer/stall_guard.rs:93:impl<R: AsyncRead + Unpin> AsyncRead for StallGuard<R> {
crates/blit-core/src/remote/transfer/stall_guard.rs:134:/// `io::ErrorKind::TimedOut` after `TRANSFER_STALL_TIMEOUT` of no
crates/blit-core/src/remote/transfer/stall_guard.rs:137:/// Symmetric in spirit with [`StallGuard`] on the read side: same idle-
crates/blit-core/src/remote/transfer/stall_guard.rs:141:pub struct StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:147:impl<W> StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:161:impl<W: AsyncWrite + Unpin> AsyncWrite for StallGuardWriter<W> {
crates/blit-core/src/remote/transfer/stall_guard.rs:226:        let mut guard = StallGuard::new(rx, Duration::from_millis(20));
crates/blit-core/src/remote/transfer/stall_guard.rs:242:        let mut guard = StallGuard::new(rx, Duration::from_secs(5));
crates/blit-core/src/remote/transfer/stall_guard.rs:245:            .read_exact(&mut buf)
crates/blit-core/src/remote/transfer/stall_guard.rs:266:        let mut guard = StallGuard::new(rx, Duration::from_millis(50));
crates/blit-core/src/remote/transfer/stall_guard.rs:276:    // ----- audit-h3b: write-side StallGuardWriter tests -----
crates/blit-core/src/remote/transfer/stall_guard.rs:283:    /// Pending, and the StallGuardWriter's idle deadline trips.
crates/blit-core/src/remote/transfer/stall_guard.rs:289:            (StallGuardWriter::new(tx, Duration::from_millis(20)), rx)
crates/blit-core/src/remote/transfer/stall_guard.rs:294:        // StallGuardWriter must surface a TimedOut error from inside
crates/blit-core/src/remote/transfer/stall_guard.rs:304:    /// so the StallGuardWriter must not trip on a fast healthy
crates/blit-core/src/remote/transfer/stall_guard.rs:314:        let mut guarded = StallGuardWriter::new(tx, Duration::from_secs(5));
crates/blit-core/src/remote/transfer/stall_guard.rs:338:        let mut guarded = StallGuardWriter::new(tx, Duration::from_millis(50));
crates/blit-core/src/remote/transfer/data_plane.rs:11:use super::stall_guard::{StallGuardWriter, TRANSFER_STALL_TIMEOUT};
crates/blit-core/src/remote/transfer/data_plane.rs:20:pub const DATA_PLANE_RECORD_END: u8 = 0xFF;
crates/blit-core/src/remote/transfer/data_plane.rs:51:/// audit-h3b: writes go through [`StallGuardWriter`] so a stalled
crates/blit-core/src/remote/transfer/data_plane.rs:53:/// [`TRANSFER_STALL_TIMEOUT`] of no observable write progress instead
crates/blit-core/src/remote/transfer/data_plane.rs:56:/// sites compose against the `AsyncWrite` impl of `StallGuardWriter`,
crates/blit-core/src/remote/transfer/data_plane.rs:59:    stream: StallGuardWriter<TcpStream>,
crates/blit-core/src/remote/transfer/data_plane.rs:81:    /// [`StallGuardWriter`] (inside `from_stream_with_probe`) so a
crates/blit-core/src/remote/transfer/data_plane.rs:82:    /// stalled peer trips after [`TRANSFER_STALL_TIMEOUT`] of no
crates/blit-core/src/remote/transfer/data_plane.rs:176:            stream: StallGuardWriter::new(stream, TRANSFER_STALL_TIMEOUT),
crates/blit-core/src/remote/transfer/data_plane.rs:238:            .write_all(&[DATA_PLANE_RECORD_END])
crates/blit-core/src/remote/transfer/data_plane.rs:676:pub async fn receive_stream_double_buffered<R, W>(
crates/blit-core/src/remote/transfer/data_plane.rs:775:            receive_stream_double_buffered(&mut src, &mut dst, payload.len() as u64, 1024, None)
crates/blit-core/src/remote/transfer/data_plane.rs:793:        let n = receive_stream_double_buffered(
crates/blit-core/src/remote/transfer/data_plane.rs:854:        receive_stream_double_buffered(
crates/blit-core/src/remote/transfer/socket.rs:37:/// R46-F7: pre-fix `read_exact(&mut token_buf).await` had no timeout —
crates/blit-core/src/remote/transfer/socket.rs:50:/// stalled peer with data in flight — is StallGuard's 30 s, not
crates/blit-core/src/remote/transfer/socket.rs:290:        tokio::io::AsyncReadExt::read_exact(&mut server, &mut buf)
crates/blit-core/src/remote/transfer/progress.rs:91:/// (`receive_stream_double_buffered`, &c.) can take an
crates/blit-core/src/remote/transfer/sink.rs:134:    /// `receive_stream_double_buffered` so chunk-granularity
crates/blit-core/src/remote/transfer/sink.rs:301:        // granular `receive_stream_double_buffered` hook never
crates/blit-core/src/remote/transfer/sink.rs:325:            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
crates/blit-core/src/remote/transfer/sink.rs:351:            receive_stream_double_buffered(
crates/blit-core/src/remote/transfer/sink.rs:377:            receive_stream_double_buffered(
crates/blit-core/src/remote/transfer/sink.rs:890:            receive_stream_double_buffered, RECEIVE_CHUNK_SIZE,
crates/blit-core/src/remote/transfer/sink.rs:897:        let n = receive_stream_double_buffered(
crates/blit-core/src/remote/transfer/sink.rs:2209:    /// inside `receive_stream_double_buffered` never fires for them.
crates/blit-core/src/remote/transfer/pipeline.rs:397:    DATA_PLANE_RECORD_BLOCK, DATA_PLANE_RECORD_BLOCK_COMPLETE, DATA_PLANE_RECORD_END,
crates/blit-core/src/remote/transfer/pipeline.rs:414:/// which uses the same `receive_stream_double_buffered` helper as the
crates/blit-core/src/remote/transfer/pipeline.rs:427:            .read_exact(&mut tag)
crates/blit-core/src/remote/transfer/pipeline.rs:432:            DATA_PLANE_RECORD_END => break,
crates/blit-core/src/remote/transfer/pipeline.rs:490:                    .read_exact(&mut bytes)
crates/blit-core/src/remote/transfer/pipeline.rs:538:    socket.read_exact(&mut buf).await.context("reading u32")?;
crates/blit-core/src/remote/transfer/pipeline.rs:544:    socket.read_exact(&mut buf).await.context("reading u64")?;
crates/blit-core/src/remote/transfer/pipeline.rs:550:    socket.read_exact(&mut buf).await.context("reading i64")?;
crates/blit-core/src/remote/transfer/pipeline.rs:584:        .read_exact(&mut buf)
crates/blit-core/src/remote/transfer/pipeline.rs:644:        .read_exact(&mut data)
crates/blit-core/src/remote/transfer/pipeline.rs:873:            ("empty stream (immediate END)", vec![DATA_PLANE_RECORD_END]),
crates/blit-core/src/remote/transfer/pipeline.rs:971:            ("unknown record tag", vec![0xAB, DATA_PLANE_RECORD_END]),
crates/blit-core/src/remote/transfer/pipeline.rs:1166:        wire.push(DATA_PLANE_RECORD_END);
crates/blit-core/src/remote/transfer/pipeline.rs:1206:        wire.push(DATA_PLANE_RECORD_END);
crates/blit-core/src/remote/transfer/pipeline.rs:1238:        wire.push(DATA_PLANE_RECORD_END);
crates/blit-core/src/remote/transfer/pipeline.rs:1391:    /// StallGuard's TimedOut rather than blocking forever. A duplex whose
crates/blit-core/src/remote/transfer/pipeline.rs:1393:    /// tag read perpetually Pending; the StallGuard wrapping it trips
crates/blit-core/src/remote/transfer/pipeline.rs:1397:        use crate::remote::transfer::stall_guard::StallGuard;
crates/blit-core/src/remote/transfer/pipeline.rs:1416:        let mut guarded = StallGuard::new(rx, std::time::Duration::from_millis(20));
crates/blit-core/src/remote/transfer/pipeline.rs:1423:            "expected a StallGuard timeout in the error chain; got: {err:#}"

exec
/usr/bin/zsh -lc "rg -n \"execute_receive_pipeline|receive_data_plane|accept.*data|DATA_PLANE_RECORD_END|SourceDone|UploadComplete|finish\" crates/blit-daemon/src/service/push crates/blit-core/src/remote/push/client -g '*.rs'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/remote/push/client/mod.rs:45:/// (via `drain_pipeline_error`) and `MultiStreamSender::finish` so
crates/blit-core/src/remote/push/client/mod.rs:60:/// Closes R43 follow-up to R42-F2: previously `finish()` duplicated
crates/blit-core/src/remote/push/client/mod.rs:104:    /// dial. Aborted on finish(); self-terminates via its Weak<dial>
crates/blit-core/src/remote/push/client/mod.rs:105:    /// if the sender is dropped without finishing.
crates/blit-core/src/remote/push/client/mod.rs:364:            // Pipeline already finished (transfer completing under the
crates/blit-core/src/remote/push/client/mod.rs:370:                let _ = sink.finish().await;
crates/blit-core/src/remote/push/client/mod.rs:411:            .ok_or_else(|| eyre!("data plane sender already finished"))?;
crates/blit-core/src/remote/push/client/mod.rs:432:    async fn finish(mut self) -> Result<()> {
crates/blit-core/src/remote/push/client/mod.rs:762:        // Don't finish the data plane until a full iteration passes with
crates/blit-core/src/remote/push/client/mod.rs:767:        // finish() so we don't close the data plane while the daemon
crates/blit-core/src/remote/push/client/mod.rs:788:                                        // early-finish check can fire on this iteration;
crates/blit-core/src/remote/push/client/mod.rs:985:                                            sender.finish().await?;
crates/blit-core/src/remote/push/client/mod.rs:1351:                            // guarantees the manifest task has finished
crates/blit-core/src/remote/push/client/mod.rs:1461:                // Send UploadComplete via a temporary GrpcFallbackSink.
crates/blit-core/src/remote/push/client/mod.rs:1462:                let finish_sink = GrpcFallbackSink::new(
crates/blit-core/src/remote/push/client/mod.rs:1468:                finish_sink.finish().await?;
crates/blit-core/src/remote/push/client/mod.rs:1481:                    sender.finish().await?;
crates/blit-core/src/remote/push/client/mod.rs:1491:            sender.finish().await?;
crates/blit-core/src/remote/push/client/mod.rs:1604:    //! `MultiStreamSender` without calling `.finish()` (the path
crates/blit-core/src/remote/push/client/mod.rs:1615:    async fn dropping_sender_without_finish_aborts_pipeline_task() {
crates/blit-core/src/remote/push/client/mod.rs:1634:        // still live: drop it without calling finish().
crates/blit-core/src/remote/push/client/mod.rs:1654:    //! `MultiStreamSender::queue` and `MultiStreamSender::finish`
crates/blit-core/src/remote/push/client/mod.rs:1712:    async fn drain_outcome_returns_value_on_clean_finish() {
crates/blit-core/src/remote/push/client/mod.rs:1714:        // helper passes it through. `finish()` relies on this to
crates/blit-core/src/remote/push/client/mod.rs:1722:        let got = drain_pipeline_outcome(handle).await.expect("clean finish");
crates/blit-core/src/remote/push/client/mod.rs:1729:        // `finish()` failure path: pipeline returned Err. The helper
crates/blit-core/src/remote/push/client/mod.rs:1753:            panic!("synthetic finish-time panic");
crates/blit-core/src/remote/push/client/mod.rs:1769:        // pipeline was about to (or had just) finished cleanly. We
crates/blit-core/src/remote/push/client/types.rs:13:    /// finished (`None` on the gRPC fallback path — no data plane).
crates/blit-daemon/src/service/push/data_plane.rs:7:use blit_core::remote::transfer::pipeline::execute_receive_pipeline;
crates/blit-daemon/src/service/push/data_plane.rs:67:pub(crate) async fn accept_data_connection_stream(
crates/blit-daemon/src/service/push/data_plane.rs:78:    // Mirrors `accept_data_connection_stream_resizable`, which fixed
crates/blit-daemon/src/service/push/data_plane.rs:170:/// accept paths (`ue-r2-2` split it out of `handle_data_plane_stream`
crates/blit-daemon/src/service/push/data_plane.rs:185:    //   socket → StallGuard → execute_receive_pipeline → FsTransferSink → disk
crates/blit-daemon/src/service/push/data_plane.rs:193:    // accepted the data plane, sent the token, then went silent would
crates/blit-daemon/src/service/push/data_plane.rs:266:/// [`accept_data_connection_stream`]. Epoch 0 behaves exactly like the
crates/blit-daemon/src/service/push/data_plane.rs:273:/// worker — initial and added — has finished.
crates/blit-daemon/src/service/push/data_plane.rs:274:pub(crate) async fn accept_data_connection_stream_resizable(
crates/blit-daemon/src/service/push/data_plane.rs:578:    // R8-F2: stream EOF without explicit UploadComplete is a wire
crates/blit-daemon/src/service/push/data_plane.rs:783:            Some(client_push_request::Payload::UploadComplete(_)) => {
crates/blit-daemon/src/service/push/data_plane.rs:801:    tar_executor.finish(&mut stats).await?;
crates/blit-daemon/src/service/push/data_plane.rs:803:    // R8-F2: stream EOF without explicit UploadComplete is a wire
crates/blit-daemon/src/service/push/data_plane.rs:815:            "fallback stream ended without UploadComplete",
crates/blit-daemon/src/service/push/data_plane.rs:961:    async fn finish(mut self, stats: &mut TransferStats) -> Result<(), Status> {
crates/blit-daemon/src/service/push/data_plane.rs:1073:/// that accepts the data plane and then stops sending bytes is reaped
crates/blit-daemon/src/service/push/data_plane.rs:1086:    execute_receive_pipeline(&mut guarded, sink, None).await
crates/blit-daemon/src/service/push/data_plane.rs:1148:        // execute_receive_pipeline surfaces it as an Err.
crates/blit-daemon/src/service/push/data_plane.rs:1238:        builder.finish().expect("finish tar shard");
crates/blit-daemon/src/service/push/data_plane.rs:1364:    // synthetic message stream so the EOF-without-UploadComplete
crates/blit-daemon/src/service/push/data_plane.rs:1384:        // the stream without sending FileData or UploadComplete. Pre
crates/blit-daemon/src/service/push/data_plane.rs:1405:            msg.contains("UploadComplete") || msg.contains("in-flight"),
crates/blit-daemon/src/service/push/data_plane.rs:1406:            "expected UploadComplete/in-flight error, got: {msg}"
crates/blit-daemon/src/service/push/data_plane.rs:1413:        // any chunks or TarShardComplete or UploadComplete.
crates/blit-daemon/src/service/push/data_plane.rs:1436:            msg.contains("UploadComplete") || msg.contains("in-flight"),
crates/blit-daemon/src/service/push/data_plane.rs:1437:            "expected UploadComplete/in-flight error, got: {msg}"
crates/blit-daemon/src/service/push/data_plane.rs:1444:        // that never carries an UploadComplete must still be
crates/blit-daemon/src/service/push/data_plane.rs:1452:            .expect_err("EOF without UploadComplete must be rejected");
crates/blit-daemon/src/service/push/data_plane.rs:1453:        assert!(err.message().contains("UploadComplete"));
crates/blit-daemon/src/service/push/data_plane.rs:1457:    /// `accept_data_connection_stream`'s per-stream workers used to be
crates/blit-daemon/src/service/push/data_plane.rs:1478:        let accept_fut = accept_data_connection_stream(listener, token.clone(), module, 2);
crates/blit-daemon/src/service/push/data_plane.rs:1481:        // DATA_PLANE_RECORD_END — an incomplete stream, which the
crates/blit-daemon/src/service/push/data_plane.rs:1511:            .expect("accept_data_connection_stream must not hang");
crates/blit-daemon/src/service/push/control.rs:7:    accept_data_connection_stream, accept_data_connection_stream_resizable,
crates/blit-daemon/src/service/push/control.rs:285:                                    accept_data_connection_stream_resizable(
crates/blit-daemon/src/service/push/control.rs:295:                                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream(
crates/blit-daemon/src/service/push/control.rs:333:                // manifest phase — `need_list_sender.finish()` below and
crates/blit-daemon/src/service/push/control.rs:359:            Some(client_push_request::Payload::UploadComplete(_)) => {}
crates/blit-daemon/src/service/push/control.rs:377:    need_list_sender.finish().await?;
crates/blit-daemon/src/service/push/control.rs:410:                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream_resizable(
crates/blit-daemon/src/service/push/control.rs:419:                AbortOnDrop::new(tokio::spawn(accept_data_connection_stream(
crates/blit-daemon/src/service/push/control.rs:553:/// acceptor that already finished.
crates/blit-daemon/src/service/push/control.rs:655:    async fn finish(mut self) -> Result<(), Status> {
crates/blit-daemon/src/service/push/control.rs:662:        // finish condition (looks complete) and the daemon still

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/data_plane.rs | sed -n '170,205p;1070,1090p;1108,1160p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   170	/// accept paths (`ue-r2-2` split it out of `handle_data_plane_stream`
   171	/// so an epoch-N socket runs the identical byte path after its
   172	/// stronger handshake).
   173	async fn receive_stream_into_module(
   174	    socket: TcpStream,
   175	    module: ModuleConfig,
   176	    start: Instant,
   177	) -> Result<TransferStats, Status> {
   178	    eprintln!(
   179	        "blitd: push data plane: token accepted (module='{}', root={})",
   180	        module.name,
   181	        module.path.display()
   182	    );
   183	
   184	    // Route the inbound wire through the unified receive pipeline:
   185	    //   socket → StallGuard → execute_receive_pipeline → FsTransferSink → disk
   186	    // Same call shape as the client's pull-receive side. Tar shards get
   187	    // extracted inline by FsTransferSink (parallelism across streams
   188	    // already comes from N concurrent invocations of this function).
   189	    //
   190	    // audit-h3a (R2/R3 finding H3): symmetric to the audit-1c CLI
   191	    // pull-receive guard. Before this slice the push-receive socket had
   192	    // no idle deadline at all — a hostile or wedged push client that
   193	    // accepted the data plane, sent the token, then went silent would
   194	    // pin this worker indefinitely (DATA_PLANE_TOKEN_TIMEOUT above only
   195	    // bounds the token read). StallGuard turns that into a clean
   196	    // TimedOut after TRANSFER_STALL_TIMEOUT of no progress.
   197	    use blit_core::remote::transfer::sink::{FsSinkConfig, FsTransferSink};
   198	
   199	    let config = FsSinkConfig {
   200	        preserve_times: true,
   201	        dry_run: false,
   202	        checksum: None,
   203	        resume: false,
   204	        compare_mode: blit_core::generated::ComparisonMode::SizeMtime,
   205	    };
  1070	}
  1071	
  1072	/// audit-h3a: wrap the push-receive socket in a `StallGuard` so a peer
  1073	/// that accepts the data plane and then stops sending bytes is reaped
  1074	/// by `TRANSFER_STALL_TIMEOUT` rather than holding the receive worker
  1075	/// open forever. Symmetric with the CLI pull-receive guard in
  1076	/// `blit_core::remote::pull` (audit-1c).
  1077	///
  1078	/// Extracted from `handle_data_plane_stream` so the wiring is unit-
  1079	/// testable without spinning up a TcpListener + token handshake — see
  1080	/// `receive_push_data_plane_aborts_on_stall` in the tests module.
  1081	async fn receive_push_data_plane<R: AsyncRead + Unpin + Send>(
  1082	    socket: R,
  1083	    sink: Arc<dyn TransferSink>,
  1084	) -> Result<SinkOutcome> {
  1085	    let mut guarded = StallGuard::new(socket, TRANSFER_STALL_TIMEOUT);
  1086	    execute_receive_pipeline(&mut guarded, sink, None).await
  1087	}
  1088	
  1089	#[cfg(test)]
  1090	mod tests {
  1108	        assert_eq!(b.len(), TOKEN_LEN);
  1109	        assert_ne!(a, b, "successive tokens must differ");
  1110	    }
  1111	
  1112	    /// audit-h3a: a push-receive socket whose peer stops sending bytes
  1113	    /// after the token must abort with the StallGuard's TimedOut, not
  1114	    /// block the worker forever. The wire-level guard is unit-tested
  1115	    /// in `blit_core::remote::transfer::stall_guard::tests` and the
  1116	    /// pipeline integration is covered by
  1117	    /// `pipeline::receive_pipeline_aborts_on_stall`; this test pins
  1118	    /// that `receive_push_data_plane` itself actually composes the two
  1119	    /// with the production `TRANSFER_STALL_TIMEOUT` — so a future
  1120	    /// refactor that removes the StallGuard wrap from the daemon's
  1121	    /// push-receive helper is caught here instead of regressing the
  1122	    /// DoS surface silently.
  1123	    ///
  1124	    /// Virtual-time pause lets the test exercise the real 30 s timeout
  1125	    /// without waiting 30 wall-clock seconds.
  1126	    #[tokio::test(start_paused = true)]
  1127	    async fn receive_push_data_plane_aborts_on_stall() {
  1128	        let tmp = tempdir().expect("dest tempdir");
  1129	        let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
  1130	            PathBuf::from("/nonexistent-src"),
  1131	            tmp.path().to_path_buf(),
  1132	            FsSinkConfig {
  1133	                preserve_times: false,
  1134	                dry_run: false,
  1135	                checksum: None,
  1136	                resume: false,
  1137	                compare_mode: blit_core::generated::ComparisonMode::SizeMtime,
  1138	            },
  1139	        ));
  1140	
  1141	        // Writer half held open but never written — the read side is
  1142	        // perpetually Pending until the StallGuard fires.
  1143	        let (rx, _tx) = tokio::io::duplex(64);
  1144	        let receive = tokio::spawn(receive_push_data_plane(rx, sink));
  1145	
  1146	        // Advance virtual time past the production idle window. The
  1147	        // StallGuard's sleep wakes; poll_read returns TimedOut;
  1148	        // execute_receive_pipeline surfaces it as an Err.
  1149	        tokio::time::advance(TRANSFER_STALL_TIMEOUT + Duration::from_secs(1)).await;
  1150	
  1151	        let err = receive
  1152	            .await
  1153	            .expect("receive task panicked")
  1154	            .expect_err("a stalled push-receive must abort, not hang");
  1155	        assert!(
  1156	            format!("{err:#}").contains("stalled"),
  1157	            "expected a StallGuard timeout in the error chain; got: {err:#}"
  1158	        );
  1159	    }
  1160	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/data_plane.rs | sed -n '199,240p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   199	    let config = FsSinkConfig {
   200	        preserve_times: true,
   201	        dry_run: false,
   202	        checksum: None,
   203	        resume: false,
   204	        compare_mode: blit_core::generated::ComparisonMode::SizeMtime,
   205	    };
   206	    let sink: Arc<dyn TransferSink> = Arc::new(FsTransferSink::new(
   207	        PathBuf::new(),
   208	        module.path.clone(),
   209	        config,
   210	    ));
   211	    let outcome = receive_push_data_plane(socket, sink)
   212	        .await
   213	        .map_err(|err| Status::internal(format!("data plane receive: {err:#}")))?;
   214	
   215	    let stats = TransferStats {
   216	        files_transferred: outcome.files_written as u64,
   217	        bytes_transferred: outcome.bytes_written,
   218	        bytes_zero_copy: 0,
   219	    };
   220	
   221	    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
   222	    let gbps = (stats.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
   223	    eprintln!(
   224	        "blitd: push data plane: stream complete: files={}, bytes={} ({:.2} Gbps)",
   225	        stats.files_transferred, stats.bytes_transferred, gbps
   226	    );
   227	    Ok(stats)
   228	}
   229	
   230	// ── ue-r2-2: resizable accept (mid-transfer stream ADD) ──────────────
   231	
   232	/// A control-loop → acceptor registration: the credential the next
   233	/// epoch-N socket must present. Sent BEFORE the daemon acks the ADD,
   234	/// so the accept is armed by the time the client dials.
   235	pub(crate) struct ResizeArm {
   236	    pub(crate) epoch: u32,
   237	    pub(crate) sub_token: Vec<u8>,
   238	}
   239	
   240	struct ArmedEpoch {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/source.rs | sed -n '1,115p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use std::path::{Path, PathBuf};
     2	use std::sync::{Arc, Mutex};
     3	
     4	use async_trait::async_trait;
     5	use eyre::{bail, Result};
     6	use tokio::fs;
     7	use tokio::sync::mpsc;
     8	
     9	use crate::fs_enum::FileFilter;
    10	use crate::generated::FileHeader;
    11	use crate::remote::pull::RemotePullClient;
    12	use crate::remote::transfer::payload::{PreparedPayload, TransferPayload};
    13	use tokio::io::AsyncReadExt;
    14	
    15	#[async_trait]
    16	pub trait TransferSource: Send + Sync {
    17	    /// Scans the source and streams discovered file headers.
    18	    /// Returns a receiver for the headers and a join handle for the scan task.
    19	    fn scan(
    20	        &self,
    21	        filter: Option<FileFilter>,
    22	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    23	    ) -> (
    24	        mpsc::Receiver<FileHeader>,
    25	        tokio::task::JoinHandle<Result<u64>>,
    26	    );
    27	
    28	    /// Prepares a payload for transfer (e.g. opens a file or builds a tar shard).
    29	    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload>;
    30	
    31	    /// Checks if the files in the headers are available for transfer.
    32	    /// Returns a list of available headers.
    33	    async fn check_availability(
    34	        &self,
    35	        headers: Vec<FileHeader>,
    36	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    37	    ) -> Result<Vec<FileHeader>>;
    38	
    39	    /// Opens a file for reading.
    40	    async fn open_file(
    41	        &self,
    42	        header: &FileHeader,
    43	    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>>;
    44	
    45	    /// Returns the root path of the source (if applicable).
    46	    fn root(&self) -> &Path;
    47	}
    48	
    49	pub struct FsTransferSource {
    50	    root: PathBuf,
    51	}
    52	
    53	impl FsTransferSource {
    54	    pub fn new(root: PathBuf) -> Self {
    55	        Self { root }
    56	    }
    57	}
    58	
    59	#[async_trait]
    60	impl TransferSource for FsTransferSource {
    61	    fn scan(
    62	        &self,
    63	        filter: Option<FileFilter>,
    64	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    65	    ) -> (
    66	        mpsc::Receiver<FileHeader>,
    67	        tokio::task::JoinHandle<Result<u64>>,
    68	    ) {
    69	        use crate::remote::push::client::helpers::spawn_manifest_task;
    70	        spawn_manifest_task(
    71	            self.root.clone(),
    72	            filter.unwrap_or_default(),
    73	            unreadable_paths,
    74	        )
    75	    }
    76	
    77	    async fn prepare_payload(&self, payload: TransferPayload) -> Result<PreparedPayload> {
    78	        use crate::remote::transfer::payload::prepare_payload;
    79	        prepare_payload(payload, self.root.clone()).await
    80	    }
    81	
    82	    async fn check_availability(
    83	        &self,
    84	        headers: Vec<FileHeader>,
    85	        unreadable_paths: Arc<Mutex<Vec<String>>>,
    86	    ) -> Result<Vec<FileHeader>> {
    87	        use crate::remote::push::client::helpers::filter_readable_headers;
    88	        filter_readable_headers(&self.root, headers, &unreadable_paths).await
    89	    }
    90	
    91	    async fn open_file(
    92	        &self,
    93	        header: &FileHeader,
    94	    ) -> Result<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
    95	        // An empty relative_path means "the root itself" — used when the
    96	        // enumeration root is a single file. Don't join, because
    97	        // PathBuf::join with some Path representations can produce a
    98	        // trailing-slash form that OS interprets as "descend into" and
    99	        // fails with ENOTDIR when the root is a regular file.
   100	        let path = if header.relative_path.is_empty() {
   101	            self.root.clone()
   102	        } else {
   103	            self.root.join(&header.relative_path)
   104	        };
   105	        let file = fs::File::open(&path).await?;
   106	        Ok(Box::new(file))
   107	    }
   108	
   109	    fn root(&self) -> &Path {
   110	        &self.root
   111	    }
   112	}
   113	
   114	/// Validate `FileHeader` sizes for a remote-source tar shard against
   115	/// the shared cap. Both the per-entry size and the cumulative shard

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/service/push/data_plane.rs | sed -n '60,170p;400,455p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    60	    let mut buf = vec![0u8; TOKEN_LEN];
    61	    SysRng
    62	        .try_fill_bytes(&mut buf)
    63	        .map_err(|err| Status::internal(format!("system RNG unavailable: {err}")))?;
    64	    Ok(buf)
    65	}
    66	
    67	pub(crate) async fn accept_data_connection_stream(
    68	    listener: TcpListener,
    69	    expected_token: Vec<u8>,
    70	    module: ModuleConfig,
    71	    stream_count: u32,
    72	) -> Result<TransferStats, Status> {
    73	    let start = Instant::now();
    74	    let streams = stream_count.max(1) as usize;
    75	    // w4-1: a JoinSet, not a Vec<JoinHandle> — dropping a JoinSet
    76	    // aborts every remaining worker, so a first-error return (or this
    77	    // whole future being cancelled) no longer detaches the survivors.
    78	    // Mirrors `accept_data_connection_stream_resizable`, which fixed
    79	    // this same class during ue-r2-2.
    80	    let mut join_set: JoinSet<Result<TransferStats, Status>> = JoinSet::new();
    81	
    82	    for idx in 0..streams {
    83	        let (accepted, addr) =
    84	            match tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await {
    85	                Ok(Ok(pair)) => pair,
    86	                Ok(Err(err)) => {
    87	                    return Err(Status::internal(format!(
    88	                        "data plane accept failed: {}",
    89	                        err
    90	                    )));
    91	                }
    92	                Err(_elapsed) => {
    93	                    return Err(Status::deadline_exceeded(format!(
    94	                        "data plane accept timed out after {:?} waiting for stream {}/{}",
    95	                        DATA_PLANE_ACCEPT_TIMEOUT,
    96	                        idx + 1,
    97	                        streams
    98	                    )));
    99	                }
   100	            };
   101	        // Enable nodelay + keepalive to prevent idle stream timeouts
   102	        // during long transfers on other streams. No tuned buffer:
   103	        // the daemon is the byte receiver here and holds no dial.
   104	        configure_data_socket(&accepted, None)
   105	            .map_err(|err| Status::internal(format!("configuring data socket: {err}")))?;
   106	        let socket = accepted;
   107	        eprintln!(
   108	            "blitd: push data plane: accepted connection {} from {}",
   109	            idx, addr
   110	        );
   111	        let expected_token = expected_token.clone();
   112	        let module_clone = module.clone();
   113	        join_set.spawn(async move {
   114	            handle_data_plane_stream(socket, expected_token, module_clone).await
   115	        });
   116	    }
   117	
   118	    let mut final_stats = TransferStats::default();
   119	    while let Some(joined) = join_set.join_next().await {
   120	        match joined {
   121	            Ok(Ok(stats)) => accumulate_transfer_stats(&mut final_stats, &stats),
   122	            Ok(Err(status)) => return Err(status),
   123	            Err(_) => return Err(Status::internal("data plane worker cancelled")),
   124	        }
   125	    }
   126	
   127	    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
   128	    let gbps = (final_stats.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
   129	    eprintln!(
   130	        "blitd: push data plane: aggregate throughput {:.2} Gbps ({} bytes in {:.2}s)",
   131	        gbps, final_stats.bytes_transferred, elapsed
   132	    );
   133	
   134	    Ok(final_stats)
   135	}
   136	
   137	async fn handle_data_plane_stream(
   138	    mut socket: TcpStream,
   139	    expected_token: Vec<u8>,
   140	    module: ModuleConfig,
   141	) -> Result<TransferStats, Status> {
   142	    let start = Instant::now();
   143	    let mut token_buf = vec![0u8; expected_token.len()];
   144	    // R46-F7: bounded wait on the token. A stalled peer that
   145	    // accepted the socket but never sent bytes would otherwise hold
   146	    // this worker indefinitely.
   147	    match tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut token_buf)).await {
   148	        Ok(Ok(_)) => {}
   149	        Ok(Err(err)) => {
   150	            return Err(Status::internal(format!(
   151	                "failed to read data plane token: {}",
   152	                err
   153	            )));
   154	        }
   155	        Err(_elapsed) => {
   156	            return Err(Status::deadline_exceeded(format!(
   157	                "data plane token read timed out after {:?}",
   158	                DATA_PLANE_TOKEN_TIMEOUT
   159	            )));
   160	        }
   161	    }
   162	    if token_buf != expected_token {
   163	        log::warn!("push data plane: invalid token");
   164	        return Err(Status::permission_denied("invalid data plane token"));
   165	    }
   166	    receive_stream_into_module(socket, module, start).await
   167	}
   168	
   169	/// The per-socket receive tail shared by the fixed and resizable
   170	/// accept paths (`ue-r2-2` split it out of `handle_data_plane_stream`
   400	                },
   401	                Err(err) => {
   402	                    log::warn!("push data plane: resize accept failed: {err}");
   403	                }
   404	            },
   405	        }
   406	    }
   407	
   408	    let elapsed = start.elapsed().as_secs_f64().max(1e-6);
   409	    let gbps = (total.bytes_transferred as f64 * 8.0) / elapsed / 1e9;
   410	    eprintln!(
   411	        "blitd: push data plane: aggregate throughput {:.2} Gbps ({} bytes in {:.2}s)",
   412	        gbps, total.bytes_transferred, elapsed
   413	    );
   414	    Ok(total)
   415	}
   416	
   417	/// Per-socket worker on the resizable path: 48-byte handshake
   418	/// (one-time token ‖ sub-token), then the shared receive tail.
   419	/// `Ok(None)` = socket dropped non-fatally (bad epoch-N credential);
   420	/// post-handshake receive errors are always fatal — an authorized
   421	/// live stream dying mid-transfer is data loss on any epoch.
   422	async fn handle_resizable_stream(
   423	    mut socket: TcpStream,
   424	    expected_token: Vec<u8>,
   425	    credential: StreamCredential,
   426	    module: ModuleConfig,
   427	) -> Result<Option<TransferStats>, Status> {
   428	    fn refuse(fatal: bool, status: Status) -> Result<Option<TransferStats>, Status> {
   429	        if fatal {
   430	            Err(status)
   431	        } else {
   432	            log::warn!("push data plane: dropping resize socket: {status}");
   433	            Ok(None)
   434	        }
   435	    }
   436	
   437	    let start = Instant::now();
   438	    let fatal = matches!(credential, StreamCredential::Epoch0(_));
   439	    let mut buf = vec![0u8; expected_token.len() + blit_core::remote::transfer::SUB_TOKEN_LEN];
   440	    match tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await {
   441	        Ok(Ok(_)) => {}
   442	        Ok(Err(err)) => {
   443	            return refuse(
   444	                fatal,
   445	                Status::internal(format!("failed to read data plane token: {}", err)),
   446	            );
   447	        }
   448	        Err(_elapsed) => {
   449	            return refuse(
   450	                fatal,
   451	                Status::deadline_exceeded(format!(
   452	                    "data plane token read timed out after {:?}",
   453	                    DATA_PLANE_TOKEN_TIMEOUT
   454	                )),
   455	            );

codex
Dependency-wise, `transfer_session/data_plane.rs` only imports the shared transfer primitives, not `remote::push` or daemon services. The stronger issues are both on the receive side: no session-level payload authorization and no read-side `StallGuard` around the accepted socket.
exec
/usr/bin/zsh -lc 'git diff --stat 881d412 --' in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'git status --short' in /home/michael/dev/Blit
 succeeded in 0ms:
?? .review/results/otp-4b1-data-plane.codex.md

 succeeded in 0ms:
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/data_plane.rs | sed -n '136,160p;173,208p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   136	    /// Accept exactly `initial_streams` authenticated data sockets and
   137	    /// drain each into `sink` via the shared receive pipeline, returning
   138	    /// the aggregated write outcome (the DESTINATION is the scorer). The
   139	    /// caller runs this concurrently with the control-stream diff loop
   140	    /// and joins it on `SourceDone`.
   141	    pub(super) async fn accept_and_receive(
   142	        self,
   143	        sink: Arc<dyn TransferSink>,
   144	    ) -> Result<SinkOutcome> {
   145	        // Epoch-0 socket credential: session_token ‖ epoch0_sub_token.
   146	        let mut expected = self.session_token.clone();
   147	        expected.extend_from_slice(&self.epoch0_sub_token);
   148	
   149	        let mut receives: JoinSet<Result<SinkOutcome>> = JoinSet::new();
   150	        for _ in 0..self.initial_streams {
   151	            let mut socket = accept_authenticated(&self.listener, &expected).await?;
   152	            let sink = Arc::clone(&sink);
   153	            receives.spawn(async move { execute_receive_pipeline(&mut socket, sink, None).await });
   154	        }
   155	
   156	        let mut total = SinkOutcome::default();
   157	        while let Some(joined) = receives.join_next().await {
   158	            let outcome =
   159	                joined.map_err(|err| dp_fault(format!("receive task panicked: {err}")))??;
   160	            total.files_written += outcome.files_written;
   173	async fn accept_authenticated(listener: &TcpListener, expected: &[u8]) -> Result<TcpStream> {
   174	    let accept = tokio::time::timeout(DATA_PLANE_ACCEPT_TIMEOUT, listener.accept()).await;
   175	    let socket = match accept {
   176	        Ok(Ok((socket, _peer))) => socket,
   177	        Ok(Err(err)) => return Err(dp_fault(format!("data-plane accept failed: {err}"))),
   178	        Err(_) => {
   179	            return Err(dp_fault(format!(
   180	            "data-plane accept timed out after {DATA_PLANE_ACCEPT_TIMEOUT:?} (source never dialed)"
   181	        )))
   182	        }
   183	    };
   184	    configure_data_socket(&socket, None)
   185	        .map_err(|err| dp_fault(format!("configuring accepted data socket: {err}")))?;
   186	
   187	    let mut socket = socket;
   188	    let mut buf = vec![0u8; expected.len()];
   189	    let read = tokio::time::timeout(DATA_PLANE_TOKEN_TIMEOUT, socket.read_exact(&mut buf)).await;
   190	    match read {
   191	        Ok(Ok(_)) => {}
   192	        Ok(Err(err)) => return Err(dp_fault(format!("reading data-plane credential: {err}"))),
   193	        Err(_) => {
   194	            return Err(dp_fault(format!(
   195	                "data-plane credential read timed out after {DATA_PLANE_TOKEN_TIMEOUT:?}"
   196	            )))
   197	        }
   198	    }
   199	    // Constant-time comparison is not required: the tokens are 16 random
   200	    // bytes read once per socket, single-session; a timing oracle buys
   201	    // nothing against per-transfer secrets (same posture as the old push
   202	    // acceptor's `token == expected_token`).
   203	    if buf != expected {
   204	        return Err(dp_fault(
   205	            "data-plane socket presented an invalid credential",
   206	        ));
   207	    }
   208	    Ok(socket)

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/transfer_session/mod.rs | sed -n '1248,1274p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1248	            Some(Frame::SourceDone(_)) => {
  1249	                if !manifest_complete {
  1250	                    return Err(violation("SourceDone before ManifestComplete".into()));
  1251	                }
  1252	                // Carrier-specific completion. In-stream: every payload
  1253	                // was consumed inline, so the need set must be fully
  1254	                // drained. Data plane: payloads rode the sockets (the
  1255	                // control lane never removed them from `outstanding`), so
  1256	                // join the receive task for the authoritative counts and
  1257	                // verify it delivered exactly the need list.
  1258	                let in_stream_carrier_used = match data_plane_recv.take() {
  1259	                    Some(recv) => {
  1260	                        let outcome = recv.join().await.map_err(|err| {
  1261	                            eyre::Report::new(SessionFault::internal(format!(
  1262	                                "data-plane receive task panicked: {err}"
  1263	                            )))
  1264	                        })??;
  1265	                        files_written = outcome.files_written as u64;
  1266	                        bytes_written = outcome.bytes_written;
  1267	                        if files_written != needed_paths.len() as u64 {
  1268	                            return Err(violation(format!(
  1269	                                "data plane delivered {} of {} needed file(s) before SourceDone",
  1270	                                files_written,
  1271	                                needed_paths.len()
  1272	                            )));
  1273	                        }
  1274	                        false

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/remote/transfer/pipeline.rs | sed -n '417,528p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   417	pub async fn execute_receive_pipeline<R: AsyncRead + Unpin + Send>(
   418	    socket: &mut R,
   419	    sink: Arc<dyn TransferSink>,
   420	    progress: Option<&RemoteTransferProgress>,
   421	) -> Result<SinkOutcome> {
   422	    let mut total = SinkOutcome::default();
   423	
   424	    loop {
   425	        let mut tag = [0u8; 1];
   426	        socket
   427	            .read_exact(&mut tag)
   428	            .await
   429	            .context("reading data-plane record tag")?;
   430	
   431	        match tag[0] {
   432	            DATA_PLANE_RECORD_END => break,
   433	            DATA_PLANE_RECORD_FILE => {
   434	                let mut header = read_file_header(socket).await?;
   435	                let file_size = read_u64(socket).await?;
   436	                let mtime = read_i64(socket).await?;
   437	                let perms = read_u32(socket).await?;
   438	                header.size = file_size;
   439	                header.mtime_seconds = mtime;
   440	                header.permissions = perms;
   441	                // Use AsyncReadExt::take to give the sink exactly
   442	                // file_size bytes of the wire. tokio's Take is the
   443	                // canonical way to limit a borrowed AsyncRead.
   444	                use tokio::io::AsyncReadExt;
   445	                let mut reader = (&mut *socket).take(file_size);
   446	                let outcome = sink
   447	                    .write_file_stream(&header, &mut reader)
   448	                    .await
   449	                    .with_context(|| format!("receiving {}", header.relative_path))?;
   450	                if let Some(p) = progress {
   451	                    p.report_payload(0, outcome.bytes_written);
   452	                    p.report_file_complete(header.relative_path.clone());
   453	                }
   454	                total.merge(&outcome);
   455	            }
   456	            DATA_PLANE_RECORD_TAR_SHARD => {
   457	                let (headers, data) = read_tar_shard(socket).await?;
   458	                let bytes = data.len() as u64;
   459	                // Capture member paths for the per-file lane before the
   460	                // payload takes ownership; skip the allocation when no
   461	                // one is listening (the daemon receive path).
   462	                let member_paths: Option<Vec<String>> =
   463	                    progress.map(|_| headers.iter().map(|h| h.relative_path.clone()).collect());
   464	                let payload = PreparedPayload::TarShard { headers, data };
   465	                let outcome = sink
   466	                    .write_payload(payload)
   467	                    .await
   468	                    .context("writing payload")?;
   469	                if let Some(p) = progress {
   470	                    p.report_payload(0, bytes);
   471	                    for path in member_paths.unwrap_or_default() {
   472	                        p.report_file_complete(path);
   473	                    }
   474	                }
   475	                total.merge(&outcome);
   476	            }
   477	            DATA_PLANE_RECORD_BLOCK => {
   478	                let path = read_string(socket).await?;
   479	                let offset = read_u64(socket).await?;
   480	                let len = read_u32(socket).await? as usize;
   481	                if len > MAX_WIRE_BLOCK_BYTES {
   482	                    bail!(
   483	                        "wire block payload {} bytes exceeds max {} (rejecting to avoid OOM)",
   484	                        len,
   485	                        MAX_WIRE_BLOCK_BYTES
   486	                    );
   487	                }
   488	                let mut bytes = vec![0u8; len];
   489	                socket
   490	                    .read_exact(&mut bytes)
   491	                    .await
   492	                    .context("reading block bytes")?;
   493	                let payload = PreparedPayload::FileBlock {
   494	                    relative_path: path,
   495	                    offset,
   496	                    bytes,
   497	                };
   498	                let outcome = sink
   499	                    .write_payload(payload)
   500	                    .await
   501	                    .context("writing payload")?;
   502	                if let Some(p) = progress {
   503	                    p.report_payload(0, outcome.bytes_written);
   504	                }
   505	                total.merge(&outcome);
   506	            }
   507	            DATA_PLANE_RECORD_BLOCK_COMPLETE => {
   508	                let path = read_string(socket).await?;
   509	                let total_size = read_u64(socket).await?;
   510	                let mtime = read_i64(socket).await?;
   511	                let perms = read_u32(socket).await?;
   512	                let path_for_progress = progress.map(|_| path.clone());
   513	                let payload = PreparedPayload::FileBlockComplete {
   514	                    relative_path: path,
   515	                    total_size,
   516	                    mtime_seconds: mtime,
   517	                    permissions: perms,
   518	                };
   519	                let outcome = sink
   520	                    .write_payload(payload)
   521	                    .await
   522	                    .context("writing payload")?;
   523	                if let Some(p) = progress {
   524	                    p.report_file_complete(path_for_progress.unwrap_or_default());
   525	                }
   526	                total.merge(&outcome);
   527	            }
   528	            other => bail!("unknown data-plane record tag: 0x{:02X}", other),

codex
- `crates/blit-core/src/transfer_session/mod.rs:1267` — **High** — Data-plane completion validates only `files_written == needed_paths.len()`. Because `execute_receive_pipeline` writes socket-provided paths directly (`pipeline.rs:446`, `:465`, `:477`), a peer can send the wrong path, duplicate one needed path while omitting another, or send non-resume block records and still pass if the count matches. Sink containment still protects against root escape, but the need-list/mode contract is not enforced.

- `crates/blit-core/src/transfer_session/data_plane.rs:153` — **High** — Accepted session data-plane sockets are passed raw into `execute_receive_pipeline`, without the read-side `StallGuard` used by the existing push receive path. After auth, a peer can stop before sending a record/END; then `run_destination` waits at `SourceDone` on `recv.join()` (`mod.rs:1260`) and can hang instead of faulting after the REV4 stall timeout.

VERDICT: **FAIL**. The token order/size and push-service dependency boundary look correct, but the receive side needs authorization/exact-completion checks and read-stall bounding before this slice is safe.
tokens used
187,401
- `crates/blit-core/src/transfer_session/mod.rs:1267` — **High** — Data-plane completion validates only `files_written == needed_paths.len()`. Because `execute_receive_pipeline` writes socket-provided paths directly (`pipeline.rs:446`, `:465`, `:477`), a peer can send the wrong path, duplicate one needed path while omitting another, or send non-resume block records and still pass if the count matches. Sink containment still protects against root escape, but the need-list/mode contract is not enforced.

- `crates/blit-core/src/transfer_session/data_plane.rs:153` — **High** — Accepted session data-plane sockets are passed raw into `execute_receive_pipeline`, without the read-side `StallGuard` used by the existing push receive path. After auth, a peer can stop before sending a record/END; then `run_destination` waits at `SourceDone` on `recv.join()` (`mod.rs:1260`) and can hang instead of faulting after the REV4 stall timeout.

VERDICT: **FAIL**. The token order/size and push-service dependency boundary look correct, but the receive side needs authorization/exact-completion checks and read-stall bounding before this slice is safe.
