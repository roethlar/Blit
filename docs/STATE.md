# STATE — single entry point for "what is true right now"

Last updated: 2026-10-04 (cr-win-1 fixed `501c408d`; drift sweep 2026-10-04). **CI FULLY GREEN on `189ae11b` (run 37570469240 + Docs Gate 37570469250, 2026-10-07): Check & Format, all three test legs (Windows x86_64 1338 passed), all three release builds (macOS notarized; Windows unsigned, see below).** Earlier green: `bc9fa381` (run 37228870627 attempt 4) after the owner accepted Apple's updated agreement (notary HTTP 403 on attempt 1). Earlier, CI's runner picked up rust 1.98.0, whose new `clippy::chunks_exact_to_as_chunks` lint redded `20d6e51f`; mechanical fix in `crates/blit-core/src/checksum.rs` landed as `2fa0eccd` (`[state: skip]`), proven by GitHub CI run 32425865763 (all 7 jobs) + Docs Gate 32425865667, plus local `cargo +1.98.0 clippy --workspace --all-targets -- -D warnings` clean. **PERF_HISTORY_PLANNING ph-6 rig proof EXECUTED 2026-08-20** — cold-vs-warm A/B both directions magneto↔skippy, clean reruns (first pass discarded: build contamination), evidence in `docs/bench/PERF_HISTORY_PLANNING/`: warm == cold within noise (no regression), seeds persist/reuse/settle (fwd 4, rev 1), merged `[daemon]`/`[operator]` reporting verified live. Win ≈ 0 on this rig (dial settles in first epochs even cold). **Owner ruled PASS (D-2026-08-20-4): plan Shipped, D-2026-08-20-1 release-tag block CLEARED** — no perf-history release blocker remains. **NOW: v0.1.3 IS RELEASED (2026-08-20).** Tag `v0.1.3` → `fe6b279d` (lightweight, matching v0.1.2 practice); tag-push CI run 32433036504 fully green (all 9 jobs: Check & Format, 3 test legs, 3 signed release builds, Publish release, Update package managers); non-draft GitHub release `Blit v0.1.3` has all six assets in one atomic attach and the bot channels (brew/scoop/AUR/winget) were re-dispatched by the green package-managers job. Workspace version 0.1.2→0.1.3, `CHANGELOG.md` `[0.1.3]` covers all work since `v0.1.2`. crates.io publish of the 0.1.3 crates DROPPED (D-2026-10-07-3: ship nothing until the open work is done; crates.io max stays 0.1.2). (Previous: v0.1.2 released 2026-08-08, D-2026-08-08-1.) Tag `v0.1.2` → `98084edf` (lightweight — v0.1.1 was annotated; left as-is on the owner's call); the public GitHub release `Blit v0.1.2` is non-draft with all six assets (3 archives + 3 `.sha256` sidecars) attached atomically by the `publish-release` job on tag-push CI run 31235325733 (green) — the atomic lane's first real-tag exercise. Owner ruled "blit 0.1.2" over 1.0.0, so the workspace version reverted `1.0.0` → `0.1.2` (`0c04db73`) and `CHANGELOG.md` `[0.1.2]` (`a63cb3fa`) states the real scope of all 177 commits since `v0.1.1` — version LABEL only, no code reverted. `docs/plan/RELEASE_1_0.md` stays **Active with G2–G6 open — deferred, not closed**; a future v1.0.0 closes them on its own candidate, `ef9a13b2`'s `1.0.0` label and its `dist/` archives are now historical pipeline proof only, and audit-18/19 shipped documented in 0.1.2. Local gate green on macOS (fmt, clippy native + linux-cross, 1854 tests, smoke, docs); CI is green at the tagged head (CI run 31235325444 + Docs Gate 31235325447 on `master`, tag run 31235325733, as of `98084edf`). NEXT: `docs/plan/INTERFACE_PLATFORM.md` (**Shipped** 2026-08-18, fully ruled D-2026-08-17-1..-5) — **ALL SEVEN SLICES LANDED 2026-08-17** (commits in the plan's execution record): the workspace is the target three crates — **blit-core (the platform crate, crates.io-publishable, proto now inside it), blit-cli, blit-daemon**; gui/tui/bridge deleted (−13/−678/−20 tests, every delta reconciled), final suite 1162/0/2 on macOS + linux cross-clippy. Also 2026-08-17: AI/LLM references excised from all app surfaces (help string, ~150 review-provenance comments, 4 LLM-tooling scripts deleted; exclusions + grep proof in DEVLOG 05:30Z). **CI FULLY GREEN on head `086823f7` (run 32089671635, all 7 jobs, 2026-08-18)** — the sf-3c Windows red was a broken test assertion (`.seconds()` is 1601-based on Windows; fixed, product untouched; DEVLOG 2026-08-18 00:50Z), and one Windows release-smoke WinError-32 teardown flake passed clean on rerun. **INTERFACE_PLATFORM is SHIPPED (owner declared 2026-08-18).** **BLIT_CONSOLE IS DEAD (D-2026-08-15-1): Blit is CLI + daemon only — no TUI, no GUI in this repo; UIs live in `http://magneto:3001/michael/BlitAdmin_UIs.git`. 1.0 gates on CLI + daemon only (D-2026-08-15-2; G5c dissolved).** G5b's cross-OS smoke matrix is still unrun.
CI `build-release` signs the shipped binaries when the signing secrets are present (macOS codesign + notarize, unstapled by design) and packages unsigned when they are not; **Windows ships UNSIGNED from 2026-10-04**: the owner cancelled the Azure subscription (Trusted Signing signs per build, so it cannot sign without one; 403 Forbidden on `fe172fbd`/`bc9fa381`) and the six `AZURE_*` repo secrets were deleted on the owner's go; the signed path is proven green end to end (run 31229611237, DEVLOG 2026-08-08). A `v*` tag push runs that same gate, and **as of `9c193968` a single downstream `publish-release` job — not the matrix legs — attaches all six files at once**, so the release is atomic: every platform leg succeeds and it publishes complete, or one fails and no release is touched (the earlier per-leg attach could publish a partial release; DEVLOG 2026-08-08 01:35Z). It is non-draft and titled `Blit <tag>`, matching `v0.1.1`, which API-verified 2026-08-08 is the ONLY release this repo has ever published (`v0.1.0` is a bare tag with no release object; the "both published directly" claim is corrected in DEVLOG 2026-08-08 01:20Z). No longer a by-hand upload — exercised for real on v0.1.2 (run 31235325733: non-draft `Blit v0.1.2`, all six assets in one attach; DEVLOG 2026-08-08 03:16Z).

- **BLIT 0.1.1 IS RELEASED (D-2026-07-23-8):** annotated tag `v0.1.1`
  resolves to exact validated candidate `d1f1152d` on LAN Gitea and GitHub.
  The public GitHub release has all three validated archives and checksum
  sidecars. Canonical hashes and scope: `docs/RELEASE_READINESS.md`.
- **ONE TRANSFER PATH IS PROVED.** There is one `Transfer` RPC. When the caller is DESTINATION, it connects to the SOURCE daemon; that daemon sends through the same SOURCE pipeline. Push/pull-facing adapters only select roles. The connection initiator still opens sockets to the responder for NAT/firewall reachability; that topology does not select byte logic or worker policy.
- **ADAPTIVE ROLE PARITY IS ACCEPTED IN ldt-2.** Deterministic real-session traces in both socket layouts emit identical ADD epochs through 17, REMOVE 4→1, idle/hysteresis holds, and receiver bounds. The old exact-eight result remains historical static-policy evidence, not an adaptive target.
- **ldt-4 EVIDENCE IS FINAL FOR RELEASE** and no policy change follows: the
  live controller resized, but fixed order confounds the Windows→q ADD/REMOVE
  split with source warmth. Session ledger and write cost:
  `docs/bench/ldt4-evidence-audit-2026-07-22/`.
- **RIG-W HOST AND QUIETNESS RULES:** `.agents/machines.md` is canonical. ldt-4 must establish quietness live on `q` and `netwatch-01`; recorded readiness is never substituted for the run gate.
- **P1 IS CLOSED WITHOUT ANOTHER TRANSFER (D-2026-07-22-2).** The failing
  builds used the old-red worker path: its deterministic guard settled SOURCE
  initiation at 3 workers and DESTINATION initiation at 2, while a second
  destination-only zero-capacity branch could cap at 1.
  `a76b785..42b9b38` fixed and mutation-proved parity;
  post-fix `8e019ef` passed the target point bar, and ldt-2 retains adaptive
  role parity. Evidence: `docs/bench/p1-evidence-reconciliation-2026-07-22/`.
- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**

Rules: this file outranks every doc below it in AGENTS.md's Source of Truth order (only the human request, AGENTS.md and `.agents/repo-guidance.md` sit above it). Keep it ≤ 200 lines and ≤ 3 handoff entries — prune into `DEVLOG.md`. Update it via the `handoff` procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.

## Handoff — 2026-10-01 (HEAD `2cfcb2fd`)
- Done: SOURCE_SIDE_CONTAINMENT slices + 4 codex fix batches; Windows fixes win-1..5 green on the ARM64 VM (all 43 binaries; restricted token for one) + Mac 1344/0/2; Windows VM recorded in `.agents/machines.md`.
- In flight: nothing. Open: cr-win-1 HIGH (`.review/findings/cr-win-1.md`); owner ruling D8 (plan §Open questions); owner offers an x86_64 Windows box (Ryzen 9) in the morning.
- First action: fix cr-win-1 (retry passes compare with IgnoreTimes; guard red→green), Mac gate + ARM VM run; then set up the Ryzen box per the machines note and run `--no-fail-fast` there; then ask the owner for `/git push all` and watch CI.

## Now (active work)

- **SOURCE_SIDE_CONTAINMENT SHIPPED (owner declared 2026-10-07, D-2026-10-07-1):** all six slices
  landed 2026-09-30 (`docs/plan/SOURCE_SIDE_CONTAINMENT.md`; a file the
  SOURCE cannot deliver is skipped or retracted and reported, never
  run-fatal; contract 7). Windows fixes win-1..5 `5eeff4ac..8da5614b`
  (stack overflow, scan-time sharing violation, push compare regression)
  are green on the Windows ARM64 VM, the Mac (1344/0/2 as of `8da5614b`)
  and CI's three test legs as of `fe172fbd` (run 36812553117). Codex
  review of win-1..5 admitted **cr-win-1 HIGH, reworked `0528e78c`+`4a06aec5`**
  (a failed write never looks finished; retries compare like the main pass; D8 per file `85e82f84`); codex review admitted cr-rework-1, fixed `75462ba9`; re-reviews admitted cr-rework-2 (`a60368b1`) and cr-rework-3 (`7c607672`); all four closed 2026-10-07 (owner), CI green at `189ae11b`. Owner
  notes: `--retry`/`--wait` vs `--retries`/`--retry-wait` naming;
  deletions after the main pass; `--detach` ignores `--retries`. Next
  release bumps at least the minor version (blit-core API).
- **ONE_TRANSFER_PATH ACTIVE (D-2026-07-05-1 directive, D-2026-07-05-4 "flip
  the plan and go").** The invariant (plan doc, verbatim): ONE block of
  transfer code; direction/initiator/verb can NEVER affect wall time by
  blit's doing — impossible by construction because the per-direction
  drivers and `Push`/`PullSync` are deleted at cutover. Slices otp-1..13; converge-up per cell (±10%);
  symmetric-fs disk-to-disk verdict cells. **D-2026-07-05-2:
  same-build peers only, refusal at session open.**
  Slice status, the closed-slice record, and the otp-12 worker-parity
  repair (historical static-policy proof, superseded as an adaptive target
  by ldt-2) live in Queue 2, `docs/history/state-archive.md` (the
  otp-1..11 record), DEVLOG, and `docs/bench/otp12*/` — not restated here.
- **SMALL_FILE_CEILING ACTIVE; sf-3b closed (D-2026-08-14-1), sf-3c
  landed 2026-08-15, sf-3d landed 2026-08-18** on the unified path. sf-3a
  named the cuts; sf-3c (descriptor-retained metadata stamping) removed the
  streamed finalize path's by-path reopen; sf-3d (candidate 3, contained-path
  canonicalization cache) closed the list. Rig A/B for sf-3d still owed; no
  further cut selected.
  Principle: ceiling-driven, never competitor-relative (D-2026-07-04-4).
## Queue (ordered)

0. **`docs/plan/JOB_LOGS.md` (DRAFT, complete 2026-10-07)** — logs on every machine involved (compressed JSON, read as text or JSON), local jobs with `--save`/`--export` and `blit jobs save|export|run|retry|log|list|delete`; owner rulings R1–R16; slices jl-1..jl-4. Codex openreview r1: acceptable_with_changes — 4 material changes await owner rulings, F1–F4 admitted (plan §Review history); then the owner's Active flip.
   SOURCE_SIDE_CONTAINMENT: SHIPPED (D-2026-10-07-1), CI green at `189ae11b`; met RELEASE_1_0 G3's fix-now item.
0. **ULTRACODE 2026-08-18 (DEVLOG 23:00Z): cv-1+cv-2 LANDED `05529c19` (contract-version gate + scan surfacing; cv-3 landed 2026-09-30 with ssc-5 `6c266dd4`); sf-3d LANDED `7ffd929d` (containment cache + resume-path stamping; rig A/B still owed); clp-3 reviewed AND F1 closed `677a8ba9` (F2/F5 stay owner calls); residue triaged (2 dead, 1 dead-as-written; item 5 rate-window FIXED — its stall-silence call was owner-ruled as revised-b, landed below; item 2 perf-history scope ruled "both" by D-2026-08-20-1 and shipped, D-2026-08-20-4); sweep-prefetch parked pending one netwatch-01 SMB run; pm-5: see Blocked / waiting.** CI FULLY GREEN on `af9b48b3` (run 32195891519, 7/7 jobs — the slices are cross-platform proven). Stall-line revised-b landed after (three red-proven silence gates; mid-payload stalls visible, summary-wait/purge tails silent).
1. **`docs/plan/INTERFACE_PLATFORM.md` (SHIPPED — owner declared 2026-08-18; all seven slices landed 2026-08-17)** — three standalone front-ends (CLI first, TUI/GUI later in BlitAdmin_UIs), all first-class consumers of `blit-core` ITSELF: `blit-app` + `blit-console-core` fold into core (no new crate, D-2026-08-17-2), `blit-core` publishes to crates.io (name available, verified; publish act owner-gated), third-party Rust apps embed via the crate, everything else via daemon gRPC. All rulings closed (D-2026-08-17-3/-4/-5: bridge, tui, gui all deleted; fresh UIs later; nothing pushes to BlitAdmin_UIs under this plan). Nothing open; execution record in the plan.
2. **`docs/plan/SMALL_FILE_CEILING.md` — sf-3b closed (D-2026-08-14-1); sf-3c landed 2026-08-15; sf-3d landed 2026-08-18** — sf-3c stamps streamed-receive mtime/permissions through the retained write handle instead of reopening by path; sf-3d (sf-3a candidate 3, contained-path canonicalization cache + resume-path stamping, `7ffd929d`) closed the named-candidate list; proxy pins + mutation proofs in the plan's Slices section. Rig A/B for sf-3d still owed (Queue 0); no further cut selected.
2. **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (ACTIVE, D-2026-07-31-4) —
   PRIORITY-1, the owner's ruling on the 2026-07-31 field check.** Local
   transfer is too slow; the owner ruled it "should have been resolved before
   this was declared release-worthy." Recorded 2026-07-13, shipped in 0.1.1
   anyway under D-2026-07-13-2's BEHIND sequencing, now superseded.
   **ls-1 step (0) IS RUN and attributed** — evidence and caveats in
   `docs/bench/ls1-phase-2026-07-31/`. A converged dry run of the owner's
   tree (46,041 files → SMB) reproduced their no-op at 273.57 s with **COMPARE
   at 100% of wall** and apply at ZERO, **falsifying L1–L4 here** (all
   apply-path).
   **SHIPPED, 273.57 s → 166.38 s (1.64×; 1.71× vs the owner's original
   283.92 s field run)**, in two parts. (a) Round-trip elimination: reuse the
   stat's attribute DWORD instead of re-reading it with `GetFileAttributesW`
   (metadata 3.653 → 2.556 ms/file). (b) **a DEDICATED comparison pool whose
   concurrency is discovered at runtime with NO user-facing flag**
   (D-2026-08-01-1; `AdaptiveCheckers` follows the `dial.rs` rule —
   conservative floor, no probe phase, one rung per chunk on measured
   throughput, settle on regression), landing within 0.4% of the best
   hand-tuned value on its own. **Correction: the earlier "destination
   saturates" conclusion was WRONG** — one datapoint on rayon's shared pool;
   8 dedicated threads beat 32 shared by 33 s. See `.../checkers.md`.
   **ls-4 SHIPPED: apply ran ONE worker; now 8 (`DEFAULT_SINK_WORKERS`).**
   The owner's local-to-local mirror re-opened what step (0) had closed —
   the bottleneck follows the DESTINATION, and on local NVMe
   **APPLY_BACKPRESSURE was 81.7% of wall** (L3, measured). Sweep: local
   17.68→6.58 s (2.69×, knee 4–8); **SMB completely flat with no penalty at
   16**, which is why this is a fixed default rather than the `--checkers`
   runtime treatment — nothing to discover, and an adaptive throttle on the
   WRITE path would be risk for no measured gain. Real tree: **17.68 → 8.70 s
   (2.03×)** on the shipped default. Evidence `.../apply-workers.md`.
   **ls-5 SHIPPED: directory-sweep destination stat, 166.38 → 114.73 s.**
   One `read_dir` per destination directory answers the diff's target
   resolutions (6,191 sweeps / 6,709 dirs, zero re-sweep waste); the
   per-file stat REMAINS authoritative for anything the sweep cannot judge
   exactly. Local NVMe A/B neutral. Evidence `.../dir-sweep.md`.
   **ls-6 SHIPPED (D-2026-08-01-4): stream interrogation deleted from the
   default compare, 114.73 → 55.57 s** (5.1× vs field complaint); remaining
   lever: sweep prefetch or wire-carrier concurrent-session measurement.
   **Review loop CLOSED: 4 rounds, 8 findings, all resolved, r4 CLEAN with
   `guard_confirmed: true`** — see `REVIEW.md`. **`ls-0` LANDED**: the
   summary separates copied from repaired files and labels the rate as a
   whole-run average. NOT the closed P1 defect (D-2026-07-22-2).
3. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4):**
   slices otp-1..13; any external review requires exact owner approval under
   D-2026-07-23-7. **otp-1 … otp-12c are all `[x]`** — closed-slice record and
   the otp-12a/b/c matrices live in `docs/history/state-archive.md`
   (otp-1..11), DEVLOG, the otp-12c row in `REVIEW.md`, and
   `docs/bench/otp12*/`; the one historical FAIL cell (wm_tcp_mixed) is
   closed by D-2026-07-22-2. **otp-12d and otp-13 are POST-RELEASE
   (D-2026-07-22-1)**; no performance acceptance matrix is a shipping
   prerequisite.
4. **`docs/plan/PER_FILE_ERROR_CONTAINMENT.md` — SHIPPED** (owner declared
   2026-08-01, D-2026-08-01-2; every acceptance criterion met, including the
   owner-only re-run convergence checkpoint — their run 2 copied 0 files,
   0 B on the originating `D:\Apps → H:\apps` workload). pfc-1..6 closed the
   root posture defect: a single survivable per-file error no longer aborts
   a transfer. audit-17 closed with it. 7 range reviews, 11 findings, 10
   verified-closed + 1 declined. Detail: the plan and DEVLOG 2026-07-31.
5. **`docs/plan/CLI_LIVE_PROGRESS.md` (ACTIVE, D-2026-07-31-2): clp-1, clp-2
   and clp-3 all `[x]` landed.** **clp-3** adds Dracula colour to the live
   row and the failure block (owner-chosen palette, scope "both"): phase word
   carries the colour, deleting is orange not red so red keeps its meaning
   for failures, counts and failed paths stay unstyled. Colour is applied
   AFTER layout — escapes are zero-width on screen but not in a `String` —
   and the plain forms are `#[cfg(test)]` references so production output
   cannot drift from them. `NO_COLOR` + TTY detection are the whole control
   surface; no flag (D-2026-08-01-1). Reviewed 2026-08-18: F1 closed `677a8ba9`, F2/F5 stay owner calls (Queue 0). clp-1/clp-2 gave one live row with truthful phases
   (enumerating → comparing/up-to-date → copying → deleting; dry runs
   never claim deletion), width-safe current-file segment, `-v` per-file
   lines through the row handle, and the `log` backend routed through
   the row while it lives (warns scroll above, never scroll it away).
   Owner confirmed on a real TTY (2026-08-01) that the row holds one line
   while `-v` lines scroll above it. Residue (d) (remote-route attachment)
   deferred to remote render work. TUI-scale redesign void (D-2026-08-15-1).
6. **POST-RELEASE performance declarations:** ue-1, ue-2, and the REV4
   performance status flip are not release gates (D-2026-07-22-1).
7. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
   **Standing owner safety rule for rig `zoey`**: `.agents/machines.md`.
8. **Post-REV4 residue** (unowned, 5 items) — list in DEVLOG 2026-07-13 21:00Z.
9. **`docs/plan/PACKAGE_MANAGER_DISTRIBUTION.md` (ACTIVE, D-2026-08-12-7)**
   — **pm-4 PROVEN; ALL BOT CHANNELS LIVE 2026-08-18**: brew tap (install-proven on arm64 macOS, exact identity, signature intact), scoop bucket, and AUR (`blit` + `blit-bin` 0.1.2-1, RPC-verified) are live; **winget LIVE — microsoft/winget-pkgs#420041 MERGED 2026-08-19** (`winget install Roethlar.Blit`); README Installation section lists the live channels (winget added 2026-08-20); cargo LIVE (D-2026-08-18-1: all three crates published 0.1.2, install proven, README advertises with the same-build caveat + BLIT_GIT_SHA pin recipe); homebrew-core (pm-5) not started. Detail: DEVLOG 2026-08-18 20:10Z/20:30Z.
10. **`docs/plan/PERF_HISTORY_PLANNING.md` (SHIPPED 2026-08-20, owner PASS — D-2026-08-20-4; the D-2026-08-20-1 release-tag block is CLEARED):** perf data feeds planning (warm-started dials from topology/role/initiator-tagged history, all routes recorded incl. daemon-served) AND reports honestly; ph-6 rig A/B proof in `docs/bench/PERF_HISTORY_PLANNING/`. Openreview 2026-08-20 (codex/gpt-5.6-sol): `acceptable_with_changes`, all material changes folded into the plan; rulings R1–R5 ALL RESOLVED by owner-delegated design (D-2026-08-20-2: retire predictor, fleet exposure deferred, rig A/B closes the plan, R4b post-open ramp/no wire change, R5b merged daemon-record reporting with origin labels). Current slice: **ph-1** (schema v3 + record every route + writer safety). Progress: ph-1a (schema v3 + writer safety) and ph-1b (client push/pull + coordinator + local route tags) landed; **ph-1c landed** — daemon-served and delegated-participant recording into the daemon's own store (`ServedSessionRecorder`, store injected via `BlitService::from_runtime`), plus a served-session close-honesty fix (terminal-summary instruments + 500ms hangup grace + destination-initiator graceful close; pre-fix every completed direct served pull was scored "client cancelled" and its summary died unflushed in the cancelled call — see plan §Design "Served-end recording + close honesty"). ph-1 recording matrix is now CLOSED: coordinator record landing test added (`coordinator_records_delegated_run_in_operator_store`, drives real `run_delegated_pull`, proven red by gating off the record), and the concurrency/migration guard audit found the criterion already pinned (`concurrent_appends_lose_no_records`, `history_len_guard_detects_concurrent_append`, atomic cap rotation, v0/v1/v2→v3 migration + lane derivation). **ph-2 landed** (honest reports, R5b): `blit diagnostics perf` merges operator + daemon stores with origin labels, per-route aggregates, fast-path/streaming split, planner/transfer averages (text + JSON); `docs/DAEMON_CONFIG.md` documents the daemon store; verified end-to-end against real recorded runs. **ph-3 landed** (seed store + R1 predictor retirement): `blit_core::seed_store` persists settled dials (`perf_seeds.json` v1, atomic, capped) keyed route|peer_key|workload-class, write gated on settled confidence + ≥100 files + Real lane + present peer_key; local close feeds the checker rung (`CheckerPool::settled_limit`); `perf_predictor.rs` deleted with its 21 tests (called out in DEVLOG; workspace now 1206/0, +8 seed tests). **ph-4 landed** (warm-start checkers): seed = measured one-chunk jump in `AdaptiveCheckers` (cold start unchanged; rejection returns the walk unsettled — unpinnable by construction), `lookup_route_latest`/`route_seed_user` read side, armed in `run_local_session`; poison recovery red-proven both directions (pinning fallback stub → fail → restore; corrupt store → cold start); pins outrank seeds; 1213/0. **ph-5 landed** (warm-start session workers, R4b wire-frozen): sender dial reads the route's settled workers seed at open and ramps toward it accelerated — acceleration only, never a pin (live controller retains full authority); seed's `workers` slot written at session close. Seeded coverage is PUSH (CLI is byte sender both ways of the seed); PULL stays cold-start (daemon is byte sender, `ResponderInstruments` predate `SessionOpen` so no route to arm — documented gap in plan ph-5, post-ph-6 candidate, not a release blocker). Gates: fmt clean, native + Linux-cross clippy clean at `-D warnings`, full suite green on macOS. **ph-6 executed and owner-ruled PASS 2026-08-20** — the plan is closed (evidence + honest-zero-win reading in D-2026-08-20-4). Side-fix from the ph-5 smoke: daemon-address-shaped LOCAL destinations (`host:port//path`) now warn on stderr with the remote forms and `./` escape hatch — classification unchanged (D-2026-08-20-3).

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — D-2026-07-05-4);**
  `docs/plan/OTP7_RESUME.md` (**Active**, D-2026-07-09-1 — otp-7 slice design).
  **`docs/plan/BLIT_CONSOLE.md` (Superseded, D-2026-08-15-1)** → `docs/plan/UI_REMOVAL.md` (**Superseded**, D-2026-08-17-1) → `docs/plan/INTERFACE_PLATFORM.md` (**Shipped** 2026-08-18, fully ruled D-2026-08-17-1..-5 — blit-core as the platform crate + out-of-repo front-ends).
  **`docs/plan/PER_FILE_ERROR_CONTAINMENT.md` (SHIPPED, D-2026-08-01-2).**
- Shipped release record: **`docs/RELEASE_READINESS.md`** and
  **`docs/plan/RELEASE_COMPLETION.md`**.
- Historical live-tuning record: **`docs/plan/LIVE_DIAL_TUNING.md`**; exact
  session audit: **`docs/bench/ldt4-evidence-audit-2026-07-22/`**.
- **`docs/plan/SOURCE_SIDE_CONTAINMENT.md` (SHIPPED 2026-10-07, D-2026-10-07-1 —
  all six slices landed 2026-09-30; cr-win-1 and its review findings closed 2026-10-07).**
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**Active**; sf-3c/sf-3d
  landed, no further cut selected) and **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** (code-
  complete; measurement gates remain). REV4 superseded v1/REV2/REV3
  (history only).
- Process: `.agents/playbooks/openreview.md` — synchronous unprimed review only
  after exact owner approval under D-2026-07-23-7 (reviews external models only,
  formal review dispatches Codex, D-2026-10-07-4; Grok advisory; landed-slice codex dispatch is standing under
  D-2026-07-31-3); `.agents/playbooks/codereview.md` supplies finding intake
  and triage only. `docs/agent/GPT_REVIEW_LOOP.md` is historical;
  `.review/README.md` is retired as the grading mechanism (its
  `findings/`/`results/` records and the REVIEW.md index remain live).
- Review loop: `REVIEW.md` (no open rows; cr-win-1 + cr-rework-1..3 closed 2026-10-07) + `.review/findings|results/`.
- **`docs/plan/PACKAGE_MANAGER_DISTRIBUTION.md` (ACTIVE, D-2026-08-12-7).**
- Other plans: `ZERO_COPY_RECEIVE_EVAL.md` (module delete ratified
  D-2026-06-12-1, executes w8-1; **capability unparked D-2026-07-05-3** —
  post-cutover write strategy), `TUI_REWORK.md` (**Superseded**, D-2026-08-02-2),
  `BENCHMARK_10GBE_PLAN.md` (Historical; env note lives in the queue).

## Blocked / waiting (owner declarations and explicitly dated external blockers; checkpoints are owner-only)

- **Rig facts:** `.agents/machines.md` is canonical; no host pairings here.
- **Two stale test firewall entries await separately approved cleanup** (helper shipped, entries untouched; re-verified present 2026-10-04); paths + no-reuse/no-removal gate: `.agents/machines.md`.
- **pm-5 (homebrew-core): HELD by the owner until everything works 100% (D-2026-10-04-2).** Its technical block cleared (`v0.1.3` `fe6b279d` carries pm-1 `042d79ad`); `docs/plan/PACKAGE_MANAGER_DISTRIBUTION.md` pm-5.

## Open questions

- **Defects found 2026-10-07:** (a) the local copy's cleanup armed before the destination was opened — FIXED `bbc75dec`; (b) Ctrl-C during the retries lost the report — FIXED `b097aec0` (exit 130, report kept); (c) a `--detach` job hides per-file failures — `docs/plan/JOB_LOGS.md` slice jl-1.
- G3 (RELEASE_1_0) still open: audit-19 (`--exclude` matches nothing for absolute paths and bare directory names) — fix or document before 1.0.
