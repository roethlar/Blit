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
session id: 019f31fc-4f78-7431-b8b2-428c98e3b790
--------
user
Review the diff of commit 06e5413 (run: git show 06e5413). It is a PLAN change (docs only, no code): docs/plan/ONE_TRANSFER_PATH.md (new Draft) + D-2026-07-05-1 in docs/DECISIONS.md + STATE/DEVLOG updates. The plan responds to an owner directive (quoted verbatim in the doc): ONE block of transfer code — direction/initiator/CLI-verb select roles, never code paths; the per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the Push/PullSync RPCs are DELETED at cutover; benchmark verdict cells must be symmetric-fs disk-to-disk. Check: internal coherence of the plan (goal/non-goals/constraints/acceptance/slices consistent); no contradiction with docs/DECISIONS.md (esp. D-2026-07-04-4 ceiling principle, D-2026-07-04-1 codex loop, D-2026-06-20-1/-2 dial contract, D-2026-06-12-1 zero-copy) or with the Active REV4 plan; whether the one-choreography design (source streams manifest, destination diffs, sf-2 shape-corrected dial as the only stream policy, fallback as byte-carrier, delegated = daemon-initiated session, local via in-process transport) has gaps or unstated risks the plan should name (e.g. resume protocol ordering, mirror delete authority, NAT/dialer asymmetry, jobs/progress integration, read-only enforcement); whether the slices are review-loop sized and correctly ordered with the tree green throughout; whether the acceptance criteria are observable and actually pin the owner's invariant (initiator/verb invariance within noise on symmetric endpoints; deletion proof; capability parity; test-count floor); and whether anything in the plan quietly re-litigates decisions the owner already settled. Output a concise markdown findings list — each finding with file/section, severity, rationale — then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll keep this as a review only and use read-only repo commands: first the target commit diff, then the relevant decision/plan sections it depends on.
exec
/usr/bin/zsh -lc 'git show --stat --decorate --find-renames --find-copies 06e5413' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 06e5413d5a0f20f4a4ff9e9a76cc38e8b89e58df (HEAD -> master)
Author: Michael Coelho <mcoelho@gmail.com>
Date:   Sun Jul 5 07:13:43 2026 -0400

    Plan: ONE_TRANSFER_PATH draft + D-2026-07-05-1 (one transfer path, pause SMALL_FILE_CEILING)
    
    Owner directive 2026-07-05, verbatim in the plan doc: one block of
    transfer code; direction/initiator/verb can never affect wall time by
    blit's doing — impossible by construction because the per-direction
    drivers and the Push/PullSync RPCs are deleted. Slices otp-1..12;
    converge-up constraint; symmetric-fs benchmark verdict cells. No code
    until the owner flips Active.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>

 DEVLOG.md                      |   2 +
 docs/DECISIONS.md              |   5 +
 docs/STATE.md                  | 159 +++++++++++++---------------
 docs/plan/ONE_TRANSFER_PATH.md | 232 +++++++++++++++++++++++++++++++++++++++++
 4 files changed, 312 insertions(+), 86 deletions(-)

codex
The commit is docs-only as advertised. I’m reading the full patch now, then I’ll cross-check the referenced decisions and active REV4 plan for contradictions and missing risk.
exec
/usr/bin/zsh -lc 'git show --find-renames --find-copies --format=fuller --stat --patch 06e5413' in /home/michael/dev/Blit
 succeeded in 0ms:
commit 06e5413d5a0f20f4a4ff9e9a76cc38e8b89e58df
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Sun Jul 5 07:13:43 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Sun Jul 5 07:13:43 2026 -0400

    Plan: ONE_TRANSFER_PATH draft + D-2026-07-05-1 (one transfer path, pause SMALL_FILE_CEILING)
    
    Owner directive 2026-07-05, verbatim in the plan doc: one block of
    transfer code; direction/initiator/verb can never affect wall time by
    blit's doing — impossible by construction because the per-direction
    drivers and the Push/PullSync RPCs are deleted. Slices otp-1..12;
    converge-up constraint; symmetric-fs benchmark verdict cells. No code
    until the owner flips Active.
    
    Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
---
 DEVLOG.md                      |   2 +
 docs/DECISIONS.md              |   5 +
 docs/STATE.md                  | 159 +++++++++++++---------------
 docs/plan/ONE_TRANSFER_PATH.md | 232 +++++++++++++++++++++++++++++++++++++++++
 4 files changed, 312 insertions(+), 86 deletions(-)

diff --git a/DEVLOG.md b/DEVLOG.md
index eff9040..273e669 100644
--- a/DEVLOG.md
+++ b/DEVLOG.md
@@ -5,6 +5,8 @@ Per R5-F5 of `docs/reviews/followup_review_2026-05-02.md`: new entries
 go at the top of the file, immediately below this header, so reviewers
 scanning chronologically don't miss appended-at-the-bottom changes.
 
+**2026-07-05 07:30:00Z** - **DIRECTIVE + PLAN (ONE_TRANSFER_PATH draft, D-2026-07-05-1, claude)**: Owner rejected the push/pull disparity, the mixed-fs benchmark methodology ("tmp on one side, spinning rust on the other is not a valid test"), and the explanation that direction symmetry lives in four separate driver loops — then issued the directive now recorded verbatim in `docs/plan/ONE_TRANSFER_PATH.md`: **"ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"**; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did"; identical whether initiated as push from skippy or pull from this machine. Scope/wire/process explicitly delegated to the agent. **Plan drafted through the plan procedure**: one `TransferSession` (roles SOURCE/DESTINATION selected by initiator/verb, never code paths), one bidi `Transfer` RPC replacing `Push`+`PullSync` (no back-compat, lockstep upgrade — repo precedent PullSyncHeader), one choreography (source streams manifest immediately; destination diffs incrementally; sf-2 shape-corrected dial as the only stream policy — absorbs the pull 1s-start residue), gRPC fallback demoted to a byte-carrier option, delegated pull = daemon-initiated session, local transfers on an in-process transport in the final phase; slices otp-1..12 ending in a **deletion slice** (the four drivers + both RPCs removed, file-by-file proof) and a **symmetric-rig acceptance matrix** (same-fs disk-to-disk verdict cells, cold caches, tmpfs as labeled wire-reference only; initiator/verb A/B within ±10%). Converge-up constraint: per cell the unified path must match the better of today's directions, not the average. **D-2026-07-05-1 recorded**: directive + SMALL_FILE_CEILING paused at sf-2 (sf-3a+ blocked; principle D-2026-07-04-4 stands) + design queue paused (w7-1 likely absorbed by otp-5). STATE rewritten around the new queue. Next: codex plan review of this commit, adjudicate, fix, then STOP for the owner's Active flip — no implementation anywhere until then.
+
 **2026-07-05 06:45:00Z** - **CODER (sf-2 shape-correction stream resize, claude)**: sf-2 landed through the codex loop (`c70c2ac` + review fix `7627e7b`). **Diagnosis correction recorded in the finding doc**: the plan said `initial_stream_proposal` was byte-weighted, but the table has had file-count tiers since ue-r2-1f — the real defect was the INPUT: the daemon proposes the push epoch-0 stream count at its early manifest flush (`FILE_LIST_EARLY_FLUSH_ENTRIES` = 128, `control.rs`), so a 10k-file push negotiated from a ~128-file prefix → 1 stream for the whole transfer (the measured 10 GbE small-cell gap AND the sf-1 probe's 1000-file/1-stream evidence). Pull checked clean (proposes from the complete post-diff need list, `pull_sync.rs:344`). **Fix, no wire change, no daemon change**: client-side shape-correction resize — as need batches accumulate, the client re-runs the shape table over the true transfer shape (need bytes + count) and corrects upward through the existing ue-r2-2 resize wire, one ADD epoch per acked roundtrip. Dial stays the single stream-policy owner (`TransferDial::propose_shape_resize`: one-in-flight, ceiling-clamped, ADD-only, no sustain/cooldown — definite signal; epoch allocation CAS'd in BOTH proposers now that two tasks allocate); tuner REMOVE permanently disables shape corrections for the transfer (live evidence outranks the static table); old daemon / gRPC fallback behavior unchanged. New observable: `RemotePushReport.data_plane_streams`. Tests +4 (suite 1479 → 1483/0): plan-cell table pins (10k×4KiB→8 via count tier; 1 GiB unchanged; mixed→8; sf-1's 1000→2), shape-resize semantics pins, and an in-process loopback e2e (REAL daemon service + REAL client, 10k tiny files) pinning `data_plane_streams > 1` — **guard proven by revert** (forced `propose_shape_resize`→None fails the pin with "settled at 1", the exact pre-fix behavior). Codex **NEEDS FIXES, 1/1 accepted**: the shape target counted `requested_files`, which `prune_unrequested_payloads` drains as payloads match — a 300-file push whose early batches predate the negotiation prunes to 0 and never corrects; now counts the append-only `files_requested` accumulator (verdict: `.review/results/sf-2-shape-correction-resize.gpt-verdict.md`). Known gaps: ramp is 1 stream per roundtrip (7 hops to 8 — fine on LAN, plan-out-of-scope on WAN); the daemon's early-flush proposal itself still lowballs (a PushHeader totals field would be wire-visible, sf-6-class gate, unnecessary while the ramp closes the gap); whether 8 streams IS the hardware ceiling is sf-4's rig question. Windows parity run not done this slice (code is platform-neutral; rides the next owner Windows run). Next: sf-3a (per-file cost limiter analysis, analysis-only).
 
 **2026-07-05 03:03:29Z** - **DECISION + CODER (D-2026-07-04-4 Active flip; sf-1 tripwire harness, claude)**: Owner "go" on the queued flip: `docs/plan/SMALL_FILE_CEILING.md` Draft → **Active** (D-2026-07-04-4, records `6ddbc68`; the four 10 GbE gate declarations were NOT part of the go and stay Blocked). **sf-1 landed through the codex loop** (`7202c1a` + review fix `80633df`): `scripts/bench_tripwires.sh` re-runs the 2026-07-05 baseline against any daemon host in one command — full matrix (blit/rsyncd/rsync-ssh/rclone-sftp × push/pull; blit/rsync/rclone/cp × local; schema-identical CSV to `docs/bench/10gbe-2026-07-05/tool_comparison.csv`), spins blitd (`--root`, module `default`) + rsyncd over ssh with full teardown, fresh never-seen targets every run, per-run `timeout`, tripwire verdict as exit code (0 clean / 3 tripped / 4 incomplete), baseline diff with ±10% flags, plus the stream-scaling probe (files/s vs streams MEASURED from blitd's per-stream completion lines, not the proposal table). Codex **NEEDS FIXES, 6/6 accepted**: teardown could rm a dir it didn't create (plain mkdir + `$$` ownership); partial coverage could read as "clean" (expected tool set enforced — INCOMPLETE cells, exit 4); `SPIN_DAEMONS=0` was broken beyond the flagged rsyncd gating — adjudication found ALL external-daemon paths diverged from the prepared targets (REL prefix + rsyncd liveness probe; verified against externally spun daemons); ±10%/missing-cell flags; two record-accuracy Lows. Verified by execution in all three modes (local-only; loopback spun; loopback external — daemons torn down, session dirs removed). Validation: fmt/clippy clean, suite 1479/0 across 37 suites (baseline held), check-docs OK. **Instrument already earning its keep**: loopback probe shows a 1000-file push riding 1 stream where the dial table (`dial.rs:429`, count≥256→2) says 2 — the sf-2 policy gap, now measurable; noted in the finding doc for sf-2. Local datapoint: `cp -a` trips blit on tiny local copies on the dev box (rig verdicts belong to sf-4). Next: sf-2 (dial file-count weighting).
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 920050c..f7e323c 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -125,3 +125,8 @@ Format:
 - Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
 - Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
 - Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).
+
+## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
+- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
+- Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
+- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands).
diff --git a/docs/STATE.md b/docs/STATE.md
index ae25057..821d2ab 100644
--- a/docs/STATE.md
+++ b/docs/STATE.md
@@ -1,12 +1,11 @@
 # STATE — single entry point for "what is true right now"
 
-Last updated: 2026-07-05 (**sf-2 landed + graded** — client-side
-shape-correction stream resize; a 10k-file push no longer rides the
-daemon's partial-manifest 1-stream proposal. sf-3a (limiter analysis,
-no code) is next. Earlier: SMALL_FILE_CEILING flipped Active
-(D-2026-07-04-4), sf-1 tripwire/scaling harness landed, 10 GbE
-benchmark session complete (wire-ceiling push/pull, ue-1 band holds),
-tool comparison measured, w9-3 landed.)
+Last updated: 2026-07-05 (**owner directive D-2026-07-05-1: ONE
+transfer path, direction-invariant by construction** — plan
+`docs/plan/ONE_TRANSFER_PATH.md` drafted, in codex review, awaiting
+the owner's Active flip. **All SMALL_FILE_CEILING work is paused**
+(sf-2 landed + graded earlier this date; sf-3a+ blocked). Earlier:
+sf-1/sf-2 landed, 10 GbE benchmark session complete, w9-3 landed.)
 **Owner pushed `master` → GitHub at `10d89e0`**; `f6e592e`..HEAD are
 local on top, unpushed — windows-latest CI check rides the next push.
 
@@ -16,28 +15,29 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
 
 ## Now (active work)
 
-- **SMALL_FILE_CEILING Active (D-2026-07-04-4) — sf-1 `[x]`, sf-2
-  `[x]`, sf-3a next** — owner correction governing all perf work:
-  FAST/SIMPLE/RELIABLE gate every change; goals are **ceiling-driven,
-  never competitor-relative** (a "beat X by N%" bar embeds a stopping
-  condition; a ≥25% margin answer was explicitly retracted — do not
-  re-litigate). Plan `docs/plan/SMALL_FILE_CEILING.md` (**Active**):
-  small-file/mixed cells to a NAMED hardware limiter, tools as
-  tripwires only; evidence durable at `docs/bench/10gbe-2026-07-05/`.
-  **sf-2 landed + graded** (`c70c2ac`+`7627e7b`, codex 1/1
-  accepted+fixed; DEVLOG 2026-07-05 06:45): the many-file stream gap
-  was the INPUT to the shape table, not the table (file-count tiers
-  since ue-r2-1f) — the daemon proposes epoch-0 streams from its
-  128-entry early manifest flush. Client now re-runs the table over
-  the accumulated need list and corrects upward via the ue-r2-2
-  resize wire (dial-owned `propose_shape_resize`; no wire/daemon
-  change; old peers unchanged). Loopback e2e pins 10k-file push > 1
-  stream, guard proven by revert; suite 1479 → 1483/0. Pull side
-  verified clean. **sf-1** (`7202c1a`+`80633df`, codex 6/6):
-  `scripts/bench_tripwires.sh` — baseline re-runnable in one command
-  + stream-scaling probe (measured streams). sf-6 keeps its own wire
-  owner gate. skippy torn down; binaries staged at `blit-bin/` for
-  sf-4.
+- **ONE_TRANSFER_PATH (D-2026-07-05-1) — Draft, codex review, then
+  owner Active flip** — owner directive 2026-07-05, verbatim in the
+  plan doc: ONE block of transfer code; direction/initiator/verb can
+  NEVER affect wall time by blit's doing, impossible by construction
+  because the per-direction drivers and the `Push`/`PullSync` RPCs
+  are deleted. One `TransferSession` (roles SOURCE/DESTINATION), one
+  `Transfer` RPC, one choreography (streaming source manifest,
+  destination diffs, sf-2 shape-corrected dial as the only stream
+  policy); gRPC fallback becomes a byte-carrier option; delegated =
+  daemon-initiated session; local rides an in-process transport.
+  Slices otp-1..12; converge-up constraint (unified path must match
+  the better direction per cell ±10%); benchmark verdict cells must
+  be symmetric-fs disk-to-disk (owner: "tmp on one side, spinning
+  rust on the other is not a valid test"), tmpfs = wire-reference
+  rows only. **No code until the owner flips Active.**
+- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1 `[x]`
+  sf-2 `[x]` (shape-correction resize, `c70c2ac`+`7627e7b`, codex 1/1,
+  suite 1479 → 1483/0, DEVLOG 2026-07-05 06:45); **sf-3a+ blocked**
+  until ONE_TRANSFER_PATH ships, then resume/re-derive on the unified
+  baseline. Its principle stands: ceiling-driven, never
+  competitor-relative (D-2026-07-04-4; a ≥25% margin answer was
+  retracted — do not re-litigate). Evidence at
+  `docs/bench/10gbe-2026-07-05/`; binaries staged at `blit-bin/`.
 - **Tool comparison measured (2026-07-05)** — blit vs rsyncd /
   rsync-ssh / rclone (sftp, webdav, no-hash fairness cells): blit
   fastest on all large/pull/local cells at the wire ceiling; rsyncd
@@ -70,51 +70,37 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
 
 ## Queue (ordered)
 
-1. **Design-review queue** — `REVIEW.md` order governs. w9-3 closed
-   `[x]` 2026-07-04 (see Now). Strict row order gives **w7-1**
-   (mirror-executor consolidation, Medium — one mirror/purge deletion
-   executor + parallel enumerate_local_manifest in blit-core, R58-F3
-   class closure; four diverged copies today, only two clear Windows
-   read-only) as the topmost ratified open row. Filed alternatives
-   (pending-review section, coder's pick): **w6-2a/-2b/-2c** (daemon
-   progress residue — independent slices, 2b→2a→2c smallest-first
-   suggestion) and Low `relay-1-subpath-double-join`.
-2. **10 GbE session ran (see Now) — owner declarations pending**:
-   ue-1 (evidence: band holds), ue-2 (no organic resize at 10 GbE —
-   interpretation call), zero-copy revisit verdict (D-2026-06-12-1;
-   evidence: wire-saturated with 0 spliced bytes), REV4 → Shipped.
-   Optional measurement follow-ups (owner-gated): Win 11 bare-metal
-   datapoint on the dual-boot client (same hardware window,
-   deployment parity, not a gate); disk-path variants (post-push
-   `zpool sync` column, cold-ARC pulls via `primarycache`);
-   sustained >ARC-size push for the pool floor. Env note: bench area
-   is now `skippy:/mnt/generic-pool/video/blit-bin/` (binaries +
-   bench.toml staged; /tmp and /home on skippy are noexec). After
-   the declarations: audit Round 1, TUI rework, H10b planner.
-3. **`docs/plan/SMALL_FILE_CEILING.md` (Active, D-2026-07-04-4 —
-   sf-1 `[x]`, sf-2 `[x]`, sf-3a per-file cost limiter analysis is
-   the current slice — analysis-only, `strace -c`/`perf` profile of
-   daemon receive + client pull-write, deliverable is a committed
-   analysis naming each per-file syscall cost; no code)**: close the
-   measured small-file/mixed gaps to the hardware ceiling. Owner
-   principle recorded in the doc (2026-07-05): goals are
-   ceiling-driven, never competitor-relative — tools like rsync are
-   tripwires, not targets. Slices sf-1..7; sf-6 (wire-visible
-   tar-shard lane) carries its own owner gate.
-4. **Post-REV4 residue** (unowned until the owner slots them): pull
-   1s-start restructuring; epoch-0/early-ADD hardening; remote
-   perf-history lanes (1e gap); `derive_local_plan_tuning`
-   fold-or-retire (statically live on the local engine path but
-   dynamically dead — nothing fills the tar/raw telemetry buckets
-   since `4ce4898`, 2026-04-07; verified during the w2-2 audit,
-   design decision not review-queue material); receive-side dial
-   tuning (rest of constants-receive-chunk-1mib-asymmetry — w3-1
-   scoped it out, wire needs no change; separate slice if wanted).
+1. **`docs/plan/ONE_TRANSFER_PATH.md` — the only work item until it
+   ships (owner directive: "do not do ANYTHING else")**: Draft
+   written 2026-07-05, codex plan review + adjudication, then STOP
+   for the owner's Active flip. After the flip: slices otp-1..12
+   through the codex loop, starting with otp-1 (wire+session
+   contract, doc+proto, no behavior).
+2. **10 GbE owner declarations (unchanged, still pending)**: ue-1,
+   ue-2, zero-copy a/b/c (D-2026-06-12-1), REV4 → Shipped. Optional
+   owner-gated measurement follow-ups (Win 11 bare-metal datapoint;
+   disk-path variants; >ARC-size push) — note the disk-path items
+   are largely absorbed by otp-11's symmetric-rig matrix. Env: bench
+   binaries staged at `skippy:/mnt/generic-pool/video/blit-bin/`
+   (/tmp and /home on skippy are noexec).
+3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
+   resumes/re-derives after ONE_TRANSFER_PATH ships.
+4. **PAUSED: design-review queue** (`REVIEW.md` order; w7-1 topmost
+   open row; filed w6-2a/b/c + relay-1) — same directive; note w7-1
+   (mirror-executor consolidation) likely lands for free inside
+   otp-5's one-delete-rule slice; re-check before picking it up.
+5. **Post-REV4 residue** (unowned): ~~pull 1s-start restructuring~~
+   (absorbed by ONE_TRANSFER_PATH choreography, D-2026-07-05-1);
+   epoch-0/early-ADD hardening; remote perf-history lanes (1e gap);
+   `derive_local_plan_tuning` fold-or-retire; receive-side dial
+   tuning residue (w3-1 scoped it out).
 
 ## Authoritative docs right now
 
-- **Active plans: `docs/plan/SMALL_FILE_CEILING.md`**
-  (D-2026-07-04-4; sf-1 current) and
+- **`docs/plan/ONE_TRANSFER_PATH.md` (Draft — governs all work; no
+  code until Active, D-2026-07-05-1)**.
+- Active plans: `docs/plan/SMALL_FILE_CEILING.md` (**paused** at
+  sf-2, D-2026-07-05-1) and
   **`docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md`** —
   code-complete; measurement gates remain (see Active context).
 - Superseded by REV4 (history only): `UNIFIED_TRANSFER_ENGINE.md` (v1),
@@ -131,6 +117,9 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
 
 ## Blocked / waiting (all owner declarations; checkpoints are owner-only)
 
+- **ONE_TRANSFER_PATH Draft → Active flip** (owner; after the codex
+  plan review is adjudicated). Until then no implementation anywhere
+  — the directive blocks all other work too.
 - **Four 10 GbE gate declarations**: ue-1 pass/fail (evidence: band
   holds), ue-2 pass/fail or re-scope (no organic resize at 10 GbE —
   sf-5 would give it a real trigger), zero-copy revisit verdict,
@@ -179,21 +168,19 @@ procedure in `docs/agent/PROTOCOL.md`; never let it describe a past session.
 
 ## Handoff log (newest first, keep ≤ 3)
 
+- **2026-07-05 (25th)** @ ONE_TRANSFER_PATH records — **owner
+  directive D-2026-07-05-1** (one transfer path,
+  direction-invariance by construction; verbatim quotes in the plan
+  doc) after the owner rejected the push/pull disparity and the
+  mixed-fs benchmark methodology. Plan drafted through the plan
+  procedure; SMALL_FILE_CEILING + design queue paused. In-flight:
+  codex plan review adjudication. **Exact first action next
+  session**: finish the plan-review adjudication if incomplete, then
+  STOP for the owner's Active flip — no implementation anywhere
+  until it lands (then otp-1: wire+session contract, doc+proto).
 - **2026-07-05 (24th)** @ `7627e7b`+records — **sf-2 landed and
   graded** (shape-correction stream resize `c70c2ac`, codex 1/1
   accepted → `7627e7b`; e2e guard proven by revert; suite
-  1479 → 1483/0; DEVLOG 2026-07-05 06:45). In-flight: none. **Exact
-  first action next session**: sf-3a (per-file cost limiter
-  analysis) through the codex loop — analysis-only, profile daemon
-  receive + client pull-write during a small transfer (`strace -c`/
-  `perf`), deliverable names each per-file syscall cost + ordered
-  candidate cuts; no code. Loopback profiling works on this box; rig
-  numbers stay sf-4's. Owner declarations (four 10 GbE gates,
-  zero-copy a/b/c, push go) remain in Blocked.
-- **2026-07-04 (23rd)** @ `80633df`+records — **Active flip + sf-1
-  landed and graded** (owner "go" → D-2026-07-04-4 at `6ddbc68`;
-  tripwire/scaling harness `7202c1a`, codex 6/6 accepted → `80633df`;
-  verified by execution in all three modes; suite 1479/0 held).
-  In-flight: none. Owner declarations
-  (four 10 GbE gates, zero-copy a/b/c, push go) remain in Blocked.
-- (older entries pruned — see DEVLOG 2026-07-05 02:07 and earlier)
+  1479 → 1483/0; DEVLOG 2026-07-05 06:45). In-flight: none.
+  (Its "next: sf-3a" is superseded by the 25th entry above.)
+- (older entries pruned — see DEVLOG 2026-07-05 03:03 and earlier)
diff --git a/docs/plan/ONE_TRANSFER_PATH.md b/docs/plan/ONE_TRANSFER_PATH.md
new file mode 100644
index 0000000..dc3d1c6
--- /dev/null
+++ b/docs/plan/ONE_TRANSFER_PATH.md
@@ -0,0 +1,232 @@
+# One transfer path — direction-invariant by construction
+
+**Status**: Draft
+**Created**: 2026-07-05
+**Supersedes**: post-REV4 residue item "pull 1s-start restructuring"
+(absorbed here); pauses `docs/plan/SMALL_FILE_CEILING.md` after sf-2
+(D-2026-07-05-1)
+**Decision ref**: D-2026-07-05-1 (directive + pause); Active flip gets
+its own entry
+
+## Directive (owner, 2026-07-05, verbatim)
+
+> "make ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF
+> ANYTHING EVER using anything else because anything else does not
+> exist."
+
+> "just make it so that I NEVER see a situation where pull is faster
+> than push or vice versa. that CAN NEVER be possible because of
+> something blit did. it should be identical if I start the transfer
+> from skippy and push to this machine or if I start the transfer on
+> this machine and pull from skippy."
+
+> On benchmark methodology: "tmp on one side, spinning rust on the
+> other is not a valid test."
+
+Scope, wire, and process were explicitly delegated to the agent
+("no idea. you architected this"; "I DO NOT CARE. FIX IT."). The
+owner's requirement is the invariant; everything below is the
+architecture that makes the invariant impossible to violate rather
+than merely maintained by discipline.
+
+## Goal
+
+One `TransferSession` implementation owns every byte transfer blit
+performs. A transfer has a SOURCE role and a DESTINATION role; which
+end initiated, and which CLI verb was used, select roles — they do not
+select code. When this plan ships, the per-direction drivers (client
+push driver, daemon push-receive, client pull driver, daemon
+pull-send, delegated-pull driver, local orchestration) **do not
+exist**: for fixed endpoints and dataset, direction/initiator/verb
+cannot affect behavior or wall time by blit's doing, because there is
+no second code path to differ.
+
+## Non-goals
+
+- Preserving wire compatibility with pre-plan builds. The `Push` and
+  `PullSync` RPCs are deleted at cutover; both ends upgrade in
+  lockstep (repo precedent: the `PullSyncHeader` removal; the owner
+  operates every deployed peer).
+- Making different hardware perform identically. If src and dst sit
+  on different disks, the two *data directions* still differ by
+  physics; the invariant is that the same data direction between the
+  same endpoints is identical regardless of who initiates and which
+  verb is used.
+- WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
+- New features. This is a consolidation; capability parity with
+  today (mirror, filters, resume, fallback, delegation, progress,
+  jobs, cancellation) is the bar.
+
+## Constraints
+
+- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
+  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
+  violated at the choreography layer.
+- **Converge up, not down**: per benchmark cell, the unified session
+  must match the better of today's two directions (within ±10% run
+  noise), not their average. Unification that slows the fast
+  direction fails review.
+- REV4 invariants carry: byte-identical results, StallGuard,
+  cancellation, byte-accounting. Existing pins are ported (not
+  dropped) as tests become role-parameterized; test count never
+  drops.
+- The sf-2 shape-correction behavior (stream count corrects as the
+  need list accumulates) becomes the one and only stream policy —
+  both directions inherit it by construction; its pins carry over.
+- Wire contract discipline (REV4 rule): the unified session's proto —
+  messages, field numbers, capability negotiation, transport
+  selection — is a reviewed doc+proto slice **before** any behavior
+  depends on it.
+- Every slice through the codex loop (D-2026-07-04-1); tree green
+  after every slice; transitional coexistence of old+new paths is
+  scaffolding only — the plan is not Shipped until the deletion slice
+  lands and the deletion proof is recorded.
+- Windows parity: suite green on the owner's machine + windows-latest
+  CI before Shipped.
+
+## Acceptance criteria
+
+- [ ] **Initiator/verb invariance (the owner's sentence, measured)**:
+      on a symmetric rig (same filesystem class both ends, cold
+      caches, disk-to-disk), for each data direction and workload
+      (large / 10k-small / mixed): wall time initiating from end A vs
+      end B, and via push-verb vs pull-verb, differs only within
+      run-to-run noise (±10%). Matrix committed as evidence.
+- [ ] **Deletion proof**: `remote/pull.rs` (driver), `remote/push/`
+      (driver), daemon `push/control.rs` choreography, daemon
+      `pull_sync.rs` choreography, the delegated-pull driver, the
+      separate local orchestration path, and the `Push`/`PullSync`
+      RPCs no longer exist in the tree; one `TransferSession` and one
+      `Transfer` RPC remain. Recorded file-by-file in the final
+      slice's finding doc.
+- [ ] Capability parity: mirror (both mirror-kinds + scan-complete
+      guard), filters, block-resume, gRPC fallback carrier, delegated
+      transfer, progress events, jobs/cancel, read-only enforcement —
+      each demonstrated by ported tests on the session.
+- [ ] Suite green throughout; final test count ≥ pre-plan baseline
+      (1483); all REV4 invariant pins and the sf-2 pin pass
+      role-parameterized.
+- [ ] Benchmark methodology corrected and recorded: symmetric-fs
+      cells are the verdict cells; tmpfs cells remain only as
+      explicitly-labeled wire-reference rows (never compared across
+      directions with asymmetric endpoints).
+- [ ] Windows: full suite green (owner machine) + windows-latest CI.
+
+## Design
+
+**What already is one code** (kept, becomes the session's engine):
+`remote/transfer/` — pipeline, sink/source abstractions, data plane,
+diff planner, tar-shard, stall guard, progress, `operation_spec` (the
+REV4 unified contract), and the engine dial (stream policy incl. sf-2
+shape correction). The defect layer is above it: four driver loops
+choreograph these pieces differently per direction.
+
+**The one choreography** (roles, not directions):
+
+1. Initiator opens the single bidi `Transfer` RPC and sends the
+   operation spec: which end is SOURCE, which is DESTINATION, path/
+   module, filters, mirror/resume flags, capabilities.
+2. SOURCE enumerates and **streams** its manifest immediately (no
+   buffered-enumeration phase — this generalizes push's fast start;
+   pull's full-enumeration-then-negotiate slow start is deleted, which
+   absorbs the "pull 1s-start" residue item).
+3. DESTINATION diffs incrementally against its own filesystem and
+   returns need-list batches (one diff owner, always the end that
+   owns the target fs — push's proven model; pull_sync's
+   source-side diff is deleted).
+4. The data plane opens at the dial floor immediately; stream count
+   shape-corrects as the need list accumulates (sf-2 mechanism, now
+   the only policy, both roles).
+5. SOURCE feeds payloads (files / tar-shards / resume blocks) through
+   the one pipeline into the data plane; DESTINATION writes through
+   the one receive path.
+6. Mirror: DESTINATION computes deletions from the completed source
+   manifest it received (filter-scoped, scan-complete-guarded) and
+   executes them locally. One rule, no per-direction delete
+   choreography.
+7. Resume: optional block-hash phase inside the same session, same
+   messages regardless of roles.
+8. Summary/byte-accounting: one record shape.
+
+**Transport facts vs choreography**: the connection-initiating end
+dials TCP data-plane sockets (NAT reality) — byte direction within a
+socket is set by role, not by who dialed. The gRPC-fallback lane
+becomes a *byte-carrier option* inside the same session (control-
+stream frames instead of TCP sockets), selected at negotiation — not
+a separate transfer path. Resize keeps its controller-at-sender rule.
+
+**Delegated transfer**: a daemon receiving a delegated request simply
+becomes an initiator of the same session against the other daemon
+(destination role on its module fs). The bespoke delegated-pull
+driver is deleted; the delegation *gate* (authorization) stays.
+
+**Local transfers**: the same session driver over an in-process
+transport (both roles in one process, no wire). The engine underneath
+is already shared; the separate local orchestration path is deleted
+in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
+guard the migration.
+
+**Affected crates**: `blit-core` (new `transfer_session` module;
+`remote/pull.rs`, `remote/push/` drivers deleted at cutover),
+`blit-daemon` (one `Transfer` handler replaces push/pull_sync/
+delegated handlers), `blit-cli`/`blit-app` (verbs map to roles),
+`proto/blit.proto` (one `Transfer` RPC; `Push`/`PullSync` deleted),
+`blit-tui` (progress/jobs consume the same events).
+
+**Risks**: largest consolidation since REV1 — pull.rs alone is ~108K;
+mitigated by strangler slices with the tree green throughout and a
+non-optional deletion slice. Per-cell regression risk on today's
+faster direction — mitigated by the converge-up constraint and
+baseline parity pins per slice. Wire break — lockstep upgrade,
+owner-controlled fleet. Windows receive paths (win_fs) — parity gate.
+Progress/jobs/TUI integration churn — the session emits the existing
+event contract (w6-1) at the same boundaries.
+
+## Slices
+
+One coherent, testable change per slice — sized for the `.review/`
+loop. Tree green after every slice; old paths keep working until
+otp-9 deletes them.
+
+1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
+   `Transfer` RPC and message set — roles, phases, field numbers,
+   capability negotiation, transport selection, resume/mirror
+   phases, error/cancel semantics. Full REV4 wire-contract
+   deliverable set; codex-reviewed before any code consumes it.
+2. **otp-2 TransferSession core (blit-core)**: role-parameterized
+   state machine over the existing engine with an in-process
+   transport; unit/e2e tests run BOTH role assignments over the same
+   fixtures — the invariance property enters the test suite here.
+3. **otp-3 daemon serves `Transfer`, client initiates as SOURCE**
+   (remote push-equivalent rides the session); A/B parity pins vs
+   old push (byte-identical trees, summary parity, sf-2 pin ported).
+4. **otp-4 roles swapped: client initiates as DESTINATION** (pull-
+   equivalent) — the same code with roles flipped; the parity suite
+   reruns with no per-direction test code.
+5. **otp-5 mirror + filters** on the session (one delete rule).
+6. **otp-6 resume** block phase.
+7. **otp-7 fallback byte-carrier** (control-stream frames) as the
+   session's alternate transport.
+8. **otp-8 delegated transfer** = daemon-initiated session; bespoke
+   delegated-pull driver retired behind the existing gate.
+9. **otp-9 cutover + deletion**: CLI/app/TUI route every remote
+   operation through the session; `Push`/`PullSync` and all four
+   drivers deleted from the tree and the proto; ported-test
+   accounting proves count never dropped. Deletion proof recorded.
+10. **otp-10 local transfers** ride the in-process transport; the
+    separate local orchestration is deleted; local perf pins hold.
+11. **otp-11 symmetric-rig acceptance run**: sf-1 harness matrix
+    corrected (same-fs disk-to-disk verdict cells, cold caches,
+    tmpfs as labeled wire-reference only) + the initiator/verb
+    invariance A/B matrix; committed as this plan's acceptance
+    evidence.
+12. **otp-12 verdict**: acceptance checklist walked with the owner;
+    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
+    against the unified baseline — owner call at that point.
+
+## Open questions
+
+- None requiring owner input now — scope, wire, and process were
+  delegated (Directive section). Slice-level unknowns (exact proto
+  shapes, resume edge semantics, TUI event wiring) are settled inside
+  their slices through the codex loop. — owner

codex
The new plan is internally readable, but a couple of claims hinge on older durable decisions. I’m checking those exact entries now rather than relying on the summary in the new text.
exec
/usr/bin/zsh -lc 'rg -n "D-2026-07-04-4|D-2026-07-04-1|D-2026-06-20-[12]|D-2026-06-12-1" docs/DECISIONS.md' in /home/michael/dev/Blit
 succeeded in 0ms:
72:## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
77:## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
78:- Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) ~~and **size-gated** (small transfers skip the probe entirely)~~ **(size-gate framing superseded by D-2026-06-20-2 q1 — there is no probe phase to skip; the engine moves within ~1s and tunes live)**. The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. ~~Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit).~~ **(A/warmup staging superseded by D-2026-06-20-2 q1 — conservative start + live tuning from the first byte; C shipped as `ue-r2-2` under REV4/D-2026-06-20-5.)** Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip). *(Stale wording struck 2026-07-04 on owner direction — "follow the existing pattern": the in-place-annotation pattern of D-2026-06-20-3/-6. The convergence direction itself stands unchanged.)*
83:## D-2026-06-20-2 — UNIFIED_TRANSFER_ENGINE.md flipped Draft → Active; four bound parameters
90:- Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
92:- Supersedes: nothing. Reverts the conditional H10b supersession that D-2026-06-20-2 had proposed (that entry is edited in-place to drop the inference and point here).
97:- Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.
102:- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*
107:- Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.
109:## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
124:## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
132:- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands).

exec
/usr/bin/zsh -lc 'rg -n "Status|sf-|ceiling|competitor|FAST|SIMPLE|RELIABLE|zero-copy|wire|Active|fallback|symmetric|benchmark|tripwire" docs/plan/SMALL_FILE_CEILING.md' in /home/michael/dev/Blit
 succeeded in 0ms:
1:# Small-file transfer to the hardware ceiling
3:**Status**: Active
6:**Decision ref**: D-2026-07-04-4 (Draft → Active, owner "go")
10:blit's guiding principles are **FAST, SIMPLE, RELIABLE** — every
13:therefore **ceiling-driven, never competitor-relative**: a
16:**tripwires** — any scenario where any tool measures faster than
17:blit is, by definition, proof blit is off its hardware ceiling and
23:measured blit off its ceiling — many-tiny-file and mixed transfers —
24:blit's wall time becomes bounded by a **named hardware limit** (wire,
34:| cell | blit today | ceiling arithmetic | tripwire |
36:| push 10k×4 KiB | 2.4–3.3 s | wire: **34 ms** (40 MiB @ 9.9 Gbit); fs floor: ~150 µs/file proven single-pipe on this ZFS, ÷ parallelism → **~0.2–0.5 s** | rsyncd 1.5 s |
37:| pull 10k×4 KiB | 446–484 ms | client fs = tmpfs (µs creates); wire+protocol class: **≪ 200 ms** | rsyncd 367 ms |
38:| push mixed 512 MiB+5k | 1.8–2.2 s | big file alone: ~450 ms wire; small remainder as above | rsyncd 1.24 s |
52:  ceiling analysis when a rig exists).
53:- Non-Linux rig ceiling targets (no measurement hardware this plan
56:  tripwires; blit's transport security model is unchanged by this
61:- Every slice serves FAST without violating SIMPLE (dial stays the
63:  survive past their measured need) or RELIABLE (REV4 invariants:
65:- No wire-visible protocol change without a dedicated owner gate on
66:  the wire design before code (sf-6); mixed-version peers keep
85:- [ ] **Tripwires clean**: no tool in the committed sf-1 harness
90:      a tripwire tool means adding it to the harness.)
99:to find the ceiling with the least machinery, not to stop early:
116:4. **Tar-shard push lane** (wire-visible, own owner gate): bundle
117:   tiny files into shard frames on the push wire as the local
125:green; lever 4 adds wire complexity — SIMPLE requires the limiter
130:1. **sf-1 tripwire harness**: commit `scripts/bench_tripwires.sh`
136:   (committed with this plan); sf-1 makes it re-runnable in one
138:2. **sf-2 dial file-count weighting**: proposal-table unit pins
142:3. **sf-3a per-file cost limiter analysis** (analysis-only, w8-1b
148:4. **sf-3b… one cut per slice**: each accepted cut from sf-3a lands
151:   without the rig). The count of sf-3x slices is set by sf-3a's
153:5. **sf-4 rig re-measure + limiter analysis**: rerun sf-1 harness on
155:   bound everywhere + tripwires clean → acceptance review with the
157:6. **sf-5 resize-on-backlog feed** (if sf-4 names stream count
159:   trigger regardless — flagged at sf-4).
160:7. **sf-6 tar-shard push lane** (if sf-4/sf-5's analysis names
161:   per-file wire framing as the binder). Wire-visible; the owner
162:   gate consumes the full REV4 wire-contract deliverable set
168:8. **sf-7 verdict**: final rig run, limiter analyses committed,
174:- **sf-6 wire gate** (standing): the tar-shard lane's wire design
176:  so no session treats sf-6 as pre-authorized code. — owner

exec
/usr/bin/zsh -lc 'rg -n "Status|Shipped|Active|measurement|gate|D-2026|Push|PullSync|zero-copy|dial|fallback|delegat|local|mirror|resume|progress|jobs|read-only" docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md' in /home/michael/dev/Blit
 succeeded in 0ms:
1:# Unified Transfer Engine REV4 — the Active convergence plan (code-reality corrected)
3:**Status**: Active
5:**Activated**: 2026-06-20 (owner: "rev4 replaces v1" — D-2026-06-20-5).
8:(D-2026-06-20-4) is lifted as to the **plan decision**; per AGENTS.md §9
15:pull single-stream claim re-scoped to PullSync, the local-fast-path
22:  question (resolved as *convergence*, not rebuild — D-2026-06-20-1).
25:  shared byte-moving leaf but never converged the sequencer+dials layer.
30:  D-2026-06-04-3 (owner vetoed that merger — D-2026-06-20-3).
31:**Decision refs**: D-2026-06-20-1 (convergence direction),
32:D-2026-06-20-2 (bound parameters), D-2026-06-20-3 (H10b veto),
33:D-2026-06-20-4 (plan-review freeze).
38:src/dst-agnostic engine, one live dial, and the already-shared
51:- Existing local fast paths are preserved as engine-owned strategies
58:- Pull parity is measured only after PullSync is actually multistream.
65:- No zero-copy receive revival (D-2026-06-12-1; revisit gated on the
68:  requirement stand on their own; D-2026-06-04-3 remains queued after
70:- The **gRPC fallback path stays single-logical-stream by design**
72:  is about PullSync's TCP data plane, not this fallback.
79:  local transfers keep their low-overhead path.
81:  push negotiation, pull sync, and delegated transfers become strategies
83:- **RELIABLE**: mirror/delete safety, scan-completeness checks, resume,
84:  StallGuard behavior, cancellation, byte-progress accounting, and
86:- Wire changes are allowed (proto unfrozen, D-2026-06-11-1), but mixed
98:      public transfer sequencer for local↔local, push, pull, and
99:      delegated daemon↔daemon transfers.
100:- [ ] Existing local fast paths are either engine-owned strategies
104:      local path bypasses the transfer behavior owner by accident.
105:- [ ] **The three static code-level stream/dial ladders plus the
106:      negotiated proto field are replaced by one dial source** (corrected
107:      against code — see Current Code Reality). Concretely, the dial
130:      unsafe — mirror/delete (scan-completeness), resume, and
139:- [ ] The sender owns the dial within the receiver's advertised rich
148:      the dial is a live mutable object read by both ends from the
149:      live-dials slice onward, and the stream-set is elastic
152:      the resize proto onto this; it does not restructure the dial or the
156:- [ ] **Pull is not counted in the loopback parity band until PullSync is
157:      actually multistream through the unified engine.** (PullSync is the
160:- [ ] Deprecated `Pull` RPC deletion waits until its multistream/fallback
161:      pattern has been harvested into PullSync and compatibility/fallback
163:- [ ] **Loopback parity band:** once pull is multistream, local↔local,
164:      local→daemon, and daemon→local all measure within a tight band on
184:- Push already streams manifest/need-list work and feeds the shared sink
196:- **Local copy** still owns a local-shaped `TransferOrchestrator`
199:  takes `LocalMirrorOptions` (:134), runs local-only fast paths
203:  its parallelism comes from this runtime + worker model, so the dial
204:  must subsume the local parallelism source as well as the remote
226:  **PullSync** — the current path — is **single-stream today**:
229:  Pull's pattern into PullSync through the engine; it is **not** adding
246:`TransferOrchestrator` in place (it is local-shaped: own runtime,
247:`LocalMirrorOptions`). `TransferOrchestrator` becomes the local adapter
248:that constructs local `Source`, `Sink`, options, and local strategy
249:inputs, then calls the engine. Push, PullSync, and delegated transfers
257:- dial creation and updates (subsuming all three ladders + local
260:- progress and telemetry wiring;
261:- invariants around first work, fallback, cancellation, and finish.
265:manifest exchange, mirror/delete authority, and legacy compatibility.
268:agent recommends the new `TransferEngine` + local adapter above, not an
275:rule: the small/huge/no-op local paths may remain, but only as
289:slice. Today the local path collects all headers before planning, and
290:PullSync waits for complete client and server manifests before transfer.
298:- preserve mirror/delete and scan-completeness reliability;
299:- measure first byte and first useful progress per transfer mode.
315:class is only the unsafe-before-full-knowledge modes (mirror/delete,
316:resume, checksum-refusal) named in the Acceptance Criteria.
320:The byte sender owns the live dial, bounded by the byte receiver's
328:- delegated: destination daemon advertises its receiver capacity when it
337:The initial dial starts **below** the profile ceiling with margin.
338:Telemetry can increase or decrease the cheap dials (chunk, prefetch, TCP
340:slice. The dial is a **mutable object read by both ends from the
341:live-dials slice onward** — this is what makes continuous (`ue-r2-2`) a
352:  the client, especially PullSync and delegated pull;
370:- one sink failure propagates the real error and shuts down cleanly;
377:Loopback parity is only meaningful after local↔local, local→daemon, and
378:daemon→local all use comparable engine paths. PullSync is single-stream
379:today (`pull_sync.rs:568`), so REV4 moves the parity-band gate after pull
406:  and resize messages are gated on advertised peer capability bits.
435:2. **`ue-r2-1b-wire-dial-contract`** — Define capacity profile, peer
440:3. **`ue-r2-1c-engine-shell-local-adapter`** — Add `TransferEngine` and
441:   convert `TransferOrchestrator` into a local adapter. Move the local
448:   first-byte / first-useful-work timing for local and push shapes, and
451:5. **`ue-r2-1e-live-cheap-dials`** — Replace the static
453:   single mutable dial. Start conservative within the receiver profile,
454:   then adjust cheap dials from PR1 telemetry. The dial is a mutable
457:   preserving manifest streaming, need-list batching, fallback timing,
459:   the daemon `desired_streams` ladder** into the dial (this is the
461:7. **`ue-r2-1g-pull-multistream-converge`** — Route PullSync through the
464:   resume, checksum refusal, delete-list authority, cancellation,
465:   per-stream failure, and gRPC fallback behavior. Absorbs
468:   `Pull` RPC (and its `pull_stream_count` ladder) after PullSync has
469:   harvested the needed multistream/fallback pattern and tests cover the
474:   `ue-r2-1a`. Wires onto the already-mutable dial and elastic
483:- `1b` (wire contract) blocks `1e` (dial uses the capacity profile), `2`
484:  (resize proto), and contributes the compat tests `1h` gates on.
486:  dial, push, and pull all run inside the engine.
502:- `ue-1c` was too large: split streaming-plan foundation, local adapter,
512:- Pull parity gate was too early: moved until after PullSync multistream.
518:  `determine_remote_tuning` drives "local and push"; it drives push +
519:  daemon pull, **not** local (local has no caller). And REV3's
521:  the deprecated multistream `Pull` RPC with single-stream PullSync.
525:- `DECISIONS.md` D-2026-06-20-1 still carries superseded warmup/size-gate
526:  language; D-2026-06-20-2 and this Draft carry the live-from-first-byte
527:  correction. If REV4 is accepted, either edit D-2026-06-20-1 with a note
534:  (mirror/delete, resume, checksum-refusal). Novel vs known workload is a
538:  in-plan as `ue-r2-1h`, sequenced last, gated on `ue-r2-1g` + the
540:- **(RESOLVED 2026-06-20, owner — D-2026-06-20-5)** REV4 **replaces**
543:  header, so nothing is lost. One Active plan; no candidates left live.
544:- Should D-2026-06-20-1 be edited now to remove the superseded
545:  warmup/size-gated wording, or should the later decisions remain the

exec
/usr/bin/zsh -lc "sed -n '72,132p' docs/DECISIONS.md" in /home/michael/dev/Blit
 succeeded in 0ms:
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
- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands).

exec
/usr/bin/zsh -lc "sed -n '1,120p' docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Unified Transfer Engine REV4 — the Active convergence plan (code-reality corrected)

**Status**: Active
**Created**: 2026-06-20
**Activated**: 2026-06-20 (owner: "rev4 replaces v1" — D-2026-06-20-5).
Replaces `UNIFIED_TRANSFER_ENGINE.md` (v1, now Superseded) and the review
candidates REV2/REV3 (now Superseded-by-REV4). The plan-review freeze
(D-2026-06-20-4) is lifted as to the **plan decision**; per AGENTS.md §9
no code slice starts without a fresh per-slice owner authorization.
**Based on**: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV3.md` plus a
2026-06-20 re-verification of every code claim against the tree at
`HEAD` (`09268eb`). REV4 = REV3 with its "Current Code Reality" and the
stream-ladder acceptance criterion **corrected against the actual code**
(REV3's "two static tables, not three" correction was itself wrong), the
pull single-stream claim re-scoped to PullSync, the local-fast-path
strategy names fixed to their real values, every named symbol grounded
with `file:line`, and v1's lineage/absorption header carried forward so
this doc can stand as plan-of-record if the owner flips it.
**Lineage** (carried forward from `UNIFIED_TRANSFER_ENGINE.md` so
supersession is not lost if REV4 replaces v1):
- Supersedes the "ground-up redesign" framing of the 2026-06-14 open
  question (resolved as *convergence*, not rebuild — D-2026-06-20-1).
- Continues the lineage of `PIPELINE_UNIFICATION.md` and
  `UNIFIED_RECEIVE_PIPELINE.md` (both **Historical**): they landed the
  shared byte-moving leaf but never converged the sequencer+dials layer.
- **Absorbs** `MULTISTREAM_PULL.md` (w2-3) as slice `ue-r2-1g` (goal
  survives; the path-specific premise does not).
- The engine's workload-shape-aware planner and 1s-start commitment are
  **not** the H10b streaming-planner concept and do **not** supersede
  D-2026-06-04-3 (owner vetoed that merger — D-2026-06-20-3).
**Decision refs**: D-2026-06-20-1 (convergence direction),
D-2026-06-20-2 (bound parameters), D-2026-06-20-3 (H10b veto),
D-2026-06-20-4 (plan-review freeze).

## Goal

Keep the v1 direction: converge the transfer subsystem around one
src/dst-agnostic engine, one live dial, and the already-shared
byte-moving leaf. The operator should experience one simple transfer
model regardless of where the command is issued. The implementation may
be complex underneath, but the behavioral contract must be FAST, SIMPLE,
and RELIABLE.

REV4 keeps convergence, not rebuild. It tightens the plan where review
found that v1 compressed too much into one slice or left compatibility
implicit, and it corrects the code-reality errors that crept into REV2/
REV3:

- The first-byte-within-about-1s requirement is a real architecture
  change and gets its own streaming-plan slice.
- Existing local fast paths are preserved as engine-owned strategies
  unless the owner later decides to delete one; they must not remain
  side doors around the engine.
- Work-stealing is treated as a scheduling behavior change, not as
  "substrate only".
- Capacity profile and resize wire shape are designed before code that
  depends on them.
- Pull parity is measured only after PullSync is actually multistream.
- The stream-count ladders the engine must subsume are enumerated
  **accurately** (REV3 under-counted them — see Current Code Reality).

## Non-goals

- No ground-up transfer rewrite.
- No zero-copy receive revival (D-2026-06-12-1; revisit gated on the
  10 GbE benchmarks).
- No H10b merger. The engine's workload-shape-aware planner and 1s start
  requirement stand on their own; D-2026-06-04-3 remains queued after
  audit Round 1.
- The **gRPC fallback path stays single-logical-stream by design**
  (unchanged from w2-3's non-goal). "Pull is single-stream today" below
  is about PullSync's TCP data plane, not this fallback.
- No coding during this review.

## Constraints

- **FAST**: bytes begin quickly, stream scheduling avoids slow-sink
  head-of-line blocking, tuning comes from measured telemetry, and small
  local transfers keep their low-overhead path.
- **SIMPLE**: there is one transfer behavior owner. Local fast paths,
  push negotiation, pull sync, and delegated transfers become strategies
  or inputs under the engine, not separate operator-visible models.
- **RELIABLE**: mirror/delete safety, scan-completeness checks, resume,
  StallGuard behavior, cancellation, byte-progress accounting, and
  byte-identical transfer tests cannot regress.
- Wire changes are allowed (proto unfrozen, D-2026-06-11-1), but mixed
  old/new peers must negotiate down to today's behavior. New fields are
  advisory until both peers advertise support.
- The 1370-test baseline must not drop.
- Windows parity remains required unless a test is genuinely platform
  specific.
- **Every stage serves FAST, SIMPLE, or RELIABLE** — a purely structural
  change with no goal payoff is out.

## Acceptance Criteria

- [ ] A single `TransferEngine` (or owner-approved equivalent) is the
      public transfer sequencer for local↔local, push, pull, and
      delegated daemon↔daemon transfers.
- [ ] Existing local fast paths are either engine-owned strategies
      (their real names: `journal_no_work`, `no_work`, `tiny_manifest`,
      `single_huge_file`, and the single-file copy shortcut at
      `orchestrator.rs:178`) or explicitly deleted by owner decision. No
      local path bypasses the transfer behavior owner by accident.
- [ ] **The three static code-level stream/dial ladders plus the
      negotiated proto field are replaced by one dial source** (corrected
      against code — see Current Code Reality). Concretely, the dial
      subsumes:
      1. `remote/tuning.rs::determine_remote_tuning` (size-keyed
         `initial_streams`/`max_streams`/`chunk_bytes`/`tcp_buffer_size`/
         `prefetch_count`; the *client's* ladder, consumed by push and by
         the daemon pull paths);
      2. `blit-daemon .../push/control.rs::desired_streams` (the daemon
         push-negotiation ladder, already keyed on **file count** as well
         as bytes — the daemon's ladder "wins" per `tuning.rs`'s own doc
         comment);
      3. `blit-daemon .../pull.rs::pull_stream_count` (the deprecated
         Pull RPC ladder, byte-keyed, capped by `tuning.max_streams`);
      and the negotiated `DataTransferNegotiation.stream_count` (proto
      field 4) those ladders feed onto the wire. After convergence no

exec
/usr/bin/zsh -lc "sed -n '120,220p' docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md" in /home/michael/dev/Blit
 succeeded in 0ms:
      field 4) those ladders feed onto the wire. After convergence no
      static size→streams table remains in any path.
- [ ] The engine starts transfer work within about 1 second without a
      probe-then-go phase. This holds for **both** novel workloads (no
      telemetry extant — start copying immediately at conservative
      defaults and tune live from the first byte) **and** known workloads
      (telemetry extant — replay the last run if it was optimal, else
      recalculate onto the live-tune path). Novel vs known is a
      tuning-strategy choice, not an exception. The only exceptions are
      modes where moving any byte before full knowledge would itself be
      unsafe — mirror/delete (scan-completeness), resume, and
      checksum-refusal — and those are explicit, tested, and reported to
      the owner instead of silently weakening RELIABLE.
- [ ] The planner is workload-shape-aware (file count and per-file
      overhead, not bytes alone) and can emit an initial safe work batch
      from partial enumeration, then refine as more headers arrive. Note
      `desired_streams` already carries partial shape-awareness today
      (it branches on `file_count`); the engine generalizes this rather
      than inventing it from nothing.
- [ ] The sender owns the dial within the receiver's advertised rich
      capacity profile. The weak end protects itself in both directions
      (strong→weak and weak→strong).
- [ ] The wire contract names the capacity-profile and stream-resize
      fields/messages, their field numbers, and the mixed-version
      behavior before code lands. (Grounded: `DataTransferNegotiation`
      uses fields 1–4 today and reserves 5–10 for RDMA, so
      `CapacityProfile receiver_capacity = 11` is the first free number.)
- [ ] **C-ready by construction, not by retrofit** (restored from v1):
      the dial is a live mutable object read by both ends from the
      live-dials slice onward, and the stream-set is elastic
      (work-stealing, work not pinned to a stream) from the salvage slice
      onward. Continuous mid-transfer stream add/drop (`ue-r2-2`) wires
      the resize proto onto this; it does not restructure the dial or the
      stream-set.
- [ ] Work-stealing is validated as behavior: slow sink, failing sink,
      cancellation, byte accounting, and StallGuard tests stay green.
- [ ] **Pull is not counted in the loopback parity band until PullSync is
      actually multistream through the unified engine.** (PullSync is the
      single-stream path today; the deprecated `Pull` RPC is already
      multistream — see Current Code Reality.)
- [ ] Deprecated `Pull` RPC deletion waits until its multistream/fallback
      pattern has been harvested into PullSync and compatibility/fallback
      tests cover old/new peer pairs.
- [ ] **Loopback parity band:** once pull is multistream, local↔local,
      local→daemon, and daemon→local all measure within a tight band on
      the same hardware (the one-engine property, measured) — no 10×/2×
      gap.
- [ ] The 10 GbE benchmark (`BENCHMARK_10GBE_PLAN.md`) remains the
      sign-off measure for final parity and stream resize, not a
      prerequisite to start the owner-approved coding slices.

## Current Code Reality

*(All references re-verified against `HEAD` `09268eb` on 2026-06-20.)*

The existing code already has useful convergence substrate:

- `TransferSource` (`remote/transfer/source.rs:16`) and `TransferSink`
  (`remote/transfer/sink.rs:44`) define the source/sink seam.
- `execute_sink_pipeline_streaming` (`remote/transfer/pipeline.rs:70`)
  and `execute_receive_pipeline` (`pipeline.rs:200`) are the shared
  byte-moving leaves; `plan_transfer_payloads`
  (`remote/transfer/payload.rs:115`, aliased `plan_push_payloads` on the
  push side) is the payload planner.
- Push already streams manifest/need-list work and feeds the shared sink
  pipeline as work arrives.
- The planner already accounts for workload shape in part: file size
  classes, file count, tar shards, raw bundles, and large-file tasks.
- Cross-run history exists in-tree: `perf_history::PerformanceRecord`
  (`perf_history.rs:135`) is appended per transfer; `perf_predictor`
  (`perf_predictor.rs`) `load()`s it (:220) and maintains per-profile
  coefficients (`HashMap<ProfileKey, PredictorProfile>`, :201). This is
  the substrate for the known-workload replay path in Design §3.

The gaps are above that leaf:

- **Local copy** still owns a local-shaped `TransferOrchestrator`
  (`orchestrator/orchestrator.rs:116`), which builds its own
  multi-thread runtime (`Builder::new_multi_thread().build()`, :137) and
  takes `LocalMirrorOptions` (:134), runs local-only fast paths
  (`orchestrator/fast_path.rs` via `maybe_select_fast_path` →
  `FastPathDecision::{NoWork, Tiny, Huge, …}`), collects all headers, and
  only then plans. **Local does not consume `determine_remote_tuning`** —
  its parallelism comes from this runtime + worker model, so the dial
  must subsume the local parallelism source as well as the remote
  ladders.
- **Three static stream-count ladders exist** (REV3 claimed two and
  called the other two "stale" — that was wrong; all three are live and
  the `tuning.rs` doc comment itself flags the multi-ladder problem):
  1. `remote/tuning.rs::determine_remote_tuning(total_bytes)` (:11) —
     size-keyed `TuningParams`; the client's ladder. Callers:
     `remote/push/client/mod.rs:232`, daemon `pull_sync.rs:500/550/687`,
     daemon `pull.rs:141/261`. Its own doc comment (`tuning.rs:7-10`):
     *"the daemon's push negotiation currently runs its own ladder and
     wins (single-owner consolidation is w2-2)."*
  2. `blit-daemon .../push/control.rs::desired_streams(files)` (:476,
     called :198/:267) — daemon push negotiation, keyed on **both**
     `total_bytes` and `file_count` (e.g. `file_count >= 200_000 → 16`).
  3. `blit-daemon .../pull.rs::pull_stream_count(total_bytes,
     tuning_max)` (:904, called :167/:277) — deprecated Pull RPC,
     byte-keyed, clamped to `tuning.max_streams`.

exec
/usr/bin/zsh -lc "sed -n '1,190p' docs/plan/SMALL_FILE_CEILING.md" in /home/michael/dev/Blit
 succeeded in 0ms:
# Small-file transfer to the hardware ceiling

**Status**: Active
**Created**: 2026-07-05
**Supersedes**: nothing
**Decision ref**: D-2026-07-04-4 (Draft → Active, owner "go")

## Principle (owner, 2026-07-05)

blit's guiding principles are **FAST, SIMPLE, RELIABLE** — every
change serves at least one or it's scrapped. blit must be the
fastest way to transfer files in **any** scenario. Goals are
therefore **ceiling-driven, never competitor-relative**: a
"beat tool X by N%" bar embeds a stopping condition and is the wrong
way to engineer this tool. Other tools function only as
**tripwires** — any scenario where any tool measures faster than
blit is, by definition, proof blit is off its hardware ceiling and
is a finding to fix, regardless of margins.

## Goal

For the workload classes where the 2026-07-04/05 10 GbE session
measured blit off its ceiling — many-tiny-file and mixed transfers —
blit's wall time becomes bounded by a **named hardware limit** (wire,
target-filesystem parallel create floor, source enumeration floor),
demonstrated by profile evidence and a stream-scaling curve, not by
blit's own stream policy or per-file overhead.

Measured gap analysis (durable evidence:
`docs/bench/10gbe-2026-07-05/` — DIAGNOSIS.md carries the daemon-log
extracts and arithmetic; the CSVs carry every matrix cell; DEVLOG
2026-07-05 entries are the narrative record):

| cell | blit today | ceiling arithmetic | tripwire |
|---|---|---|---|
| push 10k×4 KiB | 2.4–3.3 s | wire: **34 ms** (40 MiB @ 9.9 Gbit); fs floor: ~150 µs/file proven single-pipe on this ZFS, ÷ parallelism → **~0.2–0.5 s** | rsyncd 1.5 s |
| pull 10k×4 KiB | 446–484 ms | client fs = tmpfs (µs creates); wire+protocol class: **≪ 200 ms** | rsyncd 367 ms |
| push mixed 512 MiB+5k | 1.8–2.2 s | big file alone: ~450 ms wire; small remainder as above | rsyncd 1.24 s |

Diagnosis (from the session's daemon logs): the 10k push rode **one
stream** — `engine::initial_stream_proposal` is byte-weighted, so
40 MiB proposes a single stream despite 10,000 files — and paid
~215 µs/file sequentially on the daemon. The parallel machinery
(elastic streams, work-stealing, mid-transfer resize) exists and
negotiated 8 connections for the 1 GiB push in the same session.
This is a policy gap plus per-file overhead, not missing machinery.

## Non-goals

- Competitor-relative targets of any kind (see Principle).
- WAN/latency-shaped tuning (separate scenario class; gets its own
  ceiling analysis when a rig exists).
- Non-Linux rig ceiling targets (no measurement hardware this plan
  can bind to; Windows/macOS must not regress — suite + CI guard).
- Encrypted-transport scenarios (ssh-wrapped tools measured only as
  tripwires; blit's transport security model is unchanged by this
  plan).

## Constraints

- Every slice serves FAST without violating SIMPLE (dial stays the
  single tuning owner; no second engine, no special-case paths that
  survive past their measured need) or RELIABLE (REV4 invariants:
  byte-identical, StallGuard, cancellation, byte accounting).
- No wire-visible protocol change without a dedicated owner gate on
  the wire design before code (sf-6); mixed-version peers keep
  working via existing negotiation.
- No measured cell regresses beyond run-to-run noise (±10%),
  guarded by the committed baseline.
- Test count never drops; every slice through the codex loop
  (D-2026-07-04-1).
- Small-file parallel writes must respect the receiver capacity
  profile (spinning-pool receivers bound their own parallelism —
  the existing bounded-unilateral dial contract, D-2026-06-20-1).

## Acceptance criteria

- [ ] For each cell above: a recorded **limiter analysis** (profile
      + stream-scaling curve, committed with the slice records)
      demonstrating wall time is bound by a named hardware limit,
      not by stream policy or blit-controlled per-file overhead.
- [ ] Scaling evidence: files/s rises with stream count until the
      named limiter binds — the curve flattens at hardware, not at
      policy.
- [ ] **Tripwires clean**: no tool in the committed sf-1 harness
      matrix — rsyncd, rsync-over-ssh, rclone in its best measured
      config (`--ignore-checksum`, tuned `--transfers`), and `cp -a`
      for local cells — measures faster than blit on any cell. (The
      harness and this list are the same set by construction; adding
      a tripwire tool means adding it to the harness.)
- [ ] All baseline matrix cells stay within run-to-run noise (±10%)
      of the committed `docs/bench/10gbe-2026-07-05/` baseline.
- [ ] The comparison + scaling harness is committed and the owner
      can rerun it against any daemon host in one command.

## Design

Levers, cheapest first, measuring between each — sequencing exists
to find the ceiling with the least machinery, not to stop early:

1. **File-count-aware stream proposal** (blit-core `engine/`):
   `initial_stream_proposal` (and the pull-side equivalent) weight
   file count alongside bytes so many-tiny-file manifests open
   multiple streams; work-stealing spreads per-file cost across
   daemon workers. Push knows counts from enumeration, pull from
   the manifest.
2. **Per-file cost to the syscall floor** (daemon receive + client
   pull write paths): profile first (`strace -c`/`perf` during a
   small transfer), then cut — candidates: temp-file+rename
   pattern, separate set-times/set-perms syscalls, per-file
   need-list echo. The profile, not intuition, names the cuts.
3. **Resize-on-file-backlog**: feed the existing ue-2 resize
   machinery a backlog signal so a stream drowning in tiny files
   triggers mid-transfer ADD — this is also the organic resize
   trigger byte-bound workloads can never produce.
4. **Tar-shard push lane** (wire-visible, own owner gate): bundle
   tiny files into shard frames on the push wire as the local
   engine and delegated lane already do — amortizes both protocol
   roundtrips and daemon syscalls. Reached when the limiter
   analysis shows per-file framing itself is the binding cost.

Risks: parallel small-file writes can seek-storm spinning pools —
bounded by the receiver capacity profile (constraint above); lever 2
touches platform-sensitive syscall paths — Windows suite must stay
green; lever 4 adds wire complexity — SIMPLE requires the limiter
analysis to prove it earns its keep before design review.

## Slices

1. **sf-1 tripwire harness**: commit `scripts/bench_tripwires.sh`
   (derived from the session's ad-hoc runner): full matrix — blit,
   rsyncd (spun on the daemon host over ssh), rsync-over-ssh,
   rclone best-config, `cp -a` local — fresh targets every run,
   plus a stream-scaling probe mode (files/s vs stream count). The
   2026-07-05 baseline already lives in `docs/bench/10gbe-2026-07-05/`
   (committed with this plan); sf-1 makes it re-runnable in one
   command. No production code.
2. **sf-2 dial file-count weighting**: proposal-table unit pins
   (10k tiny → multi-stream; 1×1 GiB unchanged; mixed →
   intermediate) + loopback e2e pin that a 10k-file push opens >1
   data-plane connection.
3. **sf-3a per-file cost limiter analysis** (analysis-only, w8-1b
   precedent): `strace -c`/`perf` profile of daemon receive and
   client pull-write during a small transfer; deliverable is a
   committed analysis naming each per-file syscall cost and the
   ordered list of candidate cuts, each with its expected saving.
   No code.
4. **sf-3b… one cut per slice**: each accepted cut from sf-3a lands
   as its own review-loop slice with its own loopback
   per-file-cost proxy pin (so CI catches gross regressions
   without the rig). The count of sf-3x slices is set by sf-3a's
   list, not guessed here.
5. **sf-4 rig re-measure + limiter analysis**: rerun sf-1 harness on
   the 10 GbE rig; record the limiter analysis per cell. Hardware-
   bound everywhere + tripwires clean → acceptance review with the
   owner. Otherwise the analysis names what binds; continue.
6. **sf-5 resize-on-backlog feed** (if sf-4 names stream count
   under load as a binder, or the owner wants the ue-2 organic
   trigger regardless — flagged at sf-4).
7. **sf-6 tar-shard push lane** (if sf-4/sf-5's analysis names
   per-file wire framing as the binder). Wire-visible; the owner
   gate consumes the full REV4 wire-contract deliverable set
   **before any code**: the proto messages/fields and their field
   numbers named; capability negotiation for the shard lane
   specified; old-client→new-daemon and new-client→old-daemon
   behavior stated; and mixed-version compatibility tests specified
   and landing **before** any behavior depends on the lane.
8. **sf-7 verdict**: final rig run, limiter analyses committed,
   acceptance checklist walked with the owner; plan → Shipped or
   the remaining gap gets its own named follow-on.

## Open questions

- **sf-6 wire gate** (standing): the tar-shard lane's wire design
  needs explicit owner sign-off at execution time — recorded here
  so no session treats sf-6 as pre-authorized code. — owner

codex
So far I see no direct conflict with the ceiling principle or the codex loop. The main review surface is whether the draft overstates deletion/symmetry without naming a few protocol and authority hazards.
exec
/usr/bin/zsh -lc "sed -n '220,420p' docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md" in /home/michael/dev/Blit
 succeeded in 0ms:
     byte-keyed, clamped to `tuning.max_streams`.
  These feed the negotiated `DataTransferNegotiation.stream_count`
  (field 4) carried on the wire.
- **Pull is split**: the **deprecated `Pull` RPC** is already
  multistream (client reads `stream_count` and branches on
  `if stream_count <= 1`; daemon computes `pull_stream_count`).
  **PullSync** — the current path — is **single-stream today**:
  `blit-daemon .../pull_sync.rs:568` hardcodes `let stream_count = 1u32`.
  So multistream-pull work (`ue-r2-1g`) is about bringing the deprecated
  Pull's pattern into PullSync through the engine; it is **not** adding
  multistream to a path that never had it.
- The proto has `DataTransferNegotiation.stream_count` (field 4) and
  reserves 5–10 for RDMA (`proto/blit.proto:121-127`); there is no
  capacity profile and no resize messages. Field 11 is the first free
  number.
- `data_plane.rs` carries `StallGuard`/`StallGuardWriter`
  (`remote/transfer/stall_guard.rs:75/139`) today; the `Probe` generic
  (`DataPlaneSession<P: Probe = NoProbe>`) does **not** exist at HEAD —
  it arrives with the PR1 salvage commit (`e6ef095`). The hand-resolved
  conflict at salvage is `StallGuardWriter` vs the `Probe` generic.

## Design

### 1. Engine ownership

Introduce a new `TransferEngine` rather than renaming
`TransferOrchestrator` in place (it is local-shaped: own runtime,
`LocalMirrorOptions`). `TransferOrchestrator` becomes the local adapter
that constructs local `Source`, `Sink`, options, and local strategy
inputs, then calls the engine. Push, PullSync, and delegated transfers
call the same engine with different source/sink and negotiation
adapters.

The engine owns:

- strategy selection (`journal_no_work`, `no_work`, `tiny_manifest`,
  `single_huge_file`, the single-file shortcut, streaming pipeline);
- dial creation and updates (subsuming all three ladders + local
  parallelism);
- payload work queue;
- progress and telemetry wiring;
- invariants around first work, fallback, cancellation, and finish.

Path-specific code remains only at boundaries where the protocol
differs: path resolution, module authorization, source/destination
manifest exchange, mirror/delete authority, and legacy compatibility.

**Engine type (the q3 open question, owner-deferred to the agent):** the
agent recommends the new `TransferEngine` + local adapter above, not an
in-place rename. Ratified at the `ue-r2-1c` slice; owner may override.

### 2. Local fast paths are strategies, not bypasses

V1 said "no separate small-transfer path"; review found that wording
would accidentally delete useful FAST behavior. REV4 keeps the REV2/REV3
rule: the small/huge/no-op local paths may remain, but only as
engine-owned strategies with common accounting and tests. Their real
names are `journal_no_work`, `no_work`, `tiny_manifest`,
`single_huge_file`, plus the single-file copy shortcut
(`orchestrator.rs:178`, which currently bypasses history recording —
folding it under the engine gives it accounting it lacks today).

This preserves SIMPLE for the operator (one command model, one behavior
owner) and FAST (the low-overhead tiny/single-file paths stay where they
beat the full pipeline).

### 3. Streaming plan foundation

The 1s start requirement cannot be hidden inside the sequencer-converge
slice. Today the local path collects all headers before planning, and
PullSync waits for complete client and server manifests before transfer.
REV4 makes streaming planning a separate foundation:

- define an `InitialPlan` / `PlanUpdate` shape produced from a partial
  header stream;
- feed the shared streaming pipeline as safe payload batches become
  available;
- refine tar-shard and bundle targets as enumeration reveals shape;
- preserve mirror/delete and scan-completeness reliability;
- measure first byte and first useful progress per transfer mode.

The initial-plan strategy splits on whether telemetry exists for this
workload shape, reusing the cross-run history already in-tree
(`perf_history` appends a `PerformanceRecord` per transfer;
`perf_predictor` loads it and trains per-profile coefficients):

- **Novel workload** (no extant record): emit a conservative initial
  plan from the partial scan, start copying immediately, and let the live
  tuner (PR1 telemetry, landed with `ue-r2-1a`) adjust from the first
  byte.
- **Known workload** (extant record): if the last run looks optimal,
  reproduce that plan immediately; otherwise recalculate onto the
  live-tune path.

Both branches meet the 1s start. Neither is an exception — the exception
class is only the unsafe-before-full-knowledge modes (mirror/delete,
resume, checksum-refusal) named in the Acceptance Criteria.

### 4. Dial and bounded-unilateral negotiation

The byte sender owns the live dial, bounded by the byte receiver's
capacity profile. The profile travels from receiver to sender during
setup:

- push: destination daemon advertises receiver capacity to the push
  client;
- pull: pull client advertises receiver capacity in the pull spec or
  setup message so the source daemon can send within it;
- delegated: destination daemon advertises its receiver capacity when it
  asks the source daemon to send.

The rich capacity profile should include, at minimum: CPU cores
available to the transfer; disk class or drain class; current load
estimate; maximum accepted streams; drain-rate estimate; max safe chunk
bytes; max safe prefetch / in-flight bytes. (Owner: more data serves the
ubergoal; do not minimize the negotiation payload.)

The initial dial starts **below** the profile ceiling with margin.
Telemetry can increase or decrease the cheap dials (chunk, prefetch, TCP
buffers) immediately; stream-count changes arrive in the later resize
slice. The dial is a **mutable object read by both ends from the
live-dials slice onward** — this is what makes continuous (`ue-r2-2`) a
wire-up rather than a retrofit (see the C-ready acceptance criterion).

### 5. Wire contract before dependent code

REV4 makes wire shape an early deliverable. Proposed proto direction:

- append `CapacityProfile receiver_capacity = 11` to
  `DataTransferNegotiation` (field 11 is the first free number after the
  5–10 RDMA reservation);
- add a capacity profile to the request/setup side where the receiver is
  the client, especially PullSync and delegated pull;
- add explicit peer capability bits/fields so resize messages are never
  sent to an old peer;
- add `DataPlaneResize` and `DataPlaneResizeAck` as negotiated control
  messages in the relevant control streams, not as blind TCP data-plane
  records.

Exact field names and numbers are part of the wire slice acceptance
criteria. Old peers must see current behavior: no capacity profile means
use today's static/conservative behavior; no resize support means no
mid-transfer add/drop.

### 6. Work-stealing is behavior

The adaptive PR2 work queue is required for continuous stream add/drop,
but it also changes scheduling. It must land with behavior tests:

- slow sink does not block all other sinks;
- one sink failure propagates the real error and shuts down cleanly;
- cancellation still aborts spawned workers;
- byte totals and file totals remain correct;
- StallGuard coverage survives the `Probe` conflict resolution.

### 7. Pull parity after pull multistream

Loopback parity is only meaningful after local↔local, local→daemon, and
daemon→local all use comparable engine paths. PullSync is single-stream
today (`pull_sync.rs:568`), so REV4 moves the parity-band gate after pull
multistream lands through the engine.

## Risks

- **Cherry-pick StallGuard regression.** Hand-resolving the
  `data_plane.rs` conflict (`StallGuardWriter` vs the `Probe` generic)
  during salvage could regress the byte-identical property PR2 pins.
  Mitigation: the byte-identical regression tests, the 1370 baseline, and
  the new work-stealing behavior tests in `ue-r2-1a`.
- **Receiver over-advertises its capacity profile.** A receiver claiming
  more drain capacity than it has could overwhelm itself on the first
  byte, because there is no probe phase to catch it. Mitigation: the
  *initial* conservative setting starts below the advertised ceiling with
  margin (fewer streams than allowed, ramping up as telemetry proves the
  link); the live tuner then backs off via `write_blocked` / retransmit
  telemetry. The engine is never exposed at the full advertised ceiling
  on the first byte.
- **1s-start pathological source.** A slow first enumeration over a huge
  directory could blow the 1s budget. Mitigation: the planner yields an
  initial plan from a partial scan and refines; it does not wait for full
  enumeration. Any mode that genuinely cannot meet the budget must be an
  explicit, tested, reported RELIABLE exception — not a silent miss.
- **Wire-compat break with mixed peers.** Adding field 11 and resize
  messages could mispair old/new peers. Mitigation: the wire slice lands
  with explicit old-client/new-daemon and new-client/old-daemon
  compatibility tests *before* any behavior depends on the new fields,
  and resize messages are gated on advertised peer capability bits.
- **RELIABLE-exception loophole.** Allowing tested first-byte exceptions
  risks exceptions proliferating until RELIABLE is silently eroded.
  Mitigation: every exception is explicit, tested, individually reported
  to the owner, and revisited at the `ue-r2-1d` sign-off — none is added
  by coder fiat.
- **Under-converged ladders (new in REV4).** Because REV3 mis-counted the
  ladders, a coder following it would have converged only
  `determine_remote_tuning` and left `desired_streams` and
  `pull_stream_count` as live side doors — re-creating the exact
  "daemon runs its own ladder and wins" split this plan exists to kill.
  Mitigation: the acceptance criterion now enumerates all three plus the
  proto field by `file:line`; `ue-r2-1e`/`ue-r2-1f`/`ue-r2-1g` each name
  the ladder they retire.


exec
/usr/bin/zsh -lc "sed -n '420,490p' docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md" in /home/michael/dev/Blit
exec
/usr/bin/zsh -lc "nl -ba docs/plan/ONE_TRANSFER_PATH.md | sed -n '1,260p'" in /home/michael/dev/Blit
 succeeded in 0ms:

## Revised Slices

Review-loop-sized docs/code slices; one coherent, testable change each.
They replace the v1 slice shape only if the owner accepts REV4 as the
final plan. Slice IDs are unchanged from REV3 (the slice *shape* did not
change; only the code-reality grounding did).

1. **`ue-r2-1a-salvage-substrate`** — Cherry-pick adaptive PR1+PR2 up to
   `eafb187` (`e6ef095` → `af66ff5` → `b797b73` → `eafb187`), excluding
   `d9d4ec7` (PR3 WIP, does not build). Resolve `data_plane.rs`
   `StallGuardWriter`-vs-`Probe`. Treat work-stealing as behavior, not
   inert substrate: add/keep slow-sink, failing-sink, cancellation,
   byte-accounting, StallGuard, and byte-identical tests. The elastic
   work-stealing stream-set exists from this slice onward (C-ready seam).
2. **`ue-r2-1b-wire-dial-contract`** — Define capacity profile, peer
   capability, and resize proto shape (`receiver_capacity = 11`,
   `DataPlaneResize`/`Ack`). Add compatibility tests for old client/new
   daemon and new client/old daemon. No behavior depends on these fields
   until this slice is green.
3. **`ue-r2-1c-engine-shell-local-adapter`** — Add `TransferEngine` and
   convert `TransferOrchestrator` into a local adapter. Move the local
   fast paths (`journal_no_work`, `no_work`, `tiny_manifest`,
   `single_huge_file`, single-file shortcut) under engine-owned
   strategies, preserving behavior and adding accounting where the
   single-file shortcut lacked it.
4. **`ue-r2-1d-streaming-plan-foundation`** — Introduce partial-scan
   initial plans and plan updates (novel vs known per Design §3). Prove
   first-byte / first-useful-work timing for local and push shapes, and
   document any RELIABLE exception that cannot safely move bytes before
   complete knowledge.
5. **`ue-r2-1e-live-cheap-dials`** — Replace the static
   `determine_remote_tuning` chunk/prefetch/TCP-buffer ladder with the
   single mutable dial. Start conservative within the receiver profile,
   then adjust cheap dials from PR1 telemetry. The dial is a mutable
   object read by both ends from this slice onward (C-ready seam).
6. **`ue-r2-1f-push-converge`** — Route push through the engine while
   preserving manifest streaming, need-list batching, fallback timing,
   scan-completeness purge safety, and old/new compatibility. **Retire
   the daemon `desired_streams` ladder** into the dial (this is the
   ladder `tuning.rs` says currently "wins").
7. **`ue-r2-1g-pull-multistream-converge`** — Route PullSync through the
   engine and make it multistream there (it is single-stream today),
   harvesting the deprecated Pull RPC's multistream pattern. Preserve
   resume, checksum refusal, delete-list authority, cancellation,
   per-stream failure, and gRPC fallback behavior. Absorbs
   `MULTISTREAM_PULL.md` acceptance criteria.
8. **`ue-r2-1h-delete-deprecated-pull-rpc`** — Delete the deprecated
   `Pull` RPC (and its `pull_stream_count` ladder) after PullSync has
   harvested the needed multistream/fallback pattern and tests cover the
   replacement, including old/new peer pairs.
9. **`ue-r2-2-stream-resize`** — Finish negotiated
   `DataPlaneResize`/`DataPlaneResizeAck` and add/drop streams mid
   transfer from live telemetry, using the elastic work queue from
   `ue-r2-1a`. Wires onto the already-mutable dial and elastic
   stream-set — a wire-up, not a restructuring.

### Slice dependencies

Explicit blocking, since REV4 has nine slices and ordering matters:

- `1a` (substrate) blocks everything — all later slices build on the
  elastic work-stealing stream-set.
- `1b` (wire contract) blocks `1e` (dial uses the capacity profile), `2`
  (resize proto), and contributes the compat tests `1h` gates on.
- `1c` (engine shell) blocks `1d`, `1e`, `1f`, `1g` — the streaming plan,
  dial, push, and pull all run inside the engine.
- `1d` (streaming plan) blocks the 1s-start acceptance and feeds `1g`
  (pull 1s-start).
- `1f` (push) and `1g` (pull) both depend on `1c`; `1g` also depends on
  `1d`. `1f` retires `desired_streams`; `1g`/`1h` retire

 succeeded in 0ms:
     1	# One transfer path — direction-invariant by construction
     2	
     3	**Status**: Draft
     4	**Created**: 2026-07-05
     5	**Supersedes**: post-REV4 residue item "pull 1s-start restructuring"
     6	(absorbed here); pauses `docs/plan/SMALL_FILE_CEILING.md` after sf-2
     7	(D-2026-07-05-1)
     8	**Decision ref**: D-2026-07-05-1 (directive + pause); Active flip gets
     9	its own entry
    10	
    11	## Directive (owner, 2026-07-05, verbatim)
    12	
    13	> "make ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF
    14	> ANYTHING EVER using anything else because anything else does not
    15	> exist."
    16	
    17	> "just make it so that I NEVER see a situation where pull is faster
    18	> than push or vice versa. that CAN NEVER be possible because of
    19	> something blit did. it should be identical if I start the transfer
    20	> from skippy and push to this machine or if I start the transfer on
    21	> this machine and pull from skippy."
    22	
    23	> On benchmark methodology: "tmp on one side, spinning rust on the
    24	> other is not a valid test."
    25	
    26	Scope, wire, and process were explicitly delegated to the agent
    27	("no idea. you architected this"; "I DO NOT CARE. FIX IT."). The
    28	owner's requirement is the invariant; everything below is the
    29	architecture that makes the invariant impossible to violate rather
    30	than merely maintained by discipline.
    31	
    32	## Goal
    33	
    34	One `TransferSession` implementation owns every byte transfer blit
    35	performs. A transfer has a SOURCE role and a DESTINATION role; which
    36	end initiated, and which CLI verb was used, select roles — they do not
    37	select code. When this plan ships, the per-direction drivers (client
    38	push driver, daemon push-receive, client pull driver, daemon
    39	pull-send, delegated-pull driver, local orchestration) **do not
    40	exist**: for fixed endpoints and dataset, direction/initiator/verb
    41	cannot affect behavior or wall time by blit's doing, because there is
    42	no second code path to differ.
    43	
    44	## Non-goals
    45	
    46	- Preserving wire compatibility with pre-plan builds. The `Push` and
    47	  `PullSync` RPCs are deleted at cutover; both ends upgrade in
    48	  lockstep (repo precedent: the `PullSyncHeader` removal; the owner
    49	  operates every deployed peer).
    50	- Making different hardware perform identically. If src and dst sit
    51	  on different disks, the two *data directions* still differ by
    52	  physics; the invariant is that the same data direction between the
    53	  same endpoints is identical regardless of who initiates and which
    54	  verb is used.
    55	- WAN-shaped tuning (unchanged from SMALL_FILE_CEILING's non-goal).
    56	- New features. This is a consolidation; capability parity with
    57	  today (mirror, filters, resume, fallback, delegation, progress,
    58	  jobs, cancellation) is the bar.
    59	
    60	## Constraints
    61	
    62	- FAST/SIMPLE/RELIABLE and the ceiling-driven principle
    63	  (D-2026-07-04-4) stand. This plan exists because SIMPLE was
    64	  violated at the choreography layer.
    65	- **Converge up, not down**: per benchmark cell, the unified session
    66	  must match the better of today's two directions (within ±10% run
    67	  noise), not their average. Unification that slows the fast
    68	  direction fails review.
    69	- REV4 invariants carry: byte-identical results, StallGuard,
    70	  cancellation, byte-accounting. Existing pins are ported (not
    71	  dropped) as tests become role-parameterized; test count never
    72	  drops.
    73	- The sf-2 shape-correction behavior (stream count corrects as the
    74	  need list accumulates) becomes the one and only stream policy —
    75	  both directions inherit it by construction; its pins carry over.
    76	- Wire contract discipline (REV4 rule): the unified session's proto —
    77	  messages, field numbers, capability negotiation, transport
    78	  selection — is a reviewed doc+proto slice **before** any behavior
    79	  depends on it.
    80	- Every slice through the codex loop (D-2026-07-04-1); tree green
    81	  after every slice; transitional coexistence of old+new paths is
    82	  scaffolding only — the plan is not Shipped until the deletion slice
    83	  lands and the deletion proof is recorded.
    84	- Windows parity: suite green on the owner's machine + windows-latest
    85	  CI before Shipped.
    86	
    87	## Acceptance criteria
    88	
    89	- [ ] **Initiator/verb invariance (the owner's sentence, measured)**:
    90	      on a symmetric rig (same filesystem class both ends, cold
    91	      caches, disk-to-disk), for each data direction and workload
    92	      (large / 10k-small / mixed): wall time initiating from end A vs
    93	      end B, and via push-verb vs pull-verb, differs only within
    94	      run-to-run noise (±10%). Matrix committed as evidence.
    95	- [ ] **Deletion proof**: `remote/pull.rs` (driver), `remote/push/`
    96	      (driver), daemon `push/control.rs` choreography, daemon
    97	      `pull_sync.rs` choreography, the delegated-pull driver, the
    98	      separate local orchestration path, and the `Push`/`PullSync`
    99	      RPCs no longer exist in the tree; one `TransferSession` and one
   100	      `Transfer` RPC remain. Recorded file-by-file in the final
   101	      slice's finding doc.
   102	- [ ] Capability parity: mirror (both mirror-kinds + scan-complete
   103	      guard), filters, block-resume, gRPC fallback carrier, delegated
   104	      transfer, progress events, jobs/cancel, read-only enforcement —
   105	      each demonstrated by ported tests on the session.
   106	- [ ] Suite green throughout; final test count ≥ pre-plan baseline
   107	      (1483); all REV4 invariant pins and the sf-2 pin pass
   108	      role-parameterized.
   109	- [ ] Benchmark methodology corrected and recorded: symmetric-fs
   110	      cells are the verdict cells; tmpfs cells remain only as
   111	      explicitly-labeled wire-reference rows (never compared across
   112	      directions with asymmetric endpoints).
   113	- [ ] Windows: full suite green (owner machine) + windows-latest CI.
   114	
   115	## Design
   116	
   117	**What already is one code** (kept, becomes the session's engine):
   118	`remote/transfer/` — pipeline, sink/source abstractions, data plane,
   119	diff planner, tar-shard, stall guard, progress, `operation_spec` (the
   120	REV4 unified contract), and the engine dial (stream policy incl. sf-2
   121	shape correction). The defect layer is above it: four driver loops
   122	choreograph these pieces differently per direction.
   123	
   124	**The one choreography** (roles, not directions):
   125	
   126	1. Initiator opens the single bidi `Transfer` RPC and sends the
   127	   operation spec: which end is SOURCE, which is DESTINATION, path/
   128	   module, filters, mirror/resume flags, capabilities.
   129	2. SOURCE enumerates and **streams** its manifest immediately (no
   130	   buffered-enumeration phase — this generalizes push's fast start;
   131	   pull's full-enumeration-then-negotiate slow start is deleted, which
   132	   absorbs the "pull 1s-start" residue item).
   133	3. DESTINATION diffs incrementally against its own filesystem and
   134	   returns need-list batches (one diff owner, always the end that
   135	   owns the target fs — push's proven model; pull_sync's
   136	   source-side diff is deleted).
   137	4. The data plane opens at the dial floor immediately; stream count
   138	   shape-corrects as the need list accumulates (sf-2 mechanism, now
   139	   the only policy, both roles).
   140	5. SOURCE feeds payloads (files / tar-shards / resume blocks) through
   141	   the one pipeline into the data plane; DESTINATION writes through
   142	   the one receive path.
   143	6. Mirror: DESTINATION computes deletions from the completed source
   144	   manifest it received (filter-scoped, scan-complete-guarded) and
   145	   executes them locally. One rule, no per-direction delete
   146	   choreography.
   147	7. Resume: optional block-hash phase inside the same session, same
   148	   messages regardless of roles.
   149	8. Summary/byte-accounting: one record shape.
   150	
   151	**Transport facts vs choreography**: the connection-initiating end
   152	dials TCP data-plane sockets (NAT reality) — byte direction within a
   153	socket is set by role, not by who dialed. The gRPC-fallback lane
   154	becomes a *byte-carrier option* inside the same session (control-
   155	stream frames instead of TCP sockets), selected at negotiation — not
   156	a separate transfer path. Resize keeps its controller-at-sender rule.
   157	
   158	**Delegated transfer**: a daemon receiving a delegated request simply
   159	becomes an initiator of the same session against the other daemon
   160	(destination role on its module fs). The bespoke delegated-pull
   161	driver is deleted; the delegation *gate* (authorization) stays.
   162	
   163	**Local transfers**: the same session driver over an in-process
   164	transport (both roles in one process, no wire). The engine underneath
   165	is already shared; the separate local orchestration path is deleted
   166	in the final phase. Local perf pins (e.g. 1 GiB local, no-op mirror)
   167	guard the migration.
   168	
   169	**Affected crates**: `blit-core` (new `transfer_session` module;
   170	`remote/pull.rs`, `remote/push/` drivers deleted at cutover),
   171	`blit-daemon` (one `Transfer` handler replaces push/pull_sync/
   172	delegated handlers), `blit-cli`/`blit-app` (verbs map to roles),
   173	`proto/blit.proto` (one `Transfer` RPC; `Push`/`PullSync` deleted),
   174	`blit-tui` (progress/jobs consume the same events).
   175	
   176	**Risks**: largest consolidation since REV1 — pull.rs alone is ~108K;
   177	mitigated by strangler slices with the tree green throughout and a
   178	non-optional deletion slice. Per-cell regression risk on today's
   179	faster direction — mitigated by the converge-up constraint and
   180	baseline parity pins per slice. Wire break — lockstep upgrade,
   181	owner-controlled fleet. Windows receive paths (win_fs) — parity gate.
   182	Progress/jobs/TUI integration churn — the session emits the existing
   183	event contract (w6-1) at the same boundaries.
   184	
   185	## Slices
   186	
   187	One coherent, testable change per slice — sized for the `.review/`
   188	loop. Tree green after every slice; old paths keep working until
   189	otp-9 deletes them.
   190	
   191	1. **otp-1 wire+session contract (doc + proto, no behavior)**: the
   192	   `Transfer` RPC and message set — roles, phases, field numbers,
   193	   capability negotiation, transport selection, resume/mirror
   194	   phases, error/cancel semantics. Full REV4 wire-contract
   195	   deliverable set; codex-reviewed before any code consumes it.
   196	2. **otp-2 TransferSession core (blit-core)**: role-parameterized
   197	   state machine over the existing engine with an in-process
   198	   transport; unit/e2e tests run BOTH role assignments over the same
   199	   fixtures — the invariance property enters the test suite here.
   200	3. **otp-3 daemon serves `Transfer`, client initiates as SOURCE**
   201	   (remote push-equivalent rides the session); A/B parity pins vs
   202	   old push (byte-identical trees, summary parity, sf-2 pin ported).
   203	4. **otp-4 roles swapped: client initiates as DESTINATION** (pull-
   204	   equivalent) — the same code with roles flipped; the parity suite
   205	   reruns with no per-direction test code.
   206	5. **otp-5 mirror + filters** on the session (one delete rule).
   207	6. **otp-6 resume** block phase.
   208	7. **otp-7 fallback byte-carrier** (control-stream frames) as the
   209	   session's alternate transport.
   210	8. **otp-8 delegated transfer** = daemon-initiated session; bespoke
   211	   delegated-pull driver retired behind the existing gate.
   212	9. **otp-9 cutover + deletion**: CLI/app/TUI route every remote
   213	   operation through the session; `Push`/`PullSync` and all four
   214	   drivers deleted from the tree and the proto; ported-test
   215	   accounting proves count never dropped. Deletion proof recorded.
   216	10. **otp-10 local transfers** ride the in-process transport; the
   217	    separate local orchestration is deleted; local perf pins hold.
   218	11. **otp-11 symmetric-rig acceptance run**: sf-1 harness matrix
   219	    corrected (same-fs disk-to-disk verdict cells, cold caches,
   220	    tmpfs as labeled wire-reference only) + the initiator/verb
   221	    invariance A/B matrix; committed as this plan's acceptance
   222	    evidence.
   223	12. **otp-12 verdict**: acceptance checklist walked with the owner;
   224	    plan → Shipped; SMALL_FILE_CEILING resumes (or is re-derived)
   225	    against the unified baseline — owner call at that point.
   226	
   227	## Open questions
   228	
   229	- None requiring owner input now — scope, wire, and process were
   230	  delegated (Directive section). Slice-level unknowns (exact proto
   231	  shapes, resume edge semantics, TUI event wiring) are settled inside
   232	  their slices through the codex loop. — owner

exec
/usr/bin/zsh -lc "nl -ba docs/DECISIONS.md | sed -n '72,140p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    72	## D-2026-06-12-1 — zero_copy.rs: delete (w8-1b verdict)
    73	- Decision: `zero_copy.rs` is deleted rather than wired in. The w8-1b evaluation (`docs/plan/ZERO_COPY_RECEIVE_EVAL.md`) recommended deletion and the owner agreed (2026-06-12 session). The deletion executes inside w8-1 once the w5-1 sentinel (lib.rs) is graded — it is no longer excluded from that sweep.
    74	- Why: the dead draft busy-waits on EAGAIN (would be rewritten, not revived); wiring needs a raw-fd special case beside a permanent buffered fallback; the CPU saving is a fraction of one core, Linux-only, and unmeasured. Revisit gate: 10 GbE benchmarks showing receive-side CPU saturation — design notes preserved in the eval doc.
    75	- Supersedes: D-2026-06-11-2 item (b) (zero_copy exclusion from W8.1 was pending this evaluation; the evaluation is done).
    76	
    77	## D-2026-06-20-1 — Transfer-core architecture conflict resolved: convergence, not ground-up redesign
    78	- Decision: The 2026-06-14 "redesign the transfer subsystem from the ground up" framing is resolved as **convergence**, not a rebuild. One src/dst-agnostic sequencer owns all four paths (local↔local, push, pull, daemon↔daemon); the dial (stream count + all transfer knobs) is a single live object adjusted from measured telemetry; the already-shared byte-moving leaf stays. Dials are **bounded-unilateral** (receiver advertises a capacity ceiling; sender owns the dial within it) ~~and **size-gated** (small transfers skip the probe entirely)~~ **(size-gate framing superseded by D-2026-06-20-2 q1 — there is no probe phase to skip; the engine moves within ~1s and tunes live)**. The adaptive-streams stack (PR1 telemetry + PR2 work-stealing queue, up to `eafb187`) is salvaged as the substrate per D-2026-06-07-2; PR3 WIP (`d9d4ec7`) stays excluded. ~~Built A-first (warmup), C-ready by construction (mutable dial + elastic stream-set exist from A, so continuous adjustment is a later feed, not a retrofit).~~ **(A/warmup staging superseded by D-2026-06-20-2 q1 — conservative start + live tuning from the first byte; C shipped as `ue-r2-2` under REV4/D-2026-06-20-5.)** Plan: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (Draft — awaiting owner Draft→Active flip). *(Stale wording struck 2026-07-04 on owner direction — "follow the existing pattern": the in-place-annotation pattern of D-2026-06-20-3/-6. The convergence direction itself stands unchanged.)*
    79	- Why: owner (30-year IT veteran, not a developer) judges the fragmentation — one engine for local, hand-wired loops for push/pull, three competing static stream-count tables, no live tuning — is the root of the "local↔local 10× slower than local→daemon" class of drift; a single engine makes that class impossible by construction and gives the LLM agent one place to update. Ground-up rebuild was judged too much; convergence on the existing shared leaf is the FAST/SIMPLE/RELIABLE fit. The adaptive substrate was purpose-built by an earlier Fable session as C's foundation, so building A on it does not paint the design into a corner.
    80	- Scope consequence: this **moots the standalone premise** of the queued incremental work and absorbs the goals — w2-2 (three ladders → one dial) is `ue-1b`; w2-3 multi-stream pull (`MULTISTREAM_PULL.md`) is `ue-1d` via the unified sequencer; w2-4 (delete deprecated Pull RPC) is `ue-1e`; adaptive-streams cherry-pick is `ue-1a`. `MULTISTREAM_PULL.md` is superseded as a standalone plan (kept as reference); its goal survives inside this plan. The design-review queue's correctness findings (w4-1 etc.) are independent and unaffected.
    81	- Supersedes: the "ground-up redesign" framing of the 2026-06-14 open question recorded in STATE.md (that open question is now closed); `MULTISTREAM_PULL.md` as a standalone plan (goal absorbed into `UNIFIED_TRANSFER_ENGINE.md` slice `ue-1d`).
    82	
    83	## D-2026-06-20-2 — UNIFIED_TRANSFER_ENGINE.md flipped Draft → Active; four bound parameters
    84	- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` is **Active**. Owner approved with four parameters that bind the design: (q1) **no probe-then-go phase** — the engine starts moving within ~1s at conservative defaults bounded by the receiver ceiling and the tuner adjusts dials live from the first byte; the "small-transfer threshold" is obviated (no probe to skip), and the **planner** carries the workload-shape judgment (file count vs bytes) that the old size gate proxied. (q2) the receiver advertises a **rich capacity profile** (CPU cores, disk class, load, max streams, drain estimate) — "more data serves the ubergoal"; do not minimize the negotiation payload. (q3) engine type **deferred to the agent**, who recommends a new src/dst-agnostic `TransferEngine` + a local adapter over renaming `TransferOrchestrator` in place — ratified at `ue-1c`. (q4) `ue-2` (mid-transfer stream add/drop via PR3's resize proto) is **in scope at Active**, sequenced last; 11 months of owner benchmarking is the justification, the 10 GbE rig is sign-off not a gate.
    85	- Why: owner answered the four gating questions (the stated Draft→Active condition) and said "active now." q1 materially improved the design — live-from-first-byte removes the fragile size threshold and collapses the A/B/C probe staging into "adjust what is cheap in `ue-1b`, add stream resize in `ue-2`."
    86	- Inference flagged for owner (now vetoed — see D-2026-06-20-3): the agent had proposed folding the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b) in as the planner half and superseding its "after audit Round 1" timing. **Owner vetoed 2026-06-20.** The absorption is dropped; D-2026-06-04-3 stands unchanged. The engine's workload-shape-awareness + first-byte-within-~1s requirements remain, stated on their own merits, not as the H10b concept.
    87	- Supersedes: the "A-first warmup probe" and "size-gated skip-probe" framings in the Draft version of `UNIFIED_TRANSFER_ENGINE.md` (already edited in-place). *(The proposed supersession of D-2026-06-04-3's streaming-planner timing is withdrawn per the owner veto — see D-2026-06-20-3.)*
    88	
    89	## D-2026-06-20-3 — Veto: do NOT fold the streaming planner (H10b) into the unified engine
    90	- Decision: The flagged inference in D-2026-06-20-2 is **vetoed by the owner.** The unified engine does **not** absorb the ratified-but-unbuilt streaming planner (D-2026-06-04-3 / H10b), and D-2026-06-04-3's "after audit Round 1" sequencing **stands unchanged** — the convergence plan does not supersede it. What survives from the vetoed inference: the engine's planner is **workload-shape-aware** (file count vs bytes; 100k×10B ≠ 1×20MB) and must meet the **first-byte-within-~1s** commitment by yielding an initial plan from a partial scan and refining. That is an engine-internal requirement stated on its own merits, **not** the H10b streaming-planner concept and **not** a supersession of D-2026-06-04-3. Whether the engine's fast-start enumeration and the separate H10b streaming planner overlap is left to the owner at audit Round 1, not pre-resolved here.
    91	- Why: owner did not intend to revive H10b by way of the convergence plan; the inference was the agent's, flagged for confirmation, and the owner declined it. The workload-shape-awareness goal was always standalone and stands.
    92	- Supersedes: nothing. Reverts the conditional H10b supersession that D-2026-06-20-2 had proposed (that entry is edited in-place to drop the inference and point here).
    93	
    94	## D-2026-06-20-4 — Unified transfer engine plan review freeze
    95	- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV2.md` is a Draft review candidate next to the original plan, and all unified-transfer-engine coding is frozen until the owner makes a final plan decision.
    96	- Why: review found the Active plan's direction is sound but several slices need tightening before code starts: streaming initial planning was hidden inside `ue-1c`, local fast paths need to become engine-owned strategies, work-stealing is observable behavior, wire compatibility needs concrete shape, and pull parity gates must wait for multistream pull.
    97	- Supersedes: D-2026-06-20-2 only as an implementation greenlight; it does not supersede the convergence direction or the owner's four bound parameters.
    98	
    99	## D-2026-06-20-5 — REV4 replaces UNIFIED_TRANSFER_ENGINE.md as the Active convergence plan
   100	- Decision: `docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md` is the **Active** unified-transfer-engine plan (owner: "rev4 replaces v1"). `UNIFIED_TRANSFER_ENGINE.md` (v1) flips Active → Superseded; the intermediate review candidates `REV2.md` and `REV3.md` flip Draft → Superseded — all three superseded by REV4. REV4 carries v1's lineage/absorption header forward, so the supersessions v1 recorded (MULTISTREAM_PULL absorbed as the pull-multistream slice `ue-r2-1g`; PIPELINE_UNIFICATION/UNIFIED_RECEIVE_PIPELINE Historical) remain in force. The plan-review freeze (D-2026-06-20-4) is lifted as to the **plan decision**; coding still requires a fresh per-slice owner authorization (AGENTS.md §9) — no slice (`ue-r2-1a` first) starts on this decision alone.
   101	- Why: REV4 is the only candidate whose code-reality section was verified against the tree (`HEAD` `09268eb`). REV3's headline "two static tables, not three" correction was itself wrong — all three stream-count ladders are live (`remote/tuning.rs::determine_remote_tuning`, `push/control.rs::desired_streams:476`, `pull.rs::pull_stream_count:904`), v1's three-ladder count was substantially right, and `tuning.rs`'s own doc comment confirms the daemon "runs its own ladder and wins". REV3 also wrongly said `determine_remote_tuning` drives local (it drives push + daemon pull) and conflated single-stream PullSync with the already-multistream deprecated Pull. REV4 = REV3 + corrected code reality, every symbol grounded with `file:line`, v1 lineage preserved. One Active plan avoids drift between candidates.
   102	- Supersedes: `docs/plan/UNIFIED_TRANSFER_ENGINE.md` (v1, Active → Superseded) and the review candidates `REV2.md` / `REV3.md` (Draft → Superseded) — all by `REV4.md`. Lifts D-2026-06-20-4's implementation freeze (the plan decision is now made). Does **not** supersede the convergence direction (D-2026-06-20-1), the four bound parameters (D-2026-06-20-2), or the H10b veto (D-2026-06-20-3). ~~The D-2026-06-20-1 warmup/size-gate cleanup remains an open owner question, untouched here.~~ *(Resolved 2026-07-04 — cleanup applied in place; see the edited D-2026-06-20-1.)*
   103	
   104	## D-2026-06-20-6 — Code→GPT-review→fix loop for the unified engine; ungated per-slice commits
   105	- Decision: Adopt a synchronous code→review→fix loop for the `ue-r2-*` slices (`docs/agent/GPT_REVIEW_LOOP.md`, Active). Claude codes + commits each slice, invokes GPT-5.5 via `codex` (headless here via the local `headroom` proxy) to review that commit, adjudicates every finding against source/tests, fixes the accepted ones, and proceeds. Three standing authorizations the owner gave this session: (a) **per-slice commits to `master` are ungated** for this loop — no agent branches, never push (push stays owner-only); (b) **per-slice code-quality acceptance is delegated** to the loop + validation suite — the owner is not a developer and will NOT be asked to bless code that passed validation+review ("that would just be theater"); (c) the agent proceeds autonomously and pauses only for genuine decisions/issues/blockers/plan-changes and the remaining owner gates (push; 10 GbE sign-off).
   106	- Why: the owner wants forward progress without rubber-stamp checkpoints. An external reviewer (GPT-5.5) catches what a single author misses, while Claude's adjudication guards against the reviewer's false positives — demonstrated necessary the same day: a codex-class review's confident "two static tables, not three" claim was wrong (all three ladders are live). Commits are low-risk and reversible (nothing publishes until the owner pushes), so per-commit gating was pure friction.
   107	- Supersedes: nothing. ~~Scopes `.review/` usage for `ue-r2-*` only~~ **(scope clause superseded by D-2026-07-04-1 — the loop is now repo-wide for all code and plan changes)** — the async sentinel (`ready/`) + `reviewer-wait.sh` hand-off is not used (records `findings/` + `results/` are reused). Records the owner's explicit relaxation of the §9 per-slice-code checkpoint (code acceptance delegated to this loop); the §8 push gate and all other §9 owner gates stand.
   108	
   109	## D-2026-07-04-1 — Codex review loop for ALL code and plan changes; async sentinel loop retired
   110	- Decision: The synchronous code→codex-review→fix loop (`docs/agent/GPT_REVIEW_LOOP.md`) now governs **every code change and every plan change** in this repo — owner, 2026-07-04: "use codex review loop for all code and plan changes", "NO EXCEPTIONS". The `.review/README.md` async two-agent hand-off (`ready/` sentinels + `reviewer-wait.sh` + a separate reviewer agent) is retired as the grading mechanism for new work; its record formats (`.review/findings/`, `.review/results/`, the `REVIEW.md` status index) remain in use by the codex loop. Reviewer identity on verdicts: `gpt-5.5` (codex), adjudicated by the coding agent per the loop's adjudication step. For docs/plan-only changes the validation gate is `bash scripts/agent/check-docs.sh` (the cargo suite is not required, per `.agents/repo-guidance.md` Verification); the review step still runs.
   111	- Why: the codex loop demonstrably catches real defects (every `ue-r2-*` slice) while the async reviewer role sat structurally unfilled — w4-1 landed 2026-07-04 and immediately stalled at "awaiting reviewer verdict" with no reviewer in existence; a review mechanism that actually runs beats one that waits for an agent nobody spawns.
   112	- Supersedes: the scope clause of D-2026-06-20-6 ("Scopes `.review/` usage for `ue-r2-*` only" — the loop is now repo-wide; D-2026-06-20-6's standing authorizations (a)/(b)/(c) carry over unchanged to the widened scope). Also supersedes `.review/README.md`'s sentinel/reviewer-wake sections and `docs/agent/PROTOCOL.md` `slice` step 2's sentinel requirement (both edited in place, annotated).
   113	
   114	## D-2026-07-04-2 — Keep the `9f37a7a`/`48c5a11` staging-slip commits; no history rewrite
   115	- Decision: The two Windows-session commits that don't build in isolation (`9f37a7a` clippy baseline carrying a stray `pull.rs` deletion, `48c5a11` win-1) stay on `master` as pushed; no rebase, no force-push. `git bisect` runs must skip them (both are documented in the ue-r2-1h finding doc and DEVLOG). This closes the erratum question opened 2026-07-04.
   116	- Why: owner call 2026-07-04 ("leave as-is"). HEAD is fully gated and every later commit builds; the only cost is two skippable commits in bisect. Rewriting already-pushed shared history is the riskier operation — same calculus as D-2026-06-07-1, which is this repo's precedent for keeping a pushed wart over a second unsafe git operation.
   117	- Supersedes: nothing (closes the STATE.md "commit erratum" blocked item).
   118	
   119	## D-2026-07-04-3 — Flip `supports_cancellation` for Push/PullSync: CancelJob works on attached transfers
   120	- Decision: The `CancelJob` dispatch policy stops refusing attached Push/PullSync jobs. After the flip, `blit jobs cancel` (and the TUI F2 cancel) fires the row's cancel token for those kinds and the handlers — which race that token since w4-3 — tear down cleanly; the CLI contract changes from exit 2 / `FailedPrecondition` ("unsupported") to exit 0 on success, and the TUI's Unsupported surface for these kinds disappears. Implementation is a queued review-loop slice (`w4-5-supports-cancellation-flip` in REVIEW.md) through the codex loop, with tests pinning the new contract.
   121	- Why: owner call 2026-07-04 ("flip it"). The original "disconnect is the cancel" rationale predates w4-3's race wiring; the flip is now policy-only, and cancel-from-anywhere (second terminal, TUI) is strictly more operable than find-and-kill-the-client.
   122	- Supersedes: the DelegatedPull-only cancellation policy recorded in `active_jobs.rs`'s `supports_cancellation` rustdoc (edited when the slice lands) and the corresponding "policy deliberately unchanged" scope note in the w4-3 finding doc (which anticipated exactly this flip).
   123	
   124	## D-2026-07-04-4 — SMALL_FILE_CEILING.md flipped Draft → Active
   125	- Decision: `docs/plan/SMALL_FILE_CEILING.md` is **Active** (owner "go", 2026-07-04). sf-1 (tripwire harness) starts now; the in-plan gates stand unchanged — sf-6's wire-design owner sign-off before any code, and the sf-4/sf-7 acceptance reviews with the owner.
   126	- Why: the codex plan review is complete (5/5 accepted + fixed, records `219cecf`) and the plan binds the measured small-file/mixed ceiling gaps (`docs/bench/10gbe-2026-07-05/`) to the owner's ceiling-driven principle. The other four 10 GbE gate declarations (ue-1, ue-2, zero-copy a/b/c, REV4 → Shipped) were NOT part of this go and stay in STATE.md Blocked.
   127	- Supersedes: nothing (the plan's "(pending owner approval)" decision ref now points here).
   128	
   129	## D-2026-07-05-1 — One transfer path; direction-invariance by construction; SMALL_FILE_CEILING paused
   130	- Decision: All byte transfer in blit must flow through ONE `TransferSession` implementation — direction, initiator, and CLI verb select *roles* (SOURCE/DESTINATION), never code. The per-direction drivers (client push driver, daemon push-receive, client pull driver, daemon pull-send, delegated-pull driver, separate local orchestration) and the `Push`/`PullSync` RPCs are deleted when the migration completes — owner, 2026-07-05: "ONE BLOCK OF CODE that does the transfer. no POSSIBILITY OF ANYTHING EVER using anything else because anything else does not exist"; "I NEVER see a situation where pull is faster than push or vice versa... because of something blit did." Benchmark methodology corollary: cross-direction performance comparisons are valid only on symmetric endpoints ("tmp on one side, spinning rust on the other is not a valid test"); tmpfs cells are wire-reference only. Plan: `docs/plan/ONE_TRANSFER_PATH.md` (Draft; no code until the owner flips it Active). `docs/plan/SMALL_FILE_CEILING.md` is **paused** effective immediately at sf-2 (done): sf-3a and later slices are blocked until ONE_TRANSFER_PATH ships, then resume/re-derive against the unified baseline (owner delegated this sequencing: "I DO NOT CARE. FIX IT.").
   131	- Why: the measured push/pull disparity recurred because direction symmetry was discipline spread across four driver loops, not structure — the sf-2 stream-count bug existed only in the push driver, the slow-start defect only in the pull driver. Deleting the alternatives is the only arrangement in which the owner's invariant cannot regress.
   132	- Supersedes: the post-REV4 residue item "pull 1s-start restructuring" (STATE Queue item 4 — absorbed by ONE_TRANSFER_PATH's streaming-manifest choreography); SMALL_FILE_CEILING's queue position (paused, not superseded — its principle D-2026-07-04-4 stands).

codex
I’m doing one last pass on the concrete path names and deletion list, because the invariant depends on proving there are no side-door drivers left after cutover.
exec
/usr/bin/zsh -lc "rg --files | rg '("'^|/)(pull'"\\.rs|pull_sync\\.rs|push/control\\.rs|remote/push|remote/pull|delegat|orchestrator)'" in /home/michael/dev/Blit
 succeeded in 0ms:
crates/blit-daemon/src/delegation_gate.rs
crates/blit-daemon/src/service/pull_sync.rs
crates/blit-daemon/src/service/delegated_pull.rs
crates/blit-daemon/src/service/push/control.rs
crates/blit-core/src/remote/pull.rs
crates/blit-core/src/remote/push/payload.rs
crates/blit-core/src/remote/push/client/mod.rs
crates/blit-core/src/remote/push/client/types.rs
crates/blit-core/src/remote/push/client/helpers.rs
crates/blit-core/src/remote/push/mod.rs
crates/blit-core/src/remote/push/data_plane.rs
crates/blit-core/src/orchestrator/orchestrator.rs
crates/blit-core/src/orchestrator/mod.rs

exec
/usr/bin/zsh -lc "rg -n \"Push|PullSync|rpc |Transfer\\(|service Blit|Delegated|CancelJob|read-only|readonly|read_only\" proto blit-core blit-daemon blit-cli blit-app blit-tui" in /home/michael/dev/Blit
 exited 2 in 0ms:
rg: blit-core: No such file or directory (os error 2)
rg: blit-daemon: No such file or directory (os error 2)
rg: blit-cli: No such file or directory (os error 2)
rg: blit-app: No such file or directory (os error 2)
rg: blit-tui: No such file or directory (os error 2)
proto/blit.proto:5:service Blit {
proto/blit.proto:6:  // Push uses a bidirectional stream for an efficient "check-then-send" workflow.
proto/blit.proto:7:  rpc Push(stream ClientPushRequest) returns (stream ServerPushResponse);
proto/blit.proto:9:  // Removed 2026-07-03 (ue-r2-1h): rpc Pull(PullRequest) returns
proto/blit.proto:11:  // Superseded whole by PullSync; the relay client's metadata scan and
proto/blit.proto:12:  // single-file streaming moved to PullSync sessions
proto/blit.proto:16:  // and DataTransferNegotiation survive — PullSync and push share them.
proto/blit.proto:20:  rpc PullSync(stream ClientPullMessage) returns (stream ServerPullMessage);
proto/blit.proto:23:  rpc List(ListRequest) returns (ListResponse);
proto/blit.proto:26:  rpc Purge(PurgeRequest) returns (PurgeResponse);
proto/blit.proto:29:  rpc CompletePath(CompletionRequest) returns (CompletionResponse);
proto/blit.proto:32:  rpc ListModules(ListModulesRequest) returns (ListModulesResponse);
proto/blit.proto:35:  rpc Find(FindRequest) returns (stream FindEntry);
proto/blit.proto:38:  rpc DiskUsage(DiskUsageRequest) returns (stream DiskUsageEntry);
proto/blit.proto:41:  rpc FilesystemStats(FilesystemStatsRequest) returns (FilesystemStatsResponse);
proto/blit.proto:53:  rpc DelegatedPull(DelegatedPullRequest) returns (stream DelegatedPullProgress);
proto/blit.proto:62:  rpc GetState(GetStateRequest) returns (DaemonState);
proto/blit.proto:86:  rpc CancelJob(CancelJobRequest) returns (CancelJobResponse);
proto/blit.proto:99:  rpc ClearRecent(ClearRecentRequest) returns (ClearRecentResponse);
proto/blit.proto:117:  rpc Subscribe(SubscribeRequest) returns (stream DaemonEvent);
proto/blit.proto:165:  // support (PushHeader.supports_stream_resize or
proto/blit.proto:266:// Push Operation
proto/blit.proto:267:message ClientPushRequest {
proto/blit.proto:269:    PushHeader header = 1;
proto/blit.proto:286:message ServerPushResponse {
proto/blit.proto:290:    PushSummary summary = 3;
proto/blit.proto:299:message PushHeader {
proto/blit.proto:306:  bool force_grpc = 4;
proto/blit.proto:335:  // mid-transfer stream resize. Push opens with PushHeader (it does
proto/blit.proto:371:  // safely purge in mirror mode — see PushHeader.require_complete_scan.
proto/blit.proto:380:// Acknowledgment for PullSync with server capabilities
proto/blit.proto:381:message PullSyncAck {
proto/blit.proto:385:message PushSummary {
proto/blit.proto:396:// because ServerPullMessage and DelegatedPullProgress reuse them.)
proto/blit.proto:410:// PullSync Operation (bidirectional pull with manifest comparison).
proto/blit.proto:413:// previous `PullSyncHeader` bool-soup shape was removed entirely
proto/blit.proto:457:    PullSyncAck pull_sync_ack = 8;            // Acknowledgment with server capabilities
proto/blit.proto:470:    // shape as ClientPushRequest's tar_shard_* — the daemon's
proto/blit.proto:518:    bool read_only = 3;
proto/blit.proto:632:  bool force_grpc = 9;
proto/blit.proto:655:  // receives the bytes). Push ignores this field; there the receiver
proto/blit.proto:674:  // PullSync now that the deprecated Pull RPC (whose PullRequest had
proto/blit.proto:675:  // the same flag) is gone. Only valid on a direct PullSync session:
proto/blit.proto:790:  // equivalent is PushHeader.supports_stream_resize (push does not
proto/blit.proto:796:// Delegated Pull — direct daemon-to-daemon byte path for remote→remote
proto/blit.proto:800:message DelegatedPullRequest {
proto/blit.proto:839:  // failure, or `CancelJob(transfer_id)`. The CLI is free to
proto/blit.proto:845:  // Only valid on `DelegatedPull` (the daemon-to-daemon byte
proto/blit.proto:846:  // path). Push / pull_sync put the CLI in the byte
proto/blit.proto:869:message DelegatedPullProgress {
proto/blit.proto:871:    DelegatedPullStarted started = 1;
proto/blit.proto:874:    DelegatedPullSummary summary = 4;
proto/blit.proto:875:    DelegatedPullError   error = 5;
proto/blit.proto:879:message DelegatedPullStarted {
proto/blit.proto:904:message DelegatedPullSummary {
proto/blit.proto:918:message DelegatedPullError {
proto/blit.proto:967:  // accept inbound DelegatedPull requests.
proto/blit.proto:1037:// CancelJob — fire the cancellation token of an active transfer.
proto/blit.proto:1041:message CancelJobRequest {
proto/blit.proto:1045:message CancelJobResponse {

codex
I have enough for the review. I’m pulling exact line references for the findings so the result can stay short and actionable.
exec
/usr/bin/zsh -lc "nl -ba docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md | sed -n '60,115p'" in /home/michael/dev/Blit
 succeeded in 0ms:
    60	  **accurately** (REV3 under-counted them — see Current Code Reality).
    61	
    62	## Non-goals
    63	
    64	- No ground-up transfer rewrite.
    65	- No zero-copy receive revival (D-2026-06-12-1; revisit gated on the
    66	  10 GbE benchmarks).
    67	- No H10b merger. The engine's workload-shape-aware planner and 1s start
    68	  requirement stand on their own; D-2026-06-04-3 remains queued after
    69	  audit Round 1.
    70	- The **gRPC fallback path stays single-logical-stream by design**
    71	  (unchanged from w2-3's non-goal). "Pull is single-stream today" below
    72	  is about PullSync's TCP data plane, not this fallback.
    73	- No coding during this review.
    74	
    75	## Constraints
    76	
    77	- **FAST**: bytes begin quickly, stream scheduling avoids slow-sink
    78	  head-of-line blocking, tuning comes from measured telemetry, and small
    79	  local transfers keep their low-overhead path.
    80	- **SIMPLE**: there is one transfer behavior owner. Local fast paths,
    81	  push negotiation, pull sync, and delegated transfers become strategies
    82	  or inputs under the engine, not separate operator-visible models.
    83	- **RELIABLE**: mirror/delete safety, scan-completeness checks, resume,
    84	  StallGuard behavior, cancellation, byte-progress accounting, and
    85	  byte-identical transfer tests cannot regress.
    86	- Wire changes are allowed (proto unfrozen, D-2026-06-11-1), but mixed
    87	  old/new peers must negotiate down to today's behavior. New fields are
    88	  advisory until both peers advertise support.
    89	- The 1370-test baseline must not drop.
    90	- Windows parity remains required unless a test is genuinely platform
    91	  specific.
    92	- **Every stage serves FAST, SIMPLE, or RELIABLE** — a purely structural
    93	  change with no goal payoff is out.
    94	
    95	## Acceptance Criteria
    96	
    97	- [ ] A single `TransferEngine` (or owner-approved equivalent) is the
    98	      public transfer sequencer for local↔local, push, pull, and
    99	      delegated daemon↔daemon transfers.
   100	- [ ] Existing local fast paths are either engine-owned strategies
   101	      (their real names: `journal_no_work`, `no_work`, `tiny_manifest`,
   102	      `single_huge_file`, and the single-file copy shortcut at
   103	      `orchestrator.rs:178`) or explicitly deleted by owner decision. No
   104	      local path bypasses the transfer behavior owner by accident.
   105	- [ ] **The three static code-level stream/dial ladders plus the
   106	      negotiated proto field are replaced by one dial source** (corrected
   107	      against code — see Current Code Reality). Concretely, the dial
   108	      subsumes:
   109	      1. `remote/tuning.rs::determine_remote_tuning` (size-keyed
   110	         `initial_streams`/`max_streams`/`chunk_bytes`/`tcp_buffer_size`/
   111	         `prefetch_count`; the *client's* ladder, consumed by push and by
   112	         the daemon pull paths);
   113	      2. `blit-daemon .../push/control.rs::desired_streams` (the daemon
   114	         push-negotiation ladder, already keyed on **file count** as well
   115	         as bytes — the daemon's ladder "wins" per `tuning.rs`'s own doc

exec
/usr/bin/zsh -lc "nl -ba docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md | sed -n '318,360p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   318	### 4. Dial and bounded-unilateral negotiation
   319	
   320	The byte sender owns the live dial, bounded by the byte receiver's
   321	capacity profile. The profile travels from receiver to sender during
   322	setup:
   323	
   324	- push: destination daemon advertises receiver capacity to the push
   325	  client;
   326	- pull: pull client advertises receiver capacity in the pull spec or
   327	  setup message so the source daemon can send within it;
   328	- delegated: destination daemon advertises its receiver capacity when it
   329	  asks the source daemon to send.
   330	
   331	The rich capacity profile should include, at minimum: CPU cores
   332	available to the transfer; disk class or drain class; current load
   333	estimate; maximum accepted streams; drain-rate estimate; max safe chunk
   334	bytes; max safe prefetch / in-flight bytes. (Owner: more data serves the
   335	ubergoal; do not minimize the negotiation payload.)
   336	
   337	The initial dial starts **below** the profile ceiling with margin.
   338	Telemetry can increase or decrease the cheap dials (chunk, prefetch, TCP
   339	buffers) immediately; stream-count changes arrive in the later resize
   340	slice. The dial is a **mutable object read by both ends from the
   341	live-dials slice onward** — this is what makes continuous (`ue-r2-2`) a
   342	wire-up rather than a retrofit (see the C-ready acceptance criterion).
   343	
   344	### 5. Wire contract before dependent code
   345	
   346	REV4 makes wire shape an early deliverable. Proposed proto direction:
   347	
   348	- append `CapacityProfile receiver_capacity = 11` to
   349	  `DataTransferNegotiation` (field 11 is the first free number after the
   350	  5–10 RDMA reservation);
   351	- add a capacity profile to the request/setup side where the receiver is
   352	  the client, especially PullSync and delegated pull;
   353	- add explicit peer capability bits/fields so resize messages are never
   354	  sent to an old peer;
   355	- add `DataPlaneResize` and `DataPlaneResizeAck` as negotiated control
   356	  messages in the relevant control streams, not as blind TCP data-plane
   357	  records.
   358	
   359	Exact field names and numbers are part of the wire slice acceptance
   360	criteria. Old peers must see current behavior: no capacity profile means

exec
/usr/bin/zsh -lc "nl -ba proto/blit.proto | sed -n '1,60p'" in /home/michael/dev/Blit
 succeeded in 0ms:
     1	syntax = "proto3";
     2	package blit.v2;
     3	
     4	// The main service for all data transfer and remote management operations.
     5	service Blit {
     6	  // Push uses a bidirectional stream for an efficient "check-then-send" workflow.
     7	  rpc Push(stream ClientPushRequest) returns (stream ServerPushResponse);
     8	
     9	  // Removed 2026-07-03 (ue-r2-1h): rpc Pull(PullRequest) returns
    10	  // (stream PullChunk) — the deprecated server-streaming pull.
    11	  // Superseded whole by PullSync; the relay client's metadata scan and
    12	  // single-file streaming moved to PullSync sessions
    13	  // (TransferOperationSpec.metadata_only / a single-file force_grpc
    14	  // spec). PullRequest/PullChunk were deleted with it (they had no
    15	  // other referents); PullSummary, ManifestBatch, FileHeader, FileData,
    16	  // and DataTransferNegotiation survive — PullSync and push share them.
    17	
    18	  // Bidirectional pull with manifest comparison for selective transfers.
    19	  // Client sends local manifest, server compares and sends only needed files.
    20	  rpc PullSync(stream ClientPullMessage) returns (stream ServerPullMessage);
    21	
    22	  // Lists contents of a remote directory.
    23	  rpc List(ListRequest) returns (ListResponse);
    24	
    25	  // Deletes files/directories on the server for mirror operations.
    26	  rpc Purge(PurgeRequest) returns (PurgeResponse);
    27	
    28	  // Provides path completion suggestions for a given remote path prefix.
    29	  rpc CompletePath(CompletionRequest) returns (CompletionResponse);
    30	
    31	  // Lists the available modules on the server.
    32	  rpc ListModules(ListModulesRequest) returns (ListModulesResponse);
    33	
    34	  // Recursively finds files/directories starting at a module path.
    35	  rpc Find(FindRequest) returns (stream FindEntry);
    36	
    37	  // Summarises disk usage for a subtree (du-style).
    38	  rpc DiskUsage(DiskUsageRequest) returns (stream DiskUsageEntry);
    39	
    40	  // Reports module/storage capacity information (df-style).
    41	  rpc FilesystemStats(FilesystemStatsRequest) returns (FilesystemStatsResponse);
    42	
    43	  // Destination-side delegated initiator. The CLI calls this on the
    44	  // destination daemon when both endpoints in a `blit copy` are
    45	  // remote. The destination daemon validates the request through the
    46	  // delegation gate, opens its own pull against the named source, and
    47	  // streams progress/results back to the CLI. Bytes flow source→dst
    48	  // directly; the CLI is not in the byte path.
    49	  //
    50	  // See docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md for the design
    51	  // (gate ordering, allowlist semantics, client_capabilities
    52	  // override boundary).
    53	  rpc DelegatedPull(DelegatedPullRequest) returns (stream DelegatedPullProgress);
    54	
    55	  // Daemon-state snapshot for the TUI's F1 (Daemons) and F2
    56	  // (Transfers) panes, plus `blit jobs list <remote>`. Always
    57	  // available regardless of `--metrics`; see §6.4 of
    58	  // docs/plan/TUI_DESIGN.md. Counters read from the
    59	  // TransferMetrics atomics (so they're zero when the flag is
    60	  // off), but active[] / recent[] always populate from the

exec
/usr/bin/zsh -lc "nl -ba proto/blit.proto | sed -n '790,880p'" in /home/michael/dev/Blit
 succeeded in 0ms:
   790	  // equivalent is PushHeader.supports_stream_resize (push does not
   791	  // carry PeerCapabilities).
   792	  bool supports_stream_resize = 5;
   793	}
   794	
   795	// ─────────────────────────────────────────────────────────────────────
   796	// Delegated Pull — direct daemon-to-daemon byte path for remote→remote
   797	// transfers. See docs/plan/REMOTE_REMOTE_DELEGATION_PLAN.md.
   798	// ─────────────────────────────────────────────────────────────────────
   799	
   800	message DelegatedPullRequest {
   801	  // ── Destination-side fields (this daemon) ─────────────────────────
   802	  // Module on this (destination) daemon to write into. Resolved
   803	  // through the same module table push uses; F2 canonical-path
   804	  // containment applies to dst_destination_path.
   805	  string dst_module = 1;
   806	  string dst_destination_path = 2;
   807	
   808	  // ── Source-side / universal contract ──────────────────────────────
   809	  // Where this daemon should pull from. Strictly typed; arbitrary URI
   810	  // schemes are rejected during locator parsing.
   811	  RemoteSourceLocator src = 10;
   812	
   813	  // The universal transfer contract. Reuses TransferOperationSpec
   814	  // including FilterSpec, ComparisonMode, MirrorMode, ResumeSettings,
   815	  // force_grpc, ignore_existing, and spec_version. Receivers normalize
   816	  // via NormalizedTransferOperation::from_spec exactly like push/pull.
   817	  //
   818	  // OVERRIDE BOUNDARY: spec.client_capabilities describes the byte
   819	  // recipient's capabilities. In delegation, the byte recipient is
   820	  // the destination daemon, not the CLI. The destination handler
   821	  // mandatorily REPLACES client_capabilities with its own
   822	  // PeerCapabilities before forwarding the spec to the source —
   823	  // any CLI-supplied value here is non-authoritative. Every other
   824	  // field flows through unchanged.
   825	  TransferOperationSpec spec = 20;
   826	
   827	  // Diagnostics-only.
   828	  bool trace_data_plane = 31;
   829	
   830	  // Detach the transfer from the calling client.
   831	  //
   832	  // When false (the historical behavior), the destination
   833	  // daemon races the transfer against `tx.closed()` so a CLI
   834	  // disconnect drops the in-flight pull future and the data
   835	  // plane tears down (R30-F2).
   836	  //
   837	  // When true, the `tx.closed()` race disarms: the
   838	  // destination daemon owns the transfer through completion,
   839	  // failure, or `CancelJob(transfer_id)`. The CLI is free to
   840	  // exit immediately after the daemon's `Started` event
   841	  // arrives — useful for long-running remote→remote
   842	  // transfers that should outlive the operator's shell
   843	  // session.
   844	  //
   845	  // Only valid on `DelegatedPull` (the daemon-to-daemon byte
   846	  // path). Push / pull_sync put the CLI in the byte
   847	  // path — detach is meaningless there because the CLI
   848	  // disconnecting drops the bytes. The CLI rejects `--detach`
   849	  // for those routes; the daemon-side flag is therefore
   850	  // unused on non-delegated kinds.
   851	  bool detach = 32;
   852	}
   853	
   854	message RemoteSourceLocator {
   855	  // Strict Blit remote endpoint. The daemon parses this through the
   856	  // same RemoteEndpoint code the CLI uses; rejects schemes other than
   857	  // the Blit gRPC control-plane scheme.
   858	  string host = 1;
   859	  uint32 port = 2;
   860	
   861	  // Field 10 was `bytes delegated_credential` (forward-compat
   862	  // auth-passthrough hook for the removed BlitAuth service).
   863	  // Removed 2026-05-13 — auth is out of project scope. Reserved
   864	  // so it can't be reused for unrelated semantics by accident.
   865	  reserved 10;
   866	  reserved "delegated_credential";
   867	}
   868	
   869	message DelegatedPullProgress {
   870	  oneof payload {
   871	    DelegatedPullStarted started = 1;
   872	    ManifestBatch        manifest_batch = 2; // existing message reuse
   873	    BytesProgress        bytes_progress = 3;
   874	    DelegatedPullSummary summary = 4;
   875	    DelegatedPullError   error = 5;
   876	  }
   877	}
   878	
   879	message DelegatedPullStarted {
   880	  // Diagnostic: the source-side data-plane endpoint this daemon's

codex
- **[High] [ONE_TRANSFER_PATH §Non-goals](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:46) vs [REV4 §Constraints](/home/michael/dev/Blit/docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:86)**: The draft says pre-plan wire compatibility is a non-goal and `Push`/`PullSync` are deleted at lockstep cutover, but Active REV4 still requires mixed old/new peers to negotiate down. D-2026-07-05-1 does not explicitly supersede that REV4 compatibility rule, so two authorities conflict.

- **[Medium] [ONE_TRANSFER_PATH §Constraints/Design](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:73) vs [REV4 §Dial](/home/michael/dev/Blit/docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:318)**: The plan makes sf-2 shape-correction “the one and only stream policy” but does not pin the bounded-unilateral dial contract: receiver capacity profile, rich profile fields, and sender-owned dial within receiver ceiling. That is required by D-2026-06-20-1/-2 and should be in otp-1/acceptance.

- **[Medium] [ONE_TRANSFER_PATH §Acceptance/Slices](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:95)**: The deletion proof omits the current `DelegatedPull` RPC even though [proto/blit.proto](/home/michael/dev/Blit/proto/blit.proto:43) describes it as a daemon-to-daemon byte path. If it remains only as an auth/control gate, say so and prove it carries no bytes; otherwise include it in the deletion proof. Also align the remote deletion slice with local orchestration deletion in otp-10.

- **[Medium] [ONE_TRANSFER_PATH §Constraints/Acceptance](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:65)**: “Converge up, not down” is a constraint but not an observable acceptance criterion. The final checklist only proves initiator/verb invariance; a symmetric-but-slower unified path could pass. Add a symmetric pre-cutover baseline and require final cells to match the better same-rig direction within noise.

- **[Medium] [ONE_TRANSFER_PATH §Design](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:137)**: Resume ordering is underspecified. The choreography opens the data plane immediately, then names resume as an optional block-hash phase, but does not state when destination block maps/partial-file validation must complete or whether resume is a RELIABLE exception to immediate send. otp-1/otp-6 should pin ordering and stale-partial failure cases.

VERDICT: NEEDS FIXES before Active flip. No quiet re-litigation found for the ceiling principle, codex loop, or zero-copy decision; the main issues are unresolved REV4 compatibility/dial carryover and missing proof points around deletion and acceptance.
tokens used
88,495
- **[High] [ONE_TRANSFER_PATH §Non-goals](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:46) vs [REV4 §Constraints](/home/michael/dev/Blit/docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:86)**: The draft says pre-plan wire compatibility is a non-goal and `Push`/`PullSync` are deleted at lockstep cutover, but Active REV4 still requires mixed old/new peers to negotiate down. D-2026-07-05-1 does not explicitly supersede that REV4 compatibility rule, so two authorities conflict.

- **[Medium] [ONE_TRANSFER_PATH §Constraints/Design](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:73) vs [REV4 §Dial](/home/michael/dev/Blit/docs/plan/UNIFIED_TRANSFER_ENGINE_REV4.md:318)**: The plan makes sf-2 shape-correction “the one and only stream policy” but does not pin the bounded-unilateral dial contract: receiver capacity profile, rich profile fields, and sender-owned dial within receiver ceiling. That is required by D-2026-06-20-1/-2 and should be in otp-1/acceptance.

- **[Medium] [ONE_TRANSFER_PATH §Acceptance/Slices](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:95)**: The deletion proof omits the current `DelegatedPull` RPC even though [proto/blit.proto](/home/michael/dev/Blit/proto/blit.proto:43) describes it as a daemon-to-daemon byte path. If it remains only as an auth/control gate, say so and prove it carries no bytes; otherwise include it in the deletion proof. Also align the remote deletion slice with local orchestration deletion in otp-10.

- **[Medium] [ONE_TRANSFER_PATH §Constraints/Acceptance](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:65)**: “Converge up, not down” is a constraint but not an observable acceptance criterion. The final checklist only proves initiator/verb invariance; a symmetric-but-slower unified path could pass. Add a symmetric pre-cutover baseline and require final cells to match the better same-rig direction within noise.

- **[Medium] [ONE_TRANSFER_PATH §Design](/home/michael/dev/Blit/docs/plan/ONE_TRANSFER_PATH.md:137)**: Resume ordering is underspecified. The choreography opens the data plane immediately, then names resume as an optional block-hash phase, but does not state when destination block maps/partial-file validation must complete or whether resume is a RELIABLE exception to immediate send. otp-1/otp-6 should pin ordering and stale-partial failure cases.

VERDICT: NEEDS FIXES before Active flip. No quiet re-litigation found for the ceiling principle, codex loop, or zero-copy decision; the main issues are unresolved REV4 compatibility/dial carryover and missing proof points around deletion and acceptance.
