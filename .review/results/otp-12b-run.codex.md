Reading additional input from stdin...
OpenAI Codex v0.144.1
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: ultra
reasoning summaries: none
session id: 019f57aa-a8db-78c3-bad5-f6257d74d5b5
--------
user
Review the commit range f19776c..44c2046 (run: git log --oneline f19776c..44c2046; git diff f19776c..44c2046). It is otp-12b's RECORDED-RUN half per docs/plan/OTP12_ACCEPTANCE_RUN.md (Active): (1) e21cf84 fixes the win-initiated sentinel - pwsh parses bare $rc:R as a scope-qualified variable so it never printed (found at first rig contact; a manual reproduction proved the win->mac path itself works); (2) 856af64 strips CRs from the drain outcome before runs.csv - a bare \r mid-row split every row under python universal newlines and verdicted 192 valid runs INCOMPLETE; (3) 44c2046 commits docs/bench/otp12-win-2026-07-12/ (README + sanitized runs.csv + the raw CRLF original + summary/verdicts/drain/manifest). CHECK HARD: (a) recompute the README's headline numbers from runs.csv (medians over valid rows, floor-of-mean-of-middle-two): the invariance table (11/12 PASS, wm_tcp_mixed 1.237), the two converge fails (push_tcp_small 2080/1811/1868; pull_tcp_mixed 1138/867/1284), the cross rows and the gap rows (old_push/old_pull vs new_mw_worst/new_wm_worst per fixture x carrier) against docs/bench/otp2w-baseline-2026-07-10/summary.csv; (b) is the CR post-processing legitimate and fully disclosed (raw file committed; no timing value altered - verify runs.csv and runs-raw-crlf.csv differ ONLY by \r bytes); (c) does the README avoid declaring pass/fail and avoid self-adjudicating the D-2026-07-12-1 residue (gap rows RECORDED; the platform-attribution language); (d) the escalation reasoning: no pre-registered trigger fires (verify the fails have tight spreads and are not straddles per the design doc D2's rule); (e) the cross-block consistency note (block-1 push_tcp_small 2080 vs block-2 mw_tcp_small mac_init 1922) - is it honestly recorded and is any conclusion improperly drawn; (f) any overclaim in calling wm_tcp_mixed/push_tcp_small code-shaped findings. Output a concise markdown findings list - each finding with file:line, severity, rationale - then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll inspect the exact range and independently recompute the recorded-run evidence, with separate passes for arithmetic, raw-file integrity, and claims against the acceptance design. I’ll make no changes.
exec
/bin/zsh -lc "sed -n '1,240p' .agents/repo-guidance.md" in /Users/michael/Dev/blit_v2
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
- This section is the canonical home for the verification commands (the
  `.agents/repo-map.json` mirror was retired 2026-07-08 with the toolkit's
  JSON layer).

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
  PascalCase, constants SHOUT_CASE; match existing names
  (`transfer_session`, `TransferSession`, `PLAN_OPTIONS`).
- No blocking calls inside async contexts (use async send APIs in Tokio).
- Prefer async-aware tests (`#[tokio::test]`) for planner/engine work; keep
  tests deterministic; capture long logs under `logs/`.

## Project Map

- `crates/blit-core/` — core library (enumeration, planner, the unified
  `transfer_session` — every transfer, local included, since otp-11);
  most logic and unit tests live here. New modules get re-exported in
  `crates/blit-core/src/lib.rs`.
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
/bin/zsh -lc "sed -n '1,220p' .agents/state.md" in /Users/michael/Dev/blit_v2
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
- Verification commands: `.agents/repo-guidance.md` (Verification)
- How to update state: the `handoff` procedure in `docs/agent/PROTOCOL.md`

exec
/bin/zsh -lc 'git log --oneline f19776c..44c2046' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-z42zxfft' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-EhtsR5FC' (errno=Operation not permitted)
44c2046 otp-12b: record the Mac<->Windows acceptance evidence (invariance 11/12; converge 10/12; cross attributed)
856af64 otp-12b: strip CRs from the drain outcome before it lands in runs.csv
e21cf84 otp-12b: fix the sentinel rc reference (pwsh scope-qualified variable parse)

exec
/bin/zsh -lc 'git remote -v' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'git diff --stat f19776c..44c2046' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'git status --short --branch' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-uZJ2S6tr' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rq0mfyXJ' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-g0ziGd6S' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-wZZYOrwh' (errno=Operation not permitted)
## master...origin/master [ahead 3]
?? .review/results/otp-12b-run.codex.md

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-YODfKSx2' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-T2Igji4R' (errno=Operation not permitted)
 docs/bench/otp12-win-2026-07-12/README.md          | 109 ++++++++++++
 docs/bench/otp12-win-2026-07-12/drain-outcomes.txt | 192 ++++++++++++++++++++
 docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv  | 193 +++++++++++++++++++++
 docs/bench/otp12-win-2026-07-12/runs.csv           | 193 +++++++++++++++++++++
 .../otp12-win-2026-07-12/staging-manifest.txt      |   8 +
 docs/bench/otp12-win-2026-07-12/summary.csv        |  49 ++++++
 docs/bench/otp12-win-2026-07-12/verdicts.csv       | 121 +++++++++++++
 scripts/bench_otp12_win.sh                         |  12 +-
 8 files changed, 876 insertions(+), 1 deletion(-)

exec
/bin/zsh -lc 'head -n 8 docs/bench/otp12-win-2026-07-12/runs.csv' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'head -n 12 docs/bench/otp12-win-2026-07-12/summary.csv' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid
push_tcp_large,old,0f922de,mac,1,1935,921,0,drained_8s,yes
push_tcp_large,new,e21cf84,mac,1,1926,911,0,drained_6s,yes
push_tcp_large,old,0f922de,mac,2,1908,907,0,drained_8s,yes
push_tcp_large,new,e21cf84,mac,2,1914,911,0,drained_6s,yes
push_tcp_large,old,0f922de,mac,3,1909,914,0,drained_6s,yes
push_tcp_large,new,e21cf84,mac,3,1894,882,0,drained_8s,yes
push_tcp_large,old,0f922de,mac,4,1903,897,0,drained_6s,yes

 succeeded in 0ms:
cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
mw_grpc_large,mac_init,1911,1914,1910,0.9,0,4
mw_grpc_large,win_init,1931,1933,1924,1.1,0,4
mw_grpc_mixed,mac_init,1829,1839,1810,4.4,0,4
mw_grpc_mixed,win_init,1842,1845,1834,1.6,0,4
mw_grpc_small,mac_init,2261,2248,2094,13.5,0,4
mw_grpc_small,win_init,2227,2221,2096,11.4,0,4
mw_tcp_large,mac_init,1914,1930,1897,5.2,0,4
mw_tcp_large,win_init,1920,1923,1918,0.8,0,4
mw_tcp_mixed,mac_init,1587,1589,1463,17.4,0,4
mw_tcp_mixed,win_init,1502,1584,1486,24.2,0,4
mw_tcp_small,mac_init,1922,1913,1884,2.2,0,4

exec
/bin/zsh -lc 'nl -ba docs/bench/otp12-win-2026-07-12/README.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	# otp-12b — Mac↔Windows acceptance session: converge-up + initiator/verb invariance (2026-07-12)
     2	
     3	**Status**: Recorded. **Scope**: the owner-designated closest-spec pair —
     4	rig W carries the plan's cross-direction half AND the headline
     5	initiator/verb-invariance criterion (`docs/plan/OTP12_ACCEPTANCE_RUN.md`
     6	D2–D3; parent criteria 1–2 as annotated by D-2026-07-12-1). **This
     7	README declares nothing** — pass/fail belongs to the owner at otp-13;
     8	it records the computed rows.
     9	
    10	**Harness**: `scripts/bench_otp12_win.sh` at run commit `e21cf84`
    11	(design/harness codex rounds: 12 findings accepted at `d3eae58`; two
    12	found-live fixes after first rig contact: the pwsh scope-qualified
    13	`$rc:R` sentinel parse at `e21cf84`, and the CR-in-drain-outcome CSV
    14	split at `856af64` — see Post-processing). RUNS=4, ABBA, pair-void rule;
    15	**192 timed runs, zero voided pairs, zero drain anomalies**.
    16	
    17	## Builds (matched pairs, sha-verified; 7 hashes in `staging-manifest.txt`)
    18	
    19	- **old arm**: `0f922de` both ends — Mac client rebuilt clean in a
    20	  detached worktree (pre-cutover clients embed no id:
    21	  `OLD_CLIENT_PROVENANCE_BY_BUILD=1`, provenance = build procedure +
    22	  manifest); Windows daemon = the aside-copied native detached-checkout
    23	  build (embeds `+0f922de`, Select-String-verified).
    24	- **new arm**: `e21cf84` both ends (Mac local build; Windows native
    25	  build from a fresh bundle; `blit.exe` client likewise staged).
    26	- Rig note: the box is `netwatch-01` at **10.1.10.177** (the recorded
    27	  10.1.10.173 went stale — DHCP); Mac 10 GbE at 10.1.10.54.
    28	
    29	## Post-processing (recorded, reproducible)
    30	
    31	The session's `runs.csv` was CR-sanitized after the run (`tr -d '\r'`;
    32	original committed as `runs-raw-crlf.csv`): pwsh emits CRLF and the
    33	bare `\r` in the drain column split every row under python's
    34	universal-newline csv reader, verdicting everything INCOMPLETE off 192
    35	valid runs. `verdicts.csv`/`summary.csv` were recomputed with the
    36	harness's own verdict pass over the sanitized rows; the harness now
    37	strips CRs at source (`856af64`). No timing value was altered.
    38	
    39	## Block 1 — converge-up (Mac-initiated, old vs new interleaved): 10/12 PASS
    40	
    41	Combined outcomes (`verdicts.csv` carries per-reference rows):
    42	PASS everywhere except —
    43	
    44	| cell | new | old same-session | ratio | committed | ratio | outcome |
    45	|------|----:|----:|----:|----:|----:|---------|
    46	| push_tcp_small | 2080 | 1811 | **1.149** | 1868 | 1.113 | FAIL-BOTH (spreads 3.8/3.0% — real) |
    47	| pull_tcp_mixed | 1138 | 867 | **1.313** | 1284 | 0.886 | FAIL-SAME-SESSION (spreads 5.2/6.7%) |
    48	
    49	No pre-registered escalation trigger fires (no straddle with >25%
    50	spread — these are tight-spread results); both stand recorded for the
    51	otp-13 walk. Rig context: today's old arms run far FASTER than their
    52	2026-07-10 committed medians (e.g. old pull_tcp_mixed 867 vs 1284, old
    53	push_tcp_large 1908 vs 3054) — reference drift in the fast direction,
    54	so the committed bars are easy and the same-session bars are the
    55	binding ones.
    56	
    57	## Block 2 — initiator/verb invariance (new pair): 11/12 PASS
    58	
    59	The owner's sentence, measured: per direction × fixture × carrier,
    60	`max(mac_init, win_init)/min ≤ 1.10`. Eleven cells PASS at ratios
    61	1.003–1.057. The exception:
    62	
    63	- **wm_tcp_mixed FAIL at 1.237** (mac_init 1127 vs win_init 911, tight
    64	  spreads 8.2/3.3%): Win→Mac mixed over the TCP data plane is ~25%
    65	  slower when the MAC initiates (pull-verb, destination role) than when
    66	  Windows initiates (push-verb, source role). Independently
    67	  corroborated by block 1 (`pull_tcp_mixed` new 1138 vs old 867) and
    68	  NOT present on grpc (wm_grpc_mixed 1.013) or other fixtures (large
    69	  1.023, small 1.011) — the signature is specifically
    70	  TCP-carrier × mixed workload × destination-initiator. A
    71	  code-shaped finding for the otp-13 walk (and the exact class of
    72	  defect this criterion exists to catch).
    73	
    74	## Cross-direction (F4 + the D-2026-07-12-1 discriminator)
    75	
    76	- **Win→Mac: all six cells PASS** — the unified path beats even the
    77	  better committed old direction (ratios 0.71–0.99).
    78	- **Mac→Win: all six cells FAIL** `min(old_push, old_pull) × 1.10` —
    79	  and the gap rows attribute it: the same-session old direction gap
    80	  (`old_push/old_pull`) vs the unified gap (`new_mw/new_wm`) is
    81	  **unchanged on large (1.979 → 1.951 tcp; 1.956 → 1.945 grpc)** and
    82	  **narrowed on mixed (1.946 → 1.408) and grpc_small (1.929 → 1.644)**
    83	  — the residue is the Windows destination write path, present
    84	  identically without blit's old choreography (D-2026-07-12-1: such
    85	  cells count as satisfying criterion 2's cross-direction half). The
    86	  one exception: **tcp_small's gap widened (1.332 → 1.527)** — the
    87	  widening tracks the push_tcp_small code gap above, i.e. that cell's
    88	  cross miss is NOT fully platform-attributable.
    89	
    90	## Cross-block consistency note
    91	
    92	`push_tcp_small` (block 1 new arm) measured 2080 while `mw_tcp_small`
    93	mac_init (block 2, nominally the same work) measured 1922 — 8% apart in
    94	one session. Block-2 arms use precreated destination containers (design
    95	F5) where block 1 keeps the otp-2w shapes; the delta is recorded here
    96	rather than adjudicated.
    97	
    98	## Reproduction
    99	
   100	```
   101	export WIN_SSH=michael@netwatch-01 WIN_HOST=10.1.10.177
   102	export MAC_HOST=<mac 10GbE ip>  OLD_CLIENT_PROVENANCE_BY_BUILD=1
   103	RUNS=4 ./scripts/bench_otp12_win.sh
   104	PREFLIGHT_ONLY=1 ./scripts/bench_otp12_win.sh
   105	```
   106	
   107	Staging per the harness header (aside-copy the old exes BEFORE moving
   108	the checkout; bundle + native build; sha-named bins; the daemons launch
   109	from `bins\active\` under the one `blit-otp12-daemon` firewall rule).

exec
/bin/zsh -lc "sed -n '1,220p' docs/bench/otp2w-baseline-2026-07-10/summary.csv" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
cell,median_ms,avg_ms,best_ms
push_tcp_large,3054,3052,3041
push_grpc_large,3065,3067,3058
pull_tcp_large,1294,1294,1288
pull_grpc_large,1289,1289,1288
push_tcp_small,1868,1869,1841
push_grpc_small,2822,2862,2787
pull_tcp_small,1280,1315,1259
pull_grpc_small,1462,1477,1414
push_tcp_mixed,2288,2299,2266
push_grpc_mixed,2687,2675,2551
pull_tcp_mixed,1284,1304,1272
pull_grpc_mixed,1408,1409,1378

exec
/bin/zsh -lc "rg -n -C 8 'D2|stradd|escalat|D-2026-07-12-1|RECORDED|code-shaped|platform|residue|spread|tight|trigger|cross-block|consisten' docs/plan/OTP12_ACCEPTANCE_RUN.md docs/DECISIONS.md docs/STATE.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/STATE.md-29-    Suite → **1555** (as of `1ce73b5`; later commits are
docs/STATE.md-30-    bench/docs-only). SizeMtime = data-safe skip (open Q below).
docs/STATE.md-31-    Per-slice detail: DEVLOG 2026-07-10 entries + `.review/`.
docs/STATE.md-32-  - **otp-2 `[x]` (both halves).** zoey = PER-DIRECTION reference;
docs/STATE.md-33-    Mac↔Windows = cross-direction rig (otp-2w). Harnesses
docs/STATE.md-34-    `scripts/bench_otp2{,w}_baseline.sh`, evidence
docs/STATE.md-35-    `docs/bench/otp2{,w}-baseline-2026-07-10/README.md`. Key reading:
docs/STATE.md-36-    old push trails old pull on BOTH rigs — otp-12's interleaved
docs/STATE.md:37:    old-vs-new discriminates code cost from platform write-path cost.
docs/STATE.md-38-  - **otp-10 `[x]` CLOSED (a, b-1/2, c-1/2)** — verb cutover + THE
docs/STATE.md-39-    CUTOVER DELETION: one chokepoint per verb shape (`blit_app
docs/STATE.md-40-    run_remote_push`/`run_remote_pull`), ONE args→compare mapping,
docs/STATE.md-41-    move maps IgnoreTimes/Checksum-only on every route; relay removed
docs/STATE.md-42-    (D-2026-07-11-1); 4 drivers + `Push`/`PullSync` + 13 messages out
docs/STATE.md-43-    of tree AND proto (−13.8k lines, no bridge); DelegatedPull
docs/STATE.md-44-    no-payload proof recorded. Suite 1555 → … → **1488**. Per-slice
docs/STATE.md-45-    detail: DEVLOG 2026-07-11 entries + `.review/`.
--
docs/STATE.md-73-## Queue (ordered)
docs/STATE.md-74-
docs/STATE.md-75-1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
docs/STATE.md-76-   the only work item until it ships**: slices otp-1..13 through the
docs/STATE.md-77-   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
docs/STATE.md-78-   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
docs/STATE.md-79-   b-2), otp-8, otp-9 (a/b), otp-2 (+ otp-2w), otp-10 (a, b-1/2,
docs/STATE.md-80-   c-1/2), **otp-11 (a + b)**, **otp-12a (zoey)** `[x]`. otp-12 design
docs/STATE.md:81:   ACTIVE (owner flip + D-2026-07-12-1). otp-12a recorded
docs/STATE.md-82-   (`docs/bench/otp12-zoey-2026-07-12/`): 10 PASS, 2 to the otp-13 walk
docs/STATE.md-83-   (pull_tcp_large reference-drift; push_tcp_small 1.105). Current:
docs/STATE.md-84-   **otp-12b (Mac↔Windows cross-direction + invariance)**, then 12c
docs/STATE.md-85-   (delegated), 12d, otp-13.
docs/STATE.md-86-2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
docs/STATE.md-87-   Shipped (zero-copy resolved — D-2026-07-05-3). Optional follow-ups
docs/STATE.md-88-   largely absorbed by otp-2/otp-12's rig matrices; skippy env facts
docs/STATE.md-89-   moved to Blocked → Rig availability.
--
docs/STATE.md-97-   cutover as a runtime-selected write strategy in the unified receive
docs/STATE.md-98-   sink (design: eval doc §If-FAST-evidence; dead module deletes in
docs/STATE.md-99-   w8-1). Rig facts + the aarch64-musl static build recipe: DEVLOG
docs/STATE.md-100-   2026-07-05 10:00. **Standing owner safety rule**: ALL activity on
docs/STATE.md-101-   rig `zoey` is confined to its `…/blit-temp/` folder — module roots,
docs/STATE.md-102-   test data, everything; nothing written outside it, ever. Zero-copy
docs/STATE.md-103-   is pre-authorized to be tested there when the post-cutover slice set
docs/STATE.md-104-   reaches it; no daemon runs on zoey before then without a fresh go.
docs/STATE.md:105:6. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
docs/STATE.md-106-   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
docs/STATE.md-107-   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
docs/STATE.md-108-   `derive_local_plan_tuning` fold-or-retire; receive-side dial
docs/STATE.md:109:   tuning residue (w3-1 scoped it out); the source send half's bounded
docs/STATE.md-110-   `dp.queue()` is not raced against control-lane events (deferred at
docs/STATE.md-111-   codex otp-7b-1 F3; otp-8 F1 gave the in-stream sends a fault race —
docs/STATE.md-112-   residual: the narrow CANCELLED→INTERNAL decay, verdict file);
docs/STATE.md-113-   CLI progress monitor lives through the in-session mirror purge
docs/STATE.md-114-   (display-only ticks/avg dilution; fix = the M-C `AppProgressEvent`
docs/STATE.md-115-   phase reshape — deferred at codex otp-10b-2 F5).
docs/STATE.md-116-
docs/STATE.md-117-## Authoritative docs right now
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-1-# otp-12 — symmetric-rig acceptance run (design)
docs/plan/OTP12_ACCEPTANCE_RUN.md-2-
docs/plan/OTP12_ACCEPTANCE_RUN.md-3-**Status**: Active (owner "yes to both", 2026-07-12 — the doc's only open
docs/plan/OTP12_ACCEPTANCE_RUN.md:4:question was ruled by D-2026-07-12-1; design codex round closed at
docs/plan/OTP12_ACCEPTANCE_RUN.md-5-`92e1d51`. The zoey RIG RUN still requires its own fresh owner go at run
docs/plan/OTP12_ACCEPTANCE_RUN.md-6-time — standing STATE rule.)
docs/plan/OTP12_ACCEPTANCE_RUN.md-7-**Created**: 2026-07-12
docs/plan/OTP12_ACCEPTANCE_RUN.md-8-**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-12.
docs/plan/OTP12_ACCEPTANCE_RUN.md-9-**Contract**: `docs/TRANSFER_SESSION.md` (unchanged — this slice adds NO code
docs/plan/OTP12_ACCEPTANCE_RUN.md-10-and NO wire surface; it is harness scripts + rig runs + committed evidence).
docs/plan/OTP12_ACCEPTANCE_RUN.md-11-**Governs**: execution proceeds 12a → 12b → 12c → 12d, each commit through the
docs/plan/OTP12_ACCEPTANCE_RUN.md-12-codex loop (D-2026-07-04-1); rig availability may reorder 12a–12c (the otp-2
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-33-   (large / 10k-small / mixed), wall time initiating from end A vs end B —
docs/plan/OTP12_ACCEPTANCE_RUN.md-34-   push-verb vs pull-verb — within run noise (±10%). Committed as evidence.
docs/plan/OTP12_ACCEPTANCE_RUN.md-35-2. **Converge-up matrix** (criterion 2 / codex F4): every unified cell ≤ the
docs/plan/OTP12_ACCEPTANCE_RUN.md-36-   better of that cell's two old directions + noise (±10%), against the
docs/plan/OTP12_ACCEPTANCE_RUN.md-37-   recorded old-path baselines, confirmed by interleaved same-session
docs/plan/OTP12_ACCEPTANCE_RUN.md-38-   old-vs-new A/B (the otp-2 README's standing prescription for this rig
docs/plan/OTP12_ACCEPTANCE_RUN.md-39-   class).
docs/plan/OTP12_ACCEPTANCE_RUN.md-40-3. **Delegated cells** (owner rig designation, 2026-07-10, STATE Blocked):
docs/plan/OTP12_ACCEPTANCE_RUN.md:41:   remote↔remote on the Windows box + skippy — the delegated trigger must
docs/plan/OTP12_ACCEPTANCE_RUN.md-42-   not cost wall time vs the same session driven directly.
docs/plan/OTP12_ACCEPTANCE_RUN.md-43-
docs/plan/OTP12_ACCEPTANCE_RUN.md-44-## Current state (verified at HEAD `ce36da3`)
docs/plan/OTP12_ACCEPTANCE_RUN.md-45-
docs/plan/OTP12_ACCEPTANCE_RUN.md-46-Load-bearing facts, with evidence:
docs/plan/OTP12_ACCEPTANCE_RUN.md-47-
docs/plan/OTP12_ACCEPTANCE_RUN.md-48-- One `copy` verb drives everything; a remote endpoint is `host:/module/path`
docs/plan/OTP12_ACCEPTANCE_RUN.md-49-  or `host:port:/module/path`, default port 9031
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-54-  forces the in-stream carrier (`blit-cli/src/cli.rs:317-319`), and rides
docs/plan/OTP12_ACCEPTANCE_RUN.md-55-  the delegated spec too (`proto/blit.proto:408`,
docs/plan/OTP12_ACCEPTANCE_RUN.md-56-  `blit-daemon/src/service/delegated_pull.rs:334`).
docs/plan/OTP12_ACCEPTANCE_RUN.md-57-- Remote↔remote is delegated-only (D-2026-07-11-1): `blit copy A:/m/p B:/m/q`
docs/plan/OTP12_ACCEPTANCE_RUN.md-58-  always calls `DelegatedPull` on the **destination** daemon, which initiates
docs/plan/OTP12_ACCEPTANCE_RUN.md-59-  the one session against the source daemon in the DESTINATION role
docs/plan/OTP12_ACCEPTANCE_RUN.md-60-  (`blit-app/src/transfers/remote.rs:462-484`,
docs/plan/OTP12_ACCEPTANCE_RUN.md-61-  `delegated_pull.rs:312-327,352`). There is no push-shaped delegated form.
docs/plan/OTP12_ACCEPTANCE_RUN.md:62:  The RPC carries trigger + progress only (no-payload proof recorded at
docs/plan/OTP12_ACCEPTANCE_RUN.md-63-  otp-10: `cli_data_plane_outbound_bytes == 0`).
docs/plan/OTP12_ACCEPTANCE_RUN.md-64-- Delegation gate: destination daemon config `[delegation]
docs/plan/OTP12_ACCEPTANCE_RUN.md-65-  allow_delegated_pull = true` + `allowed_source_hosts` allowlist
docs/plan/OTP12_ACCEPTANCE_RUN.md-66-  (`blit-daemon/src/runtime.rs:139-145`); per-module `delegation_allowed`.
docs/plan/OTP12_ACCEPTANCE_RUN.md-67-- Same-build handshake (D-2026-07-05-2): first frame both directions; exact
docs/plan/OTP12_ACCEPTANCE_RUN.md-68-  `build_id` + `contract_version` equality or `BuildMismatch` refusal
docs/plan/OTP12_ACCEPTANCE_RUN.md-69-  (`transfer_session/mod.rs:660-701`). Dirty builds mint distinct ids
docs/plan/OTP12_ACCEPTANCE_RUN.md-70-  (`blit-core/build.rs:28-97`) — **all arms must be clean-tree builds; arms
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-89-  same-size/dest-newer candidates exist in any arm.
docs/plan/OTP12_ACCEPTANCE_RUN.md-90-
docs/plan/OTP12_ACCEPTANCE_RUN.md-91-## Rigs and what each anchors
docs/plan/OTP12_ACCEPTANCE_RUN.md-92-
docs/plan/OTP12_ACCEPTANCE_RUN.md-93-| rig | endpoints | anchors | why scoped so |
docs/plan/OTP12_ACCEPTANCE_RUN.md-94-|-----|-----------|---------|---------------|
docs/plan/OTP12_ACCEPTANCE_RUN.md-95-| **Z** | Mac (APFS SSD) ↔ zoey daemon (`10.1.10.206`, pool) | per-direction converge-up ONLY | hardware-asymmetric; cross-direction comparisons invalid here (D-2026-07-05-1; otp-2 README §Scope) |
docs/plan/OTP12_ACCEPTANCE_RUN.md-96-| **W** | Mac (APFS NVMe) ↔ Windows 11 (`10.1.10.173`, D: Gen5 NVMe) | converge-up per direction + the cross-direction half + initiator/verb invariance | owner-designated closest-spec pair ("mac to windows would be closer spec. windows is faster, both have 10gbe") |
docs/plan/OTP12_ACCEPTANCE_RUN.md:97:| **D** | Windows daemon ↔ skippy daemon (TrueNAS, x86_64), Mac as delegating CLI | delegated-vs-direct parity (trigger invariance) | owner-designated delegated rig; no old baseline exists on this pair |
docs/plan/OTP12_ACCEPTANCE_RUN.md-98-
docs/plan/OTP12_ACCEPTANCE_RUN.md-99-Contingency: skippy is available for Mac↔Linux cells "if needed" (owner) —
docs/plan/OTP12_ACCEPTANCE_RUN.md-100-used only if zoey is unavailable (it was under maintenance 2026-07-11); such
docs/plan/OTP12_ACCEPTANCE_RUN.md-101-a substitution records fresh baselines and is per-direction only.
docs/plan/OTP12_ACCEPTANCE_RUN.md-102-
docs/plan/OTP12_ACCEPTANCE_RUN.md-103-## Design decisions
docs/plan/OTP12_ACCEPTANCE_RUN.md-104-
docs/plan/OTP12_ACCEPTANCE_RUN.md-105-### D1 — matched-pair interleaved A/B (build identity is the axis)
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-123-sha, all hosts). Old arms = the pinned baseline shas (`e757dcc` zoey,
docs/plan/OTP12_ACCEPTANCE_RUN.md-124-`0f922de` Windows). Old-arm Mac clients are rebuilt at the pinned sha in a
docs/plan/OTP12_ACCEPTANCE_RUN.md-125-detached worktree (`git worktree add --detach` — the otp-11a precedent) and
docs/plan/OTP12_ACCEPTANCE_RUN.md-126-stashed at `~/blit-bench-work/bins/blit-<sha>`. The handshake enforces new-
docs/plan/OTP12_ACCEPTANCE_RUN.md-127-arm pair identity at the first frame; old arms predate it, so old-arm
docs/plan/OTP12_ACCEPTANCE_RUN.md-128-provenance rests on the staging record (`.agents/machines.md`) plus a
docs/plan/OTP12_ACCEPTANCE_RUN.md-129-sha256 manifest recorded in the evidence (Known gaps).
docs/plan/OTP12_ACCEPTANCE_RUN.md-130-
docs/plan/OTP12_ACCEPTANCE_RUN.md:131:### D2 — verdict arithmetic (what the evidence computes; the owner declares)
docs/plan/OTP12_ACCEPTANCE_RUN.md-132-
docs/plan/OTP12_ACCEPTANCE_RUN.md-133-All statistics per the recorded baselines: integer ms; median of 4, even
docs/plan/OTP12_ACCEPTANCE_RUN.md:134:count = floor of the mean of the middle two; per-cell spread
docs/plan/OTP12_ACCEPTANCE_RUN.md-135-`(max−min)/min` recorded.
docs/plan/OTP12_ACCEPTANCE_RUN.md-136-
docs/plan/OTP12_ACCEPTANCE_RUN.md-137-**Valid-run rule (codex design F7)**: a run with a nonzero blit exit OR an
docs/plan/OTP12_ACCEPTANCE_RUN.md-138-undrained pre-run window VOIDS its whole interleave pair (both arms at
docs/plan/OTP12_ACCEPTANCE_RUN.md-139-that counterbalance position); the pair is re-run — appended at the same
docs/plan/OTP12_ACCEPTANCE_RUN.md-140-position in the order — until `RUNS` valid pairs exist, capped at 2×RUNS
docs/plan/OTP12_ACCEPTANCE_RUN.md-141-pair attempts per comparison. At the cap the cell is recorded
docs/plan/OTP12_ACCEPTANCE_RUN.md-142-`INCOMPLETE` with its drain log: surfaced, never a silent pass and never
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-159-  (Windows-initiated): `max(A,B)/min(A,B) ≤ 1.10`. TCP rows are the verdict
docs/plan/OTP12_ACCEPTANCE_RUN.md-160-  rows; grpc rows are recorded, same bar, labeled secondary.
docs/plan/OTP12_ACCEPTANCE_RUN.md-161-- **Delegated parity (rig D, hard bar)**: per fixture × direction,
docs/plan/OTP12_ACCEPTANCE_RUN.md-162-  `max(delegated, direct)/min ≤ 1.10`.
docs/plan/OTP12_ACCEPTANCE_RUN.md-163-- **Cross-direction (rig W, the F4 computation)**: per fixture × carrier,
docs/plan/OTP12_ACCEPTANCE_RUN.md-164-  each unified direction's median vs
docs/plan/OTP12_ACCEPTANCE_RUN.md-165-  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
docs/plan/OTP12_ACCEPTANCE_RUN.md-166-  while passing per-direction converge-up AND invariance, the evidence
docs/plan/OTP12_ACCEPTANCE_RUN.md:167:  additionally computes the **platform-residue discriminator** the otp-2w
docs/plan/OTP12_ACCEPTANCE_RUN.md-168-  README pre-registered: compare the old arm's direction gap
docs/plan/OTP12_ACCEPTANCE_RUN.md-169-  (`old_push/old_pull`) with the new arm's (`new_MW/new_WM`), same
docs/plan/OTP12_ACCEPTANCE_RUN.md:170:  session. Gap unchanged ⇒ the residue exists identically without blit's
docs/plan/OTP12_ACCEPTANCE_RUN.md:171:  old choreography and lands on the platform write path (NTFS/Defender vs
docs/plan/OTP12_ACCEPTANCE_RUN.md-172-  APFS — the plan's Non-goals: different hardware need not perform
docs/plan/OTP12_ACCEPTANCE_RUN.md-173-  identically); gap closed ⇒ the code was the cost and the bar is met. The
docs/plan/OTP12_ACCEPTANCE_RUN.md-174-  README records BOTH computations per cell; a discriminator-attributed
docs/plan/OTP12_ACCEPTANCE_RUN.md:175:  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
docs/plan/OTP12_ACCEPTANCE_RUN.md-176-  the otp-13 walk reviews the recorded numbers.
docs/plan/OTP12_ACCEPTANCE_RUN.md-177-
docs/plan/OTP12_ACCEPTANCE_RUN.md:178:Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
docs/plan/OTP12_ACCEPTANCE_RUN.md:179:bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
docs/plan/OTP12_ACCEPTANCE_RUN.md-180-interleaved in a fresh session; both sessions are committed.
docs/plan/OTP12_ACCEPTANCE_RUN.md-181-**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
docs/plan/OTP12_ACCEPTANCE_RUN.md:182:text defined the trigger but not which session governs): the RUNS=8
docs/plan/OTP12_ACCEPTANCE_RUN.md:183:escalation session's medians govern the escalated comparison's combined
docs/plan/OTP12_ACCEPTANCE_RUN.md:184:outcome — more data where noise or a straddle made RUNS=4 undecidable is
docs/plan/OTP12_ACCEPTANCE_RUN.md:185:the escalation's entire purpose. The RUNS=4 rows stay committed and
docs/plan/OTP12_ACCEPTANCE_RUN.md-186-visible; the otp-13 walk sees both sessions.**
docs/plan/OTP12_ACCEPTANCE_RUN.md-187-
docs/plan/OTP12_ACCEPTANCE_RUN.md-188-### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
docs/plan/OTP12_ACCEPTANCE_RUN.md-189-
docs/plan/OTP12_ACCEPTANCE_RUN.md-190-For a FIXED data direction the two initiators are:
docs/plan/OTP12_ACCEPTANCE_RUN.md-191-
docs/plan/OTP12_ACCEPTANCE_RUN.md-192-- **Mac→Windows**: arm A = Mac client pushes
docs/plan/OTP12_ACCEPTANCE_RUN.md-193-  (`blit copy $MAC_WORK/src_<w> $WIN_HOST:9031:/bench/<fresh>/ --yes`);
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-226-Arm A cells run fresh inside the invariance block (interleaved A,B,A,B…) —
docs/plan/OTP12_ACCEPTANCE_RUN.md-227-block-1 new-arm numbers are NOT reused, so rig-state drift between blocks
docs/plan/OTP12_ACCEPTANCE_RUN.md-228-cannot masquerade as an initiator effect.
docs/plan/OTP12_ACCEPTANCE_RUN.md-229-
docs/plan/OTP12_ACCEPTANCE_RUN.md-230-### D4 — delegated cells = delegated-vs-direct parity (rig D)
docs/plan/OTP12_ACCEPTANCE_RUN.md-231-
docs/plan/OTP12_ACCEPTANCE_RUN.md-232-Per data direction, the delegated arm and the direct arm drive the SAME
docs/plan/OTP12_ACCEPTANCE_RUN.md-233-session code with the same roles on the same endpoints; the only deltas are
docs/plan/OTP12_ACCEPTANCE_RUN.md:234:who spawns the initiator (daemon vs CLI) and the trigger/progress relay:
docs/plan/OTP12_ACCEPTANCE_RUN.md-235-
docs/plan/OTP12_ACCEPTANCE_RUN.md-236-- **skippy→Windows**: delegated = Mac runs
docs/plan/OTP12_ACCEPTANCE_RUN.md-237-  `blit copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ $WIN_HOST:9031:/bench/<fresh>/ --yes`
docs/plan/OTP12_ACCEPTANCE_RUN.md-238-  (Windows daemon initiates, DESTINATION role); direct = Windows client
docs/plan/OTP12_ACCEPTANCE_RUN.md-239-  pulls the same source to the same disk
docs/plan/OTP12_ACCEPTANCE_RUN.md-240-  (`blit.exe copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
docs/plan/OTP12_ACCEPTANCE_RUN.md-241-- **Windows→skippy**: delegated = the mirror-image Mac command (skippy
docs/plan/OTP12_ACCEPTANCE_RUN.md-242-  daemon initiates); direct = skippy client pulls from the Windows daemon
docs/plan/OTP12_ACCEPTANCE_RUN.md-243-  (self-timed `/proc/uptime`-bracketed window over ssh, the zoey pattern).
docs/plan/OTP12_ACCEPTANCE_RUN.md-244-
docs/plan/OTP12_ACCEPTANCE_RUN.md-245-Timing: the delegated arm is timed on the Mac around the CLI invocation
docs/plan/OTP12_ACCEPTANCE_RUN.md-246-(the CLI blocks until the relayed Summary), plus the destination's
docs/plan/OTP12_ACCEPTANCE_RUN.md:247:self-timed flush — deliberately INCLUDING the trigger RPC + relay overhead
docs/plan/OTP12_ACCEPTANCE_RUN.md:248:(that is the honest end-to-end cost of delegation; on this LAN the trigger
docs/plan/OTP12_ACCEPTANCE_RUN.md-249-is sub-ms against multi-second cells). The direct arm is self-timed on the
docs/plan/OTP12_ACCEPTANCE_RUN.md-250-initiating host plus the same flush. Destination flush: Windows ⇒
docs/plan/OTP12_ACCEPTANCE_RUN.md-251-`Write-VolumeCache`; skippy ⇒ self-timed `sync` bracketed by
docs/plan/OTP12_ACCEPTANCE_RUN.md-252-`/proc/uptime` reads in one ssh shell. Cold caches: standby-purge (Windows)
docs/plan/OTP12_ACCEPTANCE_RUN.md-253-+ `drop_caches` (skippy, root/sudo) both ends every run; drain the
docs/plan/OTP12_ACCEPTANCE_RUN.md-254-destination disk (Windows counter loop; skippy `/proc/diskstats` quiet-
docs/plan/OTP12_ACCEPTANCE_RUN.md-255-window loop with a device-regex knob).
docs/plan/OTP12_ACCEPTANCE_RUN.md-256-
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-269-`scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
docs/plan/OTP12_ACCEPTANCE_RUN.md-270-`scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
docs/plan/OTP12_ACCEPTANCE_RUN.md-271-precedent: duplicate the shape, don't refactor recorded evidence;
docs/plan/OTP12_ACCEPTANCE_RUN.md-272-`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
docs/plan/OTP12_ACCEPTANCE_RUN.md-273-old scripts, both recorded sharp edges:
docs/plan/OTP12_ACCEPTANCE_RUN.md-274-
docs/plan/OTP12_ACCEPTANCE_RUN.md-275-- **Exit codes are checked**: the old harnesses swallow the blit exit code
docs/plan/OTP12_ACCEPTANCE_RUN.md-276-  inside the timed window; otp-12 records it per run (`exit` column) and a
docs/plan/OTP12_ACCEPTANCE_RUN.md:277:  nonzero exit voids the interleave pair per the D2 valid-run rule — a
docs/plan/OTP12_ACCEPTANCE_RUN.md-278-  failed transfer must never contribute a time.
docs/plan/OTP12_ACCEPTANCE_RUN.md-279-- **Multi-token flags ride an array**, not an unquoted scalar.
docs/plan/OTP12_ACCEPTANCE_RUN.md-280-
docs/plan/OTP12_ACCEPTANCE_RUN.md-281-CSV schema (all rigs):
docs/plan/OTP12_ACCEPTANCE_RUN.md-282-`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
docs/plan/OTP12_ACCEPTANCE_RUN.md:283:(`valid` = the PAIR's fate under the D2 valid-run rule — an
docs/plan/OTP12_ACCEPTANCE_RUN.md-284-individually-clean run whose partner voided reads `no`; amended at the
docs/plan/OTP12_ACCEPTANCE_RUN.md-285-12a harness slice)
docs/plan/OTP12_ACCEPTANCE_RUN.md-286-`summary.csv`:
docs/plan/OTP12_ACCEPTANCE_RUN.md:287:`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
docs/plan/OTP12_ACCEPTANCE_RUN.md:288:(medians over valid runs only — the D2 valid-run rule)
docs/plan/OTP12_ACCEPTANCE_RUN.md-289-`verdicts.csv`: `comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome`
docs/plan/OTP12_ACCEPTANCE_RUN.md-290-where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
docs/plan/OTP12_ACCEPTANCE_RUN.md-291-otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
docs/plan/OTP12_ACCEPTANCE_RUN.md-292-reference CSVs; corrected at the 12a review, codex F9),
docs/plan/OTP12_ACCEPTANCE_RUN.md-293-`<mw|wm>_<carrier>_<fixture>` for rig-W invariance cells (data
docs/plan/OTP12_ACCEPTANCE_RUN.md-294-direction Mac→Win / Win→Mac), and `gap_<carrier>_<fixture>` for the
docs/plan/OTP12_ACCEPTANCE_RUN.md:295:discriminator gap rows (kind `cross-gap`, outcome `RECORDED` — never
docs/plan/OTP12_ACCEPTANCE_RUN.md-296-self-adjudicated; added at the 12b harness slice), `arm` ∈
docs/plan/OTP12_ACCEPTANCE_RUN.md-297-`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
docs/plan/OTP12_ACCEPTANCE_RUN.md-298-`initiator` = host name, `kind` ∈
docs/plan/OTP12_ACCEPTANCE_RUN.md-299-`converge|invariance|delegated|cross|cross-gap`.
docs/plan/OTP12_ACCEPTANCE_RUN.md-300-Verdict outcome vocabulary (closed; 12b review, codex F12): per-reference
docs/plan/OTP12_ACCEPTANCE_RUN.md-301-rows carry `PASS|FAIL`; a comparison's `combined`/`invariance` row
docs/plan/OTP12_ACCEPTANCE_RUN.md:302:carries the registered D2 set
docs/plan/OTP12_ACCEPTANCE_RUN.md-303-(`PASS|FAIL-SAME-SESSION|FAIL-REFERENCE-DRIFT|FAIL-BOTH|INCOMPLETE`);
docs/plan/OTP12_ACCEPTANCE_RUN.md:304:`cross-gap` rows carry `RECORDED` only (never adjudicated); a block-2
docs/plan/OTP12_ACCEPTANCE_RUN.md-305-converge row whose same-session block-1 counterpart is absent or
docs/plan/OTP12_ACCEPTANCE_RUN.md:306:incomplete carries `NO-SAME-SESSION-REF` (an escalation-session
docs/plan/OTP12_ACCEPTANCE_RUN.md-307-artifact — the committed-reference row still governs). Nothing else is
docs/plan/OTP12_ACCEPTANCE_RUN.md-308-legal, and a missing committed-reference row aborts the verdict pass
docs/plan/OTP12_ACCEPTANCE_RUN.md-309-(fail closed).
docs/plan/OTP12_ACCEPTANCE_RUN.md-310-
docs/plan/OTP12_ACCEPTANCE_RUN.md-311-Fixtures: identical shapes to otp-2 (1 GiB large / 10k×4 KiB small /
docs/plan/OTP12_ACCEPTANCE_RUN.md-312-512 MiB+5k×2 KiB mixed), generated with the existing recipes (BSD vs GNU
docs/plan/OTP12_ACCEPTANCE_RUN.md-313-`dd` block-size spelling handled per host), staged untimed; pull sources
docs/plan/OTP12_ACCEPTANCE_RUN.md-314-shared across arms (bytes are bytes — recorded explicitly); every timed
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-391-(sha256 per binary per host). `docs/bench/otp12-acceptance-<date>/README.md`
docs/plan/OTP12_ACCEPTANCE_RUN.md-392-is the assembly. Raw session logs stay under `logs/` (untracked) as usual.
docs/plan/OTP12_ACCEPTANCE_RUN.md-393-
docs/plan/OTP12_ACCEPTANCE_RUN.md-394-## Known gaps / risks
docs/plan/OTP12_ACCEPTANCE_RUN.md-395-
docs/plan/OTP12_ACCEPTANCE_RUN.md-396-- **No rig is truly fs-identical.** The plan's "symmetric rig" is
docs/plan/OTP12_ACCEPTANCE_RUN.md-397-  instantiated by the owner-designated closest-spec pair; rig W's two
docs/plan/OTP12_ACCEPTANCE_RUN.md-398-  directions still land on different OS write paths (APFS vs NTFS +
docs/plan/OTP12_ACCEPTANCE_RUN.md:399:  Defender at its normal state). D2's discriminator computation is the
docs/plan/OTP12_ACCEPTANCE_RUN.md:400:  pre-registered, evidence-backed handling; a platform-residue cell counts
docs/plan/OTP12_ACCEPTANCE_RUN.md:401:  as satisfied per D-2026-07-12-1.
docs/plan/OTP12_ACCEPTANCE_RUN.md-402-- **Old-arm provenance is a staging record, not a handshake** (old paths
docs/plan/OTP12_ACCEPTANCE_RUN.md-403-  predate it). Mitigated by machines.md provenance + the sha256 manifest;
docs/plan/OTP12_ACCEPTANCE_RUN.md-404-  accepted residual risk.
docs/plan/OTP12_ACCEPTANCE_RUN.md-405-- **First-of-kind surfaces**: a daemon on the Mac (application firewall
docs/plan/OTP12_ACCEPTANCE_RUN.md-406-  unknown until the smoke) and a client on skippy (musl-static, untested
docs/plan/OTP12_ACCEPTANCE_RUN.md-407-  there — the zoey zigbuild recipe retargeted). Both are preflight-gated;
docs/plan/OTP12_ACCEPTANCE_RUN.md-408-  failures block the affected block only.
docs/plan/OTP12_ACCEPTANCE_RUN.md-409-- **zoey availability**: under maintenance 2026-07-11; daemon runs there
docs/plan/OTP12_ACCEPTANCE_RUN.md-410-  need a fresh owner go regardless (STATE rule).
docs/plan/OTP12_ACCEPTANCE_RUN.md:411:- **Delegated arm includes trigger/relay overhead by design** — recorded,
docs/plan/OTP12_ACCEPTANCE_RUN.md-412-  expected sub-ms on this LAN; if it ever dominates a cell, that IS a
docs/plan/OTP12_ACCEPTANCE_RUN.md-413-  finding, not noise.
docs/plan/OTP12_ACCEPTANCE_RUN.md-414-- **Suite/test count**: untouched — no crates/proto changes anywhere in
docs/plan/OTP12_ACCEPTANCE_RUN.md-415-  otp-12; the ≥1483 floor stands at 1484 from otp-11b.
docs/plan/OTP12_ACCEPTANCE_RUN.md-416-
docs/plan/OTP12_ACCEPTANCE_RUN.md:417:## Open questions — RESOLVED (owner, 2026-07-12; D-2026-07-12-1)
docs/plan/OTP12_ACCEPTANCE_RUN.md-418-
docs/plan/OTP12_ACCEPTANCE_RUN.md:419:- **Q1 — cross-direction residue on rig W**: RESOLVED "yes" — a cell that
docs/plan/OTP12_ACCEPTANCE_RUN.md-420-  beats its own old direction, is initiator-invariant, and misses the
docs/plan/OTP12_ACCEPTANCE_RUN.md-421-  `min(old_push, old_pull) × 1.10` bar only by a discriminator-attributed
docs/plan/OTP12_ACCEPTANCE_RUN.md:422:  platform write-path residue (same gap in the old arm, same session)
docs/plan/OTP12_ACCEPTANCE_RUN.md-423-  **counts as satisfying the cross-direction half of criterion 2**
docs/plan/OTP12_ACCEPTANCE_RUN.md:424:  (D-2026-07-12-1). The evidence still records both computations per
docs/plan/OTP12_ACCEPTANCE_RUN.md:425:  cell; the otp-13 walk reviews the numbers, but a platform-residue cell
docs/plan/OTP12_ACCEPTANCE_RUN.md-426-  is not a blocker.
--
docs/DECISIONS.md-19----
docs/DECISIONS.md-20-
docs/DECISIONS.md-21-## D-2026-05-31-1 — v0.1.0 shipped; release plan frozen
docs/DECISIONS.md-22-- Decision: `RELEASE_PLAN_v2_2026-05-04.md` is a frozen reference, no longer the active source of truth.
docs/DECISIONS.md-23-- Why: 0.1.0 tagged 2026-05-31; the plan served its purpose.
docs/DECISIONS.md-24-- Supersedes: RELEASE_PLAN_v2_2026-05-04.md as active plan.
docs/DECISIONS.md-25-
docs/DECISIONS.md-26-## D-2026-05-31-2 — Pick-not-Type TUI direction
docs/DECISIONS.md:27:- Decision: `TUI_REWORK.md` (dual-pane, M1–M6) supersedes `TUI_DESIGN.md` §6 trigger-modal text inputs and the F3 free-text destination prompt.
docs/DECISIONS.md-28-- Why: any field requiring the operator to recall and type an off-screen path is an interface failure.
docs/DECISIONS.md-29-- Supersedes: TUI_DESIGN.md §6 (portions).
docs/DECISIONS.md-30-
docs/DECISIONS.md-31-## D-2026-06-04-1 — R3 overrides R2 in the audit chain
docs/DECISIONS.md-32-- Decision: where R2 and R3 disagree on a finding's severity or content, R3 wins; see the ID-override table in `AUDIT_REPORT_2026-06-04_INDEX.md`.
docs/DECISIONS.md-33-- Why: R3 incorporates the GPT R2 critique and severity rebalance.
docs/DECISIONS.md-34-- Supersedes: conflicting R2 entries.
docs/DECISIONS.md-35-
--
docs/DECISIONS.md-56-
docs/DECISIONS.md-57-## D-2026-06-07-2 — Adaptive-streams lands via cherry-pick/rebase, excluding the WIP
docs/DECISIONS.md-58-- Decision: the adaptive-streams stack (live-progress → PR1 telemetry → PR2 work-queue → PR2 review fix, up to `eafb187`) lands later as a planned `docs/plan/` slice via cherry-pick or rebase onto fresh commits — never via `git merge` of the branch (see D-2026-06-07-1 trap). `d9d4ec7` (PR3 WIP, "DOES NOT BUILD") is explicitly excluded until it is finished and compiles.
docs/DECISIONS.md-59-- Why: the `-s ours` octopus recorded those tips as parents without landing their code, so the feature is not actually in master; a real merge would no-op. The one real conflict (`data_plane.rs`: `StallGuardWriter` vs the `Probe` generic) must be resolved by hand, which only a cherry-pick/rebase surfaces.
docs/DECISIONS.md-60-- Supersedes: nothing.
docs/DECISIONS.md-61-
docs/DECISIONS.md-62-## D-2026-06-11-1 — Design-coherence review plan Active; ratification covers Phase A only
docs/DECISIONS.md-63-- Decision: `docs/plan/DESIGN_COHERENCE_REVIEW.md` flipped Draft → Active. Owner approval authorizes **Phase A only** (concept-ownership map + per-crate stratum inventory); Phases B and C each need a fresh go/no-go at the preceding checkpoint. Interview decisions bound into the plan: blit-tui light pass, owner ratifies each Phase C finding, wire-breaking recommendations in scope (proto not frozen).
docs/DECISIONS.md:64:- Why: the repo was built by many models across several greenfield restarts and the owner judges it too inconsistently designed to trust as-is; mapping concept ownership precedes any re-scope (audit-h3c slice 2) or feature landing (adaptive-streams) so the fixes get designed once.
docs/DECISIONS.md-65-- Supersedes: nothing.
docs/DECISIONS.md-66-
docs/DECISIONS.md-67-## D-2026-06-11-2 — Design-review queue ratified in full; Pull-RPC delete; zero_copy gets a FAST evaluation
docs/DECISIONS.md-68-- Decision: All Phase C slices (`AUDIT_REPORT_2026-06-11_DESIGN.md`) ratified as proposed and entered into REVIEW.md in the proposed order. Embedded decisions: (a) **W2.4** — the deprecated Pull RPC is deleted once W2.3 has harvested its multi-stream pattern; criterion applied: not needed for FAST/SIMPLE/RELIABLE in any scenario. (b) **W8.1** — `zero_copy.rs` is **excluded** from the dead-code deletion sweep; owner judges it has FAST potential; disposition is an evaluation slice (`w8-1b`) that either produces a plan doc to wire splice into the receive pipeline or concludes deletion. (c) **W2.3** — writing the multi-stream-pull plan doc is authorized (no code before Status: Active).
docs/DECISIONS.md-69-- Why: review program (D-2026-06-11-1) delivered all three phases; owner is the gate for queue entry and exercised it in full.
docs/DECISIONS.md-70-- Supersedes: nothing (completes D-2026-06-11-1; `DESIGN_COHERENCE_REVIEW.md` flips Active → Shipped).
docs/DECISIONS.md-71-
docs/DECISIONS.md-72-## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
--
docs/DECISIONS.md-88-
docs/DECISIONS.md-89-## D-2026-06-20-3 — Veto: do NOT fold the streaming planner (H10b) into the unified engine
docs/DECISIONS.md-90-- Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
docs/DECISIONS.md-91-- Why: owner did not intend to revive H10b by way of the convergence plan; the inference was the agent's, flagged for confirmation, and the owner declined it. The workload-shape-awareness goal was always standalone and stands.
docs/DECISIONS.md-92-- Supersedes: nothing. Reverts the conditional H10b supersession that D-2026-06-20-2 had proposed (that entry is edited in-place to drop the inference and point here).
docs/DECISIONS.md-93-
docs/DECISIONS.md-94-## D-2026-06-20-4 — Unified transfer engine plan review freeze
docs/DECISIONS.md-95-- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is a Draft review candidate next to the original plan, and all unified-transfer-engine coding is frozen until the owner makes a final plan decision.
docs/DECISIONS.md:96:- Why: review found the Active plan's direction is sound but several slices need tightening before code starts: streaming initial planning was hidden inside `ue-1c`, local fast paths need to become engine-owned strategies, work-stealing is observable behavior, wire compatibility needs concrete shape, and pull parity gates must wait for multistream pull.
docs/DECISIONS.md-97-- Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.
docs/DECISIONS.md-98-
docs/DECISIONS.md-99-## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
docs/DECISIONS.md-100-- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` is the **Active** unified-transfer-engine plan (owner: "rev4 replaces v1"). `UNIFIED_TRANSFER_ENGINE.md` (v1) flips Active → Superseded; the intermediate review candidates `REV2.md` and `REV3.md` flip Draft → Superseded — all three superseded by REV4. REV4 carries v1's lineage/absorption header forward, so the supersessions v1 recorded (MULTISTREAM_PULL absorbed as the pull-multistream slice `ue-r2-1g`; PIPELINE_UNIFICATION/UNIFIED_RECEIVE_PIPELINE Historical) remain in force. The plan-review freeze (D-2026-06-20-4) is lifted as to the **plan decision**; coding still requires a fresh per-slice owner authorization (AGENTS.md §9) — no slice (`ue-r2-1a` first) starts on this decision alone.
docs/DECISIONS.md-101-- Why: REV4 is the only candidate whose code-reality section was verified against the tree (`HEAD` `09268eb`). REV3's headline "two static tables, not three" correction was itself wrong — all three stream-count ladders are live (`remote/tuning.rs::determine_remote_tuning`, `push/control.rs::desired_streams:476`, `pull.rs::pull_stream_count:904`), v1's three-ladder count was substantially right, and `tuning.rs`'s own doc comment confirms the daemon "runs its own ladder and wins". REV3 also wrongly said `determine_remote_tuning` drives local (it drives push + daemon pull) and conflated single-stream PullSync with the already-multistream deprecated Pull. REV4 = REV3 + corrected code reality, every symbol grounded with `file:line`, v1 lineage preserved. One Active plan avoids drift between candidates.
docs/DECISIONS.md-102-- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*
docs/DECISIONS.md-103-
docs/DECISIONS.md-104-## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
--
docs/DECISIONS.md-123-
docs/DECISIONS.md-124-## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
docs/DECISIONS.md-125-- Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
docs/DECISIONS.md-126-- Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
docs/DECISIONS.md-127-- Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).
docs/DECISIONS.md-128-
docs/DECISIONS.md-129-## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
docs/DECISIONS.md-130-- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
docs/DECISIONS.md:131:- Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
docs/DECISIONS.md:132:- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands); ~~and, effective only at ONE_TRANSFER_PATH's cutover slice (otp-10), REV4 §Constraints' "mixed old/new peers must negotiate down" rule (annotated in place; until that slice lands the rule governs)~~ **(the "only at cutover" scoping is superseded by D-2026-07-05-2 — no version compatibility, ever, effective immediately)**. The bounded-unilateral dial contract (D-2026-06-20-1/-2) is NOT superseded — it carries into the unified session unchanged.
docs/DECISIONS.md-133-
docs/DECISIONS.md-134-## D-2026-07-05-2 — No version compatibility, ever: same-build peers only
docs/DECISIONS.md-135-- Decision: Blit has NO version-compatibility obligation of any kind, in any direction, at any time — owner standing rule, restated with force 2026-07-05: "backward compatibility is NOT a consideration. I expect blit 1.2.3 not to be able to talk to blit-daemon 1.2.3.1. period. same build only. do not engineer tech debt into an unshipped product." Client and daemon interoperate only when built from the same source; the wire handshake must REFUSE a mismatched peer outright at session open (exact protocol/build identity — mechanism specified in ONE_TRANSFER_PATH otp-1 and pinned by test). Feature-capability bits that exist to tolerate version skew ("advisory until both peers advertise support", `supports_stream_resize`-style flags) are dead weight and go away with the unified session. NOT affected: the receiver capacity profile (runtime capacity of the receiving machine, D-2026-06-20-1/-2) — that is hardware negotiation, not version negotiation.
docs/DECISIONS.md-136-- Why: REV4 §Constraints carried a written "mixed old/new peers must negotiate down" rule while the owner's contrary rule lived only in chat; the ONE_TRANSFER_PATH plan review then resolved the document conflict in favor of the written rule ("governs until cutover"). Wrong direction — recording the owner's rule as a decision ends the unrecorded-intent-loses-to-stale-paper failure mode.
docs/DECISIONS.md-137-- Supersedes: REV4 §Constraints mixed-version clause (annotated in place, effective immediately — not at cutover); SMALL_FILE_CEILING §Constraints "mixed-version peers keep working via existing negotiation" clause and sf-6's mixed-version-test deliverable (annotated); the "effective only at ONE_TRANSFER_PATH's cutover slice" scoping inside D-2026-07-05-1's Supersedes line (the supersession is immediate and total); ONE_TRANSFER_PATH's Non-goals compat wording (rewritten same commit).
docs/DECISIONS.md-138-
docs/DECISIONS.md-139-## D-2026-07-05-3 — Zero-copy receive unparked: revisit gate declared met (UNAS rig)
docs/DECISIONS.md-140-- Decision: The D-2026-06-12-1 revisit gate ("receive-side CPU saturation") is **declared met by the owner** (2026-07-05): a UniFi UNAS 8 Pro daemon target whose CPU cannot saturate 10 GbE even from SSD cache. Zero-copy receive is unparked as sanctioned FAST work. Two clarifications: (a) the dead `zero_copy.rs` module still gets deleted as ratified — its EAGAIN busy-wait draft is a rewrite, not a revival (eval doc); (b) the capability returns the one-path way (owner exchange 2026-07-05): a **runtime-selected write strategy inside the unified receive sink** — the eval doc's revisit design (`AsyncFd`-readiness splice loop beside the buffered relay, selected when the reader is a raw TcpStream and the payload is a file record, buffered relay as universal fallback), capability-gated by kernel/fs support, identical in both roles — never a side path. Sequenced after ONE_TRANSFER_PATH's cutover (otp-10) as its own slice set; the UNAS is the measurement rig and the symmetric-endpoint benchmark rule (D-2026-07-05-2 era methodology) applies to its cells.
--
docs/DECISIONS.md-161-- Why: binary data-plane `BLOCK` records carry no protobuf envelope, so the 2 MiB tonic-frame rationale does not apply there; the wire already enforces `MAX_WIRE_BLOCK_BYTES` = 64 MiB on the receive side. A larger ceiling lets a data-plane session keep block-wise resume for partials up to 4 TiB (65_536 × 64 MiB) instead of degrading to full transfer at 128 GiB (the 2 MiB-ceiling limit).
docs/DECISIONS.md-162-- Supersedes: nothing — completes the revisit D-2026-07-10-1 explicitly deferred to otp-7b (OTP7_RESUME.md D5 amended in place, same commit).
docs/DECISIONS.md-163-
docs/DECISIONS.md-164-## D-2026-07-11-1 — `--relay-via-cli` removed; remote→remote is delegated-only
docs/DECISIONS.md-165-- Decision: The `--relay-via-cli` escape hatch is **removed** (owner, 2026-07-11, otp-10c-1). Remote→remote transfers are delegated-only; the CLI is never in the byte path. The relay's read half was the PullSync client's on-demand per-file remote read — a capability the unified session deliberately does not have — so PullSync's deletion (otp-10c) makes a streaming relay unrebuildable; offered the choice, the owner picked removal over a stage-to-temp-dir reimplementation. The topology the flag served (destination cannot reach source, CLI can reach both) is handled by two manual commands — pull to a local path, then push it — and the delegated CONNECT_SOURCE error hint now says exactly that. `RemoteTransferSource`, its `remote_transfer_source_constructed` counter, and every relay-combination gate (mirror/move/detach/resume × relay) die with the flag; the delegated no-CLI-byte-path pin (`cli_data_plane_outbound_bytes == 0`) remains the byte-path-isolation proof and doubles as the otp-10 deletion proof's CLI half.
docs/DECISIONS.md-166-- Why: unshipped product (no compat bar, D-2026-07-05-2 era posture); the ONE_TRANSFER_PATH directive is deletion of bespoke side paths, and a staged relay would merely automate what two commands already do — not worth a maintained transfer-adjacent code path.
docs/DECISIONS.md-167-- Supersedes: `REMOTE_REMOTE_DELEGATION_PLAN.md` (already Historical) §§relay-fallback/escape-hatch design — dated header note added, body kept verbatim per that doc's own precedent; the relay-combination gates R50-F1/R51-F2 (move×relay), audit-h1 rounds 1–2 (mirror×relay), codex otp-10a F4 (resume×relay), and the detach×relay gate — deleted with the flag they guarded (their data-loss reasoning is moot once no relay path exists to combine with); `scripts/bench_remote_remote.sh`'s relay leg (removed same commit).
docs/DECISIONS.md-168-
docs/DECISIONS.md:169:## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
docs/DECISIONS.md:170:- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
docs/DECISIONS.md:171:- Why: the plan's Non-goals already exclude making different hardware perform identically, and D-2026-07-05-1 restricts cross-direction verdicts to symmetric endpoints; no truly fs-identical pair exists in the fleet, so on the designated closest-spec rig the "better of the two old directions" bar can only bind net of the destination write-path residue the discriminator isolates. Settling the rule before the run prevents re-litigating it with numbers in hand.
docs/DECISIONS.md-172-- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).

exec
/bin/zsh -lc 'git diff --no-ext-diff --find-renames f19776c..44c2046 -- scripts/bench_otp12_win.sh' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-JUHvc45o' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rMz9KxZ2' (errno=Operation not permitted)
diff --git a/scripts/bench_otp12_win.sh b/scripts/bench_otp12_win.sh
index 9af9b9a..c69b210 100755
--- a/scripts/bench_otp12_win.sh
+++ b/scripts/bench_otp12_win.sh
@@ -378,6 +378,12 @@ drop_caches() {   # $1 = run label; sets RUN_DRAIN (pair-voiding, D2)
     local outcome
     outcome=$(drain_host || true)
     RUN_DRAIN=${outcome:-DRAIN-ERROR}
+    # Strip CRs BEFORE this lands in runs.csv: pwsh emits CRLF, and a
+    # bare \r mid-row makes python's universal-newline csv reader split
+    # the row before its `valid` field — every comparison then reads
+    # INCOMPLETE (found live: the whole first e21cf84 session verdicted
+    # INCOMPLETE off 196 perfectly valid runs).
+    RUN_DRAIN=${RUN_DRAIN//$'\r'/}
     RUN_DRAIN=${RUN_DRAIN// /_}
     echo "$1: $RUN_DRAIN" >> "$OUT_DIR/drain.log"
     [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: $1 window UNDRAINED ($RUN_DRAIN) — pair voided, will re-run"
@@ -506,7 +512,11 @@ win_client_run() {   # cell rid src dst flags_string; sets T_MS/T_RC
     # to T_RC=99 and voids the run; nothing can masquerade as a time.
     local cell="$1" rid="$2" src="$3" dst="$4" flags="${5:-}"
     local out
-    out=$(wssh "\$sw = [Diagnostics.Stopwatch]::StartNew(); & '$WIN_BLIT' copy '$src' '$dst' --yes $flags > \$null 2> '$WIN_TEST\\client-err.log'; \$rc = \$LASTEXITCODE; \$sw.Stop(); \"R:\$([int]\$sw.Elapsed.TotalMilliseconds),\$rc:R\"" \
+    # ${rc} braces are load-bearing: PowerShell parses bare `$rc:R` as
+    # a SCOPE-qualified variable (like $env:PATH), so the sentinel
+    # never printed and every win-initiated run read rc=99 (found live
+    # at the first win->mac smoke, 2026-07-12).
+    out=$(wssh "\$sw = [Diagnostics.Stopwatch]::StartNew(); & '$WIN_BLIT' copy '$src' '$dst' --yes $flags > \$null 2> '$WIN_TEST\\client-err.log'; \$rc = \$LASTEXITCODE; \$sw.Stop(); \"R:\$([int]\$sw.Elapsed.TotalMilliseconds),\${rc}:R\"" \
         | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)
     if [[ "$out" == *,* ]]; then T_MS=${out%%,*}; T_RC=${out##*,}; else T_MS=0; T_RC=99; fi
     if [[ "$T_RC" != 0 ]]; then

exec
/bin/zsh -lc 'git diff --numstat f19776c..44c2046' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-EO3o79mv' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-vcYp7aPD' (errno=Operation not permitted)
109	0	docs/bench/otp12-win-2026-07-12/README.md
192	0	docs/bench/otp12-win-2026-07-12/drain-outcomes.txt
193	0	docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv
193	0	docs/bench/otp12-win-2026-07-12/runs.csv
8	0	docs/bench/otp12-win-2026-07-12/staging-manifest.txt
49	0	docs/bench/otp12-win-2026-07-12/summary.csv
121	0	docs/bench/otp12-win-2026-07-12/verdicts.csv
11	1	scripts/bench_otp12_win.sh

codex
The first pass confirms the evidence bundle and both harness fixes are confined to the stated scope. I’m now checking the acceptance math from the raw rows rather than trusting the committed summaries; I’m also testing the CR claim byte-for-byte.
exec
/bin/zsh -lc "nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '20,90p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
    20	> "just make it so that I NEVER see a situation where pull is faster
    21	> than push or vice versa. that CAN NEVER be possible because of
    22	> something blit did. it should be identical if I start the transfer
    23	> from skippy and push to this machine or if I start the transfer on
    24	> this machine and pull from skippy."
    25	
    26	> On benchmark methodology: "tmp on one side, spinning rust on the
    27	> other is not a valid test."
    28	
    29	Scope, wire, and process were explicitly delegated to the agent
    30	("no idea. you architected this"; "I DO NOT CARE. FIX IT."). The
    31	owner's requirement is the invariant; everything below is the
    32	architecture that makes the invariant impossible to violate rather
    33	than merely maintained by discipline.
    34	
    35	## Goal
    36	
    37	One `TransferSession` implementation owns every byte transfer blit
    38	performs. A transfer has a SOURCE role and a DESTINATION role; which
    39	end initiated, and which CLI verb was used, select roles — they do not
    40	select code. When this plan ships, the per-direction drivers (client
    41	push driver, daemon push-receive, client pull driver, daemon
    42	pull-send, delegated-pull driver, local orchestration) **do not
    43	exist**: for fixed endpoints and dataset, direction/initiator/verb
    44	cannot affect behavior or wall time by blit's doing, because there is
    45	no second code path to differ.
    46	
    47	## Non-goals
    48	
    49	- Version compatibility of ANY kind (D-2026-07-05-2, owner standing
    50	  rule: "backward compatibility is NOT a consideration... same build
    51	  only. do not engineer tech debt into an unshipped product"). A blit
    52	  client talks only to a blit-daemon from the same build; the session
    53	  handshake REFUSES a mismatched peer outright. No negotiate-down, no
    54	  advisory fields, no feature-capability bits for version skew.
    55	  `Push`/`PullSync` are deleted at cutover with no bridge. (Old-path
    56	  code coexists in-tree during the migration slices solely so each
    57	  slice lands green — that is migration scaffolding, not wire
    58	  compatibility.)
    59	- Making different hardware perform identically. If src and dst sit
    60	  on different disks, the two *data directions* still differ by
    61	  physics; the invariant is that the same data direction between the
    62	  same endpoints is identical regardless of who initiates and which
    63	  verb is used.
    64	- WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
    65	- New features. This is a consolidation; capability parity with
    66	  today (mirror, filters, resume, fallback, delegation, progress,
    67	  jobs, cancellation) is the bar. Zero-copy receive is **unparked**
    68	  (D-2026-07-05-3, CPU-bound UNAS rig) but is a follow-on slice set
    69	  after cutover, not one of this plan's slices — see the Design note
    70	  on the write-strategy seam. One narrow owner-granted exception
    71	  (D-2026-07-09-1, otp-7b): the CLI end-of-operation fault summary —
    72	  name the file(s) a session fault affected and suggest a re-run —
    73	  lands inside otp-7. Nothing else new rides this plan.
    74	
    75	## Constraints
    76	
    77	- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
    78	  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
    79	  violated at the choreography layer.
    80	- **Converge up, not down**: per benchmark cell, the unified session
    81	  must match the better of today's two directions (within ±10% run
    82	  noise), not their average. Unification that slows the fast
    83	  direction fails review.
    84	- REV4 invariants carry: byte-identical results, StallGuard,
    85	  cancellation, byte-accounting. Existing pins are ported (not
    86	  dropped) as tests become role-parameterized; test count never
    87	  drops.
    88	- The sf-2 shape-correction behavior (stream count corrects as the
    89	  need list accumulates) becomes the one and only stream policy —
    90	  both directions inherit it by construction; its pins carry over.

exec
/bin/zsh -lc "nl -ba docs/plan/OTP12_ACCEPTANCE_RUN.md | sed -n '103,310p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   103	## Design decisions
   104	
   105	### D1 — matched-pair interleaved A/B (build identity is the axis)
   106	
   107	Each comparison interleaves arms in the deterministic counterbalanced
   108	order `A,B,B,A,A,B,B,A` (ABBA per pair-of-pairs — each arm leads half the
   109	pairs, so arm never confounds with within-pair position on the stateful
   110	rigs; pre-registered, no randomness, codex design F5) with `RUNS=4` per
   111	arm (8 timed runs per comparison). A = `old` (rig Z/W converge-up) or
   112	`delegated` (rig D). Interleaving is the verdict method, not a nicety:
   113	zoey's tiered write path never fully stops being stateful (otp-2 README
   114	§Run-to-run stability) and interleaving holds Defender state equal across
   115	arms on Windows (otp-2w README §Readings). Arm swap = stop one daemon
   116	pair, start the other (PID-scoped, stale-refusal preserved), always
   117	outside the timed window. Old arms exist only where an old baseline exists
   118	(rigs Z and W); invariance and delegated arms are new-build only — the old
   119	path is known non-invariant (the plan's founding defect) and has no
   120	delegated baseline.
   121	
   122	Build discipline: one clean commit per arm. New arm = the run commit (same
   123	sha, all hosts). Old arms = the pinned baseline shas (`e757dcc` zoey,
   124	`0f922de` Windows). Old-arm Mac clients are rebuilt at the pinned sha in a
   125	detached worktree (`git worktree add --detach` — the otp-11a precedent) and
   126	stashed at `~/blit-bench-work/bins/blit-<sha>`. The handshake enforces new-
   127	arm pair identity at the first frame; old arms predate it, so old-arm
   128	provenance rests on the staging record (`.agents/machines.md`) plus a
   129	sha256 manifest recorded in the evidence (Known gaps).
   130	
   131	### D2 — verdict arithmetic (what the evidence computes; the owner declares)
   132	
   133	All statistics per the recorded baselines: integer ms; median of 4, even
   134	count = floor of the mean of the middle two; per-cell spread
   135	`(max−min)/min` recorded.
   136	
   137	**Valid-run rule (codex design F7)**: a run with a nonzero blit exit OR an
   138	undrained pre-run window VOIDS its whole interleave pair (both arms at
   139	that counterbalance position); the pair is re-run — appended at the same
   140	position in the order — until `RUNS` valid pairs exist, capped at 2×RUNS
   141	pair attempts per comparison. At the cap the cell is recorded
   142	`INCOMPLETE` with its drain log: surfaced, never a silent pass and never
   143	a median over fewer than RUNS valid runs.
   144	
   145	- **Per-direction converge-up (rigs Z and W, hard bar)**: a clean PASS
   146	  requires `new_median ≤ ×1.10` of **BOTH** references — the same-session
   147	  interleaved old arm AND the committed 2026-07-10 baseline median for
   148	  that cell (codex design F2: the fixed pre-cutover bar must not be
   149	  loosened by a slower old rerun). A cell passing same-session but
   150	  failing the committed reference is recorded `FAIL-REFERENCE-DRIFT` and
   151	  gets one pre-registered fresh-session re-run; a persisting drift stands
   152	  as a recorded failure for the otp-13 walk. **Every unified arm of a
   153	  data direction — both initiators on rig W, both blocks — must meet
   154	  these bars independently** (codex design F3: the invariance ratio is an
   155	  additional constraint, never a substitute ceiling — otherwise
   156	  tolerances compound to 1.21×).
   157	- **Invariance (rig W, hard bar — the owner's sentence)**: per fixture ×
   158	  carrier × data direction, arm A (Mac-initiated) vs arm B
   159	  (Windows-initiated): `max(A,B)/min(A,B) ≤ 1.10`. TCP rows are the verdict
   160	  rows; grpc rows are recorded, same bar, labeled secondary.
   161	- **Delegated parity (rig D, hard bar)**: per fixture × direction,
   162	  `max(delegated, direct)/min ≤ 1.10`.
   163	- **Cross-direction (rig W, the F4 computation)**: per fixture × carrier,
   164	  each unified direction's median vs
   165	  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
   166	  while passing per-direction converge-up AND invariance, the evidence
   167	  additionally computes the **platform-residue discriminator** the otp-2w
   168	  README pre-registered: compare the old arm's direction gap
   169	  (`old_push/old_pull`) with the new arm's (`new_MW/new_WM`), same
   170	  session. Gap unchanged ⇒ the residue exists identically without blit's
   171	  old choreography and lands on the platform write path (NTFS/Defender vs
   172	  APFS — the plan's Non-goals: different hardware need not perform
   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
   174	  README records BOTH computations per cell; a discriminator-attributed
   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
   176	  the otp-13 walk reviews the recorded numbers.
   177	
   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
   180	interleaved in a fresh session; both sessions are committed.
   181	**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
   182	text defined the trigger but not which session governs): the RUNS=8
   183	escalation session's medians govern the escalated comparison's combined
   184	outcome — more data where noise or a straddle made RUNS=4 undecidable is
   185	the escalation's entire purpose. The RUNS=4 rows stay committed and
   186	visible; the otp-13 walk sees both sessions.**
   187	
   188	### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
   189	
   190	For a FIXED data direction the two initiators are:
   191	
   192	- **Mac→Windows**: arm A = Mac client pushes
   193	  (`blit copy $MAC_WORK/src_<w> $WIN_HOST:9031:/bench/<fresh>/ --yes`);
   194	  arm B = Windows client pulls
   195	  (`blit.exe copy $MAC_HOST:9031:/bench/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   196	- **Windows→Mac**: arm A = Mac client pulls (staged
   197	  `pull_src_<w>/src_<w>/` source, the otp-2w pattern); arm B = Windows
   198	  client pushes the same staged tree as a local path
   199	  (`blit.exe copy D:\blit-test\bench-module\pull_src_<w>\src_<w> $MAC_HOST:9031:/bench/<fresh>/ --yes`).
   200	
   201	New plumbing this requires, each keyed by ROLE not verb:
   202	
   203	1. **A daemon on the Mac** (new build only): config written like the rig
   204	   scripts do today (`[daemon] bind/port/no_mdns` + `[[module]] name =
   205	   "bench"` pointing at `$MAC_MODULE_ROOT`, **default `$MAC_WORK`
   206	   itself** — the module exports the exact fixture trees arm A pushes,
   207	   so both initiators read the same physical inodes; no fixture copy or
   208	   move on the Mac (codex design F6)), local launch, pid file,
   209	   stale-refusal, PID-scoped teardown. macOS application firewall must
   210	   admit `blit-daemon` — gated by a preflight smoke transfer from
   211	   Windows, not assumed.
   212	2. **A Windows client** (`blit.exe`, new build, built natively alongside
   213	   the daemon). Its timed window is measured ON Windows —
   214	   `[Diagnostics.Stopwatch]` bracketing the `blit.exe copy` inside one ssh
   215	   invocation, output CRLF-stripped (`tr -cd '0-9'`) — the otp-2w
   216	   self-timed pattern (README §Timing-overhead correction); the ssh
   217	   round-trip cost stays outside the window by construction.
   218	3. **Flush keyed by destination OS, never verb**: dest Windows ⇒ self-timed
   219	   `Write-VolumeCache D`; dest macOS ⇒ the local self-timed per-file fsync
   220	   walk. Cold caches both ends before every run (purge / standby-purge);
   221	   drain keyed by the destination disk (Windows `Get-Counter` loop when D:
   222	   receives; the Mac side has no drain equivalent — recorded decision: Mac
   223	   destination runs rely on `sync` + purge exactly as the recorded otp-2w
   224	   pull cells did).
   225	
   226	Arm A cells run fresh inside the invariance block (interleaved A,B,A,B…) —
   227	block-1 new-arm numbers are NOT reused, so rig-state drift between blocks
   228	cannot masquerade as an initiator effect.
   229	
   230	### D4 — delegated cells = delegated-vs-direct parity (rig D)
   231	
   232	Per data direction, the delegated arm and the direct arm drive the SAME
   233	session code with the same roles on the same endpoints; the only deltas are
   234	who spawns the initiator (daemon vs CLI) and the trigger/progress relay:
   235	
   236	- **skippy→Windows**: delegated = Mac runs
   237	  `blit copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ $WIN_HOST:9031:/bench/<fresh>/ --yes`
   238	  (Windows daemon initiates, DESTINATION role); direct = Windows client
   239	  pulls the same source to the same disk
   240	  (`blit.exe copy $SKIPPY_HOST:9031:/bench/pull_src_<w>/src_<w>/ D:\blit-test\bench-module\<fresh>\ --yes`).
   241	- **Windows→skippy**: delegated = the mirror-image Mac command (skippy
   242	  daemon initiates); direct = skippy client pulls from the Windows daemon
   243	  (self-timed `/proc/uptime`-bracketed window over ssh, the zoey pattern).
   244	
   245	Timing: the delegated arm is timed on the Mac around the CLI invocation
   246	(the CLI blocks until the relayed Summary), plus the destination's
   247	self-timed flush — deliberately INCLUDING the trigger RPC + relay overhead
   248	(that is the honest end-to-end cost of delegation; on this LAN the trigger
   249	is sub-ms against multi-second cells). The direct arm is self-timed on the
   250	initiating host plus the same flush. Destination flush: Windows ⇒
   251	`Write-VolumeCache`; skippy ⇒ self-timed `sync` bracketed by
   252	`/proc/uptime` reads in one ssh shell. Cold caches: standby-purge (Windows)
   253	+ `drop_caches` (skippy, root/sudo) both ends every run; drain the
   254	destination disk (Windows counter loop; skippy `/proc/diskstats` quiet-
   255	window loop with a device-regex knob).
   256	
   257	Carrier: TCP is the verdict carrier; one secondary grpc pair
   258	(large × skippy→Windows, both arms) is recorded as a smoke row — carrier
   259	selection reads `SessionOpen.in_stream_bytes`/policy, never role or
   260	initiator (`transfer_session/mod.rs:790,805`), and carrier invariance is
   261	measured properly on rig W.
   262	
   263	Config: BOTH daemons get `[delegation] allow_delegated_pull = true` with
   264	`allowed_source_hosts` naming the peer (each is destination in one
   265	direction); bench modules writable, `delegation_allowed` not narrowed.
   266	
   267	### D5 — three self-contained scripts; the frozen baselines stay frozen
   268	
   269	`scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
   270	`scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
   271	precedent: duplicate the shape, don't refactor recorded evidence;
   272	`bench_otp2{,w}_baseline.sh` are untouched). Two deliberate fixes over the
   273	old scripts, both recorded sharp edges:
   274	
   275	- **Exit codes are checked**: the old harnesses swallow the blit exit code
   276	  inside the timed window; otp-12 records it per run (`exit` column) and a
   277	  nonzero exit voids the interleave pair per the D2 valid-run rule — a
   278	  failed transfer must never contribute a time.
   279	- **Multi-token flags ride an array**, not an unquoted scalar.
   280	
   281	CSV schema (all rigs):
   282	`runs.csv`: `cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid`
   283	(`valid` = the PAIR's fate under the D2 valid-run rule — an
   284	individually-clean run whose partner voided reads `no`; amended at the
   285	12a harness slice)
   286	`summary.csv`:
   287	`cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted`
   288	(medians over valid runs only — the D2 valid-run rule)
   289	`verdicts.csv`: `comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome`
   290	where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
   291	otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
   292	reference CSVs; corrected at the 12a review, codex F9),
   293	`<mw|wm>_<carrier>_<fixture>` for rig-W invariance cells (data
   294	direction Mac→Win / Win→Mac), and `gap_<carrier>_<fixture>` for the
   295	discriminator gap rows (kind `cross-gap`, outcome `RECORDED` — never
   296	self-adjudicated; added at the 12b harness slice), `arm` ∈
   297	`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
   298	`initiator` = host name, `kind` ∈
   299	`converge|invariance|delegated|cross|cross-gap`.
   300	Verdict outcome vocabulary (closed; 12b review, codex F12): per-reference
   301	rows carry `PASS|FAIL`; a comparison's `combined`/`invariance` row
   302	carries the registered D2 set
   303	(`PASS|FAIL-SAME-SESSION|FAIL-REFERENCE-DRIFT|FAIL-BOTH|INCOMPLETE`);
   304	`cross-gap` rows carry `RECORDED` only (never adjudicated); a block-2
   305	converge row whose same-session block-1 counterpart is absent or
   306	incomplete carries `NO-SAME-SESSION-REF` (an escalation-session
   307	artifact — the committed-reference row still governs). Nothing else is
   308	legal, and a missing committed-reference row aborts the verdict pass
   309	(fail closed).
   310	

exec
/bin/zsh -lc "nl -ba docs/DECISIONS.md | sed -n '164,174p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   164	## D-2026-07-11-1 — `--relay-via-cli` removed; remote→remote is delegated-only
   165	- Decision: The `--relay-via-cli` escape hatch is **removed** (owner, 2026-07-11, otp-10c-1). Remote→remote transfers are delegated-only; the CLI is never in the byte path. The relay's read half was the PullSync client's on-demand per-file remote read — a capability the unified session deliberately does not have — so PullSync's deletion (otp-10c) makes a streaming relay unrebuildable; offered the choice, the owner picked removal over a stage-to-temp-dir reimplementation. The topology the flag served (destination cannot reach source, CLI can reach both) is handled by two manual commands — pull to a local path, then push it — and the delegated CONNECT_SOURCE error hint now says exactly that. `RemoteTransferSource`, its `remote_transfer_source_constructed` counter, and every relay-combination gate (mirror/move/detach/resume × relay) die with the flag; the delegated no-CLI-byte-path pin (`cli_data_plane_outbound_bytes == 0`) remains the byte-path-isolation proof and doubles as the otp-10 deletion proof's CLI half.
   166	- Why: unshipped product (no compat bar, D-2026-07-05-2 era posture); the ONE_TRANSFER_PATH directive is deletion of bespoke side paths, and a staged relay would merely automate what two commands already do — not worth a maintained transfer-adjacent code path.
   167	- Supersedes: `REMOTE_REMOTE_DELEGATION_PLAN.md` (already Historical) §§relay-fallback/escape-hatch design — dated header note added, body kept verbatim per that doc's own precedent; the relay-combination gates R50-F1/R51-F2 (move×relay), audit-h1 rounds 1–2 (mirror×relay), codex otp-10a F4 (resume×relay), and the detach×relay gate — deleted with the flag they guarded (their data-loss reasoning is moot once no relay path exists to combine with); `scripts/bench_remote_remote.sh`'s relay leg (removed same commit).
   168	
   169	## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
   170	- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
   171	- Why: the plan's Non-goals already exclude making different hardware perform identically, and D-2026-07-05-1 restricts cross-direction verdicts to symmetric endpoints; no truly fs-identical pair exists in the fleet, so on the designated closest-spec rig the "better of the two old directions" bar can only bind net of the destination write-path residue the discriminator isolates. Settling the rule before the run prevents re-litigating it with numbers in hand.
   172	- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).

exec
/bin/zsh -lc "nl -ba docs/bench/otp12-win-2026-07-12/verdicts.csv | sed -n '1,150p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome
     2	pull_grpc_large,converge,new,old_session,966,978,0.988,1.10,PASS
     3	pull_grpc_large,converge,new,old_committed,966,1289,0.749,1.10,PASS
     4	pull_grpc_large,converge,new,combined,966,,,1.10,PASS
     5	pull_grpc_mixed,converge,new,old_session,1244,1408,0.884,1.10,PASS
     6	pull_grpc_mixed,converge,new,old_committed,1244,1408,0.884,1.10,PASS
     7	pull_grpc_mixed,converge,new,combined,1244,,,1.10,PASS
     8	pull_grpc_small,converge,new,old_session,1294,1525,0.849,1.10,PASS
     9	pull_grpc_small,converge,new,old_committed,1294,1462,0.885,1.10,PASS
    10	pull_grpc_small,converge,new,combined,1294,,,1.10,PASS
    11	pull_tcp_large,converge,new,old_session,959,964,0.995,1.10,PASS
    12	pull_tcp_large,converge,new,old_committed,959,1294,0.741,1.10,PASS
    13	pull_tcp_large,converge,new,combined,959,,,1.10,PASS
    14	pull_tcp_mixed,converge,new,old_session,1138,867,1.313,1.10,FAIL
    15	pull_tcp_mixed,converge,new,old_committed,1138,1284,0.886,1.10,PASS
    16	pull_tcp_mixed,converge,new,combined,1138,,,1.10,FAIL-SAME-SESSION
    17	pull_tcp_small,converge,new,old_session,1237,1360,0.910,1.10,PASS
    18	pull_tcp_small,converge,new,old_committed,1237,1280,0.966,1.10,PASS
    19	pull_tcp_small,converge,new,combined,1237,,,1.10,PASS
    20	push_grpc_large,converge,new,old_session,1919,1913,1.003,1.10,PASS
    21	push_grpc_large,converge,new,old_committed,1919,3065,0.626,1.10,PASS
    22	push_grpc_large,converge,new,combined,1919,,,1.10,PASS
    23	push_grpc_mixed,converge,new,old_session,2081,2177,0.956,1.10,PASS
    24	push_grpc_mixed,converge,new,old_committed,2081,2687,0.774,1.10,PASS
    25	push_grpc_mixed,converge,new,combined,2081,,,1.10,PASS
    26	push_grpc_small,converge,new,old_session,2357,2942,0.801,1.10,PASS
    27	push_grpc_small,converge,new,old_committed,2357,2822,0.835,1.10,PASS
    28	push_grpc_small,converge,new,combined,2357,,,1.10,PASS
    29	push_tcp_large,converge,new,old_session,1904,1908,0.998,1.10,PASS
    30	push_tcp_large,converge,new,old_committed,1904,3054,0.623,1.10,PASS
    31	push_tcp_large,converge,new,combined,1904,,,1.10,PASS
    32	push_tcp_mixed,converge,new,old_session,1776,1687,1.053,1.10,PASS
    33	push_tcp_mixed,converge,new,old_committed,1776,2288,0.776,1.10,PASS
    34	push_tcp_mixed,converge,new,combined,1776,,,1.10,PASS
    35	push_tcp_small,converge,new,old_session,2080,1811,1.149,1.10,FAIL
    36	push_tcp_small,converge,new,old_committed,2080,1868,1.113,1.10,FAIL
    37	push_tcp_small,converge,new,combined,2080,,,1.10,FAIL-BOTH
    38	mw_grpc_large,invariance,mac_init,win_init,1911,1931,1.010,1.10,PASS
    39	mw_grpc_large,converge,mac_init,old_session,1911,1913,0.999,1.10,PASS
    40	mw_grpc_large,converge,mac_init,old_committed,1911,3065,0.623,1.10,PASS
    41	mw_grpc_large,converge,win_init,old_session,1931,1913,1.009,1.10,PASS
    42	mw_grpc_large,converge,win_init,old_committed,1931,3065,0.630,1.10,PASS
    43	mw_grpc_large,cross,worst_arm,min_old_committed,1931,1289,1.498,1.10,FAIL
    44	mw_grpc_mixed,invariance,mac_init,win_init,1829,1842,1.007,1.10,PASS
    45	mw_grpc_mixed,converge,mac_init,old_session,1829,2177,0.840,1.10,PASS
    46	mw_grpc_mixed,converge,mac_init,old_committed,1829,2687,0.681,1.10,PASS
    47	mw_grpc_mixed,converge,win_init,old_session,1842,2177,0.846,1.10,PASS
    48	mw_grpc_mixed,converge,win_init,old_committed,1842,2687,0.686,1.10,PASS
    49	mw_grpc_mixed,cross,worst_arm,min_old_committed,1842,1408,1.308,1.10,FAIL
    50	mw_grpc_small,invariance,mac_init,win_init,2261,2227,1.015,1.10,PASS
    51	mw_grpc_small,converge,mac_init,old_session,2261,2942,0.769,1.10,PASS
    52	mw_grpc_small,converge,mac_init,old_committed,2261,2822,0.801,1.10,PASS
    53	mw_grpc_small,converge,win_init,old_session,2227,2942,0.757,1.10,PASS
    54	mw_grpc_small,converge,win_init,old_committed,2227,2822,0.789,1.10,PASS
    55	mw_grpc_small,cross,worst_arm,min_old_committed,2261,1462,1.547,1.10,FAIL
    56	mw_tcp_large,invariance,mac_init,win_init,1914,1920,1.003,1.10,PASS
    57	mw_tcp_large,converge,mac_init,old_session,1914,1908,1.003,1.10,PASS
    58	mw_tcp_large,converge,mac_init,old_committed,1914,3054,0.627,1.10,PASS
    59	mw_tcp_large,converge,win_init,old_session,1920,1908,1.006,1.10,PASS
    60	mw_tcp_large,converge,win_init,old_committed,1920,3054,0.629,1.10,PASS
    61	mw_tcp_large,cross,worst_arm,min_old_committed,1920,1294,1.484,1.10,FAIL
    62	mw_tcp_mixed,invariance,mac_init,win_init,1587,1502,1.057,1.10,PASS
    63	mw_tcp_mixed,converge,mac_init,old_session,1587,1687,0.941,1.10,PASS
    64	mw_tcp_mixed,converge,mac_init,old_committed,1587,2288,0.694,1.10,PASS
    65	mw_tcp_mixed,converge,win_init,old_session,1502,1687,0.890,1.10,PASS
    66	mw_tcp_mixed,converge,win_init,old_committed,1502,2288,0.656,1.10,PASS
    67	mw_tcp_mixed,cross,worst_arm,min_old_committed,1587,1284,1.236,1.10,FAIL
    68	mw_tcp_small,invariance,mac_init,win_init,1922,1935,1.007,1.10,PASS
    69	mw_tcp_small,converge,mac_init,old_session,1922,1811,1.061,1.10,PASS
    70	mw_tcp_small,converge,mac_init,old_committed,1922,1868,1.029,1.10,PASS
    71	mw_tcp_small,converge,win_init,old_session,1935,1811,1.068,1.10,PASS
    72	mw_tcp_small,converge,win_init,old_committed,1935,1868,1.036,1.10,PASS
    73	mw_tcp_small,cross,worst_arm,min_old_committed,1935,1280,1.512,1.10,FAIL
    74	wm_grpc_large,invariance,mac_init,win_init,964,993,1.030,1.10,PASS
    75	wm_grpc_large,converge,mac_init,old_session,964,978,0.986,1.10,PASS
    76	wm_grpc_large,converge,mac_init,old_committed,964,1289,0.748,1.10,PASS
    77	wm_grpc_large,converge,win_init,old_session,993,978,1.015,1.10,PASS
    78	wm_grpc_large,converge,win_init,old_committed,993,1289,0.770,1.10,PASS
    79	wm_grpc_large,cross,worst_arm,min_old_committed,993,1289,0.770,1.10,PASS
    80	wm_grpc_mixed,invariance,mac_init,win_init,1246,1262,1.013,1.10,PASS
    81	wm_grpc_mixed,converge,mac_init,old_session,1246,1408,0.885,1.10,PASS
    82	wm_grpc_mixed,converge,mac_init,old_committed,1246,1408,0.885,1.10,PASS
    83	wm_grpc_mixed,converge,win_init,old_session,1262,1408,0.896,1.10,PASS
    84	wm_grpc_mixed,converge,win_init,old_committed,1262,1408,0.896,1.10,PASS
    85	wm_grpc_mixed,cross,worst_arm,min_old_committed,1262,1408,0.896,1.10,PASS
    86	wm_grpc_small,invariance,mac_init,win_init,1375,1326,1.037,1.10,PASS
    87	wm_grpc_small,converge,mac_init,old_session,1375,1525,0.902,1.10,PASS
    88	wm_grpc_small,converge,mac_init,old_committed,1375,1462,0.940,1.10,PASS
    89	wm_grpc_small,converge,win_init,old_session,1326,1525,0.870,1.10,PASS
    90	wm_grpc_small,converge,win_init,old_committed,1326,1462,0.907,1.10,PASS
    91	wm_grpc_small,cross,worst_arm,min_old_committed,1375,1462,0.940,1.10,PASS
    92	wm_tcp_large,invariance,mac_init,win_init,962,984,1.023,1.10,PASS
    93	wm_tcp_large,converge,mac_init,old_session,962,964,0.998,1.10,PASS
    94	wm_tcp_large,converge,mac_init,old_committed,962,1294,0.743,1.10,PASS
    95	wm_tcp_large,converge,win_init,old_session,984,964,1.021,1.10,PASS
    96	wm_tcp_large,converge,win_init,old_committed,984,1294,0.760,1.10,PASS
    97	wm_tcp_large,cross,worst_arm,min_old_committed,984,1294,0.760,1.10,PASS
    98	wm_tcp_mixed,invariance,mac_init,win_init,1127,911,1.237,1.10,FAIL
    99	wm_tcp_mixed,converge,mac_init,old_session,1127,867,1.300,1.10,FAIL
   100	wm_tcp_mixed,converge,mac_init,old_committed,1127,1284,0.878,1.10,PASS
   101	wm_tcp_mixed,converge,win_init,old_session,911,867,1.051,1.10,PASS
   102	wm_tcp_mixed,converge,win_init,old_committed,911,1284,0.710,1.10,PASS
   103	wm_tcp_mixed,cross,worst_arm,min_old_committed,1127,1284,0.878,1.10,PASS
   104	wm_tcp_small,invariance,mac_init,win_init,1253,1267,1.011,1.10,PASS
   105	wm_tcp_small,converge,mac_init,old_session,1253,1360,0.921,1.10,PASS
   106	wm_tcp_small,converge,mac_init,old_committed,1253,1280,0.979,1.10,PASS
   107	wm_tcp_small,converge,win_init,old_session,1267,1360,0.932,1.10,PASS
   108	wm_tcp_small,converge,win_init,old_committed,1267,1280,0.990,1.10,PASS
   109	wm_tcp_small,cross,worst_arm,min_old_committed,1267,1280,0.990,1.10,PASS
   110	gap_tcp_large,cross-gap,old_push,old_pull,1908,964,1.979,,RECORDED
   111	gap_tcp_large,cross-gap,new_mw_worst,new_wm_worst,1920,984,1.951,,RECORDED
   112	gap_tcp_small,cross-gap,old_push,old_pull,1811,1360,1.332,,RECORDED
   113	gap_tcp_small,cross-gap,new_mw_worst,new_wm_worst,1935,1267,1.527,,RECORDED
   114	gap_tcp_mixed,cross-gap,old_push,old_pull,1687,867,1.946,,RECORDED
   115	gap_tcp_mixed,cross-gap,new_mw_worst,new_wm_worst,1587,1127,1.408,,RECORDED
   116	gap_grpc_large,cross-gap,old_push,old_pull,1913,978,1.956,,RECORDED
   117	gap_grpc_large,cross-gap,new_mw_worst,new_wm_worst,1931,993,1.945,,RECORDED
   118	gap_grpc_small,cross-gap,old_push,old_pull,2942,1525,1.929,,RECORDED
   119	gap_grpc_small,cross-gap,new_mw_worst,new_wm_worst,2261,1375,1.644,,RECORDED
   120	gap_grpc_mixed,cross-gap,old_push,old_pull,2177,1408,1.546,,RECORDED
   121	gap_grpc_mixed,cross-gap,new_mw_worst,new_wm_worst,1842,1262,1.460,,RECORDED

exec
/bin/zsh -lc 'nl -ba docs/bench/otp12-win-2026-07-12/summary.csv' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
     2	mw_grpc_large,mac_init,1911,1914,1910,0.9,0,4
     3	mw_grpc_large,win_init,1931,1933,1924,1.1,0,4
     4	mw_grpc_mixed,mac_init,1829,1839,1810,4.4,0,4
     5	mw_grpc_mixed,win_init,1842,1845,1834,1.6,0,4
     6	mw_grpc_small,mac_init,2261,2248,2094,13.5,0,4
     7	mw_grpc_small,win_init,2227,2221,2096,11.4,0,4
     8	mw_tcp_large,mac_init,1914,1930,1897,5.2,0,4
     9	mw_tcp_large,win_init,1920,1923,1918,0.8,0,4
    10	mw_tcp_mixed,mac_init,1587,1589,1463,17.4,0,4
    11	mw_tcp_mixed,win_init,1502,1584,1486,24.2,0,4
    12	mw_tcp_small,mac_init,1922,1913,1884,2.2,0,4
    13	mw_tcp_small,win_init,1935,1947,1917,4.5,0,4
    14	pull_grpc_large,new,966,967,963,1.0,0,4
    15	pull_grpc_large,old,978,979,970,2.0,0,4
    16	pull_grpc_mixed,new,1244,1243,1217,4.4,0,4
    17	pull_grpc_mixed,old,1408,4576,1274,1015.6,0,4
    18	pull_grpc_small,new,1294,1292,1270,3.1,0,4
    19	pull_grpc_small,old,1525,1544,1504,7.8,0,4
    20	pull_tcp_large,new,959,962,955,2.0,0,4
    21	pull_tcp_large,old,964,964,956,1.9,0,4
    22	pull_tcp_mixed,new,1138,1147,1127,5.2,0,4
    23	pull_tcp_mixed,old,867,875,855,6.7,0,4
    24	pull_tcp_small,new,1237,1266,1223,12.0,0,4
    25	pull_tcp_small,old,1360,1359,1234,20.1,0,4
    26	push_grpc_large,new,1919,1921,1913,1.0,0,4
    27	push_grpc_large,old,1913,1914,1903,1.4,0,4
    28	push_grpc_mixed,new,2081,2108,1967,17.2,0,4
    29	push_grpc_mixed,old,2177,2174,2142,2.8,0,4
    30	push_grpc_small,new,2357,2325,2188,9.7,0,4
    31	push_grpc_small,old,2942,2901,2750,8.1,0,4
    32	push_tcp_large,new,1904,1906,1893,1.7,0,4
    33	push_tcp_large,old,1908,1913,1903,1.7,0,4
    34	push_tcp_mixed,new,1776,1830,1709,20.5,0,4
    35	push_tcp_mixed,old,1687,1699,1635,9.4,0,4
    36	push_tcp_small,new,2080,2075,2031,3.8,0,4
    37	push_tcp_small,old,1811,1816,1796,3.0,0,4
    38	wm_grpc_large,mac_init,964,963,960,0.6,0,4
    39	wm_grpc_large,win_init,993,992,985,1.4,0,4
    40	wm_grpc_mixed,mac_init,1246,1243,1208,5.3,0,4
    41	wm_grpc_mixed,win_init,1262,1264,1240,4.4,0,4
    42	wm_grpc_small,mac_init,1375,1379,1333,7.7,0,4
    43	wm_grpc_small,win_init,1326,1321,1302,2.1,0,4
    44	wm_tcp_large,mac_init,962,961,956,0.8,0,4
    45	wm_tcp_large,win_init,984,1594,961,258.7,0,4
    46	wm_tcp_mixed,mac_init,1127,1147,1122,8.2,0,4
    47	wm_tcp_mixed,win_init,911,911,897,3.3,0,4
    48	wm_tcp_small,mac_init,1253,1272,1216,12.5,0,4
    49	wm_tcp_small,win_init,1267,1274,1233,7.9,0,4

exec
/bin/zsh -lc 'git show --stat --oneline e21cf84 && git show --stat --oneline 856af64 && git show --stat --oneline 44c2046' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-7VV538xg' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-4v6jrIYv' (errno=Operation not permitted)
e21cf84 otp-12b: fix the sentinel rc reference (pwsh scope-qualified variable parse)
 scripts/bench_otp12_win.sh | 6 +++++-
 1 file changed, 5 insertions(+), 1 deletion(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-wDWacuCk' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-si5muZ5T' (errno=Operation not permitted)
856af64 otp-12b: strip CRs from the drain outcome before it lands in runs.csv
 scripts/bench_otp12_win.sh | 6 ++++++
 1 file changed, 6 insertions(+)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LpgAlsQW' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-McPdV8IL' (errno=Operation not permitted)
44c2046 otp-12b: record the Mac<->Windows acceptance evidence (invariance 11/12; converge 10/12; cross attributed)
 docs/bench/otp12-win-2026-07-12/README.md          | 109 ++++++++++++
 docs/bench/otp12-win-2026-07-12/drain-outcomes.txt | 192 ++++++++++++++++++++
 docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv  | 193 +++++++++++++++++++++
 docs/bench/otp12-win-2026-07-12/runs.csv           | 193 +++++++++++++++++++++
 .../otp12-win-2026-07-12/staging-manifest.txt      |   8 +
 docs/bench/otp12-win-2026-07-12/summary.csv        |  49 ++++++
 docs/bench/otp12-win-2026-07-12/verdicts.csv       | 121 +++++++++++++
 7 files changed, 865 insertions(+)

exec
/bin/zsh -lc "nl -ba docs/bench/otp12-win-2026-07-12/summary.csv | sed -n '1,60p'" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "nl -ba docs/bench/otp12-win-2026-07-12/verdicts.csv | sed -n '1,130p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
     2	mw_grpc_large,mac_init,1911,1914,1910,0.9,0,4
     3	mw_grpc_large,win_init,1931,1933,1924,1.1,0,4
     4	mw_grpc_mixed,mac_init,1829,1839,1810,4.4,0,4
     5	mw_grpc_mixed,win_init,1842,1845,1834,1.6,0,4
     6	mw_grpc_small,mac_init,2261,2248,2094,13.5,0,4
     7	mw_grpc_small,win_init,2227,2221,2096,11.4,0,4
     8	mw_tcp_large,mac_init,1914,1930,1897,5.2,0,4
     9	mw_tcp_large,win_init,1920,1923,1918,0.8,0,4
    10	mw_tcp_mixed,mac_init,1587,1589,1463,17.4,0,4
    11	mw_tcp_mixed,win_init,1502,1584,1486,24.2,0,4
    12	mw_tcp_small,mac_init,1922,1913,1884,2.2,0,4
    13	mw_tcp_small,win_init,1935,1947,1917,4.5,0,4
    14	pull_grpc_large,new,966,967,963,1.0,0,4
    15	pull_grpc_large,old,978,979,970,2.0,0,4
    16	pull_grpc_mixed,new,1244,1243,1217,4.4,0,4
    17	pull_grpc_mixed,old,1408,4576,1274,1015.6,0,4
    18	pull_grpc_small,new,1294,1292,1270,3.1,0,4
    19	pull_grpc_small,old,1525,1544,1504,7.8,0,4
    20	pull_tcp_large,new,959,962,955,2.0,0,4
    21	pull_tcp_large,old,964,964,956,1.9,0,4
    22	pull_tcp_mixed,new,1138,1147,1127,5.2,0,4
    23	pull_tcp_mixed,old,867,875,855,6.7,0,4
    24	pull_tcp_small,new,1237,1266,1223,12.0,0,4
    25	pull_tcp_small,old,1360,1359,1234,20.1,0,4
    26	push_grpc_large,new,1919,1921,1913,1.0,0,4
    27	push_grpc_large,old,1913,1914,1903,1.4,0,4
    28	push_grpc_mixed,new,2081,2108,1967,17.2,0,4
    29	push_grpc_mixed,old,2177,2174,2142,2.8,0,4
    30	push_grpc_small,new,2357,2325,2188,9.7,0,4
    31	push_grpc_small,old,2942,2901,2750,8.1,0,4
    32	push_tcp_large,new,1904,1906,1893,1.7,0,4
    33	push_tcp_large,old,1908,1913,1903,1.7,0,4
    34	push_tcp_mixed,new,1776,1830,1709,20.5,0,4
    35	push_tcp_mixed,old,1687,1699,1635,9.4,0,4
    36	push_tcp_small,new,2080,2075,2031,3.8,0,4
    37	push_tcp_small,old,1811,1816,1796,3.0,0,4
    38	wm_grpc_large,mac_init,964,963,960,0.6,0,4
    39	wm_grpc_large,win_init,993,992,985,1.4,0,4
    40	wm_grpc_mixed,mac_init,1246,1243,1208,5.3,0,4
    41	wm_grpc_mixed,win_init,1262,1264,1240,4.4,0,4
    42	wm_grpc_small,mac_init,1375,1379,1333,7.7,0,4
    43	wm_grpc_small,win_init,1326,1321,1302,2.1,0,4
    44	wm_tcp_large,mac_init,962,961,956,0.8,0,4
    45	wm_tcp_large,win_init,984,1594,961,258.7,0,4
    46	wm_tcp_mixed,mac_init,1127,1147,1122,8.2,0,4
    47	wm_tcp_mixed,win_init,911,911,897,3.3,0,4
    48	wm_tcp_small,mac_init,1253,1272,1216,12.5,0,4
    49	wm_tcp_small,win_init,1267,1274,1233,7.9,0,4

 succeeded in 0ms:
     1	comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome
     2	pull_grpc_large,converge,new,old_session,966,978,0.988,1.10,PASS
     3	pull_grpc_large,converge,new,old_committed,966,1289,0.749,1.10,PASS
     4	pull_grpc_large,converge,new,combined,966,,,1.10,PASS
     5	pull_grpc_mixed,converge,new,old_session,1244,1408,0.884,1.10,PASS
     6	pull_grpc_mixed,converge,new,old_committed,1244,1408,0.884,1.10,PASS
     7	pull_grpc_mixed,converge,new,combined,1244,,,1.10,PASS
     8	pull_grpc_small,converge,new,old_session,1294,1525,0.849,1.10,PASS
     9	pull_grpc_small,converge,new,old_committed,1294,1462,0.885,1.10,PASS
    10	pull_grpc_small,converge,new,combined,1294,,,1.10,PASS
    11	pull_tcp_large,converge,new,old_session,959,964,0.995,1.10,PASS
    12	pull_tcp_large,converge,new,old_committed,959,1294,0.741,1.10,PASS
    13	pull_tcp_large,converge,new,combined,959,,,1.10,PASS
    14	pull_tcp_mixed,converge,new,old_session,1138,867,1.313,1.10,FAIL
    15	pull_tcp_mixed,converge,new,old_committed,1138,1284,0.886,1.10,PASS
    16	pull_tcp_mixed,converge,new,combined,1138,,,1.10,FAIL-SAME-SESSION
    17	pull_tcp_small,converge,new,old_session,1237,1360,0.910,1.10,PASS
    18	pull_tcp_small,converge,new,old_committed,1237,1280,0.966,1.10,PASS
    19	pull_tcp_small,converge,new,combined,1237,,,1.10,PASS
    20	push_grpc_large,converge,new,old_session,1919,1913,1.003,1.10,PASS
    21	push_grpc_large,converge,new,old_committed,1919,3065,0.626,1.10,PASS
    22	push_grpc_large,converge,new,combined,1919,,,1.10,PASS
    23	push_grpc_mixed,converge,new,old_session,2081,2177,0.956,1.10,PASS
    24	push_grpc_mixed,converge,new,old_committed,2081,2687,0.774,1.10,PASS
    25	push_grpc_mixed,converge,new,combined,2081,,,1.10,PASS
    26	push_grpc_small,converge,new,old_session,2357,2942,0.801,1.10,PASS
    27	push_grpc_small,converge,new,old_committed,2357,2822,0.835,1.10,PASS
    28	push_grpc_small,converge,new,combined,2357,,,1.10,PASS
    29	push_tcp_large,converge,new,old_session,1904,1908,0.998,1.10,PASS
    30	push_tcp_large,converge,new,old_committed,1904,3054,0.623,1.10,PASS
    31	push_tcp_large,converge,new,combined,1904,,,1.10,PASS
    32	push_tcp_mixed,converge,new,old_session,1776,1687,1.053,1.10,PASS
    33	push_tcp_mixed,converge,new,old_committed,1776,2288,0.776,1.10,PASS
    34	push_tcp_mixed,converge,new,combined,1776,,,1.10,PASS
    35	push_tcp_small,converge,new,old_session,2080,1811,1.149,1.10,FAIL
    36	push_tcp_small,converge,new,old_committed,2080,1868,1.113,1.10,FAIL
    37	push_tcp_small,converge,new,combined,2080,,,1.10,FAIL-BOTH
    38	mw_grpc_large,invariance,mac_init,win_init,1911,1931,1.010,1.10,PASS
    39	mw_grpc_large,converge,mac_init,old_session,1911,1913,0.999,1.10,PASS
    40	mw_grpc_large,converge,mac_init,old_committed,1911,3065,0.623,1.10,PASS
    41	mw_grpc_large,converge,win_init,old_session,1931,1913,1.009,1.10,PASS
    42	mw_grpc_large,converge,win_init,old_committed,1931,3065,0.630,1.10,PASS
    43	mw_grpc_large,cross,worst_arm,min_old_committed,1931,1289,1.498,1.10,FAIL
    44	mw_grpc_mixed,invariance,mac_init,win_init,1829,1842,1.007,1.10,PASS
    45	mw_grpc_mixed,converge,mac_init,old_session,1829,2177,0.840,1.10,PASS
    46	mw_grpc_mixed,converge,mac_init,old_committed,1829,2687,0.681,1.10,PASS
    47	mw_grpc_mixed,converge,win_init,old_session,1842,2177,0.846,1.10,PASS
    48	mw_grpc_mixed,converge,win_init,old_committed,1842,2687,0.686,1.10,PASS
    49	mw_grpc_mixed,cross,worst_arm,min_old_committed,1842,1408,1.308,1.10,FAIL
    50	mw_grpc_small,invariance,mac_init,win_init,2261,2227,1.015,1.10,PASS
    51	mw_grpc_small,converge,mac_init,old_session,2261,2942,0.769,1.10,PASS
    52	mw_grpc_small,converge,mac_init,old_committed,2261,2822,0.801,1.10,PASS
    53	mw_grpc_small,converge,win_init,old_session,2227,2942,0.757,1.10,PASS
    54	mw_grpc_small,converge,win_init,old_committed,2227,2822,0.789,1.10,PASS
    55	mw_grpc_small,cross,worst_arm,min_old_committed,2261,1462,1.547,1.10,FAIL
    56	mw_tcp_large,invariance,mac_init,win_init,1914,1920,1.003,1.10,PASS
    57	mw_tcp_large,converge,mac_init,old_session,1914,1908,1.003,1.10,PASS
    58	mw_tcp_large,converge,mac_init,old_committed,1914,3054,0.627,1.10,PASS
    59	mw_tcp_large,converge,win_init,old_session,1920,1908,1.006,1.10,PASS
    60	mw_tcp_large,converge,win_init,old_committed,1920,3054,0.629,1.10,PASS
    61	mw_tcp_large,cross,worst_arm,min_old_committed,1920,1294,1.484,1.10,FAIL
    62	mw_tcp_mixed,invariance,mac_init,win_init,1587,1502,1.057,1.10,PASS
    63	mw_tcp_mixed,converge,mac_init,old_session,1587,1687,0.941,1.10,PASS
    64	mw_tcp_mixed,converge,mac_init,old_committed,1587,2288,0.694,1.10,PASS
    65	mw_tcp_mixed,converge,win_init,old_session,1502,1687,0.890,1.10,PASS
    66	mw_tcp_mixed,converge,win_init,old_committed,1502,2288,0.656,1.10,PASS
    67	mw_tcp_mixed,cross,worst_arm,min_old_committed,1587,1284,1.236,1.10,FAIL
    68	mw_tcp_small,invariance,mac_init,win_init,1922,1935,1.007,1.10,PASS
    69	mw_tcp_small,converge,mac_init,old_session,1922,1811,1.061,1.10,PASS
    70	mw_tcp_small,converge,mac_init,old_committed,1922,1868,1.029,1.10,PASS
    71	mw_tcp_small,converge,win_init,old_session,1935,1811,1.068,1.10,PASS
    72	mw_tcp_small,converge,win_init,old_committed,1935,1868,1.036,1.10,PASS
    73	mw_tcp_small,cross,worst_arm,min_old_committed,1935,1280,1.512,1.10,FAIL
    74	wm_grpc_large,invariance,mac_init,win_init,964,993,1.030,1.10,PASS
    75	wm_grpc_large,converge,mac_init,old_session,964,978,0.986,1.10,PASS
    76	wm_grpc_large,converge,mac_init,old_committed,964,1289,0.748,1.10,PASS
    77	wm_grpc_large,converge,win_init,old_session,993,978,1.015,1.10,PASS
    78	wm_grpc_large,converge,win_init,old_committed,993,1289,0.770,1.10,PASS
    79	wm_grpc_large,cross,worst_arm,min_old_committed,993,1289,0.770,1.10,PASS
    80	wm_grpc_mixed,invariance,mac_init,win_init,1246,1262,1.013,1.10,PASS
    81	wm_grpc_mixed,converge,mac_init,old_session,1246,1408,0.885,1.10,PASS
    82	wm_grpc_mixed,converge,mac_init,old_committed,1246,1408,0.885,1.10,PASS
    83	wm_grpc_mixed,converge,win_init,old_session,1262,1408,0.896,1.10,PASS
    84	wm_grpc_mixed,converge,win_init,old_committed,1262,1408,0.896,1.10,PASS
    85	wm_grpc_mixed,cross,worst_arm,min_old_committed,1262,1408,0.896,1.10,PASS
    86	wm_grpc_small,invariance,mac_init,win_init,1375,1326,1.037,1.10,PASS
    87	wm_grpc_small,converge,mac_init,old_session,1375,1525,0.902,1.10,PASS
    88	wm_grpc_small,converge,mac_init,old_committed,1375,1462,0.940,1.10,PASS
    89	wm_grpc_small,converge,win_init,old_session,1326,1525,0.870,1.10,PASS
    90	wm_grpc_small,converge,win_init,old_committed,1326,1462,0.907,1.10,PASS
    91	wm_grpc_small,cross,worst_arm,min_old_committed,1375,1462,0.940,1.10,PASS
    92	wm_tcp_large,invariance,mac_init,win_init,962,984,1.023,1.10,PASS
    93	wm_tcp_large,converge,mac_init,old_session,962,964,0.998,1.10,PASS
    94	wm_tcp_large,converge,mac_init,old_committed,962,1294,0.743,1.10,PASS
    95	wm_tcp_large,converge,win_init,old_session,984,964,1.021,1.10,PASS
    96	wm_tcp_large,converge,win_init,old_committed,984,1294,0.760,1.10,PASS
    97	wm_tcp_large,cross,worst_arm,min_old_committed,984,1294,0.760,1.10,PASS
    98	wm_tcp_mixed,invariance,mac_init,win_init,1127,911,1.237,1.10,FAIL
    99	wm_tcp_mixed,converge,mac_init,old_session,1127,867,1.300,1.10,FAIL
   100	wm_tcp_mixed,converge,mac_init,old_committed,1127,1284,0.878,1.10,PASS
   101	wm_tcp_mixed,converge,win_init,old_session,911,867,1.051,1.10,PASS
   102	wm_tcp_mixed,converge,win_init,old_committed,911,1284,0.710,1.10,PASS
   103	wm_tcp_mixed,cross,worst_arm,min_old_committed,1127,1284,0.878,1.10,PASS
   104	wm_tcp_small,invariance,mac_init,win_init,1253,1267,1.011,1.10,PASS
   105	wm_tcp_small,converge,mac_init,old_session,1253,1360,0.921,1.10,PASS
   106	wm_tcp_small,converge,mac_init,old_committed,1253,1280,0.979,1.10,PASS
   107	wm_tcp_small,converge,win_init,old_session,1267,1360,0.932,1.10,PASS
   108	wm_tcp_small,converge,win_init,old_committed,1267,1280,0.990,1.10,PASS
   109	wm_tcp_small,cross,worst_arm,min_old_committed,1267,1280,0.990,1.10,PASS
   110	gap_tcp_large,cross-gap,old_push,old_pull,1908,964,1.979,,RECORDED
   111	gap_tcp_large,cross-gap,new_mw_worst,new_wm_worst,1920,984,1.951,,RECORDED
   112	gap_tcp_small,cross-gap,old_push,old_pull,1811,1360,1.332,,RECORDED
   113	gap_tcp_small,cross-gap,new_mw_worst,new_wm_worst,1935,1267,1.527,,RECORDED
   114	gap_tcp_mixed,cross-gap,old_push,old_pull,1687,867,1.946,,RECORDED
   115	gap_tcp_mixed,cross-gap,new_mw_worst,new_wm_worst,1587,1127,1.408,,RECORDED
   116	gap_grpc_large,cross-gap,old_push,old_pull,1913,978,1.956,,RECORDED
   117	gap_grpc_large,cross-gap,new_mw_worst,new_wm_worst,1931,993,1.945,,RECORDED
   118	gap_grpc_small,cross-gap,old_push,old_pull,2942,1525,1.929,,RECORDED
   119	gap_grpc_small,cross-gap,new_mw_worst,new_wm_worst,2261,1375,1.644,,RECORDED
   120	gap_grpc_mixed,cross-gap,old_push,old_pull,2177,1408,1.546,,RECORDED
   121	gap_grpc_mixed,cross-gap,new_mw_worst,new_wm_worst,1842,1262,1.460,,RECORDED

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12_win.sh | sed -n '770,1060p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   770	    gen_fixtures
   771	    setup_host
   772	    mac_daemon_start
   773	    smoke
   774	    stage_pull_sources
   775	
   776	    local w flag carrier
   777	    # BLOCK 1 — converge-up (old vs new, Mac-initiated).
   778	    for w in large small mixed; do
   779	        for carrier in tcp grpc; do
   780	            [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
   781	            CUR_W="$w"; CUR_FLAG="$flag"
   782	            if want_cell "push_${carrier}_${w}"; then
   783	                run_pair_loop "push_${carrier}_${w}" old new b1_push_old b1_push_new
   784	            fi
   785	            if want_cell "pull_${carrier}_${w}"; then
   786	                run_pair_loop "pull_${carrier}_${w}" old new b1_pull_old b1_pull_new
   787	            fi
   788	        done
   789	    done
   790	
   791	    # BLOCK 2 — invariance (mac_init vs win_init, new pair only).
   792	    win_ensure new
   793	    for w in large small mixed; do
   794	        for carrier in tcp grpc; do
   795	            [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
   796	            CUR_W="$w"; CUR_FLAG="$flag"
   797	            if want_cell "mw_${carrier}_${w}"; then
   798	                run_pair_loop "mw_${carrier}_${w}" mac_init win_init b2_mw_mac b2_mw_win
   799	            fi
   800	            if want_cell "wm_${carrier}_${w}"; then
   801	                run_pair_loop "wm_${carrier}_${w}" mac_init win_init b2_wm_mac b2_wm_win
   802	            fi
   803	        done
   804	    done
   805	
   806	    if [[ -n "$CELLS" ]]; then
   807	        local c
   808	        for c in ${CELLS//,/ }; do
   809	            # Header excluded — CELLS=cell must not match "cell,…"
   810	            # (codex otp-12b F10).
   811	            tail -n +2 "$META" | grep -q "^$c," \
   812	                || die "CELLS entry '$c' matched no comparison — nothing was measured for it"
   813	        done
   814	    fi
   815	
   816	    win_daemon_stop
   817	    mac_daemon_stop
   818	    compute_verdicts
   819	
   820	    log ""
   821	    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
   822	    column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
   823	    log ""
   824	    log "=== VERDICTS (D2 both-references; invariance; F4 cross + gap rows) ==="
   825	    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
   826	    log "runs: $CSV"
   827	}
   828	
   829	SESSION_TAG=$(date +%H%M%S).$$
   830	trap on_exit EXIT
   831	T_MS=0; T_RC=0
   832	main "$@"

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12_win.sh | sed -n '350,535p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   350	        kill "$p"
   351	    fi
   352	    rm -f "$MAC_WORK/bench-daemon.pid"
   353	}
   354	
   355	sweep_win_push_dirs() {
   356	    wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\push_${SESSION_TAG}_*' -ErrorAction SilentlyContinue" || true
   357	}
   358	on_exit() {
   359	    if [[ "$WIN_DAEMON_STARTED" == 1 ]]; then win_daemon_stop; sweep_win_push_dirs; fi
   360	    [[ "$MAC_DAEMON_STARTED" == 1 ]] && mac_daemon_stop
   361	    rm -rf "$MAC_WORK/dst_pull_${SESSION_TAG}_"* "$MAC_MODULE_ROOT/push_${SESSION_TAG}_"* 2>/dev/null || true
   362	}
   363	
   364	# --- Drain + cold caches -------------------------------------------------
   365	drain_host() {
   366	    wssh '$ErrorActionPreference = "Stop"
   367	Write-VolumeCache '"$WIN_DRIVE"'
   368	$quiet = 0
   369	for ($i = 0; $i -lt 60; $i++) {
   370	  $w = (Get-Counter "\PhysicalDisk(_Total)\Disk Write Bytes/sec" -SampleInterval 2 -MaxSamples 1).CounterSamples[0].CookedValue
   371	  if ($null -ne $w -and [double]$w -lt 1048576) { $quiet++ } else { $quiet = 0 }
   372	  if ($quiet -ge 3) { "drained $(($i+1)*2)s"; exit 0 }
   373	}
   374	"DRAIN-TIMEOUT"'
   375	}
   376	RUN_DRAIN=""
   377	drop_caches() {   # $1 = run label; sets RUN_DRAIN (pair-voiding, D2)
   378	    local outcome
   379	    outcome=$(drain_host || true)
   380	    RUN_DRAIN=${outcome:-DRAIN-ERROR}
   381	    # Strip CRs BEFORE this lands in runs.csv: pwsh emits CRLF, and a
   382	    # bare \r mid-row makes python's universal-newline csv reader split
   383	    # the row before its `valid` field — every comparison then reads
   384	    # INCOMPLETE (found live: the whole first e21cf84 session verdicted
   385	    # INCOMPLETE off 196 perfectly valid runs).
   386	    RUN_DRAIN=${RUN_DRAIN//$'\r'/}
   387	    RUN_DRAIN=${RUN_DRAIN// /_}
   388	    echo "$1: $RUN_DRAIN" >> "$OUT_DIR/drain.log"
   389	    [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: $1 window UNDRAINED ($RUN_DRAIN) — pair voided, will re-run"
   390	    sync
   391	    sudo -n /usr/sbin/purge
   392	    wssh "pwsh -NoProfile -File '$WIN_TEST\\purge-standby.ps1'" >/dev/null
   393	}
   394	
   395	# --- Fixtures (shape-verified; the otp-12a F2 rule) ----------------------
   396	FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
   397	FIX_COUNT_small=10000; FIX_BYTES_small=40960000
   398	FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
   399	fixture_shape() {
   400	    find "$1" -type f -exec stat -f%z {} + 2>/dev/null \
   401	        | awk '{ s += $1 } END { printf "%d,%d\n", NR, s }'
   402	}
   403	verify_fixture() {
   404	    local w="$1" want_count want_bytes got
   405	    want_count=$(eval echo "\$FIX_COUNT_$w")
   406	    want_bytes=$(eval echo "\$FIX_BYTES_$w")
   407	    got=$(fixture_shape "$MAC_WORK/src_$w")
   408	    [[ "$got" == "$want_count,$want_bytes" ]] \
   409	        || die "fixture src_$w has shape $got, want $want_count,$want_bytes — remove $MAC_WORK/src_$w and re-run"
   410	}
   411	gen_fixtures() {
   412	    if [[ ! -d "$MAC_WORK/src_large" ]]; then
   413	        mkdir -p "$MAC_WORK/src_large"
   414	        dd if=/dev/urandom of="$MAC_WORK/src_large/large_1024M.bin" bs=1m count=1024 2>/dev/null
   415	    fi
   416	    if [[ ! -d "$MAC_WORK/src_small" ]]; then
   417	        mkdir -p "$MAC_WORK/src_small"
   418	        for i in $(seq 1 10000); do
   419	            d="$MAC_WORK/src_small/d$(( i / 1000 ))"; mkdir -p "$d"
   420	            dd if=/dev/urandom of="$d/f${i}.dat" bs=4096 count=1 2>/dev/null
   421	        done
   422	    fi
   423	    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
   424	        mkdir -p "$MAC_WORK/src_mixed"
   425	        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
   426	        for i in $(seq 1 5000); do
   427	            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
   428	            dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null
   429	        done
   430	    fi
   431	    local w
   432	    for w in large small mixed; do verify_fixture "$w"; done
   433	    log "fixtures verified (count + byte sum)"
   434	}
   435	
   436	win_module_count() {   # $1 = subpath under the module; prints file count
   437	    wssh "(Get-ChildItem -Path '$WIN_MODULE\\$1' -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count" | tr -cd '0-9'
   438	}
   439	stage_pull_sources() {
   440	    # Shared across arms by design (D5); verified by remote file count;
   441	    # staged with the NEW pair; the same trees serve block 1 pulls and
   442	    # block 2 win_init pushes (one physical source per direction, F6).
   443	    log "staging pull sources on the Windows module (untimed, new pair)"
   444	    win_ensure new
   445	    local w want got
   446	    for w in large small mixed; do
   447	        want=$(eval echo "\$FIX_COUNT_$w")
   448	        got=$(win_module_count "pull_src_$w\\src_$w"); got=${got:-0}
   449	        if [[ "$got" == "$want" ]]; then
   450	            log "  pull_src_$w verified ($got files, kept)"
   451	            continue
   452	        fi
   453	        log "  pull_src_$w has $got/$want files — (re)staging"
   454	        "$NEW_BLIT" copy "$MAC_WORK/src_$w" "${WIN_REMOTE}pull_src_$w/" --yes \
   455	            > /dev/null 2> "$OUT_DIR/blit-logs/stage_$w.err" \
   456	            || die "staging pull_src_$w failed"
   457	        got=$(win_module_count "pull_src_$w\\src_$w"); got=${got:-0}
   458	        [[ "$got" == "$want" ]] || die "pull_src_$w still wrong after staging ($got/$want)"
   459	        log "  staged pull_src_$w ($got files)"
   460	    done
   461	}
   462	
   463	# --- Timed runs -----------------------------------------------------------
   464	CSV="$OUT_DIR/runs.csv"
   465	echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
   466	META="$OUT_DIR/meta.csv"
   467	echo "cell,pairs_attempted,complete" > "$META"
   468	
   469	RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
   470	
   471	# Mac-initiated runs (block 1 both arms; block 2 mac_init arms).
   472	mac_push_run() {   # blit_bin cell rid dest_remote src [flags...]
   473	    local blit="$1" cell="$2" rid="$3" dest="$4" src="$5"; shift 5
   474	    local start end rc=0
   475	    drop_caches "${cell}-$rid"
   476	    start=$(now_ms)
   477	    "$blit" copy "$src" "${dest}push_${SESSION_TAG}_${cell}_${rid}/" --yes "$@" \
   478	        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
   479	    end=$(now_ms)
   480	    if [[ "$dest" == "$WIN_REMOTE" ]]; then
   481	        RUN_FLUSH=$(flush_win_ms)
   482	        wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\push_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
   483	    else
   484	        RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
   485	        rm -rf "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}"
   486	    fi
   487	    RUN_VALID=yes
   488	    [[ "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
   489	    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc
   490	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   491	}
   492	mac_pull_run() {   # blit_bin cell rid remote_src [flags...]
   493	    local blit="$1" cell="$2" rid="$3" rsrc="$4"; shift 4
   494	    local start end rc=0
   495	    local dst="$MAC_WORK/dst_pull_${SESSION_TAG}_${cell}_${rid}"
   496	    mkdir -p "$dst"
   497	    drop_caches "${cell}-$rid"
   498	    start=$(now_ms)
   499	    "$blit" copy "$rsrc" "$dst" --yes "$@" \
   500	        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
   501	    end=$(now_ms)
   502	    RUN_FLUSH=$(fsync_tree_ms "$dst")
   503	    rm -rf "$dst"
   504	    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc; RUN_VALID=yes
   505	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   506	}
   507	# Windows-initiated runs (block 2 win_init arms): the transfer window is
   508	# a Stopwatch ON Windows printing "<ms>,<exit>"; CRLF-stripped.
   509	win_client_run() {   # cell rid src dst flags_string; sets T_MS/T_RC
   510	    # Sentinel-framed (codex otp-12b F7): anything but a clean
   511	    # "R:<ms>,<rc>:R" — pwsh noise, a crash, a negative exit — parses
   512	    # to T_RC=99 and voids the run; nothing can masquerade as a time.
   513	    local cell="$1" rid="$2" src="$3" dst="$4" flags="${5:-}"
   514	    local out
   515	    # ${rc} braces are load-bearing: PowerShell parses bare `$rc:R` as
   516	    # a SCOPE-qualified variable (like $env:PATH), so the sentinel
   517	    # never printed and every win-initiated run read rc=99 (found live
   518	    # at the first win->mac smoke, 2026-07-12).
   519	    out=$(wssh "\$sw = [Diagnostics.Stopwatch]::StartNew(); & '$WIN_BLIT' copy '$src' '$dst' --yes $flags > \$null 2> '$WIN_TEST\\client-err.log'; \$rc = \$LASTEXITCODE; \$sw.Stop(); \"R:\$([int]\$sw.Elapsed.TotalMilliseconds),\${rc}:R\"" \
   520	        | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)
   521	    if [[ "$out" == *,* ]]; then T_MS=${out%%,*}; T_RC=${out##*,}; else T_MS=0; T_RC=99; fi
   522	    if [[ "$T_RC" != 0 ]]; then
   523	        wssh "Get-Content '$WIN_TEST\\client-err.log' -ErrorAction SilentlyContinue | Select-Object -First 20" \
   524	            > "$OUT_DIR/blit-logs/${cell}_${rid}.err" 2>&1 || true
   525	    fi
   526	}
   527	win_pull_run() {   # cell rid remote_src(from mac) [flag]; dest = win module
   528	    local cell="$1" rid="$2" rsrc="$3" flag="${4:-}"
   529	    drop_caches "${cell}-$rid"
   530	    win_client_run "$cell" "$rid" "$rsrc" "$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}" "$flag"
   531	    RUN_FLUSH=$(flush_win_ms)
   532	    wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
   533	    RUN_VALID=yes
   534	    [[ "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
   535	    RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC

exec
/bin/zsh -lc "rg -n 'verdict|cross-gap|invariance|combined|old_committed|new_mw_worst|median|spread_pct|summary.csv|valid' scripts/bench_otp12_win.sh" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
13:#   summary.csv), per design D2 as amended.
15:#   BLOCK 2 — initiator/verb invariance (NEW pair only; the owner's
33:# counterbalance, pair-void valid-run rule with 2xRUNS cap + INCOMPLETE,
102:BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2w-baseline-2026-07-10/summary.csv"
383:    # the row before its `valid` field — every comparison then reads
384:    # INCOMPLETE (found live: the whole first e21cf84 session verdicted
385:    # INCOMPLETE off 196 perfectly valid runs).
465:echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,valid" > "$CSV"
553:    local slot=1 attempts=0 valid=0 max_attempts=$(( 2 * RUNS ))
555:    while (( valid < RUNS && attempts < max_attempts )); do
557:        local order pair_valid=yes arm fn rid rowA="" rowB=""
568:            [[ "$RUN_VALID" == yes ]] || pair_valid=no
578:        echo "$rowA,$pair_valid" >> "$CSV"
579:        echo "$rowB,$pair_valid" >> "$CSV"
580:        if [[ "$pair_valid" == yes ]]; then
581:            valid=$(( valid + 1 )); slot=$(( slot + 1 ))
586:    if (( valid < RUNS )); then
588:        log "  $cell INCOMPLETE: $valid/$RUNS valid pairs after $attempts attempts"
639:compute_verdicts() {
640:    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
642:runs_p, meta_p, base_p, summary_p, verdicts_p = sys.argv[1:6]
645:base = {r["cell"]: int(r["median_ms"]) for r in csv.DictReader(open(base_p))}
650:    if r["valid"] == "yes":
655:def median(v):
668:out = open(verdicts_p, "w")
672:    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
678:        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
682:    return median(by_arm[(cell, arm)]) if (cell, arm) in by_arm else None
688:        out.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
696:    out.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
697:    combined = ("PASS" if p1 and p2 else "FAIL-REFERENCE-DRIFT" if p1
699:    out.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
701:# Block 2: invariance + per-arm converge (F3) + cross rows (F4) +
706:        out.write(f"{cell},invariance,mac_init,win_init,,,,1.10,INCOMPLETE\n")
711:    out.write(f"{cell},invariance,mac_init,win_init,{a},{b},{hi/lo:.3f},1.10,{'PASS' if inv else 'FAIL'}\n")
715:    # COMPLETE (codex otp-12b F4 — a partial median never referees),
729:        out.write(f"{cell},converge,{armname},old_committed,{val},{ref_m},{val/ref_m:.3f},1.10,{'PASS' if bar(val, ref_m) else 'FAIL'}\n")
738:    out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")
752:        out.write(f"gap_{carrier}_{fixture},cross-gap,old_push,old_pull,"
754:        out.write(f"gap_{carrier}_{fixture},cross-gap,new_mw_worst,new_wm_worst,"
791:    # BLOCK 2 — invariance (mac_init vs win_init, new pair only).
818:    compute_verdicts
821:    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
822:    column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
824:    log "=== VERDICTS (D2 both-references; invariance; F4 cross + gap rows) ==="
825:    column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"

exec
/bin/zsh -lc 'file docs/bench/otp12-win-2026-07-12/*' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/bench/otp12-win-2026-07-12/README.md:            Unicode text, UTF-8 text
docs/bench/otp12-win-2026-07-12/drain-outcomes.txt:   ASCII text, with CRLF line terminators
docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv:    CSV text
docs/bench/otp12-win-2026-07-12/runs.csv:             CSV text
docs/bench/otp12-win-2026-07-12/staging-manifest.txt: CSV text
docs/bench/otp12-win-2026-07-12/summary.csv:          CSV text
docs/bench/otp12-win-2026-07-12/verdicts.csv:         CSV text

exec
/bin/zsh -lc "sed -n '640,760p' scripts/bench_otp12_win.sh | nl -ba -v640" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   640	    python3 - "$CSV" "$META" "$BASELINE_SUMMARY" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" <<'PYEOF'
   641	import csv, sys
   642	runs_p, meta_p, base_p, summary_p, verdicts_p = sys.argv[1:6]
   643	rows = list(csv.DictReader(open(runs_p)))
   644	meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
   645	base = {r["cell"]: int(r["median_ms"]) for r in csv.DictReader(open(base_p))}
   646	
   647	by_arm, voided = {}, {}
   648	for r in rows:
   649	    key = (r["cell"], r["arm"])
   650	    if r["valid"] == "yes":
   651	        by_arm.setdefault(key, []).append(int(r["ms"]))
   652	    else:
   653	        voided[key] = voided.get(key, 0) + 1
   654	
   655	def median(v):
   656	    v = sorted(v); n = len(v)
   657	    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
   658	
   659	def complete(cell):
   660	    if cell not in meta or meta[cell]["complete"] != "yes":
   661	        return False
   662	    arms = [a for (c, a) in by_arm if c == cell]
   663	    return len(arms) == 2
   664	
   665	def bar(new, ref):   # new <= ref * 1.10, integer-exact
   666	    return 10 * new <= 11 * ref
   667	
   668	out = open(verdicts_p, "w")
   669	out.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
   670	
   671	with open(summary_p, "w") as f:
   672	    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
   673	    for (cell, arm) in sorted(by_arm):
   674	        if not complete(cell):
   675	            continue
   676	        v = by_arm[(cell, arm)]
   677	        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
   678	        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
   679	                f"{voided.get((cell, arm), 0)},{meta[cell]['pairs_attempted']}\n")
   680	
   681	def m(cell, arm):
   682	    return median(by_arm[(cell, arm)]) if (cell, arm) in by_arm else None
   683	
   684	# Block 1: converge-up, both references (12a logic verbatim).
   685	b1_cells = sorted(c for c in meta if c.split("_")[0] in ("push", "pull"))
   686	for cell in b1_cells:
   687	    if not complete(cell):
   688	        out.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
   689	        continue
   690	    new_m, old_m = m(cell, "new"), m(cell, "old")
   691	    if cell not in base:
   692	        sys.exit(f"FATAL: no committed reference row for {cell}")
   693	    ref_m = base[cell]
   694	    p1, p2 = bar(new_m, old_m), bar(new_m, ref_m)
   695	    out.write(f"{cell},converge,new,old_session,{new_m},{old_m},{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
   696	    out.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
   697	    combined = ("PASS" if p1 and p2 else "FAIL-REFERENCE-DRIFT" if p1
   698	                else "FAIL-SAME-SESSION" if p2 else "FAIL-BOTH")
   699	    out.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
   700	
   701	# Block 2: invariance + per-arm converge (F3) + cross rows (F4) +
   702	# discriminator gap rows (D-2026-07-12-1; recorded, not adjudicated).
   703	b2_cells = sorted(c for c in meta if c.split("_")[0] in ("mw", "wm"))
   704	for cell in b2_cells:
   705	    if not complete(cell):
   706	        out.write(f"{cell},invariance,mac_init,win_init,,,,1.10,INCOMPLETE\n")
   707	        continue
   708	    a, b = m(cell, "mac_init"), m(cell, "win_init")
   709	    hi, lo = max(a, b), min(a, b)
   710	    inv = bar(hi, lo)   # max/min <= 1.10
   711	    out.write(f"{cell},invariance,mac_init,win_init,{a},{b},{hi/lo:.3f},1.10,{'PASS' if inv else 'FAIL'}\n")
   712	    # F3: each arm independently meets the direction's converge bars.
   713	    # Committed references are MANDATORY (fail closed, codex otp-12b
   714	    # F8); the same-session reference requires the block-1 counterpart
   715	    # COMPLETE (codex otp-12b F4 — a partial median never referees),
   716	    # else the row says so in registered vocabulary.
   717	    d, carrier, fixture = cell.split("_")
   718	    verb = "push" if d == "mw" else "pull"
   719	    b1 = f"{verb}_{carrier}_{fixture}"
   720	    if b1 not in base:
   721	        sys.exit(f"FATAL: no committed reference row for {b1} (needed by {cell})")
   722	    ref_m = base[b1]
   723	    old_sess = m(b1, "old") if complete(b1) else None
   724	    for armname, val in (("mac_init", a), ("win_init", b)):
   725	        if old_sess is not None:
   726	            out.write(f"{cell},converge,{armname},old_session,{val},{old_sess},{val/old_sess:.3f},1.10,{'PASS' if bar(val, old_sess) else 'FAIL'}\n")
   727	        else:
   728	            out.write(f"{cell},converge,{armname},old_session,{val},,,1.10,NO-SAME-SESSION-REF\n")
   729	        out.write(f"{cell},converge,{armname},old_committed,{val},{ref_m},{val/ref_m:.3f},1.10,{'PASS' if bar(val, ref_m) else 'FAIL'}\n")
   730	    # F4 cross: each direction vs min of the two committed old
   731	    # directions for this fixture x carrier (mandatory, fail closed).
   732	    p_ref = base.get(f"push_{carrier}_{fixture}")
   733	    l_ref = base.get(f"pull_{carrier}_{fixture}")
   734	    if p_ref is None or l_ref is None:
   735	        sys.exit(f"FATAL: committed push/pull reference missing for {carrier}_{fixture}")
   736	    cross_ref = min(p_ref, l_ref)
   737	    worst = max(a, b)
   738	    out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")
   739	
   740	# Discriminator gap rows (D-2026-07-12-1; recorded, never adjudicated):
   741	# emitted only when ALL FOUR contributing cells are complete (codex
   742	# otp-12b F4). Row operands are labeled exactly (codex otp-12b F12).
   743	for carrier in ("tcp", "grpc"):
   744	    for fixture in ("large", "small", "mixed"):
   745	        four = [f"push_{carrier}_{fixture}", f"pull_{carrier}_{fixture}",
   746	                f"mw_{carrier}_{fixture}", f"wm_{carrier}_{fixture}"]
   747	        if not all(complete(c) for c in four):
   748	            continue
   749	        po, lo_ = m(four[0], "old"), m(four[1], "old")
   750	        mw_w = max(m(four[2], "mac_init"), m(four[2], "win_init"))
   751	        wm_w = max(m(four[3], "mac_init"), m(four[3], "win_init"))
   752	        out.write(f"gap_{carrier}_{fixture},cross-gap,old_push,old_pull,"
   753	                  f"{po},{lo_},{po/lo_:.3f},,RECORDED\n")
   754	        out.write(f"gap_{carrier}_{fixture},cross-gap,new_mw_worst,new_wm_worst,"
   755	                  f"{mw_w},{wm_w},{mw_w/wm_w:.3f},,RECORDED\n")
   756	out.close()
   757	PYEOF
   758	}
   759	
   760	# --- Matrix ----------------------------------------------------------------

exec
/bin/zsh -lc "git show f19776c:scripts/bench_otp12_win.sh | sed -n '500,525p' | nl -ba -v500" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc "git show f19776c:scripts/bench_otp12_win.sh | sed -n '640,760p' | nl -ba -v640" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-1UbEieLA' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-TdGSu5yL' (errno=Operation not permitted)
   500	}
   501	# Windows-initiated runs (block 2 win_init arms): the transfer window is
   502	# a Stopwatch ON Windows printing "<ms>,<exit>"; CRLF-stripped.
   503	win_client_run() {   # cell rid src dst flags_string; sets T_MS/T_RC
   504	    # Sentinel-framed (codex otp-12b F7): anything but a clean
   505	    # "R:<ms>,<rc>:R" — pwsh noise, a crash, a negative exit — parses
   506	    # to T_RC=99 and voids the run; nothing can masquerade as a time.
   507	    local cell="$1" rid="$2" src="$3" dst="$4" flags="${5:-}"
   508	    local out
   509	    out=$(wssh "\$sw = [Diagnostics.Stopwatch]::StartNew(); & '$WIN_BLIT' copy '$src' '$dst' --yes $flags > \$null 2> '$WIN_TEST\\client-err.log'; \$rc = \$LASTEXITCODE; \$sw.Stop(); \"R:\$([int]\$sw.Elapsed.TotalMilliseconds),\$rc:R\"" \
   510	        | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)
   511	    if [[ "$out" == *,* ]]; then T_MS=${out%%,*}; T_RC=${out##*,}; else T_MS=0; T_RC=99; fi
   512	    if [[ "$T_RC" != 0 ]]; then
   513	        wssh "Get-Content '$WIN_TEST\\client-err.log' -ErrorAction SilentlyContinue | Select-Object -First 20" \
   514	            > "$OUT_DIR/blit-logs/${cell}_${rid}.err" 2>&1 || true
   515	    fi
   516	}
   517	win_pull_run() {   # cell rid remote_src(from mac) [flag]; dest = win module
   518	    local cell="$1" rid="$2" rsrc="$3" flag="${4:-}"
   519	    drop_caches "${cell}-$rid"
   520	    win_client_run "$cell" "$rid" "$rsrc" "$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}" "$flag"
   521	    RUN_FLUSH=$(flush_win_ms)
   522	    wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
   523	    RUN_VALID=yes
   524	    [[ "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
   525	    RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-E56BppAz' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-PzDp8W9a' (errno=Operation not permitted)
   640	    if r["valid"] == "yes":
   641	        by_arm.setdefault(key, []).append(int(r["ms"]))
   642	    else:
   643	        voided[key] = voided.get(key, 0) + 1
   644	
   645	def median(v):
   646	    v = sorted(v); n = len(v)
   647	    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
   648	
   649	def complete(cell):
   650	    if cell not in meta or meta[cell]["complete"] != "yes":
   651	        return False
   652	    arms = [a for (c, a) in by_arm if c == cell]
   653	    return len(arms) == 2
   654	
   655	def bar(new, ref):   # new <= ref * 1.10, integer-exact
   656	    return 10 * new <= 11 * ref
   657	
   658	out = open(verdicts_p, "w")
   659	out.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome\n")
   660	
   661	with open(summary_p, "w") as f:
   662	    f.write("cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted\n")
   663	    for (cell, arm) in sorted(by_arm):
   664	        if not complete(cell):
   665	            continue
   666	        v = by_arm[(cell, arm)]
   667	        spread = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
   668	        f.write(f"{cell},{arm},{median(v)},{sum(v)//len(v)},{min(v)},{spread},"
   669	                f"{voided.get((cell, arm), 0)},{meta[cell]['pairs_attempted']}\n")
   670	
   671	def m(cell, arm):
   672	    return median(by_arm[(cell, arm)]) if (cell, arm) in by_arm else None
   673	
   674	# Block 1: converge-up, both references (12a logic verbatim).
   675	b1_cells = sorted(c for c in meta if c.split("_")[0] in ("push", "pull"))
   676	for cell in b1_cells:
   677	    if not complete(cell):
   678	        out.write(f"{cell},converge,new,combined,,,,1.10,INCOMPLETE\n")
   679	        continue
   680	    new_m, old_m = m(cell, "new"), m(cell, "old")
   681	    if cell not in base:
   682	        sys.exit(f"FATAL: no committed reference row for {cell}")
   683	    ref_m = base[cell]
   684	    p1, p2 = bar(new_m, old_m), bar(new_m, ref_m)
   685	    out.write(f"{cell},converge,new,old_session,{new_m},{old_m},{new_m/old_m:.3f},1.10,{'PASS' if p1 else 'FAIL'}\n")
   686	    out.write(f"{cell},converge,new,old_committed,{new_m},{ref_m},{new_m/ref_m:.3f},1.10,{'PASS' if p2 else 'FAIL'}\n")
   687	    combined = ("PASS" if p1 and p2 else "FAIL-REFERENCE-DRIFT" if p1
   688	                else "FAIL-SAME-SESSION" if p2 else "FAIL-BOTH")
   689	    out.write(f"{cell},converge,new,combined,{new_m},,,1.10,{combined}\n")
   690	
   691	# Block 2: invariance + per-arm converge (F3) + cross rows (F4) +
   692	# discriminator gap rows (D-2026-07-12-1; recorded, not adjudicated).
   693	b2_cells = sorted(c for c in meta if c.split("_")[0] in ("mw", "wm"))
   694	for cell in b2_cells:
   695	    if not complete(cell):
   696	        out.write(f"{cell},invariance,mac_init,win_init,,,,1.10,INCOMPLETE\n")
   697	        continue
   698	    a, b = m(cell, "mac_init"), m(cell, "win_init")
   699	    hi, lo = max(a, b), min(a, b)
   700	    inv = bar(hi, lo)   # max/min <= 1.10
   701	    out.write(f"{cell},invariance,mac_init,win_init,{a},{b},{hi/lo:.3f},1.10,{'PASS' if inv else 'FAIL'}\n")
   702	    # F3: each arm independently meets the direction's converge bars.
   703	    # Committed references are MANDATORY (fail closed, codex otp-12b
   704	    # F8); the same-session reference requires the block-1 counterpart
   705	    # COMPLETE (codex otp-12b F4 — a partial median never referees),
   706	    # else the row says so in registered vocabulary.
   707	    d, carrier, fixture = cell.split("_")
   708	    verb = "push" if d == "mw" else "pull"
   709	    b1 = f"{verb}_{carrier}_{fixture}"
   710	    if b1 not in base:
   711	        sys.exit(f"FATAL: no committed reference row for {b1} (needed by {cell})")
   712	    ref_m = base[b1]
   713	    old_sess = m(b1, "old") if complete(b1) else None
   714	    for armname, val in (("mac_init", a), ("win_init", b)):
   715	        if old_sess is not None:
   716	            out.write(f"{cell},converge,{armname},old_session,{val},{old_sess},{val/old_sess:.3f},1.10,{'PASS' if bar(val, old_sess) else 'FAIL'}\n")
   717	        else:
   718	            out.write(f"{cell},converge,{armname},old_session,{val},,,1.10,NO-SAME-SESSION-REF\n")
   719	        out.write(f"{cell},converge,{armname},old_committed,{val},{ref_m},{val/ref_m:.3f},1.10,{'PASS' if bar(val, ref_m) else 'FAIL'}\n")
   720	    # F4 cross: each direction vs min of the two committed old
   721	    # directions for this fixture x carrier (mandatory, fail closed).
   722	    p_ref = base.get(f"push_{carrier}_{fixture}")
   723	    l_ref = base.get(f"pull_{carrier}_{fixture}")
   724	    if p_ref is None or l_ref is None:
   725	        sys.exit(f"FATAL: committed push/pull reference missing for {carrier}_{fixture}")
   726	    cross_ref = min(p_ref, l_ref)
   727	    worst = max(a, b)
   728	    out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")
   729	
   730	# Discriminator gap rows (D-2026-07-12-1; recorded, never adjudicated):
   731	# emitted only when ALL FOUR contributing cells are complete (codex
   732	# otp-12b F4). Row operands are labeled exactly (codex otp-12b F12).
   733	for carrier in ("tcp", "grpc"):
   734	    for fixture in ("large", "small", "mixed"):
   735	        four = [f"push_{carrier}_{fixture}", f"pull_{carrier}_{fixture}",
   736	                f"mw_{carrier}_{fixture}", f"wm_{carrier}_{fixture}"]
   737	        if not all(complete(c) for c in four):
   738	            continue
   739	        po, lo_ = m(four[0], "old"), m(four[1], "old")
   740	        mw_w = max(m(four[2], "mac_init"), m(four[2], "win_init"))
   741	        wm_w = max(m(four[3], "mac_init"), m(four[3], "win_init"))
   742	        out.write(f"gap_{carrier}_{fixture},cross-gap,old_push,old_pull,"
   743	                  f"{po},{lo_},{po/lo_:.3f},,RECORDED\n")
   744	        out.write(f"gap_{carrier}_{fixture},cross-gap,new_mw_worst,new_wm_worst,"
   745	                  f"{mw_w},{wm_w},{mw_w/wm_w:.3f},,RECORDED\n")
   746	out.close()
   747	PYEOF
   748	}
   749	
   750	# --- Matrix ----------------------------------------------------------------
   751	main() {
   752	    preflight
   753	    write_manifest
   754	    if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
   755	        log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
   756	        exit 0
   757	    fi
   758	    log "session $SESSION_TAG  old=$OLD_SHA new=$NEW_SHA  win: $WIN_HOST  mac: $MAC_HOST"
   759	
   760	    gen_fixtures

exec
/bin/zsh -lc "rg -n -C 5 'straddl|spread exceeds|RUNS=8|escalation trigger' docs scripts .review --glob '"'!docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv'"'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
scripts/bench_otp12_zoey.sh-31-#   * Exit codes checked (the old harness swallowed them inside the
scripts/bench_otp12_zoey.sh-32-#     timed window); per-run blit output kept under $OUT_DIR/blit-logs/.
scripts/bench_otp12_zoey.sh-33-#   * verdicts.csv computed at the end against both references
scripts/bench_otp12_zoey.sh-34-#     (PASS / FAIL-SAME-SESSION / FAIL-REFERENCE-DRIFT / FAIL-BOTH /
scripts/bench_otp12_zoey.sh-35-#     INCOMPLETE, per design D2).
scripts/bench_otp12_zoey.sh:36:#   * Escalation (manual, design D2): a comparison that straddles its
scripts/bench_otp12_zoey.sh-37-#     bar with either arm's spread > 25% is re-run in a fresh session
scripts/bench_otp12_zoey.sh:38:#     at RUNS=8; both sessions get committed.
scripts/bench_otp12_zoey.sh-39-#
scripts/bench_otp12_zoey.sh-40-# Usage (from the client Mac):
scripts/bench_otp12_zoey.sh-41-#   export ZOEY_SSH=root@zoey
scripts/bench_otp12_zoey.sh-42-#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
scripts/bench_otp12_zoey.sh-43-#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
--
scripts/bench_otp12_zoey.sh-78-ZOEY_HOST=${ZOEY_HOST:-10.1.10.206}
scripts/bench_otp12_zoey.sh-79-PORT=${PORT:-9031}
scripts/bench_otp12_zoey.sh-80-RUNS=${RUNS:-4}
scripts/bench_otp12_zoey.sh-81-PREFLIGHT_ONLY=${PREFLIGHT_ONLY:-0}
scripts/bench_otp12_zoey.sh-82-# Comma-separated comparison allowlist for the D2 escalation rule
scripts/bench_otp12_zoey.sh:83:# (straddle + spread>25% -> fresh session at RUNS=8 for JUST those
scripts/bench_otp12_zoey.sh-84-# comparisons; both sessions committed). Empty = the full matrix.
scripts/bench_otp12_zoey.sh-85-CELLS=${CELLS:-}
scripts/bench_otp12_zoey.sh-86-# Real-disk client workdir. NOT /tmp: keep the client end on APFS SSD.
scripts/bench_otp12_zoey.sh-87-MAC_WORK=${MAC_WORK:-$HOME/blit-bench-work}
scripts/bench_otp12_zoey.sh-88-
--
scripts/bench_otp12_win.sh-46-#   export WIN_HOST=10.1.10.173
scripts/bench_otp12_win.sh-47-#   export WIN_TEST='D:\blit-test'
scripts/bench_otp12_win.sh-48-#   export MAC_HOST=<the Mac's 10GbE IP>      # required, no default
scripts/bench_otp12_win.sh-49-#   RUNS=4 ./scripts/bench_otp12_win.sh
scripts/bench_otp12_win.sh-50-#   PREFLIGHT_ONLY=1 ./scripts/bench_otp12_win.sh
scripts/bench_otp12_win.sh:51:#   CELLS=<comma-list> RUNS=8 ./scripts/bench_otp12_win.sh   # escalation
scripts/bench_otp12_win.sh-52-#
scripts/bench_otp12_win.sh-53-# Staging prerequisites (the rig session does these before preflight):
scripts/bench_otp12_win.sh-54-#   * Mac: clean tree at the run commit; `cargo build --release` (client
scripts/bench_otp12_win.sh-55-#     AND daemon — the Mac daemon serves block 2); old client rebuilt at
scripts/bench_otp12_win.sh-56-#     $OLD_SHA in a detached worktree -> $MAC_WORK/bins/blit-$OLD_SHA.
--
.review/findings/otp-12a-zoey-harness.md-62-  go. First live session may surface busybox/ssh quirks the otp-2 script
.review/findings/otp-12a-zoey-harness.md-63-  did not (pgrep availability, sha256sum path).
.review/findings/otp-12a-zoey-harness.md-64-- Old-arm provenance rests on the staging record + sha256 manifest, not
.review/findings/otp-12a-zoey-harness.md-65-  a handshake (pre-handshake binaries) — accepted residual risk per the
.review/findings/otp-12a-zoey-harness.md-66-  design doc.
.review/findings/otp-12a-zoey-harness.md:67:- The escalation rule (straddle + spread > 25% → RUNS=8 fresh session)
.review/findings/otp-12a-zoey-harness.md-68-  is manual by design, not automated in the script.
.review/findings/otp-12a-zoey-harness.md-69-- `meta.csv` (pairs-attempted/completeness) is a working file consumed
.review/findings/otp-12a-zoey-harness.md-70-  by the verdict pass; the committed evidence carries its content via
.review/findings/otp-12a-zoey-harness.md-71-  `summary.csv`'s `pairs_attempted` column and the INCOMPLETE verdict
.review/findings/otp-12a-zoey-harness.md-72-  rows.
--
docs/plan/OTP12_ACCEPTANCE_RUN.md-173-  identically); gap closed ⇒ the code was the cost and the bar is met. The
docs/plan/OTP12_ACCEPTANCE_RUN.md-174-  README records BOTH computations per cell; a discriminator-attributed
docs/plan/OTP12_ACCEPTANCE_RUN.md-175-  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
docs/plan/OTP12_ACCEPTANCE_RUN.md-176-  the otp-13 walk reviews the recorded numbers.
docs/plan/OTP12_ACCEPTANCE_RUN.md-177-
docs/plan/OTP12_ACCEPTANCE_RUN.md:178:Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
docs/plan/OTP12_ACCEPTANCE_RUN.md:179:bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
docs/plan/OTP12_ACCEPTANCE_RUN.md-180-interleaved in a fresh session; both sessions are committed.
docs/plan/OTP12_ACCEPTANCE_RUN.md-181-**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
docs/plan/OTP12_ACCEPTANCE_RUN.md:182:text defined the trigger but not which session governs): the RUNS=8
docs/plan/OTP12_ACCEPTANCE_RUN.md-183-escalation session's medians govern the escalated comparison's combined
docs/plan/OTP12_ACCEPTANCE_RUN.md:184:outcome — more data where noise or a straddle made RUNS=4 undecidable is
docs/plan/OTP12_ACCEPTANCE_RUN.md-185-the escalation's entire purpose. The RUNS=4 rows stay committed and
docs/plan/OTP12_ACCEPTANCE_RUN.md-186-visible; the otp-13 walk sees both sessions.**
docs/plan/OTP12_ACCEPTANCE_RUN.md-187-
docs/plan/OTP12_ACCEPTANCE_RUN.md-188-### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
docs/plan/OTP12_ACCEPTANCE_RUN.md-189-
--
docs/bench/otp12-win-2026-07-12/README.md-44-| cell | new | old same-session | ratio | committed | ratio | outcome |
docs/bench/otp12-win-2026-07-12/README.md-45-|------|----:|----:|----:|----:|----:|---------|
docs/bench/otp12-win-2026-07-12/README.md-46-| push_tcp_small | 2080 | 1811 | **1.149** | 1868 | 1.113 | FAIL-BOTH (spreads 3.8/3.0% — real) |
docs/bench/otp12-win-2026-07-12/README.md-47-| pull_tcp_mixed | 1138 | 867 | **1.313** | 1284 | 0.886 | FAIL-SAME-SESSION (spreads 5.2/6.7%) |
docs/bench/otp12-win-2026-07-12/README.md-48-
docs/bench/otp12-win-2026-07-12/README.md:49:No pre-registered escalation trigger fires (no straddle with >25%
docs/bench/otp12-win-2026-07-12/README.md-50-spread — these are tight-spread results); both stand recorded for the
docs/bench/otp12-win-2026-07-12/README.md-51-otp-13 walk. Rig context: today's old arms run far FASTER than their
docs/bench/otp12-win-2026-07-12/README.md-52-2026-07-10 committed medians (e.g. old pull_tcp_mixed 867 vs 1284, old
docs/bench/otp12-win-2026-07-12/README.md-53-push_tcp_large 1908 vs 3054) — reference drift in the fast direction,
docs/bench/otp12-win-2026-07-12/README.md-54-so the committed bars are easy and the same-session bars are the
--
docs/bench/otp12-zoey-2026-07-12/README.md-9-Governs); it records the computed D2 comparisons.
docs/bench/otp12-zoey-2026-07-12/README.md-10-
docs/bench/otp12-zoey-2026-07-12/README.md-11-**Harness**: `scripts/bench_otp12_zoey.sh` (methodology inherited from
docs/bench/otp12-zoey-2026-07-12/README.md-12-the frozen `bench_otp2_baseline.sh`; new mechanics — ABBA counterbalance,
docs/bench/otp12-zoey-2026-07-12/README.md-13-pair-void valid-run rule, both-reference verdicts — per the design doc
docs/bench/otp12-zoey-2026-07-12/README.md:14:D1/D2/D5). RUNS=4 main session, RUNS=8 escalation (the pre-registered D2
docs/bench/otp12-zoey-2026-07-12/README.md-15-rule). Zero voided pairs in any recorded session.
docs/bench/otp12-zoey-2026-07-12/README.md-16-
docs/bench/otp12-zoey-2026-07-12/README.md-17-## Builds (matched pairs, clean trees, sha-embedded — manifests committed)
docs/bench/otp12-zoey-2026-07-12/README.md-18-
docs/bench/otp12-zoey-2026-07-12/README.md-19-- **old arm**: clean `e757dcc` rebuilds BOTH ends (Mac client via
--
docs/bench/otp12-zoey-2026-07-12/README.md-52-   each destination right after its flush is measured (outside the timed
docs/bench/otp12-zoey-2026-07-12/README.md-53-   window). No data from this session feeds any verdict.
docs/bench/otp12-zoey-2026-07-12/README.md-54-2. **Main session** (RUNS=4; `runs.csv`/`summary.csv`/`verdicts.csv`):
docs/bench/otp12-zoey-2026-07-12/README.md-55-   full 12-comparison matrix, 48 pairs, all valid. 9/12 PASS both
docs/bench/otp12-zoey-2026-07-12/README.md-56-   references; 3 escalated per the pre-registered D2 rules.
docs/bench/otp12-zoey-2026-07-12/README.md:57:3. **Escalation session** (RUNS=8, `CELLS` allowlist;
docs/bench/otp12-zoey-2026-07-12/README.md-58-   `escalation-*.csv`): the three flagged comparisons re-run fresh.
docs/bench/otp12-zoey-2026-07-12/README.md-59-
docs/bench/otp12-zoey-2026-07-12/README.md-60-## Final per-comparison state (escalation supersedes where run — D2)
docs/bench/otp12-zoey-2026-07-12/README.md-61-
docs/bench/otp12-zoey-2026-07-12/README.md-62-| comparison | new ms | old same-session | ratio | committed | ratio | combined |
docs/bench/otp12-zoey-2026-07-12/README.md-63-|------------|-------:|-----------------:|------:|----------:|------:|----------|
docs/bench/otp12-zoey-2026-07-12/README.md:64:| push_tcp_large  | 2464 | 2570 | 0.959 | 2702 | 0.912 | **PASS** (RUNS=8 governs per the D2 supersession rule, recorded as a dated amendment after this run surfaced the gap — codex otp-12a-run F2; the RUNS=4 session read FAIL-BOTH at 100% new-arm spread and stays committed in `runs.csv`) |
docs/bench/otp12-zoey-2026-07-12/README.md-65-| push_grpc_large | 4567 | 4369 | 1.045 | 4510 | 1.013 | **PASS** |
docs/bench/otp12-zoey-2026-07-12/README.md:66:| pull_tcp_large  | 2167 | 2177 | 0.995 | 1744 | 1.243 | **FAIL-REFERENCE-DRIFT** (persisted at RUNS=8; see Drift) |
docs/bench/otp12-zoey-2026-07-12/README.md-67-| pull_grpc_large | 2702 | 2706 | 0.999 | 2585 | 1.045 | **PASS** |
docs/bench/otp12-zoey-2026-07-12/README.md-68-| push_tcp_small  | 3984 | 3605 | 1.105 | 4263 | 0.935 | **FAIL-SAME-SESSION** (persisted; see the marginal-gap note) |
docs/bench/otp12-zoey-2026-07-12/README.md-69-| push_grpc_small | 4731 | 4727 | 1.001 | 5217 | 0.907 | **PASS** |
docs/bench/otp12-zoey-2026-07-12/README.md-70-| pull_tcp_small  | 2277 | 2266 | 1.005 | 2784 | 0.818 | **PASS** |
docs/bench/otp12-zoey-2026-07-12/README.md-71-| pull_grpc_small | 3148 | 3463 | 0.909 | 4188 | 0.752 | **PASS** |
--
docs/bench/otp12-zoey-2026-07-12/README.md-92-touched the box on 07-11). Per D2 a persisting drift stands recorded,
docs/bench/otp12-zoey-2026-07-12/README.md-93-never silently excused.
docs/bench/otp12-zoey-2026-07-12/README.md-94-
docs/bench/otp12-zoey-2026-07-12/README.md-95-## The marginal same-session gap (push_tcp_small)
docs/bench/otp12-zoey-2026-07-12/README.md-96-
docs/bench/otp12-zoey-2026-07-12/README.md:97:Reproducible across both sessions (1.109 at RUNS=4, **1.105** at RUNS=8
docs/bench/otp12-zoey-2026-07-12/README.md-98-with tight spreads: new 16.7%, old 18.7%) — a real ≈10.5% same-session
docs/bench/otp12-zoey-2026-07-12/README.md-99-gap, 0.5% over the ±10% noise bar, on this cell only. Context the walk
docs/bench/otp12-zoey-2026-07-12/README.md-100-needs (stated per the CSVs, codex otp-12a-run F4): the OLD arm ran
docs/bench/otp12-zoey-2026-07-12/README.md-101-15.4% faster this session than its own committed baseline (3605 vs
docs/bench/otp12-zoey-2026-07-12/README.md-102-4263); the unified path still beats that committed baseline by 6.5%
--
docs/bench/otp12-zoey-2026-07-12/README.md-111-```
docs/bench/otp12-zoey-2026-07-12/README.md-112-export ZOEY_SSH=root@zoey
docs/bench/otp12-zoey-2026-07-12/README.md-113-export ZOEY_TEMP=/volume/<pool-uuid>/.srv/.unifi-drive/michael/.data/blit-temp
docs/bench/otp12-zoey-2026-07-12/README.md-114-export ZOEY_HOST=10.1.10.206
docs/bench/otp12-zoey-2026-07-12/README.md-115-RUNS=4 ./scripts/bench_otp12_zoey.sh                     # full matrix
docs/bench/otp12-zoey-2026-07-12/README.md:116:CELLS=<comma-list> RUNS=8 ./scripts/bench_otp12_zoey.sh  # D2 escalation
docs/bench/otp12-zoey-2026-07-12/README.md-117-PREFLIGHT_ONLY=1 ./scripts/bench_otp12_zoey.sh           # checks only
docs/bench/otp12-zoey-2026-07-12/README.md-118-```
docs/bench/otp12-zoey-2026-07-12/README.md-119-
docs/bench/otp12-zoey-2026-07-12/README.md-120-Requires: clean tree at the run commit; old client staged at
docs/bench/otp12-zoey-2026-07-12/README.md-121-`~/blit-bench-work/bins/blit-e757dcc`; both sha-named daemons staged in
--
.review/results/otp-12b-run.codex.md-9-reasoning effort: ultra
.review/results/otp-12b-run.codex.md-10-reasoning summaries: none
.review/results/otp-12b-run.codex.md-11-session id: 019f57aa-a8db-78c3-bad5-f6257d74d5b5
.review/results/otp-12b-run.codex.md-12---------
.review/results/otp-12b-run.codex.md-13-user
.review/results/otp-12b-run.codex.md:14:Review the commit range f19776c..44c2046 (run: git log --oneline f19776c..44c2046; git diff f19776c..44c2046). It is otp-12b's RECORDED-RUN half per docs/plan/OTP12_ACCEPTANCE_RUN.md (Active): (1) e21cf84 fixes the win-initiated sentinel - pwsh parses bare $rc:R as a scope-qualified variable so it never printed (found at first rig contact; a manual reproduction proved the win->mac path itself works); (2) 856af64 strips CRs from the drain outcome before runs.csv - a bare \r mid-row split every row under python universal newlines and verdicted 192 valid runs INCOMPLETE; (3) 44c2046 commits docs/bench/otp12-win-2026-07-12/ (README + sanitized runs.csv + the raw CRLF original + summary/verdicts/drain/manifest). CHECK HARD: (a) recompute the README's headline numbers from runs.csv (medians over valid rows, floor-of-mean-of-middle-two): the invariance table (11/12 PASS, wm_tcp_mixed 1.237), the two converge fails (push_tcp_small 2080/1811/1868; pull_tcp_mixed 1138/867/1284), the cross rows and the gap rows (old_push/old_pull vs new_mw_worst/new_wm_worst per fixture x carrier) against docs/bench/otp2w-baseline-2026-07-10/summary.csv; (b) is the CR post-processing legitimate and fully disclosed (raw file committed; no timing value altered - verify runs.csv and runs-raw-crlf.csv differ ONLY by \r bytes); (c) does the README avoid declaring pass/fail and avoid self-adjudicating the D-2026-07-12-1 residue (gap rows RECORDED; the platform-attribution language); (d) the escalation reasoning: no pre-registered trigger fires (verify the fails have tight spreads and are not straddles per the design doc D2's rule); (e) the cross-block consistency note (block-1 push_tcp_small 2080 vs block-2 mw_tcp_small mac_init 1922) - is it honestly recorded and is any conclusion improperly drawn; (f) any overclaim in calling wm_tcp_mixed/push_tcp_small code-shaped findings. Output a concise markdown findings list - each finding with file:line, severity, rationale - then a final VERDICT line. Be concise; do not invoke skills.
.review/results/otp-12b-run.codex.md-15-codex
.review/results/otp-12b-run.codex.md-16-I’ll inspect the exact range and independently recompute the recorded-run evidence, with separate passes for arithmetic, raw-file integrity, and claims against the acceptance design. I’ll make no changes.
.review/results/otp-12b-run.codex.md-17-exec
.review/results/otp-12b-run.codex.md-18-/bin/zsh -lc "sed -n '1,240p' .agents/repo-guidance.md" in /Users/michael/Dev/blit_v2
.review/results/otp-12b-run.codex.md-19- succeeded in 0ms:
--
.review/results/otp-12b-run.codex.md-339-    44	| cell | new | old same-session | ratio | committed | ratio | outcome |
.review/results/otp-12b-run.codex.md-340-    45	|------|----:|----:|----:|----:|----:|---------|
.review/results/otp-12b-run.codex.md-341-    46	| push_tcp_small | 2080 | 1811 | **1.149** | 1868 | 1.113 | FAIL-BOTH (spreads 3.8/3.0% — real) |
.review/results/otp-12b-run.codex.md-342-    47	| pull_tcp_mixed | 1138 | 867 | **1.313** | 1284 | 0.886 | FAIL-SAME-SESSION (spreads 5.2/6.7%) |
.review/results/otp-12b-run.codex.md-343-    48	
.review/results/otp-12b-run.codex.md:344:    49	No pre-registered escalation trigger fires (no straddle with >25%
.review/results/otp-12b-run.codex.md-345-    50	spread — these are tight-spread results); both stand recorded for the
.review/results/otp-12b-run.codex.md-346-    51	otp-13 walk. Rig context: today's old arms run far FASTER than their
.review/results/otp-12b-run.codex.md-347-    52	2026-07-10 committed medians (e.g. old pull_tcp_mixed 867 vs 1284, old
.review/results/otp-12b-run.codex.md-348-    53	push_tcp_large 1908 vs 3054) — reference drift in the fast direction,
.review/results/otp-12b-run.codex.md-349-    54	so the committed bars are easy and the same-session bars are the
--
.review/results/otp-12b-run.codex.md-586-docs/plan/OTP12_ACCEPTANCE_RUN.md-173-  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12b-run.codex.md-587-docs/plan/OTP12_ACCEPTANCE_RUN.md-174-  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12b-run.codex.md-588-docs/plan/OTP12_ACCEPTANCE_RUN.md:175:  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12b-run.codex.md-589-docs/plan/OTP12_ACCEPTANCE_RUN.md-176-  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12b-run.codex.md-590-docs/plan/OTP12_ACCEPTANCE_RUN.md-177-
.review/results/otp-12b-run.codex.md:591:docs/plan/OTP12_ACCEPTANCE_RUN.md:178:Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12b-run.codex.md:592:docs/plan/OTP12_ACCEPTANCE_RUN.md:179:bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12b-run.codex.md-593-docs/plan/OTP12_ACCEPTANCE_RUN.md-180-interleaved in a fresh session; both sessions are committed.
.review/results/otp-12b-run.codex.md-594-docs/plan/OTP12_ACCEPTANCE_RUN.md-181-**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
.review/results/otp-12b-run.codex.md:595:docs/plan/OTP12_ACCEPTANCE_RUN.md:182:text defined the trigger but not which session governs): the RUNS=8
.review/results/otp-12b-run.codex.md-596-docs/plan/OTP12_ACCEPTANCE_RUN.md:183:escalation session's medians govern the escalated comparison's combined
.review/results/otp-12b-run.codex.md:597:docs/plan/OTP12_ACCEPTANCE_RUN.md:184:outcome — more data where noise or a straddle made RUNS=4 undecidable is
.review/results/otp-12b-run.codex.md-598-docs/plan/OTP12_ACCEPTANCE_RUN.md:185:the escalation's entire purpose. The RUNS=4 rows stay committed and
.review/results/otp-12b-run.codex.md-599-docs/plan/OTP12_ACCEPTANCE_RUN.md-186-visible; the otp-13 walk sees both sessions.**
.review/results/otp-12b-run.codex.md-600-docs/plan/OTP12_ACCEPTANCE_RUN.md-187-
.review/results/otp-12b-run.codex.md-601-docs/plan/OTP12_ACCEPTANCE_RUN.md-188-### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
.review/results/otp-12b-run.codex.md-602-docs/plan/OTP12_ACCEPTANCE_RUN.md-189-
--
.review/results/otp-12b-run.codex.md-1014-   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12b-run.codex.md-1015-   174	  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12b-run.codex.md-1016-   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12b-run.codex.md-1017-   176	  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12b-run.codex.md-1018-   177	
.review/results/otp-12b-run.codex.md:1019:   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12b-run.codex.md:1020:   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12b-run.codex.md-1021-   180	interleaved in a fresh session; both sessions are committed.
.review/results/otp-12b-run.codex.md-1022-   181	**Supersession (amended 2026-07-12, codex otp-12a-run F2 — the original
.review/results/otp-12b-run.codex.md:1023:   182	text defined the trigger but not which session governs): the RUNS=8
.review/results/otp-12b-run.codex.md-1024-   183	escalation session's medians govern the escalated comparison's combined
.review/results/otp-12b-run.codex.md:1025:   184	outcome — more data where noise or a straddle made RUNS=4 undecidable is
.review/results/otp-12b-run.codex.md-1026-   185	the escalation's entire purpose. The RUNS=4 rows stay committed and
.review/results/otp-12b-run.codex.md-1027-   186	visible; the otp-13 walk sees both sessions.**
.review/results/otp-12b-run.codex.md-1028-   187	
.review/results/otp-12b-run.codex.md-1029-   188	### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
.review/results/otp-12b-run.codex.md-1030-   189	
--
.review/results/otp-12a-run.codex.md-9-reasoning effort: ultra
.review/results/otp-12a-run.codex.md-10-reasoning summaries: none
.review/results/otp-12a-run.codex.md-11-session id: 019f5706-3f71-7d13-a9ec-0830e0ae2dbd
.review/results/otp-12a-run.codex.md-12---------
.review/results/otp-12a-run.codex.md-13-user
.review/results/otp-12a-run.codex.md:14:Review the commit range 775b6b5..b0ebf73 (run: git log --oneline 775b6b5..b0ebf73 and git diff 775b6b5..b0ebf73). It is the RECORDED-RUN half of otp-12a per docs/plan/OTP12_ACCEPTANCE_RUN.md (Active): (1) b2b6901 corrects the otp-2 evidence README + .agents/machines.md — the daemon staged on zoey embedded 731023bfc8a1.dirty, not e757dcc as recorded (verify the correction's claims are internally consistent and that git diff 731023b e757dcc -- crates proto is indeed empty); (2) harness fixes earned live: LC_ALL=C grep -qa provenance checks (BSD grep binary-match gap), old-daemon default re-pointed at the clean sha-named rebuild, exec bit, per-run push-destination sweep after the measured flush (an I/O-backlog storm on the daemon host — load 444, 10x run times both arms — correlated with accumulated destinations; probes with per-run deletion held at baseline), and a CELLS allowlist implementing the design's pre-registered D2 escalation; (3) b0ebf73 commits docs/bench/otp12-zoey-2026-07-12/ — README + both sessions' CSVs + the aborted storm session. CHECK HARD: does the README faithfully report the CSVs (recompute the medians/ratios in the final table from runs.csv and escalation-runs.csv: medians over valid rows only, floor-of-mean-of-middle-two, ratios vs old_session and vs docs/bench/otp2-baseline-2026-07-10/summary.csv); does the escalation-supersedes rule match the design doc D2; is the per-run sweep methodologically sound (deletion outside the timed window, next run's drain absorbs it) and does the README avoid declaring pass/fail (checkpoints are owner-only); is the drift analysis's rig-side attribution arithmetic right (old arm 2177 vs committed 1744); any acceptance-criterion drift or overclaim; the marginal push_tcp_small gap reported honestly. Note RUNS=4 and RUNS=8 mid-matrix commits changed the new-arm sha between sessions (042c06f vs 6bc9cb6) — the README claims the inter-session diff is harness-script-only; verify via git diff 042c06f 6bc9cb6 --stat. Output a concise markdown findings list - each finding with file:line, severity, rationale - then a final VERDICT line. Be concise; do not invoke skills.
.review/results/otp-12a-run.codex.md-15-codex
.review/results/otp-12a-run.codex.md-16-I’ll audit the range read-only, recompute the reported statistics from source CSVs, and independently check the provenance, design, and methodology claims. I’ll return only actionable findings plus a verdict.
.review/results/otp-12a-run.codex.md-17-mcp: ptk/ptk_invoke started
.review/results/otp-12a-run.codex.md-18-mcp: ptk/ptk_invoke (completed)
.review/results/otp-12a-run.codex.md-19-mcp: ptk/ptk_invoke started
--
.review/results/otp-12a-run.codex.md-83-mcp: ptk/ptk_invoke started
.review/results/otp-12a-run.codex.md-84-mcp: ptk/ptk_invoke (completed)
.review/results/otp-12a-run.codex.md-85-codex
.review/results/otp-12a-run.codex.md-86-- [scripts/bench_otp12_zoey.sh:164](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:164) — **HIGH** — The provenance check searches for a 7-character SHA anywhere. The old-client artifact matching the committed manifest contains no `0.1.0+e757dcc`; its sole match is an embedded build-directory path. Dirty same-SHA builds also pass, so clean old-arm provenance is not established.
.review/results/otp-12a-run.codex.md-87-
.review/results/otp-12a-run.codex.md:88:- [docs/bench/otp12-zoey-2026-07-12/README.md:51](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:51) — **MEDIUM** — D2 defines the RUNS=8 trigger but no supersession rule. Discarding the RUNS=4 `FAIL-BOTH` and rolling `push_tcp_large` up as final `PASS` is post-run adjudication. Its best RUNS=4 result, 2597 ms, also beat only the committed 2702 ms baseline—not the 2418 ms same-session old median.
.review/results/otp-12a-run.codex.md-89-
.review/results/otp-12a-run.codex.md-90-- [docs/bench/otp2-baseline-2026-07-10/README.md:23](/Users/michael/Dev/blit_v2/docs/bench/otp2-baseline-2026-07-10/README.md:23) — **MEDIUM** — The correction asserts the dirt was harness/docs, then admits its contents are unprovable. Consequently [the “provably rig-side” claim](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:73) overreaches. The arithmetic (`1.248`, `0.995`) is correct, but only establishes correlation and same-session parity.
.review/results/otp-12a-run.codex.md-91-
.review/results/otp-12a-run.codex.md-92-- [docs/bench/otp12-zoey-2026-07-12/README.md:88](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:88) — **LOW** — The marginal-gap context misstates the data: new improved 6.5%, while old improved 15.4%; “~15% … in both arms” is unsupported. Ratios `1.001`, `1.005`, and omitted `1.043` are also slightly behind old, not “at or ahead.”
.review/results/otp-12a-run.codex.md-93-
--
.review/results/otp-12a-run.codex.md-98-**VERDICT: FAIL — medians, ratios, requested diffs, executable bit, and sweep methodology check out, but provenance and D2 supersession are blocking findings.**
.review/results/otp-12a-run.codex.md-99-tokens used
.review/results/otp-12a-run.codex.md-100-144,573
.review/results/otp-12a-run.codex.md-101-- [scripts/bench_otp12_zoey.sh:164](/Users/michael/Dev/blit_v2/scripts/bench_otp12_zoey.sh:164) — **HIGH** — The provenance check searches for a 7-character SHA anywhere. The old-client artifact matching the committed manifest contains no `0.1.0+e757dcc`; its sole match is an embedded build-directory path. Dirty same-SHA builds also pass, so clean old-arm provenance is not established.
.review/results/otp-12a-run.codex.md-102-
.review/results/otp-12a-run.codex.md:103:- [docs/bench/otp12-zoey-2026-07-12/README.md:51](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:51) — **MEDIUM** — D2 defines the RUNS=8 trigger but no supersession rule. Discarding the RUNS=4 `FAIL-BOTH` and rolling `push_tcp_large` up as final `PASS` is post-run adjudication. Its best RUNS=4 result, 2597 ms, also beat only the committed 2702 ms baseline—not the 2418 ms same-session old median.
.review/results/otp-12a-run.codex.md-104-
.review/results/otp-12a-run.codex.md-105-- [docs/bench/otp2-baseline-2026-07-10/README.md:23](/Users/michael/Dev/blit_v2/docs/bench/otp2-baseline-2026-07-10/README.md:23) — **MEDIUM** — The correction asserts the dirt was harness/docs, then admits its contents are unprovable. Consequently [the “provably rig-side” claim](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:73) overreaches. The arithmetic (`1.248`, `0.995`) is correct, but only establishes correlation and same-session parity.
.review/results/otp-12a-run.codex.md-106-
.review/results/otp-12a-run.codex.md-107-- [docs/bench/otp12-zoey-2026-07-12/README.md:88](/Users/michael/Dev/blit_v2/docs/bench/otp12-zoey-2026-07-12/README.md:88) — **LOW** — The marginal-gap context misstates the data: new improved 6.5%, while old improved 15.4%; “~15% … in both arms” is unsupported. Ratios `1.001`, `1.005`, and omitted `1.043` are also slightly behind old, not “at or ahead.”
.review/results/otp-12a-run.codex.md-108-
--
.review/results/otp-12a.codex.md-389-+  go. First live session may surface busybox/ssh quirks the otp-2 script
.review/results/otp-12a.codex.md-390-+  did not (pgrep availability, sha256sum path).
.review/results/otp-12a.codex.md-391-+- Old-arm provenance rests on the staging record + sha256 manifest, not
.review/results/otp-12a.codex.md-392-+  a handshake (pre-handshake binaries) — accepted residual risk per the
.review/results/otp-12a.codex.md-393-+  design doc.
.review/results/otp-12a.codex.md:394:+- The escalation rule (straddle + spread > 25% → RUNS=8 fresh session)
.review/results/otp-12a.codex.md-395-+  is manual by design, not automated in the script.
.review/results/otp-12a.codex.md-396-+- `meta.csv` (pairs-attempted/completeness) is a working file consumed
.review/results/otp-12a.codex.md-397-+  by the verdict pass; the committed evidence carries its content via
.review/results/otp-12a.codex.md-398-+  `summary.csv`'s `pairs_attempted` column and the INCOMPLETE verdict
.review/results/otp-12a.codex.md-399-+  rows.
--
.review/results/otp-12a.codex.md-452-+#   * Exit codes checked (the old harness swallowed them inside the
.review/results/otp-12a.codex.md-453-+#     timed window); per-run blit output kept under $OUT_DIR/blit-logs/.
.review/results/otp-12a.codex.md-454-+#   * verdicts.csv computed at the end against both references
.review/results/otp-12a.codex.md-455-+#     (PASS / FAIL-SAME-SESSION / FAIL-REFERENCE-DRIFT / FAIL-BOTH /
.review/results/otp-12a.codex.md-456-+#     INCOMPLETE, per design D2).
.review/results/otp-12a.codex.md:457:+#   * Escalation (manual, design D2): a comparison that straddles its
.review/results/otp-12a.codex.md-458-+#     bar with either arm's spread > 25% is re-run in a fresh session
.review/results/otp-12a.codex.md:459:+#     at RUNS=8; both sessions get committed.
.review/results/otp-12a.codex.md-460-+#
.review/results/otp-12a.codex.md-461-+# Usage (from the client Mac):
.review/results/otp-12a.codex.md-462-+#   export ZOEY_SSH=root@zoey
.review/results/otp-12a.codex.md-463-+#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
.review/results/otp-12a.codex.md-464-+#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
--
.review/results/otp-12a.codex.md-965-    31	#   * Exit codes checked (the old harness swallowed them inside the
.review/results/otp-12a.codex.md-966-    32	#     timed window); per-run blit output kept under $OUT_DIR/blit-logs/.
.review/results/otp-12a.codex.md-967-    33	#   * verdicts.csv computed at the end against both references
.review/results/otp-12a.codex.md-968-    34	#     (PASS / FAIL-SAME-SESSION / FAIL-REFERENCE-DRIFT / FAIL-BOTH /
.review/results/otp-12a.codex.md-969-    35	#     INCOMPLETE, per design D2).
.review/results/otp-12a.codex.md:970:    36	#   * Escalation (manual, design D2): a comparison that straddles its
.review/results/otp-12a.codex.md-971-    37	#     bar with either arm's spread > 25% is re-run in a fresh session
.review/results/otp-12a.codex.md:972:    38	#     at RUNS=8; both sessions get committed.
.review/results/otp-12a.codex.md-973-    39	#
.review/results/otp-12a.codex.md-974-    40	# Usage (from the client Mac):
.review/results/otp-12a.codex.md-975-    41	#   export ZOEY_SSH=root@zoey
.review/results/otp-12a.codex.md-976-    42	#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
.review/results/otp-12a.codex.md-977-    43	#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
--
.review/results/otp-12a.codex.md-1596-   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12a.codex.md-1597-   174	  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12a.codex.md-1598-   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12a.codex.md-1599-   176	  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12a.codex.md-1600-   177	
.review/results/otp-12a.codex.md:1601:   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12a.codex.md:1602:   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12a.codex.md-1603-   180	interleaved in a fresh session; both sessions are committed.
.review/results/otp-12a.codex.md-1604-   181	
.review/results/otp-12a.codex.md-1605-   182	### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
.review/results/otp-12a.codex.md-1606-   183	
.review/results/otp-12a.codex.md-1607-   184	For a FIXED data direction the two initiators are:
--
.review/results/otp-12a.codex.md-3053-    62	  go. First live session may surface busybox/ssh quirks the otp-2 script
.review/results/otp-12a.codex.md-3054-    63	  did not (pgrep availability, sha256sum path).
.review/results/otp-12a.codex.md-3055-    64	- Old-arm provenance rests on the staging record + sha256 manifest, not
.review/results/otp-12a.codex.md-3056-    65	  a handshake (pre-handshake binaries) — accepted residual risk per the
.review/results/otp-12a.codex.md-3057-    66	  design doc.
.review/results/otp-12a.codex.md:3058:    67	- The escalation rule (straddle + spread > 25% → RUNS=8 fresh session)
.review/results/otp-12a.codex.md-3059-    68	  is manual by design, not automated in the script.
.review/results/otp-12a.codex.md-3060-    69	- `meta.csv` (pairs-attempted/completeness) is a working file consumed
.review/results/otp-12a.codex.md-3061-    70	  by the verdict pass; the committed evidence carries its content via
.review/results/otp-12a.codex.md-3062-    71	  `summary.csv`'s `pairs_attempted` column and the INCOMPLETE verdict
.review/results/otp-12a.codex.md-3063-    72	  rows.
--
.review/results/otp-12a.codex.md-3487-   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12a.codex.md-3488-   174	  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12a.codex.md-3489-   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12a.codex.md-3490-   176	  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12a.codex.md-3491-   177	
.review/results/otp-12a.codex.md:3492:   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12a.codex.md:3493:   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12a.codex.md-3494-   180	interleaved in a fresh session; both sessions are committed.
.review/results/otp-12a.codex.md-3495-   181	
.review/results/otp-12a.codex.md-3496-   182	### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
.review/results/otp-12a.codex.md-3497-   183	
.review/results/otp-12a.codex.md-3498-   184	For a FIXED data direction the two initiators are:
--
.review/results/otp-12a-run.gpt-verdict.md-28-
.review/results/otp-12a-run.gpt-verdict.md-29-Confirmed — the trigger was pre-registered, the governing session was
.review/results/otp-12a-run.gpt-verdict.md-30-not; rolling push_tcp_large up as PASS was post-run adjudication, and
.review/results/otp-12a-run.gpt-verdict.md-31-the README's "its best run beat the old median" claim was wrong (2597
.review/results/otp-12a-run.gpt-verdict.md-32-beat the committed 2702, not the same-session 2418). Fixed: D2 amended
.review/results/otp-12a-run.gpt-verdict.md:33:with a dated supersession rule (RUNS=8 governs; RUNS=4 rows stay
.review/results/otp-12a-run.gpt-verdict.md-34-committed and visible), the README row now cites the amendment's
.review/results/otp-12a-run.gpt-verdict.md-35-provenance honestly, and the wrong claim is corrected. The otp-13 walk
.review/results/otp-12a-run.gpt-verdict.md-36-sees both sessions either way.
.review/results/otp-12a-run.gpt-verdict.md-37-
.review/results/otp-12a-run.gpt-verdict.md-38-## F3 (Medium) — "provably rig-side" / dirt-content overreach

exec
/bin/zsh -lc "git show e21cf84:scripts/bench_otp12_win.sh | sed -n '500,525p' | nl -ba -v500" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Q3yrSoSh' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-yhb8bZO6' (errno=Operation not permitted)
   500	}
   501	# Windows-initiated runs (block 2 win_init arms): the transfer window is
   502	# a Stopwatch ON Windows printing "<ms>,<exit>"; CRLF-stripped.
   503	win_client_run() {   # cell rid src dst flags_string; sets T_MS/T_RC
   504	    # Sentinel-framed (codex otp-12b F7): anything but a clean
   505	    # "R:<ms>,<rc>:R" — pwsh noise, a crash, a negative exit — parses
   506	    # to T_RC=99 and voids the run; nothing can masquerade as a time.
   507	    local cell="$1" rid="$2" src="$3" dst="$4" flags="${5:-}"
   508	    local out
   509	    # ${rc} braces are load-bearing: PowerShell parses bare `$rc:R` as
   510	    # a SCOPE-qualified variable (like $env:PATH), so the sentinel
   511	    # never printed and every win-initiated run read rc=99 (found live
   512	    # at the first win->mac smoke, 2026-07-12).
   513	    out=$(wssh "\$sw = [Diagnostics.Stopwatch]::StartNew(); & '$WIN_BLIT' copy '$src' '$dst' --yes $flags > \$null 2> '$WIN_TEST\\client-err.log'; \$rc = \$LASTEXITCODE; \$sw.Stop(); \"R:\$([int]\$sw.Elapsed.TotalMilliseconds),\${rc}:R\"" \
   514	        | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)
   515	    if [[ "$out" == *,* ]]; then T_MS=${out%%,*}; T_RC=${out##*,}; else T_MS=0; T_RC=99; fi
   516	    if [[ "$T_RC" != 0 ]]; then
   517	        wssh "Get-Content '$WIN_TEST\\client-err.log' -ErrorAction SilentlyContinue | Select-Object -First 20" \
   518	            > "$OUT_DIR/blit-logs/${cell}_${rid}.err" 2>&1 || true
   519	    fi
   520	}
   521	win_pull_run() {   # cell rid remote_src(from mac) [flag]; dest = win module
   522	    local cell="$1" rid="$2" rsrc="$3" flag="${4:-}"
   523	    drop_caches "${cell}-$rid"
   524	    win_client_run "$cell" "$rid" "$rsrc" "$WIN_MODULE\\pull_${SESSION_TAG}_${cell}_${rid}" "$flag"
   525	    RUN_FLUSH=$(flush_win_ms)

exec
/bin/zsh -lc "nl -ba docs/bench/otp12-zoey-2026-07-12/summary.csv | sed -n '1,35p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
     2	pull_grpc_large,new,2702,2745,2685,7.7,0,4
     3	pull_grpc_large,old,2706,2721,2617,9.1,0,4
     4	pull_grpc_mixed,new,2107,3600,1958,320.2,0,4
     5	pull_grpc_mixed,old,2252,2277,2121,17.1,0,4
     6	pull_grpc_small,new,3148,5393,3057,299.7,0,4
     7	pull_grpc_small,old,3463,3461,3408,3.0,0,4
     8	pull_tcp_large,new,1958,1999,1753,32.9,0,4
     9	pull_tcp_large,old,2093,2094,1984,11.3,0,4
    10	pull_tcp_mixed,new,1521,2276,1351,248.8,0,4
    11	pull_tcp_mixed,old,1575,1603,1449,25.3,0,4
    12	pull_tcp_small,new,2277,2220,1772,44.2,0,4
    13	pull_tcp_small,old,2266,4847,2111,503.7,0,4
    14	push_grpc_large,new,4567,4635,4400,13.8,0,4
    15	push_grpc_large,old,4369,4412,4098,17.4,0,4
    16	push_grpc_mixed,new,3468,3460,3352,6.1,0,4
    17	push_grpc_mixed,old,3666,3668,3413,15.1,0,4
    18	push_grpc_small,new,4731,4814,4536,15.9,0,4
    19	push_grpc_small,old,4727,4750,4567,9.0,0,4
    20	push_tcp_large,new,3536,3716,2597,100.1,0,4
    21	push_tcp_large,old,2418,2452,2203,25.7,0,4
    22	push_tcp_mixed,new,2142,2148,1870,30.5,0,4
    23	push_tcp_mixed,old,2053,2049,1874,18.4,0,4
    24	push_tcp_small,new,4033,4021,3814,10.3,0,4
    25	push_tcp_small,old,3636,5838,3500,259.4,0,4

exec
/bin/zsh -lc "nl -ba docs/bench/otp12-zoey-2026-07-12/verdicts.csv | sed -n '1,45p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,bar,outcome
     2	pull_grpc_large,converge,new,old_session,2702,2706,0.999,1.10,PASS
     3	pull_grpc_large,converge,new,old_committed,2702,2585,1.045,1.10,PASS
     4	pull_grpc_large,converge,new,combined,2702,,,1.10,PASS
     5	pull_grpc_mixed,converge,new,old_session,2107,2252,0.936,1.10,PASS
     6	pull_grpc_mixed,converge,new,old_committed,2107,2222,0.948,1.10,PASS
     7	pull_grpc_mixed,converge,new,combined,2107,,,1.10,PASS
     8	pull_grpc_small,converge,new,old_session,3148,3463,0.909,1.10,PASS
     9	pull_grpc_small,converge,new,old_committed,3148,4188,0.752,1.10,PASS
    10	pull_grpc_small,converge,new,combined,3148,,,1.10,PASS
    11	pull_tcp_large,converge,new,old_session,1958,2093,0.935,1.10,PASS
    12	pull_tcp_large,converge,new,old_committed,1958,1744,1.123,1.10,FAIL
    13	pull_tcp_large,converge,new,combined,1958,,,1.10,FAIL-REFERENCE-DRIFT
    14	pull_tcp_mixed,converge,new,old_session,1521,1575,0.966,1.10,PASS
    15	pull_tcp_mixed,converge,new,old_committed,1521,1401,1.086,1.10,PASS
    16	pull_tcp_mixed,converge,new,combined,1521,,,1.10,PASS
    17	pull_tcp_small,converge,new,old_session,2277,2266,1.005,1.10,PASS
    18	pull_tcp_small,converge,new,old_committed,2277,2784,0.818,1.10,PASS
    19	pull_tcp_small,converge,new,combined,2277,,,1.10,PASS
    20	push_grpc_large,converge,new,old_session,4567,4369,1.045,1.10,PASS
    21	push_grpc_large,converge,new,old_committed,4567,4510,1.013,1.10,PASS
    22	push_grpc_large,converge,new,combined,4567,,,1.10,PASS
    23	push_grpc_mixed,converge,new,old_session,3468,3666,0.946,1.10,PASS
    24	push_grpc_mixed,converge,new,old_committed,3468,3889,0.892,1.10,PASS
    25	push_grpc_mixed,converge,new,combined,3468,,,1.10,PASS
    26	push_grpc_small,converge,new,old_session,4731,4727,1.001,1.10,PASS
    27	push_grpc_small,converge,new,old_committed,4731,5217,0.907,1.10,PASS
    28	push_grpc_small,converge,new,combined,4731,,,1.10,PASS
    29	push_tcp_large,converge,new,old_session,3536,2418,1.462,1.10,FAIL
    30	push_tcp_large,converge,new,old_committed,3536,2702,1.309,1.10,FAIL
    31	push_tcp_large,converge,new,combined,3536,,,1.10,FAIL-BOTH
    32	push_tcp_mixed,converge,new,old_session,2142,2053,1.043,1.10,PASS
    33	push_tcp_mixed,converge,new,old_committed,2142,2070,1.035,1.10,PASS
    34	push_tcp_mixed,converge,new,combined,2142,,,1.10,PASS
    35	push_tcp_small,converge,new,old_session,4033,3636,1.109,1.10,FAIL
    36	push_tcp_small,converge,new,old_committed,4033,4263,0.946,1.10,PASS
    37	push_tcp_small,converge,new,combined,4033,,,1.10,FAIL-SAME-SESSION

exec
/bin/zsh -lc 'find docs/bench/otp12-zoey-2026-07-12 -maxdepth 1 -type f -print | sort' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
docs/bench/otp12-zoey-2026-07-12/README.md
docs/bench/otp12-zoey-2026-07-12/aborted-storm-drain.txt
docs/bench/otp12-zoey-2026-07-12/aborted-storm-runs.csv
docs/bench/otp12-zoey-2026-07-12/drain-outcomes.txt
docs/bench/otp12-zoey-2026-07-12/escalation-drain-outcomes.txt
docs/bench/otp12-zoey-2026-07-12/escalation-runs.csv
docs/bench/otp12-zoey-2026-07-12/escalation-staging-manifest.txt
docs/bench/otp12-zoey-2026-07-12/escalation-summary.csv
docs/bench/otp12-zoey-2026-07-12/escalation-verdicts.csv
docs/bench/otp12-zoey-2026-07-12/runs.csv
docs/bench/otp12-zoey-2026-07-12/staging-manifest.txt
docs/bench/otp12-zoey-2026-07-12/summary.csv
docs/bench/otp12-zoey-2026-07-12/verdicts.csv

exec
/bin/zsh -lc "rg -n -C 4 'straddl' .review/results/otp-12a-run.codex.md .review/results/otp-12a-run.gpt-verdict.md .review/findings/otp-12a-run.md 2>/dev/null" in /Users/michael/Dev/blit_v2
 exited 2 in 0ms:
exec
/bin/zsh -lc "nl -ba docs/bench/otp12-zoey-2026-07-12/README.md | sed -n '50,110p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
    50	   within minutes; three back-to-back probes WITH per-run deletion held
    51	   at baseline (2466/2525/3714 ms vs committed 2702). Harness now sweeps
    52	   each destination right after its flush is measured (outside the timed
    53	   window). No data from this session feeds any verdict.
    54	2. **Main session** (RUNS=4; `runs.csv`/`summary.csv`/`verdicts.csv`):
    55	   full 12-comparison matrix, 48 pairs, all valid. 9/12 PASS both
    56	   references; 3 escalated per the pre-registered D2 rules.
    57	3. **Escalation session** (RUNS=8, `CELLS` allowlist;
    58	   `escalation-*.csv`): the three flagged comparisons re-run fresh.
    59	
    60	## Final per-comparison state (escalation supersedes where run — D2)
    61	
    62	| comparison | new ms | old same-session | ratio | committed | ratio | combined |
    63	|------------|-------:|-----------------:|------:|----------:|------:|----------|
    64	| push_tcp_large  | 2464 | 2570 | 0.959 | 2702 | 0.912 | **PASS** (RUNS=8 governs per the D2 supersession rule, recorded as a dated amendment after this run surfaced the gap — codex otp-12a-run F2; the RUNS=4 session read FAIL-BOTH at 100% new-arm spread and stays committed in `runs.csv`) |
    65	| push_grpc_large | 4567 | 4369 | 1.045 | 4510 | 1.013 | **PASS** |
    66	| pull_tcp_large  | 2167 | 2177 | 0.995 | 1744 | 1.243 | **FAIL-REFERENCE-DRIFT** (persisted at RUNS=8; see Drift) |
    67	| pull_grpc_large | 2702 | 2706 | 0.999 | 2585 | 1.045 | **PASS** |
    68	| push_tcp_small  | 3984 | 3605 | 1.105 | 4263 | 0.935 | **FAIL-SAME-SESSION** (persisted; see the marginal-gap note) |
    69	| push_grpc_small | 4731 | 4727 | 1.001 | 5217 | 0.907 | **PASS** |
    70	| pull_tcp_small  | 2277 | 2266 | 1.005 | 2784 | 0.818 | **PASS** |
    71	| pull_grpc_small | 3148 | 3463 | 0.909 | 4188 | 0.752 | **PASS** |
    72	| push_tcp_mixed  | 2142 | 2053 | 1.043 | 2070 | 1.035 | **PASS** |
    73	| push_grpc_mixed | 3468 | 3666 | 0.946 | 3889 | 0.892 | **PASS** |
    74	| pull_tcp_mixed  | 1521 | 1575 | 0.966 | 1401 | 1.086 | **PASS** |
    75	| pull_grpc_mixed | 2107 | 2252 | 0.936 | 2222 | 0.948 | **PASS** |
    76	
    77	Rollup: **10 PASS, 1 FAIL-REFERENCE-DRIFT, 1 FAIL-SAME-SESSION** — both
    78	non-PASS cells carried to the otp-13 walk with the analysis below.
    79	
    80	## Drift analysis (pull_tcp_large)
    81	
    82	The strongest available evidence puts the drift rig-side, not the
    83	unified path's: the OLD arm ran 2177 ms median this session vs the
    84	committed 1744 ms (**1.248×**), while new-vs-old same-session is
    85	**0.995** — whatever slowed large pulls slowed both arms alike, and the
    86	unified path is not slower than the old path on this rig, this day.
    87	This is correlation plus same-session parity, not proof (codex
    88	otp-12a-run F3): the committed baseline's daemon was itself the
    89	mislabeled dirty build (see Builds), so a code-content confound in the
    90	reference cannot be fully excluded, and the rig changed between
    91	2026-07-10 and 2026-07-12 (uptime 22 days; owner-side maintenance
    92	touched the box on 07-11). Per D2 a persisting drift stands recorded,
    93	never silently excused.
    94	
    95	## The marginal same-session gap (push_tcp_small)
    96	
    97	Reproducible across both sessions (1.109 at RUNS=4, **1.105** at RUNS=8
    98	with tight spreads: new 16.7%, old 18.7%) — a real ≈10.5% same-session
    99	gap, 0.5% over the ±10% noise bar, on this cell only. Context the walk
   100	needs (stated per the CSVs, codex otp-12a-run F4): the OLD arm ran
   101	15.4% faster this session than its own committed baseline (3605 vs
   102	4263); the unified path still beats that committed baseline by 6.5%
   103	(3984 vs 4263) but sits 10.5% behind the faster same-session old arm.
   104	The neighboring small/mixed same-session ratios for reference:
   105	pull_grpc_small 0.909, push_grpc_small 1.001, pull_tcp_small 1.005,
   106	push_tcp_mixed 1.043. If a per-cell look is wanted, it is a post-otp-12
   107	item; nothing here blocks otp-12b/c mechanically.
   108	
   109	## Reproduction
   110	

exec
/bin/zsh -lc "python3 -c 'import csv, collections, pathlib, hashlib, statistics
root=pathlib.Path(\"docs/bench/otp12-win-2026-07-12\")
p=root/\"runs.csv\"
raw=(root/\"runs-raw-crlf.csv\").read_bytes()
san=p.read_bytes()
print(\"BYTE_INTEGRITY\")
print(\"raw_bytes\",len(raw),\"san_bytes\",len(san),\"cr_count\",raw.count(b\"\\r\"),\"san_cr_count\",san.count(b\"\\r\"))
print(\"raw_minus_cr_equals_sanitized\",raw.replace(b\"\\r\",b\"\")==san)
print(\"raw_lf\",raw.count(b\"\\n\"),\"san_lf\",san.count(b\"\\n\"))
print(\"sha_raw\",hashlib.sha256(raw).hexdigest())
print(\"sha_san\",hashlib.sha256(san).hexdigest())
rows=list(csv.DictReader(p.open(newline=\"\")))
print(\"\\nROW_INTEGRITY\")
print(\"rows\",len(rows),\"valid_counts\",collections.Counter(r[\"valid\"] for r in rows),\"exit_counts\",collections.Counter(r[\"exit\"] for r in rows))
print(\"drain_counts\",collections.Counter(r[\"drain\"] for r in rows))
rawrows=list(csv.DictReader((root/\"runs-raw-crlf.csv\").open()))
print(\"raw_universal_records\",len(rawrows),\"raw_valid_counts\",collections.Counter(r.get(\"valid\") for r in rawrows))
by=collections.defaultdict(list)
for r in rows:
    if r[\"valid\"]==\"yes\":
        by[(r[\"cell\"],r[\"arm\"])].append(int(r[\"ms\"]))
def med(v):
    s=sorted(v)
    return (s[len(s)//2-1]+s[len(s)//2])//2 if len(s)%2==0 else s[len(s)//2]
def spread(v):
    return round(100*(max(v)-min(v))/max(min(v),1),1)
M={k:med(v) for k,v in by.items()}
base={r[\"cell\"]:int(r[\"median_ms\"]) for r in csv.DictReader(open(\"docs/bench/otp2w-baseline-2026-07-10/summary.csv\"))}
print(\"\\nFAIL_ARM_VALUES\")
for cell,arms in [(\"push_tcp_small\",(\"new\",\"old\")),(\"pull_tcp_mixed\",(\"new\",\"old\")),(\"wm_tcp_mixed\",(\"mac_init\",\"win_init\"))]:
    print(cell)
    for arm in arms:
        v=by[(cell,arm)]
        print(\" \",arm,sorted(v),\"median\",med(v),\"spread\",spread(v))
print(\"\\nINVARIANCE\")
npass=0
for cell in sorted(set(c for c,a in by if c.startswith((\"mw_\",\"wm_\")))):
    a=M[(cell,\"mac_init\")]; b=M[(cell,\"win_init\")]; ratio=max(a,b)/min(a,b); ok=10*max(a,b)<=11*min(a,b)
    npass+=ok
    print(cell,a,b,f\"{ratio:.3f}\",\"PASS\" if ok else \"FAIL\",\"spreads\",spread(by[(cell,\"mac_init\")]),spread(by[(cell,\"win_init\")]))
print(\"invariance_pass\",npass,\"of\",12)
print(\"\\nBLOCK1_COMBINED_NONPASS\")
npass=0
for cell in sorted(set(c for c,a in by if c.startswith((\"push_\",\"pull_\")))):
    n=M[(cell,\"new\")]; o=M[(cell,\"old\")]; b=base[cell]
    p1=10*n<=11*o; p2=10*n<=11*b
    if p1 and p2: outcome=\"PASS\"; npass+=1
    elif p1: outcome=\"FAIL-REFERENCE-DRIFT\"
    elif p2: outcome=\"FAIL-SAME-SESSION\"
    else: outcome=\"FAIL-BOTH\"
    if outcome"'!="PASS":
        print(cell,n,o,b,f"{n/o:.3f}",f"{n/b:.3f}",outcome,"spreads",spread(by[(cell,"new")]),spread(by[(cell,"old")]))
print("block1_pass",npass,"of",12)
print("'"\\nCROSS\")
for carrier in (\"tcp\",\"grpc\"):
  for fixture in (\"large\",\"small\",\"mixed\"):
    pref=base[f\"push_{carrier}_{fixture}\"]; lref=base[f\"pull_{carrier}_{fixture}\"]; ref=min(pref,lref)
    vals={}
    for d in (\"mw\",\"wm\"):
      cell=f\"{d}_{carrier}_{fixture}\"
      worst=max(M[(cell,\"mac_init\")],M[(cell,\"win_init\")])
      vals[d]=worst
      print(cell,\"worst\",worst,\"base_push\",pref,\"base_pull\",lref,\"minref\",ref,\"ratio\",f\"{worst/ref:.3f}\",\"PASS\" if 10*worst<=11*ref else \"FAIL\")
    po=M[(f\"push_{carrier}_{fixture}\",\"old\")]
    lo=M[(f\"pull_{carrier}_{fixture}\",\"old\")]
    print(\"gap\",carrier,fixture,\"old\",po,lo,f\"{po/lo:.3f}\",\"new\",vals[\"mw\"],vals[\"wm\"],f\"{vals['\"'\"'mw'\"'\"']/vals['\"'\"'wm'\"'\"']:.3f}\")'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-X8CZkYMh' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-zjUnd0Lb' (errno=Operation not permitted)
BYTE_INTEGRITY
raw_bytes 11833 san_bytes 11641 cr_count 192 san_cr_count 0
raw_minus_cr_equals_sanitized True
raw_lf 193 san_lf 193
sha_raw 10d885f9a4bf2a7d3b09ba6386048e63c0af9c6ef2d1b628a6db82eb723034d2
sha_san 7216697be7768f6573b0c2d9f152bb57814f3bf12b5bb2a2719d64b92bec26d0

ROW_INTEGRITY
rows 192 valid_counts Counter({'yes': 192}) exit_counts Counter({'0': 192})
drain_counts Counter({'drained_8s': 120, 'drained_6s': 71, 'drained_62s': 1})
raw_universal_records 384 raw_valid_counts Counter({None: 384})

FAIL_ARM_VALUES
push_tcp_small
  new [2031, 2070, 2090, 2109] median 2080 spread 3.8
  old [1796, 1798, 1824, 1849] median 1811 spread 3.0
pull_tcp_mixed
  new [1127, 1134, 1142, 1186] median 1138 spread 5.2
  old [855, 857, 877, 912] median 867 spread 6.7
wm_tcp_mixed
  mac_init [1122, 1125, 1129, 1214] median 1127 spread 8.2
  win_init [897, 907, 916, 927] median 911 spread 3.3

INVARIANCE
mw_grpc_large 1911 1931 1.010 PASS spreads 0.9 1.1
mw_grpc_mixed 1829 1842 1.007 PASS spreads 4.4 1.6
mw_grpc_small 2261 2227 1.015 PASS spreads 13.5 11.4
mw_tcp_large 1914 1920 1.003 PASS spreads 5.2 0.8
mw_tcp_mixed 1587 1502 1.057 PASS spreads 17.4 24.2
mw_tcp_small 1922 1935 1.007 PASS spreads 2.2 4.5
wm_grpc_large 964 993 1.030 PASS spreads 0.6 1.4
wm_grpc_mixed 1246 1262 1.013 PASS spreads 5.3 4.4
wm_grpc_small 1375 1326 1.037 PASS spreads 7.7 2.1
wm_tcp_large 962 984 1.023 PASS spreads 0.8 258.7
wm_tcp_mixed 1127 911 1.237 FAIL spreads 8.2 3.3
wm_tcp_small 1253 1267 1.011 PASS spreads 12.5 7.9
invariance_pass 11 of 12

BLOCK1_COMBINED_NONPASS
pull_tcp_mixed 1138 867 1284 1.313 0.886 FAIL-SAME-SESSION spreads 5.2 6.7
push_tcp_small 2080 1811 1868 1.149 1.113 FAIL-BOTH spreads 3.8 3.0
block1_pass 10 of 12

CROSS
mw_tcp_large worst 1920 base_push 3054 base_pull 1294 minref 1294 ratio 1.484 FAIL
wm_tcp_large worst 984 base_push 3054 base_pull 1294 minref 1294 ratio 0.760 PASS
gap tcp large old 1908 964 1.979 new 1920 984 1.951
mw_tcp_small worst 1935 base_push 1868 base_pull 1280 minref 1280 ratio 1.512 FAIL
wm_tcp_small worst 1267 base_push 1868 base_pull 1280 minref 1280 ratio 0.990 PASS
gap tcp small old 1811 1360 1.332 new 1935 1267 1.527
mw_tcp_mixed worst 1587 base_push 2288 base_pull 1284 minref 1284 ratio 1.236 FAIL
wm_tcp_mixed worst 1127 base_push 2288 base_pull 1284 minref 1284 ratio 0.878 PASS
gap tcp mixed old 1687 867 1.946 new 1587 1127 1.408
mw_grpc_large worst 1931 base_push 3065 base_pull 1289 minref 1289 ratio 1.498 FAIL
wm_grpc_large worst 993 base_push 3065 base_pull 1289 minref 1289 ratio 0.770 PASS
gap grpc large old 1913 978 1.956 new 1931 993 1.945
mw_grpc_small worst 2261 base_push 2822 base_pull 1462 minref 1462 ratio 1.547 FAIL
wm_grpc_small worst 1375 base_push 2822 base_pull 1462 minref 1462 ratio 0.940 PASS
gap grpc small old 2942 1525 1.929 new 2261 1375 1.644
mw_grpc_mixed worst 1842 base_push 2687 base_pull 1408 minref 1408 ratio 1.308 FAIL
wm_grpc_mixed worst 1262 base_push 2687 base_pull 1408 minref 1408 ratio 0.896 PASS
gap grpc mixed old 2177 1408 1.546 new 1842 1262 1.460

exec
/bin/zsh -lc "git log -1 --format='%H%n%B' 44c2046" in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'git show --format=fuller --no-ext-diff --stat --summary 856af64' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'git show --format=fuller --no-ext-diff --stat --summary e21cf84' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-lGGmMYsV' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-YOUCXuGS' (errno=Operation not permitted)
commit 856af646df7c726c1d17cb239af1e7713f4553d2
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sun Jul 12 14:47:20 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sun Jul 12 14:47:20 2026 -0400

    otp-12b: strip CRs from the drain outcome before it lands in runs.csv
    
    pwsh emits CRLF; a bare \r mid-row splits every row before its valid
    field under python universal newlines - the whole first session
    verdicted INCOMPLETE off 196 valid runs. Post-processing of the
    recorded session sanitizes its runs.csv the same way (recorded in the
    evidence README).
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

 scripts/bench_otp12_win.sh | 6 ++++++
 1 file changed, 6 insertions(+)

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-I8sdCekk' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-2dxnWgHL' (errno=Operation not permitted)
commit e21cf847f540aab23c23e6c32def044df8147ed2
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sun Jul 12 13:48:50 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sun Jul 12 13:48:50 2026 -0400

    otp-12b: fix the sentinel rc reference (pwsh scope-qualified variable parse)
    
    PowerShell parses bare $rc:R as a scope-qualified variable (like
    $env:PATH), so the win-initiated sentinel never printed and the
    win->mac smoke correctly failed fast with the parse-failure rc=99.
    Found live at first rig contact; ${rc} braces fix it. The manual
    reproduction proved the win->mac path itself works (pull complete,
    exit 0 - no macOS firewall issue).
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

 scripts/bench_otp12_win.sh | 6 +++++-
 1 file changed, 5 insertions(+), 1 deletion(-)

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-mw0n0otT' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Uzx9LlKb' (errno=Operation not permitted)
44c2046853d398a4e474ccc7150e884239e3308f
otp-12b: record the Mac<->Windows acceptance evidence (invariance 11/12; converge 10/12; cross attributed)

192 timed runs, zero voided. Invariance PASSes at 1.003-1.057 except
wm_tcp_mixed 1.237 (TCP x mixed x destination-initiator - real,
corroborated by block-1 pull_tcp_mixed 1.313; code-shaped finding).
Converge fails: push_tcp_small 1.149 (matches zoey 1.105 - second
rig), pull_tcp_mixed (same root). Cross: Win->Mac all beat the better
old direction; Mac->Win residue platform-attributed by unchanged/
narrowed gap rows (D-2026-07-12-1) except tcp_small (gap widened =
the code component). CR post-processing recorded. Declares nothing.

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>


exec
/bin/zsh -lc "git log -1 --format='%H%n%B' 856af64" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-RshqC4vC' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-6LiPzblK' (errno=Operation not permitted)
856af646df7c726c1d17cb239af1e7713f4553d2
otp-12b: strip CRs from the drain outcome before it lands in runs.csv

pwsh emits CRLF; a bare \r mid-row splits every row before its valid
field under python universal newlines - the whole first session
verdicted INCOMPLETE off 196 valid runs. Post-processing of the
recorded session sanitizes its runs.csv the same way (recorded in the
evidence README).

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>


exec
/bin/zsh -lc 'git show --format=fuller --no-ext-diff --stat --summary 44c2046' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-vVtZJCUF' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-uBcFISSA' (errno=Operation not permitted)
commit 44c2046853d398a4e474ccc7150e884239e3308f
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sun Jul 12 14:49:57 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sun Jul 12 14:49:57 2026 -0400

    otp-12b: record the Mac<->Windows acceptance evidence (invariance 11/12; converge 10/12; cross attributed)
    
    192 timed runs, zero voided. Invariance PASSes at 1.003-1.057 except
    wm_tcp_mixed 1.237 (TCP x mixed x destination-initiator - real,
    corroborated by block-1 pull_tcp_mixed 1.313; code-shaped finding).
    Converge fails: push_tcp_small 1.149 (matches zoey 1.105 - second
    rig), pull_tcp_mixed (same root). Cross: Win->Mac all beat the better
    old direction; Mac->Win residue platform-attributed by unchanged/
    narrowed gap rows (D-2026-07-12-1) except tcp_small (gap widened =
    the code component). CR post-processing recorded. Declares nothing.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

 docs/bench/otp12-win-2026-07-12/README.md          | 109 ++++++++++++
 docs/bench/otp12-win-2026-07-12/drain-outcomes.txt | 192 ++++++++++++++++++++
 docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv  | 193 +++++++++++++++++++++
 docs/bench/otp12-win-2026-07-12/runs.csv           | 193 +++++++++++++++++++++
 .../otp12-win-2026-07-12/staging-manifest.txt      |   8 +
 docs/bench/otp12-win-2026-07-12/summary.csv        |  49 ++++++
 docs/bench/otp12-win-2026-07-12/verdicts.csv       | 121 +++++++++++++
 7 files changed, 865 insertions(+)
 create mode 100644 docs/bench/otp12-win-2026-07-12/README.md
 create mode 100644 docs/bench/otp12-win-2026-07-12/drain-outcomes.txt
 create mode 100644 docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv
 create mode 100644 docs/bench/otp12-win-2026-07-12/runs.csv
 create mode 100644 docs/bench/otp12-win-2026-07-12/staging-manifest.txt
 create mode 100644 docs/bench/otp12-win-2026-07-12/summary.csv
 create mode 100644 docs/bench/otp12-win-2026-07-12/verdicts.csv

exec
/bin/zsh -lc 'command -v pwsh || true' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
/opt/homebrew/bin/pwsh

exec
/bin/zsh -lc "nl -ba docs/bench/otp12-win-2026-07-12/runs.csv | sed -n '145,158p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   145	wm_tcp_small,win_init,e21cf84,win,4,1274,170,0,drained_8s,yes
   146	mw_grpc_small,mac_init,e21cf84,mac,1,2377,797,0,drained_8s,yes
   147	mw_grpc_small,win_init,e21cf84,win,1,2334,795,0,drained_8s,yes
   148	mw_grpc_small,mac_init,e21cf84,mac,2,2325,794,0,drained_8s,yes
   149	mw_grpc_small,win_init,e21cf84,win,2,2190,784,0,drained_8s,yes
   150	mw_grpc_small,mac_init,e21cf84,mac,3,2197,805,0,drained_8s,yes
   151	mw_grpc_small,win_init,e21cf84,win,3,2265,788,0,drained_6s,yes
   152	mw_grpc_small,mac_init,e21cf84,mac,4,2094,628,0,drained_6s,yes
   153	mw_grpc_small,win_init,e21cf84,win,4,2096,654,0,drained_62s,yes
   154	wm_grpc_small,mac_init,e21cf84,mac,1,1435,172,0,drained_8s,yes
   155	wm_grpc_small,win_init,e21cf84,win,1,1326,170,0,drained_6s,yes
   156	wm_grpc_small,mac_init,e21cf84,mac,2,1347,172,0,drained_8s,yes
   157	wm_grpc_small,win_init,e21cf84,win,2,1329,203,0,drained_6s,yes
   158	wm_grpc_small,mac_init,e21cf84,mac,3,1333,178,0,drained_8s,yes

exec
/bin/zsh -lc 'nl -ba docs/bench/otp12-win-2026-07-12/staging-manifest.txt' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
     1	arm,role,sha,sha256,path
     2	old,client,0f922de,ba919d7292dfae8f2a67aa29a9514d345496cb55f2112c0fcd6e79ab590e8dd6,/Users/michael/blit-bench-work/bins/blit-0f922de
     3	new,client,e21cf84,d37391652c3693244cbcefa122da7e7e26c770d553b72dead754eaef812234f8,/Users/michael/Dev/blit_v2/target/release/blit
     4	new,mac-daemon,e21cf84,a6dd94d7547968ca2667dfd86edc6f19bae4f5a05f24dc75b60285c4d3ab75e0,/Users/michael/Dev/blit_v2/target/release/blit-daemon
     5	old,win-daemon,0f922de,97d22441575481a4d44621720fd1db22267528a55e3a28d62c2f271dad8ff841,D:\blit-test\bins\0f922de\blit-daemon.exe
     6	new,win-daemon,e21cf84,511e0f81efa35a7e24cb5755a0c2be1c689eee1df98199e776dd907406645067,D:\blit-test\bins\e21cf84\blit-daemon.exe
     7	new,win-client,e21cf84,32660cb2c17a606922316d00379904c96162a8554407d275ebe1c4a7f38e7ffb,D:\blit-test\bins\e21cf84\blit.exe
     8	-,reference,-,9ccb5c86d8654fc9de93dd25455f03bfb4dfaf856bab64f82fe371b245aa1337,/Users/michael/Dev/blit_v2/docs/bench/otp2w-baseline-2026-07-10/summary.csv

exec
/bin/zsh -lc 'git show 44c2046:docs/bench/otp12-win-2026-07-12/runs.csv | sha256sum && sha256sum docs/bench/otp12-win-2026-07-12/runs.csv' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'git show 44c2046:docs/bench/otp12-win-2026-07-12/README.md | sha256sum && sha256sum docs/bench/otp12-win-2026-07-12/README.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-FMGnu1rj' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-EepNPPEl' (errno=Operation not permitted)
1f582aa44b0f9fe4b7f561e717ba6d4f7c2b5d63dbe5b301ca1ef43335c134b7  -
1f582aa44b0f9fe4b7f561e717ba6d4f7c2b5d63dbe5b301ca1ef43335c134b7  docs/bench/otp12-win-2026-07-12/README.md

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-7sAw4cYu' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-mROk68WW' (errno=Operation not permitted)
7216697be7768f6573b0c2d9f152bb57814f3bf12b5bb2a2719d64b92bec26d0  -
7216697be7768f6573b0c2d9f152bb57814f3bf12b5bb2a2719d64b92bec26d0  docs/bench/otp12-win-2026-07-12/runs.csv

exec
/bin/zsh -lc 'git show 44c2046:docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv | sha256sum && sha256sum docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-o4YeWhOv' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-JhhkAPVI' (errno=Operation not permitted)
10d885f9a4bf2a7d3b09ba6386048e63c0af9c6ef2d1b628a6db82eb723034d2  -
10d885f9a4bf2a7d3b09ba6386048e63c0af9c6ef2d1b628a6db82eb723034d2  docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv

exec
/bin/zsh -lc "find .review/findings -maxdepth 1 -type f -name '*otp-12b*' -print" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
.review/findings/otp-12b-win-harness.md

exec
/bin/zsh -lc "rg -n -C 5 'cross-gap|never adjudicated|platform-residue|gap unchanged|narrowed' .review/findings/otp-12b* .review/results/otp-12b* 2>/dev/null" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
.review/results/otp-12b.gpt-verdict.md-73-state are verified; a mismatch refuses with guidance (the owner's
.review/results/otp-12b.gpt-verdict.md-74-firewall is never silently mutated).
.review/results/otp-12b.gpt-verdict.md-75-
.review/results/otp-12b.gpt-verdict.md-76-## F12 (Low) — vocabulary not closed; gap-row labels inexact
.review/results/otp-12b.gpt-verdict.md-77-Confirmed. Fixed: doc vocabulary closed
.review/results/otp-12b.gpt-verdict.md:78:(`cross-gap`/`RECORDED`/`NO-SAME-SESSION-REF` registered); gap rows
.review/results/otp-12b.gpt-verdict.md-79-label their operands exactly (`old_push,old_pull` /
.review/results/otp-12b.gpt-verdict.md-80-`new_mw_worst,new_wm_worst`).
.review/results/otp-12b.gpt-verdict.md-81-
.review/results/otp-12b.gpt-verdict.md-82-## Fix commit
.review/results/otp-12b.gpt-verdict.md-83-
--
.review/results/otp-12b.codex.md-9-reasoning effort: ultra
.review/results/otp-12b.codex.md-10-reasoning summaries: none
.review/results/otp-12b.codex.md-11-session id: 019f573d-64cc-7553-9dd3-abaed2801c63
.review/results/otp-12b.codex.md-12---------
.review/results/otp-12b.codex.md-13-user
.review/results/otp-12b.codex.md:14:Review commits d30b1e3 and 772cfe6 together (run: git show d30b1e3; git show 772cfe6). They implement otp-12b's harness half per docs/plan/OTP12_ACCEPTANCE_RUN.md (Active) D1-D3/D5/D6: scripts/bench_otp12_win.sh, derived from the frozen scripts/bench_otp2w_baseline.sh and from the already-reviewed scripts/bench_otp12_zoey.sh (whose review rounds accepted 9+6 findings - assume those lessons are intended to carry: +sha provenance form, ABBA pair-void valid-run rule, per-run destination sweep, fail-closed manifest, CELLS validation, session-gated identity-verified kill traps). Block 1 = the otp-2w matrix as interleaved old(0f922de)/new matched pairs, Mac-initiated, verdicts vs BOTH references (same-session old AND docs/bench/otp2w-baseline-2026-07-10/summary.csv). Block 2 = the plan's initiator/verb-invariance cells: mw_*/wm_* (data Mac->Win / Win->Mac), arm mac_init vs win_init interleaved, new pair only, plus per-arm converge rows (design F3 - no tolerance compounding), F4 cross rows vs min committed old direction, and D-2026-07-12-1 discriminator gap rows (outcome RECORDED, never adjudicated). CHECK HARD: (1) bash correctness under macOS bash 3.2 and quote-parity through every wssh payload (772cfe6 fixed one apostrophe bug already - hunt for more, especially the win_daemon_start here-string, win_client_run, setup_host, sha256_win, win_embeds); (2) fairness of the invariance pairs - do mac_init and win_init arms do IDENTICAL work (same physical sources, same one-level-nested destination shape, flush keyed by destination OS, both windows exclude ssh overhead - the win side via Stopwatch-in-one-ssh printing ms,exit); is anything asymmetric that could masquerade as an initiator effect; (3) the Windows arm-swap via the fixed active exe path (Copy-Item over a possibly-running daemon? the stale-refusal ordering vs the copy; firewall rule scoped to the active path); (4) the python verdict pass: block-1 cells (push_/pull_ prefixes) vs block-2 (mw_/wm_), complete() gating, per-arm converge rows falling back when block 1 was CELLS-filtered, the cross-gap rows' arithmetic (old_gap = same-session old push/pull medians; new_gap = worst mw arm / worst wm arm) and whether any row self-adjudicates what D-2026-07-12-1 reserves for the owner; (5) any way a run records an unfair or bogus time (drop_caches placement inside win_pull_run/win_push_run vs the Stopwatch window, T_MS parse failure path T_RC=99, flush attribution, per-run sweeps outside timed windows); (6) the doc grammar amendment and the findings doc claims. No crates/proto changes - suite unaffected. Output a concise markdown findings list - each finding with file:line, severity, rationale - then a final VERDICT line. Be concise; do not invoke skills.
.review/results/otp-12b.codex.md-15-codex
.review/results/otp-12b.codex.md-16-I’ll review the two commits as one change set, trace each acceptance criterion back to the active plan and frozen/reviewed harnesses, and split the audit across shell/quoting, experiment fairness, and verdict/doc logic. I’ll report only actionable findings, then a single verdict.
.review/results/otp-12b.codex.md-17-mcp: ptk/ptk_invoke started
.review/results/otp-12b.codex.md-18-mcp: ptk/ptk_invoke (completed)
.review/results/otp-12b.codex.md-19-mcp: ptk/ptk_invoke started
--
.review/results/otp-12b.codex.md-71-
.review/results/otp-12b.codex.md-72-- [scripts/bench_otp12_win.sh:719](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:719) — **LOW** — `CELLS` validation is regex/header-sensitive: `CELLS=cell` selects nothing but matches the CSV header and exits successfully with empty evidence.
.review/results/otp-12b.codex.md-73-
.review/results/otp-12b.codex.md-74-- [scripts/bench_otp12_win.sh:222](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:222) — **LOW** — An existing firewall rule is trusted by display name without verifying its program path, action, or enabled state.
.review/results/otp-12b.codex.md-75-
.review/results/otp-12b.codex.md:76:- [docs/plan/OTP12_ACCEPTANCE_RUN.md:298](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:298) — **LOW** — The grammar introduces `cross-gap`/`RECORDED`, then excludes both from its closed vocabularies. Additionally, [the first gap row](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:662) labels gap-vs-gap while storing push-vs-pull operands.
.review/results/otp-12b.codex.md-77-
.review/results/otp-12b.codex.md-78-**VERDICT: FAIL** — macOS Bash 3.2 syntax and default-path quote parity pass, but fairness and evidence-integrity blockers remain.
.review/results/otp-12b.codex.md-79-tokens used
.review/results/otp-12b.codex.md-80-109,042
.review/results/otp-12b.codex.md-81-- [scripts/bench_otp12_win.sh:203](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:203) — **HIGH** — Hash failures occur inside `echo "$(…)"`; `die` exits only the subshell, so the supposedly fail-closed manifest continues malformed.
--
.review/results/otp-12b.codex.md-98-
.review/results/otp-12b.codex.md-99-- [scripts/bench_otp12_win.sh:719](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:719) — **LOW** — `CELLS` validation is regex/header-sensitive: `CELLS=cell` selects nothing but matches the CSV header and exits successfully with empty evidence.
.review/results/otp-12b.codex.md-100-
.review/results/otp-12b.codex.md-101-- [scripts/bench_otp12_win.sh:222](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:222) — **LOW** — An existing firewall rule is trusted by display name without verifying its program path, action, or enabled state.
.review/results/otp-12b.codex.md-102-
.review/results/otp-12b.codex.md:103:- [docs/plan/OTP12_ACCEPTANCE_RUN.md:298](/Users/michael/Dev/blit_v2/docs/plan/OTP12_ACCEPTANCE_RUN.md:298) — **LOW** — The grammar introduces `cross-gap`/`RECORDED`, then excludes both from its closed vocabularies. Additionally, [the first gap row](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:662) labels gap-vs-gap while storing push-vs-pull operands.
.review/results/otp-12b.codex.md-104-
.review/results/otp-12b.codex.md-105-**VERDICT: FAIL** — macOS Bash 3.2 syntax and default-path quote parity pass, but fairness and evidence-integrity blockers remain.
--
.review/findings/otp-12b-win-harness.md-15-`otp2w-baseline-2026-07-10` medians). Block 2 — the plan's headline
.review/findings/otp-12b-win-harness.md-16-criterion measured for the first time: per data direction × fixture ×
.review/findings/otp-12b-win-harness.md-17-carrier, Mac-initiated vs Windows-initiated arms interleaved ABBA
.review/findings/otp-12b-win-harness.md-18-(`mw_*`/`wm_*` cells), plus per-arm converge rows (design F3), the F4
.review/findings/otp-12b-win-harness.md-19-cross-direction rows vs `min(old_push, old_pull)` committed, and
.review/findings/otp-12b-win-harness.md:20:D-2026-07-12-1 discriminator gap rows (kind `cross-gap`, outcome
.review/findings/otp-12b-win-harness.md-21-`RECORDED` — the harness never adjudicates the residue).
.review/findings/otp-12b-win-harness.md-22-
.review/findings/otp-12b-win-harness.md-23-## Approach
.review/findings/otp-12b-win-harness.md-24-
.review/findings/otp-12b-win-harness.md-25-Windows plumbing verbatim from the frozen `bench_otp2w_baseline.sh`
--
.review/findings/otp-12b-win-harness.md-48-## Files
.review/findings/otp-12b-win-harness.md-49-
.review/findings/otp-12b-win-harness.md-50-- `scripts/bench_otp12_win.sh` (new, executable; self-contained per D5 —
.review/findings/otp-12b-win-harness.md-51-  the frozen otp-2w script untouched).
.review/findings/otp-12b-win-harness.md-52-- `docs/plan/OTP12_ACCEPTANCE_RUN.md` — D5 cell grammar extended for
.review/findings/otp-12b-win-harness.md:53:  the invariance (`mw|wm`) and `gap_*`/`cross-gap` rows.
.review/findings/otp-12b-win-harness.md-54-
.review/findings/otp-12b-win-harness.md-55-## Tests
.review/findings/otp-12b-win-harness.md-56-
.review/findings/otp-12b-win-harness.md-57-- `bash -n` clean; shellcheck not installed on this machine (recorded).
.review/findings/otp-12b-win-harness.md-58-- No crates/proto/Cargo changes; the suite stands at the recorded 1484.
--
.review/results/otp-12b-run.codex.md-372-    77	  better committed old direction (ratios 0.71–0.99).
.review/results/otp-12b-run.codex.md-373-    78	- **Mac→Win: all six cells FAIL** `min(old_push, old_pull) × 1.10` —
.review/results/otp-12b-run.codex.md-374-    79	  and the gap rows attribute it: the same-session old direction gap
.review/results/otp-12b-run.codex.md-375-    80	  (`old_push/old_pull`) vs the unified gap (`new_mw/new_wm`) is
.review/results/otp-12b-run.codex.md-376-    81	  **unchanged on large (1.979 → 1.951 tcp; 1.956 → 1.945 grpc)** and
.review/results/otp-12b-run.codex.md:377:    82	  **narrowed on mixed (1.946 → 1.408) and grpc_small (1.929 → 1.644)**
.review/results/otp-12b-run.codex.md-378-    83	  — the residue is the Windows destination write path, present
.review/results/otp-12b-run.codex.md-379-    84	  identically without blit's old choreography (D-2026-07-12-1: such
.review/results/otp-12b-run.codex.md-380-    85	  cells count as satisfying criterion 2's cross-direction half). The
.review/results/otp-12b-run.codex.md-381-    86	  one exception: **tcp_small's gap widened (1.332 → 1.527)** — the
.review/results/otp-12b-run.codex.md-382-    87	  widening tracks the push_tcp_small code gap above, i.e. that cell's
--
.review/results/otp-12b-run.codex.md-575-docs/plan/OTP12_ACCEPTANCE_RUN.md-162-  `max(delegated, direct)/min ≤ 1.10`.
.review/results/otp-12b-run.codex.md-576-docs/plan/OTP12_ACCEPTANCE_RUN.md-163-- **Cross-direction (rig W, the F4 computation)**: per fixture × carrier,
.review/results/otp-12b-run.codex.md-577-docs/plan/OTP12_ACCEPTANCE_RUN.md-164-  each unified direction's median vs
.review/results/otp-12b-run.codex.md-578-docs/plan/OTP12_ACCEPTANCE_RUN.md-165-  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
.review/results/otp-12b-run.codex.md-579-docs/plan/OTP12_ACCEPTANCE_RUN.md-166-  while passing per-direction converge-up AND invariance, the evidence
.review/results/otp-12b-run.codex.md:580:docs/plan/OTP12_ACCEPTANCE_RUN.md:167:  additionally computes the **platform-residue discriminator** the otp-2w
.review/results/otp-12b-run.codex.md-581-docs/plan/OTP12_ACCEPTANCE_RUN.md-168-  README pre-registered: compare the old arm's direction gap
.review/results/otp-12b-run.codex.md-582-docs/plan/OTP12_ACCEPTANCE_RUN.md-169-  (`old_push/old_pull`) with the new arm's (`new_MW/new_WM`), same
.review/results/otp-12b-run.codex.md-583-docs/plan/OTP12_ACCEPTANCE_RUN.md:170:  session. Gap unchanged ⇒ the residue exists identically without blit's
.review/results/otp-12b-run.codex.md-584-docs/plan/OTP12_ACCEPTANCE_RUN.md:171:  old choreography and lands on the platform write path (NTFS/Defender vs
.review/results/otp-12b-run.codex.md-585-docs/plan/OTP12_ACCEPTANCE_RUN.md-172-  APFS — the plan's Non-goals: different hardware need not perform
.review/results/otp-12b-run.codex.md-586-docs/plan/OTP12_ACCEPTANCE_RUN.md-173-  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12b-run.codex.md-587-docs/plan/OTP12_ACCEPTANCE_RUN.md-174-  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12b-run.codex.md:588:docs/plan/OTP12_ACCEPTANCE_RUN.md:175:  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12b-run.codex.md-589-docs/plan/OTP12_ACCEPTANCE_RUN.md-176-  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12b-run.codex.md-590-docs/plan/OTP12_ACCEPTANCE_RUN.md-177-
.review/results/otp-12b-run.codex.md-591-docs/plan/OTP12_ACCEPTANCE_RUN.md:178:Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12b-run.codex.md-592-docs/plan/OTP12_ACCEPTANCE_RUN.md:179:bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12b-run.codex.md-593-docs/plan/OTP12_ACCEPTANCE_RUN.md-180-interleaved in a fresh session; both sessions are committed.
--
.review/results/otp-12b-run.codex.md-661-docs/plan/OTP12_ACCEPTANCE_RUN.md-290-where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
.review/results/otp-12b-run.codex.md-662-docs/plan/OTP12_ACCEPTANCE_RUN.md-291-otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
.review/results/otp-12b-run.codex.md-663-docs/plan/OTP12_ACCEPTANCE_RUN.md-292-reference CSVs; corrected at the 12a review, codex F9),
.review/results/otp-12b-run.codex.md-664-docs/plan/OTP12_ACCEPTANCE_RUN.md-293-`<mw|wm>_<carrier>_<fixture>` for rig-W invariance cells (data
.review/results/otp-12b-run.codex.md-665-docs/plan/OTP12_ACCEPTANCE_RUN.md-294-direction Mac→Win / Win→Mac), and `gap_<carrier>_<fixture>` for the
.review/results/otp-12b-run.codex.md:666:docs/plan/OTP12_ACCEPTANCE_RUN.md:295:discriminator gap rows (kind `cross-gap`, outcome `RECORDED` — never
.review/results/otp-12b-run.codex.md-667-docs/plan/OTP12_ACCEPTANCE_RUN.md-296-self-adjudicated; added at the 12b harness slice), `arm` ∈
.review/results/otp-12b-run.codex.md-668-docs/plan/OTP12_ACCEPTANCE_RUN.md-297-`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
.review/results/otp-12b-run.codex.md-669-docs/plan/OTP12_ACCEPTANCE_RUN.md-298-`initiator` = host name, `kind` ∈
.review/results/otp-12b-run.codex.md:670:docs/plan/OTP12_ACCEPTANCE_RUN.md-299-`converge|invariance|delegated|cross|cross-gap`.
.review/results/otp-12b-run.codex.md-671-docs/plan/OTP12_ACCEPTANCE_RUN.md-300-Verdict outcome vocabulary (closed; 12b review, codex F12): per-reference
.review/results/otp-12b-run.codex.md-672-docs/plan/OTP12_ACCEPTANCE_RUN.md-301-rows carry `PASS|FAIL`; a comparison's `combined`/`invariance` row
.review/results/otp-12b-run.codex.md-673-docs/plan/OTP12_ACCEPTANCE_RUN.md:302:carries the registered D2 set
.review/results/otp-12b-run.codex.md-674-docs/plan/OTP12_ACCEPTANCE_RUN.md-303-(`PASS|FAIL-SAME-SESSION|FAIL-REFERENCE-DRIFT|FAIL-BOTH|INCOMPLETE`);
.review/results/otp-12b-run.codex.md:675:docs/plan/OTP12_ACCEPTANCE_RUN.md:304:`cross-gap` rows carry `RECORDED` only (never adjudicated); a block-2
.review/results/otp-12b-run.codex.md-676-docs/plan/OTP12_ACCEPTANCE_RUN.md-305-converge row whose same-session block-1 counterpart is absent or
.review/results/otp-12b-run.codex.md-677-docs/plan/OTP12_ACCEPTANCE_RUN.md:306:incomplete carries `NO-SAME-SESSION-REF` (an escalation-session
.review/results/otp-12b-run.codex.md-678-docs/plan/OTP12_ACCEPTANCE_RUN.md-307-artifact — the committed-reference row still governs). Nothing else is
.review/results/otp-12b-run.codex.md-679-docs/plan/OTP12_ACCEPTANCE_RUN.md-308-legal, and a missing committed-reference row aborts the verdict pass
.review/results/otp-12b-run.codex.md-680-docs/plan/OTP12_ACCEPTANCE_RUN.md-309-(fail closed).
--
.review/results/otp-12b-run.codex.md-691-docs/plan/OTP12_ACCEPTANCE_RUN.md-395-
.review/results/otp-12b-run.codex.md-692-docs/plan/OTP12_ACCEPTANCE_RUN.md-396-- **No rig is truly fs-identical.** The plan's "symmetric rig" is
.review/results/otp-12b-run.codex.md-693-docs/plan/OTP12_ACCEPTANCE_RUN.md-397-  instantiated by the owner-designated closest-spec pair; rig W's two
.review/results/otp-12b-run.codex.md-694-docs/plan/OTP12_ACCEPTANCE_RUN.md-398-  directions still land on different OS write paths (APFS vs NTFS +
.review/results/otp-12b-run.codex.md-695-docs/plan/OTP12_ACCEPTANCE_RUN.md:399:  Defender at its normal state). D2's discriminator computation is the
.review/results/otp-12b-run.codex.md:696:docs/plan/OTP12_ACCEPTANCE_RUN.md:400:  pre-registered, evidence-backed handling; a platform-residue cell counts
.review/results/otp-12b-run.codex.md-697-docs/plan/OTP12_ACCEPTANCE_RUN.md:401:  as satisfied per D-2026-07-12-1.
.review/results/otp-12b-run.codex.md-698-docs/plan/OTP12_ACCEPTANCE_RUN.md-402-- **Old-arm provenance is a staging record, not a handshake** (old paths
.review/results/otp-12b-run.codex.md-699-docs/plan/OTP12_ACCEPTANCE_RUN.md-403-  predate it). Mitigated by machines.md provenance + the sha256 manifest;
.review/results/otp-12b-run.codex.md-700-docs/plan/OTP12_ACCEPTANCE_RUN.md-404-  accepted residual risk.
.review/results/otp-12b-run.codex.md-701-docs/plan/OTP12_ACCEPTANCE_RUN.md-405-- **First-of-kind surfaces**: a daemon on the Mac (application firewall
--
.review/results/otp-12b-run.codex.md-716-docs/plan/OTP12_ACCEPTANCE_RUN.md-420-  beats its own old direction, is initiator-invariant, and misses the
.review/results/otp-12b-run.codex.md-717-docs/plan/OTP12_ACCEPTANCE_RUN.md-421-  `min(old_push, old_pull) × 1.10` bar only by a discriminator-attributed
.review/results/otp-12b-run.codex.md-718-docs/plan/OTP12_ACCEPTANCE_RUN.md:422:  platform write-path residue (same gap in the old arm, same session)
.review/results/otp-12b-run.codex.md-719-docs/plan/OTP12_ACCEPTANCE_RUN.md-423-  **counts as satisfying the cross-direction half of criterion 2**
.review/results/otp-12b-run.codex.md-720-docs/plan/OTP12_ACCEPTANCE_RUN.md:424:  (D-2026-07-12-1). The evidence still records both computations per
.review/results/otp-12b-run.codex.md:721:docs/plan/OTP12_ACCEPTANCE_RUN.md:425:  cell; the otp-13 walk reviews the numbers, but a platform-residue cell
.review/results/otp-12b-run.codex.md-722-docs/plan/OTP12_ACCEPTANCE_RUN.md-426-  is not a blocker.
.review/results/otp-12b-run.codex.md-723---
.review/results/otp-12b-run.codex.md-724-docs/DECISIONS.md-19----
.review/results/otp-12b-run.codex.md-725-docs/DECISIONS.md-20-
.review/results/otp-12b-run.codex.md-726-docs/DECISIONS.md-21-## D-2026-05-31-1 — v0.1.0 shipped; release plan frozen
--
.review/results/otp-12b-run.codex.md-800-docs/DECISIONS.md-164-## D-2026-07-11-1 — `--relay-via-cli` removed; remote→remote is delegated-only
.review/results/otp-12b-run.codex.md-801-docs/DECISIONS.md-165-- Decision: The `--relay-via-cli` escape hatch is **removed** (owner, 2026-07-11, otp-10c-1). Remote→remote transfers are delegated-only; the CLI is never in the byte path. The relay's read half was the PullSync client's on-demand per-file remote read — a capability the unified session deliberately does not have — so PullSync's deletion (otp-10c) makes a streaming relay unrebuildable; offered the choice, the owner picked removal over a stage-to-temp-dir reimplementation. The topology the flag served (destination cannot reach source, CLI can reach both) is handled by two manual commands — pull to a local path, then push it — and the delegated CONNECT_SOURCE error hint now says exactly that. `RemoteTransferSource`, its `remote_transfer_source_constructed` counter, and every relay-combination gate (mirror/move/detach/resume × relay) die with the flag; the delegated no-CLI-byte-path pin (`cli_data_plane_outbound_bytes == 0`) remains the byte-path-isolation proof and doubles as the otp-10 deletion proof's CLI half.
.review/results/otp-12b-run.codex.md-802-docs/DECISIONS.md-166-- Why: unshipped product (no compat bar, D-2026-07-05-2 era posture); the ONE_TRANSFER_PATH directive is deletion of bespoke side paths, and a staged relay would merely automate what two commands already do — not worth a maintained transfer-adjacent code path.
.review/results/otp-12b-run.codex.md-803-docs/DECISIONS.md-167-- Supersedes: `REMOTE_REMOTE_DELEGATION_PLAN.md` (already Historical) §§relay-fallback/escape-hatch design — dated header note added, body kept verbatim per that doc's own precedent; the relay-combination gates R50-F1/R51-F2 (move×relay), audit-h1 rounds 1–2 (mirror×relay), codex otp-10a F4 (resume×relay), and the detach×relay gate — deleted with the flag they guarded (their data-loss reasoning is moot once no relay path exists to combine with); `scripts/bench_remote_remote.sh`'s relay leg (removed same commit).
.review/results/otp-12b-run.codex.md-804-docs/DECISIONS.md-168-
.review/results/otp-12b-run.codex.md:805:docs/DECISIONS.md:169:## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
.review/results/otp-12b-run.codex.md:806:docs/DECISIONS.md:170:- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
.review/results/otp-12b-run.codex.md-807-docs/DECISIONS.md:171:- Why: the plan's Non-goals already exclude making different hardware perform identically, and D-2026-07-05-1 restricts cross-direction verdicts to symmetric endpoints; no truly fs-identical pair exists in the fleet, so on the designated closest-spec rig the "better of the two old directions" bar can only bind net of the destination write-path residue the discriminator isolates. Settling the rule before the run prevents re-litigating it with numbers in hand.
.review/results/otp-12b-run.codex.md-808-docs/DECISIONS.md-172-- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).
.review/results/otp-12b-run.codex.md-809-
.review/results/otp-12b-run.codex.md-810-exec
.review/results/otp-12b-run.codex.md-811-/bin/zsh -lc 'git diff --no-ext-diff --find-renames f19776c..44c2046 -- scripts/bench_otp12_win.sh' in /Users/michael/Dev/blit_v2
--
.review/results/otp-12b-run.codex.md-1003-   162	  `max(delegated, direct)/min ≤ 1.10`.
.review/results/otp-12b-run.codex.md-1004-   163	- **Cross-direction (rig W, the F4 computation)**: per fixture × carrier,
.review/results/otp-12b-run.codex.md-1005-   164	  each unified direction's median vs
.review/results/otp-12b-run.codex.md-1006-   165	  `min(old_push, old_pull) × 1.10`. Where a direction exceeds that bar
.review/results/otp-12b-run.codex.md-1007-   166	  while passing per-direction converge-up AND invariance, the evidence
.review/results/otp-12b-run.codex.md:1008:   167	  additionally computes the **platform-residue discriminator** the otp-2w
.review/results/otp-12b-run.codex.md-1009-   168	  README pre-registered: compare the old arm's direction gap
.review/results/otp-12b-run.codex.md-1010-   169	  (`old_push/old_pull`) with the new arm's (`new_MW/new_WM`), same
.review/results/otp-12b-run.codex.md-1011-   170	  session. Gap unchanged ⇒ the residue exists identically without blit's
.review/results/otp-12b-run.codex.md-1012-   171	  old choreography and lands on the platform write path (NTFS/Defender vs
.review/results/otp-12b-run.codex.md-1013-   172	  APFS — the plan's Non-goals: different hardware need not perform
.review/results/otp-12b-run.codex.md-1014-   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12b-run.codex.md-1015-   174	  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12b-run.codex.md:1016:   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12b-run.codex.md-1017-   176	  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12b-run.codex.md-1018-   177	
.review/results/otp-12b-run.codex.md-1019-   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12b-run.codex.md-1020-   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12b-run.codex.md-1021-   180	interleaved in a fresh session; both sessions are committed.
--
.review/results/otp-12b-run.codex.md-1101-   260	initiator (`transfer_session/mod.rs:790,805`), and carrier invariance is
.review/results/otp-12b-run.codex.md-1102-   261	measured properly on rig W.
.review/results/otp-12b-run.codex.md-1103-   262	
.review/results/otp-12b-run.codex.md-1104-   263	Config: BOTH daemons get `[delegation] allow_delegated_pull = true` with
.review/results/otp-12b-run.codex.md-1105-   264	`allowed_source_hosts` naming the peer (each is destination in one
.review/results/otp-12b-run.codex.md:1106:   265	direction); bench modules writable, `delegation_allowed` not narrowed.
.review/results/otp-12b-run.codex.md-1107-   266	
.review/results/otp-12b-run.codex.md-1108-   267	### D5 — three self-contained scripts; the frozen baselines stay frozen
.review/results/otp-12b-run.codex.md-1109-   268	
.review/results/otp-12b-run.codex.md-1110-   269	`scripts/bench_otp12_zoey.sh`, `scripts/bench_otp12_win.sh`,
.review/results/otp-12b-run.codex.md-1111-   270	`scripts/bench_otp12_delegated.sh` — each self-contained (the otp-2w
--
.review/results/otp-12b-run.codex.md-1131-   290	where `cell` = `<verb>_<carrier>_<fixture>` for converge-up blocks (the
.review/results/otp-12b-run.codex.md-1132-   291	otp-2 label grammar, e.g. `push_tcp_large` — matches the committed
.review/results/otp-12b-run.codex.md-1133-   292	reference CSVs; corrected at the 12a review, codex F9),
.review/results/otp-12b-run.codex.md-1134-   293	`<mw|wm>_<carrier>_<fixture>` for rig-W invariance cells (data
.review/results/otp-12b-run.codex.md-1135-   294	direction Mac→Win / Win→Mac), and `gap_<carrier>_<fixture>` for the
.review/results/otp-12b-run.codex.md:1136:   295	discriminator gap rows (kind `cross-gap`, outcome `RECORDED` — never
.review/results/otp-12b-run.codex.md-1137-   296	self-adjudicated; added at the 12b harness slice), `arm` ∈
.review/results/otp-12b-run.codex.md-1138-   297	`old|new|mac_init|win_init|delegated|direct`, `build` = short sha,
.review/results/otp-12b-run.codex.md-1139-   298	`initiator` = host name, `kind` ∈
.review/results/otp-12b-run.codex.md:1140:   299	`converge|invariance|delegated|cross|cross-gap`.
.review/results/otp-12b-run.codex.md-1141-   300	Verdict outcome vocabulary (closed; 12b review, codex F12): per-reference
.review/results/otp-12b-run.codex.md-1142-   301	rows carry `PASS|FAIL`; a comparison's `combined`/`invariance` row
.review/results/otp-12b-run.codex.md-1143-   302	carries the registered D2 set
.review/results/otp-12b-run.codex.md-1144-   303	(`PASS|FAIL-SAME-SESSION|FAIL-REFERENCE-DRIFT|FAIL-BOTH|INCOMPLETE`);
.review/results/otp-12b-run.codex.md:1145:   304	`cross-gap` rows carry `RECORDED` only (never adjudicated); a block-2
.review/results/otp-12b-run.codex.md-1146-   305	converge row whose same-session block-1 counterpart is absent or
.review/results/otp-12b-run.codex.md-1147-   306	incomplete carries `NO-SAME-SESSION-REF` (an escalation-session
.review/results/otp-12b-run.codex.md-1148-   307	artifact — the committed-reference row still governs). Nothing else is
.review/results/otp-12b-run.codex.md-1149-   308	legal, and a missing committed-reference row aborts the verdict pass
.review/results/otp-12b-run.codex.md-1150-   309	(fail closed).
--
.review/results/otp-12b-run.codex.md-1156-   164	## D-2026-07-11-1 — `--relay-via-cli` removed; remote→remote is delegated-only
.review/results/otp-12b-run.codex.md-1157-   165	- Decision: The `--relay-via-cli` escape hatch is **removed** (owner, 2026-07-11, otp-10c-1). Remote→remote transfers are delegated-only; the CLI is never in the byte path. The relay's read half was the PullSync client's on-demand per-file remote read — a capability the unified session deliberately does not have — so PullSync's deletion (otp-10c) makes a streaming relay unrebuildable; offered the choice, the owner picked removal over a stage-to-temp-dir reimplementation. The topology the flag served (destination cannot reach source, CLI can reach both) is handled by two manual commands — pull to a local path, then push it — and the delegated CONNECT_SOURCE error hint now says exactly that. `RemoteTransferSource`, its `remote_transfer_source_constructed` counter, and every relay-combination gate (mirror/move/detach/resume × relay) die with the flag; the delegated no-CLI-byte-path pin (`cli_data_plane_outbound_bytes == 0`) remains the byte-path-isolation proof and doubles as the otp-10 deletion proof's CLI half.
.review/results/otp-12b-run.codex.md-1158-   166	- Why: unshipped product (no compat bar, D-2026-07-05-2 era posture); the ONE_TRANSFER_PATH directive is deletion of bespoke side paths, and a staged relay would merely automate what two commands already do — not worth a maintained transfer-adjacent code path.
.review/results/otp-12b-run.codex.md-1159-   167	- Supersedes: `REMOTE_REMOTE_DELEGATION_PLAN.md` (already Historical) §§relay-fallback/escape-hatch design — dated header note added, body kept verbatim per that doc's own precedent; the relay-combination gates R50-F1/R51-F2 (move×relay), audit-h1 rounds 1–2 (mirror×relay), codex otp-10a F4 (resume×relay), and the detach×relay gate — deleted with the flag they guarded (their data-loss reasoning is moot once no relay path exists to combine with); `scripts/bench_remote_remote.sh`'s relay leg (removed same commit).
.review/results/otp-12b-run.codex.md-1160-   168	
.review/results/otp-12b-run.codex.md:1161:   169	## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
.review/results/otp-12b-run.codex.md:1162:   170	- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
.review/results/otp-12b-run.codex.md-1163-   171	- Why: the plan's Non-goals already exclude making different hardware perform identically, and D-2026-07-05-1 restricts cross-direction verdicts to symmetric endpoints; no truly fs-identical pair exists in the fleet, so on the designated closest-spec rig the "better of the two old directions" bar can only bind net of the destination write-path residue the discriminator isolates. Settling the rule before the run prevents re-litigating it with numbers in hand.
.review/results/otp-12b-run.codex.md-1164-   172	- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).
.review/results/otp-12b-run.codex.md-1165-
.review/results/otp-12b-run.codex.md-1166-exec
.review/results/otp-12b-run.codex.md-1167-/bin/zsh -lc "nl -ba docs/bench/otp12-win-2026-07-12/verdicts.csv | sed -n '1,150p'" in /Users/michael/Dev/blit_v2
--
.review/results/otp-12b-run.codex.md-1273-   105	wm_tcp_small,converge,mac_init,old_session,1253,1360,0.921,1.10,PASS
.review/results/otp-12b-run.codex.md-1274-   106	wm_tcp_small,converge,mac_init,old_committed,1253,1280,0.979,1.10,PASS
.review/results/otp-12b-run.codex.md-1275-   107	wm_tcp_small,converge,win_init,old_session,1267,1360,0.932,1.10,PASS
.review/results/otp-12b-run.codex.md-1276-   108	wm_tcp_small,converge,win_init,old_committed,1267,1280,0.990,1.10,PASS
.review/results/otp-12b-run.codex.md-1277-   109	wm_tcp_small,cross,worst_arm,min_old_committed,1267,1280,0.990,1.10,PASS
.review/results/otp-12b-run.codex.md:1278:   110	gap_tcp_large,cross-gap,old_push,old_pull,1908,964,1.979,,RECORDED
.review/results/otp-12b-run.codex.md:1279:   111	gap_tcp_large,cross-gap,new_mw_worst,new_wm_worst,1920,984,1.951,,RECORDED
.review/results/otp-12b-run.codex.md:1280:   112	gap_tcp_small,cross-gap,old_push,old_pull,1811,1360,1.332,,RECORDED
.review/results/otp-12b-run.codex.md:1281:   113	gap_tcp_small,cross-gap,new_mw_worst,new_wm_worst,1935,1267,1.527,,RECORDED
.review/results/otp-12b-run.codex.md:1282:   114	gap_tcp_mixed,cross-gap,old_push,old_pull,1687,867,1.946,,RECORDED
.review/results/otp-12b-run.codex.md:1283:   115	gap_tcp_mixed,cross-gap,new_mw_worst,new_wm_worst,1587,1127,1.408,,RECORDED
.review/results/otp-12b-run.codex.md:1284:   116	gap_grpc_large,cross-gap,old_push,old_pull,1913,978,1.956,,RECORDED
.review/results/otp-12b-run.codex.md:1285:   117	gap_grpc_large,cross-gap,new_mw_worst,new_wm_worst,1931,993,1.945,,RECORDED
.review/results/otp-12b-run.codex.md:1286:   118	gap_grpc_small,cross-gap,old_push,old_pull,2942,1525,1.929,,RECORDED
.review/results/otp-12b-run.codex.md:1287:   119	gap_grpc_small,cross-gap,new_mw_worst,new_wm_worst,2261,1375,1.644,,RECORDED
.review/results/otp-12b-run.codex.md:1288:   120	gap_grpc_mixed,cross-gap,old_push,old_pull,2177,1408,1.546,,RECORDED
.review/results/otp-12b-run.codex.md:1289:   121	gap_grpc_mixed,cross-gap,new_mw_worst,new_wm_worst,1842,1262,1.460,,RECORDED
.review/results/otp-12b-run.codex.md-1290-
.review/results/otp-12b-run.codex.md-1291-exec
.review/results/otp-12b-run.codex.md-1292-/bin/zsh -lc 'nl -ba docs/bench/otp12-win-2026-07-12/summary.csv' in /Users/michael/Dev/blit_v2
.review/results/otp-12b-run.codex.md-1293- succeeded in 0ms:
.review/results/otp-12b-run.codex.md-1294-     1	cell,arm,median_ms,avg_ms,best_ms,spread_pct,voided_runs,pairs_attempted
--
.review/results/otp-12b-run.codex.md-1535-   105	wm_tcp_small,converge,mac_init,old_session,1253,1360,0.921,1.10,PASS
.review/results/otp-12b-run.codex.md-1536-   106	wm_tcp_small,converge,mac_init,old_committed,1253,1280,0.979,1.10,PASS
.review/results/otp-12b-run.codex.md-1537-   107	wm_tcp_small,converge,win_init,old_session,1267,1360,0.932,1.10,PASS
.review/results/otp-12b-run.codex.md-1538-   108	wm_tcp_small,converge,win_init,old_committed,1267,1280,0.990,1.10,PASS
.review/results/otp-12b-run.codex.md-1539-   109	wm_tcp_small,cross,worst_arm,min_old_committed,1267,1280,0.990,1.10,PASS
.review/results/otp-12b-run.codex.md:1540:   110	gap_tcp_large,cross-gap,old_push,old_pull,1908,964,1.979,,RECORDED
.review/results/otp-12b-run.codex.md:1541:   111	gap_tcp_large,cross-gap,new_mw_worst,new_wm_worst,1920,984,1.951,,RECORDED
.review/results/otp-12b-run.codex.md:1542:   112	gap_tcp_small,cross-gap,old_push,old_pull,1811,1360,1.332,,RECORDED
.review/results/otp-12b-run.codex.md:1543:   113	gap_tcp_small,cross-gap,new_mw_worst,new_wm_worst,1935,1267,1.527,,RECORDED
.review/results/otp-12b-run.codex.md:1544:   114	gap_tcp_mixed,cross-gap,old_push,old_pull,1687,867,1.946,,RECORDED
.review/results/otp-12b-run.codex.md:1545:   115	gap_tcp_mixed,cross-gap,new_mw_worst,new_wm_worst,1587,1127,1.408,,RECORDED
.review/results/otp-12b-run.codex.md:1546:   116	gap_grpc_large,cross-gap,old_push,old_pull,1913,978,1.956,,RECORDED
.review/results/otp-12b-run.codex.md:1547:   117	gap_grpc_large,cross-gap,new_mw_worst,new_wm_worst,1931,993,1.945,,RECORDED
.review/results/otp-12b-run.codex.md:1548:   118	gap_grpc_small,cross-gap,old_push,old_pull,2942,1525,1.929,,RECORDED
.review/results/otp-12b-run.codex.md:1549:   119	gap_grpc_small,cross-gap,new_mw_worst,new_wm_worst,2261,1375,1.644,,RECORDED
.review/results/otp-12b-run.codex.md:1550:   120	gap_grpc_mixed,cross-gap,old_push,old_pull,2177,1408,1.546,,RECORDED
.review/results/otp-12b-run.codex.md:1551:   121	gap_grpc_mixed,cross-gap,new_mw_worst,new_wm_worst,1842,1262,1.460,,RECORDED
.review/results/otp-12b-run.codex.md-1552-
.review/results/otp-12b-run.codex.md-1553-exec
.review/results/otp-12b-run.codex.md-1554-/bin/zsh -lc "nl -ba scripts/bench_otp12_win.sh | sed -n '770,1060p'" in /Users/michael/Dev/blit_v2
.review/results/otp-12b-run.codex.md-1555- succeeded in 0ms:
.review/results/otp-12b-run.codex.md-1556-   770	    gen_fixtures
--
.review/results/otp-12b-run.codex.md-1806-   533	    RUN_VALID=yes
.review/results/otp-12b-run.codex.md-1807-   534	    [[ "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
.review/results/otp-12b-run.codex.md-1808-   535	    RUN_MS=$(( T_MS + RUN_FLUSH )); RUN_EXIT=$T_RC
.review/results/otp-12b-run.codex.md-1809-
.review/results/otp-12b-run.codex.md-1810-exec
.review/results/otp-12b-run.codex.md:1811:/bin/zsh -lc "rg -n 'verdict|cross-gap|invariance|combined|old_committed|new_mw_worst|median|spread_pct|summary.csv|valid' scripts/bench_otp12_win.sh" in /Users/michael/Dev/blit_v2
.review/results/otp-12b-run.codex.md-1812- succeeded in 0ms:
.review/results/otp-12b-run.codex.md-1813-13:#   summary.csv), per design D2 as amended.
.review/results/otp-12b-run.codex.md-1814-15:#   BLOCK 2 — initiator/verb invariance (NEW pair only; the owner's
.review/results/otp-12b-run.codex.md-1815-33:# counterbalance, pair-void valid-run rule with 2xRUNS cap + INCOMPLETE,
.review/results/otp-12b-run.codex.md-1816-102:BASELINE_SUMMARY="$REPO_ROOT/docs/bench/otp2w-baseline-2026-07-10/summary.csv"
--
.review/results/otp-12b-run.codex.md-1846-706:        out.write(f"{cell},invariance,mac_init,win_init,,,,1.10,INCOMPLETE\n")
.review/results/otp-12b-run.codex.md-1847-711:    out.write(f"{cell},invariance,mac_init,win_init,{a},{b},{hi/lo:.3f},1.10,{'PASS' if inv else 'FAIL'}\n")
.review/results/otp-12b-run.codex.md-1848-715:    # COMPLETE (codex otp-12b F4 — a partial median never referees),
.review/results/otp-12b-run.codex.md-1849-729:        out.write(f"{cell},converge,{armname},old_committed,{val},{ref_m},{val/ref_m:.3f},1.10,{'PASS' if bar(val, ref_m) else 'FAIL'}\n")
.review/results/otp-12b-run.codex.md-1850-738:    out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")
.review/results/otp-12b-run.codex.md:1851:752:        out.write(f"gap_{carrier}_{fixture},cross-gap,old_push,old_pull,"
.review/results/otp-12b-run.codex.md:1852:754:        out.write(f"gap_{carrier}_{fixture},cross-gap,new_mw_worst,new_wm_worst,"
.review/results/otp-12b-run.codex.md-1853-791:    # BLOCK 2 — invariance (mac_init vs win_init, new pair only).
.review/results/otp-12b-run.codex.md-1854-818:    compute_verdicts
.review/results/otp-12b-run.codex.md-1855-821:    log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/comparison; ABBA) ==="
.review/results/otp-12b-run.codex.md-1856-822:    column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
.review/results/otp-12b-run.codex.md-1857-824:    log "=== VERDICTS (D2 both-references; invariance; F4 cross + gap rows) ==="
--
.review/results/otp-12b-run.codex.md-1969-   735	        sys.exit(f"FATAL: committed push/pull reference missing for {carrier}_{fixture}")
.review/results/otp-12b-run.codex.md-1970-   736	    cross_ref = min(p_ref, l_ref)
.review/results/otp-12b-run.codex.md-1971-   737	    worst = max(a, b)
.review/results/otp-12b-run.codex.md-1972-   738	    out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")
.review/results/otp-12b-run.codex.md-1973-   739	
.review/results/otp-12b-run.codex.md:1974:   740	# Discriminator gap rows (D-2026-07-12-1; recorded, never adjudicated):
.review/results/otp-12b-run.codex.md-1975-   741	# emitted only when ALL FOUR contributing cells are complete (codex
.review/results/otp-12b-run.codex.md-1976-   742	# otp-12b F4). Row operands are labeled exactly (codex otp-12b F12).
.review/results/otp-12b-run.codex.md-1977-   743	for carrier in ("tcp", "grpc"):
.review/results/otp-12b-run.codex.md-1978-   744	    for fixture in ("large", "small", "mixed"):
.review/results/otp-12b-run.codex.md-1979-   745	        four = [f"push_{carrier}_{fixture}", f"pull_{carrier}_{fixture}",
--
.review/results/otp-12b-run.codex.md-1981-   747	        if not all(complete(c) for c in four):
.review/results/otp-12b-run.codex.md-1982-   748	            continue
.review/results/otp-12b-run.codex.md-1983-   749	        po, lo_ = m(four[0], "old"), m(four[1], "old")
.review/results/otp-12b-run.codex.md-1984-   750	        mw_w = max(m(four[2], "mac_init"), m(four[2], "win_init"))
.review/results/otp-12b-run.codex.md-1985-   751	        wm_w = max(m(four[3], "mac_init"), m(four[3], "win_init"))
.review/results/otp-12b-run.codex.md:1986:   752	        out.write(f"gap_{carrier}_{fixture},cross-gap,old_push,old_pull,"
.review/results/otp-12b-run.codex.md-1987-   753	                  f"{po},{lo_},{po/lo_:.3f},,RECORDED\n")
.review/results/otp-12b-run.codex.md:1988:   754	        out.write(f"gap_{carrier}_{fixture},cross-gap,new_mw_worst,new_wm_worst,"
.review/results/otp-12b-run.codex.md-1989-   755	                  f"{mw_w},{wm_w},{mw_w/wm_w:.3f},,RECORDED\n")
.review/results/otp-12b-run.codex.md-1990-   756	out.close()
.review/results/otp-12b-run.codex.md-1991-   757	PYEOF
.review/results/otp-12b-run.codex.md-1992-   758	}
.review/results/otp-12b-run.codex.md-1993-   759	
--
.review/results/otp-12b-run.codex.md-2122-   725	        sys.exit(f"FATAL: committed push/pull reference missing for {carrier}_{fixture}")
.review/results/otp-12b-run.codex.md-2123-   726	    cross_ref = min(p_ref, l_ref)
.review/results/otp-12b-run.codex.md-2124-   727	    worst = max(a, b)
.review/results/otp-12b-run.codex.md-2125-   728	    out.write(f"{cell},cross,worst_arm,min_old_committed,{worst},{cross_ref},{worst/cross_ref:.3f},1.10,{'PASS' if bar(worst, cross_ref) else 'FAIL'}\n")
.review/results/otp-12b-run.codex.md-2126-   729	
.review/results/otp-12b-run.codex.md:2127:   730	# Discriminator gap rows (D-2026-07-12-1; recorded, never adjudicated):
.review/results/otp-12b-run.codex.md-2128-   731	# emitted only when ALL FOUR contributing cells are complete (codex
.review/results/otp-12b-run.codex.md-2129-   732	# otp-12b F4). Row operands are labeled exactly (codex otp-12b F12).
.review/results/otp-12b-run.codex.md-2130-   733	for carrier in ("tcp", "grpc"):
.review/results/otp-12b-run.codex.md-2131-   734	    for fixture in ("large", "small", "mixed"):
.review/results/otp-12b-run.codex.md-2132-   735	        four = [f"push_{carrier}_{fixture}", f"pull_{carrier}_{fixture}",
--
.review/results/otp-12b-run.codex.md-2134-   737	        if not all(complete(c) for c in four):
.review/results/otp-12b-run.codex.md-2135-   738	            continue
.review/results/otp-12b-run.codex.md-2136-   739	        po, lo_ = m(four[0], "old"), m(four[1], "old")
.review/results/otp-12b-run.codex.md-2137-   740	        mw_w = max(m(four[2], "mac_init"), m(four[2], "win_init"))
.review/results/otp-12b-run.codex.md-2138-   741	        wm_w = max(m(four[3], "mac_init"), m(four[3], "win_init"))
.review/results/otp-12b-run.codex.md:2139:   742	        out.write(f"gap_{carrier}_{fixture},cross-gap,old_push,old_pull,"
.review/results/otp-12b-run.codex.md-2140-   743	                  f"{po},{lo_},{po/lo_:.3f},,RECORDED\n")
.review/results/otp-12b-run.codex.md:2141:   744	        out.write(f"gap_{carrier}_{fixture},cross-gap,new_mw_worst,new_wm_worst,"
.review/results/otp-12b-run.codex.md-2142-   745	                  f"{mw_w},{wm_w},{mw_w/wm_w:.3f},,RECORDED\n")
.review/results/otp-12b-run.codex.md-2143-   746	out.close()
.review/results/otp-12b-run.codex.md-2144-   747	PYEOF
.review/results/otp-12b-run.codex.md-2145-   748	}
.review/results/otp-12b-run.codex.md-2146-   749	
--
.review/results/otp-12b-run.codex.md-2209-.review/findings/otp-12a-zoey-harness.md-71-  `summary.csv`'s `pairs_attempted` column and the INCOMPLETE verdict
.review/results/otp-12b-run.codex.md-2210-.review/findings/otp-12a-zoey-harness.md-72-  rows.
.review/results/otp-12b-run.codex.md-2211---
.review/results/otp-12b-run.codex.md-2212-docs/plan/OTP12_ACCEPTANCE_RUN.md-173-  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12b-run.codex.md-2213-docs/plan/OTP12_ACCEPTANCE_RUN.md-174-  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12b-run.codex.md:2214:docs/plan/OTP12_ACCEPTANCE_RUN.md-175-  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12b-run.codex.md-2215-docs/plan/OTP12_ACCEPTANCE_RUN.md-176-  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12b-run.codex.md-2216-docs/plan/OTP12_ACCEPTANCE_RUN.md-177-
.review/results/otp-12b-run.codex.md-2217-docs/plan/OTP12_ACCEPTANCE_RUN.md:178:Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12b-run.codex.md-2218-docs/plan/OTP12_ACCEPTANCE_RUN.md:179:bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12b-run.codex.md-2219-docs/plan/OTP12_ACCEPTANCE_RUN.md-180-interleaved in a fresh session; both sessions are committed.
--
.review/results/otp-12b-run.codex.md-2320-.review/results/otp-12b-run.codex.md-348-    53	push_tcp_large 1908 vs 3054) — reference drift in the fast direction,
.review/results/otp-12b-run.codex.md-2321-.review/results/otp-12b-run.codex.md-349-    54	so the committed bars are easy and the same-session bars are the
.review/results/otp-12b-run.codex.md-2322---
.review/results/otp-12b-run.codex.md-2323-.review/results/otp-12b-run.codex.md-586-docs/plan/OTP12_ACCEPTANCE_RUN.md-173-  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12b-run.codex.md-2324-.review/results/otp-12b-run.codex.md-587-docs/plan/OTP12_ACCEPTANCE_RUN.md-174-  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12b-run.codex.md:2325:.review/results/otp-12b-run.codex.md-588-docs/plan/OTP12_ACCEPTANCE_RUN.md:175:  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12b-run.codex.md-2326-.review/results/otp-12b-run.codex.md-589-docs/plan/OTP12_ACCEPTANCE_RUN.md-176-  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12b-run.codex.md-2327-.review/results/otp-12b-run.codex.md-590-docs/plan/OTP12_ACCEPTANCE_RUN.md-177-
.review/results/otp-12b-run.codex.md-2328-.review/results/otp-12b-run.codex.md:591:docs/plan/OTP12_ACCEPTANCE_RUN.md:178:Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12b-run.codex.md-2329-.review/results/otp-12b-run.codex.md:592:docs/plan/OTP12_ACCEPTANCE_RUN.md:179:bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12b-run.codex.md-2330-.review/results/otp-12b-run.codex.md-593-docs/plan/OTP12_ACCEPTANCE_RUN.md-180-interleaved in a fresh session; both sessions are committed.
--
.review/results/otp-12b-run.codex.md-2338-.review/results/otp-12b-run.codex.md-601-docs/plan/OTP12_ACCEPTANCE_RUN.md-188-### D3 — reverse-initiator arms (rig W invariance; first-of-kind plumbing)
.review/results/otp-12b-run.codex.md-2339-.review/results/otp-12b-run.codex.md-602-docs/plan/OTP12_ACCEPTANCE_RUN.md-189-
.review/results/otp-12b-run.codex.md-2340---
.review/results/otp-12b-run.codex.md-2341-.review/results/otp-12b-run.codex.md-1014-   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12b-run.codex.md-2342-.review/results/otp-12b-run.codex.md-1015-   174	  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12b-run.codex.md:2343:.review/results/otp-12b-run.codex.md-1016-   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12b-run.codex.md-2344-.review/results/otp-12b-run.codex.md-1017-   176	  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12b-run.codex.md-2345-.review/results/otp-12b-run.codex.md-1018-   177	
.review/results/otp-12b-run.codex.md-2346-.review/results/otp-12b-run.codex.md:1019:   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12b-run.codex.md-2347-.review/results/otp-12b-run.codex.md:1020:   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12b-run.codex.md-2348-.review/results/otp-12b-run.codex.md-1021-   180	interleaved in a fresh session; both sessions are committed.
--
.review/results/otp-12b-run.codex.md-2432-.review/results/otp-12a.codex.md-976-    42	#   export ZOEY_TEMP=/volume/<pool>/.srv/.unifi-drive/michael/.data/blit-temp
.review/results/otp-12b-run.codex.md-2433-.review/results/otp-12a.codex.md-977-    43	#   export ZOEY_HOST=10.1.10.206        # pin the 10GbE path by IP
.review/results/otp-12b-run.codex.md-2434---
.review/results/otp-12b-run.codex.md-2435-.review/results/otp-12a.codex.md-1596-   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12b-run.codex.md-2436-.review/results/otp-12a.codex.md-1597-   174	  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12b-run.codex.md:2437:.review/results/otp-12a.codex.md-1598-   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12b-run.codex.md-2438-.review/results/otp-12a.codex.md-1599-   176	  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12b-run.codex.md-2439-.review/results/otp-12a.codex.md-1600-   177	
.review/results/otp-12b-run.codex.md-2440-.review/results/otp-12a.codex.md:1601:   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12b-run.codex.md-2441-.review/results/otp-12a.codex.md:1602:   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12b-run.codex.md-2442-.review/results/otp-12a.codex.md-1603-   180	interleaved in a fresh session; both sessions are committed.
--
.review/results/otp-12b-run.codex.md-2457-.review/results/otp-12a.codex.md-3062-    71	  `summary.csv`'s `pairs_attempted` column and the INCOMPLETE verdict
.review/results/otp-12b-run.codex.md-2458-.review/results/otp-12a.codex.md-3063-    72	  rows.
.review/results/otp-12b-run.codex.md-2459---
.review/results/otp-12b-run.codex.md-2460-.review/results/otp-12a.codex.md-3487-   173	  identically); gap closed ⇒ the code was the cost and the bar is met. The
.review/results/otp-12b-run.codex.md-2461-.review/results/otp-12a.codex.md-3488-   174	  README records BOTH computations per cell; a discriminator-attributed
.review/results/otp-12b-run.codex.md:2462:.review/results/otp-12a.codex.md-3489-   175	  platform-residue cell counts as satisfied (owner, D-2026-07-12-1), and
.review/results/otp-12b-run.codex.md-2463-.review/results/otp-12a.codex.md-3490-   176	  the otp-13 walk reviews the recorded numbers.
.review/results/otp-12b-run.codex.md-2464-.review/results/otp-12a.codex.md-3491-   177	
.review/results/otp-12b-run.codex.md-2465-.review/results/otp-12a.codex.md:3492:   178	Escalation rule (pre-registered, not ad-hoc): if a comparison straddles its
.review/results/otp-12b-run.codex.md-2466-.review/results/otp-12a.codex.md:3493:   179	bar and either arm's spread exceeds 25%, that comparison reruns at RUNS=8
.review/results/otp-12b-run.codex.md-2467-.review/results/otp-12a.codex.md-3494-   180	interleaved in a fresh session; both sessions are committed.
--
.review/results/otp-12b-run.codex.md-2873-wm_tcp_mixed 1.237 (TCP x mixed x destination-initiator - real,
.review/results/otp-12b-run.codex.md-2874-corroborated by block-1 pull_tcp_mixed 1.313; code-shaped finding).
.review/results/otp-12b-run.codex.md-2875-Converge fails: push_tcp_small 1.149 (matches zoey 1.105 - second
.review/results/otp-12b-run.codex.md-2876-rig), pull_tcp_mixed (same root). Cross: Win->Mac all beat the better
.review/results/otp-12b-run.codex.md-2877-old direction; Mac->Win residue platform-attributed by unchanged/
.review/results/otp-12b-run.codex.md:2878:narrowed gap rows (D-2026-07-12-1) except tcp_small (gap widened =
.review/results/otp-12b-run.codex.md-2879-the code component). CR post-processing recorded. Declares nothing.
.review/results/otp-12b-run.codex.md-2880-
.review/results/otp-12b-run.codex.md-2881-Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
.review/results/otp-12b-run.codex.md-2882-
.review/results/otp-12b-run.codex.md-2883-
--
.review/results/otp-12b-run.codex.md-2919-    wm_tcp_mixed 1.237 (TCP x mixed x destination-initiator - real,
.review/results/otp-12b-run.codex.md-2920-    corroborated by block-1 pull_tcp_mixed 1.313; code-shaped finding).
.review/results/otp-12b-run.codex.md-2921-    Converge fails: push_tcp_small 1.149 (matches zoey 1.105 - second
.review/results/otp-12b-run.codex.md-2922-    rig), pull_tcp_mixed (same root). Cross: Win->Mac all beat the better
.review/results/otp-12b-run.codex.md-2923-    old direction; Mac->Win residue platform-attributed by unchanged/
.review/results/otp-12b-run.codex.md:2924:    narrowed gap rows (D-2026-07-12-1) except tcp_small (gap widened =
.review/results/otp-12b-run.codex.md-2925-    the code component). CR post-processing recorded. Declares nothing.
.review/results/otp-12b-run.codex.md-2926-    
.review/results/otp-12b-run.codex.md-2927-    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
.review/results/otp-12b-run.codex.md-2928-
.review/results/otp-12b-run.codex.md-2929- docs/bench/otp12-win-2026-07-12/README.md          | 109 ++++++++++++

exec
/bin/zsh -lc "sed -n '1,220p' .review/README.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# Blit review workflow

> **Superseded as the grading mechanism (2026-07-04, D-2026-07-04-1).**
> All code and plan changes now go through the synchronous codex loop in
> `docs/agent/GPT_REVIEW_LOOP.md` — no exceptions. That loop reuses this
> directory's `findings/` + `results/` records and the `REVIEW.md` status
> index, which remain live. The async parts below — `ready/` sentinels,
> `reviewer-wait.sh`, the separate Reviewer agent, WIP limits — are
> historical; do not write new sentinels. The Identity section (sign the
> `reviewer` field honestly) still applies to verdict records.

Two-agent loop: **Coder** is the implementer, **Reviewer** is the
gate. `REVIEW.md` at the repo root is the human-readable status
index; this directory is the structured handoff channel.

For the workflow's design rationale (and onboarding any new agent
into the contract) see this contract plus `AGENTS.md`. (The portable
two-agent setup playbook is maintained in the governance system, not
in-repo.)

## Layout

```
.review/
├── README.md                     This file — the project-specific contract
├── findings/<id>.md              Implementation record per finding
├── ready/<id>.json               Coder → reviewer signal
└── results/
    ├── <id>.verified.json        Reviewer → coder: accepted
    └── <id>.reopened.md          Reviewer → coder: needs fix-ups

REVIEW.md                         (root) Human-readable status index
```

Everything under `.review/` is committed. The audit trail of
`ready/` and `results/` is part of the project's verification
history.

## Validation suite — the green-light gate

Every coder commit MUST pass all three before the sentinel goes
out. Reviewer re-runs them as the first step of grading. Run from
the repo root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Tests must show "passed" with zero failures. Test count may grow
as new tests land, but never drop versus the prior baseline
unless a test was intentionally removed (and the removal is
called out in the finding doc's **Known gaps**).

## Branch model

**Default**: one branch per finding, named
`fix/<id-lowercased>-<short-slug>` (e.g. `fix/c-1-uuid-traversal`).
One coherent slice per branch; no bundling.

**Exception — linear refactor sequences**: a single feature
branch (`phase5/blit-app-extract` is the current example) may
host multiple atomic per-commit slices when the slices form a
dependency chain (slice N requires slice N-1's structure to
exist). Each commit on the branch is its own atomic unit and
gets its own finding doc + sentinel + verdict; the branch
unifies them only because rebasing slice N onto master before
slice N-1 lands would break the build. The reviewer grades
slice by slice, just on a shared branch.

When in doubt, default to per-finding branches.

## Coder loop

1. Pick the highest-priority `[ ]` (Open) item in `REVIEW.md`.
2. Create branch (or, on a linear sequence, use the existing
   feature branch — see exception above). Implement the fix and
   write tests.
3. Run the **Validation suite**. Do not commit on failure.
4. Commit with subject `Fix <id>: <one-line summary>` (or, on a
   linear sequence, the sub-slice's natural commit subject) and a
   body mirroring `.review/findings/<id>.md`.
5. Write `.review/findings/<id>.md` with: **What / Approach /
   Files changed / Tests added / Known gaps**.
6. Update `REVIEW.md` row: `[ ]` → `[~]`, link the branch + commit.
7. Atomic sentinel write — use `mktemp` then `mv`:
   ```bash
   tmp=$(mktemp .review/ready/.<id>.json.XXXX)
   cat > "$tmp" <<EOF
   {"id":"<id>","branch":"<branch>","sha":"$(git rev-parse HEAD)","ts":"$(date -u +%Y-%m-%dT%H:%M:%SZ)"}
   EOF
   mv "$tmp" .review/ready/<id>.json
   ```
8. Commit the sentinel + finding doc + REVIEW.md update on the
   same branch.
9. Move to the next finding. Do not wait for reviewer verdict to
   start the next branch — but do not stack work on a branch that
   already has a `.review/ready/<id>.json` pending without
   refreshing the sentinel.

## Reviewer loop

The reviewer must use a wake mechanism that returns control to
the agent when a sentinel exists. A plain background shell loop is
not enough in agent harnesses where stdout buffers until manually
polled; that leaves the human in the loop.

Default wake command:

```bash
# from the repo root:
.review/reviewer-wait.sh
```

`reviewer-wait.sh` is a one-shot blocking poll. It exits as soon
as `.review/ready/*.json` contains at least one sentinel, prints
`READY: <id>.json`, prints the sentinel JSON payload, and returns
control to the reviewer. After grading and committing the verdict,
run it again to wait for the next item.

Optional tuning:

```bash
REVIEW_POLL_INTERVAL_SECONDS=2 .review/reviewer-wait.sh
REVIEW_WAIT_TIMEOUT_SECONDS=300 .review/reviewer-wait.sh
```

`REVIEW_WAIT_TIMEOUT_SECONDS=0` (the default) waits forever. A
non-zero timeout prints `NO_READY` and exits 2 if no sentinel
appears in time.

If the harness provides a real Monitor/watch tool that wakes the
agent on stdout lines, it may wrap `.review/reviewer-wait.sh` or
an equivalent watcher. Before relying on that mode, prove it with
a probe sentinel: the reviewer must receive and grade the probe
without the human prompting "check the queue." If that proof has
not happened, use the blocking wait command above.

Per-sentinel steps:

1. Read `.review/ready/<id>.json`, parse `branch` + `sha`.
2. `git checkout <branch>` (or use a worktree). Run validation.
3. Inspect the diff `<prev>..<sha>` (or `master..<sha>` for
   per-finding branches) with the finding scope in mind.
4. Write the verdict:
   - **Accepted** → `.review/results/<id>.verified.json`:
     ```json
     {"id":"<id>","sha":"<sha>","ts":"<utc-iso8601>","reviewer":"<name>"}
     ```
     Update `REVIEW.md` row to `[x]`. Delete `.review/ready/<id>.json`.
     For per-finding branches: fast-forward merge into master (or
     leave for the coder to merge if higher-stakes).
   - **Reopened** → `.review/results/<id>.reopened.md` with
     concrete file:line comments. Update `REVIEW.md` row to `[ ]`.
     Delete `.review/ready/<id>.json`. The branch stays so the
     coder can push fix-ups; coder writes a new sentinel after
     addressing comments.
5. Commit the verdict file + REVIEW.md update.

## WIP limits

- **Strict (default)**: at most one branch may have a pending
  sentinel at a time.
- **Faster mode**: multiple sentinels permitted iff each
  branch's `Files changed` is fully disjoint from every other
  pending branch.
- **Linear-sequence exception**: a single feature branch may
  have at most one pending sentinel at a time; the coder pauses
  before issuing the next slice's sentinel until the current
  one is graded.

## Anti-patterns

- **Broad sweeps.** "Fix C-1..M-6 in one commit" — kills
  bisection. Allowed only on explicit human request.
- **Editing `REVIEW.md` prose freely.** It's a status index.
  Long-form discussion goes in `.review/findings/<id>.md` or
  `.review/results/<id>.reopened.md`.
- **Skipping the sentinel.** "I committed and assumed the
  reviewer would see it." The Monitor watches sentinels, not
  commits. No sentinel = no review.
- **Stacking new commits on a pending-review branch.** Wait for
  the verdict or refresh the sentinel.
- **Reviewer modifying the coder's branch's code.** Reviewer's
  job is verdict + merge (or reopen). Reviewer's only writes
  are to `.review/results/` and the `REVIEW.md` status column.
  Reviewer does not push code fix-ups; that's the coder's role.

## Identity

- **Coder** and **Reviewer** are roles, not fixed names. Any model may
  fill either role; the same model must not grade its own slice.
- The `reviewer` field in `.review/results/<id>.verified.json` records
  **which model actually performed the review** — `claude-reviewer`,
  `grok-reviewer`, `gpt-reviewer`, `owner`, etc. Sign honestly; the
  field exists for the audit trail, not as a magic trigger.
  (2026-06-12: seven w* verdicts originally signed `claude-reviewer`
  were corrected to `grok-reviewer` — the owner had a Grok session
  adopt the name because this section used to hardcode it.)

exec
/bin/zsh -lc "rg -n '"'^## F[0-9]+|'"\\*\\*Severity|Severity:' .review/findings .review/results | head -80" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
.review/findings/relay-1-subpath-double-join.md:4:**Severity**: Low (latent user-facing break of `--relay-via-cli` with a
.review/findings/a1-4-f3-browse.md:3:**Severity**: Feature (fourth slice of milestone A.1 — adds F3)
.review/findings/bridge-3-prometheus-readme.md:3:**Severity**: Docs (Milestone E — optional Prometheus bridge, step 3)
.review/findings/audit-7d3-extract-display-f1.md:3:**Severity**: Refactor / code-health
.review/results/otp-4a-daemon-serves-transfer.gpt-verdict.md:13:## F1 — cancel emits `Status::cancelled`, not a `SessionError{CANCELLED}` frame (Medium)
.review/findings/d-20-f2-recent-throughput.md:3:**Severity**: Feature (polish — pairs F2 recent table
.review/findings/bug-mirror-literal-backslash.md:3:**Severity**: Bug fix / correctness
.review/results/otp-10b-1.gpt-verdict.md:10:## F1 (High) — hash failure dropped the header ⇒ silent absence
.review/results/otp-10b-1.gpt-verdict.md:25:## F2 (Med) — detached hashing task unowned by the session
.review/results/otp-10b-1.gpt-verdict.md:34:## F3 (Med) — destination hash chunk non-cancellable
.review/results/otp-10b-1.gpt-verdict.md:46:## F4 (Med) — `CHECKSUM_DISABLED` missing from delegated phase map
.review/results/otp-10b-1.gpt-verdict.md:53:## F5 (Low) — STATE residue line contradicted the commit
.review/findings/d-31-help-scroll.md:3:**Severity**: Feature (polish — small-terminal usability)
.review/results/ue-r2-1a.gpt-verdict.md:18:## F1 — workers don't re-check `cancelled` (pipeline.rs)
.review/results/ue-r2-1a.gpt-verdict.md:28:**Severity downgraded High → Medium:** the error still wins and byte/file
.review/results/ue-r2-1a.gpt-verdict.md:40:## F2 — `send_block` doesn't record probe bytes (data_plane.rs:537)
.review/results/ue-r2-1a.gpt-verdict.md:54:## F3 — byte/file total test only checks aggregates (pipeline.rs)
.review/results/ue-r2-1a.gpt-verdict.md:68:## F4 — no multi-sink cancel-under-backpressure test (pipeline.rs)
.review/findings/a0-endpoints-gates.md:3:**Severity**: Refactor (no behavior change)
.review/findings/audit-5-bridge-robustness.md:3:**Severity**: Robustness
.review/results/otp-2w.gpt-verdict.md:9:## F1 (Med) — drain probe fails open (`$null -lt N` is true)
.review/results/otp-2w.gpt-verdict.md:17:## F2 (Med) — WMI PID discarded; kill-by-name; stale daemon masks failures
.review/results/otp-2w.gpt-verdict.md:25:## F3 (Med) — durability costs asymmetric across directions, undisclosed
.review/results/otp-2w.gpt-verdict.md:42:## F4 (Med) — 7/12 not 8/12 cells ≤2%
.review/results/otp-2w.gpt-verdict.md:50:## F5 (Low) — purge-standby.ps1 unchecked API calls, leaked handle
.review/results/otp-2w.gpt-verdict.md:56:## F6 (Low) — "NEAR-SYMMETRIC" overstates the owner's designation
.review/results/otp-2w.gpt-verdict.md:61:## F7 (Low) — finding doc referenced nonexistent drain.log
.review/results/otp-4b3-data-plane.fix-review.codex.md:665:## F1 (High) — `mod.rs:888` `dp.queue()` not raced against a peer fault — ACCEPTED
.review/results/otp-4b3-data-plane.fix-review.codex.md:683:## F2 (Medium) — `transfer_session_e2e.rs:253` "bytes flowed" gate fires before TCP — ACCEPTED
.review/results/otp-4b3-data-plane.fix-review.codex.md:694:## F3 (Medium) — `mod.rs:1176` `recv_peer_fault` silently drops non-fault events — ACCEPTED
.review/findings/audit-15-grpc-missing-connection-timeouts.md:3:**Severity**: Robustness
.review/findings/b-1-active-jobs.md:3:**Severity**: Feature (no behavior change visible on the wire)
.review/results/otp-10c-2.gpt-verdict.md:11:## F1 (Med) — spec capability/capacity fields semantically orphaned — **Accepted**
.review/results/otp-10c-2.gpt-verdict.md:26:## F2 (Med) — five newly orphaned helpers — **Accepted**
.review/results/otp-10c-2.gpt-verdict.md:39:## F3 (Med) — relocated builder lost its direct pins — **Accepted**
.review/results/otp-10c-2.gpt-verdict.md:50:## F4 (Med) — containment claim unsupported — **Accepted**
.review/results/otp-10c-2.gpt-verdict.md:66:## F5 (Med) — stale live docs — **Accepted**
.review/results/otp-10c-2.gpt-verdict.md:82:## F6 (Low) — tracked worktree snapshot still contains the old tree — **Accepted, owner-gated**
.review/findings/d-49-f3-multiselect.md:3:**Severity**: Feature (designed — TUI_DESIGN §5.3 `space`)
.review/findings/d-25-f2-tib-tier.md:3:**Severity**: Feature (polish — cross-pane consistency)
.review/findings/keys-4-config-movement.md:3:**Severity**: Feature (Milestone E — key remapping, step 4)
.review/findings/d-71-f1-delegated-move.md:3:**Severity**: Feature (TUI_DESIGN §1 "move … between any two endpoints")
.review/findings/audit-3a-mutex-poisoning.md:3:**Severity**: Robustness
.review/findings/audit-2b-remote-connect-timeout.md:3:**Severity**: Robustness
.review/results/otp-2.gpt-verdict.md:9:## F1 (High) — "symmetric baseline" mislabeled; per-direction observations only
.review/results/otp-2.gpt-verdict.md:19:## F2 (High) — macOS `sync` does not guarantee durable pull windows
.review/results/otp-2.gpt-verdict.md:28:## F3 (High) — STATE pre-adjudicated the owner question and advanced to otp-10
.review/results/otp-2.gpt-verdict.md:36:## F4 (Medium) — drain checked quiet before remote sync; timeout silent
.review/results/otp-2.gpt-verdict.md:44:## F5 (Medium) — fixed push destinations; interrupted runs poison reruns
.review/results/otp-2.gpt-verdict.md:52:## F6 (Medium) — quantitative claims exceeded the CSV evidence
.review/results/otp-2.gpt-verdict.md:63:## F7 (Low) — median flooring unstated
.review/results/otp-2.gpt-verdict.md:69:## F8 (Low) — non-monotonic wall time + undocumented python3
.review/findings/d-14-f2-active-row-age.md:3:**Severity**: Feature (polish — closes the d-13 known
.review/results/otp-9a.gpt-verdict.md:11:## F1 (Low) — stale `PullSessionOptions` rustdoc
.review/findings/d-27-f3-sort.md:3:**Severity**: Feature (polish — UX consistency across
.review/findings/w4-4-blocking-work-off-runtime.md:8:**Severity**: Medium (FAST/RELIABLE — blocking syscalls and full-file
.review/findings/d-61-f1-trigger-push.md:3:**Severity**: Feature (designed — TUI_DESIGN §1 "between any two endpoints")
.review/findings/d-30-batch-cancel.md:3:**Severity**: Feature (polish — closes d-22 known gap #2)
.review/findings/d-67-help-clear-confirm.md:3:**Severity**: Feature (doc-consistency — keymap honesty)
.review/findings/e-4-config-tab-strip-counts.md:3:**Severity**: Feature (polish — schema growth on the
.review/results/session-2026-07-06.gpt-verdict.md:18:## F1 (Medium) — LOCAL_ERROR_TELEMETRY.md header stale re: Q5 — ACCEPTED
.review/results/session-2026-07-06.gpt-verdict.md:30:## F2 (Medium) — STATE.md contradicts the plan doc's own Q5 honesty fix — ACCEPTED
.review/results/session-2026-07-06.gpt-verdict.md:43:## F3 (Medium) — STATE.md handoff log stale — ACCEPTED
.review/results/session-2026-07-06.gpt-verdict.md:55:## F4 (Low) — DEVLOG.md ordering looks broken — ACCEPTED (note added, no rewrite)
.review/results/session-2026-07-06.gpt-verdict.md:73:## F5 (Low) — "every writer hardcodes error_count to 0" overstated — ACCEPTED
.review/findings/d-59-f1-trigger-mirror.md:3:**Severity**: Feature (designed — TUI_DESIGN §3 / §5.2)
.review/findings/c-4-transfer-progress.md:3:**Severity**: Feature (third event-family member for milestone C's Subscribe wire surface)
.review/findings/c-3-transfer-finished-events.md:3:**Severity**: Feature (second event-family pair for milestone C's Subscribe wire surface)
.review/findings/d-10-transfer-throughput.md:3:**Severity**: Feature (polish — complements d-8's
.review/results/otp-11-design.gpt-verdict.md:12:## F1 (High) — "D1 violates one-transfer-path; choreography changed, not just the carrier"
.review/results/otp-11-design.gpt-verdict.md:39:## F2 (High) — sink File payload not single-file-safe (empty rel → ENOTDIR)
.review/results/otp-11-design.gpt-verdict.md:46:## F3 (High) — session diff and sink re-check can disagree (partial-hash / mtime tolerance)
.review/results/otp-11-design.gpt-verdict.md:60:## F4 (High) — dest-inside-src exclusion lost
.review/results/otp-11-design.gpt-verdict.md:67:## F5 (High) — sink-level resume is a topology-specific resume path
.review/results/otp-11-design.gpt-verdict.md:80:## F6 (Medium) — symlink parity claim false for `preserve_symlinks=false`; `skip_unchanged=false` omitted
.review/results/otp-11-design.gpt-verdict.md:90:## F7 (Medium) — mirror pass counts/plan-only didn't exist; SourceEmpty would hide deletions
.review/results/otp-11-design.gpt-verdict.md:101:## F8 (High) — retiring journal skip conflicts with the no-op ≤ old+10% gate
.review/results/otp-11-design.gpt-verdict.md:116:## F9 (Medium) — deleting `execute_sink_pipeline_streaming` leaves callers/tests
.review/results/otp-11-design.gpt-verdict.md:122:## F10 (High) — floor arithmetic does not close; manifest "live-half tests" wrong
.review/findings/audit-7d4-extract-display-f2.md:3:**Severity**: Refactor / code-health

exec
/bin/zsh -lc "sed -n '1,200p' .review/findings/otp-12b-win-harness.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# otp-12b — Mac↔Windows harness (converge-up + initiator/verb invariance)

**Plan**: `docs/plan/OTP12_ACCEPTANCE_RUN.md` (Active), sub-slice 12b,
harness half. The recorded-run half follows on the rig (fresh bundle to
the Windows box; its checkout moves off the `0f922de` detachment — the
owner's `bench-cargo-lock` stash untouched).
**Status**: implemented, codex review pending.

## What

`scripts/bench_otp12_win.sh` — two blocks on the owner-designated
closest-spec pair. Block 1: the otp-2w matrix as matched-pair
interleaved old(`0f922de`)/new A/B, Mac-initiated, verdicts against
both references (same-session old + the committed
`otp2w-baseline-2026-07-10` medians). Block 2 — the plan's headline
criterion measured for the first time: per data direction × fixture ×
carrier, Mac-initiated vs Windows-initiated arms interleaved ABBA
(`mw_*`/`wm_*` cells), plus per-arm converge rows (design F3), the F4
cross-direction rows vs `min(old_push, old_pull)` committed, and
D-2026-07-12-1 discriminator gap rows (kind `cross-gap`, outcome
`RECORDED` — the harness never adjudicates the residue).

## Approach

Windows plumbing verbatim from the frozen `bench_otp2w_baseline.sh`
(WMI launch, stale-refusal + PID-scoped teardown, TOML literal paths,
Get-Counter drain with fail-loud errors, standby purge, self-timed
`Write-VolumeCache`, CRLF stripping). Every otp-12a lesson carried:
ABBA counterbalance, pair-void valid-run rule (2×RUNS cap, INCOMPLETE),
exit codes checked, `+sha` provenance greps (Windows side via
`Select-String -SimpleMatch`), fail-closed sha256 manifest (7 hashes),
per-run destination sweep after the measured flush (the zoey storm
lesson, kept uniform), PREFLIGHT_ONLY, CELLS allowlist + typo
validation, session-gated traps with identity-verified kills both
hosts. New mechanics: arm swap via a fixed active exe path
(`bins\active\blit-daemon.exe`, one program-scoped firewall rule
`blit-otp12-daemon`; sha-named source dirs keep provenance); a Mac
daemon serving `$MAC_WORK` itself as the module root (design F6 — both
initiators of a Mac→Win cell read the same physical inodes); the
Windows-initiated timed window measured ON Windows (Stopwatch brackets
the `blit.exe` run inside one ssh call, printing `<ms>,<exit>` — the
otp-2w F3 rule applied to a whole client run); flush keyed by
destination OS never verb; block-2 arms do identical work (symmetric
no-trailing-slash nesting both arms — block 1 keeps the otp-2w shapes
for baseline comparability); a win→mac smoke gates the macOS
application-firewall unknown before anything is timed.

## Files

- `scripts/bench_otp12_win.sh` (new, executable; self-contained per D5 —
  the frozen otp-2w script untouched).
- `docs/plan/OTP12_ACCEPTANCE_RUN.md` — D5 cell grammar extended for
  the invariance (`mw|wm`) and `gap_*`/`cross-gap` rows.

## Tests

- `bash -n` clean; shellcheck not installed on this machine (recorded).
- No crates/proto/Cargo changes; the suite stands at the recorded 1484.
- The harness itself is verified by the preflight/smoke discipline on
  the rig; the recorded-run half commits the evidence.

## Known gaps

- Not yet executed — first contact with: `Select-String` provenance
  greps on ~10 MB exes, `$LASTEXITCODE` capture through the mux'd ssh,
  the macOS application firewall (smoke-gated), and blit.exe writing
  into the served module directory as a local path.
- The old Mac client (`0f922de`) predates embedded client build ids
  (otp-12a-run F1) — `OLD_CLIENT_PROVENANCE_BY_BUILD=1` acknowledgment
  required, provenance = clean-worktree rebuild + manifest.
- Mac-destination runs have no drain equivalent (recorded design
  decision D3): `sync` + purge only, exactly as the recorded otp-2w
  pull cells.
- Block-2 converge rows compare against block-1 same-session old arms;
  if block 1 is CELLS-filtered away in an escalation session, those
  rows fall back to committed-reference-only (the python emits what
  exists — the README of the recorded run must note any such session).

exec
/bin/zsh -lc "sed -n '1,85p' .review/results/otp-2.gpt-verdict.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# otp-2 — codex review adjudication

reviewer: gpt-5.6-sol (codex exec, read-only)
reviewed commit: `e757dcc`
raw output: `.review/results/otp-2.codex.md`
verdict line: NEEDS FIXES (8 findings)
fix commit: `4286c23` (8/8; matrix re-run under the fixed harness)

## F1 (High) — "symmetric baseline" mislabeled; per-direction observations only

**Adjudication: ACCEPTED (the sharpest catch).** D-2026-07-05-1's own
text — cross-direction comparisons valid only on symmetric endpoints —
already governs, and Mac↔zoey (SSD vs pool) is exactly the excluded
shape. The dataset is re-framed everywhere (README title + load-bearing
scope caveat, STATE) as the PER-DIRECTION converge-up reference; the
cross-direction half of the otp-12 bar is an owner question (symmetric
pair vs per-direction-suffices), NOT satisfied and NOT waived here.

## F2 (High) — macOS `sync` does not guarantee durable pull windows

**Adjudication: ACCEPTED.** macOS sync(2) schedules; Linux sync waits —
a real directional bias in the harness. Fixed: pull windows now fsync
every landed file (`fsync_tree`; F_FULLFSYNC deliberately not used —
the Linux side does not pay media flush either, so drive-level flush is
the equivalent depth). The cost is visible and honest: +~150 ms on the
10k-file pull cells vs the pre-review session. Matrix re-run.

## F3 (High) — STATE pre-adjudicated the owner question and advanced to otp-10

**Adjudication: ACCEPTED.** STATE no longer says "gate satisfied" or
"Current: otp-10": the Now/Queue/Blocked entries all read HOLD on the
owner adjudication (options (a) per-direction verdicts / (b) designate
a symmetric pair), with otp-10 following it. The Queue inconsistency
codex flagged (still calling otp-2 current) is fixed in the same pass.

## F4 (Medium) — drain checked quiet before remote sync; timeout silent

**Adjudication: ACCEPTED.** `drain_pool` now syncs FIRST then waits
quiet, and `drop_caches` takes the run label so a DRAIN-TIMEOUT is
recorded per run in drain.log AND warned in bench.log — the final
dataset has exactly one (the expected post-staging first run), visible
in the committed drain.log.

## F5 (Medium) — fixed push destinations; interrupted runs poison reruns

**Adjudication: ACCEPTED.** Destinations now carry a per-invocation
SESSION_TAG and the EXIT trap sweeps them — an interrupted run cannot
leave content a rerun would no-op onto. (Proved useful the same day:
the killed mid-run rerun left dirs that the trap-less path would have
kept.)

## F6 (Medium) — quantitative claims exceeded the CSV evidence

**Adjudication: ACCEPTED.** README rewritten with exact per-probe
numbers (probe-1 spread stated per cell, up to 8.0×, not "4–8×"
blanket); the manual drained probe is now committed
(`probe3-drained-pushes.csv`); pull stability stated as ±6% typical
with the worst single run +21%; push/pull ratios stated as the actual
×1.23–×2.19 range; "physically unreachable regardless of code" replaced
by the D-2026-07-05-1 validity rule (which is the owner's own recorded
ground, not an inference from old-path timings).

## F7 (Low) — median flooring unstated

**Adjudication: ACCEPTED.** Rounding policy stated in both the harness
header and the README (integer ms; even-count median = floor of the
mean of the middle two).

## F8 (Low) — non-monotonic wall time + undocumented python3

**Adjudication: ACCEPTED IN PART.** python3 is now a preflight-checked,
documented prerequisite. The monotonic half was TRIED AND REVERTED with
evidence: start/end stamps are separate processes, and cross-process
`time.monotonic()` has an undefined reference point — the attempt
produced 0/negative windows while daemon logs showed multi-second
transfers (the aborted run4). Wall clock is the correct cross-process
choice here; the harness comment records why.

## Consequence

The matrix was fully re-run under the fixed harness (same commit's
binaries): `summary.csv`/`runs.csv`/`drain.log` are from that run; the
pre-review session is kept as `probe4-prereview-session-runs.csv` for
cross-session corroboration (~10% agreement on most cells).

exec
/bin/zsh -lc "sed -n '1,115p' .review/results/otp-12b.gpt-verdict.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# otp-12b harness review — adjudication

**Reviewed commits**: `d30b1e3` (harness + doc grammar) + `772cfe6`
(quote-parity fix). **Raw review**: `.review/results/otp-12b.codex.md`
(gpt-5.6-sol, 109,042 tokens). **Verdict**: FAIL — 12 findings (4 High,
5 Medium, 3 Low); "macOS Bash 3.2 syntax and default-path quote parity
pass." All twelve verified against the script and ACCEPTED — zero
rejected.
reviewer: gpt-5.6-sol

## F1 (High) — manifest hashes inside `echo "$(…)"` again
Confirmed — a regression of the exact otp-12a F3 lesson (the zoey
script got var-captured hashes; this one didn't). Fixed: all seven
hashes captured into variables first; the Windows daemon hashes are
kept for F2's arm-swap verification.

## F2 (High) — arm swap could launch a stale exe as the wrong arm
Confirmed: `Copy-Item` failures are non-terminating and the old arm has
no handshake to catch a stale active exe. Fixed:
`$ErrorActionPreference = 'Stop'` in the launch payload and the landed
active exe's SHA256 must equal the requested arm's manifest hash before
WMI create.

## F3 (High) — both arms of a slot share the destination path
Confirmed, the sharpest catch: `rid` lacked the arm, destination sweeps
suppress errors, so the second arm could no-op onto the first arm's
leftover data and record a bogus valid time. Fixed: the arm is baked
into every rid and therefore every destination path (the zoey harness
always had this; the wrapper refactor here lost it).

## F4 (High) — derived verdicts bypassed `complete()`
Confirmed: block-2 converge rows could reference a partial block-1
median, and gap rows could mix incomplete cells. Fixed: the same-session
reference requires the block-1 counterpart complete; gap rows emit only
when all four contributing cells are complete.

## F5 (Medium) — invariance arms not doing identical work
Confirmed on both counts (nesting-shape divergence on `mw`; one arm's
container precreated outside the window while the other paid an
in-window create). Fixed: every block-2 arm gets its destination
container precreated OUTSIDE the timed window and every source is
no-trailing-slash — all four arms land the same `container/src_<w>`
tree. Block 1 keeps the otp-2w shapes verbatim.

## F6 (Medium) — `MAC_MODULE_ROOT` override could break F6-the-design-rule
Confirmed. Fixed: hardcoded to `$MAC_WORK`.

## F7 (Medium) — fail-open Windows timing
Confirmed: an errored flush read as 0 ms; pwsh noise could parse as a
plausible `ms,0`. Fixed: sentinel-framed outputs both places
(`F:<ms>:F`, `R:<ms>,<rc>:R`) with strict extraction; a flush `NA`
voids the run per the D2 rule; a client parse failure is `T_RC=99`.

## F8 (Medium) — optional references
Confirmed: block-2-only CELLS sessions silently dropped the
same-session bar; missing committed rows were silently omitted. Fixed:
committed references are mandatory (fail closed, both the F3 rows and
the F4 cross rows); an absent/incomplete block-1 counterpart emits the
registered `NO-SAME-SESSION-REF` row instead of silence.

## F9 (Medium) — untracked daemon window
Confirmed (and the WMI pid is cmd's, not the daemon's). Fixed: the cmd
pid is recorded in the launch payload itself; the verify step resolves
the daemon as the blit-daemon whose ParentProcessId is our cmd (a name
lookup tied to THIS launch); `win_daemon_stop` covers the
interrupted-between-payloads gap via the same parent-pid resolution.

## F10 (Low) — CELLS could match the CSV header
Confirmed (`CELLS=cell`). Fixed: validation greps `tail -n +2`.

## F11 (Low) — firewall rule trusted by display name
Confirmed. Fixed: an existing rule's program path, action, and enabled
state are verified; a mismatch refuses with guidance (the owner's
firewall is never silently mutated).

## F12 (Low) — vocabulary not closed; gap-row labels inexact
Confirmed. Fixed: doc vocabulary closed
(`cross-gap`/`RECORDED`/`NO-SAME-SESSION-REF` registered); gap rows
label their operands exactly (`old_push,old_pull` /
`new_mw_worst,new_wm_worst`).

## Fix commit

fix sha: `d3eae58` (`bash -n` exit 0 verified as its own step;
check-docs green; no crates/proto changes — suite stands at the
recorded 1484).

exec
/bin/zsh -lc "pwsh -NoProfile -Command '"'$rc = 0; "bare=R:123,$rc:R"; "braced=R:123,${rc}:R"'"'" in /Users/michael/Dev/blit_v2
 succeeded in 178ms:
bare=R:123,
braced=R:123,0:R

exec
/bin/zsh -lc "python3 -c 'import csv, collections
rows=list(csv.DictReader(open(\"docs/bench/otp12-win-2026-07-12/runs.csv\",newline=\"\")))
d=collections.defaultdict(dict)
for r in rows:
    d[(r[\"cell\"],r[\"run\"])][r[\"arm\"]]=int(r[\"ms\"])
for cell,arms in [(\"push_tcp_small\",(\"new\",\"old\")),(\"pull_tcp_mixed\",(\"new\",\"old\")),(\"wm_tcp_mixed\",(\"mac_init\",\"win_init\"))]:
    print(cell)
    for key in sorted(k for k in d if k[0]==cell):
        x=d[key]
        a,b=arms
        if cell.startswith(\"wm_\"):
            ratio=max(x[a],x[b])/min(x[a],x[b])
        else:
            ratio=x[a]/x[b]
        print(key[1],x[a],x[b],f\"{ratio:.3f}\",\">1.10\" if ratio>1.1 else \"<=1.10\")'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-Coa2RUPz' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-oITZNVwu' (errno=Operation not permitted)
push_tcp_small
1 2070 1849 1.120 >1.10
2 2109 1824 1.156 >1.10
3 2090 1798 1.162 >1.10
4 2031 1796 1.131 >1.10
pull_tcp_mixed
1 1142 912 1.252 >1.10
2 1134 855 1.326 >1.10
3 1186 877 1.352 >1.10
4 1127 857 1.315 >1.10
wm_tcp_mixed
1 1214 916 1.325 >1.10
2 1129 897 1.259 >1.10
3 1125 907 1.240 >1.10
4 1122 927 1.210 >1.10

exec
/bin/zsh -lc "python3 -c 'import csv
s={(r[\"cell\"],r[\"arm\"]):int(r[\"median_ms\"]) for r in csv.DictReader(open(\"docs/bench/otp12-win-2026-07-12/summary.csv\"))}
b={r[\"cell\"]:int(r[\"median_ms\"]) for r in csv.DictReader(open(\"docs/bench/otp2w-baseline-2026-07-10/summary.csv\"))}
for c in sorted(b):
    o=s[(c,\"old\")]
    print(c,o,b[c],f\"{o/b[c]:.3f}\", \"faster\" if o<b[c] else \"equal\" if o==b[c] else \"slower\")'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-g04seGJm' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-db330k4D' (errno=Operation not permitted)
pull_grpc_large 978 1289 0.759 faster
pull_grpc_mixed 1408 1408 1.000 equal
pull_grpc_small 1525 1462 1.043 slower
pull_tcp_large 964 1294 0.745 faster
pull_tcp_mixed 867 1284 0.675 faster
pull_tcp_small 1360 1280 1.062 slower
push_grpc_large 1913 3065 0.624 faster
push_grpc_mixed 2177 2687 0.810 faster
push_grpc_small 2942 2822 1.043 slower
push_tcp_large 1908 3054 0.625 faster
push_tcp_mixed 1687 2288 0.737 faster
push_tcp_small 1811 1868 0.969 faster

exec
/bin/zsh -lc "python3 -c 'import csv,collections
rows=list(csv.DictReader(open(\"docs/bench/otp12-win-2026-07-12/runs.csv\")))
d=collections.defaultdict(dict)
for r in rows:d[(r[\"cell\"],r[\"run\"])][r[\"arm\"]]=int(r[\"ms\"])
for cell in (\"wm_tcp_large\",\"pull_grpc_mixed\"):
 print(cell)
 for k in sorted(k for k in d if k[0]==cell):
  x=d[k]
  if cell.startswith(\"wm_\"):
   a,b=x[\"mac_init\"],x[\"win_init\"]; rat=max(a,b)/min(a,b)
  else:
   a,b=x[\"new\"],x[\"old\"]; rat=a/b
  print(k[1],a,b,f\"{rat:.3f}\",\"FAIL\" if rat>1.1 else \"PASS\")'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-6PbWVhOM' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-onyjIBlS' (errno=Operation not permitted)
wm_tcp_large
1 963 961 1.002 PASS
2 961 3447 3.587 FAIL
3 956 996 1.042 PASS
4 964 973 1.009 PASS
pull_grpc_mixed
1 1217 14213 0.086 PASS
2 1226 1274 0.962 PASS
3 1262 1487 0.849 PASS
4 1270 1330 0.955 PASS

exec
/bin/zsh -lc "python3 -c 'import csv,collections
rows=list(csv.DictReader(open(\"docs/bench/otp12-zoey-2026-07-12/runs.csv\")))
d=collections.defaultdict(dict)
for r in rows:
 if r[\"valid\"]==\"yes\":d[(r[\"cell\"],r[\"run\"])][r[\"arm\"]]=int(r[\"ms\"])
for cell in sorted(set(k[0] for k in d)):
 pairs=[]
 for k in sorted(k for k in d if k[0]==cell):
  x=d[k]
  pairs.append((k[1],x[\"new\"],x[\"old\"],x[\"new\"]/x[\"old\"]))
 pvals=[x[3] for x in pairs]
 strad=min(pvals)<=1.1<max(pvals)
 valsn=[x[1] for x in pairs];valso=[x[2] for x in pairs]
 spn=100*(max(valsn)-min(valsn))/min(valsn);spo=100*(max(valso)-min(valso))/min(valso)
 if strad and max(spn,spo)>25:
  print(cell,\"TRIGGER?\",f\"sp {spn:.1f}/{spo:.1f}\",\"ratios\",*[f\"{x[3]:.3f}\" for x in pairs])'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-MPN3OliA' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-9fiwDnYx' (errno=Operation not permitted)
pull_grpc_mixed TRIGGER? sp 320.2/17.1 ratios 0.953 3.314 0.919 0.923
pull_grpc_small TRIGGER? sp 299.7/3.0 ratios 0.887 0.899 3.586 0.902
pull_tcp_mixed TRIGGER? sp 248.8/25.3 ratios 0.884 0.992 0.844 3.038
pull_tcp_small TRIGGER? sp 44.2/503.7 ratios 0.201 1.173 0.948 0.757
push_tcp_large TRIGGER? sp 100.1/25.7 ratios 1.026 2.359 1.588 1.165
push_tcp_mixed TRIGGER? sp 30.5/18.4 ratios 1.147 0.967 1.100 0.983
push_tcp_small TRIGGER? sp 10.3/259.4 ratios 0.332 1.065 1.090 1.163

codex
The arithmetic and integrity checks are complete: all 192 rows are valid, every requested median/cross/gap value recomputes, and deleting the 192 CR bytes from the committed raw blob reproduces `runs.csv` byte-for-byte. One acceptance-language issue remains material; I’m doing the final syntax/provenance sanity check now.
exec
/bin/zsh -lc 'bash -n scripts/bench_otp12_win.sh' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
exec
/bin/zsh -lc 'git rev-parse HEAD' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-2Affo8dJ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-pGNGoIDy' (errno=Operation not permitted)
44c2046853d398a4e474ccc7150e884239e3308f

exec
/bin/zsh -lc 'git rev-parse 44c2046' in /Users/michael/Dev/blit_v2
exec
/bin/zsh -lc 'git diff --quiet 44c2046 -- docs/bench/otp12-win-2026-07-12/README.md docs/bench/otp12-win-2026-07-12/runs.csv docs/bench/otp12-win-2026-07-12/runs-raw-crlf.csv docs/bench/otp12-win-2026-07-12/summary.csv docs/bench/otp12-win-2026-07-12/verdicts.csv scripts/bench_otp12_win.sh' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-TZ8pJRrZ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-v2r2dLdc' (errno=Operation not permitted)
44c2046853d398a4e474ccc7150e884239e3308f

 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-XL6VrDFC' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-UGszfbs8' (errno=Operation not permitted)

codex
- [README.md:79](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/README.md:79) — **High** — Self-adjudicates `RECORDED` gap rows as platform attribution and criterion satisfaction. D2 supports attribution for an unchanged gap, but `tcp_mixed` narrows 1.946→1.408 and `grpc_small` 1.929→1.644; this conclusion belongs to the owner’s otp-13 walk.

- [README.md:76](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/README.md:76) — **Low** — The six registered Win→Mac cross rows span 0.760–0.990, not 0.71–0.99. The 0.710 value is a non-worst per-arm converge ratio.

- [bench_otp12_win.sh:385](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:385) — **Low** — Comment says 196 valid runs; the recorded session contains 192, matching the README and expected matrix size.

**VERDICT: FAIL** — arithmetic, CR-only sanitization, escalation, cross-block disclosure, sentinel fix, and code-shaped wording otherwise check out.
tokens used
130,937
- [README.md:79](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/README.md:79) — **High** — Self-adjudicates `RECORDED` gap rows as platform attribution and criterion satisfaction. D2 supports attribution for an unchanged gap, but `tcp_mixed` narrows 1.946→1.408 and `grpc_small` 1.929→1.644; this conclusion belongs to the owner’s otp-13 walk.

- [README.md:76](/Users/michael/Dev/blit_v2/docs/bench/otp12-win-2026-07-12/README.md:76) — **Low** — The six registered Win→Mac cross rows span 0.760–0.990, not 0.71–0.99. The 0.710 value is a non-worst per-arm converge ratio.

- [bench_otp12_win.sh:385](/Users/michael/Dev/blit_v2/scripts/bench_otp12_win.sh:385) — **Low** — Comment says 196 valid runs; the recorded session contains 192, matching the README and expected matrix size.

**VERDICT: FAIL** — arithmetic, CR-only sanitization, escalation, cross-block disclosure, sentinel fix, and code-shaped wording otherwise check out.
