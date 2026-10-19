# STATE — single entry point for "what is true right now"

Last updated: 2026-10-19 — D-2026-10-19-2 makes every known imperfection a 1.0 release blocker. O4's shared filtered-subset predicate is CI-green on all three OSes (`5386b391`). O3's ASCII paired-spelling guard passed on macOS, failed when pruned recording was disabled, and passed again restored; fmt, native and Linux cross-target Clippy, and workspace tests passed (1560/0/2 over 47 binaries). Windows CI on the O3 commit is pending. O1's 200,000-file excluded fixture measured 0.46 s and 35 MB peak versus 0.33 s and 16 MB with pruned recording disabled. O1–O2 and the previously listed release limitations remain open. No v1.0.0 tag exists.
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

## Handoff — 2026-10-19 (G3b release blockers)
- Done: g3fix6-r1's three findings fixed one per commit: `882daa41` (source-pruned paths through contract 8), `9bec0bf3` (glob text as written), `0767b17e` (bounded recursive-crossing states). Each has a mutation-red guard; targeted local and daemon exclude tests, native and Linux cross-clippy passed. G5b remains closed on six passing legs.
- Done gate: macOS fmt, native and Linux cross-clippy clean; full workspace tests 1560 passed / 0 failed / 2 ignored over 47 binaries on candidate code `0767b17e`. The first test run hit a 20-minute session limit; the second completed with a one-hour limit. Windows cross-clippy on this Mac stops before project code in `blake3` because `ml64.exe` is unavailable. On 2026-10-19, the unchanged code built in release mode on macOS; both binaries reported `1.0.0+c93453bddc94`, the advisory scan was clean and blit-core 1.0.0 packaged and verified. Release docs now distinguish tag CI from separate crates.io publishes. CORRECTION 2026-10-19: the fixes and the docs after them ARE published — `git ls-remote` puts `origin` at `913d8328`, equal to local `master`; CI run 39298379677 on `11664b54` is green on all three OS legs.
- First action: O4 is CI-green; O3's guard and macOS gate passed. Push O3 and require its Windows CI test to pass, then resolve O2/O1 scaling and audit the known release limitations. No tag or publish.

## Now (active work)

- **RELEASE_1_0 IS THE WORK (owner goal 2026-10-14: "do everything necessary to
  get this ready for 1.0"; `docs/plan/RELEASE_1_0.md`, Active since 2026-08-01).**
  Readiness table and the ordered path to the tag: the plan's "Readiness as of
  2026-10-14". Done 2026-10-14: the **candidate cut `6d5d93c8`** (1.0.0,
  `CHANGELOG.md` `[1.0.0]`, gate green 1483/0/2), `docs/RELEASE_NOTES_1_0.md`
  re-cut 0.1.3 → 1.0.0 († marks a limitation awaiting a G3 ruling), G4 truth
  pass `6ac98801`, G5b Mac↔Linux PASS `aaa8af55`, audit-18 retired; then on
  D-2026-10-14-1: audit-19 fixed `fa948e01`, the detached recents reason fixed
  `b7da0d81`, the sweep and the JOB_LOGS ticks `b186e8b8`; Codex g3fix-r1: 3 MEDIUM
  on audit-19 fixed `7d3e4eb6` `47bdfb69` `eddfab6c`; pushed through `d9adb99e`, CI
  green (run 38791000789); g3fix2-r1: 2 more (HIGH symlink case under `mirror`,
  unescaped disk names) fixed `f6099aa3` `36b752b6`; g3fix3-r1: 3 more (HIGH Unicode-equivalent symlink spellings under `mirror`, escaped text matching a real name, a Windows peer rewriting `[\]`) fixed `a67087fd` `f6212938` `548625f5`; g3fix4-r1: 4 more (HIGH a hard-linked symlink / the Windows fold under `mirror`, HIGH the destination's own spelling under `mirror`, HIGH typed Unix escapes on a Windows peer, MEDIUM a class's `\` in the absolute split) fixed `e7be4970` `b304e6ff` `5624fd43` `e67c225c`; gate 1533/0/2; pushed through `de9a333f` 2026-10-16 (from this clone, not by this session), CI fully green (run 38960754619: Linux 1542, Windows 1487, macOS 1533); G5b Mac↔Linux PASS on its archives `50e4d5fc`; lockfile bump for RUSTSEC-2026-0258 (h2) and the yanked chacha20 `3f15f835`; CI fully green on `6b334db6` and `d1037c73`; g3fix5-r1: 4 more (HIGH Unicode-equivalent destination names under `mirror`, HIGH glob components under `mirror`, MEDIUM `**\/` broadened, MEDIUM a literal Windows root) fixed `80c6072e` `1435d7c7` `7f4e2116` `df61b1d6`; gate 1548/0/2; CI green on `19ace1ac`; g3fix6-r1: 3 more (HIGH a folded class loses members, HIGH Unicode pairs a fold misses, MEDIUM quadratic `**`+glob) — fix round 7 landed locally: 882daa41 (pruned paths, contract 8), 9bec0bf3 (typed globs), 0767b17e (bounded recursive states); full macOS gate green 1560/0/2 over 47 binaries; **G5b CLOSED 2026-10-18, six legs PASS**. Next: G3b release blockers (D-2026-10-19-2); start with O1.
  Further review was cancelled by the owner (D-2026-10-19-1); the firewall recipe finding is fixed in docs `d0a9a7a8` `215de5e2`. O4 is CI-green on `5386b391`; O3's paired-spelling guard is macOS mutation-red and full-gate green, with Windows CI pending after push. Next O2/O1, then the release limitations audit.
- **SOURCE_SIDE_CONTAINMENT SHIPPED (D-2026-10-07-1):** six slices, win-1..5, the
  cr-win-1 rework and D8 landed and reviewed; CI green `189ae11b`; detail in the plan, `CHANGELOG.md` `[1.0.0]`, DEVLOG.
- **ONE_TRANSFER_PATH SHIPPED 2026-10-14 (D-2026-10-14-1; D-2026-07-05-1 directive, D-2026-07-05-4 "flip
  the plan and go").** The invariant (plan doc, verbatim): ONE block of
  transfer code; direction/initiator/verb can NEVER affect wall time by
  blit's doing — impossible by construction because the per-direction
  drivers and `Push`/`PullSync` are deleted at cutover. Slices otp-1..13;
  converge-up per cell (±10%); symmetric-fs verdict cells. **D-2026-07-05-2:
  same-build peers only, refusal at session open.** Slice status, the
  closed-slice record and the otp-12 worker-parity repair (historical, superseded
  by ldt-2) live in Queue 2, `docs/history/state-archive.md`, DEVLOG, `docs/bench/otp12*/`.
- **SMALL_FILE_CEILING ACTIVE; sf-3b closed (D-2026-08-14-1), sf-3c
  landed 2026-08-15, sf-3d landed 2026-08-18** on the unified path. sf-3a
  named the cuts; sf-3c (descriptor-retained metadata stamping) removed the
  streamed finalize path's by-path reopen; sf-3d (candidate 3, contained-path
  canonicalization cache) closed the list. Rig A/B for sf-3d still owed; no
  further cut selected. Principle: ceiling-driven, never competitor-relative (D-2026-07-04-4).
## Queue (ordered)

0. **`docs/plan/JOB_LOGS.md` (ACTIVE, D-2026-10-07-5)** — logs on every machine involved (compressed JSON, read as text or JSON), local jobs with `--save`/`--export` and `blit jobs save|export|run|retry|log|list|delete`; owner rulings R1–R16; slices jl-1..jl-4. Codex openreview r1: acceptable_with_changes — all 4 material changes adopted (R17), F1–F4 folded in; slices re-cut jl-1a..jl-4. **jl-1a LANDED `c4440c54`** (`blit_core::job_log`: schema, crash-safe writer, recovery, pruning; pushed `f6ca55c0`, CI fully green run 37697776109). **jl-1b LANDED `51da3977`** (every daemon job writes its log — served push/pull from open, delegated pull from dispatch; engine names each file landed/failed/deleted via new `FileFailed`/`Deleted` progress events; logs in `<state dir>/jobs/logs`; `[jobs] keep`; contract-7 `GetJobLog`; `blit jobs log <host> <id> [--json] [--role]`; Mac gate green, 1410/0/2; known gaps in the plan). **jl-1c LANDED `f78008b7`** (contract-7 `files_failed` on `TransferRecord`/`TransferComplete`; `jobs list` shows the count; `jobs watch` exits 1 and names the failed files from the job's log — defect (c) closed in code, the acceptance case tested end to end on macOS; 1411/0/2). **Owner goal 2026-10-07: every slice reviewed by Codex to consensus, then pushed.** Reviews jl1a/jl1b/jl1c-r1 all acceptable_with_changes; 5 findings admitted and fixed one per commit (`07e2a3b4`, `91c97e61`, `c55a4d94`, `6e514464`, `86300bc5`; REVIEW.md "JOB_LOGS slice reviews"); fix-range re-review jlfix1-r1c (third dispatch; two stalled on this machine's IPv6 address churn) acceptable_with_changes → 2 more fixed (`7f7641f1`, `c4300a77`+`27a626a7`); second re-review jlfix2-r1 acceptable_with_changes → 3 more fixed (`2e193ba9`, `1add8d7b`, `b580f0c1`); third re-review jlfix3-r1c **best_approach** (consensus through `744c3708`; two earlier dispatches stalled on en11's IPv6 address churn, re-dispatched once en11 was down). **jl-2 LANDED 2026-10-08** (commit "Land JOB_LOGS jl-2: one run ID everywhere, local logs"; run ID on the wire in `SessionOpen`/`DelegatedPullRequest`, every command logs in the per-user folder, local `blit jobs list`/`jobs log <id|file>`, Windows folder → `%LOCALAPPDATA%\Blit`, CLI `config.toml` `[jobs] keep`; plan's landed note); Codex review jl2-r1 acceptable_with_changes → cr-jl2-1 (a dry-run or `--null` run was logged as copying) fixed `58ac16fb`; re-review jl2fix1-r1 acceptable_with_changes → cr-jl2fix1-1 (a dry run's projected bytes came from the bytes written) fixed `a7365b61`; second fix re-review jl2fix2-r1 **best_approach** — jl-2 has consensus through `78cf84d3`. **jl-3a LANDED 2026-10-08** (commit "Land JOB_LOGS jl-3a: …": every run's JobSpec and RunRecord in `<per-user folder>/jobs/runs/`, detached runs settled from their daemon, `jobs list` here from the records; plan's landed note); Codex review jl3a-r1 acceptable_with_changes → cr-jl3a-1 HIGH (a detached run settled from an incomplete daemon log) `71155351`, cr-jl3a-2 (one shared staging file per record) `a53e3d56`, cr-jl3a-3 (a record dropped non-UTF-8 names' bytes) `31b54c99`; re-review jl3afix1-r1 acceptable_with_changes → cr-jl3afix1-1 (a lossy-name collision gave the failed file the first entry's bytes) fixed `59439838`; re-review jl3afix2-r1 → cr-jl3afix2-1 (a failure seen live hid another of its text at close) fixed `9c4a9643`; third fix re-review jl3afix3-r1 **best_approach** — jl-3a has consensus through `778539f6`. **jl-3b LANDED 2026-10-08** (commit "Land JOB_LOGS jl-3b: …": `JobFile` (`blit-job` v1), saved jobs in `jobs/saved/`, `--save`/`--export`, `jobs save|export|run|delete`, `jobs list` adds saved jobs; plan's landed note); Codex review jl3b-r1 acceptable_with_changes → cr-jl3b-1 (a file in the current folder shadowed a saved job of its name) fixed `84aa5bf5`; re-review jl3bfix1-r1 → cr-jl3bfix1-1 LOW (a run-ID-shaped name refused without saying why) fixed `8d62c25e`; second fix re-review jl3bfix2-r1 **best_approach** — jl-3b has consensus through `51c9e4c6`. **jl-4 LANDED 2026-10-08** (commit "Land JOB_LOGS jl-4: …": `blit jobs retry <job-id|file>` — a new run, the next attempt with its parent, sending exactly the failed files as a copy, or the whole move again; refusing another machine, a run still going or waiting, a run whose failures are not all named); Codex review jl4-r1 acceptable_with_changes → cr-jl4-1 HIGH (`--ignore-existing` retry skipped its own incomplete copies) `7e935b64`, cr-jl4-2 HIGH (retry named non-UTF-8 files by text; contract 7 `FilterSpec.files_from_raw`) `e05188f8`, cr-jl4-3 (cut-short list completed with failures a pass had fixed) `0d8f43dc`; re-review jl4fix1-r1 → cr-jl4fix1-1 (detached settling kept fixed failures) `6ecc86c2`, cr-jl4fix1-2 HIGH (Windows dropped raw retry names) `27aff7d2`; second fix re-review jl4fix2-r1 → cr-jl4fix2-1 HIGH (left-in-place status keyed by text alone; two names of one text shared it) fixed `83a24929`; re-review jl4fix3-r1 (also covering the FAST commit) → cr-jl4fix3-1 (a record from before the raw list could treat a leftover raw-named file as not left in place) fixed `edf19ffc`; re-review jl4fix4-r1 → cr-jl4fix4-1 (an ambiguous legacy leftover was never settled from a complete log) `03d878a9`, cr-jl4fix4-2 LOW (a raw-list-era record read as text-only) `2f2cd1ad`; re-review jl4fix5-r1 → cr-jl4fix5-1 HIGH (a source log could erase a destination log's leftover evidence in the retry's fold) `3ac1f5b1`; re-review jl4fix6-r1 → cr-jl4fix6-1 HIGH (a later attempt could not clear a stale leftover of its own stream) `23bf5724`; seventh fix re-review jl4fix7-r1 **best_approach** — jl-4 has consensus through `cee9104e`. **Every JOB_LOGS slice (jl-1a, 1b, 1c, 2, 3a, 3b, 4) and every fix is reviewed to consensus (owner goal 2026-10-07).** **The whole range is on origin and github as of `90b8a1cd` (2026-10-09). Its first CI pass (run 37964272716) was red on the Linux and Windows test legs only — one over-specified test each, product untouched: the served-mirror log test pinned the readdir order of two stale files (fixed: the stale files are compared sorted, the folder still after its contents; proven on magneto by a mutation dropping one deletion), and the Windows spec-path test expected a typed `/` rewritten to `\\` where `absolute_local` keeps the separator as typed, which the resolver reads the same (fixed: the test expects the typed separator and pins the typed-`\\` case; its green is CI's to show — the Windows VM was unreachable). Check & Format, macOS, Docs Gate were green. The fixes (`8eb812dc`, `e6e98058`, run 37978315998) turned Linux green and let the Windows leg reach blit-cli's integration tests for the first time, where one more over-specified test failed: the jobs sub-verb help test expected `Usage: blit jobs` and Windows prints `blit.exe`; fixed (the test looks for the sub-verb, not the binary). That fix, `818dc2a9`, is FULLY GREEN (run 38002888998: all three test legs and all three release builds; Docs Gate 38002888906) — the whole JOB_LOGS range is proven on Linux, Windows and macOS.** What is the owner's: (1) the plan's acceptance criteria — each has an end-to-end test on macOS (not ticked by the agent; Linux/Windows legs run in CI on push), FAST is measured as reported, the Windows folder move is tested by its platform-neutral half only; (2) the review worktrees under the session scratchpad (`git worktree list`) — removal is a hook-gated action. **FAST**: logging had added ~50 ms per command (`scripts/bench_otp11_local_ab.sh`, old `f6ca55c0` vs `2dab5d21`: 1 GiB clone 21→73 ms, tree 29→79, no-op 25→81, 10k small 1235→1302) — about ten `F_FULLFSYNC`s in job logs and records; fixed `c3fde862` (plain `fsync(2)` on Apple, every flush point kept): 21→25, 29→31, 1235→1223, 24→26 (`docs/bench/job-logs-fast-2026-10-08/`). The remaining constant 2–4 ms per command still failed the 1 GiB clone cell; owner ruling 2026-10-13: a measurable per-command cost from logging violates FAST and is neither accepted nor benched around (nothing new to record — FAST already rules). **FIXED by jl-5 `031e0352`** (plan slice 7; built by an Opus agent on a self-contained brief): the start touches no disk (`LogWriter::start_deferred`/`RunLog::start_with`, a `blit-job-record` thread), the log and the record close concurrently (`prepare_close` + `ClosingLog::finish`), record writes buffered (same bytes), pruning reads only the records beyond `keep` (ordered by last write, as logs are); every flush point kept; five mutation-proven guards; 1479/0/2 on this Mac, Linux-cross clippy clean. Bench, old `f6ca55c0` (source proven) vs the clean `031e0352` build, verified independently of the agent: huge 21/21, tree 28/29, small 1267/1275, noop 27/27 — all PASS (`docs/bench/job-logs-fast-2026-10-13/`); ~0.7 ms per command remains by direct measurement. **Codex review jl5-r1 (owner-directed `review codex astra xhigh` → `gpt-6-astra` @ xhigh, codereview defect hunt over `df5fd1cf..031e0352`): findings — capability_ok, guard_confirmed, SHAs pin-verified — both admitted and fixed one per commit: cr-jl5-1 MEDIUM (the record thread read `--files-from` while the transfer could overwrite it; now captured once before the transfer starts, only when a list is given) `7e188159`, cr-jl5-2 LOW (`started_ms` stamped on the deferred thread; now the command's start) `75864cb0`; gate green here at `75864cb0` (1483/0/2, Linux-cross clippy clean). Re-review jl5fix1-r1 over `7cb47b4c..75864cb0` (the owner's `review`, same pair): **clean**, both findings closed, guard_confirmed (both guards mutated red/green by the reviewer), capability_ok, SHAs pin-verified — **jl-5 is reviewed to consensus through `75864cb0`: every JOB_LOGS slice (jl-1a..jl-5) and every fix is landed and reviewed. Pushed to origin and github as `8844dfbd` (2026-10-13, owner's `/git push all`); CI FULLY GREEN on `8844dfbd` (run 38474595246: all three test legs, all three release builds; Docs Gate 38474595174) — jl-5 and its fixes are proven on Linux, Windows and macOS.** Open, unscheduled: a delegated job's recents record says only "delegated_pull handler failed" (its real reason reaches the log only) — owner call whether to fix it now.
   SOURCE_SIDE_CONTAINMENT: SHIPPED (D-2026-10-07-1), CI green at `189ae11b`; met RELEASE_1_0 G3's fix-now item.
0. **ULTRACODE 2026-08-18 (DEVLOG 23:00Z): cv-1+cv-2 LANDED `05529c19` (contract-version gate + scan surfacing; cv-3 landed 2026-09-30 with ssc-5 `6c266dd4`); sf-3d LANDED `7ffd929d` (containment cache + resume-path stamping; rig A/B still owed); clp-3 reviewed AND F1 closed `677a8ba9` (F2/F5 stay owner calls); residue triaged (2 dead, 1 dead-as-written; item 5 rate-window FIXED — its stall-silence call was owner-ruled as revised-b, landed below; item 2 perf-history scope ruled "both" by D-2026-08-20-1 and shipped, D-2026-08-20-4); sweep-prefetch parked pending one netwatch-01 SMB run; pm-5: see Blocked / waiting.** CI FULLY GREEN on `af9b48b3` (run 32195891519, 7/7 jobs — the slices are cross-platform proven). Stall-line revised-b landed after (three red-proven silence gates; mid-payload stalls visible, summary-wait/purge tails silent).
1. **`docs/plan/INTERFACE_PLATFORM.md` (SHIPPED — owner declared 2026-08-18; all seven slices landed 2026-08-17)** — three standalone front-ends (CLI first, TUI/GUI later in BlitAdmin_UIs), all first-class consumers of `blit-core` ITSELF: `blit-app` + `blit-console-core` fold into core (no new crate, D-2026-08-17-2), `blit-core` publishes to crates.io (name available, verified; publish act owner-gated), third-party Rust apps embed via the crate, everything else via daemon gRPC. All rulings closed (D-2026-08-17-3/-4/-5: bridge, tui, gui all deleted; fresh UIs later; nothing pushes to BlitAdmin_UIs under this plan). Nothing open; execution record in the plan.
2. **`docs/plan/SMALL_FILE_CEILING.md` — sf-3b closed (D-2026-08-14-1); sf-3c landed 2026-08-15; sf-3d landed 2026-08-18** — sf-3c stamps streamed-receive mtime/permissions through the retained write handle instead of reopening by path; sf-3d (sf-3a candidate 3, contained-path canonicalization cache + resume-path stamping, `7ffd929d`) closed the named-candidate list; proxy pins + mutation proofs in the plan's Slices section. Rig A/B for sf-3d still owed (Queue 0); no further cut selected.
2. **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (SHIPPED 2026-10-14, D-2026-10-14-1; was Active D-2026-07-31-4) —
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
3. **`docs/plan/ONE_TRANSFER_PATH.md` (SHIPPED 2026-10-14, D-2026-10-14-1):**
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
5. **`docs/plan/CLI_LIVE_PROGRESS.md` (SHIPPED 2026-10-14, D-2026-10-14-1): clp-1, clp-2
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

- **`docs/plan/ONE_TRANSFER_PATH.md` (SHIPPED 2026-10-14, D-2026-10-14-1)**, with
  `OTP7_RESUME.md`, `OTP11_LOCAL_SESSION.md` (**Shipped**) and `OTP12_ACCEPTANCE_RUN.md` (**Historical**), same ruling.
  **`docs/plan/BLIT_CONSOLE.md` (Superseded, D-2026-08-15-1)** → `docs/plan/UI_REMOVAL.md` (**Superseded**, D-2026-08-17-1) → `docs/plan/INTERFACE_PLATFORM.md` (**Shipped** 2026-08-18, fully ruled D-2026-08-17-1..-5 — blit-core as the platform crate + out-of-repo front-ends).
  **`docs/plan/PER_FILE_ERROR_CONTAINMENT.md` (SHIPPED, D-2026-08-01-2).**
- Shipped release record: **`docs/RELEASE_READINESS.md`** and
  **`docs/plan/RELEASE_COMPLETION.md`**.
- Historical live-tuning record: **`docs/plan/LIVE_DIAL_TUNING.md`**; exact
  session audit: **`docs/bench/ldt4-evidence-audit-2026-07-22/`**.
- **`docs/plan/SOURCE_SIDE_CONTAINMENT.md` (SHIPPED 2026-10-07, D-2026-10-07-1 —
  all six slices landed 2026-09-30; cr-win-1 and its review findings closed 2026-10-07).**
- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**Active**; sf-3c/sf-3d
  landed, no further cut selected). **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`**
  SHIPPED 2026-10-14 (D-2026-10-14-1; measurement criteria post-release). REV4
  superseded v1/REV2/REV3 (history only).
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

- **Defects found 2026-10-07:** (a) the local copy's cleanup armed before the destination was opened — FIXED `bbc75dec`; (b) Ctrl-C during the retries lost the report — FIXED `b097aec0` (exit 130, report kept); (c) a `--detach` job hid per-file failures — FIXED by JOB_LOGS jl-1c `f78008b7` (pushed; CI green at `818dc2a9`).
- G3 (RELEASE_1_0) CLOSED 2026-10-14 (D-2026-10-14-1): audit-19 fixed `fa948e01`, the detached recents reason fixed `b7da0d81`; Codex g3fix-r1 found 3 MEDIUM on audit-19, fixed `7d3e4eb6` `47bdfb69` `eddfab6c`; g3fix2-r1 found 2 (HIGH symlink case under `mirror`, MEDIUM unescaped disk names), fixed `f6099aa3` `36b752b6`; g3fix3-r1 found 3 (HIGH Unicode-equivalent symlink spellings under `mirror`, MEDIUM escaped text matching a real name, MEDIUM a Windows peer rewriting `[\]`), fixed `a67087fd` `f6212938` `548625f5`; g3fix4-r1 found 4 (HIGH a hard-linked symlink and the Windows fold under `mirror`, HIGH the destination's own spelling under `mirror`, HIGH typed Unix escapes on a Windows peer, MEDIUM a class's `\` in the absolute split), fixed `e7be4970` `b304e6ff` `5624fd43` `e67c225c`; g3fix5-r1 found 4 (HIGH Unicode-equivalent destination names under `mirror`, HIGH glob components under `mirror`, MEDIUM `**\/` broadened, MEDIUM a literal Windows root split bracket-aware), fixed `80c6072e` `1435d7c7` `7f4e2116` `df61b1d6`; g3fix6-r1 found 3 (HIGH a folded class loses members, HIGH Unicode pairs a fold misses, MEDIUM quadratic `**`+glob), fix round 7 fixed locally in 882daa41, 9bec0bf3, 0767b17e; further review cancelled by the owner (D-2026-10-19-1). Open 2026-10-18 (post-1.0 unless the owner rules otherwise): should a client fall back to the gRPC data path when the data-plane dial fails behind a port-only firewall? and the OS-level connect timeout (`os error 60`) lacks the firewall hint `socket.rs`'s bounded-timeout branch carries.
