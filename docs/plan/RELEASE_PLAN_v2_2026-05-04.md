# Blit v2 Release Plan v2.1 — 2026-05-04

**Status:** Authoritative for the 0.1.0 release. Supersedes
`PROJECT_STATE_ASSESSMENT.md` (dated 2026-04-07, materially stale).

---

## Closure tracker (last updated 2026-05-07)

P0 items below are annotated with a per-section **Status** line
showing the closing commit (or pending notice) so the next agent
can read this doc as live state, not the original 2026-05-04
snapshot. Bird's-eye view:

| § | Item | Status |
|---|---|---|
| 2.1 | Binary rename `blit-cli` → `blit` | ✅ Closed `0ca489b` (R41 followup `e8f6aec`) |
| 2.2 | `blit-utils` artifact decision | ✅ Closed `aac13bf` (followup `8d43e4d`) |
| 2.3 | `blit list` smart-dispatch | ✅ Closed `4d07177` |
| 2.4 | `find --pattern` glob | ✅ Closed `090f5cd` (R41 followup `e8f6aec` for `literal_separator`) |
| 2.5 | Shell completions | ✅ Closed `0139a71` (Option A — `clap_complete` generation) |
| 2.6 | Live remote benchmark capture | ⏳ **Pending** — hardware-bound (two-daemon network) |
| 2.7 | `POST_REVIEW_FIXES` Round 1 | ✅ Closed `96cbb10` (R42 `3d953d9`, R43 `8fd928e`) |
| 2.8 | Predictor wire-or-delete | ✅ Wired (Option: wire) — phase 1 `ebcbb45`, phase 2 `da6ced2`, R44 `f83a208`, R45 `8351878` |
| 3.1 | Daemon `TransferMetrics` decision | ⏳ Pending — D5 default is "keep + document as dormant" |
| 3.2 | mDNS TXT enrichment | ⏳ Pending — see D4 |
| 3.3 | Phase 4.8 daemon FS capability | ⏳ Pending — doc-only re-scoping (D6 default: defer to 0.2.0) |
| 4 | Doc cleanup table | ✅ Closed `aac13bf` (followup `8d43e4d` caught binary-path stragglers) |

Decision-default outcomes (§7 table): D1=blit (rename), D2=merged,
D3=smart-dispatch, D7=glob, D8=Option A (clap_complete), D9=wire
were all taken. D4/D5/D6 still pending owner sign-off — they gate
the §3.1/§3.2/§3.3 closures.

**Net release-blocker count:** §2.6 (hardware-bound benchmark
capture) is the last remaining P0. Everything else either closed
or sits in P1 (§3.x) where the defaults from §7 are unambiguous
and the cost is small.

**Inputs:** `docs/audit/2026-05-04_roadmap_audit.md` (deep technical
audit, ~95 features classified across 17 plan docs) and
`docs/GPT_plan_review.md` rev 2 (release-surface audit, merged with
the roadmap audit findings). Both audits were independent; this plan
is their reconciled merge.

**Scope:** What ships in 0.1.0, what blocks the release, what's
explicitly deferred to 0.2.0+.

**v2.1 changes vs v2:**
- §2.5 (shell completions): kept P0, rationale anchored on
  `README.md:33` which explicitly promises "shell completions" as a
  feature. Either implement clap_complete generation OR edit the
  README — the doc claim is the release-blocking surface.
- §3.1 (predictor): promoted from P1 to **P0-decision**. Required
  outcome before release: predictor is consumed OR deleted. Default
  is wire (matches user's "harder, more correct thing" directive);
  fallback is delete if cushion is thin.
- §3.2 (POST_REVIEW_FIXES Round 1): promoted from P1 to P0. The
  vague "data plane pipeline closed unexpectedly" path in
  `MultiStreamSender::queue` is user-visible on real push failures;
  fixing it is a half-day item that materially affects release
  diagnostic quality.
- §6 commit sequence: revised for 8 P0 items (was 6). Total cost
  band shifts from 3-5 days to 4-7 days depending on predictor path.
- §7 decisions table: added D8 (shell completions implement vs README
  edit) and D9 (predictor wire vs delete).
- §2.1 (binary rename): added implementation caveat — ~6 test files
  hardcode `"blit-cli"` / `"blit-cli.exe"` string literals; rename
  must sweep them.

---

## 0. Why a new revision

Both audits converged on the same headline: the prior
`PROJECT_STATE_ASSESSMENT.md` overstates and understates in different
places.

- **Understates:** the entire pipeline-unification + remote→remote
  delegation work that landed since 2026-04-07 (commits
  `8a15e5a..0c00b4b` plus F14 close `30b95a2`) is invisible in PSA.
- **Overstates:** PSA marks Phase 4 "Done" when packaging beyond
  raw-tarball, per-export filesystem capability persistence, the
  predictor's actual consumption, and the daemon metrics reader are
  all incomplete or absent.

The two audits had complementary blind spots:

| Audit lens | Caught | Missed |
|---|---|---|
| Roadmap audit (Claude) | Dead code (predictor predictions never queried), write-only daemon metrics, mDNS TXT enrichment, `POST_REVIEW_FIXES.md` Round 1 status | Binary name drift, `blit list` semantics, `find --pattern` glob mismatch, shell completion script absence, Phase 4.8 daemon FS capability gap |
| Plan review (GPT) | Release-surface drift (binary name, `blit-utils`, `blit list`, `find --pattern`), Phase 4.8 daemon FS capability gap, packaging completeness | Predictor predictions are dead, daemon counters are write-only, mDNS TXT, `POST_REVIEW_FIXES` open |

Combined, they cover ~100 distinct findings. This plan acts on the
intersection plus the credible findings from each.

---

## 1. What actually shipped — 0.1.0 candidate inventory

Both audits agree. These are SHIPPING and tested:

### Core engine
- Universal `TransferOperationSpec` + `NormalizedTransferOperation::from_spec`
  validation chokepoint. Used by Push, PullSync, DelegatedPull.
- `DiffPlanner` + streaming planner with 10s stall detector and
  heartbeat scheduler.
- Fast-path routing (Tiny / Huge / NoWork).
- Always-on canonical-path containment (F2). `safe_join` and
  `verify_contained` chokepoints; per-module `canonical_root`.
- Tar-shard receive safety (R5-F2 / R6-F1 / R6-F3) — shared
  `tar_safety::safe_extract_tar_shard` across all three receive sites.
- `pull_sync` ↔ `pull_sync_with_spec` seam (R23-F1 / R25-F1).
- Hybrid TCP data plane with one-time tokens; automatic gRPC
  fallback.
- Block-resume (BlockHashRequest / BlockTransfer / BlockComplete with
  mtime+perms).
- F1 (path-safety chokepoint) through F13 (chroot removal) — 13 of 15
  baseline review findings closed.
- F14 (FSEvents → `objc2-core-services`) closed today (`30b95a2`).
- Change journals: Windows USN, macOS FSEvents, Linux metadata
  snapshot.
- Local performance history (`perf_history.rs`) with capped JSONL,
  schema versioning, settings persistence.
- Adaptive bucket-target tuning via `derive_local_plan_tuning`
  (`auto_tune::mod.rs`) reading recent records.

### Remote
- Push: control plane + bounded-channel manifest, NeedList, TCP data
  plane with parallel streams, gRPC fallback, force-grpc flag.
- Pull (PullSync): unified spec, filter parity, tar shards, delete
  list, checksum negotiation (F11/R15), gRPC fallback.
- Remote→remote delegation (`DelegatedPull`):
  - Default direct path (Phase 2, commit `0c00b4b`).
  - `--relay-via-cli` operator escape hatch.
  - Delegation gate (`[delegation]` config block, IDNA/CIDR/IP
    matching, R25-F3 special-range rule, DNS-rebinding mitigation,
    per-module override).
  - No-silent-fallback CLI dispatch.
  - R30/R32/R34 review fixes: mirror delete list applied locally on
    dst, `tx.closed()` cancellation, `AbortOnDrop` on every internal
    spawn, full `from_spec` validation, `entries_deleted` reports
    only the local count.
- Admin RPCs: ListModules, List, Find, DiskUsage, FilesystemStats,
  CompletePath, Purge.

### CLI surface
- `blit copy / mirror / move / scan / list-modules / ls / list /
  find / du / df / rm / completions / profile / check / diagnostics`.
- `blit-daemon` config file (TOML), CLI overrides, `[delegation]`
  block, mDNS, motd.

### Documentation
- `docs/cli/blit.1.md` and `docs/cli/blit-daemon.1.md`.
- `docs/DAEMON_CONFIG.md` (extensive: trust model, containment,
  delegation, mDNS).
- `README.md`, `CHANGELOG.md`.
- Reviews: `codebase_review_2026-05-01.md` and 35+ followup rounds
  in `followup_review_2026-05-02.md`.
- This plan + the two underlying audits.

### CI / build
- Tri-platform CI: `cargo fmt --all -- --check`, `cargo clippy
  --workspace --all-targets -- -D warnings`, `cargo test
  --workspace` on Linux, macOS, Windows.
- Release builds + artifact upload.

**Test totals:** 383 workspace tests, 0 failed (as of `30b95a2`,
the 2026-05-04 baseline). As of 2026-05-07: **407 / 0** after R41
through R45 review-fix commits added regression coverage for the
binary rename, glob `literal_separator`, drain-helper unit tests,
predictor schema v2 + dual-target learning, predictor-record
feature-vector consistency, and `summary.total_bytes`
bytes-written contract.

---

## 2. Release-blocking work (P0)

Each item must close before tagging 0.1.0. Each has a recommended
default; product owner can override.

### 2.1 Binary name drift — `blit-cli` vs `blit`

**Status (2026-05-07):** ✅ Closed `0ca489b`. Crate name stayed
`blit-cli`; binary produced is named `blit` via
`[[bin]] name = "blit"`. R41 followup `e8f6aec` swept the test
files that hardcoded `"blit-cli"` / `"blit-cli.exe"` as filesystem
paths.

**Source:** GPT review P0.

Workspace ships `blit-cli` as the binary name. Every doc, manpage,
README example, and CHANGELOG entry uses `blit`. Clap's
`command(name = "blit")` makes the help/error output say `blit`, but
the file users run is still named `blit-cli`. CI, build scripts, and
artifact upload all reference `blit-cli`.

**Recommendation:** Add `[[bin]] name = "blit"` to
`crates/blit-cli/Cargo.toml`, then update `scripts/build-release.sh`
(`BINARIES=(blit-cli blit-daemon)` → `BINARIES=(blit blit-daemon)`),
`.github/workflows/ci.yml` (`blit-cli` artifact paths), and every
test that exec's the binary directly.

**Cost:** ~1-2 hours including test plumbing. Crate-name stays
`blit-cli` (no Cargo.toml ripple); only the produced binary renames.

**Implementation caveat:** ~6 test files in
`crates/blit-cli/tests/` (single_file_copy, remote_remote,
remote_parity, remote_tcp_fallback, remote_checksum_negotiation,
remote_push_single_file) resolve the binary by walking
`current_exe().parent().parent()` and hardcoding the literal
`"blit-cli"` / `"blit-cli.exe"`. Rename must sweep those literals.
`CARGO_BIN_EXE_*` is not used — the test pattern is filesystem
walk + name string. The "ship both binaries" workaround adds
artifact confusion and is rejected.

**Alternative:** keep binary as `blit-cli`, update every user-facing
doc to match. Worse for users — `blit` is the planned product name.

### 2.2 `blit-utils` artifact decision

**Status (2026-05-07):** ✅ Closed `aac13bf` (sweep of the §4 doc
table) + `8d43e4d` (followup catching stale binary paths in the
benchmark playbook + whitepaper, README completions syntax). All
plan/architecture/manpage references now point at the single
`blit` binary; `BLIT_UTILS_PLAN.md` got a Superseded banner;
phase-3/phase-4 workflow docs got post-phase notes.

**Source:** GPT review P1.

`CHANGELOG.md` says admin utilities were merged into `blit`. There
is no `crates/blit-utils` crate, no `docs/cli/blit-utils.1.md`.
Several plan docs still treat `blit-utils` as a standalone artifact:
`MASTER_WORKFLOW.md`, `BLIT_UTILS_PLAN.md`, `WORKFLOW_PHASE_3.md`,
`WORKFLOW_PHASE_4.md`, `PROJECT_STATE_ASSESSMENT.md`,
`docs/ARCHITECTURE.md`. Roadmap audit incorrectly marked
`blit-utils` as shipping (an AppleDouble `._blit-utils.1.md`
sidecar likely confused the grep).

**Recommendation:** Treat the merge as final. Update every plan/doc
reference to either remove the `blit-utils` mention or rephrase as
"admin commands available under `blit <subcommand>`."

**Cost:** ~2-3 hours of doc edits. Mechanical.

### 2.3 `blit list` semantics

**Status (2026-05-07):** ✅ Closed `4d07177`. Smart-dispatch
implemented: bare-host targets route to `list-modules`;
module/path targets route to `ls`. The explicit `blit list-modules
<remote>` form continues to work. Documented in
`docs/cli/blit.1.md` Admin Commands section.

**Source:** GPT review P1.

Plan v6 and Phase 3 docs say `blit list server` lists modules. Code
behavior:
- `blit list-modules <remote>` lists modules.
- `blit ls <target>` lists directory contents.
- `blit list` is an alias for `ls`.
- `ls.rs` rejects bare hosts: "listing a bare host requires
  `list-modules` or module/path syntax".

**Recommendation:** Make `blit list` smart-dispatch. If the target
parses as a bare host (no module, no path), treat it as
`list-modules`. If the target has a path, treat it as `ls`. This
matches the plan and avoids breaking existing `blit list-modules`
users (the explicit form keeps working).

**Cost:** ~1 hour. The dispatch lives in `transfers/endpoints.rs` /
`ls.rs`; bare-host detection is already done in the endpoint
parser.

**Alternative:** keep `list-modules` as the only way to list
modules; remove `list` as an alias for `ls`; update plan and
manpage to match. Plan/docs change is bigger than the code change.

### 2.4 `find --pattern` glob vs substring

**Status (2026-05-07):** ✅ Closed `090f5cd` (initial glob via
`globset`) + R41 followup `e8f6aec` (set
`literal_separator(true)` so `*` does not cross `/`, plus added
basename-fallback regression test). CHANGELOG updated to drop the
substring known-limitation.

**Source:** GPT review P1, my audit confirmed.

`BLIT_UTILS_PLAN.md` specifies glob matching; `CHANGELOG.md`
documents the substring limitation as known. `FindArgs` uses
substring.

**Recommendation:** Implement glob via `globset` (already a
workspace dep). Substring users can still match with `*foo*`. Keep
the existing `--pattern` flag name; semantics change.

**Cost:** ~30 minutes. `globset::Glob::new(pattern)?.compile_matcher()`
in the `Find` handler in `service/admin.rs` (server side) and
`find.rs` (CLI side, if the CLI does any client-side filtering).
Update CHANGELOG to say "glob matching."

### 2.5 Shell completions — match docs to code or code to docs

**Status (2026-05-07):** ✅ Closed `0139a71` — picked Option A
(clap_complete script generation). The completions subcommand
split into two forms: `blit completions shell <SHELL>` writes a
clap_complete-generated bash/zsh/fish/powershell/elvish script to
stdout; `blit completions remote <REMOTE> [--prefix <STR>]
[--files] [--dirs]` keeps the daemon-backed `CompletePath` RPC
form (used internally by the generated shell scripts for
remote-path completion). Documented in `docs/cli/blit.1.md` and
`README.md`.

**Source:** GPT review (Phase 3.4.4 / `BLIT_UTILS_PLAN.md`); release
surface anchored on `README.md:33`.

`README.md:33` explicitly promises "shell completions" as a feature
of the merged-into-`blit` utilities:

```
Built into `blit`: daemon inspection and maintenance via mDNS
discovery, module listing, remote `ls`/`find`/`du`/`df`/`rm`,
shell completions, and performance profiling.
```

The current `blit completions` subcommand description is *"Fetch
remote path completions for interactive shells"* — which is the
remote-path-completion via `CompletePath` RPC, not bash/zsh/fish/
PowerShell **script generation**. A typical user reading "shell
completions" expects the latter (a script you source from
`~/.bashrc`).

The README claim is ambiguous-but-leaning-script-generation. To
resolve before release:

**Option A (recommended):** Add a separate `blit completions
<shell>` (or `blit completions generate <shell>`) subcommand using
`clap_complete`. Generates static bash/zsh/fish/powershell
completion. Existing remote-path completion stays as-is or moves
under `blit completions remote <REMOTE> [--prefix <STR>]
[--files] [--dirs]`. ~2 hours. `clap_complete` dep +
`Command::generate` in the handler.

**Option B:** Edit `README.md:33` to remove "shell completions"
from the feature list, OR rephrase as "remote path completion
(consumable by your shell's completion harness)" plus add docs for
wiring it in. ~30 min, no code.

Pick before release. README claim is the release-blocking surface,
not the Phase 3.4.4 plan-doc reference.

### 2.6 Live remote benchmark capture

**Status (2026-05-07):** ⏳ **PENDING — last open P0.** Hardware-bound;
needs the two-daemon network. The benchmark playbook in
`docs/plan/BENCHMARK_10GBE_PLAN.md` has been brought up to current
binary names (`blit`, not `blit-cli`) so it's now followable.

**Source:** Both audits agree.

`docs/perf/remote_remote_benchmarks.md` is template-only.
`scripts/bench_remote_remote.sh` exists. The user is spinning up
two remotes today.

**Recommendation:** Run the benchmark script on the two-daemon
network, capture results into the perf doc with avg + best MiB/s
per mode (direct vs `--relay-via-cli`), CLI byte-counter assertion
(`0` direct, `~payload` relay), and a notes column for fabric
speed.

**Cost:** ~1 hour wall-clock once daemons are reachable. Pre-flight
checks documented in earlier exchange.

### 2.7 `POST_REVIEW_FIXES.md` Round 1 closure (promoted from P1)

**Status (2026-05-07):** ✅ Closed `96cbb10`. R42 followup `3d953d9`
finished the §1.1 metadata-error sweep (caught three sites the
initial commit missed) and the §1.1b drain-helper test extraction.
R43 `8fd928e` factored `drain_pipeline_outcome` so the comment
claiming `finish()` shared the helper actually matched the code.

**Source:** Roadmap audit caught the open round; GPT rev 2
release-blocking.

Round 1 in `docs/plan/POST_REVIEW_FIXES.md` enumerates four items.
Promoted to P0 because §1.1b is user-visible on real push failures
and the round was explicitly scoped as "~half day" by the doc
author — too cheap to defer.

- §1.1 — `let _ = ...` error swallowing in `sink.rs` (most
  consequential: silently dropped `file.flush()` results).
- §1.1b — `MultiStreamSender::queue` returning a vague "data plane
  pipeline closed unexpectedly" instead of the real underlying
  error. User-visible push error quality.
  (`crates/blit-core/src/remote/push/client/mod.rs` per GPT rev 2.)
- §1.2 — Delete `TarShardExecutor` (~100 LOC dead code) or document
  why it stays. (`crates/blit-daemon/src/service/push/data_plane.rs`
  per GPT rev 2.)
- §1.3 — Update `WHITEPAPER.md` for the `BLOCK_COMPLETE`
  mtime+perms wire change (already shipped in `a7d659f`; doc lags).

**Recommendation:** Close all four. Round 1 is explicitly scoped as
"~half day" in the doc itself; treating it as a single commit is
fine.

**Cost:** ~4-6 hours.

### 2.8 Predictor — wire OR delete (P0-decision, promoted from P1)

**Source:** Roadmap audit headline. GPT rev 2 release-blocking;
GPT recalibration suggested "deletion may be better if cushion is
thin." Reframed here as a required wire-or-delete decision before
release. Either resolves the dead-code / no-tech-debt directive.

`PerformancePredictor::predict_ms` and `predict_planner_ms` are
called only in unit tests. Every run pays the cost of loading +
training + saving the predictor file, and nothing reads back its
predictions. `derive_local_plan_tuning` is the only loop that
closes — and it goes around the predictor, reading JSONL directly.

The required outcome: predictor is consumed OR removed. Silent
dead code is incompatible with the "no tech debt for the sake of
backwards compatibility" release directive.

**Default — wire it (the harder, more correct thing).** Concrete
plan:

1. Extend `PerformancePredictor` to learn both `planner_duration_ms`
   AND `transfer_duration_ms` per profile. Schema bump to v2;
   existing `load()` already drops state on version mismatch.
2. Add confidence-aware fallback: `predict_*` methods walk a
   fallback chain (exact key → drop fast_path → drop dest_fs → drop
   src_fs) and return `(prediction_ms, observations, fallback_depth)`.
3. Wire predicted total duration into the Tiny fast-path decision in
   `orchestrator.rs`: take Tiny when predicted planner cost is
   non-trivial relative to predicted transfer cost.
4. Surface predictions in `LocalMirrorSummary`, `--verbose` output,
   and `blit profile --json`.

**Cost:** ~1.5 days end-to-end with tests.

**Status (2026-05-07):** ✅ Closed. Phase 1 `ebcbb45` (data-model
v2: dual targets + fallback chain), phase 2 `da6ced2` (orchestrator
query + verbose/JSON surface). R44 `f83a208` fixed a train/query
feature mismatch (orchestrator queries with `(scanned_files,
scanned_bytes)` and the recorded `PerformanceRecord.{file_count,
total_bytes}` is now populated from the same scan-side fields, so
estimates no longer drift on incremental runs). R44-F2 pinned the
`blit profile --json` shape with an assertion. R45 `8351878` fixed
a follow-on alias bug where `summary.total_bytes` was reporting
scanned bytes instead of bytes-written; the streaming summary now
reads `pipeline_outcome.bytes_written` directly, and all three
predictor sub-calls share the explicit `(scanned_files,
scanned_bytes)` feature vector.

Steps 1, 2, and 4 are done — the predictor learns planner +
transfer separately, walks the fallback chain, and is visible in
`--verbose` and `blit profile --json`. Step 3 (Tiny extension)
was deferred to post-0.1.0 with explicit reasoning in DEVLOG. So
§2.8 closed as **predictor observability and training** — the
predictor is no longer dead code (it is queried, surfaced, and
audit-able), but adaptive planning behavior (Tiny picking up
predictor signals) is still future work. Anyone reading this
section after release should NOT assume the planner consults the
predictor for routing; only the verbose/JSON surface does.

**Fallback — delete it.** If release cushion is thin: delete
`perf_predictor.rs`, drop `update_predictor` calls, keep
`perf_history` + `derive_local_plan_tuning` which are the
load-bearing paths. Update README to remove any "adaptive
predictor" claim.

**Cost:** ~2 hours.

---

## 3. Quality work (P1) — close before release if practical

These don't strictly block 0.1.0 but are visible-quality issues
that reviewers and early users will hit.

### 3.1 Daemon `TransferMetrics` decision

**Status (2026-05-07):** ⏳ Pending. D5 default ("keep + document
as dormant") has not been actioned in code yet — `metrics.rs:14`
docstring already says "scaffolding for a future GUI/TUI", but
the `--metrics` flag's `--help` text and the module-level rustdoc
have not been updated to spell out "dormant, intended for 0.2.0
TUI." ~30 min of doc-only work when picked up.

**Source:** Roadmap audit headline.

`crates/blit-daemon/src/metrics.rs` has push/pull/purge attempt
counters, error counter, active-transfer gauge (with R5-F2
RAII guard). `--metrics` opt-in, off by default. **No reader.**
Module docstring (`metrics.rs:14`) explicitly says "scaffolding for
a future GUI/TUI gRPC `GetState`-style RPC." `TUI_DESIGN.md`
specifies the RPCs. Neither `Subscribe` nor `GetState` exists in
`proto/blit.proto`.

**Recommendation:** For 0.1.0, **do not** ship the TUI RPCs.
Either:

- (a) Drop `--metrics` and `TransferMetrics` entirely. ~30 LOC of
  removal. Honest.
- (b) Keep `TransferMetrics` and `--metrics` flag, but explicitly
  document in the module docstring + `--help` text that this is
  scaffolding with no current consumer, intended for 0.2.0 TUI.

I recommend **(b)**. The counters are correct and tested; deleting
them and re-adding them later is wasted churn. Just be explicit
that they're dormant.

**Cost:** ~30 minutes for option (b). 1 hour for option (a).

### 3.2 mDNS TXT enrichment

**Status (2026-05-07):** ⏳ Pending. D4 default is "Yes (small,
useful)" but has not been implemented. ~1-2 hours when picked up;
non-release-blocking.

**Source:** Roadmap audit. `TUI_DESIGN.md` flagged this as
"do early."

`crates/blit-core/src/mdns.rs` advertises bare `_blit._tcp.local.`
with no TXT records. Operators discovering daemons over mDNS get
hostname + port only — no module list, no version, no
capabilities. `TUI_DESIGN.md` specified TXT records for this.

**Recommendation:** Add TXT records for `version`,
`module_count`, and `delegation_enabled`. Small, useful for `blit
scan` UX, no protocol changes.

**Cost:** ~1-2 hours including a roundtrip test that the TXT
records actually surface in `blit scan`.

**Alternative:** explicitly defer to 0.2.0; remove the
`TUI_DESIGN.md` "do early" note as superseded.

### 3.3 Phase 4.8 — daemon FS capability per-export

**Status (2026-05-07):** ⏳ Pending. D6 default ("defer to 0.2.0;
doc only") not yet recorded in `WORKFLOW_PHASE_4.md` §4.8.2 / 4.8.3.
~30 min of doc-only work when picked up; non-release-blocking.

**Source:** GPT review.

`fs_capability` module exists with platform probes/cache, used by
`copy/diagnostics`. `WORKFLOW_PHASE_4.md` §4.8.2 specifies the
daemon should probe and persist per-export FS capability at
startup/idle. Currently no daemon-runtime use of `fs_capability`.
GPT also notes §4.8.3: `profile` does not run capability probes;
`diagnostics dump` does.

**Recommendation:** For 0.1.0, scope down §4.8 to "client-side
probing in `diagnostics dump`" (which is the current state) and
explicitly mark §4.8.2/4.8.3 as deferred to 0.2.0. Document in this
plan.

**Cost:** ~30 minutes (doc-only).

**Alternative:** implement the daemon-side probe. ~1 day. Probably
not worth blocking release on.

---

## 4. Doc cleanup (P2) — required for 0.1.0 honesty

**Status (2026-05-07):** ✅ Closed `aac13bf` (sweep) + `8d43e4d`
(followup catching stale binary paths in BENCHMARK_10GBE_PLAN.md
and WHITEPAPER.md, plus README completion-syntax). Reconciled
list with closing actions:

| Doc | Closing action |
|---|---|
| `docs/plan/PROJECT_STATE_ASSESSMENT.md` | Top-of-file Superseded callout pointing here. CLI Surface and Documentation sections rewritten — single `blit` binary, no `blit-utils` line. Architecture Overview ASCII diagram redrawn. |
| `docs/plan/README.md` | Current Status flipped to point at this plan as live source of truth; PSA labelled superseded snapshot; BLIT_UTILS_PLAN tagged as design-rationale-only. |
| `docs/plan/MASTER_WORKFLOW.md` | Feature-Completeness Goals, Phase 3 gates, Telemetry row reword `blit-utils` → `blit` admin subcommands. |
| `docs/plan/BLIT_UTILS_PLAN.md` | Top-of-file Superseded banner — artifact never shipped; body retained as design rationale. |
| `docs/plan/WORKFLOW_PHASE_3.md` | Post-phase note at top so historical body stays as workflow record without being read as current state. |
| `docs/plan/WORKFLOW_PHASE_4.md` | Same. |
| `docs/ARCHITECTURE.md` | User Layer ASCII diagram redrawn (no `blit-utils` column); `### blit-cli` rewritten with merged structure; `### blit-utils` section replaced with `### Admin verbs` cross-referencing the superseded plan. |
| `docs/cli/blit.1.md` | SYNOPSIS adds `list-modules`, `ls`, `completions shell <SHELL>`, `completions remote <REMOTE>`, and `profile`; Admin Commands section documents all of them plus the §2.3 smart-dispatch wording. |
| `README.md` | `crates/blit-cli/` repo-structure entry annotated to clarify it produces the `blit` binary; `blit completions remote <REMOTE> [--prefix <STR>]` syntax fixed. |

Drive-bys outside the original table caught by the `8d43e4d`
followup: `docs/plan/BENCHMARK_10GBE_PLAN.md` (active 10 GbE
benchmark playbook) and `docs/WHITEPAPER.md` (build/run example,
crate description) had `target/release/blit-cli` paths replaced
with `blit`; `docs/plan/AI_TELEMETRY_ANALYSIS.md` "Phase 3:
blit-utils Integration" section rewritten; `docs/plan/TUI_DESIGN.md`
ScanResponse-effort line corrected to point at `blit-cli`'s scan
parsing (was "blit-utils-merged").

---

## 5. Out of scope for 0.1.0 — explicit deferrals

Everything below is intentionally NOT in 0.1.0. Each has a
documented reason. Re-opening any of these requires a product
decision, not a code decision.

### 5.1 F15 — Structured logging across daemon + transfer

Many `eprintln!` paths remain in daemon and core. `tracing` /
structured `log` migration is a 1-2 week effort. Explicitly
deferred per `PROJECT_STATE_ASSESSMENT.md`. Re-open in 0.2.0 once
operational pain demonstrates need.

### 5.2 BlitAuth

Proto stub exists at `proto/blit.proto:40-42` (`BlitAuth.Authenticate`)
with zero implementations. The `RemoteSourceLocator.delegated_credential`
field is plumbed for forward-compat. 0.1.0 trust model is operator
network controls + per-transfer data-plane tokens; documented in
`DAEMON_CONFIG.md` Trust Model section.

### 5.3 RDMA / RoCE data plane (Phase 3.5)

Proto-only reservation. Hardware-bound, post-release investigation.

### 5.4 AI telemetry analysis

`AI_TELEMETRY_ANALYSIS.md` exists as design doc. No `perf_analysis`
module, no `blit diagnostics analyze`. Local-only, opt-in,
no-network guardrails are documented.

### 5.5 TUI

`TUI_DESIGN.md` exists. No `Subscribe` / `GetState` RPCs in proto.
No TUI binary or scaffolding. Deferred. The daemon's
`TransferMetrics` are kept as scaffolding (see §3.3).

### 5.6 `--detach` mode for delegated pull

CLI Ctrl-C abort is the current contract. `--detach` would require
a job-tracking RPC and durable state on the daemon. Out of scope.

### 5.7 Full packaging matrix

0.1.0 ships raw binaries + tarball + tri-platform CI artifacts.
Debian/RPM, Homebrew formula, Windows installer, systemd/launchd
service unit installers all deferred to 0.2.0. `DAEMON_CONFIG.md`
documents manual service setup for each platform.

### 5.8 Hardware benchmarks beyond the immediate

`BENCHMARK_10GBE_PLAN.md`'s NFS/SMB-mount, daemon-pair, and
reverse-direction phases all need 10GbE hardware. Do them
post-release when hardware is available; 0.1.0's release notes
note "10+ Gbps benchmarking pending hardware access."

### 5.9 Investigations

- macOS FSEvents fast-path real-network field-test (`UNVERIFIED` in
  audit).
- Windows ReFS `SeManageVolumePrivilege` requirement for block
  clone (`TODO.md:259`).

Both are post-release tracking items.

---

## 6. Suggested commit sequence

**Status (2026-05-07):** all P0 commits below have landed except
step 2 (live remote benchmark). Recorded original ordering plus
actual closing commits for traceability.

| # | Item | Status / closing commit |
|---|---|---|
| 1 | Decide D1-D9 (§7) | ✅ D1/D2/D3/D7/D8/D9 taken; D4/D5/D6 still open (P1 / doc-only) |
| 2 | §2.6 — Run remote benchmark | ⏳ **Pending** (last open P0; hardware-bound) |
| 3 | §2.4 — `find --pattern` glob | ✅ `090f5cd` (R41 followup `e8f6aec`) |
| 4 | §2.5 — Shell completions (Option A) | ✅ `0139a71` |
| 5 | §2.1 — Binary rename `blit-cli` → `blit` | ✅ `0ca489b` (R41 followup `e8f6aec`) |
| 6 | §2.3 — `blit list` smart-dispatch | ✅ `4d07177` |
| 7 | §2.2 + §4 doc cleanup | ✅ `aac13bf` (followup `8d43e4d`) |
| 8 | §2.7 — POST_REVIEW_FIXES Round 1 | ✅ `96cbb10` (R42 `3d953d9`, R43 `8fd928e`) |
| 9 | §3.2 — mDNS TXT enrichment (P1) | ⏳ Not started; deferrable |
| 10 | §3.1 — `TransferMetrics` doc-only | ⏳ Not started; deferrable |
| 11 | §3.3 — Phase 4.8 doc-only rescope | ⏳ Not started; deferrable |
| 12 | §2.8 — Predictor wire-or-delete | ✅ Wired — phase 1 `ebcbb45`, phase 2 `da6ced2`, R44 `f83a208`, R45 `8351878` |
| 13 | Tag 0.1.0 | ⏳ Blocked on step 2 (and product-owner ack of P1 deferrals) |

**Cost band actuals:** P0 work ran ~3 days of focused effort plus
the still-pending §2.6 benchmark wall-clock. Steps 9-11 (P1) total
~3 hours when scheduled and can land post-tag if owner agrees the
defaults are acceptable.

---

## 7. Decisions still owed

**Status (2026-05-07):** D1, D2, D3, D7, D8, D9 were taken (default
recommendations). D4, D5, D6 still open but each has a clear
default and the cost is small.

| # | Decision | Default | Outcome |
|---|---|---|---|
| D1 | Binary name: `blit` or `blit-cli`? | `blit` (rename via `[[bin]]`) | ✅ Taken — `0ca489b` |
| D2 | `blit-utils` artifact: standalone or merged? | Merged | ✅ Taken — `aac13bf` |
| D3 | `blit list` semantics: smart-dispatch or `list-modules`-only? | Smart-dispatch | ✅ Taken — `4d07177` |
| D4 | mDNS TXT enrichment in 0.1.0? | Yes (small, useful) | ⏳ Open — §3.2 not yet started |
| D5 | `TransferMetrics` keep-as-scaffolding or remove? | Keep + document as dormant | ⏳ Open — §3.1 doc-only work pending |
| D6 | Phase 4.8.2/4.8.3 daemon FS capability in 0.1.0? | Defer to 0.2.0; doc only | ⏳ Open — §3.3 doc-only work pending |
| D7 | `find --pattern` glob or substring? | Glob | ✅ Taken — `090f5cd` |
| D8 | Shell completions: clap_complete generation OR README edit? | Option A (clap_complete) | ✅ Taken — `0139a71` |
| D9 | Predictor: wire OR delete? | Wire | ✅ Taken — `ebcbb45` + `da6ced2` |

---

## 8. Cross-reference — audit findings to plan items

| Audit finding | Source | Plan section |
|---|---|---|
| Predictor predictions never queried | Roadmap audit | §2.8 (P0-decision) |
| Daemon TransferMetrics write-only | Roadmap audit | §3.1, §5.5 |
| mDNS TXT enrichment not started | Roadmap audit | §3.2 |
| POST_REVIEW_FIXES Round 1 open | Roadmap audit | §2.7 (P0) |
| PROJECT_STATE_ASSESSMENT stale | Both | §0, §4 |
| RDMA / AI telemetry / BlitAuth / `--detach` deferred | Both | §5 |
| F14 closed today, F15 only remaining baseline | Both | §1, §5.1 |
| ReFS SeManageVolumePrivilege open | Both | §5.9 |
| Live remote benchmark missing | Both | §2.6 |
| Binary name drift (`blit-cli` vs `blit`) | GPT | §2.1 |
| `blit-utils` doc-vs-code mismatch | GPT | §2.2, §4 |
| `blit list` semantics drift | GPT | §2.3 |
| `find --pattern` substring vs glob | GPT | §2.4 |
| Shell completion scripts missing | GPT | §2.5 (anchored on `README.md:33`) |
| Phase 4.8.2/4.8.3 daemon FS capability gap | GPT | §3.3 |
| Phase 4 packaging matrix incomplete | GPT | §5.7 |
| Final QA checklist absent | GPT | §5.7 (acceptable for 0.1.0) |

---

## 9. Methodology

This plan was synthesized from:

1. `docs/audit/2026-05-04_roadmap_audit.md` — produced by a
   research agent that read every plan doc and grepped the
   codebase for symbols. Strong on internal architecture and
   dead-code detection. Weaker on release-surface drift.
2. `docs/GPT_plan_review.md` — produced by GPT (external
   reviewer). Strong on release-surface, command-name, and
   packaging gaps. Weaker on internal architecture.
3. The reconciliation conversation between Claude and GPT
   (`compare_audits` exchange).

Where the two audits disagreed (predictor "Done" vs "Dead"; mDNS
"Implemented" vs "TXT-incomplete"; `blit-utils` "Shipping" vs
"Missing"), the more rigorous reading wins: ship-state must be
verifiable against running code, not just file presence.

Where the two audits independently agreed (deferrals, F14 closure,
benchmarks pending), confidence is high.

---

**Owner:** mcoelho
**Last updated:** 2026-05-07 (per-section status added; closure
tracker at the top reflects the §2.1/§2.2/§2.3/§2.4/§2.5/§2.7/§2.8
+ §4 closures; §2.6 + §3.x still pending)
**Next review:** after §2.6 benchmark capture, or before tagging
0.1.0.
