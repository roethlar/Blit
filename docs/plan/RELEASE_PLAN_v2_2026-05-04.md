# Blit v2 Release Plan v2.1 — 2026-05-04

**Status:** Authoritative for the 0.1.0 release. Supersedes
`PROJECT_STATE_ASSESSMENT.md` (dated 2026-04-07, materially stale).

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

**Test totals:** 383 workspace tests, 0 failed (as of `30b95a2`).

---

## 2. Release-blocking work (P0)

Each item must close before tagging 0.1.0. Each has a recommended
default; product owner can override.

### 2.1 Binary name drift — `blit-cli` vs `blit`

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
under `blit completions remote <prefix>`. ~2 hours. `clap_complete`
dep + `Command::generate` in the handler.

**Option B:** Edit `README.md:33` to remove "shell completions"
from the feature list, OR rephrase as "remote path completion
(consumable by your shell's completion harness)" plus add docs for
wiring it in. ~30 min, no code.

Pick before release. README claim is the release-blocking surface,
not the Phase 3.4.4 plan-doc reference.

### 2.6 Live remote benchmark capture

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

**Status (R44, 2026-05-04):** Phase 1 + 2 landed at `da6ced2` /
`<this commit>`. Steps 1, 2, and 4 are done — the predictor learns
planner + transfer separately, walks the fallback chain, and is
visible in `--verbose` and `blit profile --json`. Step 3 (Tiny
extension) was deferred to post-0.1.0 with explicit reasoning in
DEVLOG. So §2.8 closed as **predictor observability and training**
— the predictor is no longer dead code (it is queried, surfaced,
and audit-able), but adaptive planning behavior (Tiny picking up
predictor signals) is still future work. Anyone reading this
section after release should NOT assume the planner consults the
predictor for routing; only the verbose/JSON surface does. R44-F1
also fixed a train/query feature mismatch — the orchestrator now
queries with `(scanned_files, scanned_bytes)` and the recorded
`PerformanceRecord.{file_count,total_bytes}` is populated from the
same scan-side fields, so estimates no longer drift on incremental
runs. R44-F2 pinned the `blit profile --json` shape with an
assertion.

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

Both audits enumerate stale docs. Reconciled list:

| Doc | Drift | Action |
|---|---|---|
| `docs/plan/PROJECT_STATE_ASSESSMENT.md` | "Feature-complete 2026-04-07" predates pipeline-unification + delegation. Lists `blit-utils` as shipping. | Mark superseded by this doc; keep as historical artifact with an explicit pointer at the top to `RELEASE_PLAN_v2_2026-05-04.md`. |
| `docs/plan/README.md` | Same currency mismatch as PSA. Lists `blit-utils` as shipping. | Update "Last Updated" + add prominent pointer to this plan. Drop `blit-utils` references. |
| `docs/plan/MASTER_WORKFLOW.md` | Describes admin tooling as `blit-utils`. | Replace `blit-utils` with `blit` admin subcommands. |
| `docs/plan/BLIT_UTILS_PLAN.md` | Standalone `blit-utils`, glob `find` semantics, separate manpage. | Add header: "Superseded — admin utilities merged into `blit` subcommands. See `docs/cli/blit.1.md`." Keep the document for the command-matrix design, mark inapplicable parts. |
| `docs/plan/WORKFLOW_PHASE_3.md` | Claims `crates/blit-utils/src/main.rs` exists. | Update to reflect merged-CLI reality. |
| `docs/plan/WORKFLOW_PHASE_4.md` | Implies `blit-utils` packaging. | Same. |
| `docs/ARCHITECTURE.md` | Top-level `blit-utils` component diagram. | Remove or label "superseded — merged into `blit-cli`." |
| `docs/cli/blit.1.md` | Synopsis says `blit list <REMOTE>`; actual `list` aliases `ls`. | Match whichever 2.3 outcome we pick. |
| `README.md` | Uses `blit` command in examples; binary is currently `blit-cli`. | Match whichever 2.1 outcome we pick. |

**Cost:** ~3-4 hours total once 2.1 and 2.3 are decided.

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

If acting on this plan in order:

1. **Decide D1-D9** (§7 below). Particularly D1, D3, D8, D9.
2. **§2.6** — Run remote benchmark while daemons are reachable. ~1
   hour wall-clock. No code changes.
3. **§2.4** — `find --pattern` glob fix. ~30 min, single commit.
4. **§2.5** — Shell completions via `clap_complete` (Option A) or
   README edit (Option B). ~30 min - 2 hours, single commit.
5. **§2.1** — Binary rename `blit-cli → blit`. ~1-2 hours, single
   commit (`Cargo.toml` `[[bin]]` + `~6` test files with hardcoded
   string literals + scripts + CI).
6. **§2.3** — `blit list` smart-dispatch. ~1 hour, single commit.
7. **§2.2** + **§4 doc cleanup** — `blit-utils` references and all
   stale docs. ~4-5 hours, single doc commit.
8. **§2.7** — POST_REVIEW_FIXES Round 1. ~half day, single commit.
   (§1.1 sink.rs error swallowing, §1.1b MultiStreamSender real
   error, §1.2 TarShardExecutor, §1.3 WHITEPAPER doc.)
9. **§3.2** — mDNS TXT enrichment (if doing it; defer otherwise).
   ~1-2 hours.
10. **§3.1** — `TransferMetrics` doc-only fix (option b). ~30 min.
11. **§3.3** — Phase 4.8 doc-only re-scoping. ~30 min.
12. **§2.8** — Predictor wire-or-delete. ~1.5 days (wire) or ~2
    hours (delete). Last because it's the biggest variance.
13. **Tag 0.1.0.**

**Cost band:**
- Steps 1-11: roughly **2-3 days** of work plus benchmark
  wall-clock.
- Step 12: **+1.5 days** (wire) or **+2 hours** (delete).
- Total: **4-7 days** to release-ready depending on D9 outcome.

---

## 7. Decisions still owed

| # | Decision | Default recommendation | Owner |
|---|---|---|---|
| D1 | Binary name: `blit` or `blit-cli`? | `blit` (rename via `[[bin]]`); ~6 test files have hardcoded string literals to sweep | mcoelho |
| D2 | `blit-utils` artifact: standalone or merged? | Merged (current state); fix docs to match | mcoelho |
| D3 | `blit list` semantics: smart-dispatch or `list-modules`-only? | Smart-dispatch | mcoelho |
| D4 | mDNS TXT enrichment in 0.1.0? | Yes (small, useful) | mcoelho |
| D5 | `TransferMetrics` keep-as-scaffolding or remove? | Keep + document as dormant | mcoelho |
| D6 | Phase 4.8.2/4.8.3 daemon FS capability in 0.1.0? | Defer to 0.2.0; doc only | mcoelho |
| D7 | `find --pattern` glob or substring? | Glob (matches plan) | mcoelho |
| D8 | Shell completions: clap_complete generation OR README edit? | clap_complete generation (Option A) | mcoelho |
| D9 | Predictor: wire OR delete? | Wire (matches "harder, more correct" directive); fall back to delete if cushion is thin | mcoelho |

Once D1-D9 are decided, the "Suggested commit sequence" above is
fully unblocked.

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
**Last updated:** 2026-05-04 (v2.1)
**Next review:** after D1-D9 are decided, or before tagging 0.1.0.
