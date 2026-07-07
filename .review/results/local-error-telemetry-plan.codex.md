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
session id: 019f39dc-a803-7af3-b376-06013d37f499
--------
user
Review the diff of commit 284f7f9 (run: git show 284f7f9). It is a new
   Draft design doc docs/plan/LOCAL_ERROR_TELEMETRY.md proposing local
   transfer-failure telemetry (a new errors_local.jsonl sibling file to the
   existing perf_local.jsonl, fed from a single chokepoint in
   crates/blit-cli/src/main.rs's Copy/Mirror command arms, plus a new
   'blit diagnostics errors' read-back verb). Check: internal coherence of
   the design against the actual current code it references (perf_history.rs,
   main.rs, diagnostics.rs — do the cited line numbers/behavior/call sites
   actually match reality?), whether the design's claim that
   record_performance_history is only reached on the success path is
   accurate, whether the proposed single-chokepoint wiring in main.rs is
   actually feasible given how Commands::Copy/Commands::Mirror currently
   propagate errors via '?', internal contradictions within the doc itself,
   and whether it contradicts anything in docs/DECISIONS.md (especially
   D-2026-07-05-4 pinning the Queue to ONE_TRANSFER_PATH — this doc
   explicitly says it is Draft-only/held and not entered in the Queue,
   confirm that framing is consistent). This is a plan/docs-only change, not
   code. Output a concise markdown findings list — each finding with
   file:line, severity, rationale — then a final VERDICT line. Be concise;
   do not invoke skills.
codex
I’ll inspect the commit diff and the referenced current files, then compare the draft’s claims against the code and decisions doc. Since this is a review-only request, I’ll keep it to findings and won’t modify files.
exec
/usr/bin/zsh -lc 'git show --stat --patch --find-renames --find-copies 284f7f9' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 284f7f9e0cd3ccfc8a65a0fe7d040b5e8f1aa060
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Mon Jul 6 19:56:18 2026 -0400

    Draft LOCAL_ERROR_TELEMETRY.md: local failure-log design (Draft, held)
    
    Owner asked for durable error reporting after hitting audit-17's crash
    three times across two drives and having to copy-paste terminal output
    each time. Designs a new errors_local.jsonl (sibling to perf_local.jsonl,
    not a schema change to it) fed from a single chokepoint in main.rs's
    Copy/Mirror arms, plus a `blit diagnostics errors` read-back verb.
    
    Explicitly held out of docs/STATE.md's Queue per the owner's choice this
    session: D-2026-07-05-4 pins the Queue to ONE_TRANSFER_PATH exclusively,
    so this stays Draft-only (no code) until that gate lifts or the owner
    gives an explicit go.
---
 docs/plan/LOCAL_ERROR_TELEMETRY.md | 205 +++++++++++++++++++++++++++++++++++++
 1 file changed, 205 insertions(+)

diff --git a/docs/plan/LOCAL_ERROR_TELEMETRY.md b/docs/plan/LOCAL_ERROR_TELEMETRY.md
new file mode 100644
index 0000000..6c789dc
--- /dev/null
+++ b/docs/plan/LOCAL_ERROR_TELEMETRY.md
@@ -0,0 +1,205 @@
+# Local transfer error telemetry (design)
+
+**Status**: Draft
+**Created**: 2026-07-06
+**Supersedes**: nothing
+**Decision ref**: pending (owner review)
+
+**Held, not queued**: `docs/STATE.md`'s Queue is pinned to ONE_TRANSFER_PATH
+exclusively (**D-2026-07-05-4**, "the only work item until it ships"). The
+owner asked for this feature but explicitly chose to hold it as Draft-only —
+**not** entered in the Queue, **not** built — until ONE_TRANSFER_PATH ships
+or the owner gives an explicit go. This doc exists so the design is ready
+the moment that gate lifts.
+
+## Why this doc
+
+The owner hit the same hard-failure crash (`audit-17` — a destination
+filesystem rejecting a `:` in a filename, `os error 22`) three times across
+two different USB drives while backing up `/home/michael/`, each time having
+to copy-paste the terminal error into chat. The ask: persist transfer
+failures locally so they can be reviewed as a batch ("sweep these all up")
+instead of by hand, per-crash.
+
+Today's "telemetry" (`perf_history.rs` → `perf_local.jsonl`, read via
+`blit diagnostics perf`) only records **successful** transfers. Its schema
+has an `error_count` field, but every writer hardcodes it to `0`
+(`engine/history.rs`, `auto_tune/mod.rs`, `perf_predictor.rs`,
+`engine/tuning.rs`) — dead. Worse, `record_performance_history` is only
+reached from the success path inside `run_local_mirror` (`engine/mod.rs:220,
+277, 314, 350, 792`, `engine/single_file.rs:42`); a top-level `Err` (exactly
+the `os error 22` case) writes nothing. Hard failures leave zero trace
+on disk today.
+
+## Goal
+
+A `blit copy`/`mirror` run that returns a top-level `Err` appends one record
+— timestamp, command shape, and the full error chain already printed to
+stderr — to a new local, capped JSONL file. A new `blit diagnostics errors`
+verb lists those records (most-recent-first, `--limit`, `--json`, `--clear`),
+so the owner can review accumulated failures without re-running commands or
+pasting terminal output.
+
+## Non-goals
+
+- **Does not fix `audit-17`/`audit-18` themselves.** Those stay separate
+  TODO.md findings with their own owner design call (skip-and-report vs.
+  sanitize vs. clean fail-fast). This plan makes failures *durable and
+  reviewable*; it does not change transfer behavior on failure.
+- **Does not build a fault-kind taxonomy** (permission-denied / ENOSPC /
+  invalid-name / etc. as a structured enum). That's adjacent to the
+  deferred `F15` structured-logging epic (`TODO.md`). This plan persists the
+  raw `eyre` error-chain text, not a classified error type.
+- **Does not unify with the daemon's `recents.jsonl`** (`blit-daemon/src/
+  recents_store.rs`, read via `blit jobs list <remote>`). That mechanism
+  already covers daemon-mediated remote push/pull across a different
+  process boundary; this plan covers local `copy`/`mirror` only (see Q3).
+- **No network transmission of any kind.** Fully local, on-device, same
+  trust model as `perf_local.jsonl` — this is a diagnostic log the owner
+  reads with a CLI verb, never phoned home.
+- **No automatic remediation** (retry-with-sanitized-name, skip-and-continue,
+  etc.) — that's `audit-17`'s decision, not this plan's.
+
+## Constraints
+
+- Local-only, on-device storage (matches `perf_local.jsonl`'s trust model —
+  a backup tool must not silently exfiltrate path/filename data).
+- Append-only JSONL, capped size (reuse `perf_history.rs`'s
+  `DEFAULT_MAX_BYTES` ~1 MiB rotation convention) so a machine that hits the
+  same crash repeatedly doesn't grow the file unbounded.
+- Must not slow down the hot (success) path — the write happens once, on
+  the already-exceptional error/abort path, at process exit.
+- Cross-platform: reuses `blit_core::config::config_dir()`, already
+  cross-platform (`directories::ProjectDirs`). No new platform-specific
+  code needed.
+- The recorder itself must be failure-tolerant: a broken/unwritable config
+  dir must never mask or replace the original error — recording is
+  best-effort (log a `log::warn!` and proceed) around the real `Result`
+  that still propagates to the process exit code and stderr exactly as
+  today.
+
+## Acceptance criteria
+
+- [ ] A `blit copy`/`mirror` run whose top-level result is `Err` appends
+      exactly one record to a new local JSONL file before the process
+      exits, containing at minimum: schema_version, timestamp, mode
+      (Copy/Mirror), source root, dest root, and the error chain (every
+      `eyre` context frame's message, same content already printed to
+      stderr by `color_eyre`).
+- [ ] `blit diagnostics errors [--limit N] [--json] [--clear]` reads the
+      file back, newest-first, mirroring `blit diagnostics perf`'s flag
+      conventions.
+- [ ] The file is capped/rotated the same way as `perf_local.jsonl` (oldest
+      records evicted first) so repeated identical crashes can't grow it
+      unbounded.
+- [ ] `perf_local.jsonl` and its reader/predictor are completely unaffected
+      — this is an additive, separate file, not a schema change to the
+      existing one (see D1).
+- [ ] Process exit code and stderr output for a failing command are
+      **byte-identical** to today's — the recorder taps the `Result`, it
+      never changes what the user sees or the exit code.
+- [ ] A forced-failure integration test (e.g. an unwritable destination)
+      asserts exactly one error record lands with the expected fields.
+- [ ] `cargo fmt`/`clippy`/`test --workspace` all green; test count does
+      not drop.
+
+## Design
+
+New module `blit-core/src/error_history.rs`, mirroring `perf_history.rs`'s
+shape (`FailureRecord` struct, `record_failure(...)`, `read_failures(limit)`,
+`clear_failures()`), writing to `errors_local.jsonl` in the same
+`config::config_dir()` as `perf_local.jsonl` — a sibling file, not a shared
+schema (see D1).
+
+Draft schema (`FailureRecord`):
+- `schema_version: u32`
+- `timestamp` (same convention as `PerformanceRecord`)
+- `mode: TransferMode` (reuse the existing `Copy`/`Mirror` enum from
+  `perf_history.rs`)
+- `source: String`, `dest: String` (the two root paths as given on the CLI)
+- `error_chain: Vec<String>` — each frame of the returned `eyre::Report`'s
+  `.chain()`, in order (outermost context first, root cause last) — the
+  same information `color_eyre` prints as the numbered `0:`/`1:`/... list,
+  captured programmatically instead of scraped from stderr text.
+- `error_location: Option<String>` — best-effort; see Q2, this may not be
+  cleanly capturable without touching how `color_eyre::install()` is set
+  up, and may ship as `None` in the first slice.
+
+Wiring: `crates/blit-cli/src/main.rs`'s `Commands::Copy`/`Commands::Mirror`
+arms currently call `run_with_retries(..., || run_transfer(...)).await?` —
+the `?` bubbles straight out of the `match`, so there is no single point
+inside `main` today that sees the `Result` before it becomes the function's
+return value. The fix is the same "single chokepoint" shape this repo
+already favors elsewhere (`FilteredSource`, `safe_join`,
+`contained_join`): change those two arms to bind the `Result` instead of
+`?`-ing it immediately, call `error_history::record_failure(...)` when it's
+`Err` (best-effort — a recorder failure is logged via `log::warn!` and
+never replaces the original error), then return/`?` the *original,
+untouched* `Result` so behavior for the user is identical to today.
+
+New CLI verb: `blit diagnostics errors` alongside the existing
+`run_diagnostics_perf` in `crates/blit-cli/src/diagnostics.rs`, same flag
+shape (`--limit`, `--json`, `--clear`).
+
+## Slices
+
+1. **`error_history` module** — schema, `record_failure`/`read_failures`/
+   `clear_failures`, cap/rotation (mirrors `perf_history.rs`'s existing
+   logic), unit tests (round-trip, cap eviction, tolerant read of a
+   corrupted/partial last line — matching `perf_history.rs`'s existing
+   tolerance).
+2. **Wire the `Copy`/`Mirror` CLI arms** in `main.rs` to call
+   `record_failure` on `Err` before propagating, unchanged exit
+   code/stderr. Integration test: force a failure (e.g. destination path
+   that can't be created), assert exactly one record lands with the
+   expected `source`/`dest`/`mode`/non-empty `error_chain`, and assert
+   stderr/exit-code parity with the no-recorder baseline.
+3. **`blit diagnostics errors` read-back verb** — list/limit/json/clear,
+   unit + CLI-level tests.
+
+Deliberately **not** a slice here (future follow-ups, owner-gated): folding
+`Move`/remote-mediated commands into the same recorder (Q3); capturing
+`error_location` if a clean API surface exists (Q2); any interaction with
+`audit-17`'s eventual skip-and-report behavior, where a partially-successful
+transfer with per-file skips might also want a record here — that's a
+follow-up once `audit-17` itself is designed, not this plan's job.
+
+## Open questions for the owner
+
+- **Q1**: A new dedicated `errors_local.jsonl`, or extend `perf_local.jsonl`
+  to carry failure rows (finally populating the dead `error_count` field)?
+  Agent rec: new dedicated file. `perf_local.jsonl`'s reader
+  (`perf_predictor.rs`) is built around successful-run regression inputs;
+  mixing failure rows into that stream complicates the predictor's read
+  path for no benefit, and keeping them separate matches the existing
+  precedent of `recents.jsonl` being its own file rather than folded into
+  `perf_local.jsonl`.
+- **Q2**: Capture `error_location` (the `Location:` file:line `color_eyre`
+  prints) or ship with `error_chain` message text only? Capturing it
+  cleanly may require restructuring how `color_eyre::install()` hooks
+  panic/error reporting (a real technical risk, not yet spiked). Agent
+  rec: ship message-chain-only first (still fully "sweepable" — the chain
+  already names the failing path and OS error), file `error_location`
+  capture as a fast-follow if the `eyre`/`color_eyre` API allows it without
+  restructuring the install.
+- **Q3**: Local `copy`/`mirror` only for now — should remote push/pull
+  (daemon-mediated) ever unify onto this same file, or stay on
+  `recents.jsonl` permanently? Agent rec: leave remote alone permanently;
+  different process boundary (daemon vs. CLI), already has a working
+  mechanism — don't force a merge for its own sake.
+- **Q4**: Reuse `perf_local.jsonl`'s ~1 MiB cap as-is, given failure records
+  (full error chains, long paths) may run larger per-record than perf
+  records? Agent rec: same cap, oldest-evicted — consistent with the
+  existing convention; revisit only if it proves too small in practice.
+- **Q5 (gate, not design)**: When does this leave Draft? Per the owner's
+  choice this session, not until ONE_TRANSFER_PATH ships or the owner
+  explicitly lifts D-2026-07-05-4's Queue-exclusivity for this item.
+
+## Verification (when Active)
+
+- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --
+  -D warnings`; `cargo test --workspace` (count must not drop).
+- Each slice through the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`).
+- Stderr/exit-code byte-parity check for the failure path (before vs. after
+  wiring `record_failure` in) — the whole point is that recording is
+  invisible to the user-facing failure behavior.

exec
/usr/bin/zsh -lc 'rg --files .agents docs crates/blit-cli/src | sort' in /home/michael/dev/Blit
 succeeded in 0ms:
.agents/artifact-manifest.json
.agents/decisions.md
.agents/governance-inventory.md
.agents/harvest.md
.agents/hooks.json
.agents/playbooks/reviewloop.md
.agents/push-policy.md
.agents/repo-guidance.md
.agents/repo-map.json
.agents/skills/catchup/SKILL.md
.agents/skills/handoff/SKILL.md
.agents/state.md
crates/blit-cli/src/check.rs
crates/blit-cli/src/cli.rs
crates/blit-cli/src/completions.rs
crates/blit-cli/src/context.rs
crates/blit-cli/src/df.rs
crates/blit-cli/src/diagnostics.rs
crates/blit-cli/src/du.rs
crates/blit-cli/src/find.rs
crates/blit-cli/src/jobs.rs
crates/blit-cli/src/list_modules.rs
crates/blit-cli/src/ls.rs
crates/blit-cli/src/main.rs
crates/blit-cli/src/profile.rs
crates/blit-cli/src/rm.rs
crates/blit-cli/src/scan.rs
crates/blit-cli/src/transfers/endpoints.rs
crates/blit-cli/src/transfers/local.rs
crates/blit-cli/src/transfers/mod.rs
crates/blit-cli/src/transfers/remote.rs
crates/blit-cli/src/transfers/remote_remote_direct.rs
docs/API.md
docs/ARCHITECTURE.md
docs/DAEMON_CONFIG.md
docs/DECISIONS.md
docs/DEVLOG.md
docs/PERFORMANCE_ROADMAP.md
docs/STATE.md
docs/TRANSFER_SESSION.md
docs/WHITEPAPER.md
docs/agent/GPT_REVIEW_LOOP.md
docs/agent/PROTOCOL.md
docs/audit/2026-05-04_roadmap_audit.md
docs/audit/AUDIT_REPORT_2026-06-04.md
docs/audit/AUDIT_REPORT_2026-06-04_INDEX.md
docs/audit/AUDIT_REPORT_2026-06-04_R2.md
docs/audit/AUDIT_REPORT_2026-06-04_R3.md
docs/audit/AUDIT_REPORT_2026-06-11_DESIGN.md
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md
docs/audit/DESIGN_MAP_2026-06-11.md
docs/audit/findings/drift-cli.md
docs/audit/findings/drift-perf.md
docs/audit/findings/drift-phases.md
docs/audit/findings/drift-principles.md
docs/audit/findings/drift-tui.md
docs/audit/findings/drift-wire.md
docs/audit/findings/inconsistency-endpoints.md
docs/audit/findings/inconsistency-errors.md
docs/audit/findings/inconsistency-naming-flags-confirmations.md
docs/audit/findings/inconsistency-paths.md
docs/audit/findings/inconsistency-timeouts.md
docs/audit/inventory/code-bridge-proto.md
docs/audit/inventory/code-cli.md
docs/audit/inventory/code-core-io.md
docs/audit/inventory/code-core-misc.md
docs/audit/inventory/code-core-orch.md
docs/audit/inventory/code-core-transfer.md
docs/audit/inventory/code-daemon.md
docs/audit/inventory/code-tests-scripts.md
docs/audit/inventory/code-tui-display.md
docs/audit/inventory/code-tui-main.md
docs/audit/inventory/code-tui-state.md
docs/audit/inventory/plan-cli.md
docs/audit/inventory/plan-perf.md
docs/audit/inventory/plan-phases.md
docs/audit/inventory/plan-principles.md
docs/audit/inventory/plan-tui.md
docs/audit/inventory/plan-wire.md
docs/bench/10gbe-2026-07-05/DIAGNOSIS.md
docs/bench/10gbe-2026-07-05/SESSION_NOTES.md
docs/bench/10gbe-2026-07-05/cmp_fair.csv
docs/bench/10gbe-2026-07-05/cmp_fair2.csv
docs/bench/10gbe-2026-07-05/loopback_parity_band.csv
docs/bench/10gbe-2026-07-05/tool_comparison.csv
docs/bench/10gbe-2026-07-05/validated_matrix_results.csv
docs/bench/10gbe-2026-07-05/wire_clean_runs.csv
docs/bugs/HANDOFF-remote-pull-single-file.md
docs/bugs/copy-destination-semantics.md
docs/bugs/single-file-remote-push-crash.md
docs/bugs/single-file-source-silent-noop.md
docs/cli/blit-daemon.1.md
docs/cli/blit.1.md
docs/perf/remote_remote_benchmarks.md
docs/plan/BENCHMARK_10GBE_PLAN.md
docs/plan/BENCH_VERB_PLAN.md
docs/plan/BLIT_UTILS_PLAN.md
docs/plan/DESIGN_COHERENCE_REVIEW.md
docs/plan/LOCAL_ERROR_TELEMETRY.md
docs/plan/LOCAL_TRANSFER_HEURISTICS.md
docs/plan/MASTER_WORKFLOW.md
docs/plan/MULTISTREAM_PULL.md
docs/plan/ONE_TRANSFER_PATH.md
docs/plan/OTP7_RESUME.md
docs/plan/PIPELINE_UNIFICATION.md
docs/plan/POST_REVIEW_FIXES.md
docs/plan/PROJECT_STATE_ASSESSMENT.md
docs/plan/README.md
docs/plan/RELEASE_PLAN_v2_2026-05-04.md
docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md
docs/plan/REMOTE_TRANSFER_PARITY.md
docs/plan/SMALL_FILE_CEILING.md
docs/plan/TEMPLATE.md
docs/plan/TUI_DESIGN.md
docs/plan/TUI_REWORK.md
docs/plan/UNIFIED_RECEIVE_PIPELINE.md
docs/plan/UNIFIED_TRANSFER_ENGINE.md
docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md
docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md
docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md
docs/plan/WORKFLOW_PHASE_2.5.md
docs/plan/WORKFLOW_PHASE_2.md
docs/plan/WORKFLOW_PHASE_3.md
docs/plan/WORKFLOW_PHASE_4.md
docs/plan/WORKFLOW_V2.md
docs/plan/ZERO_COPY_RECEIVE_EVAL.md
docs/plan/greenfield_plan_v6.md
docs/plan/review/code_review_phase2.md
docs/reviews/baseline_triage_2026-05-02.md
docs/reviews/codebase_review_2026-05-01.md
docs/reviews/followup_review_2026-05-02.md
docs/ux-feedback-migrate-games-poc.md

exec
/usr/bin/zsh -lc 'git status --short' in /home/michael/dev/Blit
 succeeded in 0ms:
 M Cargo.lock
?? .review/results/local-error-telemetry-plan.codex.md

codex
The commit is a single new draft doc. I see an unrelated dirty `Cargo.lock` and an untracked review result file in the worktree; I’ll ignore those for this review.
exec
/usr/bin/zsh -lc "sed -n '1,240p' .agents/state.md" in /home/michael/dev/Blit
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

exec
/usr/bin/zsh -lc "sed -n '1,220p' .agents/repo-guidance.md" in /home/michael/dev/Blit
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
/usr/bin/zsh -lc "nl -ba docs/plan/LOCAL_ERROR_TELEMETRY.md | sed -n '1,240p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "sed -n '1,260p' docs/DECISIONS.md" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	# Local transfer error telemetry (design)
     2	
     3	**Status**: Draft
     4	**Created**: 2026-07-06
     5	**Supersedes**: nothing
     6	**Decision ref**: pending (owner review)
     7	
     8	**Held, not queued**: `docs/STATE.md`'s Queue is pinned to ONE_TRANSFER_PATH
     9	exclusively (**D-2026-07-05-4**, "the only work item until it ships"). The
    10	owner asked for this feature but explicitly chose to hold it as Draft-only —
    11	**not** entered in the Queue, **not** built — until ONE_TRANSFER_PATH ships
    12	or the owner gives an explicit go. This doc exists so the design is ready
    13	the moment that gate lifts.
    14	
    15	## Why this doc
    16	
    17	The owner hit the same hard-failure crash (`audit-17` — a destination
    18	filesystem rejecting a `:` in a filename, `os error 22`) three times across
    19	two different USB drives while backing up `/home/michael/`, each time having
    20	to copy-paste the terminal error into chat. The ask: persist transfer
    21	failures locally so they can be reviewed as a batch ("sweep these all up")
    22	instead of by hand, per-crash.
    23	
    24	Today's "telemetry" (`perf_history.rs` → `perf_local.jsonl`, read via
    25	`blit diagnostics perf`) only records **successful** transfers. Its schema
    26	has an `error_count` field, but every writer hardcodes it to `0`
    27	(`engine/history.rs`, `auto_tune/mod.rs`, `perf_predictor.rs`,
    28	`engine/tuning.rs`) — dead. Worse, `record_performance_history` is only
    29	reached from the success path inside `run_local_mirror` (`engine/mod.rs:220,
    30	277, 314, 350, 792`, `engine/single_file.rs:42`); a top-level `Err` (exactly
    31	the `os error 22` case) writes nothing. Hard failures leave zero trace
    32	on disk today.
    33	
    34	## Goal
    35	
    36	A `blit copy`/`mirror` run that returns a top-level `Err` appends one record
    37	— timestamp, command shape, and the full error chain already printed to
    38	stderr — to a new local, capped JSONL file. A new `blit diagnostics errors`
    39	verb lists those records (most-recent-first, `--limit`, `--json`, `--clear`),
    40	so the owner can review accumulated failures without re-running commands or
    41	pasting terminal output.
    42	
    43	## Non-goals
    44	
    45	- **Does not fix `audit-17`/`audit-18` themselves.** Those stay separate
    46	  TODO.md findings with their own owner design call (skip-and-report vs.
    47	  sanitize vs. clean fail-fast). This plan makes failures *durable and
    48	  reviewable*; it does not change transfer behavior on failure.
    49	- **Does not build a fault-kind taxonomy** (permission-denied / ENOSPC /
    50	  invalid-name / etc. as a structured enum). That's adjacent to the
    51	  deferred `F15` structured-logging epic (`TODO.md`). This plan persists the
    52	  raw `eyre` error-chain text, not a classified error type.
    53	- **Does not unify with the daemon's `recents.jsonl`** (`blit-daemon/src/
    54	  recents_store.rs`, read via `blit jobs list <remote>`). That mechanism
    55	  already covers daemon-mediated remote push/pull across a different
    56	  process boundary; this plan covers local `copy`/`mirror` only (see Q3).
    57	- **No network transmission of any kind.** Fully local, on-device, same
    58	  trust model as `perf_local.jsonl` — this is a diagnostic log the owner
    59	  reads with a CLI verb, never phoned home.
    60	- **No automatic remediation** (retry-with-sanitized-name, skip-and-continue,
    61	  etc.) — that's `audit-17`'s decision, not this plan's.
    62	
    63	## Constraints
    64	
    65	- Local-only, on-device storage (matches `perf_local.jsonl`'s trust model —
    66	  a backup tool must not silently exfiltrate path/filename data).
    67	- Append-only JSONL, capped size (reuse `perf_history.rs`'s
    68	  `DEFAULT_MAX_BYTES` ~1 MiB rotation convention) so a machine that hits the
    69	  same crash repeatedly doesn't grow the file unbounded.
    70	- Must not slow down the hot (success) path — the write happens once, on
    71	  the already-exceptional error/abort path, at process exit.
    72	- Cross-platform: reuses `blit_core::config::config_dir()`, already
    73	  cross-platform (`directories::ProjectDirs`). No new platform-specific
    74	  code needed.
    75	- The recorder itself must be failure-tolerant: a broken/unwritable config
    76	  dir must never mask or replace the original error — recording is
    77	  best-effort (log a `log::warn!` and proceed) around the real `Result`
    78	  that still propagates to the process exit code and stderr exactly as
    79	  today.
    80	
    81	## Acceptance criteria
    82	
    83	- [ ] A `blit copy`/`mirror` run whose top-level result is `Err` appends
    84	      exactly one record to a new local JSONL file before the process
    85	      exits, containing at minimum: schema_version, timestamp, mode
    86	      (Copy/Mirror), source root, dest root, and the error chain (every
    87	      `eyre` context frame's message, same content already printed to
    88	      stderr by `color_eyre`).
    89	- [ ] `blit diagnostics errors [--limit N] [--json] [--clear]` reads the
    90	      file back, newest-first, mirroring `blit diagnostics perf`'s flag
    91	      conventions.
    92	- [ ] The file is capped/rotated the same way as `perf_local.jsonl` (oldest
    93	      records evicted first) so repeated identical crashes can't grow it
    94	      unbounded.
    95	- [ ] `perf_local.jsonl` and its reader/predictor are completely unaffected
    96	      — this is an additive, separate file, not a schema change to the
    97	      existing one (see D1).
    98	- [ ] Process exit code and stderr output for a failing command are
    99	      **byte-identical** to today's — the recorder taps the `Result`, it
   100	      never changes what the user sees or the exit code.
   101	- [ ] A forced-failure integration test (e.g. an unwritable destination)
   102	      asserts exactly one error record lands with the expected fields.
   103	- [ ] `cargo fmt`/`clippy`/`test --workspace` all green; test count does
   104	      not drop.
   105	
   106	## Design
   107	
   108	New module `blit-core/src/error_history.rs`, mirroring `perf_history.rs`'s
   109	shape (`FailureRecord` struct, `record_failure(...)`, `read_failures(limit)`,
   110	`clear_failures()`), writing to `errors_local.jsonl` in the same
   111	`config::config_dir()` as `perf_local.jsonl` — a sibling file, not a shared
   112	schema (see D1).
   113	
   114	Draft schema (`FailureRecord`):
   115	- `schema_version: u32`
   116	- `timestamp` (same convention as `PerformanceRecord`)
   117	- `mode: TransferMode` (reuse the existing `Copy`/`Mirror` enum from
   118	  `perf_history.rs`)
   119	- `source: String`, `dest: String` (the two root paths as given on the CLI)
   120	- `error_chain: Vec<String>` — each frame of the returned `eyre::Report`'s
   121	  `.chain()`, in order (outermost context first, root cause last) — the
   122	  same information `color_eyre` prints as the numbered `0:`/`1:`/... list,
   123	  captured programmatically instead of scraped from stderr text.
   124	- `error_location: Option<String>` — best-effort; see Q2, this may not be
   125	  cleanly capturable without touching how `color_eyre::install()` is set
   126	  up, and may ship as `None` in the first slice.
   127	
   128	Wiring: `crates/blit-cli/src/main.rs`'s `Commands::Copy`/`Commands::Mirror`
   129	arms currently call `run_with_retries(..., || run_transfer(...)).await?` —
   130	the `?` bubbles straight out of the `match`, so there is no single point
   131	inside `main` today that sees the `Result` before it becomes the function's
   132	return value. The fix is the same "single chokepoint" shape this repo
   133	already favors elsewhere (`FilteredSource`, `safe_join`,
   134	`contained_join`): change those two arms to bind the `Result` instead of
   135	`?`-ing it immediately, call `error_history::record_failure(...)` when it's
   136	`Err` (best-effort — a recorder failure is logged via `log::warn!` and
   137	never replaces the original error), then return/`?` the *original,
   138	untouched* `Result` so behavior for the user is identical to today.
   139	
   140	New CLI verb: `blit diagnostics errors` alongside the existing
   141	`run_diagnostics_perf` in `crates/blit-cli/src/diagnostics.rs`, same flag
   142	shape (`--limit`, `--json`, `--clear`).
   143	
   144	## Slices
   145	
   146	1. **`error_history` module** — schema, `record_failure`/`read_failures`/
   147	   `clear_failures`, cap/rotation (mirrors `perf_history.rs`'s existing
   148	   logic), unit tests (round-trip, cap eviction, tolerant read of a
   149	   corrupted/partial last line — matching `perf_history.rs`'s existing
   150	   tolerance).
   151	2. **Wire the `Copy`/`Mirror` CLI arms** in `main.rs` to call
   152	   `record_failure` on `Err` before propagating, unchanged exit
   153	   code/stderr. Integration test: force a failure (e.g. destination path
   154	   that can't be created), assert exactly one record lands with the
   155	   expected `source`/`dest`/`mode`/non-empty `error_chain`, and assert
   156	   stderr/exit-code parity with the no-recorder baseline.
   157	3. **`blit diagnostics errors` read-back verb** — list/limit/json/clear,
   158	   unit + CLI-level tests.
   159	
   160	Deliberately **not** a slice here (future follow-ups, owner-gated): folding
   161	`Move`/remote-mediated commands into the same recorder (Q3); capturing
   162	`error_location` if a clean API surface exists (Q2); any interaction with
   163	`audit-17`'s eventual skip-and-report behavior, where a partially-successful
   164	transfer with per-file skips might also want a record here — that's a
   165	follow-up once `audit-17` itself is designed, not this plan's job.
   166	
   167	## Open questions for the owner
   168	
   169	- **Q1**: A new dedicated `errors_local.jsonl`, or extend `perf_local.jsonl`
   170	  to carry failure rows (finally populating the dead `error_count` field)?
   171	  Agent rec: new dedicated file. `perf_local.jsonl`'s reader
   172	  (`perf_predictor.rs`) is built around successful-run regression inputs;
   173	  mixing failure rows into that stream complicates the predictor's read
   174	  path for no benefit, and keeping them separate matches the existing
   175	  precedent of `recents.jsonl` being its own file rather than folded into
   176	  `perf_local.jsonl`.
   177	- **Q2**: Capture `error_location` (the `Location:` file:line `color_eyre`
   178	  prints) or ship with `error_chain` message text only? Capturing it
   179	  cleanly may require restructuring how `color_eyre::install()` hooks
   180	  panic/error reporting (a real technical risk, not yet spiked). Agent
   181	  rec: ship message-chain-only first (still fully "sweepable" — the chain
   182	  already names the failing path and OS error), file `error_location`
   183	  capture as a fast-follow if the `eyre`/`color_eyre` API allows it without
   184	  restructuring the install.
   185	- **Q3**: Local `copy`/`mirror` only for now — should remote push/pull
   186	  (daemon-mediated) ever unify onto this same file, or stay on
   187	  `recents.jsonl` permanently? Agent rec: leave remote alone permanently;
   188	  different process boundary (daemon vs. CLI), already has a working
   189	  mechanism — don't force a merge for its own sake.
   190	- **Q4**: Reuse `perf_local.jsonl`'s ~1 MiB cap as-is, given failure records
   191	  (full error chains, long paths) may run larger per-record than perf
   192	  records? Agent rec: same cap, oldest-evicted — consistent with the
   193	  existing convention; revisit only if it proves too small in practice.
   194	- **Q5 (gate, not design)**: When does this leave Draft? Per the owner's
   195	  choice this session, not until ONE_TRANSFER_PATH ships or the owner
   196	  explicitly lifts D-2026-07-05-4's Queue-exclusivity for this item.
   197	
   198	## Verification (when Active)
   199	
   200	- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --
   201	  -D warnings`; `cargo test --workspace` (count must not drop).
   202	- Each slice through the codex loop (`docs/agent/GPT_REVIEW_LOOP.md`).
   203	- Stderr/exit-code byte-parity check for the failure path (before vs. after
   204	  wiring `record_failure` in) — the whole point is that recording is
   205	  invisible to the user-facing failure behavior.

 succeeded in 0ms:
# DECISIONS — settled choices

**Status**: Active

Append-only ledger of decisions that future sessions must not relitigate or miss.
Add entries via the `decision` procedure in `docs/agent/PROTOCOL.md`. Newest last.
When a decision supersedes plan text, the plan text gets edited in the same
session — this file is the index, not a substitute for fixing the doc.

Format:

```
## D-<YYYY-MM-DD>-<n> — <short title>
- Decision: <one line>
- Why: <one line>
- Supersedes: <doc §/decision ID, or "nothing">
```

---

## D-2026-05-31-1 — v0.1.0 shipped; release plan frozen
- Decision: `RELEASE_PLAN_v2_2026-05-04.md` is a frozen reference, no longer the active source of truth.
- Why: 0.1.0 tagged 2026-05-31; the plan served its purpose.
- Supersedes: RELEASE_PLAN_v2_2026-05-04.md as active plan.

## D-2026-05-31-2 — Pick-not-Type TUI direction
- Decision: `TUI_REWORK.md` (dual-pane, M1–M6) supersedes `TUI_DESIGN.md` §6 trigger-modal text inputs and the F3 free-text destination prompt.
- Why: any field requiring the operator to recall and type an off-screen path is an interface failure.
- Supersedes: TUI_DESIGN.md §6 (portions).

## D-2026-06-04-1 — R3 overrides R2 in the audit chain
- Decision: where R2 and R3 disagree on a finding's severity or content, R3 wins; see the ID-override table in `AUDIT_REPORT_2026-06-04_INDEX.md`.
- Why: R3 incorporates the GPT R2 critique and severity rebalance.
- Supersedes: conflicting R2 entries.

## D-2026-06-04-2 — Env vars are out for app + diagnostic config
- Decision: no environment-variable configuration carve-out (R3-L39); purge completed via `audit-l39-m27-env-var-purge`.
- Why: owner policy — config surfaces stay explicit.
- Supersedes: nothing (clarifies prior ambiguity).

## D-2026-06-04-3 — Streaming planner ratified, build deferred
- Decision: `greenfield_plan_v6.md` §1.1 (streaming planner + 1 s heartbeat + 10 s stall detector) is canonical but not yet built; multi-slice implementation queued after audit Round 1 (H10b).
- Why: data-loss/DoS hardening takes priority; the plan claim is ratified rather than retired.
- Supersedes: nothing.

## D-2026-06-06-1 — STATE.md precedence model adopted
- Decision: `docs/STATE.md` is the single entry point for current state, with the precedence order in `AGENTS.md` §1; DEVLOG.md is write-only history, TODO.md is backlog-only, tool-local memories are scratch.
- Why: state smeared across TODO/DEVLOG/plan-README/Serena was the drift mechanism the 2026-06-04 audit documented (drift-* findings, M28).
- Supersedes: "Agent-Specific Expectations" in the previous AGENTS.md (Serena memories as session persistence).

## D-2026-06-07-1 — Keep the `c793df2` octopus on master; no history rewrite
- Decision: `c793df2` (a `git merge -s ours` octopus whose parents are `600023a` + `eafb187` + `d9d4ec7`) stays on `origin/master`; we do **not** rewrite history or force-push to remove it.
- Why: its tree is byte-identical to `600023a` (`git diff 600023a c793df2` is empty) and the workspace builds, so it is cosmetically ugly but harmless; rewriting already-pushed shared history is riskier than the wart. The merge was pushed without owner approval — the corrective is the new AGENTS.md §8 Git-safety contract, not a second unsafe operation.
- Consequence (the trap): because `eafb187` and `d9d4ec7` are now *ancestors* of master, `git branch --merged` falsely reports them merged and a plain `git merge` of either no-ops without landing code. `d9d4ec7` (adaptive-streams-pr3-resizable) does **not** build and its files are not in master's tree. Branch cleanup in this repo is by explicit name only, never `--merged`.
- Supersedes: nothing.

## D-2026-06-07-2 — Adaptive-streams lands via cherry-pick/rebase, excluding the WIP
- Decision: the adaptive-streams stack (live-progress → PR1 telemetry → PR2 work-queue → PR2 review fix, up to `eafb187`) lands later as a planned `docs/plan/` slice via cherry-pick or rebase onto fresh commits — never via `git merge` of the branch (see D-2026-06-07-1 trap). `d9d4ec7` (PR3 WIP, "DOES NOT BUILD") is explicitly excluded until it is finished and compiles.
- Why: the `-s ours` octopus recorded those tips as parents without landing their code, so the feature is not actually in master; a real merge would no-op. The one real conflict (`data_plane.rs`: `StallGuardWriter` vs the `Probe` generic) must be resolved by hand, which only a cherry-pick/rebase surfaces.
- Supersedes: nothing.

## D-2026-06-11-1 — Design-coherence review plan Active; ratification covers Phase A only
- Decision: `docs/plan/DESIGN_COHERENCE_REVIEW.md` flipped Draft → Active. Owner approval authorizes **Phase A only** (concept-ownership map + per-crate stratum inventory); Phases B and C each need a fresh go/no-go at the preceding checkpoint. Interview decisions bound into the plan: blit-tui light pass, owner ratifies each Phase C finding, wire-breaking recommendations in scope (proto not frozen).
- Why: the repo was built by many models across several greenfield restarts and the owner judges it too inconsistently designed to trust as-is; mapping concept ownership precedes any re-scope (audit-h3c slice 2) or feature landing (adaptive-streams) so the fixes get designed once.
- Supersedes: nothing.

## D-2026-06-11-2 — Design-review queue ratified in full; Pull-RPC delete; zero_copy gets a FAST evaluation
- Decision: All Phase C slices (`AUDIT_REPORT_2026-06-11_DESIGN.md`) ratified as proposed and entered into REVIEW.md in the proposed order. Embedded decisions: (a) **W2.4** — the deprecated Pull RPC is deleted once W2.3 has harvested its multi-stream pattern; criterion applied: not needed for FAST/SIMPLE/RELIABLE in any scenario. (b) **W8.1** — `zero_copy.rs` is **excluded** from the dead-code deletion sweep; owner judges it has FAST potential; disposition is an evaluation slice (`w8-1b`) that either produces a plan doc to wire splice into the receive pipeline or concludes deletion. (c) **W2.3** — writing the multi-stream-pull plan doc is authorized (no code before Status: Active).
- Why: review program (D-2026-06-11-1) delivered all three phases; owner is the gate for queue entry and exercised it in full.
- Supersedes: nothing (completes D-2026-06-11-1; `DESIGN_COHERENCE_REVIEW.md` flips Active → Shipped).

## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
- Decision: `zero_copy.rs` is deleted rather than wired in. The w8-1b evaluation (`docs/plan/ZERO_COPY_RECEIVE_EVAL.md`) recommended deletion and the owner agreed (2026-06-12 session). The deletion executes inside w8-1 once the w5-1 sentinel (lib.rs) is graded — it is no longer excluded from that sweep.
- Why: the dead draft busy-waits on EAGAIN (would be rewritten, not revived); wiring needs a raw-fd special case beside a permanent buffered fallback; the CPU saving is a fraction of one core, Linux-only, and unmeasured. Revisit gate: 10 GbE benchmarks showing receive-side CPU saturation — design notes preserved in the eval doc.
- Supersedes: D-2026-06-11-2 item (b) (zero_copy exclusion from W8.1 was pending this evaluation; the evaluation is done).

## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
- Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) ~~and **size-gated** (small transfers skip the probe entirely)~~ **(size-gate framing superseded by D-2026-06-20-2 q1 — there is no probe phase to skip; the engine moves within ~1s and tunes live)**. The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. ~~Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit).~~ **(A/warmup staging superseded by D-2026-06-20-2 q1 — conservative start + live tuning from the first byte; C shipped as `ue-r2-2` under REV4/D-2026-06-20-5.)** Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip). *(Stale wording struck 2026-07-04 on owner direction — "follow the existing pattern": the in-place-annotation pattern of D-2026-06-20-3/-6. The convergence direction itself stands unchanged.)*
- Why: owner (30-year IT veteran, not a developer) judges the fragmentation — one engine for local, hand-wired loops for push/pull, three competing static stream-count tables, no live tuning — is the root of the "local↔local 10× slower than local→daemon" class of drift; a single engine makes that class impossible by construction and gives the LLM agent one place to update. Ground-up rebuild was judged too much; convergence on the existing shared leaf is the FAST/SIMPLE/RELIABLE fit. The adaptive substrate was purpose-built by an earlier Fable session as C's foundation, so building A on it does not paint the design into a corner.
- Scope consequence: this **moots the standalone premise** of the queued incremental work and absorbs the goals — w2-2 (three ladders → one dial) is `ue-1b`; w2-3 multi-stream pull (`MULTISTREAM_PULL.md`) is `ue-1d` via the unified sequencer; w2-4 (delete deprecated Pull RPC) is `ue-1e`; adaptive-streams cherry-pick is `ue-1a`. `MULTISTREAM_PULL.md` is superseded as a standalone plan (kept as reference); its goal survives inside this plan. The design-review queue's correctness findings (w4-1 etc.) are independent and unaffected.
- Supersedes: the "ground-up redesign" framing of the 2026-06-14 open question recorded in STATE.md (that open question is now closed); `MULTISTREAM_PULL.md` as a standalone plan (goal absorbed into `UNIFIED_TRANSFER_ENGINE.md` slice `ue-1d`).

## D-2026-06-20-2 — UNIFIED_TRANSFER_ENGINE.md flipped Draft → Active; four bound parameters
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` is **Active**. Owner approved with four parameters that bind the design: (q1) **no probe-then-go phase** — the engine starts moving within ~1s at conservative defaults bounded by the receiver ceiling and the tuner adjusts dials live from the first byte; the "small-transfer threshold" is obviated (no probe to skip), and the **planner** carries the workload-shape judgment (file count vs bytes) that the old size gate proxied. (q2) the receiver advertises a **rich capacity profile** (CPU cores, disk class, load, max streams, drain estimate) — "more data serves the ubergoal"; do not minimize the negotiation payload. (q3) engine type **deferred to the agent**, who recommends a new src/dst-agnostic `TransferEngine` + a local adapter over renaming `TransferOrchestrator` in place — ratified at `ue-1c`. (q4) `ue-2` (mid-transfer stream add/drop via PR3's resize proto) is **in scope at Active**, sequenced last; 11 months of owner benchmarking is the justification, the 10 GbE rig is sign-off not a gate.
- Why: owner answered the four gating questions (the stated Draft→Active condition) and said "active now." q1 materially improved the design — live-from-first-byte removes the fragile size threshold and collapses the A/B/C probe staging into "adjust what is cheap in `ue-1b`, add stream resize in `ue-2`."
- Inference flagged for owner (now vetoed — see D-2026-06-20-3): the agent had proposed folding the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b) in as the planner half and superseding its "after audit Round 1" timing. **Owner vetoed 2026-06-20.** The absorption is dropped; D-2026-06-04-3 stands unchanged. The engine's workload-shape-awareness + first-byte-within-~1s requirements remain, stated on their own merits, not as the H10b concept.
- Supersedes: the "A-first warmup probe" and "size-gated skip-probe" framings in the Draft version of `UNIFIED_TRANSFER_ENGINE.md` (already edited in-place). *(The proposed supersession of D-2026-06-04-3's streaming-planner timing is withdrawn per the owner veto — see D-2026-06-20-3.)*

## D-2026-06-20-3 — Veto: do NOT fold the streaming planner (H10b) into the unified engine
- Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
- Why: owner did not intend to revive H10b by way of the convergence plan; the inference was the agent's, flagged for confirmation, and the owner declined it. The workload-shape-awareness goal was always standalone and stands.
- Supersedes: nothing. Reverts the conditional H10b supersession that D-2026-06-20-2 had proposed (that entry is edited in-place to drop the inference and point here).

## D-2026-06-20-4 — Unified transfer engine plan review freeze
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is a Draft review candidate next to the original plan, and all unified-transfer-engine coding is frozen until the owner makes a final plan decision.
- Why: review found the Active plan's direction is sound but several slices need tightening before code starts: streaming initial planning was hidden inside `ue-1c`, local fast paths need to become engine-owned strategies, work-stealing is observable behavior, wire compatibility needs concrete shape, and pull parity gates must wait for multistream pull.
- Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.

## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` is the **Active** unified-transfer-engine plan (owner: "rev4 replaces v1"). `UNIFIED_TRANSFER_ENGINE.md` (v1) flips Active → Superseded; the intermediate review candidates `REV2.md` and `REV3.md` flip Draft → Superseded — all three superseded by REV4. REV4 carries v1's lineage/absorption header forward, so the supersessions v1 recorded (MULTISTREAM_PULL absorbed as the pull-multistream slice `ue-r2-1g`; PIPELINE_UNIFICATION/UNIFIED_RECEIVE_PIPELINE Historical) remain in force. The plan-review freeze (D-2026-06-20-4) is lifted as to the **plan decision**; coding still requires a fresh per-slice owner authorization (AGENTS.md §9) — no slice (`ue-r2-1a` first) starts on this decision alone.
- Why: REV4 is the only candidate whose code-reality section was verified against the tree (`HEAD` `09268eb`). REV3's headline "two static tables, not three" correction was itself wrong — all three stream-count ladders are live (`remote/tuning.rs::determine_remote_tuning`, `push/control.rs::desired_streams:476`, `pull.rs::pull_stream_count:904`), v1's three-ladder count was substantially right, and `tuning.rs`'s own doc comment confirms the daemon "runs its own ladder and wins". REV3 also wrongly said `determine_remote_tuning` drives local (it drives push + daemon pull) and conflated single-stream PullSync with the already-multistream deprecated Pull. REV4 = REV3 + corrected code reality, every symbol grounded with `file:line`, v1 lineage preserved. One Active plan avoids drift between candidates.
- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*

## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
- Why: the owner wants forward progress without rubber-stamp checkpoints. An external reviewer (GPT-5.5) catches what a single author misses, while Claude's adjudication guards against the reviewer's false positives — demonstrated necessary the same day: a codex-class review's confident "two static tables, not three" claim was wrong (all three ladders are live). Commits are low-risk and reversible (nothing publishes until the owner pushes), so per-commit gating was pure friction.
- Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.

## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
- Decision: The synchronous code→codex-review→fix loop (`docs/agent/GPT_REVIEW_LOOP.md`) now governs **every code change and every plan change** in this repo — owner, 2026-07-04: "use codex review loop for all code and plan changes", "NO EXCEPTIONS". The `.review/README.md` async two-agent hand-off (`ready/` sentinels + `reviewer-wait.sh` + a separate reviewer agent) is retired as the grading mechanism for new work; its record formats (`.review/findings/`, `.review/results/`, the `REVIEW.md` status index) remain in use by the codex loop. Reviewer identity on verdicts: `gpt-5.5` (codex), adjudicated by the coding agent per the loop's adjudication step. For docs/plan-only changes the validation gate is `bash scripts/agent/check-docs.sh` (the cargo suite is not required, per `.agents/repo-guidance.md` Verification); the review step still runs.
- Why: the codex loop demonstrably catches real defects (every `ue-r2-*` slice) while the async reviewer role sat structurally unfilled — w4-1 landed 2026-07-04 and immediately stalled at "awaiting reviewer verdict" with no reviewer in existence; a review mechanism that actually runs beats one that waits for an agent nobody spawns.
- Supersedes: the scope clause of D-2026-06-20-6 ("Scopes `.review/` usage for `ue-r2-*` only" — the loop is now repo-wide; D-2026-06-20-6's standing authorizations (a)/(b)/(c) carry over unchanged to the widened scope). Also supersedes `.review/README.md`'s sentinel/reviewer-wake sections and `docs/agent/PROTOCOL.md` `slice` step 2's sentinel requirement (both edited in place, annotated).

## D-2026-07-04-2 — Keep the `9f37a7a`/`48c5a11` staging-slip commits; no history rewrite
- Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
- Why: owner call 2026-07-04 ("leave as-is"). HEAD is fully gated and every later commit builds; the only cost is two skippable commits in bisect. Rewriting already-pushed shared history is the riskier operation — same calculus as D-2026-06-07-1, which is this repo's precedent for keeping a pushed wart over a second unsafe git operation.
- Supersedes: nothing (closes the STATE.md "commit erratum" blocked item).

## D-2026-07-04-3 — Flip `supports_cancellation` for Push/PullSync: CancelJob works on attached transfers
- Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
- Why: owner call 2026-07-04 ("flip it"). The original "disconnect is the cancel" rationale predates w4-3's race wiring; the flip is now policy-only, and cancel-from-anywhere (second terminal, TUI) is strictly more operable than find-and-kill-the-client.
- Supersedes: the DelegatedPull-only cancellation policy recorded in `active_jobs.rs`'s `supports_cancellation` rustdoc (edited when the slice lands) and the corresponding "policy deliberately unchanged" scope note in the w4-3 finding doc (which anticipated exactly this flip).

## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
- Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
- Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
- Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).

## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
- Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.

## D-2026-07-05-2 — No version compatibility, ever: same-build peers only
- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
- Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).

## D-2026-07-05-3 — Zero-copy receive unparked: revisit gate declared met (UNAS rig)
- Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
- Why: the 10 GbE session showed skippy's 32-core receiver at 1.43 cores — gate not met on that rig — but the gate was always about CPU-bound receivers, and the owner now operates one. On a CPU-bound receiver, cutting the userspace copy is exactly the FAST lever the eval preserved design notes for.
- Supersedes: the STATE Blocked "zero-copy option a/b/c" question and the "zero-copy revisit verdict" item among the four 10 GbE owner declarations (both resolved by this entry); amends D-2026-06-12-1's revisit-gate framing from "10 GbE benchmarks showing receive-side CPU saturation" to "a CPU-bound receiver exists" (annotated in the eval doc). D-2026-06-12-1's deletion of the dead module stands.

## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
- Why: the codex plan review completed (5 findings accepted + fixed, `496357d`); D-2026-07-05-2/-3 propagated; the owner's flip is the approval the plan procedure requires.
- Supersedes: nothing (the plan's "Active flip gets its own entry" placeholder now points here).

codex
The draft’s queue framing matches the latest decision ledger at first pass: D-2026-07-05-4 makes ONE_TRANSFER_PATH Active. I’m now checking the specific code claims and cited line references against the current tree.
exec
/usr/bin/zsh -lc "nl -ba docs/STATE.md | sed -n '1,240p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	# STATE — single entry point for "what is true right now"
     2	
     3	Last updated: 2026-07-06 (**otp-6 CLOSED; otp-7 in DESIGN**. otp-6 (a/b)
     4	mirror + filters landed + graded. Current slice **otp-7** (resume block
     5	phase, the plan's RELIABLE exception): owner chose "plan doc first" — slice
     6	design drafted at `docs/plan/OTP7_RESUME.md` (**Draft**), NO CODE until the
     7	owner answers its Q1–Q3 and flips it to Active. ONE_TRANSFER_PATH otp-1..6
     8	`[x]`. SMALL_FILE_CEILING stays paused, D-2026-07-05-1. This session: filed
     9	`audit-17`/`audit-18` to `TODO.md` only — no code, no Queue change
    10	(D-2026-07-05-4).)
    11	**Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
    12	local on top, unpushed — windows-latest CI check rides the next push.
    13	
    14	Rules: this file wins over every other doc (AGENTS.md §1). Keep it ≤ 200 lines and
    15	≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff`
    16	procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
    17	
    18	## Now (active work)
    19	
    20	- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
    21	  D-2026-07-05-4 "flip the plan and go") — otp-4a landed.** The
    22	  invariant (plan doc, verbatim): ONE block of transfer code;
    23	  direction/initiator/verb can NEVER affect wall time by blit's doing
    24	  — impossible by construction because the per-direction drivers and
    25	  `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
    26	  converge-up per cell (±10%); symmetric-fs disk-to-disk verdict
    27	  cells. **D-2026-07-05-2: same-build peers only, refusal at session
    28	  open.** Progress (each through the codex loop; closed-slice detail in
    29	  DEVLOG + `.review/` + REVIEW.md):
    30	  - **otp-1 / otp-3 / otp-4a `[x]`** — wire+session contract
    31	    (`docs/TRANSFER_SESSION.md`); role-parameterized drivers over the
    32	    in-process transport (invariance property in the role suite); daemon
    33	    serves `Transfer` as Responder, client push over gRPC; A/B
    34	    byte-identical vs old push; SizeMtime = data-safe skip (open Q below).
    35	  - **otp-4b (1/2/3) `[x]` — push data plane fully on the session, closed**:
    36	    single-stream TCP data plane, mid-transfer resize/multi-stream + sf-2
    37	    shape correction, deterministic mid-transfer cancel. Detail: DEVLOG.
    38	  - **otp-5a `[x]`** (`84be1cc`, codex PASS) — the one served `Transfer`
    39	    RPC serves BOTH roles via `run_responder` (SOURCE-init→daemon
    40	    DESTINATION = push; DEST-init→daemon SOURCE = pull, in-stream).
    41	  - **otp-5b (1/2) `[x]`** — the SOURCE-responder data plane, closed:
    42	    5b-1 (`e6a0b3b`+`13485ee`) decoupled connection role (RESPONDER
    43	    binds+accepts, INITIATOR dials) from byte role; 5b-2 (`d579365`+
    44	    `773a877`) lifted the single-stream cap — the pull data plane resizes
    45	    via sf-2 (same resize frames as push). Defaults to TCP; A/B
    46	    byte-identical vs old `pull_sync`. Suite → **1522**.
    47	  - **otp-6 (a/b) `[x]`** — mirror + filters on the session, closed.
    48	    6a (`c026692`+`0bb27f5`) honors `SessionOpen.filter` via the universal
    49	    `FilteredSource` chokepoint. 6b (`01d9c41`+`3c99557`) is the one delete
    50	    rule: DESTINATION diffs the complete source manifest at SourceDone,
    51	    scan-complete-guarded + filter-scoped. Codex High: keep-set now folds
    52	    case on macOS too (case-insensitive-FS data-loss). Suite → **1529**.
    53	  - Current: **otp-7 IN DESIGN** — Draft `docs/plan/OTP7_RESUME.md`
    54	    (`9fb5e4a`) awaiting owner review (see Open questions); no code until
    55	    Active. otp-5b-3 (pull cancel) optional; otp-2 rig-gated before otp-10.
    56	- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
    57	  `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`); **sf-3a+ blocked**
    58	  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
    59	  baseline. Principle stands: ceiling-driven, never competitor-relative
    60	  (D-2026-07-04-4; a ≥25% margin answer was retracted — do not
    61	  re-litigate). Evidence `docs/bench/10gbe-2026-07-05/`.
    62	- **Background (2026-07-04/05, all `[x]`)**: REV4 code-complete +
    63	  measurement gates DATA-COMPLETE (push/pull ≈ 9.5 of 9.88 Gbit/s; owner
    64	  declarations pending in Blocked); 10 GbE session done; w9-3 + review rows
    65	  landed. Codex loop governs all changes (D-2026-07-04-1; DEVLOG 07-04/05).
    66	
    67	## Queue (ordered)
    68	
    69	1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
    70	   the only work item until it ships**: slices otp-1..13 through the
    71	   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
    72	   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b) `[x]`. Current:
    73	   **otp-7 IN DESIGN** (`docs/plan/OTP7_RESUME.md` Draft, owner review;
    74	   no code until Active). otp-2 (symmetric baseline) is RIG-GATED —
    75	   before otp-10 cutover.
    76	2. **10 GbE owner declarations (still pending)**: ue-1, ue-2,
    77	   REV4 → Shipped (zero-copy resolved — D-2026-07-05-3). Optional
    78	   owner-gated measurement follow-ups (Win 11 bare-metal datapoint;
    79	   disk-path variants; >ARC-size push) — note the disk-path items
    80	   are largely absorbed by otp-2/otp-12's symmetric-rig matrices. Env: bench
    81	   binaries staged at `skippy:/mnt/generic-pool/video/blit-bin/`
    82	   (/tmp and /home on skippy are noexec).
    83	3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
    84	   resumes/re-derives after ONE_TRANSFER_PATH ships.
    85	4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
    86	   open row; filed w6-2a/b/c + relay-1) — same directive; note w7-1
    87	   (mirror-executor consolidation) likely lands for free inside
    88	   otp-6's one-delete-rule slice; re-check before picking it up.
    89	5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
    90	   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
    91	   cutover as a runtime-selected write strategy in the unified receive
    92	   sink (design: eval doc §If-FAST-evidence; dead module deletes in
    93	   w8-1). Rig facts + the aarch64-musl static build recipe: DEVLOG
    94	   2026-07-05 10:00. **Standing owner safety rule**: ALL activity on
    95	   rig `zoey` is confined to its `…/blit-temp/` folder — module roots,
    96	   test data, everything; nothing written outside it, ever. Zero-copy
    97	   is pre-authorized to be tested there when the post-cutover slice set
    98	   reaches it; no daemon runs on zoey before then without a fresh go.
    99	6. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
   100	   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
   101	   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
   102	   `derive_local_plan_tuning` fold-or-retire; receive-side dial
   103	   tuning residue (w3-1 scoped it out).
   104	
   105	## Authoritative docs right now
   106	
   107	- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
   108	  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Draft** — otp-7 slice
   109	  design, awaiting owner review before any code).
   110	- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
   111	  sf-2) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
   112	  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
   113	  (history only).
   114	- Process: `docs/agent/GPT_REVIEW_LOOP.md` (Active) — the codex loop
   115	  for **all code and plan changes** (D-2026-07-04-1); `.review/README.md`
   116	  is retired as the grading mechanism (its `findings/`/`results/`
   117	  records and the REVIEW.md index remain live).
   118	- Review loop: `REVIEW.md` (all `ue-r2-*` rows `[x]`; design-queue
   119	  rows) + `.review/findings/` + `.review/results/`.
   120	- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
   121	  D-2026-06-12-1, executes w8-1; **capability unparked
   122	  D-2026-07-05-3** — post-cutover write strategy), `TUI_REWORK.md`
   123	  (gated on Round 1),
   124	  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).
   125	
   126	## Blocked / waiting (all owner declarations; checkpoints are owner-only)
   127	
   128	- **Three 10 GbE gate declarations**: ue-1 pass/fail (evidence: band
   129	  holds), ue-2 pass/fail or re-scope (no organic resize at 10 GbE),
   130	  REV4 → Shipped. (The zero-copy revisit verdict and the a/b/c
   131	  question are RESOLVED — D-2026-07-05-3, unparked; measured skippy
   132	  data 1.43 cores daemon-receive / 0.45 client at 9.5 Gbit/s stays
   133	  recorded in DEVLOG + DIAGNOSIS.md.)
   134	- **Push go**: local commits `f6e592e`..HEAD await the ref-listing +
   135	  approval flow; windows-latest CI on the w9-3 harness fix rides it.
   136	- `Cargo.lock`: fresh transitive-dependency drift (crossbeam-*, cc, etc.)
   137	  sits uncommitted, same class as `04c9c6d` — not from this session;
   138	  owner's call to commit or revert.
   139	
   140	## Open questions
   141	
   142	- **(OPEN — owner review, 2026-07-07, otp-7)** `docs/plan/OTP7_RESUME.md`
   143	  (Draft) awaits the owner's Q1–Q3 (graceful stale fallback; in-place-patch
   144	  mid-failure model; 7a-then-7b staging — all agent-rec yes) and the flip to
   145	  Active. That flip unblocks otp-7 implementation.
   146	- **(OPEN — owner ack, 2026-07-05, otp-4a)** Unified SizeMtime semantic:
   147	  same-size + dest-NEWER — old push clobbers, the session adopts the
   148	  **data-safe SKIP** (converge-up; `--force` still overwrites; pinned by
   149	  `same_size_newer_destination_is_skipped_not_clobbered`). Owner: confirm,
   150	  or ask for old-push clobber. Reasoning:
   151	  `.review/findings/otp-4-daemon-serves-transfer.md`.
   152	- **(OPEN)** Historical docs embed `/Users/...` paths — agent rec: leave.
   153	- **(OPEN, 2026-07-04)** `725aa07` tracked a 236-file stale worktree
   154	  snapshot (`.claude/worktrees/vigilant-mayer/`). Agent rec: `git rm -r`;
   155	  awaits owner go.
   156	- **(OPEN, 2026-07-04)** `docs/WHITEPAPER.md` §§~309/606/641 still describe
   157	  the deleted `determine_remote_tuning`/`TuningParams` — fold into
   158	  w10-docs-batch (agent rec) or rewrite sooner?
   159	- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: the 10 GbE
   160	  session delivered the measurement evidence; flip awaits the three
   161	  declarations in Blocked (was four — zero-copy resolved,
   162	  D-2026-07-05-3).
   163	- **(OPEN, new 2026-07-05)** CLI foot-gun found during the session:
   164	  `blit copy src_large dst` with an existing local dir, no `./`,
   165	  parses the bare name as an mDNS discovery endpoint and errors
   166	  "remote source must include a module or root"
   167	  (blit-app endpoints.rs). Should local-path existence win over the
   168	  discovery interpretation, or at least improve the error? Candidate
   169	  review-queue row; owner to slot.
   170	- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: full suite green
   171	  locally; daemon-spawn e2e flakiness root-caused + fixed on Linux (w9-3:
   172	  port-TOCTOU race + cargo-lock contention). Remaining: windows-latest CI
   173	  on the next push (10d89e0 predates the w9-3 fix).
   174	
   175	## Handoff log (newest first, keep ≤ 3)
   176	
   177	- **2026-07-06 (37th)** @ `deb3800` — **Two owner bug reports triaged,
   178	  filed as `audit-17`/`audit-18` (TODO.md only, D-2026-07-05-4); no
   179	  code, otp-7 untouched.** Both: a `blit copy` that aborted entirely on
   180	  one bad filename mid-copy. `audit-17` (`5628c03`): dest FAT-family fs
   181	  rejects a `:` in a NuGet cache path (`sink.rs:605`, `os error 22`) — no
   182	  invalid-filename handling exists. `audit-18` (`deb3800`): a non-UTF-8
   183	  filename is irreversibly mangled by `relative_path_to_posix`'s
   184	  `to_string_lossy()` before I/O (`payload.rs:360`) — shared by local
   185	  mirror + remote push. Both need an owner `plan` before code.
   186	  **Exact first action next session**: otp-7 — owner's Q1–Q3 on
   187	  `docs/plan/OTP7_RESUME.md`, flip Active, codex-review, implement
   188	  otp-7a. In-flight: none. Done since 36th: nothing on
   189	  ONE_TRANSFER_PATH — pure bug triage.
   190	- **2026-07-07 (36th)** @ `9fb5e4a` — **otp-7 slice design drafted; owner
   191	  review pending (NO CODE)**. Owner picked "plan doc first" for the RELIABLE
   192	  resume slice; `docs/plan/OTP7_RESUME.md` (Draft) records the strict-ordering
   193	  block-hash choreography, the reuse map, design decisions (graceful stale
   194	  fallback, source block-diff as a session helper, in-place-patch mid-failure),
   195	  7a-in-stream/7b-data-plane staging, and the four guard-proof targets.
   196	  **Exact first action next session**: get the owner's Q1–Q3 answers + flip to
   197	  Active, then codex-review the plan change (D-2026-07-04-1) and implement
   198	  otp-7a. In-flight: none. Done since 35th: otp-6 fully closed; `f6e592e`..HEAD
   199	  unpushed (incl. otp-6a/6b + this design doc).

exec
/usr/bin/zsh -lc "nl -ba crates/blit-cli/src/diagnostics.rs | sed -n '1,260p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-cli/src/main.rs | sed -n '1,380p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	mod check;
     2	mod cli;
     3	mod completions;
     4	mod context;
     5	mod df;
     6	mod diagnostics;
     7	mod du;
     8	mod find;
     9	mod jobs;
    10	mod list_modules;
    11	mod ls;
    12	mod profile;
    13	mod rm;
    14	mod scan;
    15	mod transfers;
    16	
    17	use crate::check::run_check;
    18	use crate::cli::{Cli, Commands, DiagnosticsCommand};
    19	use crate::context::AppContext;
    20	use crate::diagnostics::{run_diagnostics_dump, run_diagnostics_perf};
    21	use crate::jobs::run_jobs;
    22	use crate::transfers::{run_move, run_transfer};
    23	use blit_app::transfers::dispatch::TransferKind;
    24	use blit_app::transfers::retry::run_with_retries;
    25	use blit_core::config;
    26	use clap::Parser;
    27	use eyre::Result;
    28	use std::process::ExitCode;
    29	use std::time::Duration;
    30	
    31	#[tokio::main]
    32	async fn main() -> Result<ExitCode> {
    33	    // w5-1: without a backend every log::warn!/error! in blit-core is
    34	    // silently discarded. Stderr, warn level, `blit: <level>: <msg>`.
    35	    blit_core::stderr_log::init("blit");
    36	    color_eyre::install()?;
    37	    let Cli {
    38	        config_dir,
    39	        diagnostics_counter_file,
    40	        command,
    41	    } = Cli::parse();
    42	
    43	    if let Some(dir) = config_dir.as_ref() {
    44	        config::set_config_dir(dir);
    45	    }
    46	
    47	    // audit-l39: pre-0.1.1 this was BLIT_TEST_COUNTER_FILE. Env vars
    48	    // are out for app + diagnostic config; install via the CLI flag.
    49	    if let Some(path) = diagnostics_counter_file {
    50	        blit_core::remote::instrumentation::set_counter_path(path);
    51	    }
    52	
    53	    let mut ctx = AppContext::load();
    54	
    55	    match command {
    56	        Commands::Copy(args) => {
    57	            let wait = Duration::from_secs(args.wait);
    58	            run_with_retries(args.retry, wait, |_n| {
    59	                run_transfer(&ctx, &args, TransferKind::Copy)
    60	            })
    61	            .await?
    62	        }
    63	        Commands::Mirror(args) => {
    64	            let wait = Duration::from_secs(args.wait);
    65	            run_with_retries(args.retry, wait, |_n| {
    66	                run_transfer(&ctx, &args, TransferKind::Mirror)
    67	            })
    68	            .await?
    69	        }
    70	        Commands::Move(args) => {
    71	            let wait = Duration::from_secs(args.wait);
    72	            run_with_retries(args.retry, wait, |_n| run_move(&ctx, &args)).await?
    73	        }
    74	        Commands::Scan(args) => scan::run_scan(args).await?,
    75	        Commands::ListModules(args) => list_modules::run_list_modules(args).await?,
    76	        Commands::Ls(args) => ls::run_ls(args).await?,
    77	        Commands::Du(args) => du::run_du(args).await?,
    78	        Commands::Df(args) => df::run_df(args).await?,
    79	        Commands::Rm(args) => rm::run_rm(args).await?,
    80	        Commands::Find(args) => find::run_find(args).await?,
    81	        Commands::Completions(args) => completions::run_completions(args).await?,
    82	        Commands::Profile(args) => profile::run_profile(args)?,
    83	        // `check` is the only command whose exit code carries semantic
    84	        // info (0 identical / 1 differences / 2 errors), so we propagate
    85	        // it directly. Other commands return Ok(()) and use the default 0.
    86	        Commands::Check(args) => return run_check(&args).await,
    87	        Commands::Diagnostics { command } => match command {
    88	            DiagnosticsCommand::Perf(args) => run_diagnostics_perf(&mut ctx, &args)?,
    89	            DiagnosticsCommand::Dump(args) => run_diagnostics_dump(&args)?,
    90	        },
    91	        // `jobs cancel` exits 0/1/2 (Cancelled / NotFound /
    92	        // Unsupported) per the §6.5 contract; `jobs list`
    93	        // always exits 0. The runner returns the right
    94	        // `ExitCode` for both; propagate it directly like
    95	        // `check`.
    96	        Commands::Jobs { command } => return run_jobs(command).await,
    97	    }
    98	
    99	    Ok(ExitCode::SUCCESS)
   100	}

 succeeded in 0ms:
     1	use crate::cli::{DiagnosticsDumpArgs, PerfArgs};
     2	use crate::context::AppContext;
     3	use blit_app::diagnostics::dump::{endpoint_display, endpoint_snapshot, same_device};
     4	use blit_app::diagnostics::perf;
     5	use blit_app::endpoints::parse_transfer_endpoint;
     6	use blit_app::transfers::resolution::{dest_is_container, resolve_destination, source_is_contents};
     7	use chrono::{DateTime, Utc};
     8	use eyre::Result;
     9	use serde_json::{json, Value};
    10	use std::time::{Duration, UNIX_EPOCH};
    11	
    12	pub fn run_diagnostics_perf(ctx: &mut AppContext, args: &PerfArgs) -> Result<()> {
    13	    if args.enable {
    14	        perf::set_enabled(true)?;
    15	        ctx.perf_history_enabled = true;
    16	        println!("Performance history enabled (persisted).");
    17	    }
    18	
    19	    if args.disable {
    20	        perf::set_enabled(false)?;
    21	        ctx.perf_history_enabled = false;
    22	        println!("Performance history disabled (persisted).");
    23	    }
    24	
    25	    if args.clear {
    26	        match perf::clear()? {
    27	            true => println!("Cleared performance history log."),
    28	            false => println!("No performance history log to clear."),
    29	        }
    30	    }
    31	
    32	    // Best-effort refresh of the cached enabled flag — matches
    33	    // pre-A.0 semantics. A malformed `settings.json` here doesn't
    34	    // block the rest of the verb: the startup warning already
    35	    // surfaced the parse error, and we fall back to whatever
    36	    // value AppContext loaded with.
    37	    if let Ok(enabled) = perf::read_enabled() {
    38	        ctx.perf_history_enabled = enabled;
    39	    }
    40	
    41	    let history_path = perf::history_path()?;
    42	    let records = perf::read_records(args.limit)?;
    43	
    44	    if args.json {
    45	        let output = json!({
    46	            "enabled": ctx.perf_history_enabled,
    47	            "history_path": history_path.to_string_lossy(),
    48	            "record_count": records.len(),
    49	            "records": records,
    50	        });
    51	        println!("{}", serde_json::to_string_pretty(&output)?);
    52	        return Ok(());
    53	    }
    54	
    55	    println!(
    56	        "Performance history (showing up to {} entries): {}",
    57	        args.limit,
    58	        records.len()
    59	    );
    60	    println!("History file: {}", history_path.display());
    61	    println!(
    62	        "Status: {}",
    63	        if ctx.perf_history_enabled {
    64	            if records.is_empty() {
    65	                "enabled (no entries yet)"
    66	            } else {
    67	                "enabled"
    68	            }
    69	        } else {
    70	            "disabled via CLI settings"
    71	        }
    72	    );
    73	
    74	    if records.is_empty() {
    75	        return Ok(());
    76	    }
    77	
    78	    let total_runs = records.len();
    79	    let total_runs_f64 = total_runs as f64;
    80	    let avg_planner = records
    81	        .iter()
    82	        .map(|r| r.planner_duration_ms as f64)
    83	        .sum::<f64>()
    84	        / total_runs_f64;
    85	    let avg_transfer = records
    86	        .iter()
    87	        .map(|r| r.transfer_duration_ms as f64)
    88	        .sum::<f64>()
    89	        / total_runs_f64;
    90	    let fast_path_runs = records.iter().filter(|r| r.fast_path.is_some()).count();
    91	    let fast_pct = if total_runs == 0 {
    92	        0.0
    93	    } else {
    94	        100.0 * fast_path_runs as f64 / total_runs_f64
    95	    };
    96	
    97	    println!(
    98	        "Fast-path runs: {} ({:.1}%), streaming runs: {}",
    99	        fast_path_runs,
   100	        fast_pct,
   101	        total_runs - fast_path_runs
   102	    );
   103	    println!(
   104	        "Average planner: {:.1} ms | Average transfer: {:.1} ms",
   105	        avg_planner, avg_transfer
   106	    );
   107	
   108	    if let Some(last) = records.last() {
   109	        let millis = last.timestamp_epoch_ms.min(u64::MAX as u128) as u64;
   110	        let timestamp = DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_millis(millis));
   111	        let mode = match last.mode {
   112	            blit_core::perf_history::TransferMode::Copy => "copy",
   113	            blit_core::perf_history::TransferMode::Mirror => "mirror",
   114	        };
   115	        let fast_path_label = last.fast_path.as_deref().unwrap_or("streaming");
   116	
   117	        println!("Most recent run:");
   118	        println!(
   119	            "  Timestamp : {}",
   120	            timestamp.format("%Y-%m-%d %H:%M:%S UTC")
   121	        );
   122	        println!("  Mode      : {}", mode);
   123	        println!("  Fast path : {}", fast_path_label);
   124	        println!(
   125	            "  Planned   : {} file(s), {} bytes",
   126	            last.file_count, last.total_bytes
   127	        );
   128	        println!(
   129	            "  Planner   : {} ms | Transfer: {} ms",
   130	            last.planner_duration_ms, last.transfer_duration_ms
   131	        );
   132	        println!(
   133	            "  Options   : checksum={} skip_unchanged={} workers={}",
   134	            last.options.checksum, last.options.skip_unchanged, last.options.workers
   135	        );
   136	        if let Some(fs) = &last.source_fs {
   137	            println!("  Source FS : {}", fs);
   138	        }
   139	        if let Some(fs) = &last.dest_fs {
   140	            println!("  Dest FS   : {}", fs);
   141	        }
   142	    }
   143	
   144	    Ok(())
   145	}
   146	
   147	/// Emit a diagnostic snapshot for a SRC/DEST pair without performing a
   148	/// transfer. Motivation: bug reporters and bisectors need a consistent
   149	/// way to answer "what did blit see when you ran this?" — parse results,
   150	/// rsync destination resolution, filesystem caps, disk space — without
   151	/// reading source. One invocation → a single pasteable blob.
   152	///
   153	/// The per-endpoint snapshot helpers (`endpoint_snapshot`,
   154	/// `endpoint_display`, `same_device`) live in
   155	/// `blit_app::diagnostics::dump`; the rsync-resolution helpers
   156	/// (`source_is_contents`, `dest_is_container`,
   157	/// `resolve_destination`) live in `blit_app::transfers::resolution`.
   158	/// Both sets are imported directly at the top of this file; this
   159	/// function orchestrates them.
   160	pub fn run_diagnostics_dump(args: &DiagnosticsDumpArgs) -> Result<()> {
   161	    let src_endpoint = parse_transfer_endpoint(&args.source)?;
   162	    let raw_dst = parse_transfer_endpoint(&args.destination)?;
   163	    let pre_resolve_dst = raw_dst.clone();
   164	    let resolved_dst = resolve_destination(&args.source, &args.destination, &src_endpoint, raw_dst);
   165	
   166	    let source_contents_mode = source_is_contents(&args.source);
   167	    let dest_is_container_flag = dest_is_container(&args.destination, &pre_resolve_dst);
   168	
   169	    let src_json = endpoint_snapshot(&args.source, &src_endpoint);
   170	    let dst_json = endpoint_snapshot(&args.destination, &resolved_dst);
   171	    let pre_resolve_json = endpoint_display(&pre_resolve_dst);
   172	    let resolved_display = endpoint_display(&resolved_dst);
   173	
   174	    let same_device_result = same_device(&src_endpoint, &resolved_dst);
   175	
   176	    let output = json!({
   177	        "blit_version": env!("CARGO_PKG_VERSION"),
   178	        "invocation": std::env::args().collect::<Vec<_>>(),
   179	        "source": src_json,
   180	        "destination": dst_json,
   181	        "rsync_resolution": {
   182	            "source_is_contents": source_contents_mode,
   183	            "destination_is_container": dest_is_container_flag,
   184	            "pre_resolve_destination": pre_resolve_json,
   185	            "resolved_destination": resolved_display,
   186	            "resolution_changed": pre_resolve_json != resolved_display,
   187	        },
   188	        "same_device": same_device_result,
   189	    });
   190	
   191	    if args.json {
   192	        println!("{}", serde_json::to_string_pretty(&output)?);
   193	    } else {
   194	        print_dump_human(&output);
   195	    }
   196	    Ok(())
   197	}
   198	
   199	fn print_dump_human(v: &Value) {
   200	    println!("blit diagnostics dump");
   201	    println!(
   202	        "  version     : {}",
   203	        v["blit_version"].as_str().unwrap_or("?")
   204	    );
   205	    if let Some(invocation) = v["invocation"].as_array() {
   206	        let joined: Vec<&str> = invocation.iter().filter_map(|s| s.as_str()).collect();
   207	        println!("  invocation  : {}", joined.join(" "));
   208	    }
   209	    println!();
   210	    println!("Source");
   211	    print_endpoint_human(&v["source"]);
   212	    println!();
   213	    println!("Destination");
   214	    print_endpoint_human(&v["destination"]);
   215	    println!();
   216	    let res = &v["rsync_resolution"];
   217	    println!("Rsync resolution");
   218	    println!(
   219	        "  source_is_contents     : {}",
   220	        res["source_is_contents"].as_bool().unwrap_or(false)
   221	    );
   222	    println!(
   223	        "  destination_is_container: {}",
   224	        res["destination_is_container"].as_bool().unwrap_or(false)
   225	    );
   226	    println!(
   227	        "  pre_resolve_destination: {}",
   228	        res["pre_resolve_destination"].as_str().unwrap_or("?")
   229	    );
   230	    println!(
   231	        "  resolved_destination   : {}",
   232	        res["resolved_destination"].as_str().unwrap_or("?")
   233	    );
   234	    println!(
   235	        "  resolution_changed     : {}",
   236	        res["resolution_changed"].as_bool().unwrap_or(false)
   237	    );
   238	    if let Some(same) = v["same_device"].as_bool() {
   239	        println!();
   240	        println!("Transport hints");
   241	        println!("  same_device (local-only): {}", same);
   242	    }
   243	}
   244	
   245	fn print_endpoint_human(v: &Value) {
   246	    if v["kind"] == "local" {
   247	        println!("  raw            : {}", v["raw"].as_str().unwrap_or("?"));
   248	        println!(
   249	            "  absolute_path  : {}",
   250	            v["absolute_path"].as_str().unwrap_or("(not canonicalized)")
   251	        );
   252	        println!(
   253	            "  exists         : {}  is_file={} is_dir={}",
   254	            v["exists"].as_bool().unwrap_or(false),
   255	            v["is_file"].as_bool().unwrap_or(false),
   256	            v["is_dir"].as_bool().unwrap_or(false)
   257	        );
   258	        if let Some(size) = v["size_bytes"].as_u64() {
   259	            println!("  size_bytes     : {}", size);
   260	        }

exec
/usr/bin/zsh -lc 'rg -n "record_performance_history|PerformanceRecord|error_count|TransferMode|perf_local|DEFAULT_MAX_BYTES|read_performance|clear_performance" crates/blit-core crates/blit-cli' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-cli/src/diagnostics.rs:112:            blit_core::perf_history::TransferMode::Copy => "copy",
crates/blit-cli/src/diagnostics.rs:113:            blit_core::perf_history::TransferMode::Mirror => "mirror",
crates/blit-core/tests/predictor_streaming.rs:9:    CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
crates/blit-core/tests/predictor_streaming.rs:47:fn seed_record(file_count: usize, total_bytes: u64, planner_ms: u128) -> PerformanceRecord {
crates/blit-core/tests/predictor_streaming.rs:48:    PerformanceRecord::new(
crates/blit-core/tests/predictor_streaming.rs:49:        TransferMode::Copy,
crates/blit-core/src/engine/mod.rs:53:use crate::perf_history::{read_recent_records, TransferMode};
crates/blit-core/src/engine/mod.rs:62:use self::history::{record_performance_history, update_predictor};
crates/blit-core/src/engine/mod.rs:220:            if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:277:                    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:314:                    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:350:                    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:404:            // (DEFAULT_MAX_BYTES in perf_history.rs), so reading
crates/blit-core/src/engine/mod.rs:408:                TransferMode::Mirror
crates/blit-core/src/engine/mod.rs:410:                TransferMode::Copy
crates/blit-core/src/engine/mod.rs:563:        // BEFORE observe() (record_performance_history →
crates/blit-core/src/engine/mod.rs:568:        // here; `record_performance_history` populates the matching
crates/blit-core/src/engine/mod.rs:569:        // `PerformanceRecord.{file_count,total_bytes}` from
crates/blit-core/src/engine/mod.rs:590:                crate::perf_history::TransferMode::Mirror
crates/blit-core/src/engine/mod.rs:592:                crate::perf_history::TransferMode::Copy
crates/blit-core/src/engine/mod.rs:792:        if let Some(record) = record_performance_history(
crates/blit-core/src/engine/single_file.rs:14:use super::history::{record_performance_history, update_predictor};
crates/blit-core/src/engine/single_file.rs:42:    if let Some(record) = record_performance_history(
crates/blit-core/src/perf_predictor.rs:15:use crate::perf_history::{config_dir, PerformanceRecord, TransferMode};
crates/blit-core/src/perf_predictor.rs:161:    mode: TransferMode,
crates/blit-core/src/perf_predictor.rs:168:    fn new(record: &PerformanceRecord) -> Self {
crates/blit-core/src/perf_predictor.rs:182:        mode: TransferMode,
crates/blit-core/src/perf_predictor.rs:242:        mode: TransferMode,
crates/blit-core/src/perf_predictor.rs:330:    pub fn predict_planner(&self, record: &PerformanceRecord) -> Option<Prediction> {
crates/blit-core/src/perf_predictor.rs:346:    pub fn predict_transfer(&self, record: &PerformanceRecord) -> Option<Prediction> {
crates/blit-core/src/perf_predictor.rs:363:    pub fn predict_total(&self, record: &PerformanceRecord) -> Option<Prediction> {
crates/blit-core/src/perf_predictor.rs:390:    pub fn observe(&mut self, record: &PerformanceRecord) {
crates/blit-core/src/perf_predictor.rs:420:        mode: TransferMode,
crates/blit-core/src/perf_predictor.rs:494:    ) -> Result<Vec<PerformanceRecord>> {
crates/blit-core/src/perf_predictor.rs:500:            if let Ok(record) = serde_json::from_str::<PerformanceRecord>(&line) {
crates/blit-core/src/perf_predictor.rs:607:        mode: TransferMode,
crates/blit-core/src/perf_predictor.rs:611:    ) -> PerformanceRecord {
crates/blit-core/src/perf_predictor.rs:629:        mode: TransferMode,
crates/blit-core/src/perf_predictor.rs:637:    ) -> PerformanceRecord {
crates/blit-core/src/perf_predictor.rs:638:        PerformanceRecord {
crates/blit-core/src/perf_predictor.rs:660:            error_count: 0,
crates/blit-core/src/perf_predictor.rs:672:    fn predict_planner_ms(predictor: &PerformancePredictor, record: &PerformanceRecord) -> f64 {
crates/blit-core/src/perf_predictor.rs:713:                TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:722:            &make_record(TransferMode::Copy, file_count, total_bytes, 0),
crates/blit-core/src/perf_predictor.rs:751:                TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:759:            &make_record(TransferMode::Copy, file_count, total_bytes, 0),
crates/blit-core/src/perf_predictor.rs:765:                TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:773:            &make_record(TransferMode::Copy, file_count, total_bytes, 0),
crates/blit-core/src/perf_predictor.rs:795:                TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:803:                TransferMode::Mirror,
crates/blit-core/src/perf_predictor.rs:812:            &make_record(TransferMode::Copy, file_count, total_bytes, 0),
crates/blit-core/src/perf_predictor.rs:816:            &make_record(TransferMode::Mirror, file_count, total_bytes, 0),
crates/blit-core/src/perf_predictor.rs:833:            predictor.observe(&make_record(TransferMode::Copy, 200, 50 * 1024 * 1024, 250));
crates/blit-core/src/perf_predictor.rs:838:            &make_record(TransferMode::Copy, 200, 50 * 1024 * 1024, 0),
crates/blit-core/src/perf_predictor.rs:852:            &make_record(TransferMode::Copy, 200, 50 * 1024 * 1024, 0),
crates/blit-core/src/perf_predictor.rs:868:        let result = predictor.predict_planner(&make_record(TransferMode::Copy, 100, 1024, 50));
crates/blit-core/src/perf_predictor.rs:883:        predictor.observe(&make_record(TransferMode::Copy, 100, 1024, 50));
crates/blit-core/src/perf_predictor.rs:884:        let result = predictor.predict_planner(&make_record(TransferMode::Copy, 100, 1024, 0));
crates/blit-core/src/perf_predictor.rs:898:            predictor.observe(&make_record(TransferMode::Copy, 100, 1024, 50));
crates/blit-core/src/perf_predictor.rs:901:            .predict_planner(&make_record(TransferMode::Copy, 100, 1024, 0))
crates/blit-core/src/perf_predictor.rs:917:            predictor.observe(&make_record(TransferMode::Copy, 100, 0, 100));
crates/blit-core/src/perf_predictor.rs:918:            predictor.observe(&make_record(TransferMode::Copy, 1000, 0, 1000));
crates/blit-core/src/perf_predictor.rs:921:        let pred_100 = predict_planner_ms(&predictor, &make_record(TransferMode::Copy, 100, 0, 0));
crates/blit-core/src/perf_predictor.rs:923:            predict_planner_ms(&predictor, &make_record(TransferMode::Copy, 1000, 0, 0));
crates/blit-core/src/perf_predictor.rs:949:                TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:960:        let probe = make_record(TransferMode::Copy, file_count, total_bytes, 0);
crates/blit-core/src/perf_predictor.rs:989:                TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1000:        let probe = make_record(TransferMode::Copy, file_count, total_bytes, 0);
crates/blit-core/src/perf_predictor.rs:1028:                TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1042:            TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1060:            TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1116:    ) -> PerformanceRecord {
crates/blit-core/src/perf_predictor.rs:1130:        let mut record = PerformanceRecord::new(
crates/blit-core/src/perf_predictor.rs:1131:            TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1168:                    TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1197:                    TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1233:                    TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1263:                TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1309:            mode: TransferMode::Copy,
crates/blit-core/src/perf_predictor.rs:1350:            mode: TransferMode::Copy,
crates/blit-core/src/perf_history.rs:17:const DEFAULT_MAX_BYTES: u64 = 1_000_000; // ~1 MiB cap per design docs
crates/blit-core/src/perf_history.rs:20:/// Current schema version for PerformanceRecord.
crates/blit-core/src/perf_history.rs:46:pub enum TransferMode {
crates/blit-core/src/perf_history.rs:51:/// Measurement lane for a [`PerformanceRecord`]. Determines whether the
crates/blit-core/src/perf_history.rs:135:pub struct PerformanceRecord {
crates/blit-core/src/perf_history.rs:139:    pub mode: TransferMode,
crates/blit-core/src/perf_history.rs:157:    pub error_count: u32,
crates/blit-core/src/perf_history.rs:176:impl PerformanceRecord {
crates/blit-core/src/perf_history.rs:179:        mode: TransferMode,
crates/blit-core/src/perf_history.rs:189:        error_count: u32,
crates/blit-core/src/perf_history.rs:221:            error_count,
crates/blit-core/src/perf_history.rs:239:pub fn append_local_record(record: &PerformanceRecord) -> Result<()> {
crates/blit-core/src/perf_history.rs:264:    enforce_size_cap(&path, DEFAULT_MAX_BYTES)?;
crates/blit-core/src/perf_history.rs:273:pub fn migrate_record(mut record: PerformanceRecord) -> PerformanceRecord {
crates/blit-core/src/perf_history.rs:302:pub fn read_recent_records(limit: usize) -> Result<Vec<PerformanceRecord>> {
crates/blit-core/src/perf_history.rs:307:fn read_records_from_path(path: &Path, limit: usize) -> Result<Vec<PerformanceRecord>> {
crates/blit-core/src/perf_history.rs:321:        if let Ok(record) = serde_json::from_str::<PerformanceRecord>(&line) {
crates/blit-core/src/perf_history.rs:362:    Ok(config_dir()?.join("perf_local.jsonl"))
crates/blit-core/src/perf_history.rs:500:        r#"{"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":10,"total_bytes":1024,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":true,"checksum":false,"workers":4},"fast_path":null,"planner_duration_ms":50,"transfer_duration_ms":200,"stall_events":0,"error_count":0}"#
crates/blit-core/src/perf_history.rs:504:        r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"mirror","source_fs":"apfs","dest_fs":"apfs","file_count":5,"total_bytes":512,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":true,"workers":2},"fast_path":"tiny","planner_duration_ms":10,"transfer_duration_ms":100,"stall_events":0,"error_count":0,"tar_shard_tasks":1,"tar_shard_files":5,"tar_shard_bytes":512,"raw_bundle_tasks":0,"raw_bundle_files":0,"raw_bundle_bytes":0,"large_tasks":0,"large_bytes":0}"#
crates/blit-core/src/perf_history.rs:509:        let record: PerformanceRecord =
crates/blit-core/src/perf_history.rs:518:        let record: PerformanceRecord =
crates/blit-core/src/perf_history.rs:522:        assert_eq!(record.mode, TransferMode::Mirror);
crates/blit-core/src/perf_history.rs:527:        let old: PerformanceRecord =
crates/blit-core/src/perf_history.rs:555:        assert_eq!(records[0].mode, TransferMode::Copy);
crates/blit-core/src/perf_history.rs:556:        assert_eq!(records[1].mode, TransferMode::Mirror);
crates/blit-core/src/perf_history.rs:586:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:587:            TransferMode::Copy,
crates/blit-core/src/perf_history.rs:625:        let record: PerformanceRecord =
crates/blit-core/src/perf_history.rs:628:        assert_eq!(migrated.mode, TransferMode::Copy);
crates/blit-core/src/perf_history.rs:640:        let record: PerformanceRecord =
crates/blit-core/src/perf_history.rs:645:            TransferMode::Mirror,
crates/blit-core/src/perf_history.rs:659:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":true,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":null,"planner_duration_ms":5,"transfer_duration_ms":0,"stall_events":0,"error_count":0}"#;
crates/blit-core/src/perf_history.rs:660:        let record: PerformanceRecord = serde_json::from_str(json).expect("deserialize v1 dry-run");
crates/blit-core/src/perf_history.rs:667:        assert_eq!(migrated.mode, TransferMode::Copy);
crates/blit-core/src/perf_history.rs:673:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":"null_sink","planner_duration_ms":5,"transfer_duration_ms":2,"stall_events":0,"error_count":0}"#;
crates/blit-core/src/perf_history.rs:674:        let record: PerformanceRecord =
crates/blit-core/src/perf_history.rs:698:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:699:            TransferMode::Mirror,
crates/blit-core/src/perf_history.rs:712:        assert_eq!(record.mode, TransferMode::Mirror);
crates/blit-core/src/perf_history.rs:726:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:727:            TransferMode::Copy,
crates/blit-core/src/perf_history.rs:753:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:754:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:6:use crate::perf_history::TransferMode;
crates/blit-core/src/engine/tuning.rs:25:    history: &[crate::perf_history::PerformanceRecord],
crates/blit-core/src/engine/tuning.rs:26:    target_mode: TransferMode,
crates/blit-core/src/engine/tuning.rs:29:) -> Vec<crate::perf_history::PerformanceRecord> {
crates/blit-core/src/engine/tuning.rs:65:/// (DEFAULT_MAX_BYTES in perf_history.rs), so reading all
crates/blit-core/src/engine/tuning.rs:75:    target_mode: TransferMode,
crates/blit-core/src/engine/tuning.rs:78:) -> Option<Vec<crate::perf_history::PerformanceRecord>>
crates/blit-core/src/engine/tuning.rs:80:    F: FnOnce(usize) -> Result<Vec<crate::perf_history::PerformanceRecord>>,
crates/blit-core/src/engine/tuning.rs:106:        CompareModeSnapshot, OptionSnapshot, PerformanceRecord, RunKind, TransferMode,
crates/blit-core/src/engine/tuning.rs:112:        mode: TransferMode,
crates/blit-core/src/engine/tuning.rs:116:    ) -> PerformanceRecord {
crates/blit-core/src/engine/tuning.rs:117:        let mut r = PerformanceRecord::new(
crates/blit-core/src/engine/tuning.rs:160:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:170:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:179:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:205:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:214:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:222:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:241:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:250:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:259:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:267:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:282:                    TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:291:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:314:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:325:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:335:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:364:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:374:            let mut r = record(RunKind::Real, TransferMode::Copy, 0, 0, 10_000 + i);
crates/blit-core/src/engine/tuning.rs:389:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:427:        records: Vec<PerformanceRecord>,
crates/blit-core/src/engine/tuning.rs:428:    ) -> impl FnOnce(usize) -> Result<Vec<PerformanceRecord>> {
crates/blit-core/src/engine/tuning.rs:441:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:455:        let reader = |_limit: usize| -> Result<Vec<PerformanceRecord>> {
crates/blit-core/src/engine/tuning.rs:460:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:473:                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 100),
crates/blit-core/src/engine/tuning.rs:474:                record(RunKind::NullSink, TransferMode::Copy, 4, 1024 * 1024, 200),
crates/blit-core/src/engine/tuning.rs:479:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:492:                record(RunKind::Real, TransferMode::Copy, 4, 16 * 1024 * 1024, 100),
crates/blit-core/src/engine/tuning.rs:493:                record(RunKind::DryRun, TransferMode::Copy, 4, 1024 * 1024, 200),
crates/blit-core/src/engine/tuning.rs:498:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:518:                TransferMode::Mirror,
crates/blit-core/src/engine/tuning.rs:527:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:534:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:539:        assert_eq!(window[0].mode, TransferMode::Copy);
crates/blit-core/src/engine/tuning.rs:554:                TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:563:        let mut sm = record(RunKind::Real, TransferMode::Copy, 2, 8 * 1024 * 1024, 500);
crates/blit-core/src/engine/tuning.rs:570:            TransferMode::Copy,
crates/blit-core/src/engine/tuning.rs:583:            TransferMode::Copy,
crates/blit-core/src/engine/history.rs:2:    append_local_record, CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
crates/blit-core/src/engine/history.rs:17:pub(super) fn record_performance_history(
crates/blit-core/src/engine/history.rs:23:) -> Option<PerformanceRecord> {
crates/blit-core/src/engine/history.rs:44:/// Construct the `PerformanceRecord` from a summary without
crates/blit-core/src/engine/history.rs:45:/// touching disk. Split out from `record_performance_history` so
crates/blit-core/src/engine/history.rs:55:) -> PerformanceRecord {
crates/blit-core/src/engine/history.rs:67:        TransferMode::Mirror
crates/blit-core/src/engine/history.rs:69:        TransferMode::Copy
crates/blit-core/src/engine/history.rs:87:    let mut record = PerformanceRecord::new(
crates/blit-core/src/engine/history.rs:114:    record: &PerformanceRecord,
crates/blit-core/src/remote/push/client/mod.rs:5:pub use types::{RemotePushProgress, RemotePushReport, TransferMode};
crates/blit-core/src/remote/push/client/mod.rs:727:            TransferMode::Fallback
crates/blit-core/src/remote/push/client/mod.rs:729:            TransferMode::Undecided
crates/blit-core/src/remote/push/client/mod.rs:816:                                    if !matches!(transfer_mode, TransferMode::Fallback) {
crates/blit-core/src/remote/push/client/mod.rs:828:                                        TransferMode::Fallback => {
crates/blit-core/src/remote/push/client/mod.rs:861:                                        TransferMode::DataPlane => {
crates/blit-core/src/remote/push/client/mod.rs:942:                                        TransferMode::Undecided => {}
crates/blit-core/src/remote/push/client/mod.rs:949:                                        transfer_mode = TransferMode::Fallback;
crates/blit-core/src/remote/push/client/mod.rs:1099:                                        transfer_mode = TransferMode::DataPlane;
crates/blit-core/src/remote/push/client/mod.rs:1246:                                TransferMode::Fallback => {
crates/blit-core/src/remote/push/client/mod.rs:1277:                                TransferMode::DataPlane => {
crates/blit-core/src/remote/push/client/mod.rs:1342:                                TransferMode::Undecided => {}
crates/blit-core/src/remote/push/client/mod.rs:1453:            if matches!(transfer_mode, TransferMode::Fallback)
crates/blit-core/src/remote/push/client/mod.rs:1472:            if matches!(transfer_mode, TransferMode::DataPlane)
crates/blit-core/src/remote/push/client/types.rs:20:pub enum TransferMode {
crates/blit-core/src/auto_tune/mod.rs:31:    records: &[crate::perf_history::PerformanceRecord],
crates/blit-core/src/auto_tune/mod.rs:85:    use crate::perf_history::{OptionSnapshot, PerformanceRecord, TransferMode};
crates/blit-core/src/auto_tune/mod.rs:93:    ) -> PerformanceRecord {
crates/blit-core/src/auto_tune/mod.rs:94:        let mut record = PerformanceRecord::new(
crates/blit-core/src/auto_tune/mod.rs:95:            TransferMode::Copy,
crates/blit-core/src/auto_tune/mod.rs:155:    ) -> PerformanceRecord {

codex
So far the main chokepoint description is accurate: `Copy` and `Mirror` both immediately `?` the `run_with_retries(...).await` result. I’m checking whether the success-path/perf-history claims are equally exact.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/history.rs | sed -n '1,170p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use crate::perf_history::{
     2	    append_local_record, CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
     3	};
     4	use crate::perf_predictor::PerformancePredictor;
     5	
     6	use super::{LocalMirrorOptions, LocalMirrorSummary};
     7	
     8	/// Map the orchestrator's `LocalCompareMode` onto the perf-history
     9	/// snapshot enum so tuning records preserve the user's full intent
    10	/// (not just `checksum: bool`).
    11	fn snapshot_compare_mode(options: &LocalMirrorOptions) -> CompareModeSnapshot {
    12	    options
    13	        .compare_mode
    14	        .resolve_compare_snapshot(options.checksum)
    15	}
    16	
    17	pub(super) fn record_performance_history(
    18	    summary: &LocalMirrorSummary,
    19	    options: &LocalMirrorOptions,
    20	    fast_path: Option<&str>,
    21	    planner_duration_ms: u128,
    22	    transfer_duration_ms: u128,
    23	) -> Option<PerformanceRecord> {
    24	    if !options.perf_history {
    25	        return None;
    26	    }
    27	
    28	    let record = build_performance_record(
    29	        summary,
    30	        options,
    31	        fast_path,
    32	        planner_duration_ms,
    33	        transfer_duration_ms,
    34	    );
    35	
    36	    if let Err(err) = append_local_record(&record) {
    37	        if options.verbose {
    38	            eprintln!("Failed to update performance history: {err:?}");
    39	        }
    40	    }
    41	    Some(record)
    42	}
    43	
    44	/// Construct the `PerformanceRecord` from a summary without
    45	/// touching disk. Split out from `record_performance_history` so
    46	/// the record-shape contract — specifically R44-F1's "train and
    47	/// query against the same feature vector" invariant — is
    48	/// unit-testable without writing to the global perf history file.
    49	fn build_performance_record(
    50	    summary: &LocalMirrorSummary,
    51	    options: &LocalMirrorOptions,
    52	    fast_path: Option<&str>,
    53	    planner_duration_ms: u128,
    54	    transfer_duration_ms: u128,
    55	) -> PerformanceRecord {
    56	    let options_snapshot = OptionSnapshot {
    57	        dry_run: options.dry_run,
    58	        preserve_symlinks: options.preserve_symlinks,
    59	        include_symlinks: options.include_symlinks,
    60	        skip_unchanged: options.skip_unchanged,
    61	        checksum: options.checksum,
    62	        compare_mode: snapshot_compare_mode(options),
    63	        workers: options.workers,
    64	    };
    65	
    66	    let mode = if options.mirror {
    67	        TransferMode::Mirror
    68	    } else {
    69	        TransferMode::Copy
    70	    };
    71	
    72	    // R44-F1: train against scanned features so the predictor's
    73	    // training inputs match its query inputs. The orchestrator
    74	    // queries `predict(...)` with `all_headers.len()` (scanned
    75	    // count) and `total_bytes` (scanned bytes); pre-fix the record
    76	    // was populated with `summary.copied_files`, so the predictor
    77	    // saw a different feature vector at training time than at
    78	    // query time, and predictions drifted on every incremental
    79	    // workload. The `total_bytes` field on the record was already
    80	    // scanned-bytes by accident; this aligns both axes deliberately.
    81	    //
    82	    // `summary.copied_files` and the per-bucket counts
    83	    // (tar_shard_files / raw_bundle_files / large_tasks) still
    84	    // reflect actual writes — they're the load-bearing inputs for
    85	    // `derive_local_plan_tuning`'s bucket-target heuristics, which
    86	    // are computed from observed apply behavior, not scan size.
    87	    let mut record = PerformanceRecord::new(
    88	        mode,
    89	        None,
    90	        None,
    91	        summary.scanned_files,
    92	        summary.scanned_bytes,
    93	        options_snapshot,
    94	        fast_path.map(|s| s.to_string()),
    95	        planner_duration_ms,
    96	        transfer_duration_ms,
    97	        0,
    98	        0,
    99	    );
   100	    record.tar_shard_tasks = summary.tar_shard_tasks as u32;
   101	    record.tar_shard_files = summary.tar_shard_files as u32;
   102	    record.tar_shard_bytes = summary.tar_shard_bytes;
   103	    record.raw_bundle_tasks = summary.raw_bundle_tasks as u32;
   104	    record.raw_bundle_files = summary.raw_bundle_files as u32;
   105	    record.raw_bundle_bytes = summary.raw_bundle_bytes;
   106	    record.large_tasks = summary.large_tasks as u32;
   107	    record.large_bytes = summary.large_bytes;
   108	
   109	    record
   110	}
   111	
   112	pub(super) fn update_predictor(
   113	    predictor: &mut Option<PerformancePredictor>,
   114	    record: &PerformanceRecord,
   115	    verbose: bool,
   116	) {
   117	    if let Some(ref mut predictor) = predictor {
   118	        predictor.observe(record);
   119	        if let Err(err) = predictor.save() {
   120	            if verbose {
   121	                eprintln!("Failed to persist predictor state: {err:?}");
   122	            }
   123	        }
   124	    }
   125	}
   126	
   127	#[cfg(test)]
   128	mod tests {
   129	    use super::super::summary::TransferOutcome;
   130	    use super::*;
   131	    use std::time::Duration;
   132	
   133	    fn options_with_mirror(mirror: bool) -> LocalMirrorOptions {
   134	        LocalMirrorOptions {
   135	            mirror,
   136	            ..LocalMirrorOptions::default()
   137	        }
   138	    }
   139	
   140	    /// R44-F1 contract: the record's `(file_count, total_bytes)`
   141	    /// must mirror the orchestrator's predictor-query features.
   142	    /// Pre-fix this assertion would have failed: the record was
   143	    /// populated from `summary.copied_files` and `summary.total_bytes`
   144	    /// while the query used scanned values, so on this incremental
   145	    /// scenario (1000 scanned, 5 actually written) the predictor
   146	    /// trained on (5, 100KB) but was queried with
   147	    /// (1000, ~10MB).
   148	    #[test]
   149	    fn record_uses_scanned_features_not_copied() {
   150	        let summary = LocalMirrorSummary {
   151	            // Mostly-unchanged incremental run: 1000 files scanned,
   152	            // only 5 actually written.
   153	            scanned_files: 1000,
   154	            scanned_bytes: 10 * 1024 * 1024,
   155	            planned_files: 5,
   156	            copied_files: 5,
   157	            total_bytes: 100 * 1024,
   158	            duration: Duration::from_millis(200),
   159	            outcome: TransferOutcome::Transferred,
   160	            ..LocalMirrorSummary::default()
   161	        };
   162	        let options = options_with_mirror(false);
   163	        let record = build_performance_record(&summary, &options, Some("streaming"), 150, 50);
   164	
   165	        assert_eq!(
   166	            record.file_count, 1000,
   167	            "record.file_count must reflect scanned (planner-side) workload, not copied count"
   168	        );
   169	        assert_eq!(
   170	            record.total_bytes, summary.scanned_bytes,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mod.rs | sed -n '180,370p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   180	                            .map(|probe| matches!(probe.state, ChangeState::NoChanges))
   181	                            .unwrap_or(false);
   182	
   183	                        if src_no_change && dest_no_change {
   184	                            journal_skip = true;
   185	                        }
   186	                    }
   187	                    Err(err) => {
   188	                        if options.verbose {
   189	                            eprintln!("Filesystem journal probe failed: {err:?}");
   190	                        }
   191	                    }
   192	                }
   193	            }
   194	        }
   195	
   196	        if journal_skip {
   197	            if options.verbose {
   198	                eprintln!(
   199	                    "Filesystem journal fast-path: source/destination unchanged; skipping planner."
   200	                );
   201	            }
   202	            if let Some(tracker) = journal_tracker.as_mut() {
   203	                persist_journal_checkpoints(
   204	                    tracker,
   205	                    journal_tokens.as_mut_slice(),
   206	                    options.verbose,
   207	                );
   208	            }
   209	
   210	            // Journal said both sides match, so we never enumerated.
   211	            // scanned_{files,bytes} stay 0 — predictor sees this as
   212	            // "noop with no scan cost" which is what actually happened.
   213	            let summary = LocalMirrorSummary {
   214	                dry_run: options.dry_run,
   215	                duration: start_time.elapsed(),
   216	                outcome: TransferOutcome::JournalSkip,
   217	                ..Default::default()
   218	            };
   219	
   220	            if let Some(record) = record_performance_history(
   221	                &summary,
   222	                &options,
   223	                Some("journal_no_work"),
   224	                0,
   225	                summary.duration.as_millis(),
   226	            ) {
   227	                update_predictor(&mut predictor, &record, options.verbose);
   228	            }
   229	
   230	            return Ok(summary);
   231	        }
   232	
   233	        // Skip fast path when using null sink — it bypasses the sink abstraction.
   234	        let fast_path_outcome = if options.null_sink {
   235	            self::strategy::FastPathOutcome::streaming()
   236	        } else {
   237	            maybe_select_fast_path(src_root, dest_root, &options)?
   238	        };
   239	        if let Some(decision) = fast_path_outcome.decision {
   240	            // R47-F4: propagate the fast-path scan's suppressed
   241	            // errors into the per-branch summary. Each fast-path
   242	            // outcome below clones this into `unreadable_paths`
   243	            // so the CLI's source-delete step can detect a
   244	            // partial scan even on the Tiny/Huge/NoWork paths.
   245	            let fast_path_unreadable = fast_path_outcome.unreadable_paths.clone();
   246	            let summary = match decision {
   247	                FastPathDecision::NoWork { examined } => {
   248	                    let outcome = if examined == 0 {
   249	                        TransferOutcome::SourceEmpty
   250	                    } else {
   251	                        TransferOutcome::UpToDate
   252	                    };
   253	                    if options.verbose {
   254	                        match outcome {
   255	                            TransferOutcome::SourceEmpty => {
   256	                                eprintln!("Fast-path routing: source yielded no file entries")
   257	                            }
   258	                            _ => eprintln!(
   259	                                "Fast-path routing: {} files examined, all up to date",
   260	                                examined
   261	                            ),
   262	                        }
   263	                    }
   264	                    // NoWork ran a real fast-path scan but copied nothing.
   265	                    // scanned_files = examined captures the planner-side
   266	                    // workload; scanned_bytes is 0 because the fast-path
   267	                    // scanner only resolves names + identity, not sizes.
   268	                    let summary = LocalMirrorSummary {
   269	                        planned_files: examined,
   270	                        scanned_files: examined,
   271	                        dry_run: options.dry_run,
   272	                        duration: start_time.elapsed(),
   273	                        outcome,
   274	                        unreadable_paths: fast_path_unreadable.clone(),
   275	                        ..Default::default()
   276	                    };
   277	                    if let Some(record) = record_performance_history(
   278	                        &summary,
   279	                        &options,
   280	                        Some("no_work"),
   281	                        0,
   282	                        summary.duration.as_millis(),
   283	                    ) {
   284	                        update_predictor(&mut predictor, &record, options.verbose);
   285	                    }
   286	                    summary
   287	                }
   288	                FastPathDecision::Tiny { files } => {
   289	                    let total_bytes: u64 = files.iter().map(|(_, size)| *size).sum();
   290	                    if options.verbose {
   291	                        eprintln!(
   292	                            "Fast-path routing: tiny manifest ({} file(s), {} bytes)",
   293	                            files.len(),
   294	                            total_bytes
   295	                        );
   296	                    }
   297	                    let rels: Vec<PathBuf> = files.iter().map(|(rel, _)| rel.clone()).collect();
   298	                    copy_paths_blocking(src_root, dest_root, &rels, &copy_config)?;
   299	                    // Tiny copies everything it scanned, so scanned ==
   300	                    // copied here. Setting both lets the predictor
   301	                    // train on the actual workload size for the
   302	                    // tiny_manifest fast-path key.
   303	                    let summary = LocalMirrorSummary {
   304	                        planned_files: files.len(),
   305	                        copied_files: files.len(),
   306	                        total_bytes,
   307	                        scanned_files: files.len(),
   308	                        scanned_bytes: total_bytes,
   309	                        dry_run: options.dry_run,
   310	                        duration: start_time.elapsed(),
   311	                        unreadable_paths: fast_path_unreadable.clone(),
   312	                        ..Default::default()
   313	                    };
   314	                    if let Some(record) = record_performance_history(
   315	                        &summary,
   316	                        &options,
   317	                        Some("tiny_manifest"),
   318	                        0,
   319	                        summary.duration.as_millis(),
   320	                    ) {
   321	                        update_predictor(&mut predictor, &record, options.verbose);
   322	                    }
   323	                    summary
   324	                }
   325	                FastPathDecision::Huge { file, size } => {
   326	                    if options.verbose {
   327	                        eprintln!(
   328	                            "Fast-path routing: huge file {} ({} bytes)",
   329	                            file.display(),
   330	                            size
   331	                        );
   332	                    }
   333	                    copy_large_blocking(src_root, dest_root, &file, &copy_config)?;
   334	                    // Huge fast-path copies a single file: scan size
   335	                    // and copy size are identical (one file, `size`
   336	                    // bytes).
   337	                    let summary = LocalMirrorSummary {
   338	                        planned_files: 1,
   339	                        copied_files: 1,
   340	                        total_bytes: size,
   341	                        scanned_files: 1,
   342	                        scanned_bytes: size,
   343	                        dry_run: options.dry_run,
   344	                        duration: start_time.elapsed(),
   345	                        large_tasks: 1,
   346	                        large_bytes: size,
   347	                        unreadable_paths: fast_path_unreadable.clone(),
   348	                        ..Default::default()
   349	                    };
   350	                    if let Some(record) = record_performance_history(
   351	                        &summary,
   352	                        &options,
   353	                        Some("single_huge_file"),
   354	                        0,
   355	                        summary.duration.as_millis(),
   356	                    ) {
   357	                        update_predictor(&mut predictor, &record, options.verbose);
   358	                    }
   359	                    summary
   360	                }
   361	            };
   362	
   363	            if let Some(tracker) = journal_tracker.as_mut() {
   364	                persist_journal_checkpoints(
   365	                    tracker,
   366	                    journal_tokens.as_mut_slice(),
   367	                    options.verbose,
   368	                );
   369	            }
   370	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mod.rs | sed -n '760,810p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   760	            // can audit the predictor against this run's actual
   761	            // numbers. The bare percentage error per phase is the
   762	            // most useful single number; we keep absolute ms in the
   763	            // line above for context.
   764	            if let Some(est) = summary.predictor_estimate.as_ref() {
   765	                let pct = |predicted_ms: u128, actual_ms: u128| -> String {
   766	                    if actual_ms == 0 {
   767	                        "n/a".to_string()
   768	                    } else {
   769	                        let pred = predicted_ms as f64;
   770	                        let act = actual_ms as f64;
   771	                        format!("{:+.0}%", ((pred - act) / act) * 100.0)
   772	                    }
   773	                };
   774	                eprintln!(
   775	                    "Predictor delta: planner {} ({} vs {} ms), \
   776	                     transfer {} ({} vs {} ms)",
   777	                    pct(est.planner_ms, planner_duration_ms),
   778	                    est.planner_ms,
   779	                    planner_duration_ms,
   780	                    pct(est.transfer_ms, transfer_duration_ms),
   781	                    est.transfer_ms,
   782	                    transfer_duration_ms,
   783	                );
   784	            }
   785	        }
   786	
   787	        let fast_path_label = if options.null_sink {
   788	            Some("null_sink")
   789	        } else {
   790	            None
   791	        };
   792	        if let Some(record) = record_performance_history(
   793	            &summary,
   794	            &options,
   795	            fast_path_label,
   796	            planner_duration_ms,
   797	            transfer_duration_ms,
   798	        ) {
   799	            // Don't update the predictor from null-sink runs — the zero
   800	            // write cost would teach it that transfers are faster than
   801	            // they really are.
   802	            if !options.null_sink {
   803	                update_predictor(&mut predictor, &record, options.verbose);
   804	            }
   805	        }
   806	
   807	        Ok(summary)
   808	    }
   809	}
   810	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/single_file.rs | sed -n '1,80p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Single-file copy strategy. Moved from
     2	//! `orchestrator/orchestrator.rs` at ue-r2-1c; the same slice adds
     3	//! the perf-history/predictor accounting this path lacked
     4	//! (REV4 Design §2: engine strategies share common accounting).
     5	
     6	use std::path::{Path, PathBuf};
     7	use std::time::Instant;
     8	
     9	use eyre::{Context, Result};
    10	
    11	use crate::generated::ComparisonMode;
    12	use crate::perf_predictor::PerformancePredictor;
    13	
    14	use super::history::{record_performance_history, update_predictor};
    15	use super::options::LocalMirrorOptions;
    16	use super::summary::{LocalMirrorSummary, TransferOutcome};
    17	
    18	/// Copy a single file source directly to `dest_root` (the CLI's
    19	/// destination resolver has already produced the exact target path),
    20	/// then account for the run. ue-r2-1c: before the engine existed this
    21	/// shortcut bypassed perf-history/predictor recording entirely — the
    22	/// only strategy that did. It now records like every other strategy:
    23	/// tag `single_file` (or `null_sink`, matching the streaming path's
    24	/// lane convention so RunKind::NullSink derivation keeps working), no
    25	/// predictor update on null-sink runs (zero write cost would teach the
    26	/// predictor that transfers are faster than they really are). Records
    27	/// carry `tar_shard_tasks == raw_bundle_tasks == 0`, so the tuning
    28	/// window's signal filter already excludes them from auto-tuning.
    29	pub(super) fn execute_single_file_copy(
    30	    src_root: &Path,
    31	    dest_root: &Path,
    32	    options: &LocalMirrorOptions,
    33	    start_time: Instant,
    34	) -> Result<LocalMirrorSummary> {
    35	    let summary = single_file_copy_inner(src_root, dest_root, options, start_time)?;
    36	
    37	    let fast_path_label = if options.null_sink {
    38	        "null_sink"
    39	    } else {
    40	        "single_file"
    41	    };
    42	    if let Some(record) = record_performance_history(
    43	        &summary,
    44	        options,
    45	        Some(fast_path_label),
    46	        0,
    47	        summary.duration.as_millis(),
    48	    ) {
    49	        if !options.null_sink {
    50	            let mut predictor = PerformancePredictor::load().ok();
    51	            update_predictor(&mut predictor, &record, options.verbose);
    52	        }
    53	    }
    54	
    55	    Ok(summary)
    56	}
    57	
    58	/// The copy itself, bypassing the enumerator/planner/pipeline
    59	/// machinery which assumes `src_root` is a directory.
    60	fn single_file_copy_inner(
    61	    src_root: &Path,
    62	    dest_root: &Path,
    63	    options: &LocalMirrorOptions,
    64	    start_time: Instant,
    65	) -> Result<LocalMirrorSummary> {
    66	    use crate::buffer::BufferSizer;
    67	    use crate::copy::{copy_file, file_needs_copy_with_mode, resume_copy_file};
    68	    use crate::logger::NoopLogger;
    69	    use filetime::FileTime;
    70	
    71	    let src_meta = std::fs::metadata(src_root)
    72	        .with_context(|| format!("stat source file {}", src_root.display()))?;
    73	    let size = src_meta.len();
    74	
    75	    // R58-followup: route compare-mode for the single-file path
    76	    // through the same translation the directory path uses
    77	    // (orchestrator.rs:481). Pre-fix the short-circuit only looked
    78	    // at `options.checksum`, so `--size-only` / `--ignore-times` /
    79	    // `--force` were silently dropped — repro: copy src.txt dst.txt
    80	    // --size-only re-copied even when sizes matched.

codex
The cited `record_performance_history` line numbers in the draft match the current tree. The remaining question is more subtle: whether all those calls are truly post-success relative to top-level failures, including the single-file path and retry wrapper behavior.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_history.rs | sed -n '1,380p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Local performance history writer for adaptive planning.
     2	//!
     3	//! Records summarized run information to a capped JSONL file under the user's
     4	//! config directory. The data stays on-device and can be toggled via the CLI
     5	//! (`blit diagnostics perf --enable/--disable`).
     6	
     7	use crate::config;
     8	use std::collections::VecDeque;
     9	use std::fs::{self, File, OpenOptions};
    10	use std::io::{BufRead, BufReader, Write};
    11	use std::path::{Path, PathBuf};
    12	use std::time::{SystemTime, UNIX_EPOCH};
    13	
    14	use eyre::{Context, Result};
    15	use serde::{Deserialize, Serialize};
    16	
    17	const DEFAULT_MAX_BYTES: u64 = 1_000_000; // ~1 MiB cap per design docs
    18	const SETTINGS_FILE: &str = "settings.json";
    19	
    20	/// Current schema version for PerformanceRecord.
    21	///
    22	/// Bump this when making changes to the record format. Old records without a
    23	/// version field deserialize as version 0 thanks to `#[serde(default)]`.
    24	///
    25	/// Version history:
    26	///   0 - implicit (records written before versioning was added)
    27	///   1 - added schema_version field
    28	///   2 - added `run_kind` to separate measurement lanes (real transfer
    29	///       vs dry-run vs null-sink vs bench). Pre-v2 records carry their
    30	///       lane implicitly in `options.dry_run` and
    31	///       `fast_path == Some("null_sink")`; migration derives `run_kind`
    32	///       from those without touching `mode`. R56-F1.
    33	pub const CURRENT_SCHEMA_VERSION: u32 = 2;
    34	
    35	/// High-level category of a transfer run (intent-side).
    36	///
    37	/// `mode` answers "what was the operator asking for?" — copy or mirror.
    38	/// Orthogonal to `RunKind`, which answers "what kind of measurement is
    39	/// this record?" — a real transfer, a dry-run, a null-sink benchmark,
    40	/// etc. A `(mode=Mirror, run_kind=DryRun)` record means the user asked
    41	/// for a mirror operation but routed it through the dry-run path; that
    42	/// record should NOT teach the predictor anything about real-mirror
    43	/// transfer cost (no writes happened).
    44	#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
    45	#[serde(rename_all = "snake_case")]
    46	pub enum TransferMode {
    47	    Copy,
    48	    Mirror,
    49	}
    50	
    51	/// Measurement lane for a [`PerformanceRecord`]. Determines whether the
    52	/// record is eligible to feed real-transfer predictor training or local
    53	/// auto-tune aggregates. R56-F1: pre-fix, `derive_local_plan_tuning`
    54	/// read every record indiscriminately, so dry-run records (zero writes)
    55	/// and null-sink benchmarks (zero writes by definition) taught the
    56	/// tuner that destination writes were free. Filtering by `run_kind ==
    57	/// Real` is the single chokepoint that closes that contamination.
    58	#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
    59	#[serde(rename_all = "snake_case")]
    60	pub enum RunKind {
    61	    /// Normal production transfer. Eligible for predictor training and
    62	    /// auto-tune aggregates.
    63	    #[default]
    64	    Real,
    65	    /// `--dry-run`: plan-and-stop, no writes happened. Useful for
    66	    /// debugging but not representative of real transfer cost.
    67	    DryRun,
    68	    /// `--null` / null-sink benchmark: pipeline ran, destination
    69	    /// writes discarded. Useful for diagnostics but writes were zero
    70	    /// cost.
    71	    NullSink,
    72	    /// `blit bench transfer` (planned 0.2.0 verb): real source reads,
    73	    /// null destination. Separate predictor lane.
    74	    BenchTransfer,
    75	    /// `blit bench wire` (planned 0.2.0 verb): synthetic source,
    76	    /// null destination. Pure data-plane measurement.
    77	    BenchWire,
    78	}
    79	
    80	impl RunKind {
    81	    /// True iff the record is a "real transfer" — eligible to feed
    82	    /// the predictor's real-transfer profile and the local auto-tune
    83	    /// bucket aggregates. R56-F1: every consumer of historical
    84	    /// records that drives production behavior MUST filter on this
    85	    /// before consulting per-record fields.
    86	    pub fn is_real_transfer(&self) -> bool {
    87	        matches!(self, RunKind::Real)
    88	    }
    89	}
    90	
    91	/// Comparison policy snapshot for performance history. Distinct
    92	/// from `generated::ComparisonMode` (proto enum) because the perf
    93	/// history file is JSONL and shouldn't depend on the generated
    94	/// proto serialization surface. R59 finding #5: pre-fix the
    95	/// tuning window keyed on `checksum: bool` alone, mixing
    96	/// SizeMtime / SizeOnly / Force / IgnoreTimes records into the
    97	/// same bucket.
    98	#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
    99	#[serde(rename_all = "snake_case")]
   100	pub enum CompareModeSnapshot {
   101	    #[default]
   102	    SizeMtime,
   103	    Checksum,
   104	    SizeOnly,
   105	    Force,
   106	    IgnoreTimes,
   107	}
   108	
   109	/// Snapshot of the options that influence performance.
   110	#[derive(Debug, Clone, Serialize, Deserialize)]
   111	pub struct OptionSnapshot {
   112	    pub dry_run: bool,
   113	    pub preserve_symlinks: bool,
   114	    pub include_symlinks: bool,
   115	    pub skip_unchanged: bool,
   116	    /// Legacy boolean — kept for back-compat with pre-R59
   117	    /// history records. New records also set `compare_mode` to
   118	    /// preserve the user's intent across the four non-default
   119	    /// comparison policies. Tuning window selection should key
   120	    /// on `compare_mode`; this bool stays as the legacy fallback.
   121	    pub checksum: bool,
   122	    /// R59 finding #5: full comparison policy. `serde(default)`
   123	    /// so old records (which lack this field) deserialize as
   124	    /// `SizeMtime`, which is the historical default behavior.
   125	    #[serde(default)]
   126	    pub compare_mode: CompareModeSnapshot,
   127	    pub workers: usize,
   128	}
   129	
   130	/// Telemetry-free performance record captured after each run.
   131	///
   132	/// The `schema_version` field tracks the format version for migration support.
   133	/// See [`CURRENT_SCHEMA_VERSION`] for the version history.
   134	#[derive(Debug, Clone, Serialize, Deserialize)]
   135	pub struct PerformanceRecord {
   136	    #[serde(default)]
   137	    pub schema_version: u32,
   138	    pub timestamp_epoch_ms: u128,
   139	    pub mode: TransferMode,
   140	    /// R56-F1: measurement lane. Pre-v2 records omit this; the
   141	    /// migration derives it from `options.dry_run` and
   142	    /// `fast_path == Some("null_sink")`. Filtering on
   143	    /// `run_kind.is_real_transfer()` is the single chokepoint
   144	    /// that keeps dry-run / null-sink / bench records out of
   145	    /// production training data.
   146	    #[serde(default)]
   147	    pub run_kind: RunKind,
   148	    pub source_fs: Option<String>,
   149	    pub dest_fs: Option<String>,
   150	    pub file_count: usize,
   151	    pub total_bytes: u64,
   152	    pub options: OptionSnapshot,
   153	    pub fast_path: Option<String>,
   154	    pub planner_duration_ms: u128,
   155	    pub transfer_duration_ms: u128,
   156	    pub stall_events: u32,
   157	    pub error_count: u32,
   158	    #[serde(default)]
   159	    pub tar_shard_tasks: u32,
   160	    #[serde(default)]
   161	    pub tar_shard_files: u32,
   162	    #[serde(default)]
   163	    pub tar_shard_bytes: u64,
   164	    #[serde(default)]
   165	    pub raw_bundle_tasks: u32,
   166	    #[serde(default)]
   167	    pub raw_bundle_files: u32,
   168	    #[serde(default)]
   169	    pub raw_bundle_bytes: u64,
   170	    #[serde(default)]
   171	    pub large_tasks: u32,
   172	    #[serde(default)]
   173	    pub large_bytes: u64,
   174	}
   175	
   176	impl PerformanceRecord {
   177	    #[allow(clippy::too_many_arguments)]
   178	    pub fn new(
   179	        mode: TransferMode,
   180	        source_fs: Option<String>,
   181	        dest_fs: Option<String>,
   182	        file_count: usize,
   183	        total_bytes: u64,
   184	        options: OptionSnapshot,
   185	        fast_path: Option<String>,
   186	        planner_duration_ms: u128,
   187	        transfer_duration_ms: u128,
   188	        stall_events: u32,
   189	        error_count: u32,
   190	    ) -> Self {
   191	        // R56-F1: derive `run_kind` from the call-site inputs. The
   192	        // callers that need a specific kind (bench verbs, future
   193	        // synthetic source) should mutate `record.run_kind` after
   194	        // construction; this default infers from existing fields so
   195	        // we don't have to thread a new parameter through every
   196	        // caller right now.
   197	        let run_kind = if options.dry_run {
   198	            RunKind::DryRun
   199	        } else if fast_path.as_deref() == Some("null_sink") {
   200	            RunKind::NullSink
   201	        } else {
   202	            RunKind::Real
   203	        };
   204	        let now = SystemTime::now()
   205	            .duration_since(UNIX_EPOCH)
   206	            .unwrap_or_default();
   207	        Self {
   208	            schema_version: CURRENT_SCHEMA_VERSION,
   209	            timestamp_epoch_ms: now.as_millis(),
   210	            mode,
   211	            run_kind,
   212	            source_fs,
   213	            dest_fs,
   214	            file_count,
   215	            total_bytes,
   216	            options,
   217	            fast_path,
   218	            planner_duration_ms,
   219	            transfer_duration_ms,
   220	            stall_events,
   221	            error_count,
   222	            tar_shard_tasks: 0,
   223	            tar_shard_files: 0,
   224	            tar_shard_bytes: 0,
   225	            raw_bundle_tasks: 0,
   226	            raw_bundle_files: 0,
   227	            raw_bundle_bytes: 0,
   228	            large_tasks: 0,
   229	            large_bytes: 0,
   230	        }
   231	    }
   232	}
   233	
   234	/// Append a record to the local performance history store.
   235	///
   236	/// Errors are bubbled up so callers can decide whether to log or ignore them.
   237	/// The function honours the persisted enable/disable flag; callers do not need
   238	/// to perform a separate check.
   239	pub fn append_local_record(record: &PerformanceRecord) -> Result<()> {
   240	    if !perf_history_enabled()? {
   241	        return Ok(());
   242	    }
   243	
   244	    let path = history_path()?;
   245	    if let Some(parent) = path.parent() {
   246	        fs::create_dir_all(parent).with_context(|| {
   247	            format!(
   248	                "failed to create performance history directory {}",
   249	                parent.display()
   250	            )
   251	        })?;
   252	    }
   253	
   254	    let mut file = OpenOptions::new()
   255	        .create(true)
   256	        .append(true)
   257	        .open(&path)
   258	        .with_context(|| format!("failed to open performance history file {}", path.display()))?;
   259	
   260	    let line = serde_json::to_string(record).context("serialize performance record")?;
   261	    writeln!(file, "{line}").context("write performance record")?;
   262	    drop(file);
   263	
   264	    enforce_size_cap(&path, DEFAULT_MAX_BYTES)?;
   265	    Ok(())
   266	}
   267	
   268	/// Migrate a record from an older schema version to the current version.
   269	///
   270	/// Returns the record with `schema_version` set to `CURRENT_SCHEMA_VERSION`.
   271	/// Future migrations (e.g., field renames, type changes) should be added here
   272	/// as version-gated transformations.
   273	pub fn migrate_record(mut record: PerformanceRecord) -> PerformanceRecord {
   274	    // v0 → v1: no field changes; v1 just stamped the version field.
   275	    //
   276	    // v1 → v2: introduced `run_kind`. Older records didn't carry it
   277	    // explicitly; the lane was implicit in `options.dry_run` and
   278	    // `fast_path == Some("null_sink")`. R56-F1: derive the kind
   279	    // without touching `mode` (which already correctly captures
   280	    // copy vs mirror — old mirror records stay mirror, not
   281	    // collapsed to Copy).
   282	    //
   283	    // We re-derive on every load below v2 — serde's #[serde(default)]
   284	    // on the field gives us RunKind::Real for a missing-field
   285	    // deserialize, which is the WRONG default for a dry-run record
   286	    // whose run_kind we never wrote. The explicit migration here
   287	    // is what makes loaded-from-v1 dry-run records actually carry
   288	    // the DryRun lane.
   289	    if record.schema_version < 2 {
   290	        record.run_kind = if record.options.dry_run {
   291	            RunKind::DryRun
   292	        } else if record.fast_path.as_deref() == Some("null_sink") {
   293	            RunKind::NullSink
   294	        } else {
   295	            RunKind::Real
   296	        };
   297	    }
   298	    record.schema_version = CURRENT_SCHEMA_VERSION;
   299	    record
   300	}
   301	
   302	pub fn read_recent_records(limit: usize) -> Result<Vec<PerformanceRecord>> {
   303	    let path = history_path()?;
   304	    read_records_from_path(&path, limit)
   305	}
   306	
   307	fn read_records_from_path(path: &Path, limit: usize) -> Result<Vec<PerformanceRecord>> {
   308	    if !path.exists() {
   309	        return Ok(Vec::new());
   310	    }
   311	
   312	    let file = File::open(path)?;
   313	    let reader = BufReader::new(file);
   314	    let mut records = Vec::new();
   315	
   316	    for line in reader.lines() {
   317	        let Ok(line) = line else { continue };
   318	        if line.trim().is_empty() {
   319	            continue;
   320	        }
   321	        if let Ok(record) = serde_json::from_str::<PerformanceRecord>(&line) {
   322	            records.push(migrate_record(record));
   323	        }
   324	    }
   325	
   326	    if limit == 0 || records.len() <= limit {
   327	        return Ok(records);
   328	    }
   329	
   330	    let start = records.len().saturating_sub(limit);
   331	    Ok(records[start..].to_vec())
   332	}
   333	
   334	/// Rewrite the history file, migrating all records to the current schema version.
   335	///
   336	/// This is safe to call at any time. Records that fail to parse are dropped.
   337	/// Returns the number of records migrated, or `Ok(0)` if the file doesn't exist.
   338	pub fn migrate_history_file() -> Result<usize> {
   339	    let path = history_path()?;
   340	    if !path.exists() {
   341	        return Ok(0);
   342	    }
   343	
   344	    let records = read_records_from_path(&path, 0)?;
   345	    let count = records.len();
   346	
   347	    let mut file = File::create(&path)
   348	        .with_context(|| format!("rewriting history file {}", path.display()))?;
   349	    for record in &records {
   350	        let line = serde_json::to_string(record).context("serialize migrated record")?;
   351	        writeln!(file, "{line}")?;
   352	    }
   353	
   354	    Ok(count)
   355	}
   356	
   357	pub fn config_dir() -> Result<PathBuf> {
   358	    config::config_dir()
   359	}
   360	
   361	fn history_path() -> Result<PathBuf> {
   362	    Ok(config_dir()?.join("perf_local.jsonl"))
   363	}
   364	
   365	#[derive(Debug, Clone, Serialize, Deserialize)]
   366	struct Settings {
   367	    #[serde(default = "default_perf_history_enabled")]
   368	    perf_history_enabled: bool,
   369	}
   370	
   371	impl Default for Settings {
   372	    fn default() -> Self {
   373	        Self {
   374	            perf_history_enabled: true,
   375	        }
   376	    }
   377	}
   378	
   379	fn default_perf_history_enabled() -> bool {
   380	    true

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/auto_tune/mod.rs | sed -n '80,180p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    80	}
    81	
    82	#[cfg(test)]
    83	mod local_tests {
    84	    use super::*;
    85	    use crate::perf_history::{OptionSnapshot, PerformanceRecord, TransferMode};
    86	
    87	    fn sample_record(
    88	        tar_tasks: u32,
    89	        tar_files: u32,
    90	        tar_bytes: u64,
    91	        raw_tasks: u32,
    92	        raw_bytes: u64,
    93	    ) -> PerformanceRecord {
    94	        let mut record = PerformanceRecord::new(
    95	            TransferMode::Copy,
    96	            None,
    97	            None,
    98	            10,
    99	            1024,
   100	            OptionSnapshot {
   101	                dry_run: false,
   102	                preserve_symlinks: true,
   103	                include_symlinks: true,
   104	                skip_unchanged: true,
   105	                checksum: false,
   106	                compare_mode: crate::perf_history::CompareModeSnapshot::SizeMtime,
   107	                workers: 4,
   108	            },
   109	            None,
   110	            100,
   111	            1_000,
   112	            0,
   113	            0,
   114	        );
   115	        record.tar_shard_tasks = tar_tasks;
   116	        record.tar_shard_files = tar_files;
   117	        record.tar_shard_bytes = tar_bytes;
   118	        record.raw_bundle_tasks = raw_tasks;
   119	        record.raw_bundle_bytes = raw_bytes;
   120	        record
   121	    }
   122	
   123	    #[test]
   124	    fn derive_tuning_from_history() {
   125	        let records = vec![
   126	            sample_record(4, 4000, 32 * 1024 * 1024, 2, 256 * 1024 * 1024),
   127	            sample_record(2, 1800, 20 * 1024 * 1024, 1, 128 * 1024 * 1024),
   128	        ];
   129	        let tuning = derive_local_plan_tuning(&records).expect("tuning");
   130	        assert!(tuning.small_target_bytes >= 4 * 1024 * 1024);
   131	        assert!(tuning.small_target_bytes <= 128 * 1024 * 1024);
   132	        assert!(tuning.small_count_target >= 128 && tuning.small_count_target <= 4096);
   133	        assert!(tuning.medium_target_bytes >= 64 * 1024 * 1024);
   134	    }
   135	
   136	    #[test]
   137	    fn returns_none_without_tasks() {
   138	        let records = vec![sample_record(0, 0, 0, 0, 0)];
   139	        assert!(derive_local_plan_tuning(&records).is_none());
   140	    }
   141	
   142	    // ── R56-F1: run_kind filtering ─────────────────────────────────────
   143	
   144	    /// Records carry `run_kind` lane. Only `Real` lane contributes to
   145	    /// the tuning aggregates. A pollutant DryRun / NullSink / Bench
   146	    /// record sitting in the history must NOT shift the bucket
   147	    /// targets — that was the latent bug `derive_local_plan_tuning`
   148	    /// shipped with before R56-F1.
   149	    fn record_in_lane(
   150	        kind: crate::perf_history::RunKind,
   151	        tar_tasks: u32,
   152	        tar_bytes: u64,
   153	        raw_tasks: u32,
   154	        raw_bytes: u64,
   155	    ) -> PerformanceRecord {
   156	        let mut r = sample_record(tar_tasks, tar_tasks * 100, tar_bytes, raw_tasks, raw_bytes);
   157	        r.run_kind = kind;
   158	        r
   159	    }
   160	
   161	    #[test]
   162	    fn tuning_ignores_dry_run_records() {
   163	        use crate::perf_history::RunKind;
   164	        // One real record + ten dry-run records with absurd byte
   165	        // counts. The dry-runs must NOT pull the small_target_bytes
   166	        // toward their bogus values.
   167	        let mut records = vec![record_in_lane(
   168	            RunKind::Real,
   169	            4,
   170	            32 * 1024 * 1024,
   171	            2,
   172	            256 * 1024 * 1024,
   173	        )];
   174	        for _ in 0..10 {
   175	            // Massive byte counts in a dry-run — if these leaked
   176	            // into the average we'd see them in the output.
   177	            records.push(record_in_lane(
   178	                RunKind::DryRun,
   179	                4,
   180	                512 * 1024 * 1024,

exec
/usr/bin/zsh -lc "rg -n \"execute_single_file_copy|run_local_mirror|struct LocalMirrorOptions|pub async fn run|Result<LocalMirrorSummary>|\\?;|return Err|bail"'!|with_context" crates/blit-core/src/engine/mod.rs crates/blit-core/src/engine/single_file.rs crates/blit-app/src/transfers crates/blit-cli/src/transfers' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/engine/single_file.rs:29:pub(super) fn execute_single_file_copy(
crates/blit-core/src/engine/single_file.rs:34:) -> Result<LocalMirrorSummary> {
crates/blit-core/src/engine/single_file.rs:35:    let summary = single_file_copy_inner(src_root, dest_root, options, start_time)?;
crates/blit-core/src/engine/single_file.rs:65:) -> Result<LocalMirrorSummary> {
crates/blit-core/src/engine/single_file.rs:72:        .with_context(|| format!("stat source file {}", src_root.display()))?;
crates/blit-core/src/engine/single_file.rs:166:            .with_context(|| format!("resume copy {}", src_root.display()))?;
crates/blit-core/src/engine/single_file.rs:176:                .with_context(|| format!("copy {}", src_root.display()))?;
crates/blit-core/src/engine/mod.rs:65:use self::single_file::execute_single_file_copy;
crates/blit-core/src/engine/mod.rs:101:    pub async fn execute(&self, request: EngineRequest) -> Result<LocalMirrorSummary> {
crates/blit-core/src/engine/mod.rs:121:            return execute_single_file_copy(src_root, dest_root, &options, start_time);
crates/blit-core/src/engine/mod.rs:298:                    copy_paths_blocking(src_root, dest_root, &rels, &copy_config)?;
crates/blit-core/src/engine/mod.rs:333:                    copy_large_blocking(src_root, dest_root, &file, &copy_config)?;
crates/blit-core/src/engine/mod.rs:541:        let pipeline_outcome = pipeline_result.context("transfer pipeline failed")?;
crates/blit-core/src/engine/mod.rs:542:        let plan_outcome = plan_result?;
crates/blit-core/src/engine/mod.rs:545:            .context("scan failed")?;
crates/blit-core/src/engine/mod.rs:604:            )?;
crates/blit-core/src/engine/mod.rs:707:                bail!(
crates/blit-core/src/engine/mod.rs:736:            )?;
crates/blit-cli/src/transfers/remote.rs:161:pub async fn run_remote_push_transfer(
crates/blit-cli/src/transfers/remote.rs:175:pub async fn run_remote_push_transfer_deferred(
crates/blit-cli/src/transfers/remote.rs:217:    let filter = super::build_filter(args)?;
crates/blit-cli/src/transfers/remote.rs:259:    let outcome = outcome?;
crates/blit-cli/src/transfers/remote.rs:271:pub async fn run_remote_pull_transfer(
crates/blit-cli/src/transfers/remote.rs:297:pub async fn run_remote_pull_transfer_deferred(
crates/blit-cli/src/transfers/remote.rs:354:    let filter_spec = super::build_filter_spec(args)?;
crates/blit-cli/src/transfers/remote.rs:405:    let sync_outcome = run_pull_sync(execution, progress_handle.as_ref()).await?;
crates/blit-cli/src/transfers/remote.rs:412:    let mirror_purge_stats = apply_pull_mirror_purge(&sync_outcome, mirror_mode).await?;
crates/blit-cli/src/transfers/mod.rs:94:    io::stdout().flush()?;
crates/blit-cli/src/transfers/mod.rs:96:    io::stdin().read_line(&mut input)?;
crates/blit-cli/src/transfers/mod.rs:101:pub async fn run_transfer(ctx: &AppContext, args: &TransferArgs, mode: TransferKind) -> Result<()> {
crates/blit-cli/src/transfers/mod.rs:102:    let src_endpoint = parse_transfer_endpoint(&args.source)?;
crates/blit-cli/src/transfers/mod.rs:103:    let raw_dst = parse_transfer_endpoint(&args.destination)?;
crates/blit-cli/src/transfers/mod.rs:133:            bail!(
crates/blit-cli/src/transfers/mod.rs:146:            bail!(
crates/blit-cli/src/transfers/mod.rs:163:            (Endpoint::Local(_), _) | (_, Endpoint::Local(_)) => bail!(
crates/blit-cli/src/transfers/mod.rs:168:            (Endpoint::Remote(_), Endpoint::Remote(_)) if args.relay_via_cli => bail!(
crates/blit-cli/src/transfers/mod.rs:195:            bail!(
crates/blit-cli/src/transfers/mod.rs:237:                bail!("source path does not exist: {}", src.display());
crates/blit-cli/src/transfers/mod.rs:245:                bail!("source path does not exist: {}", src.display());
crates/blit-cli/src/transfers/mod.rs:247:            ensure_remote_push_supported(args)?;
crates/blit-cli/src/transfers/mod.rs:248:            ensure_remote_destination_supported(&dst)?;
crates/blit-cli/src/transfers/mod.rs:252:            ensure_remote_pull_supported(args)?;
crates/blit-cli/src/transfers/mod.rs:253:            ensure_remote_source_supported(&src)?;
crates/blit-cli/src/transfers/mod.rs:260:            ensure_remote_source_supported(&src)?;
crates/blit-cli/src/transfers/mod.rs:261:            ensure_remote_destination_supported(&dst)?;
crates/blit-cli/src/transfers/mod.rs:276:            ensure_remote_push_supported(args)?;
crates/blit-cli/src/transfers/mod.rs:280:            ensure_remote_source_supported(&src)?;
crates/blit-cli/src/transfers/mod.rs:281:            ensure_remote_destination_supported(&dst)?;
crates/blit-cli/src/transfers/mod.rs:282:            ensure_remote_pull_supported(args)?;
crates/blit-cli/src/transfers/mod.rs:288:pub async fn run_move(ctx: &AppContext, args: &TransferArgs) -> Result<()> {
crates/blit-cli/src/transfers/mod.rs:289:    let src_endpoint = parse_transfer_endpoint(&args.source)?;
crates/blit-cli/src/transfers/mod.rs:290:    let raw_dst = parse_transfer_endpoint(&args.destination)?;
crates/blit-cli/src/transfers/mod.rs:295:        bail!("move does not support --dry-run");
crates/blit-cli/src/transfers/mod.rs:306:        bail!(
crates/blit-cli/src/transfers/mod.rs:331:        bail!(
crates/blit-cli/src/transfers/mod.rs:350:        bail!(
crates/blit-cli/src/transfers/mod.rs:367:        bail!(
crates/blit-cli/src/transfers/mod.rs:400:        bail!(
crates/blit-cli/src/transfers/mod.rs:428:        bail!(
crates/blit-cli/src/transfers/mod.rs:476:                bail!("source path does not exist: {}", src_path.display());
crates/blit-cli/src/transfers/mod.rs:490:                local::run_local_transfer_deferred(ctx, args, &src_path, &dst_path, false).await?;
crates/blit-cli/src/transfers/mod.rs:506:                bail!(
crates/blit-cli/src/transfers/mod.rs:527:                    .with_context(|| format!("removing {}", src_path.display()))?;
crates/blit-cli/src/transfers/mod.rs:530:                    .with_context(|| format!("removing {}", src_path.display()))?;
crates/blit-cli/src/transfers/mod.rs:545:            )?;
crates/blit-cli/src/transfers/mod.rs:549:            ensure_remote_pull_supported(args)?;
crates/blit-cli/src/transfers/mod.rs:550:            ensure_remote_source_supported(&remote)?;
crates/blit-cli/src/transfers/mod.rs:564:            .await?;
crates/blit-cli/src/transfers/mod.rs:571:                _ => bail!("unsupported remote source for move"),
crates/blit-cli/src/transfers/mod.rs:573:            delete_remote_path(&remote, &rel_path).await?;
crates/blit-cli/src/transfers/mod.rs:579:                bail!("source path does not exist: {}", src_path.display());
crates/blit-cli/src/transfers/mod.rs:581:            ensure_remote_push_supported(args)?;
crates/blit-cli/src/transfers/mod.rs:582:            ensure_remote_destination_supported(&remote)?;
crates/blit-cli/src/transfers/mod.rs:595:            .await?;
crates/blit-cli/src/transfers/mod.rs:600:                    .with_context(|| format!("removing {}", src_path.display()))?;
crates/blit-cli/src/transfers/mod.rs:603:                    .with_context(|| format!("removing {}", src_path.display()))?;
crates/blit-cli/src/transfers/mod.rs:609:            ensure_remote_source_supported(&src)?;
crates/blit-cli/src/transfers/mod.rs:610:            ensure_remote_destination_supported(&dst)?;
crates/blit-cli/src/transfers/mod.rs:624:                bail!(
crates/blit-cli/src/transfers/mod.rs:635:            ensure_remote_pull_supported(args)?;
crates/blit-cli/src/transfers/mod.rs:649:            .await?;
crates/blit-cli/src/transfers/mod.rs:656:                _ => bail!("unsupported remote source for move"),
crates/blit-cli/src/transfers/mod.rs:658:            delete_remote_path(&src, &rel_path).await?;
crates/blit-cli/src/transfers/mod.rs:679:        let tmp = tempdir()?;
crates/blit-cli/src/transfers/mod.rs:682:        std::fs::create_dir_all(&src)?;
crates/blit-cli/src/transfers/mod.rs:683:        std::fs::write(src.join("hello.txt"), b"hello")?;
crates/blit-cli/src/transfers/mod.rs:720:        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
crates/blit-cli/src/transfers/mod.rs:721:        let copied = std::fs::read(dest.join("hello.txt"))?;
crates/blit-cli/src/transfers/mod.rs:728:        let tmp = tempdir()?;
crates/blit-cli/src/transfers/mod.rs:731:        std::fs::create_dir_all(&src)?;
crates/blit-cli/src/transfers/mod.rs:732:        std::fs::write(src.join("hello.txt"), b"hello")?;
crates/blit-cli/src/transfers/mod.rs:769:        runtime().block_on(run_local_transfer(&ctx, &args, &src, &dest, false))?;
crates/blit-app/src/transfers/remote.rs:107:                    let rel = path.strip_prefix(&root_path).ok()?;
crates/blit-app/src/transfers/remote.rs:114:                    let meta = std::fs::metadata(path).ok()?;
crates/blit-app/src/transfers/remote.rs:123:                    let checksum = hash_file(path, ChecksumType::Blake3).ok()?;
crates/blit-app/src/transfers/remote.rs:140:                    let rel = path.strip_prefix(&root_path).ok()?;
crates/blit-app/src/transfers/remote.rs:147:                    let meta = std::fs::metadata(path).ok()?;
crates/blit-app/src/transfers/remote.rs:220:    })?;
crates/blit-app/src/transfers/remote.rs:229:        })?;
crates/blit-app/src/transfers/remote.rs:233:            bail!("daemon delete list referenced the destination root itself");
crates/blit-app/src/transfers/remote.rs:250:                return Err(eyre!("failed to delete {}: {}", target.display(), e));
crates/blit-app/src/transfers/remote.rs:318:/// let sync = run_pull_sync(execution, handle.as_ref()).await?;
crates/blit-app/src/transfers/remote.rs:321:/// let purge = apply_pull_mirror_purge(&sync, mirror_mode).await?;
crates/blit-app/src/transfers/remote.rs:330:pub async fn run_pull_sync(
crates/blit-app/src/transfers/remote.rs:336:        .with_context(|| format!("connecting to {}", execution.remote.control_plane_uri()))?;
crates/blit-app/src/transfers/remote.rs:340:        enumerate_local_manifest(&actual_dest, execution.compute_checksums).await?;
crates/blit-app/src/transfers/remote.rs:351:        .with_context(|| {
crates/blit-app/src/transfers/remote.rs:357:        })?;
crates/blit-app/src/transfers/remote.rs:453:/// let outcome = run_remote_push(execution, handle.as_ref()).await?;
crates/blit-app/src/transfers/remote.rs:462:pub async fn run_remote_push(
crates/blit-app/src/transfers/remote.rs:468:        .with_context(|| format!("connecting to {}", execution.remote.control_plane_uri()))?;
crates/blit-app/src/transfers/remote.rs:475:                .with_context(|| {
crates/blit-app/src/transfers/remote.rs:477:                })?;
crates/blit-app/src/transfers/remote.rs:504:        .with_context(|| {
crates/blit-app/src/transfers/remote.rs:510:        })?;
crates/blit-app/src/transfers/remote.rs:636:        RemotePath::Discovery => bail!(
crates/blit-app/src/transfers/remote.rs:674:pub async fn run_delegated_pull<F>(
crates/blit-app/src/transfers/remote.rs:682:    let spec = RemotePullClient::build_spec_from_options(&execution.src, &execution.options)?;
crates/blit-app/src/transfers/remote.rs:683:    let (dst_module, dst_destination_path) = destination_spec_fields(&execution.dst)?;
crates/blit-app/src/transfers/remote.rs:700:        .with_context(|| format!("connecting to destination {}", execution.dst_label))?;
crates/blit-app/src/transfers/remote.rs:730:    })?;
crates/blit-app/src/transfers/remote.rs:788:        return Err(error);
crates/blit-app/src/transfers/remote.rs:791:    let summary = summary.ok_or_else(|| eyre!("delegation ended before summary"))?;
crates/blit-app/src/transfers/remote.rs:818:pub async fn run_delegated_pull_until_started(
crates/blit-app/src/transfers/remote.rs:822:        return Err(eyre!(
crates/blit-app/src/transfers/remote.rs:827:    let spec = RemotePullClient::build_spec_from_options(&execution.src, &execution.options)?;
crates/blit-app/src/transfers/remote.rs:828:    let (dst_module, dst_destination_path) = destination_spec_fields(&execution.dst)?;
crates/blit-app/src/transfers/remote.rs:845:        .with_context(|| format!("connecting to destination {}", execution.dst_label))?;
crates/blit-app/src/transfers/remote.rs:864:    })?;
crates/blit-app/src/transfers/remote.rs:886:                    return Err(eyre!(
crates/blit-app/src/transfers/retry.rs:29:pub async fn run_with_retries<F, Fut>(retries: u32, wait: Duration, mut attempt: F) -> Result<()>
crates/blit-app/src/transfers/retry.rs:40:                    return Err(err);
crates/blit-cli/src/transfers/remote_remote_direct.rs:21:pub async fn run_remote_to_remote_direct(
crates/blit-cli/src/transfers/remote_remote_direct.rs:50:pub async fn run_remote_to_remote_direct_deferred(
crates/blit-cli/src/transfers/remote_remote_direct.rs:92:    let filter_spec = super::build_filter_spec(args)?;
crates/blit-cli/src/transfers/remote_remote_direct.rs:168:        let (started, _dst) = run_delegated_pull_until_started(execution).await?;
crates/blit-cli/src/transfers/remote_remote_direct.rs:211:    let state = outcome?;
crates/blit-cli/src/transfers/local.rs:15:pub async fn run_local_transfer(
crates/blit-cli/src/transfers/local.rs:21:) -> Result<LocalMirrorSummary> {
crates/blit-cli/src/transfers/local.rs:30:pub async fn run_local_transfer_deferred(
crates/blit-cli/src/transfers/local.rs:36:) -> Result<LocalMirrorSummary> {
crates/blit-cli/src/transfers/local.rs:55:    let options = build_local_options(ctx, args, mirror)?;
crates/blit-cli/src/transfers/local.rs:80:) -> Result<LocalMirrorSummary> {
crates/blit-cli/src/transfers/local.rs:82:        bail!("source path does not exist: {}", src_path.display());
crates/blit-cli/src/transfers/local.rs:85:    let options = build_local_options(ctx, args, mirror)?;
crates/blit-cli/src/transfers/local.rs:118:    let summary = blit_app::transfers::local::run(src_path, dest_path, options).await?;
crates/blit-app/src/transfers/local.rs:36:pub async fn run(
crates/blit-app/src/transfers/local.rs:40:) -> Result<LocalMirrorSummary> {
crates/blit-app/src/transfers/local.rs:47:            .with_context(|| {
crates/blit-app/src/transfers/filter.rs:44:        filter.min_size = Some(parse_size(s).with_context(|| format!("--min-size {s}"))?);
crates/blit-app/src/transfers/filter.rs:47:        filter.max_size = Some(parse_size(s).with_context(|| format!("--max-size {s}"))?);
crates/blit-app/src/transfers/filter.rs:50:        filter.min_age = Some(parse_duration(s).with_context(|| format!("--min-age {s}"))?);
crates/blit-app/src/transfers/filter.rs:53:        filter.max_age = Some(parse_duration(s).with_context(|| format!("--max-age {s}"))?);
crates/blit-app/src/transfers/filter.rs:73:        .map_err(|msg| eyre!("invalid filter pattern: {msg}"))?;
crates/blit-app/src/transfers/filter.rs:94:        spec.min_size = Some(parse_size(s).with_context(|| format!("--min-size {s}"))?);
crates/blit-app/src/transfers/filter.rs:97:        spec.max_size = Some(parse_size(s).with_context(|| format!("--max-size {s}"))?);
crates/blit-app/src/transfers/filter.rs:102:                .with_context(|| format!("--min-age {s}"))?
crates/blit-app/src/transfers/filter.rs:109:                .with_context(|| format!("--max-age {s}"))?
crates/blit-app/src/transfers/filter.rs:114:        let entries = FileFilter::load_files_from(path)?;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/tuning.rs | sed -n '100,135p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_predictor.rs | sed -n '600,670p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   100	    //! window and force tuning to fall back to defaults even when
   101	    //! older real records existed.
   102	
   103	    use super::*;
   104	    use crate::auto_tune::derive_local_plan_tuning;
   105	    use crate::perf_history::{
   106	        CompareModeSnapshot, OptionSnapshot, PerformanceRecord, RunKind, TransferMode,
   107	    };
   108	    use eyre::eyre;
   109	
   110	    fn record(
   111	        kind: RunKind,
   112	        mode: TransferMode,
   113	        tar_tasks: u32,
   114	        tar_bytes: u64,
   115	        timestamp_ms: u128,
   116	    ) -> PerformanceRecord {
   117	        let mut r = PerformanceRecord::new(
   118	            mode,
   119	            None,
   120	            None,
   121	            10,
   122	            1024,
   123	            OptionSnapshot {
   124	                dry_run: false,
   125	                preserve_symlinks: true,
   126	                include_symlinks: false,
   127	                skip_unchanged: true,
   128	                checksum: false,
   129	                compare_mode: CompareModeSnapshot::SizeMtime,
   130	                workers: 4,
   131	            },
   132	            None,
   133	            10,
   134	            100,
   135	            0,

 succeeded in 0ms:
   600	
   601	#[cfg(test)]
   602	mod tests {
   603	    use super::*;
   604	    use crate::perf_history::{CompareModeSnapshot, OptionSnapshot};
   605	
   606	    fn make_record(
   607	        mode: TransferMode,
   608	        file_count: usize,
   609	        total_bytes: u64,
   610	        planner_ms: u128,
   611	    ) -> PerformanceRecord {
   612	        make_record_full(
   613	            mode,
   614	            file_count,
   615	            total_bytes,
   616	            planner_ms,
   617	            0,
   618	            None,
   619	            None,
   620	            None,
   621	        )
   622	    }
   623	
   624	    /// Full constructor for records with non-default fs class /
   625	    /// fast-path / transfer-duration. Used by the v2 fallback-chain
   626	    /// and dual-target tests that need control over those fields.
   627	    #[allow(clippy::too_many_arguments)]
   628	    fn make_record_full(
   629	        mode: TransferMode,
   630	        file_count: usize,
   631	        total_bytes: u64,
   632	        planner_ms: u128,
   633	        transfer_ms: u128,
   634	        source_fs: Option<&str>,
   635	        dest_fs: Option<&str>,
   636	        fast_path: Option<&str>,
   637	    ) -> PerformanceRecord {
   638	        PerformanceRecord {
   639	            schema_version: crate::perf_history::CURRENT_SCHEMA_VERSION,
   640	            timestamp_epoch_ms: 0,
   641	            mode,
   642	            run_kind: crate::perf_history::RunKind::Real,
   643	            source_fs: source_fs.map(str::to_string),
   644	            dest_fs: dest_fs.map(str::to_string),
   645	            file_count,
   646	            total_bytes,
   647	            options: OptionSnapshot {
   648	                dry_run: false,
   649	                preserve_symlinks: true,
   650	                include_symlinks: false,
   651	                skip_unchanged: true,
   652	                checksum: false,
   653	                compare_mode: CompareModeSnapshot::SizeMtime,
   654	                workers: 4,
   655	            },
   656	            fast_path: fast_path.map(str::to_string),
   657	            planner_duration_ms: planner_ms,
   658	            transfer_duration_ms: transfer_ms,
   659	            stall_events: 0,
   660	            error_count: 0,
   661	            tar_shard_tasks: 0,
   662	            tar_shard_files: 0,
   663	            tar_shard_bytes: 0,
   664	            raw_bundle_tasks: 0,
   665	            raw_bundle_files: 0,
   666	            raw_bundle_bytes: 0,
   667	            large_tasks: 0,
   668	            large_bytes: 0,
   669	        }
   670	    }

codex
One doc claim is already weaker than written: `error_count` is not literally “every writer hardcodes it to 0” if “writer” means only production appenders. The real production path passes `0` through `record_performance_history`; some named files are test helpers rather than file writers.
codex
I’ll check all `PerformanceRecord` construction sites now, because the draft names four files as “writers” and that wording may not survive code reality.
exec
/usr/bin/zsh -lc "rg -n \"PerformanceRecord::new|PerformanceRecord \\{\" crates/blit-core crates/blit-cli crates/blit-app crates/blit-daemon" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/tests/predictor_streaming.rs:47:fn seed_record(file_count: usize, total_bytes: u64, planner_ms: u128) -> PerformanceRecord {
crates/blit-core/tests/predictor_streaming.rs:48:    PerformanceRecord::new(
crates/blit-core/src/perf_predictor.rs:611:    ) -> PerformanceRecord {
crates/blit-core/src/perf_predictor.rs:637:    ) -> PerformanceRecord {
crates/blit-core/src/perf_predictor.rs:638:        PerformanceRecord {
crates/blit-core/src/perf_predictor.rs:1116:    ) -> PerformanceRecord {
crates/blit-core/src/perf_predictor.rs:1130:        let mut record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:135:pub struct PerformanceRecord {
crates/blit-core/src/perf_history.rs:176:impl PerformanceRecord {
crates/blit-core/src/perf_history.rs:273:pub fn migrate_record(mut record: PerformanceRecord) -> PerformanceRecord {
crates/blit-core/src/perf_history.rs:586:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:698:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:726:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:753:        let record = PerformanceRecord::new(
crates/blit-core/src/engine/tuning.rs:116:    ) -> PerformanceRecord {
crates/blit-core/src/engine/tuning.rs:117:        let mut r = PerformanceRecord::new(
crates/blit-core/src/engine/history.rs:55:) -> PerformanceRecord {
crates/blit-core/src/engine/history.rs:87:    let mut record = PerformanceRecord::new(
crates/blit-core/src/auto_tune/mod.rs:93:    ) -> PerformanceRecord {
crates/blit-core/src/auto_tune/mod.rs:94:        let mut record = PerformanceRecord::new(
crates/blit-core/src/auto_tune/mod.rs:155:    ) -> PerformanceRecord {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/tuning.rs | sed -n '1,95p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! History-derived plan tuning for the engine's streaming strategy.
     2	//! Moved verbatim from `orchestrator/orchestrator.rs` at ue-r2-1c.
     3	
     4	use eyre::Result;
     5	
     6	use crate::perf_history::TransferMode;
     7	
     8	/// Maximum number of recent eligible records the local tuner looks
     9	/// at. The cap exists so a recent regime change (new disk, fresh
    10	/// install) propagates into tuning within ~20 transfers instead of
    11	/// being diluted by older history.
    12	const TUNING_WINDOW_SIZE: usize = 20;
    13	
    14	/// R56-F2: select the window of recent records that should feed
    15	/// `derive_local_plan_tuning`. Filters on `run_kind.is_real_transfer()`
    16	/// FIRST, then the per-operation discriminants, THEN takes the
    17	/// last `TUNING_WINDOW_SIZE`. Pre-fix the take() ran before the
    18	/// run_kind filter, so 20 recent dry-run / null-sink records with
    19	/// matching mode could fill the window and force tuning to fall
    20	/// back to defaults even when older real records existed.
    21	///
    22	/// Extracted so the contract is unit-testable without touching
    23	/// the global perf-history JSONL.
    24	pub(super) fn select_tuning_window(
    25	    history: &[crate::perf_history::PerformanceRecord],
    26	    target_mode: TransferMode,
    27	    compare_mode: crate::perf_history::CompareModeSnapshot,
    28	    skip_unchanged: bool,
    29	) -> Vec<crate::perf_history::PerformanceRecord> {
    30	    history
    31	        .iter()
    32	        .rev()
    33	        .filter(|record| record.run_kind.is_real_transfer())
    34	        .filter(|record| record.mode == target_mode)
    35	        // R59 finding #5: key on the full comparison policy
    36	        // (not just `checksum: bool`) so SizeMtime / SizeOnly /
    37	        // Force / IgnoreTimes runs don't mix into the same tuning
    38	        // bucket. Pre-fix a session of `--size-only` runs trained
    39	        // the SizeMtime bucket (and vice versa).
    40	        .filter(|record| record.options.compare_mode == compare_mode)
    41	        .filter(|record| record.options.skip_unchanged == skip_unchanged)
    42	        .filter(|record| record.fast_path.as_deref() != Some("tiny_manifest"))
    43	        // R58-followup: require a tuning signal. `derive_local_plan_tuning`
    44	        // only aggregates `tar_shard_*` + `raw_bundle_*`; records with
    45	        // `tar_shard_tasks == 0 && raw_bundle_tasks == 0` (no_work,
    46	        // journal_no_work, single_huge_file, streaming no-ops) are
    47	        // RunKind::Real and pass every other gate but contribute
    48	        // nothing. Pre-fix they could fill the 20-slot window and
    49	        // hide older bucket-bearing records. If the tuner ever
    50	        // starts consuming `large_tasks`, add it here too.
    51	        .filter(|record| record.tar_shard_tasks > 0 || record.raw_bundle_tasks > 0)
    52	        .take(TUNING_WINDOW_SIZE)
    53	        .cloned()
    54	        .collect()
    55	}
    56	
    57	/// R57-F1: wrapper that always reads the FULL history before
    58	/// applying the run_kind filter. The caller used to pass
    59	/// `read_recent_records(50)`, which pre-capped the input slice
    60	/// at 50 records — so 50 recent non-real records could hide
    61	/// older real records before `select_tuning_window` ever saw
    62	/// them. Baking the "ask for all records" invariant into the
    63	/// wrapper means the limit can't drift back to a finite value.
    64	/// The history file is already size-capped at ~1 MiB upstream
    65	/// (DEFAULT_MAX_BYTES in perf_history.rs), so reading all
    66	/// records is bounded.
    67	///
    68	/// Generic over the reader so unit tests can inject a synthetic
    69	/// history; production passes `read_recent_records` directly.
    70	/// Returns `None` if the reader errored OR no eligible records
    71	/// were found; the caller treats either case as "fall back to
    72	/// defaults."
    73	pub(super) fn select_tuning_window_from_history<F>(
    74	    reader: F,
    75	    target_mode: TransferMode,
    76	    compare_mode: crate::perf_history::CompareModeSnapshot,
    77	    skip_unchanged: bool,
    78	) -> Option<Vec<crate::perf_history::PerformanceRecord>>
    79	where
    80	    F: FnOnce(usize) -> Result<Vec<crate::perf_history::PerformanceRecord>>,
    81	{
    82	    // `0` means "all records" per read_recent_records' contract
    83	    // (see read_records_from_path in perf_history.rs:298). This
    84	    // is the load-bearing literal — passing anything else
    85	    // reintroduces R57-F1.
    86	    let history = reader(0).ok()?;
    87	    let window = select_tuning_window(&history, target_mode, compare_mode, skip_unchanged);
    88	    if window.is_empty() {
    89	        None
    90	    } else {
    91	        Some(window)
    92	    }
    93	}
    94	
    95	#[cfg(test)]

exec
/usr/bin/zsh -lc "nl -ba crates/blit-app/src/diagnostics/perf.rs | sed -n '1,220p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Performance-history toggles + summary read.
     2	//!
     3	//! Moved from `crates/blit-cli/src/diagnostics.rs` (perf path)
     4	//! in A.0. Thin wrappers around `blit_core::perf_history` plus a
     5	//! `query()` helper that bundles the three reads (enabled flag,
     6	//! history path, recent records) into a single result struct.
     7	
     8	use blit_core::perf_history;
     9	use eyre::Result;
    10	use std::path::PathBuf;
    11	
    12	pub use blit_core::perf_history::PerformanceRecord;
    13	
    14	/// Bundle returned by [`query`]. Callers (`blit diagnostics perf`,
    15	/// future TUI F4 diagnostics screen) render this however they
    16	/// like — JSON, text, table cells.
    17	#[derive(Debug, Clone)]
    18	pub struct PerfReport {
    19	    pub enabled: bool,
    20	    pub history_path: PathBuf,
    21	    pub records: Vec<PerformanceRecord>,
    22	}
    23	
    24	/// Read the persisted "perf history enabled" flag. Separate
    25	/// from [`query`] because the CLI's `diagnostics perf` verb
    26	/// treats the post-toggle refresh as best-effort: a malformed
    27	/// `settings.json` shows the startup warning and falls back to
    28	/// the caller's existing value rather than failing the verb.
    29	/// Callers that want the bundled read (TUI F4 pane, scripted
    30	/// consumers) use [`query`] and propagate the error.
    31	pub fn read_enabled() -> Result<bool> {
    32	    perf_history::perf_history_enabled()
    33	}
    34	
    35	/// Path to `perf_local.jsonl`. Pre-A.0 callers used
    36	/// `perf_history::config_dir()?.join("perf_local.jsonl")`
    37	/// inline; centralizing the join here keeps the filename out of
    38	/// presenter code.
    39	pub fn history_path() -> Result<PathBuf> {
    40	    Ok(perf_history::config_dir()?.join("perf_local.jsonl"))
    41	}
    42	
    43	/// Read up to `limit` most recent records. `0` means "all" per
    44	/// `blit_core::perf_history::read_recent_records`'s contract.
    45	pub fn read_records(limit: usize) -> Result<Vec<PerformanceRecord>> {
    46	    perf_history::read_recent_records(limit)
    47	}
    48	
    49	/// One-call read of the three perf-history surfaces. Convenience
    50	/// for callers that want one-shot reads and don't need the
    51	/// best-effort split (TUI's F4 pane will likely use this; the
    52	/// CLI uses the granular functions above so it can keep
    53	/// pre-A.0's best-effort enabled-refresh semantics).
    54	pub fn query(limit: usize) -> Result<PerfReport> {
    55	    Ok(PerfReport {
    56	        enabled: read_enabled()?,
    57	        history_path: history_path()?,
    58	        records: read_records(limit)?,
    59	    })
    60	}
    61	
    62	/// Persist a new "perf history enabled" setting. Caller refreshes
    63	/// any in-process mirror (e.g. `AppContext.perf_history_enabled`)
    64	/// after this returns.
    65	pub fn set_enabled(enabled: bool) -> Result<()> {
    66	    perf_history::set_perf_history_enabled(enabled)
    67	}
    68	
    69	/// Remove the on-disk history log. Returns `true` if a log
    70	/// existed (and was deleted), `false` if there was nothing to
    71	/// clear.
    72	pub fn clear() -> Result<bool> {
    73	    perf_history::clear_history()
    74	}

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_predictor.rs | sed -n '470,520p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   470	                }
   471	            }
   472	        }
   473	        None
   474	    }
   475	
   476	    pub fn save(&self) -> Result<()> {
   477	        if let Some(parent) = self.path.parent() {
   478	            fs::create_dir_all(parent)?;
   479	        }
   480	        let mut file = File::create(&self.path)?;
   481	        let data = serde_json::to_string_pretty(&self.state)?;
   482	        file.write_all(data.as_bytes())?;
   483	        Ok(())
   484	    }
   485	
   486	    pub fn path(&self) -> &Path {
   487	        &self.path
   488	    }
   489	
   490	    pub fn load_recent_records(
   491	        &self,
   492	        history_path: &Path,
   493	        limit: usize,
   494	    ) -> Result<Vec<PerformanceRecord>> {
   495	        use crate::perf_history::migrate_record;
   496	        let file = File::open(history_path)?;
   497	        let reader = BufReader::new(file);
   498	        let mut records = Vec::new();
   499	        for line in reader.lines().map_while(Result::ok) {
   500	            if let Ok(record) = serde_json::from_str::<PerformanceRecord>(&line) {
   501	                records.push(migrate_record(record));
   502	            }
   503	        }
   504	        let len = records.len();
   505	        if len > limit {
   506	            records = records[len - limit..].to_vec();
   507	        }
   508	        Ok(records)
   509	    }
   510	}
   511	
   512	#[derive(Serialize, Deserialize)]
   513	struct ProfileEntry {
   514	    key: ProfileKey,
   515	    value: PredictorProfile,
   516	}
   517	
   518	mod profile_map {
   519	    use super::{PredictorProfile, ProfileEntry, ProfileKey};
   520	    use serde::{Deserialize, Deserializer, Serialize, Serializer};

exec
/usr/bin/zsh -lc "nl -ba crates/blit-cli/src/cli.rs | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use clap::{Args, Parser, Subcommand};
     2	use std::io::IsTerminal;
     3	use std::path::PathBuf;
     4	
     5	/// Appended to `--help` (not `-h`) for copy/mirror/move so the three
     6	/// semantic rules that bit real users are visible without a manpage trip.
     7	const PATH_SEMANTICS_HELP: &str = "\
     8	PATHS (rsync-style trailing-slash semantics):
     9	  blit copy SRC/  DEST/   merge SRC's contents into DEST (no basename nesting)
    10	  blit copy SRC   DEST/   nest SRC under DEST  -> DEST/<basename(SRC)>
    11	  blit copy SRC   DEST    if DEST exists as a dir: nest; else DEST becomes the copy
    12	  blit copy f.txt DEST/   DEST/f.txt (into the directory)
    13	  blit copy f.txt new.txt rename (when new.txt does not exist)
    14	
    15	A trailing slash on SRC means \"copy the contents\". Without one, the basename is
    16	appended when DEST is (or ends in) a directory; otherwise DEST is the exact
    17	target path. See blit(1) for the full table.";
    18	
    19	/// Appended to `blit check --help` so the equivalence model is
    20	/// discoverable. F12 of docs/reviews/codebase_review_2026-05-01.md.
    21	const CHECK_SEMANTICS_HELP: &str = "\
    22	EQUIVALENCE MODEL:
    23	  blit check verifies that a destination tree matches what `blit copy` or
    24	  `blit mirror` would have produced — not full filesystem equivalence.
    25	
    26	  Compared:    Regular files (by size+mtime, or hash with --checksum).
    27	  Skipped:     Symlinks, FIFOs, devices, and other non-regular entries.
    28	               Empty directories. Two trees differing only in those will
    29	               be reported identical.
    30	  Mismatches:  File-vs-directory at the same path produces a diff entry
    31	               on the file side.
    32	
    33	If you need full filesystem-tree equivalence (symlinks-as-targets,
    34	empty-dir presence, etc.), use `diff -r` or a similar tool.";
    35	
    36	#[derive(Parser)]
    37	#[command(name = "blit")]
    38	#[command(about = "A fast, AI-built file transfer tool (v2)")]
    39	#[command(after_help = "Run '<command> --help' for detailed options on each command.")]
    40	pub struct Cli {
    41	    /// Override the configuration directory for this invocation
    42	    #[arg(long, global = true, value_name = "PATH")]
    43	    pub config_dir: Option<PathBuf>,
    44	
    45	    /// Diagnostics only: write internal byte-path counters to this
    46	    /// file (one `<event> <value>` line per record). Used by the
    47	    /// integration tests and `scripts/bench_remote_remote.sh` to
    48	    /// assert byte-path isolation; not intended for operator use.
    49	    /// Replaces the pre-0.1.1 `BLIT_TEST_COUNTER_FILE` env var
    50	    /// (audit-l39: env vars are out for app + diagnostic config).
    51	    ///
    52	    /// `hide_short_help = true` hides this flag from the short `-h`
    53	    /// summary; it still appears in the full `--help` output so it's
    54	    /// discoverable for troubleshooting.
    55	    #[arg(long, global = true, value_name = "PATH", hide_short_help = true)]
    56	    pub diagnostics_counter_file: Option<PathBuf>,
    57	
    58	    #[command(subcommand)]
    59	    pub command: Commands,
    60	}
    61	
    62	#[derive(Subcommand)]
    63	pub enum Commands {
    64	    /// Copy files between local and/or remote locations (rsync-style slash semantics)
    65	    Copy(TransferArgs),
    66	    /// Mirror a directory, deleting extraneous files at destination (rsync-style slash semantics)
    67	    Mirror(TransferArgs),
    68	    /// Move files (copy + remove source, rsync-style slash semantics)
    69	    Move(TransferArgs),
    70	    /// Discover daemons advertising via mDNS
    71	    Scan(ScanArgs),
    72	    /// List modules exported by a remote daemon
    73	    ListModules(ListModulesArgs),
    74	    /// List directory entries (remote or local)
    75	    #[command(alias = "list")]
    76	    Ls(ListArgs),
    77	    /// Show disk usage for a remote path
    78	    Du(DuArgs),
    79	    /// Show filesystem statistics for a remote module
    80	    Df(DfArgs),
    81	    /// Remove a file or directory on a remote daemon
    82	    Rm(RmArgs),
    83	    /// Search for files on a remote daemon
    84	    Find(FindArgs),
    85	    /// Fetch remote path completions for interactive shells
    86	    Completions(CompletionArgs),
    87	    /// Show local performance history summary
    88	    Profile(ProfileArgs),
    89	    /// Compare two trees by size+mtime or hash (no transfer — read-only verification)
    90	    Check(CheckArgs),
    91	    /// Diagnostics and tooling commands
    92	    Diagnostics {
    93	        #[command(subcommand)]
    94	        command: DiagnosticsCommand,
    95	    },
    96	    /// Inspect transfer jobs on a remote daemon
    97	    Jobs {
    98	        #[command(subcommand)]
    99	        command: JobsCommand,
   100	    },
   101	}
   102	
   103	#[derive(Subcommand)]
   104	pub enum JobsCommand {
   105	    /// List active and recent transfers on a remote daemon
   106	    List(JobsListArgs),
   107	    /// Cancel an active transfer on a remote daemon
   108	    Cancel(JobsCancelArgs),
   109	    /// Watch an active transfer until it completes
   110	    Watch(JobsWatchArgs),
   111	}
   112	
   113	#[derive(Args, Clone, Debug)]
   114	pub struct JobsListArgs {
   115	    /// Remote host (e.g. server or server:port)
   116	    pub remote: String,
   117	    /// Maximum number of recent transfers to return. 0 means
   118	    /// the daemon's default (50).
   119	    #[arg(long, default_value_t = 0)]
   120	    pub recent_limit: u32,
   121	    /// Output as JSON
   122	    #[arg(long)]
   123	    pub json: bool,
   124	}
   125	
   126	#[derive(Args, Clone, Debug)]
   127	pub struct JobsWatchArgs {
   128	    /// Remote host (e.g. server or server:port)
   129	    pub remote: String,
   130	    /// Transfer id to watch — typically obtained from
   131	    /// `blit jobs list <remote>` or the `--detach` output.
   132	    pub transfer_id: String,
   133	    /// Poll interval in milliseconds. Default 1000ms. A future
   134	    /// milestone-C `Subscribe` RPC will replace polling with a
   135	    /// streaming subscription; until then this flag controls
   136	    /// the GetState polling cadence.
   137	    #[arg(long, default_value_t = 1000)]
   138	    pub interval_ms: u64,
   139	    /// Maximum wall-clock seconds to watch before giving up.
   140	    /// 0 = wait forever. Useful for scripts that don't want
   141	    /// to hang on a stuck transfer.
   142	    #[arg(long, default_value_t = 0)]
   143	    pub timeout_secs: u64,
   144	    /// Output as JSON-Lines (one object per poll, plus a
   145	    /// final outcome line). Default is a human-readable
   146	    /// updating ticker.
   147	    #[arg(long)]
   148	    pub json: bool,
   149	}
   150	
   151	#[derive(Args, Clone, Debug)]
   152	pub struct JobsCancelArgs {
   153	    /// Remote host (e.g. server or server:port)
   154	    pub remote: String,
   155	    /// Transfer id to cancel — typically obtained from
   156	    /// `blit jobs list <remote>`.
   157	    pub transfer_id: String,
   158	    /// Output as JSON
   159	    #[arg(long)]
   160	    pub json: bool,
   161	}
   162	
   163	#[derive(Subcommand)]
   164	pub enum DiagnosticsCommand {
   165	    /// Manage performance history capture (enable/disable/clear)
   166	    Perf(PerfArgs),
   167	    /// Emit a diagnostic snapshot for a SRC -> DEST invocation (no transfer performed)
   168	    Dump(DiagnosticsDumpArgs),
   169	}
   170	
   171	#[derive(Args, Clone, Debug)]
   172	pub struct DiagnosticsDumpArgs {
   173	    /// Source path or remote endpoint (same syntax as `blit copy`)
   174	    pub source: String,
   175	    /// Destination path or remote endpoint (same syntax as `blit copy`)
   176	    pub destination: String,
   177	    /// Emit JSON instead of the default human-readable report
   178	    #[arg(long)]
   179	    pub json: bool,
   180	}
   181	
   182	#[derive(Args, Clone, Debug)]
   183	pub struct PerfArgs {
   184	    /// Number of recent records to display (0 = all)
   185	    #[arg(long, default_value_t = 50)]
   186	    pub limit: usize,
   187	    /// Enable performance history capture
   188	    #[arg(long, conflicts_with = "disable")]
   189	    pub enable: bool,
   190	    /// Disable performance history capture
   191	    #[arg(long, conflicts_with = "enable")]
   192	    pub disable: bool,
   193	    /// Remove the stored performance history file
   194	    #[arg(long)]
   195	    pub clear: bool,
   196	    /// Output as JSON
   197	    #[arg(long)]
   198	    pub json: bool,
   199	}
   200	
   201	#[derive(Args, Clone, Debug)]
   202	#[command(after_long_help = PATH_SEMANTICS_HELP)]
   203	pub struct TransferArgs {
   204	    /// Source path or remote endpoint (host:/module/path).
   205	    ///
   206	    /// Trailing slash means "copy contents" (merge). Without a trailing slash,
   207	    /// the source directory is nested under the destination (if destination is
   208	    /// a container) or used as the exact target (otherwise).
   209	    pub source: String,
   210	    /// Destination path or remote endpoint.
   211	    ///
   212	    /// Trailing slash means "into this directory" (container). See `blit(1)`
   213	    /// for the full rsync-style resolution rules.
   214	    pub destination: String,
   215	
   216	    // -- Common options (no heading — rendered in the default "Options"
   217	    // section so first-time users see them at the top).
   218	    /// Perform a dry run without making changes
   219	    #[arg(long)]
   220	    pub dry_run: bool,
   221	    /// Keep verbose logs from the orchestrator
   222	    #[arg(long, short = 'v')]
   223	    pub verbose: bool,
   224	    /// Show an interactive progress indicator.
   225	    ///
   226	    /// Auto-enabled when stdout is a TTY (and --json is not set) so
   227	    /// interactive users get feedback by default; piping/redirecting
   228	    /// stdout disables it so scripts aren't affected. Use this flag to
   229	    /// force-enable when stdout is not a TTY (e.g. under `tee`).
   230	    #[arg(long, short = 'p')]
   231	    pub progress: bool,
   232	    /// Skip confirmation prompt for destructive operations (mirror deletions, move)
   233	    #[arg(long, short = 'y')]
   234	    pub yes: bool,
   235	    /// Output as JSON. With -p, emits NDJSON progress to stderr. Final
   236	    /// transfer summary is written to stdout as a JSON object.
   237	    #[arg(long)]
   238	    pub json: bool,
   239	
   240	    // -- Comparison options: how blit decides which files to transfer.
   241	    /// Force checksum comparison of files (slower but more accurate)
   242	    #[arg(long, short = 'c', help_heading = "Comparison")]
   243	    pub checksum: bool,
   244	    /// Compare only by size, ignoring modification time
   245	    #[arg(long, conflicts_with = "checksum", help_heading = "Comparison")]
   246	    pub size_only: bool,
   247	    /// Transfer all files unconditionally, ignoring size and modification time
   248	    #[arg(long, conflicts_with_all = ["checksum", "size_only"], help_heading = "Comparison")]
   249	    pub ignore_times: bool,
   250	    /// Skip files that already exist on the destination (regardless of differences)
   251	    #[arg(long, conflicts_with = "force", help_heading = "Comparison")]
   252	    pub ignore_existing: bool,
   253	    /// Force exact mirror even if destination files are newer (dangerous)
   254	    #[arg(long, help_heading = "Comparison")]
   255	    pub force: bool,
   256	    /// Mirror deletion scope: `subset` (default) deletes only files in the
   257	    /// source filter scope; `all` deletes any destination file absent from
   258	    /// the (filtered) source set, including files that wouldn't have been
   259	    /// transferred in the first place. `all` is destructive — use with
   260	    /// caution.

codex
The retry wrapper matters for the “exactly one record” criterion. If the recorder wraps the whole `run_with_retries` result in `main`, it would log only the final exhausted error, which is consistent with the draft’s top-level-result wording.
exec
/usr/bin/zsh -lc "nl -ba crates/blit-app/src/transfers/retry.rs | sed -n '1,90p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-cli/src/transfers/mod.rs | sed -n '90,290p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Retry-with-wait for transfers (owner-approved robocopy-style
     2	//! `--retry`/`--wait`). Part 1: the retryable-error classifier and the
     3	//! generic retry loop. Part 2 wires the CLI flags and the transfer
     4	//! dispatch through `run_with_retries`.
     5	//!
     6	//! This is viable because blit transfers are **resumable** — a retry
     7	//! re-runs the same transfer, the manifest compare only re-sends
     8	//! missing/changed files, so a retry continues rather than restarts. The
     9	//! audit-1c stall-timeout is what turns an infinite stall into the clean,
    10	//! fast, retryable failure this loop catches.
    11	
    12	use std::future::Future;
    13	use std::time::Duration;
    14	
    15	use eyre::Result;
    16	
    17	// w5-2: the classifier moved to blit-core (single owner of retry
    18	// policy, next to the transfer code that produces the errors). The
    19	// re-export keeps this module's public API stable.
    20	pub use blit_core::remote::retry::is_retryable;
    21	
    22	/// Run `attempt` (a fresh transfer attempt; `attempt_no` is 0 on the
    23	/// first try) with up to `retries` retries spaced by `wait`. Retries fire
    24	/// only when [`is_retryable`] accepts the error; a fatal error returns
    25	/// immediately. `retries == 0` reproduces the no-retry default.
    26	///
    27	/// The transfer's resumability means each retry continues the prior
    28	/// attempt rather than restarting from scratch.
    29	pub async fn run_with_retries<F, Fut>(retries: u32, wait: Duration, mut attempt: F) -> Result<()>
    30	where
    31	    F: FnMut(u32) -> Fut,
    32	    Fut: Future<Output = Result<()>>,
    33	{
    34	    let mut attempt_no = 0u32;
    35	    loop {
    36	        match attempt(attempt_no).await {
    37	            Ok(()) => return Ok(()),
    38	            Err(err) => {
    39	                if attempt_no >= retries || !is_retryable(&err) {
    40	                    return Err(err);
    41	                }
    42	                attempt_no += 1;
    43	                eprintln!(
    44	                    "blit: transfer failed, retrying ({attempt_no}/{retries}) in {}s: {err:#}",
    45	                    wait.as_secs()
    46	                );
    47	                tokio::time::sleep(wait).await;
    48	            }
    49	        }
    50	    }
    51	}
    52	
    53	#[cfg(test)]
    54	mod tests {
    55	    use super::*;
    56	    use std::cell::Cell;
    57	    use std::io;
    58	
    59	    fn io_err(kind: io::ErrorKind) -> eyre::Report {
    60	        // Wrap in a context layer so the io::Error is a *source* in the
    61	        // chain, mirroring how the transfer code reports it.
    62	        eyre::Report::new(io::Error::new(kind, "boom")).wrap_err("receiving data")
    63	    }
    64	
    65	    #[test]
    66	    fn classifies_transient_io_as_retryable() {
    67	        assert!(is_retryable(&io_err(io::ErrorKind::TimedOut)));
    68	        assert!(is_retryable(&io_err(io::ErrorKind::ConnectionReset)));
    69	        assert!(is_retryable(&io_err(io::ErrorKind::BrokenPipe)));
    70	        assert!(is_retryable(&io_err(io::ErrorKind::UnexpectedEof)));
    71	    }
    72	
    73	    #[test]
    74	    fn classifies_fatal_errors_as_not_retryable() {
    75	        // A plain eyre message (path-safety / gate / invalid-arg shape).
    76	        assert!(!is_retryable(&eyre::eyre!("path escapes module root")));
    77	        // An io error of a non-transient kind.
    78	        assert!(!is_retryable(&io_err(io::ErrorKind::PermissionDenied)));
    79	        assert!(!is_retryable(&io_err(io::ErrorKind::NotFound)));
    80	    }
    81	
    82	    #[tokio::test]
    83	    async fn retries_a_retryable_failure_then_succeeds() {
    84	        let calls = Cell::new(0u32);
    85	        let result = run_with_retries(3, Duration::from_millis(0), |_n| {
    86	            calls.set(calls.get() + 1);
    87	            let this_call = calls.get();
    88	            async move {
    89	                if this_call < 3 {
    90	                    Err(io_err(io::ErrorKind::TimedOut)) // transient: retried

 succeeded in 0ms:
    90	        return Ok(true);
    91	    }
    92	
    93	    print!("{} [y/N]: ", message);
    94	    io::stdout().flush()?;
    95	    let mut input = String::new();
    96	    io::stdin().read_line(&mut input)?;
    97	    let decision = input.trim().to_ascii_lowercase();
    98	    Ok(decision == "y" || decision == "yes")
    99	}
   100	
   101	pub async fn run_transfer(ctx: &AppContext, args: &TransferArgs, mode: TransferKind) -> Result<()> {
   102	    let src_endpoint = parse_transfer_endpoint(&args.source)?;
   103	    let raw_dst = parse_transfer_endpoint(&args.destination)?;
   104	    let pre_resolve_display = display_endpoint(&raw_dst);
   105	    let dst_endpoint = resolve_destination(&args.source, &args.destination, &src_endpoint, raw_dst);
   106	
   107	    let operation = match mode {
   108	        TransferKind::Copy => "copy",
   109	        TransferKind::Mirror => "mirror",
   110	    };
   111	    let src_display = display_endpoint(&src_endpoint);
   112	    let dst_display = display_endpoint(&dst_endpoint);
   113	
   114	    // R54-F1 (data-loss / silent bug): `--null` only works
   115	    // correctly for LOCAL COPY. Outside that envelope it's
   116	    // either destructive or silently ignored:
   117	    //   - `blit mirror --null`: the null sink discards writes,
   118	    //     but `apply_mirror_deletions` still runs (only
   119	    //     `options.dry_run` gates the actual remove_* calls)
   120	    //     and deletes destination-only files — turning a
   121	    //     supposedly read-only benchmark into a destructive op.
   122	    //   - `blit copy --null` to/from a remote endpoint: the
   123	    //     remote push/pull paths don't implement null
   124	    //     semantics, so the flag is silently ignored and a
   125	    //     normal write happens.
   126	    //
   127	    // The narrowest safe contract for 0.1.0: --null is local
   128	    // copy only. Reject the other combinations at the CLI;
   129	    // proper plumbing of null semantics through mirror-delete
   130	    // and the remote paths is a post-release item.
   131	    if args.null {
   132	        if mode.is_mirror() {
   133	            bail!(
   134	                "--null is not supported with `blit mirror`: the \
   135	                 null sink discards writes, but mirror's \
   136	                 destination-purge step would still delete \
   137	                 destination-only files, turning what's supposed \
   138	                 to be a read-only benchmark into a destructive \
   139	                 operation. Use `blit copy --null SRC DST` (local \
   140	                 only) for read-path benchmarking."
   141	            );
   142	        }
   143	        if matches!(src_endpoint, Endpoint::Remote(_))
   144	            || matches!(dst_endpoint, Endpoint::Remote(_))
   145	        {
   146	            bail!(
   147	                "--null is not supported with remote endpoints: \
   148	                 the remote push/pull paths don't implement null \
   149	                 semantics, so the flag would be silently \
   150	                 ignored and a real write would happen. Use \
   151	                 `blit copy --null SRC DST` between two local \
   152	                 paths for read-path benchmarking."
   153	            );
   154	        }
   155	    }
   156	
   157	    // `--detach` is only honored on daemon-to-daemon
   158	    // delegated transfers. The CLI gates it up-front so a
   159	    // misuse fails before any RPCs fire — clearer than
   160	    // letting the daemon emit a phased error mid-stream.
   161	    if args.detach {
   162	        match (&src_endpoint, &dst_endpoint) {
   163	            (Endpoint::Local(_), _) | (_, Endpoint::Local(_)) => bail!(
   164	                "--detach is only supported for remote→remote transfers \
   165	                 (the CLI is in the byte path for any local endpoint, so \
   166	                 disconnecting would drop the bytes)"
   167	            ),
   168	            (Endpoint::Remote(_), Endpoint::Remote(_)) if args.relay_via_cli => bail!(
   169	                "--detach is incompatible with --relay-via-cli: the relay \
   170	                 path puts the CLI in the byte path, so detach would drop \
   171	                 the bytes. Drop --relay-via-cli to use the daemon-to-daemon \
   172	                 delegated path (which is the default for remote→remote)."
   173	            ),
   174	            (Endpoint::Remote(_), Endpoint::Remote(_)) => {
   175	                // Delegated remote→remote — detach is valid.
   176	            }
   177	        }
   178	    }
   179	
   180	    // audit-h1 round 2: refuse `mirror --relay-via-cli` for
   181	    // remote→remote BEFORE the destructive-confirm prompt below.
   182	    // Reviewer caught that the original h1 fix placed the gate
   183	    // inside the RemoteToRemoteRelay branch, AFTER this prompt —
   184	    // so an operator without --yes could defeat the data-loss
   185	    // guard by answering "no" (or EOF / empty stdin) and the
   186	    // process would exit 0 with no surfacing of the unsafe
   187	    // combination. The reject-gate must precede every user-
   188	    // interaction surface for the bail's intent to land.
   189	    //
   190	    // Same data-loss reasoning as the in-route comment below.
   191	    // Symmetric with the --detach reject-gate at line 161 above
   192	    // and with the `move --relay-via-cli` gate further down.
   193	    if mode.is_mirror() && args.relay_via_cli {
   194	        if let (Endpoint::Remote(_), Endpoint::Remote(_)) = (&src_endpoint, &dst_endpoint) {
   195	            bail!(
   196	                "mirror does not support --relay-via-cli with remote \
   197	                 endpoints: the legacy relay path does not verify \
   198	                 that the source-side scan was complete, so an \
   199	                 unreadable subtree on the source daemon would let \
   200	                 mirror's destination-purge step delete destination-\
   201	                 only files that may correspond to the unreadable \
   202	                 source entries. Drop --relay-via-cli to use the \
   203	                 direct delegated path, which enforces the \
   204	                 complete-scan gate."
   205	            );
   206	        }
   207	    }
   208	
   209	    // For mirror operations, prompt unless --yes or --dry-run
   210	    if mode.is_mirror() && !args.dry_run {
   211	        let prompt = format!(
   212	            "Mirror will delete extraneous files at destination '{}'. Continue?",
   213	            dst_display
   214	        );
   215	        if !confirm_destructive_operation(&prompt, args.yes)? {
   216	            println!("Aborted.");
   217	            return Ok(());
   218	        }
   219	    }
   220	
   221	    // Banner goes to stderr so stdout stays reserved for the summary /
   222	    // JSON output. Version dropped — `blit --version` is the right place
   223	    // for that, not every invocation.
   224	    if !args.json {
   225	        eprintln!("starting {} {} -> {}", operation, src_display, dst_display);
   226	        if args.verbose && dst_display != pre_resolve_display {
   227	            eprintln!(
   228	                "  (destination resolved by rsync trailing-slash rule: {} -> {})",
   229	                pre_resolve_display, dst_display
   230	            );
   231	        }
   232	    }
   233	
   234	    match select_transfer_route(src_endpoint, dst_endpoint, mode, args.relay_via_cli) {
   235	        TransferRoute::LocalToLocal { src, dst, mirror } => {
   236	            if !src.exists() {
   237	                bail!("source path does not exist: {}", src.display());
   238	            }
   239	            run_local_transfer(ctx, args, &src, &dst, mirror)
   240	                .await
   241	                .map(|_| ())
   242	        }
   243	        TransferRoute::LocalToRemote { src, dst, mirror } => {
   244	            if !src.exists() {
   245	                bail!("source path does not exist: {}", src.display());
   246	            }
   247	            ensure_remote_push_supported(args)?;
   248	            ensure_remote_destination_supported(&dst)?;
   249	            run_remote_push_transfer(args, Endpoint::Local(src), dst, mirror).await
   250	        }
   251	        TransferRoute::RemoteToLocal { src, dst, mirror } => {
   252	            ensure_remote_pull_supported(args)?;
   253	            ensure_remote_source_supported(&src)?;
   254	            run_remote_pull_transfer(
   255	                args, src, &dst, mirror, false, // not a move — source survives
   256	            )
   257	            .await
   258	        }
   259	        TransferRoute::RemoteToRemoteRelay { src, dst, mirror } => {
   260	            ensure_remote_source_supported(&src)?;
   261	            ensure_remote_destination_supported(&dst)?;
   262	            // audit-h1 (data-loss): mirror is rejected up-front before
   263	            // any user-interaction surface (mirror confirm prompt) —
   264	            // see the gate above near line 180. The relay path's
   265	            // RemoteTransferSource::scan discards unreadable_paths,
   266	            // so for mirror this risks a destination-purge run on
   267	            // an incomplete source view. Copy is fine: no purge step.
   268	            // If a future change loosens the front gate, this branch
   269	            // would silently regress, so keep a debug_assert as
   270	            // defense-in-depth.
   271	            debug_assert!(
   272	                !mirror,
   273	                "audit-h1: mirror --relay-via-cli reaching the relay branch — \
   274	                 the front gate in run_transfer was bypassed",
   275	            );
   276	            ensure_remote_push_supported(args)?;
   277	            run_remote_push_transfer(args, Endpoint::Remote(src), dst, mirror).await
   278	        }
   279	        TransferRoute::RemoteToRemoteDelegated { src, dst, mirror } => {
   280	            ensure_remote_source_supported(&src)?;
   281	            ensure_remote_destination_supported(&dst)?;
   282	            ensure_remote_pull_supported(args)?;
   283	            run_remote_to_remote_direct(args, src, dst, mirror, false /* not a move */).await
   284	        }
   285	    }
   286	}
   287	
   288	pub async fn run_move(ctx: &AppContext, args: &TransferArgs) -> Result<()> {
   289	    let src_endpoint = parse_transfer_endpoint(&args.source)?;
   290	    let raw_dst = parse_transfer_endpoint(&args.destination)?;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-cli/src/transfers/local.rs | sed -n '1,140p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	use crate::cli::TransferArgs;
     2	use crate::context::AppContext;
     3	use blit_app::display::format_bytes;
     4	use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOutcome};
     5	use eyre::{bail, Result};
     6	use indicatif::{ProgressBar, ProgressStyle};
     7	use std::path::Path;
     8	use std::time::{Duration, Instant};
     9	
    10	/// Convenience wrapper for callers that always want the summary
    11	/// printed inline. Most CLI paths (copy / mirror) want this; move
    12	/// uses [`run_local_transfer_deferred`] so it can suppress the
    13	/// "success" output until after the source-delete decision is
    14	/// made (R49-F3).
    15	pub async fn run_local_transfer(
    16	    ctx: &AppContext,
    17	    args: &TransferArgs,
    18	    src_path: &Path,
    19	    dest_path: &Path,
    20	    mirror: bool,
    21	) -> Result<LocalMirrorSummary> {
    22	    run_local_transfer_inner(ctx, args, src_path, dest_path, mirror, false).await
    23	}
    24	
    25	/// Same as [`run_local_transfer`] but the caller takes ownership
    26	/// of when (and whether) to print the final summary. Move uses
    27	/// this so a failure during source-delete can still surface as
    28	/// an error without first having emitted a successful-looking
    29	/// JSON document on stdout.
    30	pub async fn run_local_transfer_deferred(
    31	    ctx: &AppContext,
    32	    args: &TransferArgs,
    33	    src_path: &Path,
    34	    dest_path: &Path,
    35	    mirror: bool,
    36	) -> Result<LocalMirrorSummary> {
    37	    run_local_transfer_inner(ctx, args, src_path, dest_path, mirror, true).await
    38	}
    39	
    40	/// Print the standard summary block for a completed local
    41	/// transfer. Exposed for `run_local_transfer_deferred` callers
    42	/// (move) that need to emit output AFTER their own follow-up
    43	/// (source-delete) succeeds. Mirrors the inline print in
    44	/// `run_local_transfer_inner` so deferred + inline callers
    45	/// produce byte-identical output.
    46	pub fn print_local_transfer_summary(
    47	    ctx: &AppContext,
    48	    args: &TransferArgs,
    49	    mirror: bool,
    50	    summary: &LocalMirrorSummary,
    51	    elapsed: Duration,
    52	    src_path: &Path,
    53	    dest_path: &Path,
    54	) -> Result<()> {
    55	    let options = build_local_options(ctx, args, mirror)?;
    56	    if args.json {
    57	        print_summary_json(mirror, summary, elapsed, src_path, dest_path);
    58	    } else {
    59	        print_summary(
    60	            mirror,
    61	            options.dry_run,
    62	            options.null_sink,
    63	            options.verbose,
    64	            options.debug_mode,
    65	            options.workers,
    66	            summary,
    67	            elapsed,
    68	        );
    69	    }
    70	    Ok(())
    71	}
    72	
    73	async fn run_local_transfer_inner(
    74	    ctx: &AppContext,
    75	    args: &TransferArgs,
    76	    src_path: &Path,
    77	    dest_path: &Path,
    78	    mirror: bool,
    79	    defer_output: bool,
    80	) -> Result<LocalMirrorSummary> {
    81	    if !src_path.exists() {
    82	        bail!("source path does not exist: {}", src_path.display());
    83	    }
    84	
    85	    let options = build_local_options(ctx, args, mirror)?;
    86	    let dry_run = options.dry_run;
    87	    let null_sink = options.null_sink;
    88	    let json_output = args.json;
    89	    let verbose = options.verbose;
    90	    let debug_mode = options.debug_mode;
    91	    let workers = options.workers;
    92	    if debug_mode {
    93	        eprintln!(
    94	            "blit: debug: worker limiter active – FAST planner auto-tuning capped to {workers} thread(s)."
    95	        );
    96	    }
    97	
    98	    let progress_bar = if !args.effective_progress() {
    99	        None
   100	    } else {
   101	        let pb = ProgressBar::new_spinner();
   102	        pb.set_style(
   103	            ProgressStyle::with_template("{spinner} {msg}")
   104	                .unwrap()
   105	                .tick_strings(&["-", "\\", "|", "/"]),
   106	        );
   107	        pb.enable_steady_tick(Duration::from_millis(120));
   108	        pb.set_message(format!(
   109	            "{} {} → {}",
   110	            if mirror { "Mirroring" } else { "Copying" },
   111	            src_path.display(),
   112	            dest_path.display()
   113	        ));
   114	        Some(pb)
   115	    };
   116	
   117	    let start = Instant::now();
   118	    let summary = blit_app::transfers::local::run(src_path, dest_path, options).await?;
   119	
   120	    if let Some(pb) = progress_bar {
   121	        pb.finish_and_clear();
   122	    }
   123	
   124	    let elapsed = start.elapsed();
   125	    if !defer_output {
   126	        if json_output {
   127	            print_summary_json(mirror, &summary, elapsed, src_path, dest_path);
   128	        } else {
   129	            print_summary(
   130	                mirror, dry_run, null_sink, verbose, debug_mode, workers, &summary, elapsed,
   131	            );
   132	        }
   133	    }
   134	
   135	    Ok(summary)
   136	}
   137	
   138	fn build_local_options(
   139	    ctx: &AppContext,
   140	    args: &TransferArgs,

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mod.rs | sed -n '90,130p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba crates/blit-app/src/transfers/local.rs | sed -n '1,80p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Local→local transfer: the orchestration call.
     2	//!
     3	//! First sub-slice of the transfers track. Moves only the
     4	//! `spawn_blocking` lift + the orchestrator invocation + the
     5	//! error-context wrapping out of `crates/blit-cli/src/transfers
     6	//! /local.rs`. Everything else (clap-arg → `LocalMirrorOptions`
     7	//! translation, indicatif spinner, summary print) stays in the
     8	//! CLI for now; those will move when `transfers/mod.rs` lands
     9	//! (it owns `build_filter` etc., which the options builder
    10	//! depends on).
    11	//!
    12	//! The CLI's `build_local_options` still reads `TransferArgs`
    13	//! directly, so this function accepts the already-built
    14	//! `LocalMirrorOptions` struct from blit-core. The TUI's future
    15	//! local-transfer trigger will build its own `LocalMirrorOptions`
    16	//! from a TUI input modal and call this function — same shape,
    17	//! no clap coupling.
    18	
    19	use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOrchestrator};
    20	use eyre::{Context, Result};
    21	use std::path::Path;
    22	
    23	pub use blit_core::orchestrator::TransferOutcome;
    24	
    25	/// Run a local→local copy / mirror to completion. Wraps the
    26	/// blocking orchestrator call in `spawn_blocking` so async
    27	/// callers don't stall the runtime. Returns the summary
    28	/// verbatim; caller handles presentation (spinner clear,
    29	/// stdout / JSON / TUI render).
    30	///
    31	/// `options.mirror` decides copy vs mirror semantics inside the
    32	/// orchestrator and also drives the error-message wording
    33	/// ("failed to mirror …" vs "failed to copy …") when the
    34	/// underlying call fails — matching the pre-A.0 CLI version
    35	/// of this site.
    36	pub async fn run(
    37	    src: &Path,
    38	    dst: &Path,
    39	    options: LocalMirrorOptions,
    40	) -> Result<LocalMirrorSummary> {
    41	    let src = src.to_path_buf();
    42	    let dst = dst.to_path_buf();
    43	    let mirror = options.mirror;
    44	    tokio::task::spawn_blocking(move || {
    45	        TransferOrchestrator::new()
    46	            .execute_local_mirror(&src, &dst, options)
    47	            .with_context(|| {
    48	                format!(
    49	                    "failed to {} from {} to {}",
    50	                    if mirror { "mirror" } else { "copy" },
    51	                    src.display(),
    52	                    dst.display()
    53	                )
    54	            })
    55	    })
    56	    .await?
    57	}

 succeeded in 0ms:
    90	
    91	impl TransferEngine {
    92	    pub fn new() -> Self {
    93	        Self
    94	    }
    95	
    96	    /// Execute one transfer: select a strategy (single-file, journal
    97	    /// no-work, fast path, or streaming pipeline) and run it to a
    98	    /// summary. Behavior moved verbatim from
    99	    /// `TransferOrchestrator::execute_local_mirror_async` at
   100	    /// ue-r2-1c; the caller-visible contract is unchanged.
   101	    pub async fn execute(&self, request: EngineRequest) -> Result<LocalMirrorSummary> {
   102	        let EngineRequest {
   103	            src_root,
   104	            dest_root,
   105	            source,
   106	            sink,
   107	            options,
   108	        } = request;
   109	        let src_root = src_root.as_path();
   110	        let dest_root = dest_root.as_path();
   111	
   112	        let start_time = Instant::now();
   113	
   114	        // Single-file source: bypass the enumerator/planner/pipeline machinery
   115	        // entirely and copy the file directly. The destination resolver in the
   116	        // CLI has already produced the exact target path (accounting for
   117	        // trailing-slash / existing-dir semantics), so we just invoke copy_file.
   118	        // Without this short-circuit, the enumerator would skip the depth-0
   119	        // root entry and the fast-path would report NoWork — silent data loss.
   120	        if src_root.is_file() {
   121	            return execute_single_file_copy(src_root, dest_root, &options, start_time);
   122	        }
   123	
   124	        let mut journal_tracker = ChangeTracker::load().ok();
   125	        let mut journal_tokens: Vec<ProbeToken> = Vec::new();
   126	        let mut journal_skip = false;
   127	
   128	        let mut predictor = PerformancePredictor::load().ok();
   129	
   130	        let copy_config = CopyConfig {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/auto_tune/mod.rs | sed -n '1,75p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! History-derived local planning thresholds.
     2	//!
     3	//! w2-1 deleted the dead warmup machinery (`analyze_warmup_result`,
     4	//! `determine_tuning`); ue-r2-1e replaced the remaining static remote
     5	//! table (`remote::tuning::determine_remote_tuning`) with the live
     6	//! engine dial (`crate::engine::TransferDial`); w2-2 removed the
     7	//! orphaned `TuningParams` carrier those left behind. What remains is
     8	//! the local-plan tuner below: it derives tar-shard / raw-bundle
     9	//! targets for `transfer_plan::PlanOptions` from perf-history
    10	//! records. A real warmup probe is H10b-class future work behind its
    11	//! own plan doc.
    12	
    13	/// Local plan tuning derived from historical performance records.
    14	#[derive(Debug, Clone, Copy, PartialEq, Eq)]
    15	pub struct LocalPlanTuning {
    16	    pub small_target_bytes: u64,
    17	    pub small_count_target: usize,
    18	    pub medium_target_bytes: u64,
    19	}
    20	
    21	/// Derive local planning thresholds from recent performance history.
    22	///
    23	/// R56-F1: only real-transfer records contribute to the aggregate.
    24	/// Dry-run records have zero write bytes by design, null-sink
    25	/// records have zero write bytes by definition, and bench records
    26	/// represent diagnostic workloads — including any of them would
    27	/// teach the tuner that destination writes are cheap or free, and
    28	/// that's exactly the contamination the run_kind lane was added
    29	/// to prevent.
    30	pub fn derive_local_plan_tuning(
    31	    records: &[crate::perf_history::PerformanceRecord],
    32	) -> Option<LocalPlanTuning> {
    33	    if records.is_empty() {
    34	        return None;
    35	    }
    36	
    37	    let mut tar_tasks: u64 = 0;
    38	    let mut tar_bytes: u128 = 0;
    39	    let mut tar_files: u64 = 0;
    40	    let mut raw_tasks: u64 = 0;
    41	    let mut raw_bytes: u128 = 0;
    42	
    43	    for record in records {
    44	        if !record.run_kind.is_real_transfer() {
    45	            continue;
    46	        }
    47	        tar_tasks = tar_tasks.saturating_add(record.tar_shard_tasks as u64);
    48	        tar_bytes = tar_bytes.saturating_add(record.tar_shard_bytes as u128);
    49	        tar_files = tar_files.saturating_add(record.tar_shard_files as u64);
    50	        raw_tasks = raw_tasks.saturating_add(record.raw_bundle_tasks as u64);
    51	        raw_bytes = raw_bytes.saturating_add(record.raw_bundle_bytes as u128);
    52	    }
    53	
    54	    if tar_tasks == 0 && raw_tasks == 0 {
    55	        return None;
    56	    }
    57	
    58	    let mut small_target_bytes = 8 * 1024 * 1024;
    59	    let mut small_count_target: usize = 2048;
    60	    if tar_tasks > 0 && tar_bytes > 0 {
    61	        let avg_bytes = (tar_bytes / u128::from(tar_tasks)).min(u128::from(u64::MAX)) as u64;
    62	        small_target_bytes = avg_bytes.clamp(4 * 1024 * 1024, 128 * 1024 * 1024);
    63	    }
    64	    if tar_tasks > 0 && tar_files > 0 {
    65	        let avg_files = (tar_files / tar_tasks) as usize;
    66	        small_count_target = avg_files.clamp(128, 4096);
    67	    }
    68	
    69	    let mut medium_target_bytes = 128 * 1024 * 1024;
    70	    if raw_tasks > 0 && raw_bytes > 0 {
    71	        let avg_bytes = (raw_bytes / u128::from(raw_tasks)).min(u128::from(u64::MAX)) as u64;
    72	        medium_target_bytes = avg_bytes.clamp(64 * 1024 * 1024, 512 * 1024 * 1024);
    73	    }
    74	
    75	    Some(LocalPlanTuning {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_predictor.rs | sed -n '1100,1145p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1100	        } else {
  1101	            parsed
  1102	        };
  1103	        assert_eq!(post.version, STATE_VERSION);
  1104	        assert!(post.profiles.is_empty());
  1105	    }
  1106	
  1107	    // ── R56-F1: observe() filters non-real records ────────────────────
  1108	
  1109	    /// Build a record with the requested durations + lane. Uses the
  1110	    /// same shape as the existing test helpers so the contract is
  1111	    /// "same record, different lane → different observe behavior."
  1112	    fn record_with_lane(
  1113	        kind: crate::perf_history::RunKind,
  1114	        planner_ms: u128,
  1115	        transfer_ms: u128,
  1116	    ) -> PerformanceRecord {
  1117	        let opts = crate::perf_history::OptionSnapshot {
  1118	            dry_run: matches!(kind, crate::perf_history::RunKind::DryRun),
  1119	            preserve_symlinks: false,
  1120	            include_symlinks: false,
  1121	            skip_unchanged: true,
  1122	            checksum: false,
  1123	            compare_mode: CompareModeSnapshot::SizeMtime,
  1124	            workers: 4,
  1125	        };
  1126	        let fast_path = match kind {
  1127	            crate::perf_history::RunKind::NullSink => Some("null_sink".to_string()),
  1128	            _ => None,
  1129	        };
  1130	        let mut record = PerformanceRecord::new(
  1131	            TransferMode::Copy,
  1132	            None,
  1133	            None,
  1134	            100,
  1135	            1024 * 1024,
  1136	            opts,
  1137	            fast_path,
  1138	            planner_ms,
  1139	            transfer_ms,
  1140	            0,
  1141	            0,
  1142	        );
  1143	        record.run_kind = kind;
  1144	        record
  1145	    }

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mod.rs | sed -n '500,750p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   500	                .map(|rel| rel.to_path_buf())
   501	        } else {
   502	            None
   503	        };
   504	        let (payload_tx, payload_rx) =
   505	            mpsc::channel::<TransferPayload>(DEFAULT_PAYLOAD_PREFETCH.max(1));
   506	        let planner_fut = run_streaming_plan(
   507	            header_rx,
   508	            StreamingPlanInputs {
   509	                src_root: src_root.to_path_buf(),
   510	                dest_root: dest_root.to_path_buf(),
   511	                compare_mode,
   512	                ignore_existing: options.ignore_existing,
   513	                skip_unchanged: options.skip_unchanged,
   514	                initial,
   515	                collect_source_paths: options.mirror,
   516	                exclude_dest_subtree,
   517	            },
   518	            payload_tx,
   519	            planning_start,
   520	        );
   521	        let pipeline_fut = execute_sink_pipeline_streaming(
   522	            source.clone(),
   523	            vec![sink],
   524	            payload_rx,
   525	            DEFAULT_PAYLOAD_PREFETCH,
   526	            None,
   527	        );
   528	        let (plan_result, pipeline_result) = tokio::join!(planner_fut, pipeline_fut);
   529	        // Observe the scan task UNCONDITIONALLY before any error
   530	        // return (codex ue-r2-1d F2): both join arms have completed,
   531	        // so the walker has either finished or aborted on its dead
   532	        // header channel — this await is prompt and never detaches a
   533	        // running scan or drops its panic.
   534	        let scan_result = scan_handle.await;
   535	        // Error precedence: the pipeline's error is the root cause when
   536	        // the planner aborted on a payload-send failure (the walker
   537	        // then also aborts with a queue error) — so surface pipeline
   538	        // first, then planner (diff/plan failures), then the scan
   539	        // result (walk errors reach the planner only as a channel
   540	        // close; the real error lives in the handle).
   541	        let pipeline_outcome = pipeline_result.context("transfer pipeline failed")?;
   542	        let plan_outcome = plan_result?;
   543	        let _total_scanned = scan_result
   544	            .context("scan task panicked")?
   545	            .context("scan failed")?;
   546	
   547	        // Phase split under overlap (ue-r2-1d redefinition, consumed by
   548	        // the predictor): `planner` = serial latency until the FIRST
   549	        // payload reached the pipeline (what the user waits before
   550	        // bytes can move); `transfer` = the remainder. Pre-1d the split
   551	        // was scan+plan vs pipeline, which streaming dissolves.
   552	        let total_elapsed_ms = planning_start.elapsed().as_millis();
   553	        let planner_duration_ms = plan_outcome
   554	            .first_payload_elapsed
   555	            .map(|d| d.as_millis())
   556	            .unwrap_or(total_elapsed_ms);
   557	
   558	        // §2.8 phase 2: query the predictor for the estimate surfaced
   559	        // in summary.predictor_estimate (`--verbose`, `blit profile
   560	        // --json`). ue-r2-1d: the query needs final scan totals, which
   561	        // streaming only knows once the scan ends — so it now runs
   562	        // after the transfer instead of before it. It still runs
   563	        // BEFORE observe() (record_performance_history →
   564	        // update_predictor below), so train/query hygiene is intact.
   565	        //
   566	        // R44-F1: query and observation must use the same feature
   567	        // vector. We query with `(scanned_files, scanned_bytes)`
   568	        // here; `record_performance_history` populates the matching
   569	        // `PerformanceRecord.{file_count,total_bytes}` from
   570	        // `summary.{scanned_files,scanned_bytes}`. Pre-fix the
   571	        // record was populated from `summary.copied_files`, so on
   572	        // any incremental run the predictor was queried with one
   573	        // workload size and trained against another.
   574	        //
   575	        // src_fs/dest_fs are left None for 0.1.0 — wiring
   576	        // `fs_capability` per-path probes into the predictor query
   577	        // is post-release work (see §3.3 / Phase 4.8.2 deferral).
   578	        let scanned_files = plan_outcome.scanned_files;
   579	        let scanned_bytes: u64 = plan_outcome.scanned_bytes;
   580	        // R45 follow-up to R44-F1: never alias `total_bytes` to
   581	        // `scanned_bytes`. `summary.total_bytes` is the
   582	        // pipeline-wrote-bytes contract (see `LocalMirrorSummary`
   583	        // rustdoc); the predictor uses scan features only. Pre-fix
   584	        // this aliased the two so `summary.total_bytes` reported
   585	        // scanned bytes as bytes-written, overcounting throughput
   586	        // on incremental runs.
   587	        let predictor_estimate = predictor.as_ref().and_then(|p| {
   588	            let kind_total = crate::perf_predictor::DurationKind::Total;
   589	            let mode = if options.mirror {
   590	                crate::perf_history::TransferMode::Mirror
   591	            } else {
   592	                crate::perf_history::TransferMode::Copy
   593	            };
   594	            let total_pred = p.predict(
   595	                kind_total,
   596	                mode.clone(),
   597	                None,
   598	                None,
   599	                None,
   600	                options.skip_unchanged,
   601	                options.checksum,
   602	                scanned_files,
   603	                scanned_bytes,
   604	            )?;
   605	            // Pull planner + transfer separately too so the verbose
   606	            // line and the JSON profile can break down the estimate.
   607	            // All three predictor calls share the same
   608	            // (scanned_files, scanned_bytes) feature vector — both
   609	            // for consistency with the recording side, and so a
   610	            // future maintainer can't accidentally reintroduce a
   611	            // train/query mismatch by editing one branch and
   612	            // missing another.
   613	            let planner_pred = p
   614	                .predict(
   615	                    crate::perf_predictor::DurationKind::Planner,
   616	                    mode.clone(),
   617	                    None,
   618	                    None,
   619	                    None,
   620	                    options.skip_unchanged,
   621	                    options.checksum,
   622	                    scanned_files,
   623	                    scanned_bytes,
   624	                )
   625	                .map(|p| p.predicted_ms)
   626	                .unwrap_or(0.0);
   627	            let transfer_pred = p
   628	                .predict(
   629	                    crate::perf_predictor::DurationKind::Transfer,
   630	                    mode,
   631	                    None,
   632	                    None,
   633	                    None,
   634	                    options.skip_unchanged,
   635	                    options.checksum,
   636	                    scanned_files,
   637	                    scanned_bytes,
   638	                )
   639	                .map(|p| p.predicted_ms)
   640	                .unwrap_or(0.0);
   641	            Some(self::summary::PredictorEstimate {
   642	                planner_ms: planner_pred.max(0.0) as u128,
   643	                transfer_ms: transfer_pred.max(0.0) as u128,
   644	                total_ms: total_pred.predicted_ms.max(0.0) as u128,
   645	                observations: total_pred.observations,
   646	                fallback_depth: total_pred.fallback_depth,
   647	            })
   648	        });
   649	        if options.verbose {
   650	            if let Some(est) = predictor_estimate.as_ref() {
   651	                eprintln!(
   652	                    "Predictor estimate: planner ~{} ms, transfer ~{} ms, \
   653	                     total ~{} ms (n={}, fallback_depth={})",
   654	                    est.planner_ms,
   655	                    est.transfer_ms,
   656	                    est.total_ms,
   657	                    est.observations,
   658	                    est.fallback_depth
   659	                );
   660	            } else {
   661	                eprintln!("Predictor estimate: unavailable (no profile yet for this workload)");
   662	            }
   663	        }
   664	
   665	        let transfer_duration_ms = total_elapsed_ms.saturating_sub(planner_duration_ms);
   666	
   667	        // R47-F4: snapshot unreadable paths so the CLI's source-
   668	        // delete step (in `blit move`) can refuse to remove a
   669	        // source it couldn't fully scan. The R46-F2 gate inside
   670	        // the orchestrator only fires on `options.mirror`, but
   671	        // move uses mirror=false — without this surface, an
   672	        // unreadable source file would get skipped during the
   673	        // copy and then silently deleted from the source by the
   674	        // CLI's `remove_dir_all` step.
   675	        let unreadable_snapshot: Vec<String> = unreadable
   676	            .lock()
   677	            .map(|guard| guard.clone())
   678	            .unwrap_or_default();
   679	
   680	        let mut summary = LocalMirrorSummary {
   681	            planned_files: pipeline_outcome.files_written,
   682	            copied_files: pipeline_outcome.files_written,
   683	            // R45: bytes the pipeline actually wrote, not scanned
   684	            // bytes. Distinct on incremental runs.
   685	            total_bytes: pipeline_outcome.bytes_written,
   686	            scanned_files,
   687	            scanned_bytes,
   688	            dry_run: options.dry_run,
   689	            duration: start_time.elapsed(),
   690	            predictor_estimate: predictor_estimate.clone(),
   691	            unreadable_paths: unreadable_snapshot.clone(),
   692	            ..Default::default()
   693	        };
   694	
   695	        if options.mirror {
   696	            // R46-F2: refuse to mirror-delete when the source scan
   697	            // was incomplete. The `unreadable_snapshot` captured
   698	            // above (R47-F4) covers the per-file open path
   699	            // (PermissionDenied / NotFound on individual files) and
   700	            // the walkdir non-root error path (unreadable
   701	            // subdirectories). Either case means the header set
   702	            // we're about to use as the source-of-truth for "what
   703	            // the destination should contain" is missing entries,
   704	            // and a delete pass would silently remove matching
   705	            // destination subtrees.
   706	            if !unreadable_snapshot.is_empty() {
   707	                bail!(
   708	                    "refusing to mirror-delete from {}: source scan was \
   709	                     incomplete ({} unreadable entr{}); the first {} \
   710	                     reported: {}. Resolve the scan errors (typically \
   711	                     permissions) or run as a non-mirror copy.",
   712	                    dest_root.display(),
   713	                    unreadable_snapshot.len(),
   714	                    if unreadable_snapshot.len() == 1 {
   715	                        "y"
   716	                    } else {
   717	                        "ies"
   718	                    },
   719	                    unreadable_snapshot.len().min(5),
   720	                    unreadable_snapshot
   721	                        .iter()
   722	                        .take(5)
   723	                        .cloned()
   724	                        .collect::<Vec<_>>()
   725	                        .join("; "),
   726	                );
   727	            }
   728	
   729	            let deletions = apply_mirror_deletions(
   730	                &plan_outcome.source_paths,
   731	                dest_root,
   732	                &options.filter,
   733	                options.delete_scope,
   734	                !options.dry_run,
   735	                options.verbose,
   736	            )?;
   737	            summary.deleted_files = deletions.0;
   738	            summary.deleted_dirs = deletions.1;
   739	        }
   740	
   741	        if let Some(tracker) = journal_tracker.as_mut() {
   742	            persist_journal_checkpoints(tracker, journal_tokens.as_mut_slice(), options.verbose);
   743	        }
   744	
   745	        if options.verbose {
   746	            eprintln!(
   747	                "Planning enumerated {} file(s), {} bytes",
   748	                scanned_files, scanned_bytes
   749	            );
   750	            eprintln!(

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_history.rs | sed -n '560,770p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   560	    fn read_records_skips_invalid_lines() {
   561	        let dir = tempfile::tempdir().expect("tempdir");
   562	        let path = dir.path().join("test_history.jsonl");
   563	
   564	        let mut file = File::create(&path).expect("create");
   565	        writeln!(file, "{}", sample_v0_json()).expect("write v0");
   566	        writeln!(file, "{{not valid json}}").expect("write garbage");
   567	        writeln!(file).expect("write empty");
   568	        writeln!(file, "{}", sample_v1_json()).expect("write v1");
   569	        drop(file);
   570	
   571	        let records = read_records_from_path(&path, 0).expect("read");
   572	        assert_eq!(records.len(), 2, "should skip invalid/empty lines");
   573	    }
   574	
   575	    #[test]
   576	    fn new_record_has_current_version() {
   577	        let options = OptionSnapshot {
   578	            dry_run: false,
   579	            preserve_symlinks: true,
   580	            include_symlinks: false,
   581	            skip_unchanged: true,
   582	            checksum: false,
   583	            compare_mode: CompareModeSnapshot::SizeMtime,
   584	            workers: 4,
   585	        };
   586	        let record = PerformanceRecord::new(
   587	            TransferMode::Copy,
   588	            None,
   589	            None,
   590	            1,
   591	            100,
   592	            options,
   593	            None,
   594	            10,
   595	            20,
   596	            0,
   597	            0,
   598	        );
   599	        assert_eq!(record.schema_version, CURRENT_SCHEMA_VERSION);
   600	    }
   601	
   602	    #[test]
   603	    fn read_records_respects_limit() {
   604	        let dir = tempfile::tempdir().expect("tempdir");
   605	        let path = dir.path().join("test_history.jsonl");
   606	
   607	        let mut file = File::create(&path).expect("create");
   608	        for _ in 0..5 {
   609	            writeln!(file, "{}", sample_v0_json()).expect("write");
   610	        }
   611	        drop(file);
   612	
   613	        let records = read_records_from_path(&path, 2).expect("read");
   614	        assert_eq!(records.len(), 2, "should return only the last 2 records");
   615	    }
   616	
   617	    // ── R56-F1: run_kind lane + migration ──────────────────────────────
   618	
   619	    /// Pre-v2 records carried lane in `options.dry_run` and
   620	    /// `fast_path == Some("null_sink")`. Migration must derive the
   621	    /// lane without collapsing `mode` — an old mirror record stays
   622	    /// mirror.
   623	    #[test]
   624	    fn migration_v1_real_copy_record_lands_in_real_lane() {
   625	        let record: PerformanceRecord =
   626	            serde_json::from_str(sample_v0_json()).expect("deserialize v0");
   627	        let migrated = migrate_record(record);
   628	        assert_eq!(migrated.mode, TransferMode::Copy);
   629	        assert_eq!(
   630	            migrated.run_kind,
   631	            RunKind::Real,
   632	            "real copy record should land in Real lane"
   633	        );
   634	    }
   635	
   636	    /// GPT explicit ask: "old mirror record migrates without
   637	    /// becoming copy."
   638	    #[test]
   639	    fn migration_v1_mirror_record_preserves_mirror_mode_and_real_lane() {
   640	        let record: PerformanceRecord =
   641	            serde_json::from_str(sample_v1_json()).expect("deserialize v1");
   642	        let migrated = migrate_record(record);
   643	        assert_eq!(
   644	            migrated.mode,
   645	            TransferMode::Mirror,
   646	            "mirror must NOT be collapsed to Copy by migration"
   647	        );
   648	        assert_eq!(
   649	            migrated.run_kind,
   650	            RunKind::Real,
   651	            "non-dry-run mirror record should land in Real lane"
   652	        );
   653	    }
   654	
   655	    #[test]
   656	    fn migration_dry_run_record_lands_in_dryrun_lane() {
   657	        // Old v1 record with options.dry_run = true and no
   658	        // explicit run_kind field on the wire.
   659	        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":true,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":null,"planner_duration_ms":5,"transfer_duration_ms":0,"stall_events":0,"error_count":0}"#;
   660	        let record: PerformanceRecord = serde_json::from_str(json).expect("deserialize v1 dry-run");
   661	        let migrated = migrate_record(record);
   662	        assert_eq!(
   663	            migrated.run_kind,
   664	            RunKind::DryRun,
   665	            "options.dry_run=true must migrate to DryRun lane"
   666	        );
   667	        assert_eq!(migrated.mode, TransferMode::Copy);
   668	    }
   669	
   670	    #[test]
   671	    fn migration_null_sink_record_lands_in_nullsink_lane() {
   672	        // Old v1 record with fast_path = "null_sink".
   673	        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":"null_sink","planner_duration_ms":5,"transfer_duration_ms":2,"stall_events":0,"error_count":0}"#;
   674	        let record: PerformanceRecord =
   675	            serde_json::from_str(json).expect("deserialize v1 null-sink");
   676	        let migrated = migrate_record(record);
   677	        assert_eq!(
   678	            migrated.run_kind,
   679	            RunKind::NullSink,
   680	            "fast_path=null_sink must migrate to NullSink lane"
   681	        );
   682	    }
   683	
   684	    /// New records via the constructor pick up the lane from
   685	    /// `options.dry_run` and `fast_path` so callers don't have to
   686	    /// thread a new parameter through every existing path.
   687	    #[test]
   688	    fn new_record_with_dry_run_options_picks_dryrun_lane() {
   689	        let options = OptionSnapshot {
   690	            dry_run: true,
   691	            preserve_symlinks: true,
   692	            include_symlinks: false,
   693	            skip_unchanged: true,
   694	            checksum: false,
   695	            compare_mode: CompareModeSnapshot::SizeMtime,
   696	            workers: 4,
   697	        };
   698	        let record = PerformanceRecord::new(
   699	            TransferMode::Mirror,
   700	            None,
   701	            None,
   702	            10,
   703	            1024,
   704	            options,
   705	            None,
   706	            5,
   707	            0,
   708	            0,
   709	            0,
   710	        );
   711	        assert_eq!(record.run_kind, RunKind::DryRun);
   712	        assert_eq!(record.mode, TransferMode::Mirror);
   713	    }
   714	
   715	    #[test]
   716	    fn new_record_with_null_sink_fast_path_picks_nullsink_lane() {
   717	        let options = OptionSnapshot {
   718	            dry_run: false,
   719	            preserve_symlinks: true,
   720	            include_symlinks: false,
   721	            skip_unchanged: true,
   722	            checksum: false,
   723	            compare_mode: CompareModeSnapshot::SizeMtime,
   724	            workers: 4,
   725	        };
   726	        let record = PerformanceRecord::new(
   727	            TransferMode::Copy,
   728	            None,
   729	            None,
   730	            10,
   731	            1024,
   732	            options,
   733	            Some("null_sink".to_string()),
   734	            5,
   735	            2,
   736	            0,
   737	            0,
   738	        );
   739	        assert_eq!(record.run_kind, RunKind::NullSink);
   740	    }
   741	
   742	    #[test]
   743	    fn new_record_default_is_real() {
   744	        let options = OptionSnapshot {
   745	            dry_run: false,
   746	            preserve_symlinks: true,
   747	            include_symlinks: false,
   748	            skip_unchanged: true,
   749	            checksum: false,
   750	            compare_mode: CompareModeSnapshot::SizeMtime,
   751	            workers: 4,
   752	        };
   753	        let record = PerformanceRecord::new(
   754	            TransferMode::Copy,
   755	            None,
   756	            None,
   757	            10,
   758	            1024,
   759	            options,
   760	            None,
   761	            5,
   762	            10,
   763	            0,
   764	            0,
   765	        );
   766	        assert_eq!(record.run_kind, RunKind::Real);
   767	        assert!(record.run_kind.is_real_transfer());
   768	    }
   769	
   770	    /// The eligibility helper is the actual chokepoint other modules

codex
I found one likely internal contradiction: the plan wants byte-identical stderr for failing transfers, but also says recorder failures should be `log::warn!`ed. In this binary, warn logs are intentionally sent to stderr, so that warning would itself break byte identity in the recorder-failure case.
exec
/usr/bin/zsh -lc 'rg -n "Local transfer error telemetry|errors_local|record_failure|error telemetry|audit-17|audit-18|error_count|recents.jsonl|ONE_TRANSFER_PATH" docs/DECISIONS.md docs/STATE.md docs/plan/ONE_TRANSFER_PATH.md docs/plan/LOCAL_ERROR_TELEMETRY.md TODO.md crates/blit-daemon/src crates/blit-core/src crates/blit-cli/src' in /home/michael/dev/Blit
 succeeded in 0ms:
TODO.md:129:- [ ] **audit-17** Local `copy` aborts the entire transfer on one
TODO.md:173:- [ ] **audit-18** Non-UTF-8 source filenames are silently corrupted
TODO.md:177:      audit-17, later re-run) failed ~377k entries in at
TODO.md:207:      audit-17 (one bad filename kills the entire run instead of
TODO.md:209:      behavior gets designed for audit-17 should likely cover this
docs/plan/LOCAL_ERROR_TELEMETRY.md:1:# Local transfer error telemetry (design)
docs/plan/LOCAL_ERROR_TELEMETRY.md:8:**Held, not queued**: `docs/STATE.md`'s Queue is pinned to ONE_TRANSFER_PATH
docs/plan/LOCAL_ERROR_TELEMETRY.md:11:**not** entered in the Queue, **not** built — until ONE_TRANSFER_PATH ships
docs/plan/LOCAL_ERROR_TELEMETRY.md:17:The owner hit the same hard-failure crash (`audit-17` — a destination
docs/plan/LOCAL_ERROR_TELEMETRY.md:26:has an `error_count` field, but every writer hardcodes it to `0`
docs/plan/LOCAL_ERROR_TELEMETRY.md:45:- **Does not fix `audit-17`/`audit-18` themselves.** Those stay separate
docs/plan/LOCAL_ERROR_TELEMETRY.md:53:- **Does not unify with the daemon's `recents.jsonl`** (`blit-daemon/src/
docs/plan/LOCAL_ERROR_TELEMETRY.md:61:  etc.) — that's `audit-17`'s decision, not this plan's.
docs/plan/LOCAL_ERROR_TELEMETRY.md:109:shape (`FailureRecord` struct, `record_failure(...)`, `read_failures(limit)`,
docs/plan/LOCAL_ERROR_TELEMETRY.md:110:`clear_failures()`), writing to `errors_local.jsonl` in the same
docs/plan/LOCAL_ERROR_TELEMETRY.md:135:`?`-ing it immediately, call `error_history::record_failure(...)` when it's
docs/plan/LOCAL_ERROR_TELEMETRY.md:146:1. **`error_history` module** — schema, `record_failure`/`read_failures`/
docs/plan/LOCAL_ERROR_TELEMETRY.md:152:   `record_failure` on `Err` before propagating, unchanged exit
docs/plan/LOCAL_ERROR_TELEMETRY.md:163:`audit-17`'s eventual skip-and-report behavior, where a partially-successful
docs/plan/LOCAL_ERROR_TELEMETRY.md:165:follow-up once `audit-17` itself is designed, not this plan's job.
docs/plan/LOCAL_ERROR_TELEMETRY.md:169:- **Q1**: A new dedicated `errors_local.jsonl`, or extend `perf_local.jsonl`
docs/plan/LOCAL_ERROR_TELEMETRY.md:170:  to carry failure rows (finally populating the dead `error_count` field)?
docs/plan/LOCAL_ERROR_TELEMETRY.md:175:  precedent of `recents.jsonl` being its own file rather than folded into
docs/plan/LOCAL_ERROR_TELEMETRY.md:187:  `recents.jsonl` permanently? Agent rec: leave remote alone permanently;
docs/plan/LOCAL_ERROR_TELEMETRY.md:195:  choice this session, not until ONE_TRANSFER_PATH ships or the owner
docs/plan/LOCAL_ERROR_TELEMETRY.md:204:  wiring `record_failure` in) — the whole point is that recording is
docs/DECISIONS.md:130:- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
docs/DECISIONS.md:132:- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.
docs/DECISIONS.md:135:- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
docs/DECISIONS.md:136:- Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
docs/DECISIONS.md:137:- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).
docs/DECISIONS.md:140:- Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
docs/DECISIONS.md:144:## D-2026-07-05-4 — ONE_TRANSFER_PATH flipped Draft → Active
docs/DECISIONS.md:145:- Decision: `docs/plan/ONE_TRANSFER_PATH.md` is **Active** (owner: "flip the plan and go", 2026-07-05). Slice execution starts at otp-1 (wire+session contract, doc + proto, no behavior). The owner re-affirmed the per-slice codex review loop in the same message ("reviewloop codex for each slice") — already binding via D-2026-07-04-1; recorded here as an explicit re-affirmation. All in-plan gates stand: converge-up baseline pins (otp-2), deletion proof + DelegatedPull no-payload-bytes assertion (otp-10), symmetric-rig acceptance (otp-12), owner checklist walk (otp-13). Standing constraints in force: D-2026-07-05-2 (same-build only), zoey activity restricted to the blit-temp test folder with the zero-copy test pre-authorized there (STATE queue item 5).
docs/STATE.md:7:owner answers its Q1–Q3 and flips it to Active. ONE_TRANSFER_PATH otp-1..6
docs/STATE.md:9:`audit-17`/`audit-18` to `TODO.md` only — no code, no Queue change
docs/STATE.md:20:- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive,
docs/STATE.md:58:  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
docs/STATE.md:69:1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
docs/STATE.md:84:   resumes/re-derives after ONE_TRANSFER_PATH ships.
docs/STATE.md:100:   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
docs/STATE.md:107:- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
docs/STATE.md:178:  filed as `audit-17`/`audit-18` (TODO.md only, D-2026-07-05-4); no
docs/STATE.md:180:  one bad filename mid-copy. `audit-17` (`5628c03`): dest FAT-family fs
docs/STATE.md:182:  invalid-filename handling exists. `audit-18` (`deb3800`): a non-UTF-8
docs/STATE.md:189:  ONE_TRANSFER_PATH — pure bug triage.
crates/blit-daemon/src/active_jobs.rs:370:    /// rewrites `recents.jsonl`. A unit `()` signal (not the record)
crates/blit-daemon/src/active_jobs.rs:762:    /// affects the recents ring and its dedicated `recents.jsonl`
crates/blit-daemon/src/active_jobs.rs:774:        // and atomically rewrites recents.jsonl empty. Non-blocking, and
crates/blit-daemon/src/active_jobs.rs:1519:        let path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/active_jobs.rs:1536:        let path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/active_jobs.rs:1566:        let path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/active_jobs.rs:1607:        let recents_path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/active_jobs.rs:1634:            "recents.jsonl rewritten empty by the writer"
crates/blit-daemon/src/recents_store.rs:9://! own file, `recents.jsonl`, alongside but entirely separate from
crates/blit-daemon/src/recents_store.rs:12://! `recents.jsonl` only — the planner's telemetry is never touched.
crates/blit-daemon/src/recents_store.rs:29:const RECENTS_FILE: &str = "recents.jsonl";
crates/blit-daemon/src/recents_store.rs:31:/// Absolute path to the recents store — `config_dir()/recents.jsonl`.
crates/blit-daemon/src/recents_store.rs:118:        let path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/recents_store.rs:125:        let path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/recents_store.rs:136:        let path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/recents_store.rs:147:        let path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/recents_store.rs:158:        let path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/recents_store.rs:166:        let path = dir.path().join("recents.jsonl");
crates/blit-core/src/transfer_session/mod.rs:2://! (docs/plan/ONE_TRANSFER_PATH.md, D-2026-07-05-1).
crates/blit-core/src/transfer_session/data_plane.rs:8://! (`remote::push::client`) are per-direction drivers ONE_TRANSFER_PATH
crates/blit-daemon/src/service/transfer_session_e2e.rs:1://! ONE_TRANSFER_PATH otp-4a/4b loopback e2e: the daemon serves the
crates/blit-daemon/src/service/transfer.rs:1://! ONE_TRANSFER_PATH unified `Transfer` session — daemon side.
crates/blit-daemon/src/service/core.rs:353:    /// ONE_TRANSFER_PATH otp-4a: the daemon serves the unified session
crates/blit-daemon/src/service/core.rs:1155:    /// persistence writer to rewrite `recents.jsonl` empty. Deliberately
crates/blit-core/src/perf_predictor.rs:660:            error_count: 0,
crates/blit-core/src/perf_history.rs:157:    pub error_count: u32,
crates/blit-core/src/perf_history.rs:189:        error_count: u32,
crates/blit-core/src/perf_history.rs:221:            error_count,
crates/blit-core/src/perf_history.rs:500:        r#"{"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":10,"total_bytes":1024,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":true,"checksum":false,"workers":4},"fast_path":null,"planner_duration_ms":50,"transfer_duration_ms":200,"stall_events":0,"error_count":0}"#
crates/blit-core/src/perf_history.rs:504:        r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"mirror","source_fs":"apfs","dest_fs":"apfs","file_count":5,"total_bytes":512,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":true,"workers":2},"fast_path":"tiny","planner_duration_ms":10,"transfer_duration_ms":100,"stall_events":0,"error_count":0,"tar_shard_tasks":1,"tar_shard_files":5,"tar_shard_bytes":512,"raw_bundle_tasks":0,"raw_bundle_files":0,"raw_bundle_bytes":0,"large_tasks":0,"large_bytes":0}"#
crates/blit-core/src/perf_history.rs:659:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":true,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":null,"planner_duration_ms":5,"transfer_duration_ms":0,"stall_events":0,"error_count":0}"#;
crates/blit-core/src/perf_history.rs:673:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":"null_sink","planner_duration_ms":5,"transfer_duration_ms":2,"stall_events":0,"error_count":0}"#;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/stderr_log.rs | sed -n '1,180p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Stderr backend for the `log` facade, shared by every workspace binary.
     2	//!
     3	//! The `log` crate's default logger is a no-op: until a binary installs a
     4	//! backend, every `log::warn!` / `log::error!` in the workspace — including
     5	//! security-degradation warnings and best-effort metadata failures whose
     6	//! *only* surface is the warn — is formatted and discarded (w5-1,
     7	//! errors-log-facade-has-no-backend). Each binary calls [`init`] first
     8	//! thing in `main`.
     9	//!
    10	//! Output follows the one workspace stderr convention (w5-1,
    11	//! errors-stderr-prefix-babel): `<binary>: <level>: <message>`.
    12	
    13	use log::{Level, LevelFilter, Log, Metadata, Record};
    14	use std::sync::OnceLock;
    15	
    16	struct StderrLogger {
    17	    binary: &'static str,
    18	}
    19	
    20	impl Log for StderrLogger {
    21	    fn enabled(&self, metadata: &Metadata) -> bool {
    22	        metadata.level() <= log::max_level()
    23	    }
    24	
    25	    fn log(&self, record: &Record) {
    26	        if self.enabled(record.metadata()) {
    27	            eprintln!(
    28	                "{}: {}: {}",
    29	                self.binary,
    30	                level_str(record.level()),
    31	                record.args()
    32	            );
    33	        }
    34	    }
    35	
    36	    fn flush(&self) {}
    37	}
    38	
    39	fn level_str(level: Level) -> &'static str {
    40	    match level {
    41	        Level::Error => "error",
    42	        Level::Warn => "warn",
    43	        Level::Info => "info",
    44	        Level::Debug => "debug",
    45	        Level::Trace => "trace",
    46	    }
    47	}
    48	
    49	/// Resolve the max level from a `BLIT_LOG` value (`off|error|warn|info|
    50	/// debug|trace`, case-insensitive). Unset or unparseable → warn.
    51	fn level_from_env(value: Option<&str>) -> LevelFilter {
    52	    value
    53	        .and_then(|v| v.trim().parse().ok())
    54	        .unwrap_or(LevelFilter::Warn)
    55	}
    56	
    57	static LOGGER: OnceLock<StderrLogger> = OnceLock::new();
    58	
    59	/// Install the stderr logger for `binary` (e.g. `"blit"`, `"blitd"`).
    60	/// Default max level is warn; `BLIT_LOG` overrides. Idempotent: only the
    61	/// first successful install in a process takes effect.
    62	pub fn init(binary: &'static str) {
    63	    let logger = LOGGER.get_or_init(|| StderrLogger { binary });
    64	    if log::set_logger(logger).is_ok() {
    65	        log::set_max_level(level_from_env(std::env::var("BLIT_LOG").ok().as_deref()));
    66	    }
    67	}
    68	
    69	#[cfg(test)]
    70	mod tests {
    71	    use super::*;
    72	
    73	    #[test]
    74	    fn levels_render_lowercase_for_the_prefix_convention() {
    75	        assert_eq!(level_str(Level::Error), "error");
    76	        assert_eq!(level_str(Level::Warn), "warn");
    77	        assert_eq!(level_str(Level::Info), "info");
    78	        assert_eq!(level_str(Level::Debug), "debug");
    79	        assert_eq!(level_str(Level::Trace), "trace");
    80	    }
    81	
    82	    #[test]
    83	    fn unset_or_garbage_blit_log_defaults_to_warn() {
    84	        assert_eq!(level_from_env(None), LevelFilter::Warn);
    85	        assert_eq!(level_from_env(Some("")), LevelFilter::Warn);
    86	        assert_eq!(level_from_env(Some("loud")), LevelFilter::Warn);
    87	    }
    88	
    89	    #[test]
    90	    fn blit_log_overrides_are_parsed_case_insensitively() {
    91	        assert_eq!(level_from_env(Some("debug")), LevelFilter::Debug);
    92	        assert_eq!(level_from_env(Some("ERROR")), LevelFilter::Error);
    93	        assert_eq!(level_from_env(Some(" off ")), LevelFilter::Off);
    94	        assert_eq!(level_from_env(Some("Trace")), LevelFilter::Trace);
    95	    }
    96	}

exec
/usr/bin/zsh -lc 'rg -n "pub fn clear_history|clear_history|set_perf_history_enabled|perf_history_enabled|fn enforce_size_cap" crates/blit-core/src/perf_history.rs' in /home/michael/dev/Blit
 succeeded in 0ms:
240:    if !perf_history_enabled()? {
367:    #[serde(default = "default_perf_history_enabled")]
368:    perf_history_enabled: bool,
374:            perf_history_enabled: true,
379:fn default_perf_history_enabled() -> bool {
422:pub fn perf_history_enabled() -> Result<bool> {
423:    Ok(load_settings()?.perf_history_enabled)
427:pub fn set_perf_history_enabled(enabled: bool) -> Result<()> {
429:    settings.perf_history_enabled = enabled;
435:pub fn clear_history() -> Result<bool> {
446:fn enforce_size_cap(path: &Path, max_bytes: u64) -> Result<()> {

exec
/usr/bin/zsh -lc 'rg -n "recents|Recent|recent" crates/blit-daemon/src crates/blit-cli/src/jobs.rs crates/blit-app/src | head -120' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-cli/src/jobs.rs:35:    let state = jobs::query(&remote, args.recent_limit).await?;
crates/blit-cli/src/jobs.rs:146:///                              of the recent ring)
crates/blit-cli/src/jobs.rs:156:///    - Already in recent[] → drop stream, emit terminal,
crates/blit-cli/src/jobs.rs:204:    // c-7: ask for replay_recent so any TransferProgress
crates/blit-cli/src/jobs.rs:238:                     and rotated out of the recent ring, or never existed)",
crates/blit-cli/src/jobs.rs:624:    let recent: Vec<_> = state
crates/blit-cli/src/jobs.rs:625:        .recent
crates/blit-cli/src/jobs.rs:669:        "recent": recent,
crates/blit-cli/src/jobs.rs:718:    if state.recent.is_empty() {
crates/blit-cli/src/jobs.rs:719:        println!("Recent: (none)");
crates/blit-cli/src/jobs.rs:723:        println!("Recent ({}):", state.recent.len());
crates/blit-cli/src/jobs.rs:724:        for r in state.recent.iter().rev() {
crates/blit-daemon/src/main.rs:5:mod recents_store;
crates/blit-daemon/src/main.rs:118:    // rec-1: hydrate the recent-runs ring from disk and arm
crates/blit-daemon/src/main.rs:119:    // write-through persistence before serving, so `GetState.recent[]`
crates/blit-daemon/src/main.rs:121:    // an empty ring (handled in `recents_store::load`); only path
crates/blit-daemon/src/main.rs:124:    let _recents_writer = active_jobs::spawn_recents_writer(service.active_jobs.arm_persistence()?);
crates/blit-app/src/profile.rs:60:/// `0` means "all records" per `read_recent_records`'s contract.
crates/blit-app/src/profile.rs:63:    let records = perf_history::read_recent_records(limit)?;
crates/blit-daemon/src/active_jobs.rs:24://!   via recents-history rehydration (see
crates/blit-daemon/src/active_jobs.rs:26://! - `b-3-recent-ring`: bounded recent-runs ring of
crates/blit-daemon/src/active_jobs.rs:32://!   configurable via `ActiveJobs::with_recent_limit`.
crates/blit-daemon/src/active_jobs.rs:34://!   `recent()`. No active_jobs changes — wire-layer only.
crates/blit-daemon/src/active_jobs.rs:50://!   `GetState.recent[].bytes`. The sink type lives in
crates/blit-daemon/src/active_jobs.rs:102:/// Default depth of the recent-runs ring buffer. Mirrors the
crates/blit-daemon/src/active_jobs.rs:103:/// `GetStateRequest.recent_limit = 0 → 50` default the design
crates/blit-daemon/src/active_jobs.rs:109:/// most recent N events emitted for that transfer so a c-5b
crates/blit-daemon/src/active_jobs.rs:110:/// `Subscribe(replay_recent=true, transfer_id_filter=X)` can
crates/blit-daemon/src/active_jobs.rs:172:    /// of that kind survive only in recents-history rehydration,
crates/blit-daemon/src/active_jobs.rs:242:/// One entry in the recent-runs ring buffer. Fields mirror
crates/blit-daemon/src/active_jobs.rs:244:/// `GetState.recent[]` in §6.3 of the TUI design doc. Missing
crates/blit-daemon/src/active_jobs.rs:250:/// `recent()` tests in this module exercise them so the
crates/blit-daemon/src/active_jobs.rs:331:    /// c-4: byte counter value observed by the most recent
crates/blit-daemon/src/active_jobs.rs:336:    /// c-4: unix-ms timestamp of the most recent
crates/blit-daemon/src/active_jobs.rs:354:    /// oldest-first; callers reading `GetState.recent[]` will
crates/blit-daemon/src/active_jobs.rs:356:    recent: Mutex<VecDeque<TransferRecord>>,
crates/blit-daemon/src/active_jobs.rs:357:    /// Maximum number of entries kept in `recent`. Sized at
crates/blit-daemon/src/active_jobs.rs:360:    recent_limit: usize,
crates/blit-daemon/src/active_jobs.rs:370:    /// rewrites `recents.jsonl`. A unit `()` signal (not the record)
crates/blit-daemon/src/active_jobs.rs:378:    /// Construct a registry with the default recent-runs
crates/blit-daemon/src/active_jobs.rs:381:        Self::with_recent_limit(DEFAULT_RECENT_LIMIT)
crates/blit-daemon/src/active_jobs.rs:384:    /// Construct a registry with a custom recent-runs ring
crates/blit-daemon/src/active_jobs.rs:388:    /// `GetState.GetStateRequest.recent_limit` plumbing.
crates/blit-daemon/src/active_jobs.rs:390:    pub fn with_recent_limit(limit: usize) -> Self {
crates/blit-daemon/src/active_jobs.rs:394:                recent: Mutex::new(VecDeque::with_capacity(limit)),
crates/blit-daemon/src/active_jobs.rs:395:                recent_limit: limit,
crates/blit-daemon/src/active_jobs.rs:743:    /// Snapshot of the recent-runs ring, oldest first. Will
crates/blit-daemon/src/active_jobs.rs:744:    /// be consumed by `GetState.recent[]` once the RPC
crates/blit-daemon/src/active_jobs.rs:747:    pub fn recent(&self) -> Vec<TransferRecord> {
crates/blit-daemon/src/active_jobs.rs:749:            .recent
crates/blit-daemon/src/active_jobs.rs:757:    /// rec-2: clear the recent-runs ring and, if persistence is armed,
crates/blit-daemon/src/active_jobs.rs:762:    /// affects the recents ring and its dedicated `recents.jsonl`
crates/blit-daemon/src/active_jobs.rs:765:    /// [`crate::recents_store`] and [`blit_core::perf_history`]).
crates/blit-daemon/src/active_jobs.rs:766:    pub fn clear_recent(&self) -> usize {
crates/blit-daemon/src/active_jobs.rs:768:            let mut ring = self.inner.recent.lock().unwrap_or_else(|e| e.into_inner());
crates/blit-daemon/src/active_jobs.rs:774:        // and atomically rewrites recents.jsonl empty. Non-blocking, and
crates/blit-daemon/src/active_jobs.rs:782:    /// rec-1: hydrate the ring from the on-disk recents store and arm
crates/blit-daemon/src/active_jobs.rs:784:    /// [`crate::recents_store::recents_path`]. Called once at daemon
crates/blit-daemon/src/active_jobs.rs:786:    /// already reflects pre-restart recents.
crates/blit-daemon/src/active_jobs.rs:788:    /// Returns the [`RecentsWriter`] the caller must hand to
crates/blit-daemon/src/active_jobs.rs:789:    /// [`spawn_recents_writer`]; until it runs, ring appends still
crates/blit-daemon/src/active_jobs.rs:792:    pub fn arm_persistence(&self) -> eyre::Result<RecentsWriter> {
crates/blit-daemon/src/active_jobs.rs:793:        Ok(self.arm_persistence_at(crate::recents_store::recents_path()?))
crates/blit-daemon/src/active_jobs.rs:799:    pub fn arm_persistence_at(&self, path: std::path::PathBuf) -> RecentsWriter {
crates/blit-daemon/src/active_jobs.rs:803:        let loaded = crate::recents_store::load(&path, self.inner.recent_limit);
crates/blit-daemon/src/active_jobs.rs:805:            let mut ring = self.inner.recent.lock().unwrap_or_else(|e| e.into_inner());
crates/blit-daemon/src/active_jobs.rs:814:        RecentsWriter {
crates/blit-daemon/src/active_jobs.rs:822:/// rec-1: the write-through half of recents persistence, produced by
crates/blit-daemon/src/active_jobs.rs:824:/// receiver and a handle back to the ring; [`spawn_recents_writer`]
crates/blit-daemon/src/active_jobs.rs:825:/// drives its [`RecentsWriter::run`] loop on the runtime for the
crates/blit-daemon/src/active_jobs.rs:827:pub struct RecentsWriter {
crates/blit-daemon/src/active_jobs.rs:833:impl RecentsWriter {
crates/blit-daemon/src/active_jobs.rs:834:    /// Drain persistence signals and rewrite the recents store. Each
crates/blit-daemon/src/active_jobs.rs:845:            let records = self.active_jobs.recent();
crates/blit-daemon/src/active_jobs.rs:846:            if let Err(e) = crate::recents_store::write_atomic(&self.path, &records) {
crates/blit-daemon/src/active_jobs.rs:847:                log::warn!("failed to persist recents to {:?}: {e}", self.path);
crates/blit-daemon/src/active_jobs.rs:853:/// rec-1: spawn the recents writer on the runtime. The handle is owned
crates/blit-daemon/src/active_jobs.rs:855:/// final unpersisted recents are immaterial). Mirrors
crates/blit-daemon/src/active_jobs.rs:857:pub fn spawn_recents_writer(writer: RecentsWriter) -> tokio::task::JoinHandle<()> {
crates/blit-daemon/src/active_jobs.rs:871:/// [`TransferRecord`] onto the registry's recent-runs ring.
crates/blit-daemon/src/active_jobs.rs:876:/// `GetState.recent[]` rely on.
crates/blit-daemon/src/active_jobs.rs:992:    /// on the next snapshot, and in `GetState.recent[].bytes`
crates/blit-daemon/src/active_jobs.rs:1011:    /// branch overrides) the most recent value is used. In
crates/blit-daemon/src/active_jobs.rs:1036:        // Lock order: table → recent. Held sequentially (no
crates/blit-daemon/src/active_jobs.rs:1050:            if self.inner.recent_limit > 0 {
crates/blit-daemon/src/active_jobs.rs:1059:                push_recent(&self.inner.recent, record, self.inner.recent_limit);
crates/blit-daemon/src/active_jobs.rs:1065:                // we ignore: a recents write lost at shutdown is
crates/blit-daemon/src/active_jobs.rs:1096:fn push_recent(recent: &Mutex<VecDeque<TransferRecord>>, record: TransferRecord, limit: usize) {
crates/blit-daemon/src/active_jobs.rs:1097:    let mut buf = recent.lock().unwrap_or_else(|e| e.into_inner());
crates/blit-daemon/src/active_jobs.rs:1377:    async fn drop_with_recorded_outcome_pushes_to_recent() {
crates/blit-daemon/src/active_jobs.rs:1388:        let recent = table.recent();
crates/blit-daemon/src/active_jobs.rs:1389:        assert_eq!(recent.len(), 1);
crates/blit-daemon/src/active_jobs.rs:1390:        let r = &recent[0];
crates/blit-daemon/src/active_jobs.rs:1415:        let recent = table.recent();
crates/blit-daemon/src/active_jobs.rs:1416:        assert_eq!(recent.len(), 1);
crates/blit-daemon/src/active_jobs.rs:1417:        assert!(!recent[0].ok);
crates/blit-daemon/src/active_jobs.rs:1418:        assert_eq!(recent[0].error_message, "module not found");
crates/blit-daemon/src/active_jobs.rs:1437:        let recent = table.recent();
crates/blit-daemon/src/active_jobs.rs:1438:        assert_eq!(recent.len(), 1);
crates/blit-daemon/src/active_jobs.rs:1439:        assert!(!recent[0].ok);
crates/blit-daemon/src/active_jobs.rs:1440:        assert_eq!(recent[0].error_message, "cancelled before outcome recorded");
crates/blit-daemon/src/active_jobs.rs:1444:    async fn recent_ring_bounded_evicts_oldest() {
crates/blit-daemon/src/active_jobs.rs:1445:        let table = ActiveJobs::with_recent_limit(3);
crates/blit-daemon/src/active_jobs.rs:1456:        let recent = table.recent();
crates/blit-daemon/src/active_jobs.rs:1457:        assert_eq!(recent.len(), 3);
crates/blit-daemon/src/active_jobs.rs:1459:        assert_eq!(recent[0].peer, "peer2");
crates/blit-daemon/src/active_jobs.rs:1460:        assert_eq!(recent[1].peer, "peer3");
crates/blit-daemon/src/active_jobs.rs:1461:        assert_eq!(recent[2].peer, "peer4");
crates/blit-daemon/src/active_jobs.rs:1465:    async fn recent_ring_zero_limit_disables_history() {
crates/blit-daemon/src/active_jobs.rs:1466:        let table = ActiveJobs::with_recent_limit(0);
crates/blit-daemon/src/active_jobs.rs:1478:        assert!(table.recent().is_empty());
crates/blit-daemon/src/active_jobs.rs:1499:    /// Poll the recents file until it holds at least `want` records or
crates/blit-daemon/src/active_jobs.rs:1502:    async fn poll_recents(path: &std::path::Path, want: usize) -> Vec<TransferRecord> {
crates/blit-daemon/src/active_jobs.rs:1504:            let loaded = crate::recents_store::load(path, DEFAULT_RECENT_LIMIT);
crates/blit-daemon/src/active_jobs.rs:1510:        crate::recents_store::load(path, DEFAULT_RECENT_LIMIT)
crates/blit-daemon/src/active_jobs.rs:1515:    /// `GetState.recent[]` reflects pre-restart runs.
crates/blit-daemon/src/active_jobs.rs:1519:        let path = dir.path().join("recents.jsonl");
crates/blit-daemon/src/active_jobs.rs:1520:        crate::recents_store::write_atomic(&path, &[persisted_record("a"), persisted_record("b")])
crates/blit-daemon/src/active_jobs.rs:1523:        let table = ActiveJobs::with_recent_limit(DEFAULT_RECENT_LIMIT);
crates/blit-daemon/src/active_jobs.rs:1524:        assert!(table.recent().is_empty(), "ring starts empty pre-arm");

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/active_jobs.rs | sed -n '1018,1070p'" in /home/michael/dev/Blit
 succeeded in 0ms:
  1018	    pub fn record_outcome(&self, ok: bool, error_message: Option<String>) {
  1019	        let mut cell = self.outcome.lock().unwrap_or_else(|e| e.into_inner());
  1020	        *cell = Some(RecordedOutcome {
  1021	            ok,
  1022	            error_message: error_message.unwrap_or_default(),
  1023	        });
  1024	    }
  1025	}
  1026	
  1027	impl Drop for ActiveJobGuard {
  1028	    fn drop(&mut self) {
  1029	        // Synchronous remove-and-record. PoisonError still
  1030	        // hands us the inner guards via `into_inner`, so the
  1031	        // active row and the ring are updated even if a panic
  1032	        // poisoned a mutex on the way in. This matches the
  1033	        // rest of the codebase's stance on poisoning — surface
  1034	        // the failure, but don't leak state.
  1035	        //
  1036	        // Lock order: table → recent. Held sequentially (no
  1037	        // nested acquisitions). `cancel(id)` takes only the
  1038	        // table lock, so it can't deadlock against this Drop
  1039	        // path.
  1040	        let id = std::mem::take(&mut self.transfer_id);
  1041	        let outcome = {
  1042	            let mut cell = self.outcome.lock().unwrap_or_else(|e| e.into_inner());
  1043	            cell.take()
  1044	        };
  1045	        let entry = {
  1046	            let mut table = self.inner.table.lock().unwrap_or_else(|e| e.into_inner());
  1047	            table.remove(&id)
  1048	        };
  1049	        if let Some(entry) = entry {
  1050	            if self.inner.recent_limit > 0 {
  1051	                // Final byte count: load before the entry's
  1052	                // Arc<AtomicU64> goes out of scope. The
  1053	                // ActiveJobGuard's clone is still alive (we're
  1054	                // inside its Drop), but reading off the entry
  1055	                // is equivalent and keeps the lookup paired
  1056	                // with the row being drained.
  1057	                let bytes = entry.bytes_counter.load(Ordering::Relaxed);
  1058	                let record = build_record(entry.job, outcome, bytes);
  1059	                push_recent(&self.inner.recent, record, self.inner.recent_limit);
  1060	                // rec-1: nudge the persistence writer (if armed). Drop
  1061	                // is synchronous and on the runtime, so we must not do
  1062	                // file I/O here — an unbounded-channel send is
  1063	                // non-blocking and never awaits. `send` only errs if
  1064	                // the writer task is gone (daemon shutting down), which
  1065	                // we ignore: a recents write lost at shutdown is
  1066	                // immaterial.
  1067	                if let Some(tx) = self.inner.persist_tx.get() {
  1068	                    let _ = tx.send(());
  1069	                }
  1070	            }

exec
/usr/bin/zsh -lc "nl -ba crates/blit-daemon/src/recents_store.rs | sed -n '1,80p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! Durable backing store for the `GetState.recent[]` ring.
     2	//!
     3	//! `rec-1` of the recent-persistence feature. The daemon's recent-runs
     4	//! ring ([`crate::active_jobs::ActiveJobs`]) is otherwise in-memory only
     5	//! (lost on restart). This module persists it to a dedicated JSONL file
     6	//! so the TUI's F2 recent list survives daemon restarts.
     7	//!
     8	//! **Separation from planner telemetry.** The recents store lives in its
     9	//! own file, `recents.jsonl`, alongside but entirely separate from
    10	//! `perf_local.jsonl` (the predictor's training data in
    11	//! [`blit_core::perf_history`]). A future `ClearRecent` (rec-2) wipes
    12	//! `recents.jsonl` only — the planner's telemetry is never touched.
    13	//!
    14	//! **Format & durability.** One [`TransferRecord`] per line, oldest
    15	//! first (the ring's own order). The file is rewritten in full on each
    16	//! update — the ring is bounded ([`crate::active_jobs::DEFAULT_RECENT_LIMIT`]),
    17	//! so the file stays small — via a temp-file + atomic rename so a crash
    18	//! mid-write can never leave a torn file. Loading is tolerant: a missing
    19	//! file yields an empty ring, and an unparseable line is skipped rather
    20	//! than failing the daemon (a hand-edited or partially-migrated file
    21	//! must never prevent startup).
    22	
    23	use crate::active_jobs::TransferRecord;
    24	use std::io::Write;
    25	use std::path::{Path, PathBuf};
    26	
    27	/// Filename of the recents store, inside [`blit_core::config::config_dir`]
    28	/// next to `perf_local.jsonl`.
    29	const RECENTS_FILE: &str = "recents.jsonl";
    30	
    31	/// Absolute path to the recents store — `config_dir()/recents.jsonl`.
    32	/// Same directory as the planner's `perf_local.jsonl`, separate file.
    33	pub fn recents_path() -> eyre::Result<PathBuf> {
    34	    Ok(blit_core::config::config_dir()?.join(RECENTS_FILE))
    35	}
    36	
    37	/// Load up to `limit` most-recent records from `path`, oldest first.
    38	///
    39	/// Tolerant by design: a missing file (or empty file) yields an empty
    40	/// `Vec`, and any line that fails to parse as a [`TransferRecord`] is
    41	/// skipped. Never returns an error for a malformed store — recents are
    42	/// informational and must not block daemon startup. Only the last
    43	/// `limit` records are kept (matching the ring's eviction policy), so a
    44	/// file that grew beyond `limit` from an older build is trimmed on load.
    45	pub fn load(path: &Path, limit: usize) -> Vec<TransferRecord> {
    46	    let contents = match std::fs::read_to_string(path) {
    47	        Ok(c) => c,
    48	        // Missing file is the common first-run case; any other read
    49	        // error (permissions, etc.) degrades to "no recents" rather
    50	        // than failing startup.
    51	        Err(_) => return Vec::new(),
    52	    };
    53	    let mut records: Vec<TransferRecord> = contents
    54	        .lines()
    55	        .filter(|line| !line.trim().is_empty())
    56	        .filter_map(|line| serde_json::from_str::<TransferRecord>(line).ok())
    57	        .collect();
    58	    if limit == 0 {
    59	        return Vec::new();
    60	    }
    61	    if records.len() > limit {
    62	        // Keep the newest `limit` (records are oldest-first).
    63	        records.drain(0..records.len() - limit);
    64	    }
    65	    records
    66	}
    67	
    68	/// Atomically rewrite `path` with `records` (oldest first), one JSON
    69	/// object per line. Writes to a sibling temp file then renames over the
    70	/// target, so a reader (or a crash) never observes a partially-written
    71	/// store. Creates the parent directory if needed.
    72	pub fn write_atomic(path: &Path, records: &[TransferRecord]) -> std::io::Result<()> {
    73	    if let Some(parent) = path.parent() {
    74	        std::fs::create_dir_all(parent)?;
    75	    }
    76	    let tmp = path.with_extension("jsonl.tmp");
    77	    {
    78	        let mut file = std::fs::File::create(&tmp)?;
    79	        for record in records {
    80	            // `to_vec` can only fail on a non-string map key or a

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/perf_history.rs | sed -n '420,470p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   420	
   421	/// Returns whether performance history is currently enabled.
   422	pub fn perf_history_enabled() -> Result<bool> {
   423	    Ok(load_settings()?.perf_history_enabled)
   424	}
   425	
   426	/// Persist the performance history enablement flag.
   427	pub fn set_perf_history_enabled(enabled: bool) -> Result<()> {
   428	    let mut settings = load_settings().unwrap_or_default();
   429	    settings.perf_history_enabled = enabled;
   430	    store_settings(&settings)
   431	}
   432	
   433	/// Remove the stored performance history file. Returns `Ok(true)` if the file
   434	/// was removed, `Ok(false)` if it did not exist.
   435	pub fn clear_history() -> Result<bool> {
   436	    let path = history_path()?;
   437	    match fs::remove_file(&path) {
   438	        Ok(_) => Ok(true),
   439	        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
   440	        Err(err) => Err(err.into()),
   441	    }
   442	}
   443	
   444	/// Best-effort rotation that prefers keeping the newest records over enforcing the cap exactly.
   445	/// If a concurrent writer appends while we're trimming, we skip rotation to avoid data loss.
   446	fn enforce_size_cap(path: &Path, max_bytes: u64) -> Result<()> {
   447	    let metadata = match fs::metadata(path) {
   448	        Ok(meta) => meta,
   449	        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
   450	        Err(err) => return Err(err.into()),
   451	    };
   452	
   453	    if metadata.len() <= max_bytes {
   454	        return Ok(());
   455	    }
   456	
   457	    // Capture the size we observed so we can detect concurrent appends.
   458	    let observed_len = metadata.len();
   459	
   460	    let file = File::open(path)?;
   461	    let reader = BufReader::new(file);
   462	    let mut lines: VecDeque<String> = reader
   463	        .lines()
   464	        .collect::<std::result::Result<Vec<String>, _>>()
   465	        .context("read performance history for rotation")?
   466	        .into_iter()
   467	        .filter(|line| !line.trim().is_empty())
   468	        .collect();
   469	
   470	    if lines.is_empty() {

exec
/usr/bin/zsh -lc "nl -ba crates/blit-cli/src/jobs.rs | sed -n '30,50p;710,735p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    30	}
    31	
    32	async fn run_jobs_list(args: JobsListArgs) -> Result<()> {
    33	    let remote = RemoteEndpoint::parse(&args.remote)
    34	        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    35	    let state = jobs::query(&remote, args.recent_limit).await?;
    36	
    37	    if args.json {
    38	        print_json(&state)?;
    39	    } else {
    40	        print_human(&remote, &state);
    41	    }
    42	    Ok(())
    43	}
    44	
    45	async fn run_jobs_cancel(args: JobsCancelArgs) -> Result<ExitCode> {
    46	    let remote = RemoteEndpoint::parse(&args.remote)
    47	        .with_context(|| format!("parsing remote endpoint '{}'", args.remote))?;
    48	    let outcome = jobs::cancel(&remote, &args.transfer_id).await?;
    49	    if args.json {
    50	        print_cancel_json(&outcome);
   710	                module_path(&a.module, &a.path),
   711	                a.peer,
   712	                format_ms(age_ms),
   713	            );
   714	        }
   715	    }
   716	
   717	    println!();
   718	    if state.recent.is_empty() {
   719	        println!("Recent: (none)");
   720	    } else {
   721	        // Display newest-first for human eyes — the wire is
   722	        // oldest-first, so iterate in reverse.
   723	        println!("Recent ({}):", state.recent.len());
   724	        for r in state.recent.iter().rev() {
   725	            let status = if r.ok {
   726	                "ok".to_string()
   727	            } else {
   728	                format!("FAILED: {}", r.error_message)
   729	            };
   730	            println!(
   731	                "  {}  {}  {}  peer={}  duration={}  {}",
   732	                r.transfer_id,
   733	                jobs::kind_label(r.kind),
   734	                module_path(&r.module, &r.path),
   735	                r.peer,

codex
The success-path claim is substantively right for failures: every production call is after a successful branch or after `single_file_copy_inner` returns `Ok`; pipeline/mirror-delete errors return before the recorder. The wording still has a stale function name issue if it calls this `run_local_mirror`.
exec
/usr/bin/zsh -lc 'rg -n "run_local_mirror|execute_local_mirror|TransferOrchestrator|pub fn execute_local|LocalMirrorSummary" crates/blit-core/src crates/blit-app/src crates/blit-cli/src' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-app/src/transfers/local.rs:19:use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOrchestrator};
crates/blit-app/src/transfers/local.rs:40:) -> Result<LocalMirrorSummary> {
crates/blit-app/src/transfers/local.rs:45:        TransferOrchestrator::new()
crates/blit-app/src/transfers/local.rs:46:            .execute_local_mirror(&src, &dst, options)
crates/blit-core/src/engine/mod.rs:9://! [`TransferEngine::execute`]; `TransferOrchestrator` is the local
crates/blit-core/src/engine/mod.rs:41:pub use summary::{LocalMirrorSummary, TransferOutcome};
crates/blit-core/src/engine/mod.rs:99:    /// `TransferOrchestrator::execute_local_mirror_async` at
crates/blit-core/src/engine/mod.rs:101:    pub async fn execute(&self, request: EngineRequest) -> Result<LocalMirrorSummary> {
crates/blit-core/src/engine/mod.rs:213:            let summary = LocalMirrorSummary {
crates/blit-core/src/engine/mod.rs:268:                    let summary = LocalMirrorSummary {
crates/blit-core/src/engine/mod.rs:303:                    let summary = LocalMirrorSummary {
crates/blit-core/src/engine/mod.rs:337:                    let summary = LocalMirrorSummary {
crates/blit-core/src/engine/mod.rs:582:        // pipeline-wrote-bytes contract (see `LocalMirrorSummary`
crates/blit-core/src/engine/mod.rs:680:        let mut summary = LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:16:use super::summary::{LocalMirrorSummary, TransferOutcome};
crates/blit-core/src/engine/single_file.rs:34:) -> Result<LocalMirrorSummary> {
crates/blit-core/src/engine/single_file.rs:65:) -> Result<LocalMirrorSummary> {
crates/blit-core/src/engine/single_file.rs:107:        return Ok(LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:123:        return Ok(LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:136:        return Ok(LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:149:        return Ok(LocalMirrorSummary {
crates/blit-core/src/engine/single_file.rs:193:    Ok(LocalMirrorSummary {
crates/blit-cli/src/transfers/local.rs:4:use blit_core::orchestrator::{LocalMirrorOptions, LocalMirrorSummary, TransferOutcome};
crates/blit-cli/src/transfers/local.rs:21:) -> Result<LocalMirrorSummary> {
crates/blit-cli/src/transfers/local.rs:36:) -> Result<LocalMirrorSummary> {
crates/blit-cli/src/transfers/local.rs:50:    summary: &LocalMirrorSummary,
crates/blit-cli/src/transfers/local.rs:80:) -> Result<LocalMirrorSummary> {
crates/blit-cli/src/transfers/local.rs:214:    summary: &LocalMirrorSummary,
crates/blit-cli/src/transfers/local.rs:320:    summary: &LocalMirrorSummary,
crates/blit-core/src/engine/strategy.rs:37:    /// fast-path scan. Propagated into `LocalMirrorSummary.
crates/blit-core/src/engine/history.rs:6:use super::{LocalMirrorOptions, LocalMirrorSummary};
crates/blit-core/src/engine/history.rs:18:    summary: &LocalMirrorSummary,
crates/blit-core/src/engine/history.rs:50:    summary: &LocalMirrorSummary,
crates/blit-core/src/engine/history.rs:150:        let summary = LocalMirrorSummary {
crates/blit-core/src/engine/history.rs:160:            ..LocalMirrorSummary::default()
crates/blit-core/src/engine/history.rs:183:        let summary = LocalMirrorSummary {
crates/blit-core/src/engine/history.rs:196:            ..LocalMirrorSummary::default()
crates/blit-core/src/engine/summary.rs:75:pub struct LocalMirrorSummary {
crates/blit-core/src/orchestrator/mod.rs:4:    LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions, LocalMirrorSummary,
crates/blit-core/src/orchestrator/mod.rs:7:pub use orchestrator::TransferOrchestrator;
crates/blit-core/src/orchestrator/orchestrator.rs:11:use super::{LocalMirrorOptions, LocalMirrorSummary};
crates/blit-core/src/orchestrator/orchestrator.rs:21:pub struct TransferOrchestrator;
crates/blit-core/src/orchestrator/orchestrator.rs:23:impl TransferOrchestrator {
crates/blit-core/src/orchestrator/orchestrator.rs:28:    /// Sync wrapper around [`execute_local_mirror_async`]. Builds a
crates/blit-core/src/orchestrator/orchestrator.rs:31:    /// inside an async runtime must use `execute_local_mirror_async`
crates/blit-core/src/orchestrator/orchestrator.rs:36:    /// [`execute_local_mirror_async`]: Self::execute_local_mirror_async
crates/blit-core/src/orchestrator/orchestrator.rs:37:    pub fn execute_local_mirror(
crates/blit-core/src/orchestrator/orchestrator.rs:42:    ) -> Result<LocalMirrorSummary> {
crates/blit-core/src/orchestrator/orchestrator.rs:49:        runtime.block_on(self.execute_local_mirror_async(src_root, dest_root, options))
crates/blit-core/src/orchestrator/orchestrator.rs:55:    pub async fn execute_local_mirror_async(
crates/blit-core/src/orchestrator/orchestrator.rs:60:    ) -> Result<LocalMirrorSummary> {
crates/blit-core/src/orchestrator/orchestrator.rs:127:impl Default for TransferOrchestrator {
crates/blit-core/src/orchestrator/orchestrator.rs:135:    //! F9 regression: `execute_local_mirror_async` must be callable
crates/blit-core/src/orchestrator/orchestrator.rs:137:    //! sync `execute_local_mirror` wrapper builds its own runtime
crates/blit-core/src/orchestrator/orchestrator.rs:169:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:171:            .execute_local_mirror_async(&src, &dst, opts())
crates/blit-core/src/orchestrator/orchestrator.rs:190:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:191:        let summary = orch.execute_local_mirror(&src, &dst, opts()).unwrap();
crates/blit-core/src/orchestrator/orchestrator.rs:229:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:231:            .execute_local_mirror_async(&src, &dst, run_opts.clone())
crates/blit-core/src/orchestrator/orchestrator.rs:247:            .execute_local_mirror_async(&src, &dst, run_opts)
crates/blit-core/src/orchestrator/orchestrator.rs:321:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:322:        let result = orch.execute_local_mirror_async(&src, &dst, opts).await;
crates/blit-core/src/orchestrator/orchestrator.rs:389:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:390:        let result = orch.execute_local_mirror_async(&src, &dst, opts).await;
crates/blit-core/src/orchestrator/orchestrator.rs:425:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:427:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:463:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:465:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:494:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:496:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:522:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:524:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:560:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:562:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:599:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:601:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:645:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:647:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:678:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:680:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:705:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:707:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:746:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:748:            .execute_local_mirror_async(&src, &dst, opts)
crates/blit-core/src/orchestrator/orchestrator.rs:788:        let orch = TransferOrchestrator::new();
crates/blit-core/src/orchestrator/orchestrator.rs:790:            .execute_local_mirror_async(&src, &dst, opts)

exec
/usr/bin/zsh -lc "rg -n \"record_performance_history\\(\" crates/blit-core/src crates/blit-app/src crates/blit-cli/src crates/blit-daemon/src" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/engine/mod.rs:220:            if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:277:                    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:314:                    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:350:                    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:792:        if let Some(record) = record_performance_history(
crates/blit-core/src/engine/single_file.rs:42:    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/history.rs:17:pub(super) fn record_performance_history(

exec
/usr/bin/zsh -lc "rg -n \"append_local_record|perf_history::append|record_performance_history|PerformanceRecord::new\\(\" crates/blit-core/src crates/blit-cli/src crates/blit-app/src crates/blit-daemon/src" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/engine/mod.rs:62:use self::history::{record_performance_history, update_predictor};
crates/blit-core/src/engine/mod.rs:220:            if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:277:                    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:314:                    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:350:                    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/mod.rs:563:        // BEFORE observe() (record_performance_history →
crates/blit-core/src/engine/mod.rs:568:        // here; `record_performance_history` populates the matching
crates/blit-core/src/engine/mod.rs:792:        if let Some(record) = record_performance_history(
crates/blit-core/src/engine/single_file.rs:14:use super::history::{record_performance_history, update_predictor};
crates/blit-core/src/engine/single_file.rs:42:    if let Some(record) = record_performance_history(
crates/blit-core/src/engine/tuning.rs:117:        let mut r = PerformanceRecord::new(
crates/blit-core/src/perf_predictor.rs:1130:        let mut record = PerformanceRecord::new(
crates/blit-core/src/engine/history.rs:2:    append_local_record, CompareModeSnapshot, OptionSnapshot, PerformanceRecord, TransferMode,
crates/blit-core/src/engine/history.rs:17:pub(super) fn record_performance_history(
crates/blit-core/src/engine/history.rs:36:    if let Err(err) = append_local_record(&record) {
crates/blit-core/src/engine/history.rs:45:/// touching disk. Split out from `record_performance_history` so
crates/blit-core/src/engine/history.rs:87:    let mut record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:239:pub fn append_local_record(record: &PerformanceRecord) -> Result<()> {
crates/blit-core/src/perf_history.rs:586:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:698:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:726:        let record = PerformanceRecord::new(
crates/blit-core/src/perf_history.rs:753:        let record = PerformanceRecord::new(
crates/blit-core/src/auto_tune/mod.rs:94:        let mut record = PerformanceRecord::new(

exec
/usr/bin/zsh -lc 'rg -n "error_count" crates/blit-core/src crates/blit-cli/src crates/blit-app/src crates/blit-daemon/src' in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-core/src/perf_predictor.rs:660:            error_count: 0,
crates/blit-core/src/perf_history.rs:157:    pub error_count: u32,
crates/blit-core/src/perf_history.rs:189:        error_count: u32,
crates/blit-core/src/perf_history.rs:221:            error_count,
crates/blit-core/src/perf_history.rs:500:        r#"{"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":10,"total_bytes":1024,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":true,"checksum":false,"workers":4},"fast_path":null,"planner_duration_ms":50,"transfer_duration_ms":200,"stall_events":0,"error_count":0}"#
crates/blit-core/src/perf_history.rs:504:        r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"mirror","source_fs":"apfs","dest_fs":"apfs","file_count":5,"total_bytes":512,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":true,"workers":2},"fast_path":"tiny","planner_duration_ms":10,"transfer_duration_ms":100,"stall_events":0,"error_count":0,"tar_shard_tasks":1,"tar_shard_files":5,"tar_shard_bytes":512,"raw_bundle_tasks":0,"raw_bundle_files":0,"raw_bundle_bytes":0,"large_tasks":0,"large_bytes":0}"#
crates/blit-core/src/perf_history.rs:659:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":true,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":null,"planner_duration_ms":5,"transfer_duration_ms":0,"stall_events":0,"error_count":0}"#;
crates/blit-core/src/perf_history.rs:673:        let json = r#"{"schema_version":1,"timestamp_epoch_ms":1700000000000,"mode":"copy","source_fs":null,"dest_fs":null,"file_count":3,"total_bytes":100,"options":{"dry_run":false,"preserve_symlinks":true,"include_symlinks":false,"skip_unchanged":false,"checksum":false,"workers":1},"fast_path":"null_sink","planner_duration_ms":5,"transfer_duration_ms":2,"stall_events":0,"error_count":0}"#;

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/orchestrator/orchestrator.rs | sed -n '20,75p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    20	/// the pre-engine orchestrator.
    21	pub struct TransferOrchestrator;
    22	
    23	impl TransferOrchestrator {
    24	    pub fn new() -> Self {
    25	        Self
    26	    }
    27	
    28	    /// Sync wrapper around [`execute_local_mirror_async`]. Builds a
    29	    /// new multi-thread Tokio runtime and blocks on it. Use this from
    30	    /// non-async callers (CLI commands, tests). Callers already
    31	    /// inside an async runtime must use `execute_local_mirror_async`
    32	    /// directly -- calling this from inside a Tokio context will
    33	    /// panic at `Runtime::new` (closes F9 of
    34	    /// `docs/reviews/codebase_review_2026-05-01.md`).
    35	    ///
    36	    /// [`execute_local_mirror_async`]: Self::execute_local_mirror_async
    37	    pub fn execute_local_mirror(
    38	        &self,
    39	        src_root: &Path,
    40	        dest_root: &Path,
    41	        options: LocalMirrorOptions,
    42	    ) -> Result<LocalMirrorSummary> {
    43	        let workers = options.workers.max(1);
    44	        let runtime = Builder::new_multi_thread()
    45	            .worker_threads(workers)
    46	            .enable_all()
    47	            .build()
    48	            .context("build tokio runtime")?;
    49	        runtime.block_on(self.execute_local_mirror_async(src_root, dest_root, options))
    50	    }
    51	
    52	    /// Async local-transfer entry point: validate the local
    53	    /// preconditions, construct the local source/sink pair, and hand
    54	    /// execution to the engine.
    55	    pub async fn execute_local_mirror_async(
    56	        &self,
    57	        src_root: &Path,
    58	        dest_root: &Path,
    59	        options: LocalMirrorOptions,
    60	    ) -> Result<LocalMirrorSummary> {
    61	        if !src_root.exists() {
    62	            return Err(eyre!("source path does not exist: {}", src_root.display()));
    63	        }
    64	
    65	        if !options.dry_run {
    66	            if let Some(parent) = dest_root.parent() {
    67	                std::fs::create_dir_all(parent).with_context(|| {
    68	                    format!("failed to create destination parent {}", parent.display())
    69	                })?;
    70	            }
    71	        }
    72	
    73	        // Local source, wrapped in FilteredSource so the user filter
    74	        // applies through the universal pipeline chokepoint (identical
    75	        // to push/pull/remote-remote behavior -- full parity).

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mod.rs | sed -n '1,90p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	//! The unified transfer engine (`ue-r2-1c`, REV4 Design §1).
     2	//!
     3	//! `TransferEngine` owns transfer execution: strategy selection
     4	//! (`journal_no_work`, `no_work`, `tiny_manifest`, `single_huge_file`,
     5	//! the single-file shortcut, streaming pipeline), the streaming leg
     6	//! (plan tuning -> scan -> diff/plan -> sink pipeline -> mirror
     7	//! deletions), and the perf-history/predictor accounting hooks. Path
     8	//! adapters construct the source, sink, and options, then call
     9	//! [`TransferEngine::execute`]; `TransferOrchestrator` is the local
    10	//! adapter today, and push/pull converge here at `ue-r2-1f`/`1g`.
    11	//! Dial creation and streaming plans arrive with `ue-r2-1d`/`1e`
    12	//! (REV4 "Slice dependencies").
    13	//!
    14	//! The option/summary types keep their `LocalMirror*` names until the
    15	//! remote paths converge -- renaming ahead of those slices would churn
    16	//! every caller twice.
    17	
    18	mod dial;
    19	mod history;
    20	mod journal;
    21	mod mirror;
    22	mod options;
    23	mod single_file;
    24	mod strategy;
    25	mod streaming_plan;
    26	mod summary;
    27	mod tuning;
    28	
    29	pub use dial::{
    30	    initial_stream_proposal, local_receiver_capacity, spawn_dial_tuner,
    31	    spawn_dial_tuner_with_resize, ResizeProposal, SharedStreamProbes, TransferDial,
    32	    DIAL_CEILING_CHUNK_BYTES, DIAL_CEILING_MAX_STREAMS, DIAL_CEILING_PREFETCH,
    33	    DIAL_FLOOR_CHUNK_BYTES, DIAL_FLOOR_PREFETCH, DIAL_STEP_DOWN_BLOCKED_RATIO,
    34	    DIAL_STEP_UP_BLOCKED_RATIO, DIAL_TUNER_TICK, RESIZE_COOLDOWN_TICKS, RESIZE_SUSTAIN_TICKS,
    35	};
    36	pub use options::{LocalCompareMode, LocalMirrorDeleteScope, LocalMirrorOptions};
    37	pub use streaming_plan::{
    38	    InitialPlan, InitialPlanStrategy, PlanUpdate, STREAMING_PLAN_BATCH_HEADERS,
    39	    STREAMING_PLAN_FLUSH_AFTER,
    40	};
    41	pub use summary::{LocalMirrorSummary, TransferOutcome};
    42	
    43	use std::path::PathBuf;
    44	use std::sync::{Arc, Mutex};
    45	use std::time::Instant;
    46	
    47	use eyre::{bail, Context, Result};
    48	use tokio::sync::mpsc;
    49	
    50	use crate::auto_tune::derive_local_plan_tuning;
    51	use crate::change_journal::{ChangeState, ChangeTracker, ProbeToken};
    52	use crate::local_worker::{copy_large_blocking, copy_paths_blocking};
    53	use crate::perf_history::{read_recent_records, TransferMode};
    54	use crate::perf_predictor::PerformancePredictor;
    55	use crate::remote::transfer::payload::{TransferPayload, DEFAULT_PAYLOAD_PREFETCH};
    56	use crate::remote::transfer::pipeline::execute_sink_pipeline_streaming;
    57	use crate::remote::transfer::sink::TransferSink;
    58	use crate::remote::transfer::source::TransferSource;
    59	use crate::transfer_plan::PlanOptions;
    60	use crate::CopyConfig;
    61	
    62	use self::history::{record_performance_history, update_predictor};
    63	use self::journal::{log_probe, persist_journal_checkpoints};
    64	use self::mirror::apply_mirror_deletions;
    65	use self::single_file::execute_single_file_copy;
    66	use self::strategy::{maybe_select_fast_path, FastPathDecision};
    67	use self::streaming_plan::{run_streaming_plan, StreamingPlanInputs};
    68	use self::tuning::select_tuning_window_from_history;
    69	
    70	/// Everything the engine needs to run one transfer. The adapter owns
    71	/// path-specific construction (REV4 Design §1): it resolves roots,
    72	/// builds the (already filter-wrapped) source and the sink, translates
    73	/// its option surface, and hands over execution.
    74	pub struct EngineRequest {
    75	    pub src_root: PathBuf,
    76	    pub dest_root: PathBuf,
    77	    /// Filter-wrapped source; used by the streaming strategy's scan.
    78	    pub source: Arc<dyn TransferSource>,
    79	    /// Destination sink for the streaming strategy (`FsTransferSink`
    80	    /// or `NullSink` locally). Fast-path strategies use their own
    81	    /// blocking executors, exactly as before the engine existed.
    82	    pub sink: Arc<dyn TransferSink>,
    83	    pub options: LocalMirrorOptions,
    84	}
    85	
    86	/// The unified transfer engine. Stateless today (all state is
    87	/// per-execute); the live dial (`ue-r2-1e`) is the first field that
    88	/// will change that.
    89	pub struct TransferEngine;
    90	

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/mod.rs | sed -n '370,430p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   370	
   371	            if options.verbose {
   372	                eprintln!(
   373	                    "Completed local {} via fast-path: {} file(s), {} bytes in {:.2?}",
   374	                    if options.mirror { "mirror" } else { "copy" },
   375	                    summary.copied_files,
   376	                    summary.total_bytes,
   377	                    summary.duration
   378	                );
   379	            }
   380	
   381	            return Ok(summary);
   382	        }
   383	
   384	        // --- Unified pipeline: same path as remote transfers ---
   385	        let mut plan_options = PlanOptions {
   386	            force_tar: options.force_tar,
   387	            ..PlanOptions::default()
   388	        };
   389	
   390	        // ue-r2-1d: the Design §3 novel/known split, made explicit.
   391	        // Known = cross-run telemetry existed for this workload shape
   392	        // (the tuning window produced records; plan targets derive from
   393	        // them). Novel = no telemetry -> conservative defaults. Both
   394	        // start immediately and refine live; neither probes.
   395	        let mut initial_strategy = InitialPlanStrategy::Novel;
   396	        if options.perf_history {
   397	            // R57-F1: read ALL history, not a pre-cap window. The
   398	            // R56-F2 fix correctly filtered run_kind before the
   399	            // 20-record cap inside `select_tuning_window`, but the
   400	            // caller was still pre-capping at 50 records from the
   401	            // JSONL — so 50 recent non-real records could still
   402	            // hide older real records one layer up. The file is
   403	            // already size-capped at ~1 MiB upstream
   404	            // (DEFAULT_MAX_BYTES in perf_history.rs), so reading
   405	            // all records is bounded; `read_recent_records(0)`
   406	            // means "all" per its limit semantics.
   407	            let target_mode = if options.mirror {
   408	                TransferMode::Mirror
   409	            } else {
   410	                TransferMode::Copy
   411	            };
   412	            // R59 finding #5: tuning window keys on full compare_mode,
   413	            // not just options.checksum. Translate via the same enum
   414	            // the history snapshot uses so the bucket lookup matches
   415	            // what the writer recorded.
   416	            let query_compare_mode = options
   417	                .compare_mode
   418	                .resolve_compare_snapshot(options.checksum);
   419	            if let Some(filtered) = select_tuning_window_from_history(
   420	                read_recent_records,
   421	                target_mode,
   422	                query_compare_mode,
   423	                options.skip_unchanged,
   424	            ) {
   425	                initial_strategy = InitialPlanStrategy::Known {
   426	                    window_records: filtered.len(),
   427	                };
   428	                if let Some(tuning) = derive_local_plan_tuning(&filtered) {
   429	                    plan_options.small_target = Some(tuning.small_target_bytes);
   430	                    plan_options.small_count_target = Some(tuning.small_count_target);

exec
/usr/bin/zsh -lc "nl -ba crates/blit-core/src/engine/single_file.rs | sed -n '90,205p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    90	    //
    91	    // Filter: the source root is itself the only entry. Run
    92	    // `filter.allows_entry` against the source name. If excluded,
    93	    // return a "scanned 1 / copied 0" summary so the user sees
    94	    // "no work performed" rather than the file being copied
    95	    // anyway.
    96	    let src_name = src_root.file_name().map(PathBuf::from);
    97	    let allows = match src_name {
    98	        Some(name) => {
    99	            let mtime = src_meta.modified().ok();
   100	            options
   101	                .filter
   102	                .allows_entry(Some(&name), src_root, size, mtime)
   103	        }
   104	        None => true,
   105	    };
   106	    if !allows {
   107	        return Ok(LocalMirrorSummary {
   108	            planned_files: 0,
   109	            copied_files: 0,
   110	            total_bytes: 0,
   111	            scanned_files: 1,
   112	            scanned_bytes: size,
   113	            duration: start_time.elapsed(),
   114	            outcome: TransferOutcome::UpToDate,
   115	            ..Default::default()
   116	        });
   117	    }
   118	
   119	    // ignore_existing: if the destination file already exists,
   120	    // skip the copy entirely. Matches the diff_planner behavior
   121	    // for the streaming-pipeline path (diff_planner.rs).
   122	    if options.ignore_existing && dest_root.exists() {
   123	        return Ok(LocalMirrorSummary {
   124	            planned_files: 0,
   125	            copied_files: 0,
   126	            total_bytes: 0,
   127	            scanned_files: 1,
   128	            scanned_bytes: size,
   129	            duration: start_time.elapsed(),
   130	            outcome: TransferOutcome::UpToDate,
   131	            ..Default::default()
   132	        });
   133	    }
   134	
   135	    if options.dry_run {
   136	        return Ok(LocalMirrorSummary {
   137	            planned_files: 1,
   138	            copied_files: 1,
   139	            total_bytes: size,
   140	            scanned_files: 1,
   141	            scanned_bytes: size,
   142	            dry_run: true,
   143	            duration: start_time.elapsed(),
   144	            ..Default::default()
   145	        });
   146	    }
   147	
   148	    if options.null_sink {
   149	        return Ok(LocalMirrorSummary {
   150	            planned_files: 1,
   151	            copied_files: 1,
   152	            total_bytes: size,
   153	            scanned_files: 1,
   154	            scanned_bytes: size,
   155	            duration: start_time.elapsed(),
   156	            ..Default::default()
   157	        });
   158	    }
   159	
   160	    let mut did_copy = false;
   161	    let mut clone_succeeded = false;
   162	    let mut bytes_copied = 0u64;
   163	
   164	    if options.resume {
   165	        let outcome = resume_copy_file(src_root, dest_root, 0)
   166	            .with_context(|| format!("resume copy {}", src_root.display()))?;
   167	        did_copy = outcome.bytes_transferred > 0;
   168	        bytes_copied = outcome.bytes_transferred;
   169	    } else {
   170	        let needs_copy = !options.skip_unchanged
   171	            || file_needs_copy_with_mode(src_root, dest_root, compare_mode).unwrap_or(true);
   172	        if needs_copy {
   173	            let sizer = BufferSizer::default();
   174	            let logger = NoopLogger;
   175	            let outcome = copy_file(src_root, dest_root, &sizer, false, &logger)
   176	                .with_context(|| format!("copy {}", src_root.display()))?;
   177	            did_copy = true;
   178	            clone_succeeded = outcome.clone_succeeded;
   179	            bytes_copied = outcome.bytes_copied;
   180	        }
   181	    }
   182	
   183	    if options.preserve_times && did_copy && !clone_succeeded {
   184	        if let Ok(modified) = src_meta.modified() {
   185	            let ft = FileTime::from_system_time(modified);
   186	            // R42-F1: warn-don't-silence (was `let _ = ...`).
   187	            if let Err(e) = filetime::set_file_mtime(dest_root, ft) {
   188	                log::warn!("set mtime on {}: {}", dest_root.display(), e);
   189	            }
   190	        }
   191	    }
   192	
   193	    Ok(LocalMirrorSummary {
   194	        planned_files: 1,
   195	        copied_files: if did_copy { 1 } else { 0 },
   196	        total_bytes: bytes_copied,
   197	        // Single-file path always saw exactly one entry of `size`
   198	        // bytes; whether we copied it or not is the
   199	        // copied_files/total_bytes story, but the scan saw it.
   200	        scanned_files: 1,
   201	        scanned_bytes: size,
   202	        duration: start_time.elapsed(),
   203	        outcome: if did_copy {
   204	            TransferOutcome::Transferred
   205	        } else {

codex
There’s also an undefined internal reference: the draft says “see D1” twice, but the document has no D1 section. That is low-risk but worth calling out because the doc is meant to be durable plan text.
exec
/usr/bin/zsh -lc "nl -ba TODO.md | sed -n '120,220p'" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc 'rg -n "D1|## D|Decision|see D1|Open questions" docs/plan/LOCAL_ERROR_TELEMETRY.md' in /home/michael/dev/Blit
 succeeded in 0ms:
6:**Decision ref**: pending (owner review)
97:      existing one (see D1).
106:## Design
112:schema (see D1).
167:## Open questions for the owner

 succeeded in 0ms:
   120	      `engine/strategy.rs:81` (mirror/checksum/force-tar always take the
   121	      streaming path, which is unified with remote push). Fix: gate the
   122	      print in `spawn_manifest_task` behind the caller's verbose option
   123	      (the function has no visibility into CLI args today — thread a flag
   124	      through `LocalMirrorOptions`/the equivalent remote-push options).
   125	      Related, already filed, do not re-file: `--progress` (`-p`)
   126	      auto-enabling on TTY regardless of the flag is
   127	      `docs/audit/findings/drift-principles.md`
   128	      (`drift-spinner-vs-quiet-default-decision-conflict`).
   129	- [ ] **audit-17** Local `copy` aborts the entire transfer on one
   130	      filename the destination filesystem rejects, instead of
   131	      skipping/warning and continuing. Reported: `blit copy
   132	      /home/michael/ /run/media/michael/8247-7E92/michael -ypv`
   133	      failed enumerating ~88k entries in, at
   134	      `crates/blit-core/src/remote/transfer/sink.rs:605`
   135	      (`write_tar_shard_payload`'s parallel-write closure) —
   136	      `std::fs::create_dir_all` on a NuGet http-cache path whose
   137	      final component is `670c1461c...$ps:_api.nuget.org_v3_index.json`
   138	      returned `Invalid argument (os error 22)`. The source filename
   139	      is valid on the source (Linux/ext4) fs but contains a `:`,
   140	      which is illegal on FAT/exFAT/NTFS-strict destinations.
   141	      *Assumption, unverified — drive wasn't mounted to confirm:*
   142	      the destination (`/run/media/...`, hex volume label
   143	      `8247-7E92`, the classic Linux label format for an
   144	      unlabeled FAT-family volume) is exFAT or FAT32. Grepped
   145	      `crates/blit-core/src` for existing invalid-filename handling
   146	      (`os error 22`, `sanitize_name`, `illegal.*char`, etc.) — none
   147	      found; this is a real gap, not a regression. One bad name
   148	      currently kills the whole run rather than being
   149	      skipped/reported/renamed. Fix needs a design call (owner
   150	      input required, `plan` this before coding): per-file
   151	      skip-and-report vs. optional name-sanitization vs. fail-fast
   152	      with a clear top-level error instead of a raw `os error 22` +
   153	      internal path/line. Whatever is chosen must apply uniformly
   154	      to both the local-mirror and remote tar-shard receive paths
   155	      (same `write_tar_shard_payload` helper backs both, per
   156	      audit-16 above). **Confirmed reproducible**: identical failure
   157	      (same path, same error, same line, same ~88129-entry offset)
   158	      recurred verbatim on a second run targeting a different mount
   159	      (`/run/media/michael/USB_DRIVE/michael`) — not a one-off fluke;
   160	      the FAT-family-destination assumption above is now corroborated
   161	      by a second independent mount exhibiting the same `:`-rejection.
   162	      **Second manifestation, same closure, one line down**: after
   163	      that NuGet-cache directory issue was worked past (retried on a
   164	      cleared destination), the identical `os error 22` recurred at
   165	      `sink.rs:608` — this time `std::fs::write` on a regular file
   166	      whose name is `frostfell06.dds:crc` (unrelated content, a game
   167	      asset tree, not NuGet). Confirms the bug is general to any path
   168	      component containing an illegal character, not create_dir_all-
   169	      specific or NuGet-cache-specific — the fix must cover both the
   170	      `create_dir_all` (line 605) and `write` (line 608) call sites in
   171	      the same closure (and their mirror in the local-mirror path, per
   172	      the note above).
   173	- [ ] **audit-18** Non-UTF-8 source filenames are silently corrupted
   174	      during enumeration, then fail to open, aborting the whole
   175	      transfer. Reported: `blit copy /home/michael/
   176	      /run/media/michael/USB_DRIVE/michael -ypv` (same host as
   177	      audit-17, later re-run) failed ~377k entries in at
   178	      `crates/blit-core/src/remote/transfer/payload.rs:360`
   179	      (`build_tar_shard`'s `std::fs::File::open(&full_path)`) with
   180	      `No such file or directory (os error 2)` opening
   181	      `.../Claudia-by-Choice-codex_1.0<U+FFFD>` — the trailing
   182	      replacement character is the tell. Root cause:
   183	      `crate::path_posix::relative_path_to_posix` (`path_posix.rs:36-44`)
   184	      builds the canonical relative-path `String` with
   185	      `c.as_os_str().to_string_lossy()` per component — on Linux/ext4
   186	      a filename can be any non-`/`/non-NUL byte sequence, and a
   187	      non-UTF-8 one gets its invalid bytes replaced with U+FFFD
   188	      *irreversibly*. `build_tar_shard` then does
   189	      `source_root.join(rel)` on that corrupted string, which no
   190	      longer names the real file → `ENOENT`. `FileEntry.path` itself
   191	      (`fs_enum.rs:14`) is a real `PathBuf` (exact bytes preserved),
   192	      so the corruption happens specifically at the `PathBuf → String`
   193	      relative-path conversion, not at enumeration. `relative_path_to_posix`
   194	      is the single canonical helper for this and is called from
   195	      `engine/mirror.rs`, `mirror_planner.rs`, `remote/transfer/{payload,tar_safety}.rs`,
   196	      `remote/endpoint.rs`, and `remote/push/client/helpers.rs` — i.e.
   197	      local mirror and remote push both go through the lossy path, so
   198	      this isn't a remote-only/proto-only constraint. Fix needs a
   199	      design call (owner input required, `plan` this before coding):
   200	      the wire `FileHeader.relative_path`/`FileBlock.relative_path`
   201	      fields are proto3 `string`, which is UTF-8-only at the gRPC
   202	      layer, so a full fix for the remote path needs an encoding
   203	      scheme that round-trips arbitrary bytes through a UTF-8-safe
   204	      string (e.g. percent-encode invalid bytes, or WTF-8) — local
   205	      mirror has no such wire constraint and could preserve raw
   206	      `OsString`/`PathBuf` throughout instead. Same failure class as
   207	      audit-17 (one bad filename kills the entire run instead of
   208	      being skipped/reported) — whatever skip/report/fail-fast
   209	      behavior gets designed for audit-17 should likely cover this
   210	      case too, but the root cause here is enumeration-side path
   211	      corruption, not destination-fs charset rejection, so treat as
   212	      a separate fix even if the error-handling policy ends up shared.
   213	
   214	### Deferred design calls
   215	
   216	These are intentionally not next-actionable. Don't pick them up
   217	without the listed prerequisite — they're tracked here so they
   218	don't get lost, not so the next agent reimplements them on a hunch.
   219	
   220	- [x] **Remote→remote re-evaluation** — resolved by

exec
/usr/bin/zsh -lc 'rg -n "F15|structured-logging|structured logging|TODO" TODO.md docs crates | head -80' in /home/michael/dev/Blit
 succeeded in 0ms:
TODO.md:1:# Blit v2 TODO
TODO.md:19:explicitly-deferred logging epic (F15).
TODO.md:92:      synced. TODO.md (this section) synced.
TODO.md:99:- [ ] **F15 — Low** Adopt `tracing` or structured `log` across daemon
TODO.md:363:- [x] Review logging/error output for production readiness. *(2026-04-07: Audited all crates. Removed duplicate println/eprintln debug output from block clone paths. Added expect() messages to bare unwrap() calls. No dbg!()/todo!() found. Remaining eprintln calls in orchestrator/daemon are intentional verbose output. Full migration to structured logging deferred to post-release.)*
docs/STATE.md:9:`audit-17`/`audit-18` to `TODO.md` only — no code, no Queue change
docs/STATE.md:178:  filed as `audit-17`/`audit-18` (TODO.md only, D-2026-07-05-4); no
docs/DECISIONS.md:47:- Decision: `docs/STATE.md` is the single entry point for current state, with the precedence order in `AGENTS.md` §1; DEVLOG.md is write-only history, TODO.md is backlog-only, tool-local memories are scratch.
docs/DECISIONS.md:48:- Why: state smeared across TODO/DEVLOG/plan-README/Serena was the drift mechanism the 2026-06-04 audit documented (drift-* findings, M28).
docs/audit/DESIGN_MAP_2026-06-11.md:13:them — cross-crate coupling enforced only by TODO comments. The two genuinely
docs/audit/DESIGN_MAP_2026-06-11.md:45:   `is_retryable` needs; the code documents this about itself in TODOs. A dead
docs/audit/DESIGN_MAP_2026-06-11.md:223:No single module owns timeouts & liveness. The closest thing to an owner is crates/blit-core/src/remote/transfer/stall_guard.rs (StallGuard/StallGuardWriter + the owner-decided 30s TRANSFER_STALL_TIMEOUT), but it deliberately covers only raw-TCP data-plane progress, and at least four other strata each grew their own local policy: blit-daemon's net_timeout::within helper, blit-app's client.rs CONNECT_TIMEOUT, the TUI's SUBSCRIBE_OPEN/SNAPSHOT_FETCH constants, and the prometheus bridge's 5/8/10s set — while the two blit-core channel builders carry bare inline 30s literals. The smear is asymmetric in a dangerous way: connects and daemon-side accepts are well bounded (30s/15s, declared four separate times), but once connected, every client-side gRPC await in the workspace is structurally unbounded — no client channel sets HTTP/2 keepalive, TCP keepalive, or a per-RPC deadline, so delegation progress (remote.rs:734), push responses (helpers.rs:245), all three pull fallback loops (pull.rs:330/505/790), and `blit jobs watch` by default will wait forever on a silently-dead daemon; only the daemon server protects itself (main.rs:138). The sharpest single problem is that the planned fix — the audit-h3c slice-2 progress watchdog — exists only as comments plus a pass-through helper (grpc_fallback.rs:150), and its own TODOs record that the surrounding error-conversion sites already strip the error chain the watchdog would need for retries to work. Secondary findings: raw TCP data-plane connects are unbounded (pull.rs:1710, data_plane.rs:92) while control-plane connects are triple-bounded; pull_sync.rs redefines its own accept/token constants twice in one file under a comment claiming reuse; and the two set_keepalive(true) calls run at OS-default ~2h timing while comments claim they prevent idle-stream timeouts.
docs/audit/DESIGN_MAP_2026-06-11.md:228:- audit-h3c slice 2 (the gRPC-fallback progress watchdog) is unimplemented: the TCP path has StallGuard but force-gRPC / fallback transfers have zero stall protection (stall_guard.rs:35-41, grpc_fallback.rs:24). Worse, the TODOs at pull.rs:322-329 and :781-788 record that current map_err sites strip the io::Error chain, so even when the watchdog ships, blit_app::transfers::retry::is_retryable will not classify stalls as retryable until every conversion site is also changed — a cross-file coupling already documented as a future drift trap.
docs/audit/DESIGN_MAP_2026-06-11.md:280:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:330 — UNBOUNDED: pull() gRPC-fallback receive loop via recv_fallback_message — no liveness bound until h3c slice 2. TODO at :322-329: map_err strips the io::Error chain, so a future watchdog TimedOut would not be classified retryable.
docs/audit/DESIGN_MAP_2026-06-11.md:282:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:790 — UNBOUNDED: pull_sync_with_spec control+data fallback receive — the load-bearing site GPT-12/R3 H3 named. Same error-chain-stripping TODO (:781-788).
docs/audit/DESIGN_MAP_2026-06-11.md:434:The retry concept has a nominal owner — blit_app::transfers::retry (is_retryable + run_with_retries), wired to --retry/--wait for exactly three CLI verbs in blit-cli/main.rs:55-69 — but its effectiveness is smeared across strata that don't cooperate. The classifier sees only std::io::Error in the eyre chain, while the gRPC layers (pull.rs, push client helpers, the whole delegated remote-to-remote path, daemon delegated_pull) flatten every tonic::Status into a bare string through at least four independently-written conversion idioms, so on those paths --retry is documented-by-TODO to silently do nothing (pull.rs:322-329, 780-788). A second, dead classifier in blit-core/src/errors.rs contradicts the live one on ConnectionRefused/UnexpectedEof/NotConnected and remains publicly exported. The three seed channel builders diverge on the one axis that matters here: blit-app/client.rs preserves the error chain (connect failures retryable), the core pull/push builders strip it (same failure fatal). Sharpest problem: the combination of unshipped audit-h3c slice 2, no client keepalive, and chain-stripping means the gRPC fallback path — used precisely when networks misbehave — can hang forever, and when it does fail, the user's --retry flag won't fire and nothing says why.
docs/audit/DESIGN_MAP_2026-06-11.md:438:- --retry/--wait is silently inert on the primary gRPC paths. Every tonic::Status->String conversion (pull.rs:307/332/493/507/541, format_status at pull.rs:116 feeding PullSyncError at 792-798, push helpers.rs:48, the entire delegated path in blit-app/remote.rs and daemon delegated_pull.rs:216/245) destroys the io::Error chain that is_retryable (retry.rs:27-33) requires. The code knows: TODOs at pull.rs:322-329 and 780-788 literally say '--retry/--wait will silently stop working on this code path' — a RELIABLE failure mode where the advertised flag does nothing and tells no one. audit-h3c slice 2, which would fix it, is unshipped (stall_guard.rs:35-41).
docs/audit/DESIGN_MAP_2026-06-11.md:444:- Retry ownership sits in the wrong layer: the classifier and loop live in blit-app (retry.rs), but the errors it must classify are minted and stripped in blit-core, which cannot depend on blit-app. The TODO comments in blit-core reference blit_app::transfers::retry::is_retryable by name (pull.rs:325, 786) — cross-crate coupling by comment, with no shared error type enforcing it.
docs/audit/DESIGN_MAP_2026-06-11.md:455:  - divergence: All copies agree in the harmful effect (source chain destroyed -> is_retryable blind) but diverge in what survives: format_status keeps the gRPC code, the inline pull.rs/helpers.rs/admin copies drop it, and blit-app/remote.rs branches on Code::Unavailable/Unimplemented to compose bespoke advice text. At least four independently-written conversion idioms for the same operation; none preserves the chain (e.g. eyre::Report::new(status).wrap_err(...) as the pull.rs:322-329 TODO prescribes).
docs/audit/DESIGN_MAP_2026-06-11.md:490:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:307 — CLASSIFIER-BLIND: pull() control/fallback receive maps tonic::Status -> eyre!(status.message().to_string()) at 307 and 332 — Explicit TODO(audit-h3c-2) at 322-329: this strip means is_retryable will not classify a stall/drop here and --retry/--wait will not fire.
docs/audit/DESIGN_MAP_2026-06-11.md:491:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:493 — CLASSIFIER-BLIND: scan_remote_files (493, 507 with TODO at 502-504) and open_remote_file (541) strip Status to String the same way
docs/audit/DESIGN_MAP_2026-06-11.md:492:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:792 — CLASSIFIER-BLIND: pull_sync_with_spec maps Status into PullSyncError::transfer/negotiation, whose format_status (116-122) flattens to String — TODO(audit-h3c-2) at 780-788 says exactly this: '--retry/--wait will silently stop working on this code path'. This is the PRIMARY pull path.
docs/audit/DESIGN_MAP_2026-06-11.md:514:No module owns error handling; ownership is smeared across all six crates plus the proto. The nominal owner, blit_core::errors (TransferError/ErrorCategory/categorize_io_error, errors.rs:12-121, exported at lib.rs:9), is dead code with zero consumers, while the live retry policy lives in blit-app/src/transfers/retry.rs with an io::ErrorKind table that directly contradicts the dead one on three kinds (ConnectionRefused, UnexpectedEof, NotConnected). The workspace is uniformly eyre (thiserror and anyhow appear nowhere), but at least seven distinct styles coexist: chain-preserving context, chain-destroying eyre! stringification, three hand-rolled typed errors, tonic::Status-as-error with 116/189 constructors collapsed to Status::internal and line-dependent {err} vs {err:#} formatting, sentinel io::Errors for retry signaling, proto-carried string errors, and five different stderr prefixes. The sharpest problem is a self-documented broken contract: stall_guard.rs manufactures downcastable TimedOut errors precisely so --retry/--wait can classify stalls as retryable, but PullSyncError/format_status (pull.rs:82/116) and the dominant eyre!(status.message().to_string()) pattern (15+ sites) flatten errors to strings before the classifier sees them — the TODO at pull.rs:780-788 admits retry silently no-ops on the pull-sync path. Secondary: the same stringification drops the gRPC code, so a code-only Status renders as an empty user-facing error, violating RELIABLE's plain-failure principle.
docs/audit/DESIGN_MAP_2026-06-11.md:518:- --retry/--wait is silently broken on the pull-sync path: stall_guard manufactures io::ErrorKind::TimedOut precisely so retry.rs:27 can downcast it, but PullSyncError (pull.rs:82) and format_status (pull.rs:116) flatten the Status to a String before it reaches the eyre chain. The repo knows: TODO(audit-h3c-2) at pull.rs:780-788 documents the break, unfixed. RELIABLE violation — the feature degrades with no error at all.
docs/audit/DESIGN_MAP_2026-06-11.md:553:- `TRANSFER_STALL_TIMEOUT` = Duration::from_secs(30) (/home/michael/dev/Blit/crates/blit-core/src/remote/transfer/stall_guard.rs:69) — SIMPLE: good — adaptive watchdog re-armed on progress, no tuning knob. RELIABLE tension: its entire purpose (a downcastable TimedOut for --retry) is voided on the pull-sync path by PullSyncError stringification (pull.rs:780 TODO).
docs/audit/DESIGN_MAP_2026-06-11.md:562:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:780 — In-code TODO(audit-h3c-2) admitting the defect: PullSyncError's stringification of Status destroys the io::Error so blit_app::transfers::retry::is_retryable cannot classify stall TimedOut as retryable — `--retry/--wait` silently stops working on the pull-sync path. Acknowledged, not fixed.
docs/audit/DESIGN_MAP_2026-06-11.md:1032:  - crates/blit-core/src/remote/push/client/helpers.rs:48-50 `map_status` → `eyre!(status.message().to_string())` and pull.rs:307,332 same idiom — discards tonic status code/chain (the audit stratum left a TODO about exactly this at pull.rs:322-329)
docs/audit/DESIGN_MAP_2026-06-11.md:1392:- **[abandoned]** The three TODO benchmark captures (remote fallback, 10+ Gbps TCP, TCP-vs-gRPC results recorded into CHANGELOG)
docs/audit/DESIGN_MAP_2026-06-11.md:1556:- **[partial]** Deferred: full structured-logging migration (eprintln → log macros across ~50 sites)
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:292:**Mechanism**: blit_core::errors::categorize_io_error (errors.rs:90-117) marks ConnectionRefused/UnexpectedEof/NotConnected as Fatal (lines 107-113) while the live blit_app::transfers::retry::is_retryable_io_kind (retry.rs:35-46) marks the same three kinds Retryable. errors.rs has zero consumers (rg for categorize_io_error/ErrorCategory/crate::errors across crates/ and tests/ returns only lib.rs:9 `pub mod errors`), yet it is the discoverable, doc-commented module a future contributor would wire up — silently flipping retry semantics. Meanwhile the TODO(audit-h3c-2) blocks at pull.rs:322-329 and 780-788 name `blit_app::transfers::retry::is_retryable` as the contract their future fix must satisfy — cross-crate coupling enforced only by comment, untestable from blit-core because the dependency direction forbids the import. The queued slice-2 chain-preservation work depends on exactly this contract holding.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:298:- /home/michael/dev/Blit/crates/blit-core/src/remote/pull.rs:325 — TODO names blit_app::transfers::retry::is_retryable across the crate boundary by comment; same at :786
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1149:  - kill reason: Mechanism verified (all strip sites exist: e.g. /home/michael/dev/Blit/crates/blit-app/src/admin/rm.rs:30, blit-core/src/remote/pull.rs:307/332/493/507/541, push/client/helpers.rs:48-49; private format_status at pull.rs:116-122; code-preserving outlier at admin/jobs.rs:91-95), but the finding duplicates two already-tracked items. The admin/CLI half is audit finding R3-H12 (AUDIT_REPORT_2026-06-04_R2.md:333-348, kept HIGH at _R3.md:168), whose written remediation is exactly the proposed fix — a shared `status_to_eyre(rpc_name, status)` helper with code preservation used everywhere. The blit-core half is the queued slice-2 "tonic Status → eyre" preservation on the DO-NOT-RE-REPORT list (docs/STATE.md:34), with TODO(audit-h3c-2) at pull.rs:322-329 pinning those exact map_err sites for a chain-preserving conversion that inherently restores the code and fixes the empty-message blank render. The novel residue (empty-message fallback detail, completions.rs:84 site) belongs as annotation to R3-H12/slice-2, not a separate medium finding.
docs/audit/DESIGN_FINDINGS_2026-06-11_PHASE_B.md:1171:Checked and re-derived from code this session: blit-core/src/errors.rs (full read), blit-app/src/transfers/retry.rs (full read), blit-core/src/remote/pull.rs 75-135 (PullSyncError/format_status), push/client/helpers.rs (map_status, [push] prefix), all daemon service files via targeted rg + reads (Status constructor tally: 116 internal / ~199 total; {err} vs {err:#} counts 12 vs ~69), util.rs (full read), delegated_pull.rs 190-260 and 360-375, core.rs outcome_from_status and TransferError event construction, blit-cli/src/jobs.rs watch render paths, blit-app/src/admin/{rm,jobs}.rs, blit-app/src/transfers/remote.rs 698-760, proto/blit.proto error messages, sink.rs log::warn and mpsc sites, logger.rs (full read), workspace-wide rg for log backends (none exist). Found clean: run_with_retries loop logic and its tests (correct budget/classification semantics); format_status itself (the one well-designed converter — finding is that it is private); admin/jobs.rs cancel_job code branching (good pattern, includes code+message); delegated_pull's R37-F1 negotiation-phase preservation via PullSyncError downcast (works as documented); color_eyre installed in blit-cli main; prometheus bridge stderr usage is internally consistent; copy-engine errors propagate correctly despite the noop Logger (verified Err(e) returned at file_copy/mod.rs:215-223). Not covered per instructions: deep TUI internals (only the main.rs:5734 boundary site, kept as light-pass evidence); the queued slice-2 territory — connect-site chain stripping (pull.rs:245, push/client/mod.rs:313), the TODO(audit-h3c-2) retry no-op at pull.rs:780-788, Status→eyre chain preservation, and retry-classifier extension to tonic codes — cross-referenced in findings 4 and 3 rather than re-reported; the three already-filed design-1/2/3 findings. Not exhaustively audited: blit-app TUI-facing transfer progress text, Windows-only win_fs error paths, and the bug-report-style empty-message reachability of every individual tonic transport error variant (mechanism shown via the in-repo format_status guard plus core.rs unwrap_or(\"\") instead).
docs/audit/inventory/plan-tui.md:610:"Live structured logging stream. F15 deferred; if structured logs are wanted in the TUI later, they ride on Subscribe."
docs/audit/inventory/plan-principles.md:730:#### deferred-structured-logging-f15
docs/audit/inventory/plan-principles.md:770:> macOS FSEvents fast-path real-network field-test (`UNVERIFIED` in audit). Windows ReFS `SeManageVolumePrivilege` requirement for block clone (`TODO.md:259`).
docs/audit/inventory/plan-phases.md:628:> "✅ RPC handlers live (2025-10-24); F2 canonical containment + per-call read-only checks added 2026-05-02." / "✅ `crates/blit-utils/src/main.rs` updated; docs + TODO synced (2025-10-24)."
docs/audit/inventory/plan-phases.md:646:> "[x] Audit `RemoteTransferSource` usages. … [x] Update `docs/DAEMON_CONFIG.md`; [x] Update CLI usage docs; [x] Update TODO.md; [x] Add benchmark script."
docs/audit/inventory/plan-phases.md:724:#### deferred-structured-logging
docs/audit/inventory/plan-phases.md:728:> "Full structured logging migration (eprintln → log macros across ~50 sites)"
docs/audit/inventory/plan-perf.md:492:"After all phases, results are in `logs/bench_10gbe_*/`. To document: (1) Copy the best results.csv into CHANGELOG.md benchmark section; (2) Update TODO.md — check off the three benchmark items; (3) Note any issues found (throughput bottlenecks, errors, etc.)."
docs/audit/inventory/plan-perf.md:495:**Source**: BENCHMARK_10GBE_PLAN.md §TODO Items Covered
docs/audit/inventory/plan-perf.md:498:Plan covers three TODO items: Benchmark remote fallback + data-plane streaming (line 78); Benchmark TCP data plane throughput targeting 10+ Gbps (line 98); Capture remote benchmark runs TCP vs gRPC fallback (line 116).
docs/audit/inventory/plan-perf.md:842:| docs/plan/BENCHMARK_10GBE_PLAN.md | 133 | Read in full. All five phases + prerequisites + recording + TODO mapping. |
docs/audit/inventory/code-tui-display.md:149:- **build-delegated-execution** — `crates/blit-tui/src/exec_plan.rs:91-108` — Constructs `DelegatedPullExecution` with `detach: false` always. _(notes: detached/F2-visible delegation is "a follow-up" — TODO marker in doc comment.)_
docs/audit/inventory/code-tui-display.md:190:### TODO/follow-up markers (not formal TODO comments, but explicit follow-ups in comments)
docs/audit/inventory/code-tui-display.md:191:- **detach-follow-up** — `exec_plan.rs:106` — `detach: false` always; "detached/F2-visible delegation is a follow-up". Implicit TODO.
docs/audit/inventory/code-tui-display.md:192:- **a1-3b-getstate-detail** — `screens/f1.rs:341` — Comment says "A follow-up slice (a1-3b-f1-getstate-detail) will populate the modules cell from a loopback `GetState` query when a local daemon is running." Active TODO marker.
docs/audit/inventory/code-tui-display.md:193:- **relay-fallback-suggestable** — `exec_plan.rs:103-104` — "The TUI doesn't surface a `--relay-via-cli` toggle yet, so don't suggest it in transport-error hints." TODO posture.
docs/audit/inventory/code-daemon.md:141:- **stderr-info-messages** — `crates/blit-daemon/src/main.rs:34-72` — Daemon emits `[warn]`, `[info]` messages directly to stderr (e.g. mDNS advertise success, delegation enabled). No structured logging framework.
docs/audit/inventory/code-daemon.md:204:### TODO/FIXME/HACK scan
docs/audit/inventory/code-daemon.md:205:None of the read files contain literal `TODO`, `FIXME`, `XXX`, or `HACK` markers. Instead, the codebase uses inline "in scope for slice X / lands in slice Y / deferred per §N" prose comments (e.g. `active_jobs.rs:55-69`, `service/core.rs:1099-1101`, `service/push/data_plane.rs:602-627`) to defer work. The deferred work is heavily documented but harder to grep for than `TODO`.
docs/audit/inventory/code-core-transfer.md:243:- **No TODO/FIXME/HACK/XXX comments in cluster** — Searched, none found. (Comment-level hygiene is unusually clean.)
docs/audit/inventory/code-core-misc.md:121:- **macos-metadata-mtime-then-perms** — `crates/blit-core/src/fs_capability/macos.rs:36-77` — Sets file mtime via filetime; preserves Unix permissions via PermissionsExt; xattrs hardcoded to `false`. _(notes: xattrs comment "would need xattr crate dependency" — TODO-ish but no explicit TODO marker)_
docs/audit/inventory/code-core-misc.md:151:- **macOS `xattrs` preservation perpetually `false`** — `fs_capability/macos.rs:72-74` says "would need xattr crate dependency" — effectively a TODO without the marker. The `Capabilities::xattrs: true` flag advertises support that `preserve_metadata` doesn't implement.
docs/audit/inventory/code-core-io.md:176:18. **No TODO/FIXME/XXX/HACK markers present** in any cluster file (manually grepped during reading). All annotated issues are documented as audit-NN or R58-FNN inline comments.
docs/audit/inventory/code-cli.md:187:- **No TODO/FIXME/HACK markers and no cfg(windows)/cfg(unix) divergence** in any blit-cli/src file — clean from those classes, but suggests platform-specific quirks (if any) live in blit-core/blit-app dependencies.
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:33:   despite the TODO/CHANGELOG claiming byte-level instrumentation shipped (GPT-15).
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:42:benchmark deferral), and GPT-23 (REVIEW.md / .review/findings / TODO state are
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:158:  fires consistently. TODOs at the three call sites document the
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:254:**Plan**: TODO marks byte-level progress shipped; CHANGELOG §"Daemon" claims live byte
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:767:truth, even though TODO/CHANGELOG say 0.1.0 shipped and Phase 6 is active
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:771:### L4. REVIEW.md, .review/findings/, and TODO.md state are mutually inconsistent
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:775:verification; TODO header says only F15 remains while later unchecked work exists
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:791:checklist, TODO entries
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:967:| 23 | REVIEW.md/findings/TODO unreliable | none | **NEW** → L4 |
docs/audit/AUDIT_REPORT_2026-06-04_R2.md:1073:41. **L4** Sweep REVIEW.md / .review/findings/ / TODO.md for verified/open consistency.
docs/audit/2026-05-04_roadmap_audit.md:12:documents plus `TODO.md`/`DEVLOG.md`/two reviews. State distribution:
docs/audit/2026-05-04_roadmap_audit.md:18:| DEFERRED     | ~7    | Plan explicitly defers (RDMA, AI telemetry, structured logging, TLS, BlitAuth, TUI features, `--detach`). |
docs/audit/2026-05-04_roadmap_audit.md:35:   delegation Phase 3 item. Several benchmark items in `TODO.md` Phase 3
docs/audit/2026-05-04_roadmap_audit.md:38:4. F15 (structured logging) is the single explicitly-deferred baseline
docs/audit/2026-05-04_roadmap_audit.md:63:| Phase 3 Remote Ops done | PARTIAL | Hybrid TCP done (`remote/transfer/data_plane.rs`), gRPC fallback (`remote/pull.rs`, `service/pull.rs`), all admin RPCs (`Find`, `DiskUsage`, `FilesystemStats`, `CompletePath`, `Purge`, `ListModules` — `proto/blit.proto:24-36`). Remote→remote delegation Phases 1+2 shipped (`service/delegated_pull.rs`, `transfers/remote_remote_direct.rs`); CLI relay primitive retained as `--relay-via-cli`. | Phase 3 *still has open TODO items* for "Capture remote benchmark runs (TCP vs forced gRPC fallback)" (`TODO.md:228`) and "Benchmark remote fallback + data-plane streaming…" (`TODO.md:190`). |
docs/audit/2026-05-04_roadmap_audit.md:64:| Phase 4 Production done | PARTIAL | CI workflow at `.github/workflows/ci.yml`. Release scripts: `scripts/build-release.sh`, `scripts/windows/build-release.ps1`. Man pages: `docs/cli/blit.1.md`, `blit-daemon.1.md`, `blit-utils.1.md`. CHANGELOG.md present. | F15 (structured logging) explicitly deferred. AI telemetry implementation deferred. |
docs/audit/2026-05-04_roadmap_audit.md:78:| Windows CopyFileExW, USN Change Journal, ReFS block clone | SHIPPING (block clone gated on privilege) | `copy/windows.rs`; `change_journal/snapshot.rs::windows`; `fs_capability/windows.rs:54+` `probe_block_clone_support`. `win_fs.rs:134+` documents `SeManageVolumePrivilege` requirement. | ReFS benchmark in `WORKFLOW_PHASE_2.5.md:72` shows blit 0.59s vs robocopy 0.165s — block clone path operates but ~3.5× slower than robocopy. Privilege investigation is `TODO.md:259` open item. |
docs/audit/2026-05-04_roadmap_audit.md:87:| ReFS block clone privilege investigation | UNVERIFIED / open | `TODO.md:259` open: "Investigate SeManageVolumePrivilege requirement". `win_fs.rs:134+` has the privilege-enable scaffold but `WORKFLOW_PHASE_2.5.md:72` records the fallback path is ~3.5× slower than robocopy. | Confirmed open. Listed in PROJECT_STATE_ASSESSMENT.md "Post-Release". |
docs/audit/2026-05-04_roadmap_audit.md:89:| Full structured logging migration | DEFERRED (F15) | `TODO.md:99-101` explicitly deferred. Confirmed `eprintln!` and `println!` calls remain across orchestrator/daemon. | |
docs/audit/2026-05-04_roadmap_audit.md:90:| Benchmark TCP data plane throughput targeting 10+ Gbps | NOT-STARTED | `TODO.md:210` unchecked. `BENCHMARK_10GBE_PLAN.md` is the plan. `scripts/bench_10gbe.sh` exists but no captured results. | Hardware-bound. |
docs/audit/2026-05-04_roadmap_audit.md:91:| Benchmark remote fallback + data-plane streaming | NOT-STARTED | `TODO.md:190` unchecked. | Hardware-bound. |
docs/audit/2026-05-04_roadmap_audit.md:92:| Capture remote benchmark runs (TCP vs gRPC fallback) | NOT-STARTED | `TODO.md:228` unchecked. | Hardware-bound. |
docs/audit/2026-05-04_roadmap_audit.md:112:| Benchmarks include remote scenarios over TCP and gRPC fallback | NOT-STARTED | Remote benchmark capture remains a TODO item. |

exec
/usr/bin/zsh -lc 'git show --name-status --format=fuller 284f7f9 -- docs/plan/LOCAL_ERROR_TELEMETRY.md' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 284f7f9e0cd3ccfc8a65a0fe7d040b5e8f1aa060
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Mon Jul 6 19:56:18 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Mon Jul 6 19:56:18 2026 -0400

    Draft LOCAL_ERROR_TELEMETRY.md: local failure-log design (Draft, held)
    
    Owner asked for durable error reporting after hitting audit-17's crash
    three times across two drives and having to copy-paste terminal output
    each time. Designs a new errors_local.jsonl (sibling to perf_local.jsonl,
    not a schema change to it) fed from a single chokepoint in main.rs's
    Copy/Mirror arms, plus a `blit diagnostics errors` read-back verb.
    
    Explicitly held out of docs/STATE.md's Queue per the owner's choice this
    session: D-2026-07-05-4 pins the Queue to ONE_TRANSFER_PATH exclusively,
    so this stays Draft-only (no code) until that gate lifts or the owner
    gives an explicit go.

A	docs/plan/LOCAL_ERROR_TELEMETRY.md

codex
**Findings**
- [High] [docs/plan/LOCAL_ERROR_TELEMETRY.md:128](/home/michael/dev/Blit/docs/plan/LOCAL_ERROR_TELEMETRY.md:128) — The proposed `main.rs` chokepoint conflicts with the doc’s “local `copy`/`mirror` only” scope. `Commands::Copy`/`Mirror` wrap all `run_transfer` routes, not just local→local: current dispatch includes local→remote, remote→local, remote→remote, plus pre-transfer validation bails in [transfers/mod.rs:234](/home/michael/dev/Blit/crates/blit-cli/src/transfers/mod.rs:234). This would log remote/argument failures into `errors_local.jsonl` unless the scope changes or recording moves to a route-aware local-only point.

- [Medium] [docs/plan/LOCAL_ERROR_TELEMETRY.md:75](/home/michael/dev/Blit/docs/plan/LOCAL_ERROR_TELEMETRY.md:75) — Recorder-failure handling contradicts the byte-identical stderr requirement. The doc says recorder failures use `log::warn!`, but `blit` installs a stderr logger in [main.rs:33](/home/michael/dev/Blit/crates/blit-cli/src/main.rs:33), and warns are emitted via `eprintln!` in [stderr_log.rs:25](/home/michael/dev/Blit/crates/blit-core/src/stderr_log.rs:25). That changes stderr when the config dir/log write fails, violating lines 98-100 and 203-205.

- [Low] [docs/plan/LOCAL_ERROR_TELEMETRY.md:97](/home/michael/dev/Blit/docs/plan/LOCAL_ERROR_TELEMETRY.md:97) — `see D1` is an undefined internal reference; there is no D1 section in the draft. Same issue repeats at line 112.

**VERDICT: CHANGES REQUESTED.** Success-path perf-history claim is substantively accurate and the D-2026-07-05-4 “Draft-only/not queued” framing is consistent, but the local-only vs `main.rs` chokepoint conflict needs fixing before this design is coherent.
tokens used
146,668
**Findings**
- [High] [docs/plan/LOCAL_ERROR_TELEMETRY.md:128](/home/michael/dev/Blit/docs/plan/LOCAL_ERROR_TELEMETRY.md:128) — The proposed `main.rs` chokepoint conflicts with the doc’s “local `copy`/`mirror` only” scope. `Commands::Copy`/`Mirror` wrap all `run_transfer` routes, not just local→local: current dispatch includes local→remote, remote→local, remote→remote, plus pre-transfer validation bails in [transfers/mod.rs:234](/home/michael/dev/Blit/crates/blit-cli/src/transfers/mod.rs:234). This would log remote/argument failures into `errors_local.jsonl` unless the scope changes or recording moves to a route-aware local-only point.

- [Medium] [docs/plan/LOCAL_ERROR_TELEMETRY.md:75](/home/michael/dev/Blit/docs/plan/LOCAL_ERROR_TELEMETRY.md:75) — Recorder-failure handling contradicts the byte-identical stderr requirement. The doc says recorder failures use `log::warn!`, but `blit` installs a stderr logger in [main.rs:33](/home/michael/dev/Blit/crates/blit-cli/src/main.rs:33), and warns are emitted via `eprintln!` in [stderr_log.rs:25](/home/michael/dev/Blit/crates/blit-core/src/stderr_log.rs:25). That changes stderr when the config dir/log write fails, violating lines 98-100 and 203-205.

- [Low] [docs/plan/LOCAL_ERROR_TELEMETRY.md:97](/home/michael/dev/Blit/docs/plan/LOCAL_ERROR_TELEMETRY.md:97) — `see D1` is an undefined internal reference; there is no D1 section in the draft. Same issue repeats at line 112.

**VERDICT: CHANGES REQUESTED.** Success-path perf-history claim is substantively accurate and the D-2026-07-05-4 “Draft-only/not queued” framing is consistent, but the local-only vs `main.rs` chokepoint conflict needs fixing before this design is coherent.
