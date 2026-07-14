Reading additional input from stdin...
OpenAI Codex v0.144.3
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: ultra
reasoning summaries: none
session id: 019f5f8d-d302-7030-9519-1fe2ea8bd239
--------
user
Review the diff of commit e1e351d (run: git show e1e351d), focusing on the NEW BENCHMARK HARNESS scripts/bench_otp12pf_mac.sh. It implements the pre-registered design in docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md (rev 2, same commit) -- read that first: the harness must compute exactly what that rule is graded on.

THE INSTRUMENT IS THE RISK. This project has retracted three separate claims that all traced to a benchmark harness that lied while looking correct: a durability sync charged to only ONE arm, which MANUFACTURED the very invariance failure under test; a throughput probe that reported "6787 MB/s CONFIRMED" from a transfer that moved ZERO bytes; and a ps %CPU decaying average that read a FINISHED backup as 255%. Assume this harness lies until proven otherwise.

Check, hardest first:

1. ARM SYMMETRY. The ONLY difference between the srcinit and destinit arms must be which host's CLI initiates. Trace both arms end to end: arm_srcinit, arm_destinit, timed_run, fsync_tree_ms, prep_run. Does either arm pay a cost the other does not? Specifically the destination-keyed fsync walk: each arm passes a DIFFERENT "landed" path, because blit uses rsync-style slash semantics -- a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>, while a pull into RUNDIR lands the files directly IN RUNDIR. If either landed path is wrong or missing, os.walk silently walks nothing and returns 0 ms, so that arm pays ZERO durability while the other pays full -- exactly the otp-2w bug. Is the timing bracket equivalent for a LOCAL bash -c initiator vs a REMOTE ssh initiator?

2. DOES IT MEASURE NOTHING? Could any step silently succeed while moving no bytes, or fsync an empty or absent directory? Is a failed transfer, a failed purge, a failed drain, or an empty fsync distinguishable from a genuinely fast one? Is the pair-void logic actually reachable, and does anything fail OPEN?

3. FIDELITY TO THE PRE-REGISTERED RULE. Paired stats: d_i per ABBA slot, D = median, S = spread. The POWER GATE: powered_for_null. The integer-exact 1.10 bar. All runs printed. Is the slot pairing correct given that a voided pair RE-RUNS the slot and the csv "run" column is the slot number -- can a stale or duplicate row corrupt d_i? Any off-by-one, wrong median convention, or wrong sign on d_i?

4. GATES: quiescence, Time Machine fail-closed on running OR merely enabled, Spotlight, load1 over 3.0, purge, provenance clean +sha and reject +sha.dirty, stale daemon, fixtures by count, ARP peer MAC. Are any fail-OPEN -- a gate whose command errors and is swallowed, an inverted grep exit code, or a value that parses to empty and compares as pass? Does the daemon-listening check actually prove it ACCEPTS connections?

5. Shell correctness: quoting through hrun's bash -c versus ssh -- the SAME string must behave identically both ways. set -euo pipefail interactions, the trap/cleanup path, CRLF, and anything that differs between the local and the remote host.

Output a concise markdown findings list -- each finding with file:line, severity, rationale -- then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll audit the preregistered rule first, then trace both arms and independently check statistics, gates, and shell/transport behavior. I’ll treat silent success and asymmetric timing as presumptive defects until the script proves otherwise.
exec
/bin/zsh -lc "sed -n '1,260p' docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
sed -n '1,220p' AGENTS.md
sed -n '1,260p' .agents/repo-guidance.md
sed -n '1,220p' .agents/state.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered, **revision 2**. **No data exists yet.**
Codex round 1 (of `f0343f4`): **NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW → 9/9
accepted.** Adjudication: `.review/results/macmac-prereg.gpt-verdict.md`.
Committed BEFORE the data so the decision rule cannot be authored around the
numbers (the pf-0 discipline).

**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1).

## What revision 1 got WRONG, and what this experiment actually answers

Revision 1 claimed this rig "discriminates H1 outright": *P1 reproduces
macOS↔macOS ⇒ H1 dies, because H1 accuses the Windows accept branch.*

**That inference is invalid, and the premise is false.** H1, verbatim in the
parent, accuses **blit's own code paths** — the `SourceSockets` Dial/Accept
branches, `InitiatorReceivePlaneRun.add_dialed_stream`, and the destination's
synchronous dial-before-ACK at `transfer_session/mod.rs:3113`. **The word
"Windows" appears nowhere in H1.** Windows is merely *who happens to be the
accepting source* in P1's slow arm on rig W. The accused code runs on macOS too.
So a Mac↔Mac reproduction is **consistent with H1**, not fatal to it — and the
parent already warns that *"'consistent with H1' is not confirmation."*

The bad framing was inherited from `docs/STATE.md` ("H1 accuses the *Windows*
accept branch") and copied without checking H1's text. **That is a repo error and
it is corrected wherever it appears.**

**What this rig CAN answer — and it is still decision-relevant:**

> **Does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost
> of the destination-initiated layout?**

| outcome | what it licenses |
|---|---|
| **P1 REPRODUCES macOS↔macOS** | The failure needs **no Windows peer**. P1 is **not platform residue** — it is a cost of the layout/code that survives with the Windows half removed. This **closes the "accept it as platform residue" escape** (the D-2026-07-12-1 shape) and **strengthens every code-level hypothesis, H1 included**. It does **not** name the mechanism. |
| **P1 VANISHES macOS↔macOS** | The failure **requires the Windows peer**: it is pairing-dependent / platform-interacting. Code-only mechanisms that should bite on any OS are **weakened**; a Windows-specific cost, or a macOS↔Windows interaction, rises. It does **not** confirm H1 — H1's accept branch would then have to be *platform-conditionally* slow, which is a further claim needing pf-1's counterfactual. |

Either outcome materially reshapes the hypothesis space and bears directly on
whether P1 **must be fixed in code** or **could be accepted as platform residue**.
That is why it runs before pf-1. **It is not an H1 kill/confirm and this document
must never be cited as one.**

## Rig

- **nagatha** (owner's workstation): 10GbE `en11` = **10.1.10.92**, MTU 9000.
- **`q`** (M4 mini, dedicated bench Mac): 10GbE `en8` = **10.1.10.54**, MTU 9000.
- **Build**: `f35702a`, **clean `+f35702a`** on all four binaries (the `.dirty`
  form is rejected) — the cutover sha behind every P1 measurement (12b/12c,
  q-baseline, pf-0). HEAD is **not** code-identical to it, so the pin is
  deliberate.

**Endpoint asymmetry does NOT simply "cancel" (round-1 HIGH).** Revision 1 claimed
it did. It does not: switching the initiator also **reassigns which machine runs
the CLI and which runs the daemon**, and `q` is the faster Mac. Only
arm-independent costs cancel; **host×role interactions do not.** This is handled
by *measuring both data directions and reporting them separately* (below), not by
assertion — and any conclusion that depends on the cancellation being perfect is
out of bounds.

## Cells

Grammar `<nq|qn>_<carrier>_<fixture>`: `nq_*` = data **nagatha→q**, `qn_*` = data
**q→nagatha**. Arms — the only variable — are `srcinit` (source's CLI pushes) and
`destinit` (dest's CLI pulls).

    CELLS = nq_tcp_mixed,  qn_tcp_mixed     <- THE MEASURAND (P1's shape)
            nq_grpc_mixed, qn_grpc_mixed    <- carrier control (P1 is TCP-only)
            nq_tcp_large,  qn_tcp_large     <- fixture control (P1 is mixed-only)

`RUNS=8`, ABBA-counterbalanced, pair-void.

**Both directions are measured, but a reproduction is NOT required in both
(round-1 HIGH).** P1's recorded signature on rig W is **one-directional**:
`wm_tcp_mixed` FAILS while `mw_tcp_mixed` PASSES. Demanding failure in both
directions here would rewrite the finding. So: **a reproduction in EITHER
direction demonstrates the layout cost without a Windows peer.** Whether it is
direction-symmetric is reported as a descriptive fact — and, because the two
directions differ in *which machine is the destination*, a one-directional result
is explicitly **not** dismissible as "machine asymmetry" (revision 1 did exactly
that, which would have let a real reproduction be waved away).

## The noise model — PAIRED and within-cell (round-1 HIGH; revision 1's was not a noise floor at all)

Revision 1 defined `N` = max |ratio−1| over the four control cells. That is **not
a noise floor**: it is four point estimates drawn from different carriers,
fixtures and destinations, so it conflates *genuine control-specific initiator
effects* with *sampling noise*, and could equally mask a real effect or bless a
fake one.

Replaced with the **paired within-cell** statistic — the same construction pf-0's
review demanded of pf-1:

    For each cell, each ABBA slot i yields a matched pair (srcinit_i, destinit_i).
      d_i   = destinit_i − srcinit_i          (positive = P1's direction)
      D     = median(d_i)                     <- the effect
      S     = the spread of d_i               <- the PAIRED noise (report max−min AND IQR)
      MDE   = the smallest |D| this cell can resolve, taken as S (conservative)

`D` and `S` come from the *same* slots, under the *same* conditions, so ABBA
pairing is respected and between-session drift cannot enter. Every threshold below
is expressed against `S`, the 1.10 bar, or rig W's measured `Δ_P1 ≈ 230 ms` — none
is invented.

## POWER GATE — evaluated BEFORE any "vanish" claim (round-1 HIGH; pf-0's exact error, pre-empted)

pf-0 reported a KILL with an instrument that could not have resolved the effect it
killed. That must not recur.

For each TCP×mixed cell, **before** reading a verdict:

1. Compute `MDE` (above) and the effect size that a rig-W-scale P1 would have
   here: `Δ_ref = 230 ms` (rig W's Δ_P1), and also in ratio terms against **this
   rig's own fast arm** — because the 1.10 bar is a *ratio*, a 230 ms effect is
   only visible if the fast arm is fast enough (at a 2.3 s fast arm, 230 ms is
   exactly 1.10 and would sit **on** the bar).
2. **If `MDE > Δ_ref`, or if `Δ_ref` on this cell's fast arm does not exceed the
   1.10 bar, the cell is UNDERPOWERED and a PASS there is INCONCLUSIVE — it may
   NOT be reported as "P1 vanishes".** The rig gets reported as unable to see the
   effect, and the experiment does not close.

A **reproduction** does not need this gate (an effect that is seen is seen); a
**null** does.

## Decision rule — pre-registered, exhaustive, mutually exclusive, evaluated in order

Invariance uses the harness's **exact integer arithmetic** (`10·hi ≤ 11·lo`),
never the printed ratio. Per TCP×mixed cell: `D` = median paired difference,
`S` = paired spread.

1. **RIG-VOID.** Any control cell FAILS the 1.10 bar → the rig is not measuring
   cleanly and **no verdict is read**. (A rig whose gRPC control fails cannot
   adjudicate a TCP-only claim.) Report and stop.
2. **REPRODUCES (in a named direction).** A TCP×mixed cell FAILS the 1.10 bar with
   `D > 0` **and** `D > S`. Reported per direction; **either direction suffices.**
   → *P1 does not need a Windows peer.*
3. **INVERSION (in a named direction).** A TCP×mixed cell FAILS with `D < 0` and
   `|D| > S` (source-initiated is the slow arm). A **new finding**, reported as
   such — never banked as "P1 absent" and never counted as a reproduction.
4. **VANISHES.** *Both* TCP×mixed cells PASS the 1.10 bar, **and** `|D| ≤ S` in
   both, **and both cells cleared the POWER GATE.** → *P1 requires the Windows
   peer.* If the power gate was not cleared, this branch is unavailable and the
   result is **INCONCLUSIVE-UNDERPOWERED**.
5. **PARTIAL.** Any TCP×mixed cell PASSES the bar but has `|D| > S` in P1's
   direction — a real, sub-bar asymmetry. Reported with `D` stated against
   `Δ_ref = 230 ms`. Neither a reproduction nor a vanish; pf-1 owns it.
6. **MIXED-SIGN.** One direction reproduces (case 2) and the other inverts
   (case 3). Reported verbatim as a **host×role interaction**, which the rig
   cannot decompose. Explicitly **inconclusive** for the pairing question.

Cases 2/3/5/6 are read per direction and then combined by this order; the first
matching case that applies to the *session* is the headline, with every cell's own
outcome recorded. **No case is left unmapped, and no outcome may be reported that
is not one of these.**

**Bistability override, defined as a statistic, not a vibe (round-1 HIGH).** pf-0
found the rig-W fast arm bimodal, where the mode *mixture* moved a median 72 ms at
constant conditions. Here: if any arm's 8 runs split into two clusters separated by
more than `S` **and** the cell's verdict would flip when graded on the pooled runs
rather than the medians, the cell is reported **UNSTABLE**, not resolved. All 8
runs of every arm are printed in `summary.csv` so this is checkable, not asserted.

## Gates — fail-closed (round-1 HIGH: revision 1 only *warned* on the one that bit pf-0)

A run that misses any of these is **VOID**, not "close enough":

- **QUIESCENCE, BOTH MACS.** Refuse to start if `codex`/`cargo`/`rustc` runs on
  **either** Mac (both are bench **ends** here — nagatha is no longer just the
  driver). *(Already proven live: this gate fired on its first invocation and
  refused to start while the codex review of revision 1 was running.)*
- **TIME MACHINE, BOTH MACS — FAIL-CLOSED, not a warning.** Refuse to start if a
  backup is running **or if autobackup is merely ENABLED**, because macOS repeats
  hourly and a backup can begin *inside* the window (pf-0's did, 1 minute before
  the run; one destination is a network share on `skippy` — the same 10 GbE
  fabric). Revision 1 downgraded this to a warning; that is exactly the hole pf-0
  exposed.
- **SPOTLIGHT, BOTH MACS.** `mds_stores` is a recorded contaminant
  (`.agents/machines.md`). Refuse to start while it is actively indexing.
- **LOAD THRESHOLD.** `load1` recorded on both Macs at start **and end**; a start
  `load1` above **3.0** on either end VOIDs the session (the Macs idle at ~1.5–2).
- **Cold caches both ends every run** via `sudo -n /usr/sbin/purge` (NOPASSWD
  granted on both); a failed purge **VOIDS the pair** — a warm row is worse than
  no row.
- **Destination-keyed durability, never verb-keyed**: the macOS per-file `fsync`
  walk runs **on the destination host for both arms**, is **timed**, and a failed
  walk returns `NA` → the pair **VOIDS** (it must never read as a plausible flush).
  (The otp-2w rule: a sync inside the initiator's bracket charges the pull arm for
  writeback the push arm gets free and *manufactures* invariance failures — the
  gRPC control is what exposed it.)
- **Drain**: destination disk quiet before each timed window — macOS `iostat`,
  `< 2 MB/s` for 3 consecutive 2 s samples; `DRAIN-TIMEOUT` VOIDs the pair.
- **Fixtures verified by count on both ends** (`large` 1, `mixed` 5001,
  `small` 10000) before any timed run — the arms must read the same trees.
- **Provenance**: clean `+f35702a` on all four binaries; sha256 staging manifest.
- **Link validity, measured not assumed**: peer ARP resolves to the **peer's** MAC
  (a host route on a directly-connected subnet installs a black hole that still
  reports the right interface); an ssh throughput test **cannot** validate this
  link (~79 MB/s either way regardless) and is not used.

## What this does NOT establish

- **It is not an H1 verdict** (see the top). H1 names code paths, not a platform.
- **It cannot measure P2** — P2 is a converge bar against the OLD build and no old
  pair is staged on the Macs. P2 on the Linux rig is a separate, outstanding
  experiment.
- **A null here cannot retract rig W.** P1 is measured and reproduced four times
  there; this rig can only speak about the *pairing*.
- **It cannot decompose host×role.** nagatha and `q` differ; outcome 6 exists
  precisely because that case is beyond this rig.

## The runs

```sh
EXPECT_SHA=f35702a RUNS=8 \
  CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large \
  bash scripts/bench_otp12pf_mac.sh
```

Harness: `scripts/bench_otp12pf_mac.sh`. It **computes; it declares nothing** —
the verdict is read off the rule above.
# Agent Guidance

## Prime Invariants
<!-- prime:begin — keep terse; re-grounded after compaction -->
These outrank everything below. After a context compaction, re-read this block from AGENTS.md before continuing.

- Words first. Answer questions and musings in words; act only on an explicit instruction or go. A handed-over report, plan, or spec is evidence to assess, not a decision to implement.
- No code change without an approved plan; docs and other non-code edits don't need one (e.g. a README). When unsure, treat it as code. Do not expand scope without approval.
- Commit each slice as it lands; never leave finished work uncommitted. History-rewrite and destructive or outward-facing actions always need an explicit go. Push policy: see `.agents/push-policy.md`.
- Repo is memory. Durable truth lives in the repo, not chat or working memory. Under context pressure, re-ground from AGENTS.md; prefer a fresh session when degraded.
<!-- prime:end -->

## Repo-Specific Guidance

@.agents/repo-guidance.md

Repo-specific rules live in `.agents/repo-guidance.md`, imported above (read it directly if your harness does not process `@` imports). It extends this file and never overrides it — flag any genuine conflict.

## Universal Invariants

- The Prime Invariants above are the hardest-to-reverse rules; this section adds the rest.
- Agent-local or harness-local memory stores kept outside the repo are not durable memory, on any harness. Persist project-specific durable knowledge into the repo's `.agents/` files; reserve out-of-repo stores for genuinely cross-project facts (owner identity, preferences).
- Record important repo facts, decisions, invariants, verification rules, non-goals, and open questions in repo files, or explicitly report them as unrecorded. Write them generalized, tied to repo evidence or explicit human intent, so they make sense without the conversation that produced them — never as transient chat wording. Label inferred-but-unverified facts as assumptions until repo evidence or explicit human approval supports them.
- Keep one canonical location for each durable truth. Prefer pointers over duplicating the same rule; never keep a second copy of a count or enumeration another doc owns.
- Establish one immediately discoverable current-state entry point (`.agents/state.md`). Do not reconstruct current state from chat, long journals, or tool-local memory.
- When repo documents disagree, flag the conflict instead of silently choosing whichever source is convenient. Code and tests are evidence for behavior; approved plans and guidance are evidence for intent.
- Specific over generic: an explicit authority or scope boundary, or a rule or decision whose wording removes discretion for the case it names ("unconditional", "no per-run choice", "deterministic"), outranks every generic default for that case — flag-conflicts, one-canonical-location, smallest-guidance-set included. Apply it as written; do not reopen the case it settles as a conflict or approval question against surrounding repo state such as git history. Generic defaults govern only questions no more specific rule has already resolved.
- Prefer the smallest durable guidance set that fits the repo.
- Do not circumvent a roadblock whose provenance you have not established — a failing test, a guard or assertion, a lint or type error, a `.gitignore` rule, a refusal or permission denial, a config prohibition, a CI gate. Before removing or bypassing one, inspect its origin thoroughly enough to confirm it is not load-bearing; if you cannot, treat it as legitimate and stop or ask.
- Escalate an iterative process on stalled progress, never on duration. Each cycle must bank a verifiable delta — a test moving red→green, a finding closed with its guard proof, a build or type error resolved, a committed slice; a cycle that produces none is a stall. After a few consecutive stalled cycles (state the threshold you are using; default ~2-3), stop and surface to a human. A long run that banks a delta each cycle is healthy and must not be capped on duration or turn count.
- `AGENTS.md` is governance only — it must be portable. The test: would this line still be true and useful if copied unchanged into an unrelated repo? Process, invariants, and operator definitions pass. Anything true only of *this* repo — a concrete source path, the repo's own name as a fact, its verification commands, a restatement of current state or the decisions queue — fails and lives in `.agents/`, with `AGENTS.md` pointing to it, never restating it. References to the toolkit's own standard layout — `.agents/state.md`, operator names — are portable and allowed.
- `AGENTS.md` is the toolkit template, installed and replaced whole by governance refresh; no agent hand-edits it. Durable repo-specific rules go to `.agents/repo-guidance.md` and facts to the other `.agents/` files; a proposed `AGENTS.md` edit is out of bounds — question it, do not perform it.

## Session Startup

1. Read `AGENTS.md`, `.agents/repo-guidance.md`, and `.agents/state.md` if present, plus relevant `.agents/` files, before making changes; note any untracked or ignored agent-control files that affect the task.
2. Clone freshness: before trusting `.agents/state.md`, compare this clone against its canonical remote with a read-only check (`git ls-remote <remote> HEAD` against the local ref). Behind or diverged — say so and treat recorded state as possibly stale; unreachable — proceed with a one-line caveat, never block.
3. This repo ships a compaction re-ground hook (Claude Code; other harnesses only as listed in the toolkit's harness-capabilities record); if your harness gates hooks until the workspace is trusted, say what the hook does and run the trust step only on an explicit go — never bypass the gate.

## Source Of Truth

1. Human request.
2. `AGENTS.md`, extended by `.agents/repo-guidance.md` (extends, never overrides).
3. `.agents/state.md` for current work; `.agents/decisions.md` for settled decisions; approved `.agents/playbooks/*`.
4. Current code, tests, and CI as evidence for behavior.
5. Existing docs, only when consistent with current repo evidence.

When sources disagree, apply the flag-conflicts invariant (Universal Invariants): surface the conflict and fix the lower-authority source, or ask which should win.

## Operator Requests

Treat these owner words as process requests:

- `catchup`: re-read `AGENTS.md` (the Prime Invariants in full), `.agents/state.md`, and active repo docs; summarize current state, next action, blockers, and one proposed first action. Make no changes until the human responds.
- `handoff`: a fast save-my-place snapshot — seconds, not minutes. Update `.agents/state.md` `## Now` / `## Next` (and `## Blockers` if something is live) so the next session resumes without chat context: in-flight work, next action, stop. No archive rotation, no re-verification sweep, no mandatory re-anchoring of volatile facts — that hygiene belongs to `drift`. Machine-specific facts (CLI paths, local tool versions, host layout) go to the tracked `.agents/machines.md` under a heading for the current machine, dated, created on first use — never into `.agents/state.md`, which stays portable and may at most point there.
- `drift`: compare a doc, decision, or guidance claim against repo evidence; fix the lower-authority source or report the unresolved conflict. The guidance files themselves — `AGENTS.md` and `.agents/*` — are in scope as drift targets, not just sources of truth. `drift` also owns the deliberate state-hygiene pass: rotate landed or superseded `## Now` entries verbatim to `docs/history/state-archive.md` (create on first use); re-verify the recorded basis of every parked or blocked item and move anything falsified into `## Blockers` with the new evidence; volatile facts (CI state, counts) carry `as of <commit>` and are re-verified or dropped; push status is never recorded in state files — git owns it, sessions check it live, and unpushed work is mentioned only in the moment it matters — so any recorded push-state line is deleted on sight, not refreshed; a count or enumeration another file owns is pointed to, never copied; machine-specific facts relocate to `.agents/machines.md`, and stale entries there are pruned.
- `decision`: record a settled durable decision in `.agents/decisions.md` and update affected guidance.
- `plan`: draft or update a durable plan before broad implementation work. Plan documents are written for agents, never the owner: self-contained and technical, implementable by a completely cold, less-capable agent — no human-facing summary prose, no chat or session references that need the originating conversation to make sense. The owner does not read plan documents; present every decision a plan needs in chat as roughly 25-50 plain-English words — the problem, the change, the cost or risk — one decision at a time, never a batch, no jargon. Record the owner's approved wording durably (the decisions log, the plan's status line) so the approval survives the chat.
- `playbook <name>`: read `.agents/playbooks/<name>.md` and follow it. Playbooks are approved durable workflows; this operator is how a session invokes one by name. If the named playbook does not exist, say so rather than guessing.

## Verification

Use the repo's current automated verification entry point recorded in `.agents/repo-guidance.md` (Verification).

- For code changes, run the current automated verification before claiming completion.
- When a change ships with a new test, prove the test guards it: temporarily revert the change, confirm the test fails, restore it, confirm everything passes. A test that passes with its fix reverted is vacuous and must be replaced.
- For docs-only changes, code verification is not required unless the docs affect setup, commands, runtime behavior, generated files, or user-visible behavior.
- For behavior that automation does not cover, run the relevant manual check, smoke test, or playtest, or state clearly that it was not run.
- If no verification entry point is recorded yet, identify the likely command from repo evidence, record it, and label uncertainty. Ask the human only when evidence conflicts, no plausible command exists, or the command appears destructive, expensive, credentialed, or otherwise unsafe to run automatically.

## Git Safety

- Never conclude a branch is merged from ancestry alone: `git branch --merged` can lie after an `-s ours` or octopus merge records ancestry without content. Verify the content actually arrived (`git diff <branch> <main>`) before deleting anything or treating work as landed.
- When working through a list of findings or fixes, address exactly one item per commit and commit each before starting the next. Batch sweeps spanning many findings happen only on the owner's explicit request. Whether work happens on a branch is this repo's policy, not this rule's.
- Do not rewrite history or restructure existing commits without explicit owner approval: no `git commit --amend`, `rebase`, `squash`, or force-push, and no reordering or collapsing commits already made. The owner's approval authorizes the scoped commit as announced — it does not authorize a later rewrite of it. Default to a new commit per fix; if history genuinely needs reshaping, stop and ask.

## Final Response

Explain what changed, what was validated, and any remaining risk in plain English.
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

- `origin` — `http://q:3000/michael/blit_v2.git` (**LAN gitea**). This is
  what a bare `git push` / `git ls-remote origin` talks to.
- `github` — `https://github.com/roethlar/Blit.git` (**GitHub**).
- **`git push` does NOT update GitHub.** The two remotes are independent
  and nothing auto-syncs them: pushing `origin` moves the LAN gitea only,
  and GitHub needs its own explicit `git push github`. Either can lag the
  other by many commits. A ref-listing before a push must therefore name
  the remote's **URL**, not just "origin" — the name alone does not say
  which host is being published to.
- **CORRECTED 2026-07-13** against `git remote -v` (evidence in the
  session's push output: a bare `git push` printed
  `To http://q:3000/michael/blit_v2.git`, while `git push github` printed
  `To https://github.com/roethlar/Blit.git` and fast-forwarded GitHub
  `6d37a22..be1989d` — i.e. GitHub had been dozens of commits stale while
  every agent push went to the mirror). The prior text had the two names
  **inverted** (`origin` = GitHub, plus a `gitea` remote that does not
  exist) and asserted it had been "verified against `git remote -v`
  2026-07-04" while disparaging an earlier revision that was in fact
  correct. Do not re-invert this without re-reading `git remote -v`.
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
/bin/zsh -lc 'git status --short
git remote -v
git rev-parse --verify e1e351d''^{commit}
git show --stat --oneline --decorate --no-renames e1e351d
git show --format=fuller --no-ext-diff --no-renames e1e351d -- scripts/bench_otp12pf_mac.sh docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
sed -n '"'1,220p' docs/STATE.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-B2fK2Nlz' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-2GPPThr7' (errno=Operation not permitted)
?? .review/results/macmac-harness.codex.md
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-u220VmHE' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jazGGtAl' (errno=Operation not permitted)
github	https://github.com/roethlar/Blit.git (fetch)
github	https://github.com/roethlar/Blit.git (push)
origin	http://q:3000/michael/blit_v2.git (fetch)
origin	http://q:3000/michael/blit_v2.git (push)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Yn35D1jE' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-IhcmltWQ' (errno=Operation not permitted)
e1e351d8bd40ea12dda3184030eaebdf79d4d1c5
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-M6uogW1Y' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-RmZvtBOh' (errno=Operation not permitted)
e1e351d macmac-prereg rev 2 + the Mac<->Mac harness: the rig does NOT discriminate H1
 docs/plan/OTP12_PERF_FINDINGS.md | 11 +++++++++++
 scripts/bench_otp12pf_mac.sh     | 25 +++++++++++++++++++++----
 2 files changed, 32 insertions(+), 4 deletions(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-LlfIDpx7' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-HH7t5KOh' (errno=Operation not permitted)
commit e1e351d8bd40ea12dda3184030eaebdf79d4d1c5
Author:     Michael Coelho <mcoelho@gmail.com>
AuthorDate: Tue Jul 14 03:00:10 2026 -0400
Commit:     Michael Coelho <mcoelho@gmail.com>
CommitDate: Tue Jul 14 03:00:10 2026 -0400

    macmac-prereg rev 2 + the Mac<->Mac harness: the rig does NOT discriminate H1
    
    codex (gpt-5.6-sol, ultra) on the rev-1 pre-registration f0343f4: NOT READY --
    1 BLOCKER, 7 HIGH, 1 LOW. 9/9 accepted. NO DATA EXISTED WHEN THIS RAN, which is
    the entire point: the review killed an invalid central inference BEFORE a minute
    of rig time was spent.
    
    BLOCKER -- rev 1 claimed "P1 reproduces macOS<->macOS => H1 DIES, because H1
    accuses the Windows accept branch". H1 DOES NOT ACCUSE WINDOWS. Verbatim in the
    parent it accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
    add_dialed_stream, the dial-before-ACK at transfer_session/mod.rs:3113) -- the
    word "Windows" appears nowhere in it. Windows merely happens to be the accepting
    source in P1's slow arm on rig W, and that code runs on macOS too. So a Mac<->Mac
    reproduction is CONSISTENT with H1, not fatal to it. I took the framing from
    docs/STATE.md and never opened H1 -- the SECOND time this session I propagated a
    wrong claim about the hypotheses instead of reading them.
    
    The experiment is re-scoped to the question it can actually answer, which is
    still decision-relevant: does P1 REQUIRE the macOS<->Windows PAIRING, or is it a
    platform-general cost of the destination-initiated layout? Reproduces => P1 is
    NOT platform residue, the "accept it as residue" escape closes, and every
    code-level hypothesis strengthens. Vanishes => P1 is pairing-dependent.
    
    To stop this recurring a third time, H1 itself now carries a CANONICAL note in
    OTP12_PERF_FINDINGS.md: it accuses CODE, not a platform; "H1's Windows accept
    branch" elsewhere is shorthand for where that code runs on rig W; a reproduction
    without Windows does not kill H1, and a disappearance does not confirm it.
    
    The other 8, all accepted:
    - endpoint asymmetry does NOT cancel (switching the initiator also reassigns
      which Mac runs the CLI vs the daemon, and q is faster) -- claim withdrawn; both
      directions reported separately; a MIXED-SIGN outcome covers what the rig cannot
      decompose.
    - "both directions must fail" REWROTE P1, whose rig-W signature is
      one-directional (wm FAILS, mw PASSES) -- a reproduction in EITHER direction now
      suffices, so a real one cannot be waved away as "machine asymmetry".
    - "VANISHES" had NO POWER GATE -- pf-0's exact error, about to repeat. The 1.10
      bar is a RATIO, so a 230 ms effect on a 2.3 s fast arm sits ON the bar and a
      PASS would prove nothing. A power gate is now evaluated BEFORE any null: the
      harness emits paired.csv with powered_for_null per cell; underpowered => the
      null branch is unavailable (INCONCLUSIVE-UNDERPOWERED).
    - N was not a noise floor (max |ratio-1| over four cells of different carriers,
      fixtures and destinations) -- replaced by the PAIRED within-cell statistic
      (d_i = destinit_i - srcinit_i per ABBA slot; D = median, S = spread), the exact
      construction pf-0's review demanded of pf-1.
    - the outcome set was neither exhaustive nor unique -- six ordered, mutually
      exclusive, numerically defined cases; the bistability override is now a
      STATISTIC (clusters separated by > S AND a verdict that flips on pooled runs
      => cell reported UNSTABLE), not a vibe.
    - gates were not fail-closed -- Time Machine now refuses on RUNNING **or merely
      ENABLED** (pf-0's hole), Spotlight indexing refuses, load1 > 3.0 refuses.
      Sampled with top -l 2, never ps (whose decaying %CPU read a FINISHED backup as
      255% during pf-0).
    - "initiator is the only variable" was not instrumented -- fixtures verified by
      count on both ends, fsync walk timed and fail-closed (NA => pair VOIDS), drain
      a named macOS metric (iostat < 2 MB/s x3, DRAIN-TIMEOUT voids), peer ARP must
      resolve to the PEER's MAC (the black-hole trap), and an ssh throughput test is
      explicitly REJECTED as an instrument (~79 MB/s either way regardless).
    - sequencing conflict with durable guidance -- STATE's queue entry rewritten (it
      carried the retracted framing too).
    
    The quiescence gate already proved itself: it fired on its first invocation and
    refused to start while the codex review of rev 1 was running on nagatha -- which
    is now a bench END, not just the driver.
    
    check-docs.sh OK; bash -n OK; embedded python parses. The harness is a CODE slice
    and goes through its own codex review before any timed run.

diff --git a/scripts/bench_otp12pf_mac.sh b/scripts/bench_otp12pf_mac.sh
index b57a974..b6cdcd6 100755
--- a/scripts/bench_otp12pf_mac.sh
+++ b/scripts/bench_otp12pf_mac.sh
@@ -323,10 +323,27 @@ timed_run() {   # $1=initiating host $2=src spec $3=dst spec $4=DEST host $5=lan
   # The window is self-timed ON the initiating host (locally for nagatha; inside a
   # SINGLE ssh for q), so dispatch/round-trip is outside it by construction.
   # NO sync in here — durability is charged to the destination, below.
-  out="$(hrun "$ih" "python3 -c 'import time;print(int(time.monotonic()*1000))' > /tmp/mm_t0
-'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/mm-client.err; rc=\$?
-t1=\$(python3 -c 'import time;print(int(time.monotonic()*1000))'); t0=\$(cat /tmp/mm_t0)
-echo \"R:\$((t1-t0)),\${rc}:R\"" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
+  # ONE python process brackets the transfer. Two reasons, both load-bearing:
+  #   1. time.monotonic()'s REFERENCE POINT IS UNDEFINED ACROSS PROCESSES (python
+  #      docs; only same-process differences are valid). The first draft of this
+  #      function read t0 in one `python3 -c` and t1 in another and subtracted
+  #      them — which is meaningless, and measurably so: consecutive reads on this
+  #      rig returned -1 and -4 ms. It would have produced garbage timings that
+  #      still looked plausible.
+  #   2. Interpreter startup now falls OUTSIDE the timer. With a per-invocation
+  #      clock read, startup sat INSIDE the window — and since the two arms of a
+  #      cell are initiated by DIFFERENT Macs, any startup difference between them
+  #      is charged to one arm. That is the otp-2w failure mode (a cost billed to
+  #      one arm and not the other) in a new disguise.
+  out="$(hrun "$ih" "python3 - '$bin' '$src' '$dst' '$flag' <<'PYEOF'
+import subprocess, sys, time
+binp, src, dst, flag = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
+cmd = [binp, 'copy', src, dst, '--yes'] + ([flag] if flag else [])
+err = open('/tmp/mm-client.err', 'wb')
+t = time.monotonic()
+rc = subprocess.call(cmd, stdout=subprocess.DEVNULL, stderr=err)
+print('R:%d,%d:R' % (int((time.monotonic() - t) * 1000), rc))
+PYEOF" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
   if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   RUN_FLUSH="$(fsync_tree_ms "$dh" "$landed")"
   RUN_VALID=yes
# STATE — single entry point for "what is true right now"

Last updated: 2026-07-14

- **⛔ THE MAC↔MAC RIG IS NOT CLEARED TO RUN — rev 3 required (codex round 2: 3 BLOCKER, 6 HIGH, 2 LOW; 11/11 accepted, `.review/results/macmac-prereg-r2.gpt-verdict.md`).** No rig time spent. Three things must land first: **(a)** rev 2's inference is STILL overclaimed — a reproduction on these two Macs could be **macOS/APFS or host×role residue**, not a "platform-general layout cost", and a null licenses only "did not reproduce on this pair", NOT "Windows required" (and the "platform-residue escape" it claims to close **does not exist** — the parent says P1 has no escape hatch); **(b)** the power gate is **broken** — `S = max−min` is a range, not an MDE, and codex's counterexample (`d = [0,180,180,190,190,200,200,200]`, 7/8 positive, effect 83% of the 230 ms reference) returns **"VANISHES, powered=yes"**; it needs a real paired **equivalence** test (distribution-free CI on median(d_i); at n=8 the order stats `[d₍₂₎,d₍₇₎]`); **(c)** `compute_verdicts` implements **none** of the registered rule (no control gate, no clustering, no six outcomes — just PASS/FAIL), so a human would apply it after seeing the numbers. **The harness must COMPUTE the verdict; the prose must only describe what the code does.**
- **NEXT ACTION — rev 3 of the Mac↔Mac pre-registration + harness, then run it (Queue 1(ii); the second of the two experiments that come BEFORE any pf code).** Experiment (i), the A-B-B-A MTU run, is **DONE** (pf-0 below). Rig: nagatha `10.1.10.92` ↔ `q` `10.1.10.54`, both 10GbE/9000. Pre-registered (**rev 2**, codex round 1 = 9 findings, **9/9 accepted**): `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`; harness `scripts/bench_otp12pf_mac.sh`. **⚠ CORRECTED 2026-07-14 — it is NOT an H1 discriminator.** The earlier claim here ("reproduces ⇒ **H1 DIES**, H1 accuses the *Windows* accept branch") was **WRONG and is retracted**: H1 accuses **blit's own code paths** (`SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`) — **the word "Windows" appears nowhere in H1**, and that code runs on macOS too. A Mac↔Mac reproduction is therefore *consistent with* H1, not fatal to it. What the rig **does** answer: **does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost of the destination-initiated layout?** Reproduces ⇒ P1 is **not platform residue** (no Windows peer needed) → the "accept it as platform residue" escape closes and every code-level hypothesis strengthens. Vanishes ⇒ P1 is **pairing-dependent** → platform-agnostic code mechanisms weaken. Either way the hypothesis space moves, which is why it precedes pf-1. **Then `pf-1`** (the HARD GATE), which two pf-0 results now BIND: (a) **between-session grading is dead** (a 20% recovery = 46 ms sits under the 78 ms between-session floor) — pf-1 must **measure its own paired within-session floor and register a resolution check** before grading; (b) **the fast arm is BISTABLE** — grade the run distribution, not the median.
- **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
- **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
- **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.
- **P1 REPRODUCES ON A SECOND MAC (2026-07-13, `docs/bench/otp12-q-baseline-2026-07-13/`).** `wm_tcp_mixed` = **1.385 FAIL** on `q`↔netwatch-01 **at MTU 9000**, while all three controls PASS at **1.002–1.043** in the same session (so rig noise is ~2–4% and P1 is 10× outside it). **P1 is a property of the macOS↔Windows PAIRING, not of one machine** — the assumption **H1** rests on (corrected 2026-07-14: H5/H6/H7 are **P2** hypotheses; the earlier "H1/H5/H6/H7" was wrong), never tested until now. **And jumbo does NOT dissolve P1** — pf-0 has now measured the matched 1500 arm and killed MTU outright (above).
- **THE MAC IS A BENCH END — the codex loop and a rig-W session CANNOT run concurrently** (`.agents/machines.md`). A 53-min A-B-B-A attempt was destroyed by codex load on the Mac and discarded; the contamination is *asymmetric* (it inflates `mac_init` and MANUFACTURES P1). **Rig-W now runs on `q`** (dedicated M4 mini, quiet, faster than nagatha), which decouples the two for good.
- Recent sessions (2026-07-11/13, 44th–46th): **otp-10/otp-11 closed; otp-12c RECORDED (rig D 7/7); the perf plan is ACTIVE (D-2026-07-13-1).** Every transfer rides the ONE session (separate local orchestration gone, −6.2k lines at 11b; the unsound journal fast path died with it). Suite **1488**. SMALL_FILE_CEILING paused (D-2026-07-05-1).
- **P1 (the headline invariance criterion) — the one thing between blit and shipping.** Fails rig W (`wm_tcp_mixed` 1.237 and 1.300 — do NOT read that as a regression, it is **two different Mac NICs**), but **PASSES 8/8 with Linux on both ends** (`docs/bench/otp12-perf-2026-07-13/`; P1's own cell 1.092/1.003). So it is **platform-INTERACTING, not pure layout** — yet **NOT exonerated**: a code path that only bites on one platform (H1's Windows accept branch) looks identical. **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a *cross-direction* miss for a cell that ALREADY passes invariance — P1 *is* the invariance failure. So: **fix it to ≤1.10, or the owner amends acceptance criterion 1.** Neither is assumed.
- **⚠ THREE of my claims were reported and RETRACTED on 2026-07-13**, all the same root cause — trusting an instrument I had not validated: (1) "P1 is code" (a harness that keyed durability to the *initiator*, not the destination); (2) "P1 is acceptable platform residue" (D-2026-07-12-1 does not cover it); (3) "macOS can't send jumbo / the switch is broken" (it was `net.inet.raw.maxdgram` capping *ping*; TCP was always fine — it cost the owner a pointless adapter swap). **Verify the instrument before believing the measurement.**

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
  open.** Progress (each slice through the codex loop; per-slice
  detail lives in DEVLOG + `.review/`, NOT here):
  - **Closed `[x]`: otp-1 … otp-11** — the whole session machine, the
    baselines (otp-2/2w), the **CUTOVER DELETION** (4 drivers +
    `Push`/`PullSync` + 13 messages out of tree AND proto, −13.8k lines,
    no bridge; relay removed D-2026-07-11-1), and **otp-11b's deletion of
    the entire old orchestration** (−6.2k lines: orchestrator, engine,
    local_worker, auto_tune, change_journal — the last an UNSOUND fast
    path that silently lost data). The deletion-proof acceptance line
    COMPLETES. Detail: DEVLOG 2026-07-10/11/12; evidence
    `docs/bench/otp2{,w}-baseline-2026-07-10/`, `otp11-local-2026-07-11/`.
- **SMALL_FILE_CEILING PAUSED at sf-2 (D-2026-07-05-1)** — sf-1/sf-2
  `[x]`; **sf-3a+ blocked** until ONE_TRANSFER_PATH ships, then
  resume/re-derive on the unified baseline. Principle: ceiling-driven,
  never competitor-relative (D-2026-07-04-4 — do not re-litigate).
- **Background**: REV4 code-complete, gates DATA-COMPLETE (declarations
  in Blocked); the codex loop governs all changes (D-2026-07-04-1).

## Queue (ordered)

1. **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE, D-2026-07-05-4) —
   the only work item until it ships**: slices otp-1..13 through the
   codex loop per slice (owner re-affirmed). otp-1, otp-3, otp-4a,
   otp-4b (1/2/3), otp-5a, otp-5b (1/2), otp-6 (a/b), otp-7 (a, b-1,
   b-2), otp-8, otp-9 (a/b), otp-2 (+ otp-2w), otp-10 (a, b-1/2,
   c-1/2), **otp-11 (a + b)**, **otp-12a (zoey)**, **otp-12b
   (Mac↔Windows)** `[x]`. 12a: 10 PASS, 2 to the walk. 12b — THE
   INVARIANCE CRITERION: 11/12 PASS (1.003–1.057); wm_tcp_mixed 1.237
   (TCP×mixed×dest-initiator, code-shaped); push_tcp_small 1.149
   (both rigs); Win→Mac beats the better old direction 6/6; Mac→Win
   gap shapes recorded for the walk
   (`docs/bench/otp12-{zoey,win}-2026-07-12/`). **otp-12c `[x]`
   RECORDED 2026-07-13**: direct-path baseline at the cutover sha
   (`docs/bench/otp12c-win-2026-07-13/`) + the delegated rig-D
   matrix (`docs/bench/otp12c-delegated-2026-07-13/`, 5/7 PASS at
   RUNS=4; both FAIL cells PASS at RUNS=8 — see Blocked; rig D 7/7).
   **otp-12d and otp-13 are DEFERRED, not next** — otp-12c's rows are
   PRE-FIX, and `docs/plan/OTP12_PERF_FINDINGS.md` (pf-final) voids
   pre-fix new arms for acceptance. Assembling the acceptance matrix now
   would build otp-13's artifact from void rows.
1a. **`docs/plan/OTP12_PERF_FINDINGS.md` — THE REAL NEXT ITEM**
   (**ACTIVE**, D-2026-07-13-1 — owner: "just write the code and
   reviewloop slice by slice"; implementation proceeds, each slice
   through the codex loop).
   Two experiments come BEFORE any code; both docs own their detail.
   **(i) The A-B-B-A MTU run on `q` — `[x]` DONE 2026-07-14: MTU KILLED**
   (`r = −3.1%`; `docs/bench/otp12-jumbo-win-2026-07-13/`). See the pf-0
   bullet at the top for the two limits it puts on pf-1.
   **(ii) THE MAC↔MAC RIG — the missing cell of the 2×2** (owner,
   2026-07-13). Linux↔Linux = **no P1** (8/8 PASS); macOS↔Windows = **P1**
   (1.237/1.300/1.385/1.362); macOS↔macOS = **?** Design, decision rule and
   the retraction of the "H1 dies" framing: **see NEXT ACTION at the top**
   and the rev-2 pre-registration. **Both Macs are bench ENDS: the codex
   loop CANNOT run during the session** (the gate enforces it).
   **P1 HAS NO ESCAPE HATCH** (codex r5 F1): D-2026-07-12-1 waives only a
   *cross-direction* miss for a cell that ALREADY passes invariance — P1
   *is* the invariance failure. **Fix it to ≤1.10, or the owner amends
   acceptance criterion 1.** Not assumed either way. P2
   (`push_tcp_small` 1.105–1.201) is a converge bar vs the OLD build,
   UNTESTED on the Linux rig. Sequence: **MTU run + Mac↔Mac → pf-1 → fix
   → pf-final (ALL rigs) → otp-12d → otp-13.**
1b. **AFTER otp-12 — the Windows/local pair, planned TOGETHER** (same tar
   path, opposite directions: a fidelity fix ADDS per-file work to a path
   already losing to robocopy, so planning them apart optimises one against
   the other). Both docs own their detail; do not restate it here.
   - **`docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md` (D-2026-07-13-3)**
     — Windows attributes + ADS silently dropped, exit 0, **both routes
     (measured)**; loss is **conditional on file count**
     (`transfer_plan.rs:103-109`). Unlanded Windows support, NOT a regression.
     **Fix = WIRE CONTRACT change** → amend `TRANSFER_SESSION.md` first.
   - **`docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft, D-2026-07-13-2)** — local
     apply **does not scale** (8 workers buy 1.05×; robocopy gets ~2.2× from 8
     threads) and ships **one** worker. At EQUAL concurrency blit BEATS
     robocopy; at 8-vs-8 it loses 1.9×. `docs/bench/win-local-ab-2026-07-13/`.
2. **10 GbE owner declarations (still pending)**: ue-1, ue-2, REV4 →
   Shipped (zero-copy resolved — D-2026-07-05-3). Follow-ups largely
   absorbed by otp-2/otp-12's rig matrices.
3. **PAUSED: `docs/plan/SMALL_FILE_CEILING.md`** (D-2026-07-05-1) —
   resumes/re-derives after ONE_TRANSFER_PATH ships.
4. **PAUSED: design-review queue** (`REVIEW.md`; w7-1 topmost open row —
   likely landed inside otp-6's one-delete-rule slice; re-check first).
5. **Zero-copy receive — UNPARKED (D-2026-07-05-3)**: gate met (UNAS 8
   Pro daemon CPU-bound below 10 GbE from SSD cache). Executes AFTER
   cutover as a runtime-selected write strategy in the unified receive
   sink (design: eval doc §If-FAST-evidence; dead module deletes in
   w8-1). Rig facts + build recipe: DEVLOG 2026-07-05 10:00.
   **Standing owner safety rule**: ALL activity on rig `zoey` stays
   inside its `…/blit-temp/` folder — nothing written outside it, ever;
   no daemon runs on zoey without a fresh go.
6. **Post-REV4 residue** (unowned, 5 items) — list in DEVLOG 2026-07-13 21:00Z.

## Authoritative docs right now

- **`docs/plan/ONE_TRANSFER_PATH.md` (ACTIVE — governs all work;
  D-2026-07-05-4)**; `docs/plan/OTP7_RESUME.md` (**Active**,
  D-2026-07-09-1 — otp-7 slice design; governs otp-7a/7b).
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

- **Rigs**: owner go standing through otp-12. zoey (12a), netwatch-01
  (12b), netwatch-01↔skippy (12c) done; **magneto↔skippy = the same-OS
  rig** (new 2026-07-13). Rig facts + the macOS ping/MTU trap:
  `.agents/machines.md`.
- **otp-12c RECORDED 2026-07-13** (pre-fix rows = replication/control
  evidence, NOT acceptance evidence; Queue 1a):
  `docs/bench/otp12c-win-2026-07-13/` (198 runs) and
  `otp12c-delegated-2026-07-13/` (**rig D 7/7 PASS**). Codex: FAIL →
  **7/7 accepted**. Detail: DEVLOG 2026-07-13.
- **Three 10 GbE gate declarations**: ue-1, ue-2 (pass/fail or
  re-scope), REV4 → Shipped. (Zero-copy RESOLVED — D-2026-07-05-3.)

## Open questions

- **(OPEN, ripe — data in hand)** REV4 → Shipped flip: awaits the
  three declarations in Blocked (zero-copy resolved, D-2026-07-05-3).
- **(SLOTTED 2026-07-12 — owner ack)** `docs/WHITEPAPER.md` §8 (~line
  592) describes the deleted `determine_remote_tuning` — fix folded
  into **w10-docs-batch**; no one-off edit.
- **(PARTIALLY RESOLVED 2026-07-04)** Windows triage: w9-3 fixed the
  Linux daemon-spawn flakiness; windows-latest CI pending a push.
  NOTE 2026-07-12: the macOS `blit_utils` residual (pre-existing,
  reproduced at `6d37a22`) ran ELEVATED under heavy load (~3/12 vs 2/8
  historical) — own finding if it persists on a quiet machine.
- *(Resolved 2026-07-12/13: SizeMtime SKIP, `725aa07`, the `./NAME` foot-gun,
  otp-5b-3 cancel, the change-journal premise — all landed; see DEVLOG.)*

## Handoff log (newest first, keep ≤ 3)

- **2026-07-14 (48th)** — **pf-0 ran and MTU is KILLED as a cause of P1**
  (`r = −3.1%`; A-B-B-A on `q`, 256 runs, 0 voided, MSS gate held every session;
  `docs/bench/otp12-jumbo-win-2026-07-13/`). codex NOT READY → **7/7 accepted**
  (`11f0c2a`) — it confirmed every number and killed every *claim* that outran
  them: the run is **not powered** to exclude a *contributing*-size effect
  (46 ms < the 78 ms floor), "P1 is code-shaped" was **not** established (MTU is
  one variable; segment fill unmeasured), and declaring the frozen baseline VOID
  was **not an agent's call**. **The fast arm is BISTABLE** (bimodal `win_init`;
  the mode mixture, not MTU, sets the noise floor) — a pf-1 counterfactual that
  shifts the mixture would fake a recovery. Rig: Time Machine on `q` fired 1 min
  before the run (owner disabled it; **the harness's quiet-gate does not catch
  it**), and three starts died on a **physically flapping `en8`** the owner
  reseated — I chased three deterministic theories and falsified all three.
  **In-flight: none. Rigs clean, Windows MTU 9000, TM still OFF on `q` (owner
  re-enables), 4 + 4 commits unpushed.**
  **NEXT: the MAC↔MAC rig** (Queue 1(ii) — the last experiment before any pf
  code), **then pf-1.** The baseline re-record (D-2026-07-14-1) is a `pf-final`
  prerequisite, not a pf-1 blocker.
- **2026-07-13/14 (47th)** — P1 reproduces on a second Mac (`q`); new bench Mac;
  Windows attrs+ADS bug (D-2026-07-13-3); the robocopy headline was WRONG
  (D-2026-07-13-2); MTU prereg rev 1→4. Full: **DEVLOG 2026-07-14 00:15Z**.
- **2026-07-13 (46th)** — otp-12c closed (rig D 7/7); same-OS Linux rig (8/8 PASS
  → P1 is platform-INTERACTING); perf plan ACTIVE (D-2026-07-13-1); **three claims
  retracted, all from unvalidated instruments**. Full: **DEVLOG 2026-07-13 20:00Z**.
- *(45th and earlier pruned to the cap — see DEVLOG 2026-07-06..13.)*

exec
/bin/zsh -lc "git ls-remote origin HEAD
git rev-parse HEAD
git log --oneline --decorate -8
wc -l scripts/bench_otp12pf_mac.sh
git show e1e351d:scripts/bench_otp12pf_mac.sh | nl -ba | sed -n '1,220p'
git show e1e351d:scripts/bench_otp12pf_mac.sh | nl -ba | sed -n '221,460p'
git show e1e351d:scripts/bench_otp12pf_mac.sh | nl -ba | sed -n '461,760p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-4DQwe2Ub' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-nJTgl5dN' (errno=Operation not permitted)
fatal: unable to access 'http://q:3000/michael/blit_v2.git/': Could not resolve host: q
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-7VsYrN0K' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-0FmqwF2b' (errno=Operation not permitted)
d881340d36143808c06ebb2d3358004722e7ccd7
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-WzEv6iI4' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-bnYULTdT' (errno=Operation not permitted)
d881340 (HEAD -> master) macmac round 2: NOT READY again (3 BLOCKER) — the rig is not cleared to run
e1e351d macmac-prereg rev 2 + the Mac<->Mac harness: the rig does NOT discriminate H1
8375c0a macmac-prereg: address review (9 findings) — the experiment could not do what I claimed
f0343f4 pre-register the Mac<->Mac rig BEFORE any data (queue 1(ii); discriminates H1)
c4f8921 DEVLOG: D-2026-07-14-1 and its review (the decision draft was not executable)
c9933d8 D-2026-07-14-1: address review (6 findings) — the decision was not executable
d71c0ed decision D-2026-07-14-1: re-record the committed baselines at MTU 9000
9957d44 STATE: prune to the 200-line cap (fixes the docs gate I broke in bb912f4)
     531 scripts/bench_otp12pf_mac.sh
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-PagTUF2z' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-8iQRULE9' (errno=Operation not permitted)
     1	#!/usr/bin/env bash
     2	# =============================================================================
     3	# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
     4	# Design + decision rule: docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
     5	# Parent plan: docs/plan/OTP12_PERF_FINDINGS.md (queue 1(ii)).
     6	# =============================================================================
     7	#
     8	# WHY THIS RIG EXISTS
     9	# -------------------
    10	# P1 (destination-initiated TCP x mixed pays ~25-38%) has only ever been measured
    11	# on macOS<->Windows. Linux<->Linux shows NO P1 (8/8 PASS). macOS<->macOS is the
    12	# untested cell of the 2x2. It answers ONE question:
    13	#
    14	#     Does P1 REQUIRE the macOS<->Windows PAIRING, or is it a platform-general
    15	#     cost of the destination-initiated layout?
    16	#
    17	#   * reproduces -> P1 needs no Windows peer: it is NOT platform residue, the
    18	#     "accept it as platform residue" escape closes, and every code-level
    19	#     hypothesis strengthens;
    20	#   * vanishes   -> P1 is pairing-dependent: platform-agnostic code mechanisms
    21	#     weaken and a Windows-specific cost (or an interaction) rises.
    22	#
    23	# ⚠ IT IS **NOT** AN H1 DISCRIMINATOR, AND MUST NEVER BE CITED AS ONE.
    24	# Revision 1 of this script and of docs/STATE.md claimed "reproduces => H1 DIES,
    25	# because H1 accuses the Windows accept branch". That is FALSE and is retracted:
    26	# H1 accuses blit's OWN CODE PATHS (SourceSockets Dial/Accept branches,
    27	# InitiatorReceivePlaneRun.add_dialed_stream, the synchronous dial-before-ACK at
    28	# transfer_session/mod.rs:3113). The word "Windows" appears NOWHERE in H1, and
    29	# that code runs on macOS too — so a Mac<->Mac reproduction is CONSISTENT with
    30	# H1, not fatal to it. (The parent plan itself warns: "'consistent with H1' is
    31	# not confirmation.") Caught by codex review of the pre-registration, BEFORE any
    32	# rig time was spent.
    33	#
    34	# WHAT IT MEASURES
    35	#   cell = <nq|qn>_<carrier>_<fixture>
    36	#     nq_* : data nagatha -> q        qn_* : data q -> nagatha
    37	#   arms per cell (the ONLY variable):
    38	#     srcinit  : the SOURCE host's CLI pushes      (source-initiated)
    39	#     destinit : the DEST   host's CLI pulls       (destination-initiated)
    40	#   BOTH data directions are measured, but a reproduction is NOT required in
    41	#   both: P1's recorded signature on rig W is ONE-DIRECTIONAL (wm_tcp_mixed FAILS
    42	#   while mw_tcp_mixed PASSES), so demanding failure in both would rewrite the
    43	#   finding. A reproduction in EITHER direction demonstrates the layout cost
    44	#   without a Windows peer.
    45	#
    46	#   Endpoint asymmetry does NOT simply cancel: switching the initiator also
    47	#   reassigns which Mac runs the CLI and which runs the daemon, and q is the
    48	#   faster machine. Only arm-independent costs cancel; host x role interactions
    49	#   do not. Hence both directions are reported SEPARATELY and no conclusion may
    50	#   lean on perfect cancellation.
    51	#
    52	# VERDICT: invariance bar, max(srcinit,destinit)/min <= 1.10, integer-exact
    53	# (10*hi <= 11*lo). This script COMPUTES; it DECLARES nothing.
    54	#
    55	# METHODOLOGY (otp-12 shape + the two gates pf-0 proved were missing)
    56	#   * QUIESCENCE gate on BOTH Macs (codex/cargo/rustc) — here nagatha is a bench
    57	#     END, not merely the driver; load on either end contaminates ASYMMETRICALLY.
    58	#   * TIME MACHINE gate on BOTH Macs — the hole pf-0 found: the old quiet-gate
    59	#     watched only codex/cargo/rustc and would have sailed straight through the
    60	#     backup that fired 1 minute before pf-0's run (hourly cadence; one
    61	#     destination is a network share on skippy = the same 10GbE fabric).
    62	#   * cold caches BOTH ends every run via `sudo -n /usr/sbin/purge` (a failed
    63	#     purge VOIDS the pair — a warm row is worse than no row);
    64	#   * destination disk drained to quiet (iostat) before each timed window;
    65	#   * DURABILITY IS KEYED BY THE DESTINATION HOST, NEVER BY THE INITIATOR/VERB:
    66	#     the macOS per-file fsync walk runs on the destination for BOTH arms. (The
    67	#     otp-2w rule, re-learned the hard way: a sync inside the initiator's bracket
    68	#     charges the pull arm for writeback the push arm gets free and MANUFACTURES
    69	#     invariance failures — including on the gRPC control that must stay clean.)
    70	#   * fresh never-seen destination per run; ABBA counterbalance; pair-void with a
    71	#     2*RUNS cap then INCOMPLETE; nonzero exit or undrained window voids the pair;
    72	#   * same-build gate: every binary embeds a CLEAN +EXPECT_SHA (never +sha.dirty).
    73	#
    74	# TOPOLOGY NOTE (why one end is local): the driver runs on nagatha, so the nagatha
    75	# end is LOCAL and the q end is over ssh. This is the proven rig-W shape: each
    76	# timed window is self-timed ON the initiating host — locally for nagatha, and
    77	# INSIDE a single ssh for q — so the ssh round trip is outside the window by
    78	# construction and neither arm is charged for dispatch. The driver is blocked
    79	# waiting during every timed window, so its own load is idle and identical across
    80	# arms.
    81	#
    82	# Usage:
    83	#   EXPECT_SHA=f35702a RUNS=8 bash scripts/bench_otp12pf_mac.sh
    84	#   PREFLIGHT_ONLY=1 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
    85	#   CELLS=nq_tcp_mixed,qn_tcp_mixed RUNS=8 EXPECT_SHA=f35702a bash scripts/bench_otp12pf_mac.sh
    86	# =============================================================================
    87	set -euo pipefail
    88	
    89	SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    90	REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    91	
    92	HARNESS_HEAD="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
    93	EXPECT_SHA="${EXPECT_SHA:?set EXPECT_SHA to the pinned build (e.g. f35702a) — the binaries are gated on it}"
    94	
    95	# --- nagatha: LOCAL end (driver runs here) -----------------------------------
    96	N_IP="${N_IP:-10.1.10.92}"                       # 10GbE en11, MTU 9000
    97	N_ROOT="${N_ROOT:-$HOME/Dev/blit_v2_f35702a}"    # the pinned clone
    98	N_BLIT="${N_BLIT:-$N_ROOT/target/release/blit}"
    99	N_DAEMON="${N_DAEMON:-$N_ROOT/target/release/blit-daemon}"
   100	N_MODULE="${N_MODULE:-$HOME/blit-bench-work}"
   101	
   102	# --- q: REMOTE end ------------------------------------------------------------
   103	Q_SSH="${Q_SSH:-michael@q}"
   104	Q_IP="${Q_IP:-10.1.10.54}"                       # 10GbE en8, MTU 9000
   105	Q_ROOT="${Q_ROOT:-/Users/michael/Dev/blit_v2_f35702a}"
   106	Q_BLIT="${Q_BLIT:-$Q_ROOT/target/release/blit}"
   107	Q_DAEMON="${Q_DAEMON:-$Q_ROOT/target/release/blit-daemon}"
   108	Q_MODULE="${Q_MODULE:-/Users/michael/blit-bench-work}"
   109	
   110	PORT="${PORT:-9031}"
   111	RUNS="${RUNS:-8}"
   112	PREFLIGHT_ONLY="${PREFLIGHT_ONLY:-0}"
   113	CELLS="${CELLS:-}"
   114	SESSION_TAG="$(date +%Y%m%dT%H%M%S)"
   115	OUT_DIR="${OUT_DIR:-$REPO_ROOT/logs/otp12pf_mac_$SESSION_TAG}"
   116	DRAIN_ITERS="${DRAIN_ITERS:-60}"; DRAIN_QUIET="${DRAIN_QUIET:-3}"
   117	DRAIN_MBPS="${DRAIN_MBPS:-2}"     # dest disk considered quiet below this MB/s
   118	
   119	# /tmp, not $TMPDIR: macOS TMPDIR busts ssh's 104-byte ControlPath cap (otp-12c).
   120	MUX="$(mktemp -d /tmp/blit-mm-mux.XXXXXX)"
   121	SSH_MUX=(-o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=20
   122	         -o ControlMaster=auto -o "ControlPath=$MUX/%C" -o ControlPersist=180)
   123	qssh() { ssh "${SSH_MUX[@]}" "$Q_SSH" "$@"; }
   124	
   125	mkdir -p "$OUT_DIR/blit-logs"
   126	log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT_DIR/bench.log" >&2; }
   127	die() { log "FATAL: $*"; exit 1; }
   128	nocr() { tr -d '\r'; }
   129	want_cell() { [[ -z "$CELLS" ]] || [[ ",$CELLS," == *",$1,"* ]]; }
   130	
   131	# --- host abstraction: $1 = n (local nagatha) | q (remote) --------------------
   132	# if/else, never `[[ ]] && a || b` — a non-zero command in the && chain silently
   133	# falls through to the wrong host (the exact trap the Linux harness documents).
   134	hrun() {
   135	  local h="$1"; shift
   136	  if [[ "$h" == n ]]; then bash -c "$*"; else qssh "$*"; fi
   137	}
   138	hblit()   { [[ "$1" == n ]] && echo "$N_BLIT"   || echo "$Q_BLIT"; }
   139	hdaemon() { [[ "$1" == n ]] && echo "$N_DAEMON" || echo "$Q_DAEMON"; }
   140	hmod()    { [[ "$1" == n ]] && echo "$N_MODULE" || echo "$Q_MODULE"; }
   141	hip()     { [[ "$1" == n ]] && echo "$N_IP"     || echo "$Q_IP"; }
   142	hname()   { [[ "$1" == n ]] && echo nagatha     || echo q; }
   143	
   144	# --- fixtures (otp-2 shapes; verified by count, never trusted) ----------------
   145	FIX_COUNT_large=1;     FIX_COUNT_small=10000;  FIX_COUNT_mixed=5001
   146	
   147	# --- provenance: embed +sha AND reject +sha.dirty -----------------------------
   148	embeds_clean() {   # $1=host $2=path
   149	  hrun "$1" "grep -qa -- '+$EXPECT_SHA' '$2' && ! grep -qa -- '+$EXPECT_SHA.dirty' '$2'"
   150	}
   151	sha256_of() {      # $1=host $2=path
   152	  hrun "$1" "shasum -a 256 '$2' | cut -d' ' -f1" | nocr | tr -cd '0-9a-f'
   153	}
   154	
   155	# --- the two gates pf-0 proved were missing -----------------------------------
   156	quiescence_gate() {   # $1 = host. Bench ENDS must be quiet; load contaminates ASYMMETRICALLY.
   157	  local h="$1" busy
   158	  busy="$(hrun "$h" "pgrep -x codex >/dev/null && echo codex; pgrep -x cargo >/dev/null && echo cargo; pgrep -x rustc >/dev/null && echo rustc; true" | nocr | tr '\n' ' ')"
   159	  busy="$(echo "$busy" | xargs || true)"
   160	  [[ -z "$busy" ]] || die "$(hname "$h") is NOT quiet (running: $busy). Both Macs are bench ENDS — a busy end inflates one arm and MANUFACTURES P1 (.agents/machines.md). Stop them (do NOT blanket-kill the owner's sessions) and re-run."
   161	}
   162	timemachine_gate() {   # $1 = host. FAIL-CLOSED — the hole pf-0 found.
   163	  local h="$1" running auto
   164	  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';'" | nocr | tr -cd '0-9')"
   165	  [[ "${running:-0}" == 1 ]] && die "$(hname "$h"): a Time Machine backup is RUNNING — it hammers CPU and disk on a bench END (one destination is on skippy, the same 10GbE fabric)."
   166	  # AUTOBACKUP ENABLED is itself disqualifying, not a warning: macOS repeats
   167	  # HOURLY, so a backup can begin *inside* the window. pf-0's fired 1 minute
   168	  # before its run and the old gate never looked. A warning here would let the
   169	  # session start and be silently contaminated mid-flight.
   170	  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null || echo 0" | nocr | tr -cd '0-9')"
   171	  [[ "${auto:-0}" == 1 ]] && die "$(hname "$h"): Time Machine AUTOBACKUP is ENABLED — macOS repeats hourly, so a backup can start MID-SESSION. Disable it for the window (\`sudo tmutil disable\`) and re-enable after."
   172	  true
   173	}
   174	spotlight_gate() {   # $1 = host. mds_stores is a recorded contaminant (.agents/machines.md).
   175	  # Instantaneous sample: `ps` %CPU is a DECAYING AVERAGE and reads a finished
   176	  # backup as 255% (learned in pf-0) — top -l 2 is the honest instrument.
   177	  local h="$1" cpu
   178	  cpu="$(hrun "$h" "top -l 2 -n 20 -o cpu -stats command,cpu 2>/dev/null | awk '/mds_stores|^mds /{c=\$NF} END{print c+0}'" | nocr | tr -cd '0-9.')"
   179	  awk -v c="${cpu:-0}" 'BEGIN{exit !(c+0 > 20)}' \
   180	    && die "$(hname "$h"): Spotlight (mds_stores) is actively indexing at ${cpu}% CPU — a recorded bench contaminant. Wait for it to settle."
   181	  true
   182	}
   183	load1() { hrun "$1" "sysctl -n vm.loadavg" | nocr | awk '{print $2}'; }
   184	load_gate() {   # $1 = host. The Macs idle at ~1.5-2.0; above 3.0 something is running.
   185	  local h="$1" l; l="$(load1 "$h")"
   186	  awk -v l="${l:-0}" 'BEGIN{exit !(l+0 > 3.0)}' \
   187	    && die "$(hname "$h"): load1 is $l (> 3.0) — a bench END must be quiet. Find what is running before starting a timed session."
   188	  true
   189	}
   190	
   191	preflight() {
   192	  [[ "$RUNS" -ge 2 ]] || die "RUNS must be >= 2"
   193	  local h p
   194	  for h in n q; do
   195	    quiescence_gate "$h"
   196	    timemachine_gate "$h"
   197	    spotlight_gate "$h"
   198	    load_gate "$h"
   199	    for p in "$(hblit "$h")" "$(hdaemon "$h")"; do
   200	      hrun "$h" "test -x '$p'" || die "$(hname "$h"): missing/not executable: $p"
   201	      embeds_clean "$h" "$p" \
   202	        || die "$(hname "$h"): $p does not embed a CLEAN +$EXPECT_SHA (same-build rule, D-2026-07-05-2)"
   203	    done
   204	    # Cold-cache capability is METHODOLOGY, not a nicety — hard gate, fail closed.
   205	    hrun "$h" "sudo -n /usr/sbin/purge" \
   206	      || die "$(hname "$h") cannot purge without a password (need the NOPASSWD /usr/sbin/purge sudoers rule) — every run would read WARM"
   207	    hrun "$h" "pgrep -x blit-daemon >/dev/null" \
   208	      && die "$(hname "$h"): a blit-daemon is already running — stop it first (stale-daemon refusal)"
   209	    # Fixtures.
   210	    local w want got
   211	    for w in large mixed small; do
   212	      want="$(eval echo "\$FIX_COUNT_$w")"
   213	      got="$(hrun "$h" "find '$(hmod "$h")/src_$w' -type f 2>/dev/null | wc -l" | tr -cd '0-9')"
   214	      [[ "${got:-0}" == "$want" ]] \
   215	        || die "$(hname "$h"): src_$w has ${got:-0}/$want files — stage the fixtures before a timed run"
   216	    done
   217	  done
   218	  # Link validity, MEASURED not assumed (.agents/machines.md): the peer's ARP entry
   219	  # must be the PEER's MAC, never our own — a host route on a directly-connected
   220	  # subnet installs a BLACK HOLE that still reports the right interface.
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-5OPyrd42' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-jtrtloGb' (errno=Operation not permitted)
   221	  local pmac
   222	  ping -c1 -W1 "$Q_IP" >/dev/null 2>&1 || true
   223	  pmac="$(arp -n "$Q_IP" 2>/dev/null | awk '{print $4}')"
   224	  [[ -n "$pmac" && "$pmac" != "(incomplete)" ]] || die "no ARP entry for q ($Q_IP) — the link is not up"
   225	  log "preflight OK  build=$EXPECT_SHA (harness HEAD=$HARNESS_HEAD)  runs/arm=$RUNS  q_mac=$pmac"
   226	  log "  load1: nagatha=$(load1 n)  q=$(load1 q)"
   227	}
   228	
   229	write_manifest() {
   230	  local f="$OUT_DIR/staging-manifest.txt" h
   231	  { echo "# harness_head=$HARNESS_HEAD binary_identity=$EXPECT_SHA"
   232	    echo "host,role,sha,sha256,path"
   233	    for h in n q; do
   234	      echo "$(hname "$h"),client,$EXPECT_SHA,$(sha256_of "$h" "$(hblit "$h")"),$(hblit "$h")"
   235	      echo "$(hname "$h"),daemon,$EXPECT_SHA,$(sha256_of "$h" "$(hdaemon "$h")"),$(hdaemon "$h")"
   236	    done; } > "$f"
   237	  log "staging manifest recorded (4 hashes)"
   238	}
   239	
   240	# --- daemons (both ends serve: the source daemon serves pulls, the dest daemon
   241	#     serves pushes) --------------------------------------------------------
   242	N_PID=""; Q_PID=""
   243	daemon_start() {   # $1 = host
   244	  local h="$1" cfg mod bin pid
   245	  mod="$(hmod "$h")"; bin="$(hdaemon "$h")"
   246	  cfg="$mod/mm-bench.toml"
   247	  hrun "$h" "mkdir -p '$mod'
   248	printf '[daemon]\nbind = \"0.0.0.0\"\nport = $PORT\nno_mdns = true\n\n[[module]]\nname = \"bench\"\npath = \"$mod\"\nread_only = false\n' > '$cfg'
   249	nohup '$bin' --config '$cfg' > '$mod/mm-daemon.log' 2>&1 < /dev/null &
   250	sleep 2
   251	pgrep -x blit-daemon | head -1" >/dev/null 2>&1 || true
   252	  pid="$(hrun "$h" "pgrep -x blit-daemon | head -1" | nocr | tr -cd '0-9')"
   253	  [[ -n "$pid" ]] || die "$(hname "$h"): daemon failed to start (see $(hmod "$h")/mm-daemon.log)"
   254	  # Listening, not merely alive (the rig-W lesson: the process check passed while
   255	  # the socket was not accepting, and the smoke died on a transport error).
   256	  hrun "$h" "nc -z -G 3 127.0.0.1 $PORT" \
   257	    || die "$(hname "$h"): daemon pid $pid is up but NOT listening on $PORT"
   258	  [[ "$h" == n ]] && N_PID="$pid" || Q_PID="$pid"
   259	  log "$(hname "$h") daemon up (pid $pid) on $(hip "$h"):$PORT"
   260	}
   261	daemon_stop() {   # $1 = host; PID-scoped, comm-verified, and the death is VERIFIED
   262	  local h="$1" pid; pid="$([[ "$h" == n ]] && echo "$N_PID" || echo "$Q_PID")"
   263	  [[ -n "$pid" ]] || return 0
   264	  hrun "$h" "if ps -p $pid -o comm= 2>/dev/null | grep -q blit-daemon; then kill $pid 2>/dev/null; for i in 1 2 3 4 5 6; do ps -p $pid >/dev/null 2>&1 || break; sleep 0.5; done; ps -p $pid >/dev/null 2>&1 && kill -9 $pid 2>/dev/null; fi; true" >/dev/null 2>&1 || true
   265	  if hrun "$h" "ps -p $pid >/dev/null 2>&1"; then
   266	    log "ERROR: $(hname "$h") daemon pid $pid SURVIVED teardown — port $PORT may still be held"
   267	    return 1
   268	  fi
   269	  log "$(hname "$h") daemon stopped (pid $pid, verified gone)"
   270	}
   271	cleanup() {
   272	  daemon_stop n || true
   273	  daemon_stop q || true
   274	  rm -rf "$MUX" 2>/dev/null || true
   275	}
   276	trap cleanup EXIT
   277	
   278	# --- cold + drain -------------------------------------------------------------
   279	RUN_DRAIN=""; RUN_COLD=""
   280	drain_host() {   # $1 = DESTINATION host; wait until its disk is quiet (macOS iostat)
   281	  hrun "$1" "quiet=0
   282	for i in \$(seq 1 $DRAIN_ITERS); do
   283	  w=\$(iostat -d -w 2 -c 2 disk0 2>/dev/null | tail -1 | awk '{print \$3}')
   284	  ok=\$(awk -v w=\"\${w:-99}\" -v t=$DRAIN_MBPS 'BEGIN{print (w+0 < t) ? 1 : 0}')
   285	  if [ \"\$ok\" = 1 ]; then quiet=\$((quiet+1)); else quiet=0; fi
   286	  if [ \$quiet -ge $DRAIN_QUIET ]; then echo \"drained \${i}x2s\"; exit 0; fi
   287	done
   288	echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
   289	}
   290	prep_run() {   # $1 = dest host. Drain the DEST, then cold BOTH ends. A failed purge VOIDS.
   291	  local dh="$1" out cn=ok cq=ok
   292	  out="$(drain_host "$dh")"; RUN_DRAIN="${out:-DRAIN-ERROR}"; RUN_DRAIN="${RUN_DRAIN// /_}"
   293	  [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: dest($(hname "$dh")) UNDRAINED ($RUN_DRAIN) — pair voids"
   294	  hrun n "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cn=FAIL
   295	  hrun q "sync; sudo -n /usr/sbin/purge" >/dev/null 2>&1 || cq=FAIL
   296	  if [[ "$cn" == ok && "$cq" == ok ]]; then RUN_COLD=cold
   297	  else RUN_COLD="COLD-FAIL(nagatha=$cn,q=$cq)"; log "  WARNING: cold-cache FAILED ($RUN_COLD) — pair voids"; fi
   298	  echo "$RUN_DRAIN $RUN_COLD" >> "$OUT_DIR/drain.log"
   299	}
   300	
   301	# --- durability: ALWAYS the DESTINATION host, identically for both arms --------
   302	fsync_tree_ms() {   # $1 = DEST host, $2 = landed path. Prints ms, or NA (=> VOID).
   303	  local out
   304	  out="$(hrun "$1" "python3 - '$2' <<'PYEOF'
   305	import os, sys, time
   306	t = time.monotonic()
   307	for root, dirs, files in os.walk(sys.argv[1]):
   308	    for name in files:
   309	        fd = os.open(os.path.join(root, name), os.O_RDONLY)
   310	        os.fsync(fd)
   311	        os.close(fd)
   312	print('F:%d:F' % int((time.monotonic() - t) * 1000))
   313	PYEOF" 2>/dev/null | nocr | sed -n 's/.*F:\([0-9][0-9]*\):F.*/\1/p' | head -1)"
   314	  echo "${out:-NA}"   # a failed fsync must never read as a plausible flush
   315	}
   316	
   317	# --- one timed run ------------------------------------------------------------
   318	RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
   319	timed_run() {   # $1=initiating host $2=src spec $3=dst spec $4=DEST host $5=landed path $6=flag
   320	  local ih="$1" src="$2" dst="$3" dh="$4" landed="$5" flag="${6:-}" out bin
   321	  bin="$(hblit "$ih")"
   322	  prep_run "$dh"
   323	  # The window is self-timed ON the initiating host (locally for nagatha; inside a
   324	  # SINGLE ssh for q), so dispatch/round-trip is outside it by construction.
   325	  # NO sync in here — durability is charged to the destination, below.
   326	  # ONE python process brackets the transfer. Two reasons, both load-bearing:
   327	  #   1. time.monotonic()'s REFERENCE POINT IS UNDEFINED ACROSS PROCESSES (python
   328	  #      docs; only same-process differences are valid). The first draft of this
   329	  #      function read t0 in one `python3 -c` and t1 in another and subtracted
   330	  #      them — which is meaningless, and measurably so: consecutive reads on this
   331	  #      rig returned -1 and -4 ms. It would have produced garbage timings that
   332	  #      still looked plausible.
   333	  #   2. Interpreter startup now falls OUTSIDE the timer. With a per-invocation
   334	  #      clock read, startup sat INSIDE the window — and since the two arms of a
   335	  #      cell are initiated by DIFFERENT Macs, any startup difference between them
   336	  #      is charged to one arm. That is the otp-2w failure mode (a cost billed to
   337	  #      one arm and not the other) in a new disguise.
   338	  out="$(hrun "$ih" "python3 - '$bin' '$src' '$dst' '$flag' <<'PYEOF'
   339	import subprocess, sys, time
   340	binp, src, dst, flag = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4]
   341	cmd = [binp, 'copy', src, dst, '--yes'] + ([flag] if flag else [])
   342	err = open('/tmp/mm-client.err', 'wb')
   343	t = time.monotonic()
   344	rc = subprocess.call(cmd, stdout=subprocess.DEVNULL, stderr=err)
   345	print('R:%d,%d:R' % (int((time.monotonic() - t) * 1000), rc))
   346	PYEOF" | nocr | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
   347	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   348	  RUN_FLUSH="$(fsync_tree_ms "$dh" "$landed")"
   349	  RUN_VALID=yes
   350	  if [[ "$RUN_FLUSH" == NA ]]; then RUN_VALID=no; RUN_FLUSH=0; fi
   351	  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   352	  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* && "$RUN_COLD" == cold ]] || RUN_VALID=no
   353	}
   354	
   355	# --- arms: the ONLY variable is which host's CLI initiates --------------------
   356	CUR_W=""; CUR_FLAG=""
   357	arm_srcinit() {    # the SOURCE host pushes into the DEST daemon
   358	  local cell="$1" rid="$2" sh="$3" dh="$4" landed
   359	  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}/src_$CUR_W"
   360	  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" \
   361	                  "$(hip "$dh"):$PORT:/bench/${SESSION_TAG}_${cell}_${rid}/" \
   362	                  "$dh" "$landed" "$CUR_FLAG"
   363	  hrun "$dh" "rm -rf '$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}'" >/dev/null 2>&1 || true
   364	}
   365	arm_destinit() {   # the DEST host pulls from the SOURCE daemon
   366	  local cell="$1" rid="$2" sh="$3" dh="$4" landed
   367	  landed="$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}"
   368	  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" \
   369	                  "$landed" \
   370	                  "$dh" "$landed" "$CUR_FLAG"
   371	  hrun "$dh" "rm -rf '$landed'" >/dev/null 2>&1 || true
   372	}
   373	
   374	CSV="$OUT_DIR/runs.csv"; echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,cold,valid" > "$CSV"
   375	META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"
   376	
   377	run_pair_loop() {   # $1=cell $2=src host $3=dest host
   378	  local cell="$1" sh="$2" dh="$3"
   379	  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
   380	  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
   381	  while (( valid < RUNS && attempts < max )); do
   382	    attempts=$(( attempts + 1 ))
   383	    local order pair=yes rowA="" rowB="" arm rid aname init
   384	    if (( slot % 2 )); then order="A B"; else order="B A"; fi
   385	    for arm in $order; do
   386	      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
   387	      rid="${aname}_s${slot}a${attempts}"
   388	      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh"
   389	      else arm_destinit "$cell" "$rid" "$sh" "$dh"; fi
   390	      [[ "$RUN_VALID" == yes ]] || pair=no
   391	      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
   392	      [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
   393	      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-fsync ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
   394	    done
   395	    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
   396	    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
   397	    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
   398	  done
   399	  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
   400	  else echo "$cell,$attempts,yes" >> "$META"; fi
   401	}
   402	
   403	compute_verdicts() {
   404	  python3 - "$CSV" "$META" "$OUT_DIR/summary.csv" "$OUT_DIR/verdicts.csv" "$OUT_DIR/paired.csv" <<'PY'
   405	import csv, sys
   406	runs_p, meta_p, sum_p, ver_p, pair_p = sys.argv[1:6]
   407	rows = list(csv.DictReader(open(runs_p)))
   408	meta = {r["cell"]: r for r in csv.DictReader(open(meta_p))}
   409	by, void = {}, {}
   410	# PAIRED slots: the pre-registered noise model. Each ABBA slot yields a matched
   411	# (srcinit, destinit) pair under identical conditions, so d_i = destinit - srcinit
   412	# is a WITHIN-slot difference — no between-session drift can enter it. pf-0's
   413	# review established that an unpaired spread is NOT a noise floor.
   414	slots = {}
   415	for r in rows:
   416	    k = (r["cell"], r["arm"])
   417	    if r["valid"] == "yes":
   418	        by.setdefault(k, []).append(int(r["ms"]))
   419	        slots.setdefault((r["cell"], r["run"]), {})[r["arm"]] = int(r["ms"])
   420	    else:
   421	        void[k] = void.get(k, 0) + 1
   422	
   423	def med(v):
   424	    v = sorted(v); n = len(v)
   425	    return v[n // 2] if n % 2 else (v[n // 2 - 1] + v[n // 2]) // 2
   426	
   427	def complete(c):
   428	    if c not in meta or meta[c]["complete"] != "yes":
   429	        return False
   430	    arms = [a for (cc, a) in by if cc == c]
   431	    return "srcinit" in arms and "destinit" in arms
   432	
   433	with open(sum_p, "w") as f:
   434	    f.write("cell,arm,median_ms,avg_ms,best_ms,worst_ms,spread_pct,voided_runs,pairs_attempted,runs\n")
   435	    for (c, a) in sorted(by):
   436	        if not complete(c):
   437	            continue
   438	        v = by[(c, a)]
   439	        sp = round(100.0 * (max(v) - min(v)) / max(min(v), 1), 1)
   440	        # every run is printed: pf-0 found the fast arm BIMODAL, and a median
   441	        # alone hides a mode-mixture shift that would fake a recovery.
   442	        f.write("%s,%s,%d,%d,%d,%d,%s,%d,%s,%s\n" % (
   443	            c, a, med(v), sum(v) // len(v), min(v), max(v), sp,
   444	            void.get((c, a), 0), meta[c]["pairs_attempted"],
   445	            " ".join(str(x) for x in v)))
   446	
   447	# The paired statistics the pre-registered rule is actually graded on.
   448	#   D = median(d_i)  -> the effect (positive = destination-initiated is slower)
   449	#   S = spread(d_i)  -> the PAIRED noise floor (max-min; IQR also reported)
   450	#   MDE = S          -> conservatively, the smallest |D| this cell can resolve
   451	# DELTA_REF = 230 ms: rig W's measured Delta_P1, the effect size this rig must be
   452	# able to see before any "vanishes" claim is permitted (the POWER GATE).
   453	DELTA_REF = 230
   454	with open(pair_p, "w") as f:
   455	    f.write("cell,n_pairs,D_median_ms,S_spread_ms,IQR_ms,MDE_ms,fast_arm_ms,"
   456	            "delta_ref_ms,ref_ratio_on_fast_arm,powered_for_null,d_i\n")
   457	    for c in sorted(meta):
   458	        ds = sorted(v["destinit"] - v["srcinit"]
   459	                    for (cc, _r), v in slots.items()
   460	                    if cc == c and "srcinit" in v and "destinit" in v)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-KfDcqO9U' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-51WefqWY' (errno=Operation not permitted)
   461	        if not ds:
   462	            continue
   463	        n = len(ds)
   464	        D = med(ds)
   465	        S = max(ds) - min(ds)
   466	        q1, q3 = ds[n // 4], ds[(3 * n) // 4 - (1 if n % 4 == 0 else 0)]
   467	        fast = min(med(by[(c, "srcinit")]), med(by[(c, "destinit")])) if complete(c) else 0
   468	        # A 230 ms effect is only VISIBLE against a ratio bar if the fast arm is
   469	        # fast enough: at a 2.3 s fast arm, 230 ms IS exactly 1.10 and sits ON the
   470	        # bar. So the null branch needs BOTH: MDE <= DELTA_REF, and a ref-sized
   471	        # effect that would actually breach 1.10 here.
   472	        ref_ratio = (fast + DELTA_REF) / fast if fast else 0.0
   473	        powered = "yes" if (S <= DELTA_REF and 10 * (fast + DELTA_REF) > 11 * fast) else "NO"
   474	        f.write("%s,%d,%d,%d,%d,%d,%d,%d,%.3f,%s,%s\n" % (
   475	            c, n, D, S, q3 - q1, S, fast, DELTA_REF, ref_ratio, powered,
   476	            " ".join(str(x) for x in ds)))
   477	
   478	with open(ver_p, "w") as f:
   479	    f.write("comparison,kind,lhs,rhs,lhs_ms,rhs_ms,ratio,D_ms,S_ms,bar,outcome,powered_for_null\n")
   480	    for c in sorted(meta):
   481	        if not complete(c):
   482	            f.write("%s,invariance,srcinit,destinit,,,,,,1.10,INCOMPLETE,\n" % c)
   483	            continue
   484	        s, d = med(by[(c, "srcinit")]), med(by[(c, "destinit")])
   485	        hi, lo = max(s, d), min(s, d)
   486	        # integer-exact bar (10*hi <= 11*lo) — never the printed 3-decimal ratio
   487	        outcome = "PASS" if 10 * hi <= 11 * lo else "FAIL"
   488	        ds = sorted(v["destinit"] - v["srcinit"]
   489	                    for (cc, _r), v in slots.items()
   490	                    if cc == c and "srcinit" in v and "destinit" in v)
   491	        D = med(ds) if ds else 0
   492	        S = (max(ds) - min(ds)) if ds else 0
   493	        fast = lo
   494	        powered = "yes" if (ds and S <= DELTA_REF and 10 * (fast + DELTA_REF) > 11 * fast) else "NO"
   495	        f.write("%s,invariance,srcinit,destinit,%d,%d,%.3f,%d,%d,1.10,%s,%s\n" % (
   496	            c, s, d, (hi / lo) if lo else 0.0, D, S, outcome, powered))
   497	PY
   498	}
   499	
   500	main() {
   501	  preflight
   502	  write_manifest
   503	  if [[ "$PREFLIGHT_ONLY" == 1 ]]; then
   504	    log "PREFLIGHT_ONLY: checks passed; no daemon started, nothing timed"
   505	    exit 0
   506	  fi
   507	  log "session $SESSION_TAG  build=$EXPECT_SHA  nagatha=$N_IP  q=$Q_IP"
   508	  daemon_start n
   509	  daemon_start q
   510	
   511	  local carrier w flag cell
   512	  for w in mixed large small; do
   513	    for carrier in tcp grpc; do
   514	      [[ "$carrier" == grpc ]] && flag="--force-grpc" || flag=""
   515	      CUR_W="$w"; CUR_FLAG="$flag"
   516	      cell="nq_${carrier}_${w}"                       # data nagatha -> q
   517	      want_cell "$cell" && run_pair_loop "$cell" n q
   518	      cell="qn_${carrier}_${w}"                       # data q -> nagatha
   519	      want_cell "$cell" && run_pair_loop "$cell" q n
   520	    done
   521	  done
   522	
   523	  compute_verdicts
   524	  log "  load1 (end): nagatha=$(load1 n)  q=$(load1 q)"
   525	  log "=== SUMMARY (cold, drained, durable; $RUNS valid pairs/cell; ABBA) ==="
   526	  column -t -s, "$OUT_DIR/summary.csv" | tee -a "$OUT_DIR/bench.log"
   527	  log "=== VERDICTS (computed, NOT declared — read the pre-registered rule) ==="
   528	  column -t -s, "$OUT_DIR/verdicts.csv" | tee -a "$OUT_DIR/bench.log"
   529	  log "runs: $CSV"
   530	}
   531	main "$@"

2026-07-14T07:45:03.959030Z ERROR codex_models_manager::manager: failed to refresh available models: timeout waiting for child process to exit
codex
The first hard failure is confirmed: the push and pull path formulas match blit’s slash semantics, but the durability walk never proves the landed tree exists or contains files. I’m now checking whether the surrounding gates and statistical outputs can independently fail open or legitimize such a row.
exec
/bin/zsh -lc "git show e1e351d:docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md | nl -ba | sed -n '1,240p'
rg -n \"src_mixed|FIX_COUNT_mixed|fsync_tree_ms|DRAIN-TIMEOUT|tmutil status|AutoBackup|arp -n\" scripts docs/bench -g '*.sh' -g '*.md' | head -240
rg -n \"src_\\"'$|src_mixed|mixed.*5001|5001" scripts -g '"'*.sh' | head -200" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-zyVJXSXQ' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-Z77EpQgp' (errno=Operation not permitted)
     1	# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)
     2	
     3	**Status**: Pre-registered, **revision 2**. **No data exists yet.**
     4	Codex round 1 (of `f0343f4`): **NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW → 9/9
     5	accepted.** Adjudication: `.review/results/macmac-prereg.gpt-verdict.md`.
     6	Committed BEFORE the data so the decision rule cannot be authored around the
     7	numbers (the pf-0 discipline).
     8	
     9	**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1).
    10	
    11	## What revision 1 got WRONG, and what this experiment actually answers
    12	
    13	Revision 1 claimed this rig "discriminates H1 outright": *P1 reproduces
    14	macOS↔macOS ⇒ H1 dies, because H1 accuses the Windows accept branch.*
    15	
    16	**That inference is invalid, and the premise is false.** H1, verbatim in the
    17	parent, accuses **blit's own code paths** — the `SourceSockets` Dial/Accept
    18	branches, `InitiatorReceivePlaneRun.add_dialed_stream`, and the destination's
    19	synchronous dial-before-ACK at `transfer_session/mod.rs:3113`. **The word
    20	"Windows" appears nowhere in H1.** Windows is merely *who happens to be the
    21	accepting source* in P1's slow arm on rig W. The accused code runs on macOS too.
    22	So a Mac↔Mac reproduction is **consistent with H1**, not fatal to it — and the
    23	parent already warns that *"'consistent with H1' is not confirmation."*
    24	
    25	The bad framing was inherited from `docs/STATE.md` ("H1 accuses the *Windows*
    26	accept branch") and copied without checking H1's text. **That is a repo error and
    27	it is corrected wherever it appears.**
    28	
    29	**What this rig CAN answer — and it is still decision-relevant:**
    30	
    31	> **Does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost
    32	> of the destination-initiated layout?**
    33	
    34	| outcome | what it licenses |
    35	|---|---|
    36	| **P1 REPRODUCES macOS↔macOS** | The failure needs **no Windows peer**. P1 is **not platform residue** — it is a cost of the layout/code that survives with the Windows half removed. This **closes the "accept it as platform residue" escape** (the D-2026-07-12-1 shape) and **strengthens every code-level hypothesis, H1 included**. It does **not** name the mechanism. |
    37	| **P1 VANISHES macOS↔macOS** | The failure **requires the Windows peer**: it is pairing-dependent / platform-interacting. Code-only mechanisms that should bite on any OS are **weakened**; a Windows-specific cost, or a macOS↔Windows interaction, rises. It does **not** confirm H1 — H1's accept branch would then have to be *platform-conditionally* slow, which is a further claim needing pf-1's counterfactual. |
    38	
    39	Either outcome materially reshapes the hypothesis space and bears directly on
    40	whether P1 **must be fixed in code** or **could be accepted as platform residue**.
    41	That is why it runs before pf-1. **It is not an H1 kill/confirm and this document
    42	must never be cited as one.**
    43	
    44	## Rig
    45	
    46	- **nagatha** (owner's workstation): 10GbE `en11` = **10.1.10.92**, MTU 9000.
    47	- **`q`** (M4 mini, dedicated bench Mac): 10GbE `en8` = **10.1.10.54**, MTU 9000.
    48	- **Build**: `f35702a`, **clean `+f35702a`** on all four binaries (the `.dirty`
    49	  form is rejected) — the cutover sha behind every P1 measurement (12b/12c,
    50	  q-baseline, pf-0). HEAD is **not** code-identical to it, so the pin is
    51	  deliberate.
    52	
    53	**Endpoint asymmetry does NOT simply "cancel" (round-1 HIGH).** Revision 1 claimed
    54	it did. It does not: switching the initiator also **reassigns which machine runs
    55	the CLI and which runs the daemon**, and `q` is the faster Mac. Only
    56	arm-independent costs cancel; **host×role interactions do not.** This is handled
    57	by *measuring both data directions and reporting them separately* (below), not by
    58	assertion — and any conclusion that depends on the cancellation being perfect is
    59	out of bounds.
    60	
    61	## Cells
    62	
    63	Grammar `<nq|qn>_<carrier>_<fixture>`: `nq_*` = data **nagatha→q**, `qn_*` = data
    64	**q→nagatha**. Arms — the only variable — are `srcinit` (source's CLI pushes) and
    65	`destinit` (dest's CLI pulls).
    66	
    67	    CELLS = nq_tcp_mixed,  qn_tcp_mixed     <- THE MEASURAND (P1's shape)
    68	            nq_grpc_mixed, qn_grpc_mixed    <- carrier control (P1 is TCP-only)
    69	            nq_tcp_large,  qn_tcp_large     <- fixture control (P1 is mixed-only)
    70	
    71	`RUNS=8`, ABBA-counterbalanced, pair-void.
    72	
    73	**Both directions are measured, but a reproduction is NOT required in both
    74	(round-1 HIGH).** P1's recorded signature on rig W is **one-directional**:
    75	`wm_tcp_mixed` FAILS while `mw_tcp_mixed` PASSES. Demanding failure in both
    76	directions here would rewrite the finding. So: **a reproduction in EITHER
    77	direction demonstrates the layout cost without a Windows peer.** Whether it is
    78	direction-symmetric is reported as a descriptive fact — and, because the two
    79	directions differ in *which machine is the destination*, a one-directional result
    80	is explicitly **not** dismissible as "machine asymmetry" (revision 1 did exactly
    81	that, which would have let a real reproduction be waved away).
    82	
    83	## The noise model — PAIRED and within-cell (round-1 HIGH; revision 1's was not a noise floor at all)
    84	
    85	Revision 1 defined `N` = max |ratio−1| over the four control cells. That is **not
    86	a noise floor**: it is four point estimates drawn from different carriers,
    87	fixtures and destinations, so it conflates *genuine control-specific initiator
    88	effects* with *sampling noise*, and could equally mask a real effect or bless a
    89	fake one.
    90	
    91	Replaced with the **paired within-cell** statistic — the same construction pf-0's
    92	review demanded of pf-1:
    93	
    94	    For each cell, each ABBA slot i yields a matched pair (srcinit_i, destinit_i).
    95	      d_i   = destinit_i − srcinit_i          (positive = P1's direction)
    96	      D     = median(d_i)                     <- the effect
    97	      S     = the spread of d_i               <- the PAIRED noise (report max−min AND IQR)
    98	      MDE   = the smallest |D| this cell can resolve, taken as S (conservative)
    99	
   100	`D` and `S` come from the *same* slots, under the *same* conditions, so ABBA
   101	pairing is respected and between-session drift cannot enter. Every threshold below
   102	is expressed against `S`, the 1.10 bar, or rig W's measured `Δ_P1 ≈ 230 ms` — none
   103	is invented.
   104	
   105	## POWER GATE — evaluated BEFORE any "vanish" claim (round-1 HIGH; pf-0's exact error, pre-empted)
   106	
   107	pf-0 reported a KILL with an instrument that could not have resolved the effect it
   108	killed. That must not recur.
   109	
   110	For each TCP×mixed cell, **before** reading a verdict:
   111	
   112	1. Compute `MDE` (above) and the effect size that a rig-W-scale P1 would have
   113	   here: `Δ_ref = 230 ms` (rig W's Δ_P1), and also in ratio terms against **this
   114	   rig's own fast arm** — because the 1.10 bar is a *ratio*, a 230 ms effect is
   115	   only visible if the fast arm is fast enough (at a 2.3 s fast arm, 230 ms is
   116	   exactly 1.10 and would sit **on** the bar).
   117	2. **If `MDE > Δ_ref`, or if `Δ_ref` on this cell's fast arm does not exceed the
   118	   1.10 bar, the cell is UNDERPOWERED and a PASS there is INCONCLUSIVE — it may
   119	   NOT be reported as "P1 vanishes".** The rig gets reported as unable to see the
   120	   effect, and the experiment does not close.
   121	
   122	A **reproduction** does not need this gate (an effect that is seen is seen); a
   123	**null** does.
   124	
   125	## Decision rule — pre-registered, exhaustive, mutually exclusive, evaluated in order
   126	
   127	Invariance uses the harness's **exact integer arithmetic** (`10·hi ≤ 11·lo`),
   128	never the printed ratio. Per TCP×mixed cell: `D` = median paired difference,
   129	`S` = paired spread.
   130	
   131	1. **RIG-VOID.** Any control cell FAILS the 1.10 bar → the rig is not measuring
   132	   cleanly and **no verdict is read**. (A rig whose gRPC control fails cannot
   133	   adjudicate a TCP-only claim.) Report and stop.
   134	2. **REPRODUCES (in a named direction).** A TCP×mixed cell FAILS the 1.10 bar with
   135	   `D > 0` **and** `D > S`. Reported per direction; **either direction suffices.**
   136	   → *P1 does not need a Windows peer.*
   137	3. **INVERSION (in a named direction).** A TCP×mixed cell FAILS with `D < 0` and
   138	   `|D| > S` (source-initiated is the slow arm). A **new finding**, reported as
   139	   such — never banked as "P1 absent" and never counted as a reproduction.
   140	4. **VANISHES.** *Both* TCP×mixed cells PASS the 1.10 bar, **and** `|D| ≤ S` in
   141	   both, **and both cells cleared the POWER GATE.** → *P1 requires the Windows
   142	   peer.* If the power gate was not cleared, this branch is unavailable and the
   143	   result is **INCONCLUSIVE-UNDERPOWERED**.
   144	5. **PARTIAL.** Any TCP×mixed cell PASSES the bar but has `|D| > S` in P1's
   145	   direction — a real, sub-bar asymmetry. Reported with `D` stated against
   146	   `Δ_ref = 230 ms`. Neither a reproduction nor a vanish; pf-1 owns it.
   147	6. **MIXED-SIGN.** One direction reproduces (case 2) and the other inverts
   148	   (case 3). Reported verbatim as a **host×role interaction**, which the rig
   149	   cannot decompose. Explicitly **inconclusive** for the pairing question.
   150	
   151	Cases 2/3/5/6 are read per direction and then combined by this order; the first
   152	matching case that applies to the *session* is the headline, with every cell's own
   153	outcome recorded. **No case is left unmapped, and no outcome may be reported that
   154	is not one of these.**
   155	
   156	**Bistability override, defined as a statistic, not a vibe (round-1 HIGH).** pf-0
   157	found the rig-W fast arm bimodal, where the mode *mixture* moved a median 72 ms at
   158	constant conditions. Here: if any arm's 8 runs split into two clusters separated by
   159	more than `S` **and** the cell's verdict would flip when graded on the pooled runs
   160	rather than the medians, the cell is reported **UNSTABLE**, not resolved. All 8
   161	runs of every arm are printed in `summary.csv` so this is checkable, not asserted.
   162	
   163	## Gates — fail-closed (round-1 HIGH: revision 1 only *warned* on the one that bit pf-0)
   164	
   165	A run that misses any of these is **VOID**, not "close enough":
   166	
   167	- **QUIESCENCE, BOTH MACS.** Refuse to start if `codex`/`cargo`/`rustc` runs on
   168	  **either** Mac (both are bench **ends** here — nagatha is no longer just the
   169	  driver). *(Already proven live: this gate fired on its first invocation and
   170	  refused to start while the codex review of revision 1 was running.)*
   171	- **TIME MACHINE, BOTH MACS — FAIL-CLOSED, not a warning.** Refuse to start if a
   172	  backup is running **or if autobackup is merely ENABLED**, because macOS repeats
   173	  hourly and a backup can begin *inside* the window (pf-0's did, 1 minute before
   174	  the run; one destination is a network share on `skippy` — the same 10 GbE
   175	  fabric). Revision 1 downgraded this to a warning; that is exactly the hole pf-0
   176	  exposed.
   177	- **SPOTLIGHT, BOTH MACS.** `mds_stores` is a recorded contaminant
   178	  (`.agents/machines.md`). Refuse to start while it is actively indexing.
   179	- **LOAD THRESHOLD.** `load1` recorded on both Macs at start **and end**; a start
   180	  `load1` above **3.0** on either end VOIDs the session (the Macs idle at ~1.5–2).
   181	- **Cold caches both ends every run** via `sudo -n /usr/sbin/purge` (NOPASSWD
   182	  granted on both); a failed purge **VOIDS the pair** — a warm row is worse than
   183	  no row.
   184	- **Destination-keyed durability, never verb-keyed**: the macOS per-file `fsync`
   185	  walk runs **on the destination host for both arms**, is **timed**, and a failed
   186	  walk returns `NA` → the pair **VOIDS** (it must never read as a plausible flush).
   187	  (The otp-2w rule: a sync inside the initiator's bracket charges the pull arm for
   188	  writeback the push arm gets free and *manufactures* invariance failures — the
   189	  gRPC control is what exposed it.)
   190	- **Drain**: destination disk quiet before each timed window — macOS `iostat`,
   191	  `< 2 MB/s` for 3 consecutive 2 s samples; `DRAIN-TIMEOUT` VOIDs the pair.
   192	- **Fixtures verified by count on both ends** (`large` 1, `mixed` 5001,
   193	  `small` 10000) before any timed run — the arms must read the same trees.
   194	- **Provenance**: clean `+f35702a` on all four binaries; sha256 staging manifest.
   195	- **Link validity, measured not assumed**: peer ARP resolves to the **peer's** MAC
   196	  (a host route on a directly-connected subnet installs a black hole that still
   197	  reports the right interface); an ssh throughput test **cannot** validate this
   198	  link (~79 MB/s either way regardless) and is not used.
   199	
   200	## What this does NOT establish
   201	
   202	- **It is not an H1 verdict** (see the top). H1 names code paths, not a platform.
   203	- **It cannot measure P2** — P2 is a converge bar against the OLD build and no old
   204	  pair is staged on the Macs. P2 on the Linux rig is a separate, outstanding
   205	  experiment.
   206	- **A null here cannot retract rig W.** P1 is measured and reproduced four times
   207	  there; this rig can only speak about the *pairing*.
   208	- **It cannot decompose host×role.** nagatha and `q` differ; outcome 6 exists
   209	  precisely because that case is beyond this rig.
   210	
   211	## The runs
   212	
   213	```sh
   214	EXPECT_SHA=f35702a RUNS=8 \
   215	  CELLS=nq_tcp_mixed,qn_tcp_mixed,nq_grpc_mixed,qn_grpc_mixed,nq_tcp_large,qn_tcp_large \
   216	  bash scripts/bench_otp12pf_mac.sh
   217	```
   218	
   219	Harness: `scripts/bench_otp12pf_mac.sh`. It **computes; it declares nothing** —
   220	the verdict is read off the rule above.
scripts/bench_otp12_zoey.sh:128:fsync_tree_ms() {
scripts/bench_otp12_zoey.sh:303:echo "DRAIN-TIMEOUT"'
scripts/bench_otp12_zoey.sh:327:FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
scripts/bench_otp12_zoey.sh:355:    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp12_zoey.sh:356:        mkdir -p "$MAC_WORK/src_mixed"
scripts/bench_otp12_zoey.sh:357:        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp12_zoey.sh:359:            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
scripts/bench_otp12_zoey.sh:463:    RUN_FLUSH=$(fsync_tree_ms "$dst")   # durable, self-timed
scripts/bench_otp12_delegated.sh:405:'DRAIN-TIMEOUT'" 2>/dev/null | nocr || echo DRAIN-ERROR
scripts/bench_otp12_delegated.sh:423:echo DRAIN-TIMEOUT" 2>/dev/null | nocr || echo DRAIN-ERROR
scripts/bench_otp12_delegated.sh:462:FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
scripts/bench_otp12_delegated.sh:479:  if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp12_delegated.sh:480:    mkdir -p "$MAC_WORK/src_mixed"; local i d
scripts/bench_otp12_delegated.sh:481:    dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp12_delegated.sh:482:    for i in $(seq 1 5000); do d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"; dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null; done
scripts/bench_otp12_win.sh:130:fsync_tree_ms() {   # macOS per-file fsync walk; prints its own elapsed ms
scripts/bench_otp12_win.sh:377:"DRAIN-TIMEOUT"'
scripts/bench_otp12_win.sh:401:FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
scripts/bench_otp12_win.sh:426:    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp12_win.sh:427:        mkdir -p "$MAC_WORK/src_mixed"
scripts/bench_otp12_win.sh:428:        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp12_win.sh:430:            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
scripts/bench_otp12_win.sh:487:        RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
scripts/bench_otp12_win.sh:505:    RUN_FLUSH=$(fsync_tree_ms "$dst")
scripts/bench_otp12_win.sh:545:    RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
scripts/bench_otp2_baseline.sh:110:fsync_tree_ms() {
scripts/bench_otp2_baseline.sh:185:echo "DRAIN-TIMEOUT"'
scripts/bench_otp2_baseline.sh:215:    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp2_baseline.sh:216:        mkdir -p "$MAC_WORK/src_mixed"
scripts/bench_otp2_baseline.sh:217:        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp2_baseline.sh:219:            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
scripts/bench_otp2_baseline.sh:278:        fsync_ms=$(fsync_tree_ms "$MAC_WORK/dst_pull")   # durable, self-timed
scripts/bench_10gbe.sh:159:SRC_MIXED="$WORK/src_mixed"
scripts/bench_otp12pf_linux.sh:117:FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
scripts/bench_otp12pf_linux.sh:193:echo DRAIN-TIMEOUT" 2>/dev/null | nocr || echo DRAIN-ERROR
docs/bench/otp12c-win-2026-07-13/README.md:17:DRAIN-TIMEOUT and re-run to completion (`push_grpc_mixed` slot 3,
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:191:  `< 2 MB/s` for 3 consecutive 2 s samples; `DRAIN-TIMEOUT` VOIDs the pair.
scripts/bench_tripwires.sh:238:    gen_mixed "$WORK/src_mixed"
scripts/bench_otp12pf_mac.sh:145:FIX_COUNT_large=1;     FIX_COUNT_small=10000;  FIX_COUNT_mixed=5001
scripts/bench_otp12pf_mac.sh:164:  running="$(hrun "$h" "tmutil status 2>/dev/null | awk '/Running/{print \$3}' | tr -d ';'" | nocr | tr -cd '0-9')"
scripts/bench_otp12pf_mac.sh:170:  auto="$(hrun "$h" "defaults read /Library/Preferences/com.apple.TimeMachine AutoBackup 2>/dev/null || echo 0" | nocr | tr -cd '0-9')"
scripts/bench_otp12pf_mac.sh:223:  pmac="$(arp -n "$Q_IP" 2>/dev/null | awk '{print $4}')"
scripts/bench_otp12pf_mac.sh:288:echo DRAIN-TIMEOUT" 2>/dev/null | nocr | tail -1 || echo DRAIN-ERROR
scripts/bench_otp12pf_mac.sh:302:fsync_tree_ms() {   # $1 = DEST host, $2 = landed path. Prints ms, or NA (=> VOID).
scripts/bench_otp12pf_mac.sh:348:  RUN_FLUSH="$(fsync_tree_ms "$dh" "$landed")"
scripts/bench_otp2w_baseline.sh:84:fsync_tree_ms() {   # macOS per-file fsync walk; prints its own elapsed ms
scripts/bench_otp2w_baseline.sh:184:"DRAIN-TIMEOUT"'
scripts/bench_otp2w_baseline.sh:212:    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp2w_baseline.sh:213:        mkdir -p "$MAC_WORK/src_mixed"
scripts/bench_otp2w_baseline.sh:214:        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp2w_baseline.sh:216:            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
scripts/bench_otp2w_baseline.sh:268:        fsync_ms=$(fsync_tree_ms "$MAC_WORK/dst_pull")   # durable, self-timed
scripts/bench_otp12_zoey.sh:327:FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
scripts/bench_otp12_zoey.sh:355:    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp12_zoey.sh:356:        mkdir -p "$MAC_WORK/src_mixed"
scripts/bench_otp12_zoey.sh:357:        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp12_zoey.sh:359:            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
scripts/bench_otp12_delegated.sh:462:FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
scripts/bench_otp12_delegated.sh:479:  if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp12_delegated.sh:480:    mkdir -p "$MAC_WORK/src_mixed"; local i d
scripts/bench_otp12_delegated.sh:481:    dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp12_delegated.sh:482:    for i in $(seq 1 5000); do d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"; dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null; done
scripts/bench_otp12_win.sh:401:FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
scripts/bench_otp12_win.sh:426:    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp12_win.sh:427:        mkdir -p "$MAC_WORK/src_mixed"
scripts/bench_otp12_win.sh:428:        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp12_win.sh:430:            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
scripts/bench_10gbe.sh:159:SRC_MIXED="$WORK/src_mixed"
scripts/bench_win_local_ab.sh:81:FIX_FILES_mixed=5001
scripts/bench_otp2_baseline.sh:215:    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp2_baseline.sh:216:        mkdir -p "$MAC_WORK/src_mixed"
scripts/bench_otp2_baseline.sh:217:        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp2_baseline.sh:219:            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
scripts/bench_otp12pf_linux.sh:117:FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
scripts/bench_tripwires.sh:238:    gen_mixed "$WORK/src_mixed"
scripts/bench_otp2w_baseline.sh:212:    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
scripts/bench_otp2w_baseline.sh:213:        mkdir -p "$MAC_WORK/src_mixed"
scripts/bench_otp2w_baseline.sh:214:        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
scripts/bench_otp2w_baseline.sh:216:            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
scripts/bench_otp12pf_mac.sh:145:FIX_COUNT_large=1;     FIX_COUNT_small=10000;  FIX_COUNT_mixed=5001

exec
/bin/zsh -lc "sed -n '1,280p' .review/results/macmac-prereg-r2.gpt-verdict.md
sed -n '1,240p' .review/results/macmac-harness.codex.md 2>/dev/null || true
git show --stat --oneline d881340
git show --format= --no-ext-diff d881340 -- docs/STATE.md .review/results/macmac-prereg-r2.gpt-verdict.md | sed -n '1,340p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# macmac-prereg — adjudication of codex round 2

**Slice**: `8375c0a` + `e1e351d` — pre-registration rev 2 + the new harness
`scripts/bench_otp12pf_mac.sh`.
**Reviewer**: `gpt-5.6-sol` @ `ultra` (per `~/.codex/config.toml`).
**Raw review**: `.review/results/macmac-prereg-r2.codex.md`
**Verdict**: **NOT READY — 3 BLOCKER, 6 HIGH, 2 LOW.**
**Adjudication: 11 findings, 11 ACCEPTED, 0 rejected.** → **revision 3 required
before the rig may run.**

**The rig is NOT cleared to run.** Round 1 killed the experiment's central
inference; round 2 shows the *replacement* inference is still overclaimed, the
statistics cannot support a null, and the harness does not implement its own
decision rule. No data has been taken, which is the point — but this is now two
consecutive rounds where the design, not the prose, was wrong.

---

## BLOCKER 1 — rev 2 substitutes ANOTHER false dichotomy → **ACCEPTED**

Rev 2 claims: *reproduces ⇒ "platform-general cost of the layout" (not platform
residue); vanishes ⇒ "P1 requires the Windows peer".* **Both halves overreach.**

- A reproduction on **these two Macs** is equally consistent with a **macOS/APFS
  or host×role residue**. "Not Windows-specific" does **not** imply
  "platform-general".
- A null licenses only **"P1 did not reproduce on this macOS↔macOS pair"** — not
  "Windows is required". The pair is two specific machines with specific disks.
- Rev 2 also asserts a reproduction "closes the platform-residue escape". **There
  is no such escape on the books**: the parent states plainly that D-2026-07-12-1
  does *not* cover P1 (P1 *is* the invariance failure its precondition requires),
  so P1 today has **no escape hatch** and none can be closed. That sentence
  invents a consequence.

**Required in rev 3**: state only what the rig licenses —
*reproduces* ⇒ **P1 does not require a Windows peer** (nothing more);
*vanishes* ⇒ **P1 does not reproduce on this pair** (nothing more) — and name
macOS-specific / host×role residue as live alternatives to a reproduction.

## BLOCKER 2 — the power gate is broken, and the counterexample is damning → **ACCEPTED**

`S = max(d_i) − min(d_i)` is a **range**, not an MDE, not a precision estimate,
and not an equivalence bound; at n=8 it grows with the sample. The registered
"powered" test also never compares the observed `D` to the reference effect, and
it takes the fast arm as `min(src,dest)` rather than the **source-initiated
baseline**.

Codex's counterexample, which my rule blesses:

> `srcinit = 2000×8`, `d = [0,180,180,190,190,200,200,200]`
> → ratio 1.095 **PASS**, `D = 190`, `S = 200`, `powered = yes` → **VANISHES**

Seven of eight pairs positive, an effect **83% of the 230 ms reference**, and the
rule reports *P1 is absent*. That is precisely the class of error pf-0 committed
(a null from an instrument that could not see the effect) — reproduced here in a
document written to prevent it.

**Required in rev 3**: a genuine paired **equivalence** procedure —
distribution-free CI on the median of `d_i` (at n=8 the order statistics
`[d₍₂₎, d₍₇₎]` give a ≈93% interval), with:
- **REPRODUCES** iff the cell FAILS the bar **and** the CI lower bound > 0;
- **VANISHES** iff the CI **upper** bound < the pre-registered equivalence margin
  `Δ_eq` (the bar in ms on this rig: `0.10 × median(srcinit)`), **and** that
  margin is itself below the reference effect being excluded;
- **UNDERPOWERED/INCONCLUSIVE** whenever the CI is too wide to do either.
`D` alone never decides anything.

## BLOCKER 3 — the harness implements none of the rule → **ACCEPTED**

`compute_verdicts` emits per-cell `PASS/FAIL/INCOMPLETE` only. It contains **no**
rig-validity (control) gate, **no** clustering/bistability statistic, **no** power
gate applied to a verdict, and **none** of the six outcomes. The registered rule
therefore lives only in prose — meaning a human applies it **after seeing the
numbers**, which is exactly what pre-registration exists to prevent. The six
outcomes are also still overlapping (MIXED-SIGN is shadowed by
REPRODUCES/INVERSION) and incomplete (FAIL with |D| ≤ S; sub-bar negative
asymmetry; incomplete cells are unmapped; UNDERPOWERED and UNSTABLE are extra
outcomes never listed among the six).

**Required in rev 3**: the harness **computes the session verdict itself** — one
exhaustive, mutually exclusive decision tree, emitted as a single machine-readable
line — and the prose merely describes what the code does.

## HIGH 1 — durable time is two disjoint intervals → **ACCEPTED**

The single-process monotonic rewrite fixed the clock bug, but the transfer window
**ends before the call returns**, and the fsync walk **begins after another
dispatch + interpreter startup** — so arm-dependent writeback can occur *free*, in
the gap. The destination is also declared **drained before** `sync; purge` runs,
with no re-drain, and non-numeric `iostat` output coerces to `0` and reads as
"quiet".

## HIGH 2 — landing/data-shape validation is fail-open → **ACCEPTED**

Fixtures are checked **by count only** (a truncated/wrong-size tree passes), and
`os.walk()` over a **missing or empty** landed path silently visits nothing and
prints `F:0:F` — i.e. **an unlanded transfer is accepted as a valid 0 ms flush.**
Must assert file count **and byte sum** at the landed path, and a zero-file walk
must VOID.

## HIGH 3 — the ARP gate is cosmetic → **ACCEPTED**

A failed ping is ignored; *any* complete ARP entry passes; the peer MAC is never
compared to the known value; interface/MTU/media are unchecked; and the
`q`→nagatha direction is never tested. The documented **own-MAC black hole** and
the **wrong-NIC route** both pass this gate as written.

## HIGH 4 — the environmental gates are not fail-closed → **ACCEPTED**

Every probe **fails open**: a `tmutil` error reads as "not running"; an AutoBackup
read failure is explicitly coerced to `0`; a Spotlight probe failure emits `0`,
and a trailing idle `mds` row can overwrite a hot `mds_stores` sample. A parse
failure must VOID, never pass.

## HIGH 5 — the registered protocol is unenforced → **ACCEPTED**

`RUNS=2` is accepted (the registration says 8); an arbitrary `CELLS` list can omit
**every control** yet still receive verdicts and `powered=yes`; the harness's own
documented default runs **12 cells, not the registered six**; drain thresholds are
overridable and unrecorded. The harness must refuse anything that is not the
registered protocol, or label the output NON-REGISTERED.

## HIGH 6 — instrument provenance is weaker than binary provenance → **ACCEPTED**

Binaries reject `+sha.dirty`, but the **harness itself** is labelled only with the
committed `HEAD` — an edited worktree is invisible. sha256 outputs are not
validated as 64 hex chars, so the manifest can claim "4 hashes" while holding
empty values.

## LOW 1 — median flooring + an unspecified IQR → **ACCEPTED**

Even-`n` medians are floored *before* the "integer-exact" bar, permitting a
half-millisecond boundary flip. The IQR is an ad-hoc `x₆−x₃` at n=8 and can report
zero where Tukey hinges report a large spread.

## LOW 2 — the sequencing conflict was only half-fixed → **ACCEPTED**

I changed `docs/STATE.md` only. The **active plan** still says pf-1 starts now, and
`docs/DECISIONS.md` still records the settled MTU→pf-1 sequence, while rev 2
inserts Mac↔Mac before pf-1. Durable guidance still disagrees with itself.

---

## Status

**Rev 3 is required before any timed run.** Nothing is lost — no rig time was
spent, and two independent instrument bugs (the cross-process `time.monotonic()`
subtraction, and the landed-path semantics) were caught by live validation before
this review even ran. But the honest summary is that **the Mac↔Mac design has now
failed review twice on substance**, and the next revision must make the *harness*
the authority (it computes the verdict) rather than the prose.
Reading additional input from stdin...
OpenAI Codex v0.144.3
--------
workdir: /Users/michael/Dev/blit_v2
model: gpt-5.6-sol
provider: openai
approval: never
sandbox: read-only
reasoning effort: ultra
reasoning summaries: none
session id: 019f5f8d-d302-7030-9519-1fe2ea8bd239
--------
user
Review the diff of commit e1e351d (run: git show e1e351d), focusing on the NEW BENCHMARK HARNESS scripts/bench_otp12pf_mac.sh. It implements the pre-registered design in docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md (rev 2, same commit) -- read that first: the harness must compute exactly what that rule is graded on.

THE INSTRUMENT IS THE RISK. This project has retracted three separate claims that all traced to a benchmark harness that lied while looking correct: a durability sync charged to only ONE arm, which MANUFACTURED the very invariance failure under test; a throughput probe that reported "6787 MB/s CONFIRMED" from a transfer that moved ZERO bytes; and a ps %CPU decaying average that read a FINISHED backup as 255%. Assume this harness lies until proven otherwise.

Check, hardest first:

1. ARM SYMMETRY. The ONLY difference between the srcinit and destinit arms must be which host's CLI initiates. Trace both arms end to end: arm_srcinit, arm_destinit, timed_run, fsync_tree_ms, prep_run. Does either arm pay a cost the other does not? Specifically the destination-keyed fsync walk: each arm passes a DIFFERENT "landed" path, because blit uses rsync-style slash semantics -- a push to /bench/RUNDIR/ lands the tree at RUNDIR/src_<W>, while a pull into RUNDIR lands the files directly IN RUNDIR. If either landed path is wrong or missing, os.walk silently walks nothing and returns 0 ms, so that arm pays ZERO durability while the other pays full -- exactly the otp-2w bug. Is the timing bracket equivalent for a LOCAL bash -c initiator vs a REMOTE ssh initiator?

2. DOES IT MEASURE NOTHING? Could any step silently succeed while moving no bytes, or fsync an empty or absent directory? Is a failed transfer, a failed purge, a failed drain, or an empty fsync distinguishable from a genuinely fast one? Is the pair-void logic actually reachable, and does anything fail OPEN?

3. FIDELITY TO THE PRE-REGISTERED RULE. Paired stats: d_i per ABBA slot, D = median, S = spread. The POWER GATE: powered_for_null. The integer-exact 1.10 bar. All runs printed. Is the slot pairing correct given that a voided pair RE-RUNS the slot and the csv "run" column is the slot number -- can a stale or duplicate row corrupt d_i? Any off-by-one, wrong median convention, or wrong sign on d_i?

4. GATES: quiescence, Time Machine fail-closed on running OR merely enabled, Spotlight, load1 over 3.0, purge, provenance clean +sha and reject +sha.dirty, stale daemon, fixtures by count, ARP peer MAC. Are any fail-OPEN -- a gate whose command errors and is swallowed, an inverted grep exit code, or a value that parses to empty and compares as pass? Does the daemon-listening check actually prove it ACCEPTS connections?

5. Shell correctness: quoting through hrun's bash -c versus ssh -- the SAME string must behave identically both ways. set -euo pipefail interactions, the trap/cleanup path, CRLF, and anything that differs between the local and the remote host.

Output a concise markdown findings list -- each finding with file:line, severity, rationale -- then a final VERDICT line. Be concise; do not invoke skills.
codex
I’ll audit the preregistered rule first, then trace both arms and independently check statistics, gates, and shell/transport behavior. I’ll treat silent success and asymmetric timing as presumptive defects until the script proves otherwise.
exec
/bin/zsh -lc "sed -n '1,260p' docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md
sed -n '1,220p' AGENTS.md
sed -n '1,260p' .agents/repo-guidance.md
sed -n '1,220p' .agents/state.md" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
# otp-12 Mac↔Mac rig — PRE-REGISTRATION (written before any timed run)

**Status**: Pre-registered, **revision 2**. **No data exists yet.**
Codex round 1 (of `f0343f4`): **NOT READY — 1 BLOCKER + 7 HIGH + 1 LOW → 9/9
accepted.** Adjudication: `.review/results/macmac-prereg.gpt-verdict.md`.
Committed BEFORE the data so the decision rule cannot be authored around the
numbers (the pf-0 discipline).

**Parent**: `docs/plan/OTP12_PERF_FINDINGS.md` (Active, D-2026-07-13-1).

## What revision 1 got WRONG, and what this experiment actually answers

Revision 1 claimed this rig "discriminates H1 outright": *P1 reproduces
macOS↔macOS ⇒ H1 dies, because H1 accuses the Windows accept branch.*

**That inference is invalid, and the premise is false.** H1, verbatim in the
parent, accuses **blit's own code paths** — the `SourceSockets` Dial/Accept
branches, `InitiatorReceivePlaneRun.add_dialed_stream`, and the destination's
synchronous dial-before-ACK at `transfer_session/mod.rs:3113`. **The word
"Windows" appears nowhere in H1.** Windows is merely *who happens to be the
accepting source* in P1's slow arm on rig W. The accused code runs on macOS too.
So a Mac↔Mac reproduction is **consistent with H1**, not fatal to it — and the
parent already warns that *"'consistent with H1' is not confirmation."*

The bad framing was inherited from `docs/STATE.md` ("H1 accuses the *Windows*
accept branch") and copied without checking H1's text. **That is a repo error and
it is corrected wherever it appears.**

**What this rig CAN answer — and it is still decision-relevant:**

> **Does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost
> of the destination-initiated layout?**

| outcome | what it licenses |
|---|---|
| **P1 REPRODUCES macOS↔macOS** | The failure needs **no Windows peer**. P1 is **not platform residue** — it is a cost of the layout/code that survives with the Windows half removed. This **closes the "accept it as platform residue" escape** (the D-2026-07-12-1 shape) and **strengthens every code-level hypothesis, H1 included**. It does **not** name the mechanism. |
| **P1 VANISHES macOS↔macOS** | The failure **requires the Windows peer**: it is pairing-dependent / platform-interacting. Code-only mechanisms that should bite on any OS are **weakened**; a Windows-specific cost, or a macOS↔Windows interaction, rises. It does **not** confirm H1 — H1's accept branch would then have to be *platform-conditionally* slow, which is a further claim needing pf-1's counterfactual. |

Either outcome materially reshapes the hypothesis space and bears directly on
whether P1 **must be fixed in code** or **could be accepted as platform residue**.
That is why it runs before pf-1. **It is not an H1 kill/confirm and this document
must never be cited as one.**

## Rig

- **nagatha** (owner's workstation): 10GbE `en11` = **10.1.10.92**, MTU 9000.
- **`q`** (M4 mini, dedicated bench Mac): 10GbE `en8` = **10.1.10.54**, MTU 9000.
- **Build**: `f35702a`, **clean `+f35702a`** on all four binaries (the `.dirty`
  form is rejected) — the cutover sha behind every P1 measurement (12b/12c,
  q-baseline, pf-0). HEAD is **not** code-identical to it, so the pin is
  deliberate.

**Endpoint asymmetry does NOT simply "cancel" (round-1 HIGH).** Revision 1 claimed
it did. It does not: switching the initiator also **reassigns which machine runs
the CLI and which runs the daemon**, and `q` is the faster Mac. Only
arm-independent costs cancel; **host×role interactions do not.** This is handled
by *measuring both data directions and reporting them separately* (below), not by
assertion — and any conclusion that depends on the cancellation being perfect is
out of bounds.

## Cells

Grammar `<nq|qn>_<carrier>_<fixture>`: `nq_*` = data **nagatha→q**, `qn_*` = data
**q→nagatha**. Arms — the only variable — are `srcinit` (source's CLI pushes) and
`destinit` (dest's CLI pulls).

    CELLS = nq_tcp_mixed,  qn_tcp_mixed     <- THE MEASURAND (P1's shape)
            nq_grpc_mixed, qn_grpc_mixed    <- carrier control (P1 is TCP-only)
            nq_tcp_large,  qn_tcp_large     <- fixture control (P1 is mixed-only)

`RUNS=8`, ABBA-counterbalanced, pair-void.

**Both directions are measured, but a reproduction is NOT required in both
(round-1 HIGH).** P1's recorded signature on rig W is **one-directional**:
`wm_tcp_mixed` FAILS while `mw_tcp_mixed` PASSES. Demanding failure in both
directions here would rewrite the finding. So: **a reproduction in EITHER
direction demonstrates the layout cost without a Windows peer.** Whether it is
direction-symmetric is reported as a descriptive fact — and, because the two
directions differ in *which machine is the destination*, a one-directional result
is explicitly **not** dismissible as "machine asymmetry" (revision 1 did exactly
that, which would have let a real reproduction be waved away).

## The noise model — PAIRED and within-cell (round-1 HIGH; revision 1's was not a noise floor at all)

Revision 1 defined `N` = max |ratio−1| over the four control cells. That is **not
a noise floor**: it is four point estimates drawn from different carriers,
fixtures and destinations, so it conflates *genuine control-specific initiator
effects* with *sampling noise*, and could equally mask a real effect or bless a
fake one.

Replaced with the **paired within-cell** statistic — the same construction pf-0's
review demanded of pf-1:

    For each cell, each ABBA slot i yields a matched pair (srcinit_i, destinit_i).
      d_i   = destinit_i − srcinit_i          (positive = P1's direction)
      D     = median(d_i)                     <- the effect
      S     = the spread of d_i               <- the PAIRED noise (report max−min AND IQR)
      MDE   = the smallest |D| this cell can resolve, taken as S (conservative)

`D` and `S` come from the *same* slots, under the *same* conditions, so ABBA
pairing is respected and between-session drift cannot enter. Every threshold below
is expressed against `S`, the 1.10 bar, or rig W's measured `Δ_P1 ≈ 230 ms` — none
is invented.

## POWER GATE — evaluated BEFORE any "vanish" claim (round-1 HIGH; pf-0's exact error, pre-empted)

pf-0 reported a KILL with an instrument that could not have resolved the effect it
killed. That must not recur.

For each TCP×mixed cell, **before** reading a verdict:

1. Compute `MDE` (above) and the effect size that a rig-W-scale P1 would have
   here: `Δ_ref = 230 ms` (rig W's Δ_P1), and also in ratio terms against **this
   rig's own fast arm** — because the 1.10 bar is a *ratio*, a 230 ms effect is
   only visible if the fast arm is fast enough (at a 2.3 s fast arm, 230 ms is
   exactly 1.10 and would sit **on** the bar).
2. **If `MDE > Δ_ref`, or if `Δ_ref` on this cell's fast arm does not exceed the
   1.10 bar, the cell is UNDERPOWERED and a PASS there is INCONCLUSIVE — it may
   NOT be reported as "P1 vanishes".** The rig gets reported as unable to see the
   effect, and the experiment does not close.

A **reproduction** does not need this gate (an effect that is seen is seen); a
**null** does.

## Decision rule — pre-registered, exhaustive, mutually exclusive, evaluated in order

Invariance uses the harness's **exact integer arithmetic** (`10·hi ≤ 11·lo`),
never the printed ratio. Per TCP×mixed cell: `D` = median paired difference,
`S` = paired spread.

1. **RIG-VOID.** Any control cell FAILS the 1.10 bar → the rig is not measuring
   cleanly and **no verdict is read**. (A rig whose gRPC control fails cannot
   adjudicate a TCP-only claim.) Report and stop.
2. **REPRODUCES (in a named direction).** A TCP×mixed cell FAILS the 1.10 bar with
   `D > 0` **and** `D > S`. Reported per direction; **either direction suffices.**
   → *P1 does not need a Windows peer.*
3. **INVERSION (in a named direction).** A TCP×mixed cell FAILS with `D < 0` and
   `|D| > S` (source-initiated is the slow arm). A **new finding**, reported as
   such — never banked as "P1 absent" and never counted as a reproduction.
4. **VANISHES.** *Both* TCP×mixed cells PASS the 1.10 bar, **and** `|D| ≤ S` in
   both, **and both cells cleared the POWER GATE.** → *P1 requires the Windows
   peer.* If the power gate was not cleared, this branch is unavailable and the
   result is **INCONCLUSIVE-UNDERPOWERED**.
5. **PARTIAL.** Any TCP×mixed cell PASSES the bar but has `|D| > S` in P1's
   direction — a real, sub-bar asymmetry. Reported with `D` stated against
   `Δ_ref = 230 ms`. Neither a reproduction nor a vanish; pf-1 owns it.
6. **MIXED-SIGN.** One direction reproduces (case 2) and the other inverts
   (case 3). Reported verbatim as a **host×role interaction**, which the rig
   cannot decompose. Explicitly **inconclusive** for the pairing question.

Cases 2/3/5/6 are read per direction and then combined by this order; the first
matching case that applies to the *session* is the headline, with every cell's own
outcome recorded. **No case is left unmapped, and no outcome may be reported that
is not one of these.**

**Bistability override, defined as a statistic, not a vibe (round-1 HIGH).** pf-0
found the rig-W fast arm bimodal, where the mode *mixture* moved a median 72 ms at
constant conditions. Here: if any arm's 8 runs split into two clusters separated by
more than `S` **and** the cell's verdict would flip when graded on the pooled runs
rather than the medians, the cell is reported **UNSTABLE**, not resolved. All 8
runs of every arm are printed in `summary.csv` so this is checkable, not asserted.

## Gates — fail-closed (round-1 HIGH: revision 1 only *warned* on the one that bit pf-0)

A run that misses any of these is **VOID**, not "close enough":

- **QUIESCENCE, BOTH MACS.** Refuse to start if `codex`/`cargo`/`rustc` runs on
  **either** Mac (both are bench **ends** here — nagatha is no longer just the
  driver). *(Already proven live: this gate fired on its first invocation and
  refused to start while the codex review of revision 1 was running.)*
- **TIME MACHINE, BOTH MACS — FAIL-CLOSED, not a warning.** Refuse to start if a
  backup is running **or if autobackup is merely ENABLED**, because macOS repeats
  hourly and a backup can begin *inside* the window (pf-0's did, 1 minute before
  the run; one destination is a network share on `skippy` — the same 10 GbE
  fabric). Revision 1 downgraded this to a warning; that is exactly the hole pf-0
  exposed.
- **SPOTLIGHT, BOTH MACS.** `mds_stores` is a recorded contaminant
  (`.agents/machines.md`). Refuse to start while it is actively indexing.
- **LOAD THRESHOLD.** `load1` recorded on both Macs at start **and end**; a start
  `load1` above **3.0** on either end VOIDs the session (the Macs idle at ~1.5–2).
- **Cold caches both ends every run** via `sudo -n /usr/sbin/purge` (NOPASSWD
  granted on both); a failed purge **VOIDS the pair** — a warm row is worse than
  no row.
- **Destination-keyed durability, never verb-keyed**: the macOS per-file `fsync`
  walk runs **on the destination host for both arms**, is **timed**, and a failed
  walk returns `NA` → the pair **VOIDS** (it must never read as a plausible flush).
  (The otp-2w rule: a sync inside the initiator's bracket charges the pull arm for
  writeback the push arm gets free and *manufactures* invariance failures — the
  gRPC control is what exposed it.)
- **Drain**: destination disk quiet before each timed window — macOS `iostat`,
  `< 2 MB/s` for 3 consecutive 2 s samples; `DRAIN-TIMEOUT` VOIDs the pair.
- **Fixtures verified by count on both ends** (`large` 1, `mixed` 5001,
  `small` 10000) before any timed run — the arms must read the same trees.
- **Provenance**: clean `+f35702a` on all four binaries; sha256 staging manifest.
- **Link validity, measured not assumed**: peer ARP resolves to the **peer's** MAC
  (a host route on a directly-connected subnet installs a black hole that still
  reports the right interface); an ssh throughput test **cannot** validate this
  link (~79 MB/s either way regardless) and is not used.

## What this does NOT establish

- **It is not an H1 verdict** (see the top). H1 names code paths, not a platform.
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-icTeNOwI' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-24Mh1dge' (errno=Operation not permitted)
d881340 macmac round 2: NOT READY again (3 BLOCKER) — the rig is not cleared to run
 .review/results/macmac-prereg-r2.codex.md       | 171 ++++++++++++++++++++++++
 .review/results/macmac-prereg-r2.gpt-verdict.md | 151 +++++++++++++++++++++
 DEVLOG.md                                       |   2 +
 docs/STATE.md                                   |   3 +-
 4 files changed, 326 insertions(+), 1 deletion(-)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-kcoZHUUU' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-NY4FYISI' (errno=Operation not permitted)
diff --git a/.review/results/macmac-prereg-r2.gpt-verdict.md b/.review/results/macmac-prereg-r2.gpt-verdict.md
new file mode 100644
index 0000000..c80a7f2
--- /dev/null
+++ b/.review/results/macmac-prereg-r2.gpt-verdict.md
@@ -0,0 +1,151 @@
+# macmac-prereg — adjudication of codex round 2
+
+**Slice**: `8375c0a` + `e1e351d` — pre-registration rev 2 + the new harness
+`scripts/bench_otp12pf_mac.sh`.
+**Reviewer**: `gpt-5.6-sol` @ `ultra` (per `~/.codex/config.toml`).
+**Raw review**: `.review/results/macmac-prereg-r2.codex.md`
+**Verdict**: **NOT READY — 3 BLOCKER, 6 HIGH, 2 LOW.**
+**Adjudication: 11 findings, 11 ACCEPTED, 0 rejected.** → **revision 3 required
+before the rig may run.**
+
+**The rig is NOT cleared to run.** Round 1 killed the experiment's central
+inference; round 2 shows the *replacement* inference is still overclaimed, the
+statistics cannot support a null, and the harness does not implement its own
+decision rule. No data has been taken, which is the point — but this is now two
+consecutive rounds where the design, not the prose, was wrong.
+
+---
+
+## BLOCKER 1 — rev 2 substitutes ANOTHER false dichotomy → **ACCEPTED**
+
+Rev 2 claims: *reproduces ⇒ "platform-general cost of the layout" (not platform
+residue); vanishes ⇒ "P1 requires the Windows peer".* **Both halves overreach.**
+
+- A reproduction on **these two Macs** is equally consistent with a **macOS/APFS
+  or host×role residue**. "Not Windows-specific" does **not** imply
+  "platform-general".
+- A null licenses only **"P1 did not reproduce on this macOS↔macOS pair"** — not
+  "Windows is required". The pair is two specific machines with specific disks.
+- Rev 2 also asserts a reproduction "closes the platform-residue escape". **There
+  is no such escape on the books**: the parent states plainly that D-2026-07-12-1
+  does *not* cover P1 (P1 *is* the invariance failure its precondition requires),
+  so P1 today has **no escape hatch** and none can be closed. That sentence
+  invents a consequence.
+
+**Required in rev 3**: state only what the rig licenses —
+*reproduces* ⇒ **P1 does not require a Windows peer** (nothing more);
+*vanishes* ⇒ **P1 does not reproduce on this pair** (nothing more) — and name
+macOS-specific / host×role residue as live alternatives to a reproduction.
+
+## BLOCKER 2 — the power gate is broken, and the counterexample is damning → **ACCEPTED**
+
+`S = max(d_i) − min(d_i)` is a **range**, not an MDE, not a precision estimate,
+and not an equivalence bound; at n=8 it grows with the sample. The registered
+"powered" test also never compares the observed `D` to the reference effect, and
+it takes the fast arm as `min(src,dest)` rather than the **source-initiated
+baseline**.
+
+Codex's counterexample, which my rule blesses:
+
+> `srcinit = 2000×8`, `d = [0,180,180,190,190,200,200,200]`
+> → ratio 1.095 **PASS**, `D = 190`, `S = 200`, `powered = yes` → **VANISHES**
+
+Seven of eight pairs positive, an effect **83% of the 230 ms reference**, and the
+rule reports *P1 is absent*. That is precisely the class of error pf-0 committed
+(a null from an instrument that could not see the effect) — reproduced here in a
+document written to prevent it.
+
+**Required in rev 3**: a genuine paired **equivalence** procedure —
+distribution-free CI on the median of `d_i` (at n=8 the order statistics
+`[d₍₂₎, d₍₇₎]` give a ≈93% interval), with:
+- **REPRODUCES** iff the cell FAILS the bar **and** the CI lower bound > 0;
+- **VANISHES** iff the CI **upper** bound < the pre-registered equivalence margin
+  `Δ_eq` (the bar in ms on this rig: `0.10 × median(srcinit)`), **and** that
+  margin is itself below the reference effect being excluded;
+- **UNDERPOWERED/INCONCLUSIVE** whenever the CI is too wide to do either.
+`D` alone never decides anything.
+
+## BLOCKER 3 — the harness implements none of the rule → **ACCEPTED**
+
+`compute_verdicts` emits per-cell `PASS/FAIL/INCOMPLETE` only. It contains **no**
+rig-validity (control) gate, **no** clustering/bistability statistic, **no** power
+gate applied to a verdict, and **none** of the six outcomes. The registered rule
+therefore lives only in prose — meaning a human applies it **after seeing the
+numbers**, which is exactly what pre-registration exists to prevent. The six
+outcomes are also still overlapping (MIXED-SIGN is shadowed by
+REPRODUCES/INVERSION) and incomplete (FAIL with |D| ≤ S; sub-bar negative
+asymmetry; incomplete cells are unmapped; UNDERPOWERED and UNSTABLE are extra
+outcomes never listed among the six).
+
+**Required in rev 3**: the harness **computes the session verdict itself** — one
+exhaustive, mutually exclusive decision tree, emitted as a single machine-readable
+line — and the prose merely describes what the code does.
+
+## HIGH 1 — durable time is two disjoint intervals → **ACCEPTED**
+
+The single-process monotonic rewrite fixed the clock bug, but the transfer window
+**ends before the call returns**, and the fsync walk **begins after another
+dispatch + interpreter startup** — so arm-dependent writeback can occur *free*, in
+the gap. The destination is also declared **drained before** `sync; purge` runs,
+with no re-drain, and non-numeric `iostat` output coerces to `0` and reads as
+"quiet".
+
+## HIGH 2 — landing/data-shape validation is fail-open → **ACCEPTED**
+
+Fixtures are checked **by count only** (a truncated/wrong-size tree passes), and
+`os.walk()` over a **missing or empty** landed path silently visits nothing and
+prints `F:0:F` — i.e. **an unlanded transfer is accepted as a valid 0 ms flush.**
+Must assert file count **and byte sum** at the landed path, and a zero-file walk
+must VOID.
+
+## HIGH 3 — the ARP gate is cosmetic → **ACCEPTED**
+
+A failed ping is ignored; *any* complete ARP entry passes; the peer MAC is never
+compared to the known value; interface/MTU/media are unchecked; and the
+`q`→nagatha direction is never tested. The documented **own-MAC black hole** and
+the **wrong-NIC route** both pass this gate as written.
+
+## HIGH 4 — the environmental gates are not fail-closed → **ACCEPTED**
+
+Every probe **fails open**: a `tmutil` error reads as "not running"; an AutoBackup
+read failure is explicitly coerced to `0`; a Spotlight probe failure emits `0`,
+and a trailing idle `mds` row can overwrite a hot `mds_stores` sample. A parse
+failure must VOID, never pass.
+
+## HIGH 5 — the registered protocol is unenforced → **ACCEPTED**
+
+`RUNS=2` is accepted (the registration says 8); an arbitrary `CELLS` list can omit
+**every control** yet still receive verdicts and `powered=yes`; the harness's own
+documented default runs **12 cells, not the registered six**; drain thresholds are
+overridable and unrecorded. The harness must refuse anything that is not the
+registered protocol, or label the output NON-REGISTERED.
+
+## HIGH 6 — instrument provenance is weaker than binary provenance → **ACCEPTED**
+
+Binaries reject `+sha.dirty`, but the **harness itself** is labelled only with the
+committed `HEAD` — an edited worktree is invisible. sha256 outputs are not
+validated as 64 hex chars, so the manifest can claim "4 hashes" while holding
+empty values.
+
+## LOW 1 — median flooring + an unspecified IQR → **ACCEPTED**
+
+Even-`n` medians are floored *before* the "integer-exact" bar, permitting a
+half-millisecond boundary flip. The IQR is an ad-hoc `x₆−x₃` at n=8 and can report
+zero where Tukey hinges report a large spread.
+
+## LOW 2 — the sequencing conflict was only half-fixed → **ACCEPTED**
+
+I changed `docs/STATE.md` only. The **active plan** still says pf-1 starts now, and
+`docs/DECISIONS.md` still records the settled MTU→pf-1 sequence, while rev 2
+inserts Mac↔Mac before pf-1. Durable guidance still disagrees with itself.
+
+---
+
+## Status
+
+**Rev 3 is required before any timed run.** Nothing is lost — no rig time was
+spent, and two independent instrument bugs (the cross-process `time.monotonic()`
+subtraction, and the landed-path semantics) were caught by live validation before
+this review even ran. But the honest summary is that **the Mac↔Mac design has now
+failed review twice on substance**, and the next revision must make the *harness*
+the authority (it computes the verdict) rather than the prose.
diff --git a/docs/STATE.md b/docs/STATE.md
index c5c85c0..c5b5ea1 100644
--- a/docs/STATE.md
+++ b/docs/STATE.md
@@ -2,7 +2,8 @@
 
 Last updated: 2026-07-14
 
-- **NEXT ACTION — the MAC↔MAC RIG (Queue 1(ii)), the second of the two experiments that come BEFORE any pf code.** Experiment (i), the A-B-B-A MTU run, is **DONE** (pf-0 below). Rig: nagatha `10.1.10.92` ↔ `q` `10.1.10.54`, both 10GbE/9000. Pre-registered (**rev 2**, codex round 1 = 9 findings, **9/9 accepted**): `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`; harness `scripts/bench_otp12pf_mac.sh`. **⚠ CORRECTED 2026-07-14 — it is NOT an H1 discriminator.** The earlier claim here ("reproduces ⇒ **H1 DIES**, H1 accuses the *Windows* accept branch") was **WRONG and is retracted**: H1 accuses **blit's own code paths** (`SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`) — **the word "Windows" appears nowhere in H1**, and that code runs on macOS too. A Mac↔Mac reproduction is therefore *consistent with* H1, not fatal to it. What the rig **does** answer: **does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost of the destination-initiated layout?** Reproduces ⇒ P1 is **not platform residue** (no Windows peer needed) → the "accept it as platform residue" escape closes and every code-level hypothesis strengthens. Vanishes ⇒ P1 is **pairing-dependent** → platform-agnostic code mechanisms weaken. Either way the hypothesis space moves, which is why it precedes pf-1. **Then `pf-1`** (the HARD GATE), which two pf-0 results now BIND: (a) **between-session grading is dead** (a 20% recovery = 46 ms sits under the 78 ms between-session floor) — pf-1 must **measure its own paired within-session floor and register a resolution check** before grading; (b) **the fast arm is BISTABLE** — grade the run distribution, not the median.
+- **⛔ THE MAC↔MAC RIG IS NOT CLEARED TO RUN — rev 3 required (codex round 2: 3 BLOCKER, 6 HIGH, 2 LOW; 11/11 accepted, `.review/results/macmac-prereg-r2.gpt-verdict.md`).** No rig time spent. Three things must land first: **(a)** rev 2's inference is STILL overclaimed — a reproduction on these two Macs could be **macOS/APFS or host×role residue**, not a "platform-general layout cost", and a null licenses only "did not reproduce on this pair", NOT "Windows required" (and the "platform-residue escape" it claims to close **does not exist** — the parent says P1 has no escape hatch); **(b)** the power gate is **broken** — `S = max−min` is a range, not an MDE, and codex's counterexample (`d = [0,180,180,190,190,200,200,200]`, 7/8 positive, effect 83% of the 230 ms reference) returns **"VANISHES, powered=yes"**; it needs a real paired **equivalence** test (distribution-free CI on median(d_i); at n=8 the order stats `[d₍₂₎,d₍₇₎]`); **(c)** `compute_verdicts` implements **none** of the registered rule (no control gate, no clustering, no six outcomes — just PASS/FAIL), so a human would apply it after seeing the numbers. **The harness must COMPUTE the verdict; the prose must only describe what the code does.**
+- **NEXT ACTION — rev 3 of the Mac↔Mac pre-registration + harness, then run it (Queue 1(ii); the second of the two experiments that come BEFORE any pf code).** Experiment (i), the A-B-B-A MTU run, is **DONE** (pf-0 below). Rig: nagatha `10.1.10.92` ↔ `q` `10.1.10.54`, both 10GbE/9000. Pre-registered (**rev 2**, codex round 1 = 9 findings, **9/9 accepted**): `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`; harness `scripts/bench_otp12pf_mac.sh`. **⚠ CORRECTED 2026-07-14 — it is NOT an H1 discriminator.** The earlier claim here ("reproduces ⇒ **H1 DIES**, H1 accuses the *Windows* accept branch") was **WRONG and is retracted**: H1 accuses **blit's own code paths** (`SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`) — **the word "Windows" appears nowhere in H1**, and that code runs on macOS too. A Mac↔Mac reproduction is therefore *consistent with* H1, not fatal to it. What the rig **does** answer: **does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost of the destination-initiated layout?** Reproduces ⇒ P1 is **not platform residue** (no Windows peer needed) → the "accept it as platform residue" escape closes and every code-level hypothesis strengthens. Vanishes ⇒ P1 is **pairing-dependent** → platform-agnostic code mechanisms weaken. Either way the hypothesis space moves, which is why it precedes pf-1. **Then `pf-1`** (the HARD GATE), which two pf-0 results now BIND: (a) **between-session grading is dead** (a 20% recovery = 46 ms sits under the 78 ms between-session floor) — pf-1 must **measure its own paired within-session floor and register a resolution check** before grading; (b) **the fast arm is BISTABLE** — grade the run distribution, not the median.
 - **BASELINE RE-RECORD (D-2026-07-14-1, owner 2026-07-14) — a prerequisite slice for `pf-final`, NOT for pf-1.** Both committed ceilings were recorded at **MTU 1500** before the fabric went jumbo, and pf-0 showed jumbo makes both arms 3–4% faster — so a jumbo build graded against them is **LENIENT** and could let a regression pass. Each rig's baseline is **re-recorded once with its ORIGINAL old build at MTU 9000**, then re-frozen (rig W `bench_otp12_win.sh:105`; rig Z `bench_otp12_zoey.sh:102`; rig D unaffected). Constraints — same old build per rig, `BASELINE_SUMMARY` stays override-free, pf-0's start-AND-end MSS gate applies — in **D-2026-07-14-1**.
 - **pf-0 DONE — MTU is KILLED as a material cause of P1 (2026-07-14, `docs/bench/otp12-jumbo-win-2026-07-13/`).** A-B-B-A on `q` (9000/1500/1500/9000), **256 timed runs, 0 voided**, MSS gate held start AND end of every session. `Δ_9000 = 236`, `Δ_1500 = 229`, measured noise floor **N_Δ = 78 ms**, **r = −3.1% → KILLED**. The null is **not vacuous** — `wm_tcp_large` ran 3–4% faster at jumbo on **both** arms, so the manipulation reached the wire; the benefit is **symmetric**, which is why it cannot explain an **asymmetry**. codex NOT READY → **7/7 accepted** (`11f0c2a`): every finding was a *claim* outrunning the *data* (it recomputed and confirmed all the numbers). **Two limits that now bind pf-1**: (a) the run is **NOT powered** to exclude a *contributing*-size effect (20% of Δ = 46 ms < the 78 ms floor) — it excludes a DOMINANT one only; (b) 78 ms is **between**-session noise, so cross-session grading of a counterfactual is dead, and **pf-1 must measure its own paired within-session floor and register a resolution check before grading**.
 - **THE FAST ARM IS BISTABLE — the trap named in pf-1's gate above.** `win_init` runs are **bimodal** (~730/~840 ms): S1 drew 6 low/2 high and S4 drew 2 low/6 high **at the same MTU**, and that mode mixture — not MTU — is what sets N_Δ. `mac_init` is stable to 5–6 ms. A counterfactual that merely shifts the mixture would **fake a recovery**.

2026-07-14T07:56:48.720060Z ERROR codex_core::tools::router: error=timeout_ms must be at least 10000
exec
/bin/zsh -lc "bash -c 'set -euo pipefail; f(){ local out; out=\""'$(false | sed -n x | head -1)"; echo fallback; }; g(){ x="$(f)"; echo survived-g; }; g; echo survived-main'"'; echo rc="'$?
bash -c '"'set -euo pipefail; f(){ local out; out=\""'$(false | sed -n x | head -1)"; echo fallback; }; f; echo survived-main'"'; echo rc="'$?
git show e1e351d:scripts/bench_otp12pf_mac.sh | bash -n; echo bash_n_rc=$?
git diff --quiet e1e351d -- scripts/bench_otp12pf_mac.sh; echo script_diff_rc=$?
git status --short scripts/bench_otp12pf_mac.sh docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
survived-g
survived-main
rc=0
rc=1
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-rMRa8Zic' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-FpxAgvoM' (errno=Operation not permitted)
bash_n_rc=0
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-GK2beSDe' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-7ich9bdb' (errno=Operation not permitted)
script_diff_rc=0
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-MdI2BFqn' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-V220O5Xy' (errno=Operation not permitted)

exec
/bin/zsh -lc 'tail -220 .review/results/macmac-prereg-r2.codex.md' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
Reading additional input from stdin...
OpenAI Codex v0.144.3
--------
[1mworkdir:[0m /Users/michael/Dev/blit_v2
[1mmodel:[0m gpt-5.6-sol
[1mprovider:[0m openai
[1mapproval:[0m never
[1msandbox:[0m read-only
[1mreasoning effort:[0m ultra
[1mreasoning summaries:[0m none
[1msession id:[0m 019f5f6f-bc21-7602-874a-168449251f31
--------
[36muser[0m
Review commit HEAD (run: git show HEAD, and also: git show 8375c0a). This is ROUND 2 of the Mac<->Mac pre-registration (docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md rev 2) plus its NEW harness scripts/bench_otp12pf_mac.sh. Round 1 returned NOT READY (1 BLOCKER + 7 HIGH + 1 LOW), all accepted; adjudication in .review/results/macmac-prereg.gpt-verdict.md. Verify the fixes are REAL, not cosmetic, and hunt what round 1 missed.

Read: the prereg (rev 2), the harness, docs/plan/OTP12_PERF_FINDINGS.md (H1 and the pf-1 decision rule), docs/bench/otp12-jumbo-win-2026-07-13/README.md (pf-0), scripts/bench_otp12pf_linux.sh and scripts/bench_otp12_win.sh (the harnesses this one copies), .agents/machines.md.

Attack hardest here:

(1) DOES THE HARNESS IMPLEMENT THE PRE-REGISTERED RULE? Read compute_verdicts, not the prose. Check: the paired statistic (d_i per ABBA slot, D=median, S=spread), the POWER GATE (powered_for_null), the integer-exact bar, the six outcomes. Is the power-gate formula correct and conservative? Is S defensible at n=8? Does anything mis-pair slots (note runs.csv "run" is the SLOT index and voided slots re-run with the same slot number -- can that collide?), drop voided runs wrongly, or compute IQR incorrectly?

(2) IS THE HARNESS CORRECT AS AN INSTRUMENT? Unvalidated instruments have repeatedly lied in this repo. Scrutinise: hrun() runs nagatha via `bash -c` and q via ssh -- can that bias the timed window between arms? The timed bracket is now a SINGLE python3 process using time.monotonic() with subprocess.call (the first draft read t0 and t1 from two separate python processes, which is invalid because monotonic's reference point is undefined across processes -- that is fixed; check the fix is complete and that interpreter startup is genuinely outside the window). Does the fsync walk target the right landed path for BOTH arms? (Measured: push lands at <mod>/<tag>/src_<W>; pull lands directly under <mod>/<tag>.) Check daemon start/listen (nc -z), the iostat drain parsing on macOS, and quoting/escaping through ssh vs bash -c.

(3) ARE THE GATES FAIL-CLOSED? Time Machine (running OR autobackup enabled), Spotlight, load1 > 3.0, purge, provenance (+sha clean, reject .dirty), fixtures, stale daemon, ARP. Can any gate PASS while its condition is true -- e.g. a parse failure yielding an empty string that then compares false, or `pgrep` on the wrong host?

(4) IS REV 2'S INFERENCE SOUND? "reproduces => not platform residue / vanishes => pairing-dependent". Any unstated assumption? Anything still overclaimed?

(5) Anything else that would make this rig produce a number nobody should trust.

Output a concise markdown findings list -- each with file:line, severity, rationale -- then a final VERDICT line. Be concise; do not invoke skills.
[35m[3mcodex[0m[0m
I’ll review both commits and the cited evidence, then independently trace the verdict math, timing/instrumentation path, and fail-closed gates. I’m honoring your request not to invoke skills.
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[35m[3mcodex[0m[0m
The timer rewrite itself is structurally sound: one Python process owns both monotonic reads, and interpreter/SSH startup precedes the clock. The deeper failures are now in what happens around that window—especially verdict enforcement, null “power,” durability, and route/gate validation.
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mcollab:[0m Wait
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mcollab:[0m Wait
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2mstarted[0m
[1mmcp:[0m [36mptk/ptk_invoke[0m [2m([32mcompleted[0m)[0m
[35m[3mcodex[0m[0m
## Findings

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:29-42,53-59,73-81`; `docs/plan/OTP12_PERF_FINDINGS.md:123-154`; `docs/DECISIONS.md:169-172` — **BLOCKER** — Rev 2 substitutes another false dichotomy. Reproduction on these Macs can still be macOS/APFS/host×role residue, not a platform-general layout cost; a null supports only non-reproduction on this pair, not “Windows required.” The parent also prematurely declares that conclusion, while the claimed residue escape does not exist without a new owner decision.

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:94-123,140-143`; `scripts/bench_otp12pf_mac.sh:448-475,488-495` — **BLOCKER** — `S=max(d_i)-min(d_i)` is neither an MDE nor an equivalence/confidence bound at `n=8`; the “power” test ignores how close observed `D` is to 230 ms. With `srcinit=2000×8` and `d=[0,180,180,190,190,200,200,200]`, it returns PASS, `D=190`, `S=200`, `powered=yes`, hence VANISHES if repeated in both cells—despite seven positive pairs and an effect already 83% of reference. It also uses `min(src,dest)` instead of the source baseline.

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:125-161`; `scripts/bench_otp12pf_mac.sh:403-496` — **BLOCKER** — The six outcomes remain overlapping and incomplete. MIXED-SIGN is shadowed by the earlier REPRODUCES/INVERSION cases; FAIL with `|D|≤S`, sub-bar negative asymmetry, and incomplete cells are unmapped; UNDERPOWERED and UNSTABLE are extra outcomes. `compute_verdicts` implements none of the session rule, control gate, clustering, or six outcomes—it emits only per-cell PASS/FAIL/INCOMPLETE.

- `scripts/bench_otp12pf_mac.sh:280-298,323-351` — **HIGH** — The single-process monotonic rewrite fixes the original clock bug, but durable time is still two disjoint intervals. Transfer timing ends before local/SSH return; fsync begins after another dispatch and Python startup, letting arm-dependent writeback occur free. The destination is also declared drained before `sync; purge`, with no re-drain; nonnumeric `iostat` output is coerced to zero and can pass as quiet.

- `scripts/bench_otp12pf_mac.sh:144-145,209-216,302-315` — **HIGH** — Data-shape and landing validation remain fail-open. Fixtures are checked only by count, so truncated/wrong-size trees pass. `os.walk()` on a missing, unreadable, or empty landed path silently visits no files and prints `F:0:F`, accepting an unlanded transfer as a valid zero-millisecond flush.

- `scripts/bench_otp12pf_mac.sh:218-225`; `.agents/machines.md:112-130` — **HIGH** — The ARP fix is cosmetic. Failed ping is ignored; any complete nagatha→`q` ARP entry passes without comparison to the known peer MAC; interface, MTU, and media are unchecked; and `q`→nagatha is never tested. The documented own-MAC black hole and wrong-NIC route can pass this gate.

- `scripts/bench_otp12pf_mac.sh:156-188,191-226` — **HIGH** — Environmental gates are not fail-closed. `tmutil` failure becomes “not running,” AutoBackup read failure explicitly becomes `0`, Spotlight probe failure emits `0` and the last idle `mds` row can overwrite hot `mds_stores`, and probe parse failures generally become clean. Load is gated early, while the actual pre-session value is merely logged.

- `scripts/bench_otp12pf_mac.sh:82-85,111-117,191-192,511-523` — **HIGH** — The registered protocol is unenforced. `RUNS=2` is accepted, arbitrary `CELLS` can omit every control, and the documented default invocation runs 12 cells rather than the registered six. Such runs still receive ordinary verdict and `powered=yes` output; drain thresholds are also overridable without being recorded.

- `scripts/bench_otp12pf_mac.sh:92,147-153,229-237` — **HIGH** — Binary `.dirty` rejection is real, but instrument provenance is not. An edited worktree harness is labeled only with committed `HEAD`, and SHA-256 command/parse failures are not validated as 64 hex characters, so the manifest can claim “4 hashes” with missing values.

- `scripts/bench_otp12pf_mac.sh:423-425,466,484-487` — **LOW** — Even-sample medians are floored before the “integer-exact” bar, allowing a half-millisecond boundary flip. The IQR is an unspecified `x6−x3` estimator at `n=8`; it can report zero where Tukey hinges report a large spread.

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:39-42`; `docs/plan/OTP12_PERF_FINDINGS.md:3-17`; `docs/DECISIONS.md:179-180` — **LOW** — The accepted sequencing conflict was changed only in `docs/STATE.md`: the active plan still says pf-1 starts now and the settled sequence says MTU→pf-1, while rev 2 inserts Mac↔Mac before pf-1.

**VERDICT: NOT READY — 3 BLOCKER, 6 HIGH, 2 LOW.**
[2mtokens used[0m
177,891
## Findings

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:29-42,53-59,73-81`; `docs/plan/OTP12_PERF_FINDINGS.md:123-154`; `docs/DECISIONS.md:169-172` — **BLOCKER** — Rev 2 substitutes another false dichotomy. Reproduction on these Macs can still be macOS/APFS/host×role residue, not a platform-general layout cost; a null supports only non-reproduction on this pair, not “Windows required.” The parent also prematurely declares that conclusion, while the claimed residue escape does not exist without a new owner decision.

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:94-123,140-143`; `scripts/bench_otp12pf_mac.sh:448-475,488-495` — **BLOCKER** — `S=max(d_i)-min(d_i)` is neither an MDE nor an equivalence/confidence bound at `n=8`; the “power” test ignores how close observed `D` is to 230 ms. With `srcinit=2000×8` and `d=[0,180,180,190,190,200,200,200]`, it returns PASS, `D=190`, `S=200`, `powered=yes`, hence VANISHES if repeated in both cells—despite seven positive pairs and an effect already 83% of reference. It also uses `min(src,dest)` instead of the source baseline.

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:125-161`; `scripts/bench_otp12pf_mac.sh:403-496` — **BLOCKER** — The six outcomes remain overlapping and incomplete. MIXED-SIGN is shadowed by the earlier REPRODUCES/INVERSION cases; FAIL with `|D|≤S`, sub-bar negative asymmetry, and incomplete cells are unmapped; UNDERPOWERED and UNSTABLE are extra outcomes. `compute_verdicts` implements none of the session rule, control gate, clustering, or six outcomes—it emits only per-cell PASS/FAIL/INCOMPLETE.

- `scripts/bench_otp12pf_mac.sh:280-298,323-351` — **HIGH** — The single-process monotonic rewrite fixes the original clock bug, but durable time is still two disjoint intervals. Transfer timing ends before local/SSH return; fsync begins after another dispatch and Python startup, letting arm-dependent writeback occur free. The destination is also declared drained before `sync; purge`, with no re-drain; nonnumeric `iostat` output is coerced to zero and can pass as quiet.

- `scripts/bench_otp12pf_mac.sh:144-145,209-216,302-315` — **HIGH** — Data-shape and landing validation remain fail-open. Fixtures are checked only by count, so truncated/wrong-size trees pass. `os.walk()` on a missing, unreadable, or empty landed path silently visits no files and prints `F:0:F`, accepting an unlanded transfer as a valid zero-millisecond flush.

- `scripts/bench_otp12pf_mac.sh:218-225`; `.agents/machines.md:112-130` — **HIGH** — The ARP fix is cosmetic. Failed ping is ignored; any complete nagatha→`q` ARP entry passes without comparison to the known peer MAC; interface, MTU, and media are unchecked; and `q`→nagatha is never tested. The documented own-MAC black hole and wrong-NIC route can pass this gate.

- `scripts/bench_otp12pf_mac.sh:156-188,191-226` — **HIGH** — Environmental gates are not fail-closed. `tmutil` failure becomes “not running,” AutoBackup read failure explicitly becomes `0`, Spotlight probe failure emits `0` and the last idle `mds` row can overwrite hot `mds_stores`, and probe parse failures generally become clean. Load is gated early, while the actual pre-session value is merely logged.

- `scripts/bench_otp12pf_mac.sh:82-85,111-117,191-192,511-523` — **HIGH** — The registered protocol is unenforced. `RUNS=2` is accepted, arbitrary `CELLS` can omit every control, and the documented default invocation runs 12 cells rather than the registered six. Such runs still receive ordinary verdict and `powered=yes` output; drain thresholds are also overridable without being recorded.

- `scripts/bench_otp12pf_mac.sh:92,147-153,229-237` — **HIGH** — Binary `.dirty` rejection is real, but instrument provenance is not. An edited worktree harness is labeled only with committed `HEAD`, and SHA-256 command/parse failures are not validated as 64 hex characters, so the manifest can claim “4 hashes” with missing values.

- `scripts/bench_otp12pf_mac.sh:423-425,466,484-487` — **LOW** — Even-sample medians are floored before the “integer-exact” bar, allowing a half-millisecond boundary flip. The IQR is an unspecified `x6−x3` estimator at `n=8`; it can report zero where Tukey hinges report a large spread.

- `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:39-42`; `docs/plan/OTP12_PERF_FINDINGS.md:3-17`; `docs/DECISIONS.md:179-180` — **LOW** — The accepted sequencing conflict was changed only in `docs/STATE.md`: the active plan still says pf-1 starts now and the settled sequence says MTU→pf-1, while rev 2 inserts Mac↔Mac before pf-1.

**VERDICT: NOT READY — 3 BLOCKER, 6 HIGH, 2 LOW.**

exec
/bin/zsh -lc "nl -ba .agents/machines.md | sed -n '85,145p'
nl -ba docs/plan/OTP12_PERF_FINDINGS.md | sed -n '1,210p'
nl -ba docs/DECISIONS.md | sed -n '155,190p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
    85	  `ssh root@zoey 'cat /sys/class/net/enp0s0/mtu'` → want 9000.
    86	  Method for any risky remote NIC change: arm a self-healing revert
    87	  first — `nohup setsid bash -c 'sleep 90; [ -f /tmp/ok ] || ip link set
    88	  IFACE mtu 1500' &` — then confirm with `touch /tmp/ok`. Change the NIC
    89	  you are NOT ssh'd through when a second one exists.
    90	- **Live NFS/TCP connections do NOT pick up a new MTU.** MSS is fixed at
    91	  connect time, so an existing mount keeps its old segment size until it
    92	  reconnects (reboot/remount). Not worth forcing for low-bandwidth
    93	  mounts.
    94	- Two-NICs-on-one-subnet (both `altiera` and `zoey`, and it is the
    95	  default `arp_ignore=0 arp_announce=0`) invites ARP flux + asymmetric
    96	  routing. Working today; a latent source of intermittent stalls.
    97	- Local VM on the Mac — Ubuntu ARM (aarch64), per owner. Build-only
    98	  fallback likewise.
    99	
   100	## `q` — THE DEDICATED BENCH MAC (new 2026-07-13; use this, not nagatha)
   101	
   102	`ssh michael@q` — Apple **M4 Mac mini**, 16 GB, macOS 26.5.2, arm64. It is now
   103	the rig-W Mac end: **quiet, dedicated, and faster than nagatha** (1 GiB in
   104	~908 ms ≈ 1.18 GB/s, vs nagatha's ~1.3–1.8 s). Using it **decouples the codex
   105	review loop from rig-W benchmarking** — the contention that destroyed a
   106	53-minute experiment (below).
   107	
   108	- **10GbE**: `en8` = **10.1.10.54**, MTU **9000**, media 10Gbase-T. This is the
   109	  **Aquantia adapter physically moved off nagatha**, so nagatha's 10GbE is now a
   110	  *different* NIC at **10.1.10.92** (also MTU 9000). Any doc naming
   111	  "Aquantia @ .54 on nagatha" is stale.
   112	- **⚠ THE MULTI-NIC ROUTING TRAP (cost ~1h).** `q` has THREE IPs on
   113	  10.1.10.0/24 — `en0` (1GbE, .221), `en1` (Wi-Fi, .108), `en8` (10GbE, .54) —
   114	  and macOS routes the subnet via the highest-ranked **network service**, not by
   115	  which IP "matches". `en0` outranked `en8`, so **every benchmark would have run
   116	  over gigabit**. Fixed by promoting the service that owns `en8` — confusingly
   117	  named **"Thunderbolt Ethernet Slot 3"** — to rank 1
   118	  (`sudo networksetup -ordernetworkservices …`). It has the same router
   119	  (10.1.10.1), so `q` keeps its internet.
   120	- **DO NOT "fix" this with a host route.**
   121	  `sudo route -n add -host 10.1.10.177 -interface en8` on a *directly-connected*
   122	  subnet installs a next hop of **the interface's own MAC** — a black hole. It
   123	  drops 100% of packets while `route -n get` still cheerfully reports
   124	  `interface: en8`. Verify with `arp -n <peer>`: the MAC must be the PEER's, not
   125	  `q`'s (`00:01:d2:19:04:a3`).
   126	- **An ssh transfer CANNOT verify this link.** ssh caps at ~79 MB/s on this path
   127	  (nagatha's known-good 10GbE scores the same 79), which is *below* the gigabit
   128	  ceiling — so a degraded link and a healthy one look identical through it. Use
   129	  `ifconfig en8 | grep media` (the PHY's negotiated rate) and blit's own
   130	  `wm_tcp_large` time (~908 ms for 1 GiB = 10GbE; ~10 s = 1GbE).
   131	- **Staged**: repo clone at `~/Dev/blit_v2_f35702a` (detached `f35702a`, cloned
   132	  from the LOCAL gitea — `q` *is* the gitea host); `target/release/{blit,blit-daemon}`
   133	  arm64 copied from nagatha (embed-verified `+f35702a`); old client at
   134	  `~/blit-bench-work/bins/blit-0f922de`; fixtures in `~/blit-bench-work`.
   135	  NOPASSWD `/usr/sbin/purge` granted (`/etc/sudoers.d/blit-bench`, mode 0440 —
   136	  `visudo -c` rejects any other mode). ssh key authorized on netwatch-01 in
   137	  **`C:\ProgramData\ssh\administrators_authorized_keys`** (michael is an admin
   138	  there, so the per-user file is ignored). macOS firewall is OFF on `q`.
   139	- **`q` RUNS GITEA** (it is `origin`, `http://q:3000`). It idles cheaply, but
   140	  **do not push to `origin` during a benchmark session**.
   141	
   142	## THE MAC IS A BENCH END — keep it quiet (recorded 2026-07-13, learned the hard way)
   143	
   144	**A rig-W (Mac↔Windows) benchmark requires a QUIET Mac.** The Mac is not a
   145	neutral driver: it runs the client in `mac_init` arms and serves the daemon in
     1	# otp-12 perf findings — investigate + fix before acceptance (design)
     2	
     3	**Status**: Active
     4	**Approved**: D-2026-07-13-1 — owner, 2026-07-13, verbatim:
     5	**"one more round with codex on the plan then just write the code and
     6	reviewloop slice by slice. that converges faster than plans with no
     7	ground truth to test."** The final round ran (round 5, verdict NOT READY,
     8	3 blockers — F1 the missing P1 escape, F2 the non-isolating H1
     9	counterfactual, F3 the inexecutable decision rule); all three are fixed
    10	in this revision, and implementation now proceeds **slice by slice, each
    11	through the codex loop** (D-2026-07-04-1 unchanged). A non-converged plan
    12	verdict is no longer a gate — the plan's earlier "flip to Active at codex
    13	convergence" rule is superseded by D-2026-07-13-1, because rounds 2–5
    14	were increasingly finding defects in the *prose* while the plan's central
    15	factual claim was settled by *measurement* (the same-OS rig refuted a
    16	claim four review rounds had left standing). pf-1 exists to generate
    17	ground truth; it starts now.
    18	
    19	**⚠ THE DECISION P1 NEEDS (surfaced round 5, owner's to make — NOT
    20	assumed by this plan):** P1 has **no escape hatch on the books**.
    21	D-2026-07-12-1 waives a cross-direction converge-up miss only for a cell
    22	that is *already* invariance-passing; P1 is the invariance failure
    23	itself. So P1 must either be **FIXED** (≤1.10 on rig W — the default this
    24	plan pursues) or the owner must **amend acceptance criterion 1** in a new
    25	decision. pf-1 proceeds either way: it produces the evidence that
    26	decision would rest on.
    27	**Created**: 2026-07-12
    28	**Parent**: `docs/plan/ONE_TRANSFER_PATH.md` (Active), whose Constraints
    29	say the quiet part: "Unification that slows the fast direction fails
    30	review." P1 is a miss of the parent's HEADLINE acceptance criterion
    31	(initiator/verb invariance, ±10%) — not a nice-to-have.
    32	**Contract**: `docs/TRANSFER_SESSION.md` — no wire changes are expected;
    33	if an investigation slice needs one, it stops and this doc is amended
    34	through the loop first.
    35	
    36	**Sequencing (corrected 2026-07-13).** This doc originally deferred
    37	otp-12c/12d/13 outright. In fact **otp-12c RAN on 2026-07-13** under a
    38	fresh in-session owner go (rig D delegated parity + a rig-W re-baseline
    39	at the cutover sha `f35702a`; `docs/bench/otp12c-{delegated,win}-2026-07-13/`).
    40	That does not change this plan's standing, and the rows are not lost
    41	work — under `pf-final` they are **pre-fix rows, void for acceptance**,
    42	but they serve two real purposes: (a) an **independent replication** of
    43	both findings at the shipped sha (below), which is exactly the
    44	independent corroboration the round-2 review said P1 lacked; and (b) the
    45	pre-pf-1 control the investigation needs. **otp-12d and otp-13 remain
    46	deferred** until P1/P2 are fixed or explained at code level — assembling
    47	an acceptance matrix out of pre-fix rows would build the artifact otp-13
    48	walks from rows this plan declares void.
    49	
    50	## The two findings (evidence, both committed)
    51	
    52	**P1 — destination-initiated TCP mixed transfers pay ~25–30%**
    53	(`docs/bench/otp12-win-2026-07-12/`, replicated in
    54	`docs/bench/otp12c-win-2026-07-13/`). `wm_tcp_mixed` invariance FAILs in
    55	**two independent sessions**, and got WORSE at the shipped sha:
    56	
    57	| session | build | mac_init | win_init | ratio | arm spreads |
    58	|---|---|---|---|---|---|
    59	| 12b (2026-07-12) | `e21cf84` | 1127 | 911 | **1.237** | 8.2 / 3.3% |
    60	| 12c-win (2026-07-13) | `f35702a` (cutover) | 1221 | 939 | **1.300** | 6.4 / 8.4% |
    61	
    62	Corroborated by block-1 `pull_tcp_mixed` new-vs-old-same-session:
    63	**1.313** (12b: 1138/867) and **1.247** (12c-win: 1192/956).
    64	
    65	**This cannot be re-run away.** Both sessions' arm spreads are far below
    66	D2's 25% escalation trigger, so no escalation session is even available;
    67	the cells stand as measured. (The 12c-win session was a fresh staging on
    68	a different day at a different sha — the round-2 review's objection that
    69	the 1.313 corroboration was "same rig/session, not independent" is now
    70	answered by an independent session reproducing the same cell.)
    71	
    72	**What the evidence actually supports — and the confound it does NOT
    73	escape** (corrected, review round 3; an earlier draft of this section
    74	claimed the `mw` cell was a clean control isolating "destination
    75	initiation" as the cause. It is not, and the correction matters because
    76	it re-aims the hypotheses):
    77	
    78	Every invariance cell compares two arms that share the same endpoints
    79	and the same data direction, so **within** a cell the initiator is the
    80	only variable — that part is clean. Arm medians (12c-win):
    81	
    82	| cell | data direction | dest-initiated arm | source-initiated arm | ratio | spreads |
    83	|---|---|---|---|---|---|
    84	| `wm_tcp_mixed` | Win→Mac | 1221 | 939 | **1.300 FAIL** | 6.4 / 8.4% |
    85	| `mw_tcp_mixed` | Mac→Win | 1477 | 1415 | 1.044 PASS | 20.8 / 20.5% |
    86	
    87	The initiator penalty is therefore **real and large in the Win→Mac
    88	direction only**. In Mac→Win the two layouts are within noise, and the
    89	ordering even **flips between sessions** (12b: dest-initiated 1502 was
    90	*faster* than source-initiated 1587), on spreads of 17–25%.
    91	
    92	Crossing from `wm` to `mw` is **not** a controlled swap of one variable:
    93	it also swaps the destination filesystem (APFS vs NTFS), the TCP stack,
    94	which host runs the client, and the flush method. So the supported
    95	signature is an **interaction — TCP × mixed × Win→Mac × initiator** —
    96	not "destination initiation" on its own.
    97	
    98	Worse, on a two-host rig the failing configuration is **confounded by
    99	construction**: in the slow arm the destination is the Mac (which dials)
   100	*and* the source is Windows (which accepts). With only two hosts, **host
   101	identity IS role** — "Mac-as-dialing-destination" and
   102	"Windows-as-accepting-source" are the same configuration and cannot be
   103	separated by any number of additional runs on this rig.
   104	
   105	### THE CONFOUND IS BROKEN — and it breaks toward PLATFORM (2026-07-13)
   106	
   107	**Evidence: `docs/bench/otp12-perf-2026-07-13/` — magneto↔skippy, Linux on
   108	BOTH ends, real 10 GbE, full otp-12 methodology** (cold caches both ends,
   109	destination drained, ABBA, pair-void, RUNS=4; 64 runs, 8/8 cells, zero
   110	voided). Harness `scripts/bench_otp12pf_linux.sh`.
   111	
   112	**P1 does NOT reproduce.** Its own cell passes with room to spare:
   113	
   114	| cell | srcinit | destinit | ratio | outcome |
   115	|---|---|---|---|---|
   116	| `sm_tcp_mixed` (P1's cell) | 1745 | 1905 | **1.092** | PASS |
   117	| `ms_tcp_mixed` (P1's cell) | 2085 | 2079 | **1.003** | PASS |
   118	
   119	**8/8 invariance cells PASS** (`ms_grpc_mixed` via its pre-registered
   120	RUNS=8 escalation → 1.063). There is no destination-initiator penalty at
   121	all when both ends are Linux.
   122	
   123	Therefore:
   124	
   125	- **P1 requires the Mac↔Windows pairing.** It is NOT a pure layout
   126	  property of blit's code — a pure layout cost would have appeared here,
   127	  on the same code, same carrier, same fixture.
   128	
   129	- **⚠ BUT P1 HAS NO ESCAPE HATCH TODAY (review round 5, BLOCKER).** An
   130	  earlier revision of this section said D-2026-07-12-1 lets the owner
   131	  accept P1 as a platform residue. **It does not.** That decision excuses
   132	  a **cross-direction converge-up** miss for a cell that has ALREADY
   133	  satisfied its precondition **"(b) is initiator/verb-invariant within
   134	  ±10%"** (`docs/DECISIONS.md` D-2026-07-12-1). **P1 IS the invariance
   135	  failure** (`wm_tcp_mixed` 1.300 FAIL) — the precondition it would need
   136	  is the very thing it violates. No decision on the books waives it.
   137	  Therefore exactly two exits exist, and pf-1 must aim at them:
   138	  1. **FIX IT** — P1 ≤ 1.10 on rig W. This remains the default and the
   139	     bar (`ONE_TRANSFER_PATH.md` acceptance criterion 1 is mandatory).
   140	  2. **A NEW OWNER DECISION amending criterion 1** — for which the
   141	     same-OS result is the honest evidence base: criterion 1 asks for
   142	     invariance "on a symmetric rig", Mac↔Windows was designated only
   143	     because no better pair existed, and one now does — magneto↔skippy,
   144	     where blit measures **8/8 invariant**. An owner could reasonably
   145	     rule that criterion 1 is judged on the rig that isolates blit's own
   146	     behaviour, with the Mac↔Windows delta recorded as platform residue.
   147	     **That ruling does not exist. It must not be assumed, and this plan
   148	     must not be written as though it will be granted.**
   149	- **This does NOT fully exonerate the code.** It rules out a pure layout
   150	  property; it does not rule out a code path whose cost only becomes
   151	  material under a particular platform — e.g. a slow accept branch on the
   152	  Windows side, which is exactly what H1 accuses. H1/H5/H6 stay LIVE but
   153	  are now **narrowed to platform-interacting mechanisms**, and only the
   154	  dial/accept inversion counterfactual on rig W can finish the job.
   155	- **P2 is untested by this rig** (it is a converge bar vs the OLD build,
   156	  and no `0f922de` build is staged on these hosts). Nothing here speaks
   157	  to it.
   158	
   159	> **⚠ A RETRACTED CLAIM LIVED HERE.** An earlier revision of this section
   160	> asserted the opposite — "P1 reproduces at 1.78 → the confound breaks
   161	> toward CODE → the fix is mandatory and cannot be waived" — and STATE and
   162	> the acceptance plan were amended to match. That was **WRONG**. It rested
   163	> on a scratch probe (and a first harness revision) that ran the durability
   164	> `sync` inside the INITIATING host's timed bracket: in the push arm the
   165	> initiator is the SOURCE, which only read, so its sync was a no-op and the
   166	> destination's writeback was never paid; in the pull arm the initiator IS
   167	> the destination, so it paid the full writeback. One arm was charged for
   168	> durability the other got free — multi-second on skippy's ZFS — which
   169	> manufactured "failures" on every carrier and fixture, **including the
   170	> gRPC control that is supposed to be clean**. That carrier-independence is
   171	> what exposed it: a real code effect is carrier-specific; an accounting
   172	> artifact is not. Fixed at `2c0af86` (durability keyed by DESTINATION,
   173	> never by verb — the otp-2w rule, re-learned). The retraction is recorded
   174	> rather than quietly overwritten because the wrong number was reported to
   175	> the owner and briefly drove this plan.
   176	
   177	### The residual confound (WHICH code) still needs a counterfactual
   178	
   179	Breaking platform-vs-code does NOT tell us *which* layout property costs
   180	the time. On any two-host rig, host identity remains welded to role, so
   181	"the accepting end" cannot be separated from "that host" by more runs:
   182	
   183	- **pf-1 must compare all four rig-W arms** (both cells × both
   184	  initiators), not two, and report the interaction — not a single ratio.
   185	- **The disambiguator is a dial/accept inversion counterfactual, not a
   186	  rig** — but it is **NOT sufficient on its own** (review round 5): the
   187	  inversion swaps the source's `Accept`, the destination's `Dial`, AND
   188	  the epoch-0 topology **simultaneously**, so a positive result implicates
   189	  *the topology pair*, not H1 specifically. It cannot distinguish
   190	  source-accept serialization from synchronous destination dialing
   191	  (`transfer_session/mod.rs:3113`), nor prove the resize-specific claim.
   192	  pf-1 therefore runs **three ablations, not one**, each varying ONE thing:
   193	  1. **dial/accept inversion** — same direction, same hosts, same fixture;
   194	     only who dials changes. Implicates the topology pair (or exonerates it).
   195	  2. **no-resize / pre-opened streams** — force the final stream count at
   196	     epoch 0 so no resize epoch ever fires. If the gap survives with zero
   197	     resizes, H1's resize-specific mechanism is **KILLED** regardless of
   198	     what (1) shows (and note `dial.rs:474`: all three fixtures already
   199	     target 8 streams, so resize *count* was never the discriminator).
   200	  3. **per-side ordering** — hold the topology fixed and vary only whether
   201	     the destination's dial-before-ACK is synchronous. Separates the two
   202	     halves the inversion conflates.
   203	  H1 is CONFIRMED only if the wall-time recovery tracks the **accept role**
   204	  across (1) AND survives (2); it is KILLED if the gap persists with no
   205	  resizes, or if (3) shows the cost is the synchronous dial rather than the
   206	  accept branch. Any of these that changes connection topology — (1) and
   207	  (2) do — **trips this plan's Contract stop-and-amend rule**
   208	  (`TRANSFER_SESSION.md` amended through the loop BEFORE the flag is
   209	  written). Same-build-both-ends (D-2026-07-05-2) means no compatibility
   210	  surface is created.
   155	- Decision: The session's resume block phase is bounded so no legal open can produce a frame the gRPC-served in-stream carrier cannot deliver, nor an amplified hash list (codex otp-7a F1). The DESTINATION clamps `ResumeSettings.block_size` into **[64 KiB, 2 MiB]** (`MIN_RESUME_BLOCK_SIZE`, `MAX_IN_STREAM_RESUME_BLOCK_SIZE`; `0` ⇒ 1 MiB default) — floor kills block_size=1's 32× hash-list amplification, ceiling keeps a one-block `BlockTransfer` frame under tonic's default 4 MiB decode limit — and caps any one `BlockHashList` at **65_536 hashes** (2 MiB of hashes); a partial with more blocks degrades to the empty list, i.e. the plan-D1 graceful full-transfer fallback, never an oversized frame. The SOURCE range-validates the wire block size at frame arrival (same-build peers, D-2026-07-05-2: out-of-range is a protocol violation, not a negotiation). otp-7b revisits the ceiling for the TCP data plane, whose binary block records carry no protobuf envelope.
   156	- Why: plan D5 as drafted clamped only to `MAX_BLOCK_SIZE` (64 MiB), which is fine for local block copies but 16× over the unraised tonic frame limit the served in-stream carrier actually has — a legal open would fail mid-transfer (RELIABLE violation), and a hostile-or-buggy tiny block size would OOM-amplify the hash list. Pinned by `resume_block_size_floor_clamps_tiny_requests`, `resume_block_size_ceiling_clamps_oversized_requests` (guard-proven by clamp removal), and the pure-fn cap boundary test.
   157	- Supersedes: OTP7_RESUME.md D5's "clamped to `MAX_BLOCK_SIZE`" wording (amended in place, same commit).
   158	
   159	## D-2026-07-10-2 — Resume block-size ceiling is per carrier (completes the D-2026-07-10-1 revisit)
   160	- Decision: The resume block-size ceiling the DESTINATION clamps to (and the SOURCE range-validates at `BlockHashList` arrival) is **the carrier's**: **2 MiB** on the in-stream carrier (unchanged, D-2026-07-10-1) and **64 MiB** on the TCP data plane (`MAX_DATA_PLANE_RESUME_BLOCK_SIZE` = the receive pipeline's `MAX_WIRE_BLOCK_BYTES` = the old resume path's `MAX_BLOCK_SIZE`). Both ends decide by grant presence — grant ⇒ data plane — so same-build peers agree without negotiation. The floor (64 KiB) and the 65_536-hash `BlockHashList` cap are carrier-independent (the hash list always rides the control lane as protobuf); a partial with more blocks than the cap still degrades to the D1 full-transfer fallback. Session-wide block size stays; per-file block-size auto-scaling for very large partials (>4 TiB at 64 MiB blocks) remains future work.
   161	- Why: binary data-plane `BLOCK` records carry no protobuf envelope, so the 2 MiB tonic-frame rationale does not apply there; the wire already enforces `MAX_WIRE_BLOCK_BYTES` = 64 MiB on the receive side. A larger ceiling lets a data-plane session keep block-wise resume for partials up to 4 TiB (65_536 × 64 MiB) instead of degrading to full transfer at 128 GiB (the 2 MiB-ceiling limit).
   162	- Supersedes: nothing — completes the revisit D-2026-07-10-1 explicitly deferred to otp-7b (OTP7_RESUME.md D5 amended in place, same commit).
   163	
   164	## D-2026-07-11-1 — `--relay-via-cli` removed; remote→remote is delegated-only
   165	- Decision: The `--relay-via-cli` escape hatch is **removed** (owner, 2026-07-11, otp-10c-1). Remote→remote transfers are delegated-only; the CLI is never in the byte path. The relay's read half was the PullSync client's on-demand per-file remote read — a capability the unified session deliberately does not have — so PullSync's deletion (otp-10c) makes a streaming relay unrebuildable; offered the choice, the owner picked removal over a stage-to-temp-dir reimplementation. The topology the flag served (destination cannot reach source, CLI can reach both) is handled by two manual commands — pull to a local path, then push it — and the delegated CONNECT_SOURCE error hint now says exactly that. `RemoteTransferSource`, its `remote_transfer_source_constructed` counter, and every relay-combination gate (mirror/move/detach/resume × relay) die with the flag; the delegated no-CLI-byte-path pin (`cli_data_plane_outbound_bytes == 0`) remains the byte-path-isolation proof and doubles as the otp-10 deletion proof's CLI half.
   166	- Why: unshipped product (no compat bar, D-2026-07-05-2 era posture); the ONE_TRANSFER_PATH directive is deletion of bespoke side paths, and a staged relay would merely automate what two commands already do — not worth a maintained transfer-adjacent code path.
   167	- Supersedes: `REMOTE_REMOTE_DELEGATION_PLAN.md` (already Historical) §§relay-fallback/escape-hatch design — dated header note added, body kept verbatim per that doc's own precedent; the relay-combination gates R50-F1/R51-F2 (move×relay), audit-h1 rounds 1–2 (mirror×relay), codex otp-10a F4 (resume×relay), and the detach×relay gate — deleted with the flag they guarded (their data-loss reasoning is moot once no relay path exists to combine with); `scripts/bench_remote_remote.sh`'s relay leg (removed same commit).
   168	
   169	## D-2026-07-12-1 — otp-12 cross-direction bar: platform-residue cells count as satisfied
   170	- Decision: On the owner-designated cross-direction rig (Mac↔Windows), an otp-12 cell that (a) meets per-direction converge-up (new ≤ old, same direction, ±10%) and (b) is initiator/verb-invariant within ±10%, but (c) exceeds `min(old_push, old_pull) × 1.10` with the platform-residue discriminator attributing the gap to the destination write path (the old arm shows the same direction gap in the same interleaved session), **counts as satisfying the cross-direction half of ONE_TRANSFER_PATH acceptance criterion 2** (owner "yes", 2026-07-12, to the plain-English framing: "if a direction is exactly as fast as the old code, and the time is identical no matter which machine starts the copy, but it still misses that bar purely because of the Windows write path — does it count as a pass?"). The evidence README still records BOTH computations per cell (the F4 arithmetic and the discriminator); the otp-13 walk reviews the numbers, but a platform-residue cell is not a blocker.
   171	- Why: the plan's Non-goals already exclude making different hardware perform identically, and D-2026-07-05-1 restricts cross-direction verdicts to symmetric endpoints; no truly fs-identical pair exists in the fleet, so on the designated closest-spec rig the "better of the two old directions" bar can only bind net of the destination write-path residue the discriminator isolates. Settling the rule before the run prevents re-litigating it with numbers in hand.
   172	- Supersedes: nothing — refines how acceptance criterion 2 is evaluated on the designated rig (`docs/plan/ONE_TRANSFER_PATH.md` criterion annotated in place, same commit); resolves `docs/plan/OTP12_ACCEPTANCE_RUN.md` Q1 (rewritten as resolved, same commit).
   173	
   174	## D-2026-07-13-1 — OTP12_PERF_FINDINGS goes Active after one final codex round; implementation proceeds slice-by-slice
   175	- Decision: `docs/plan/OTP12_PERF_FINDINGS.md` flips **Draft → Active** after ONE final codex round, and implementation then proceeds regardless of whether that round returns a "converged" verdict — owner, 2026-07-13, verbatim: **"one more round with codex on the plan then just write the code and reviewloop slice by slice. that converges faster than plans with no ground truth to test."** Each code slice still goes through the codex review loop (D-2026-07-04-1, unchanged); what is retired is *plan-only* iteration as the gate on starting work. The plan's own Status line ("the flip to Active happens at codex convergence") is amended by this decision: the round happens, its accepted findings are fixed, and then code starts — a non-converged verdict is no longer a blocker, it is input to the first slice.
   176	- Why: rounds 2–4 each returned real findings, but they were increasingly findings about the *plan text* (falsifiability wording, thresholds, bar phrasing) rather than about reality, and the plan's central factual claim was settled not by review but by *measurement* — the same-OS rig, which refuted a claim four review rounds had left standing (`docs/bench/otp12-perf-2026-07-13/`; a wrong "P1 is code" claim was reported and retracted the same day). Ground truth comes from instrumented code and rigs, not from more prose; pf-1 exists precisely to generate it. Continuing to polish the plan has diminishing returns against the cost of not yet having a single measured counterfactual.
   177	- Supersedes: the "flip to Active at codex convergence" gate in `OTP12_PERF_FINDINGS.md`'s Status line (rewritten in place, same commit). Does NOT supersede D-2026-07-04-1 — every code slice is still codex-reviewed before the next begins.
   178	
   179	## D-2026-07-13-2 — the local small-file finding queues BEHIND OTP12_PERF_FINDINGS
   180	- Decision: `docs/plan/LOCAL_SMALL_FILE_PATH.md` (Draft) is sequenced **behind** the ACTIVE `docs/plan/OTP12_PERF_FINDINGS.md` — the MTU experiment, then pf-1, then its fix slices. Owner, 2026-07-13, verbatim: **"well, odds that one affects the other? if this is contributory, would we know? probably irrelevant. behind."** No local-path code lands until otp-12's investigation has its attribution. The finding itself (blit vs robocopy, local `D: -> E:`, `docs/bench/win-local-ab-2026-07-13/`) is recorded now; only the *fix* waits.
   181	- Why: two reasons, one causal and one procedural. **Causal**: the local finding is very unlikely to explain either otp-12 finding. P1 is an *initiator-invariance* failure — both arms run identical code and differ only in who dials, so a worker-count or per-file cost cancels between them, and a local copy has no initiator axis at all. P2 is a *new-vs-old* regression, whereas the local cost is *old*: otp-11's own gate measured old-vs-new local `small` at 1684 -> 1750 ms (+3.9% PASS, `docs/bench/otp11-local-2026-07-11/`) and otp-11 D1 explicitly preserved the old pipeline's payload shapes (`PreparedPayload::File`/`TarShard` "exactly as the old local pipeline"). A long-standing cost cannot produce a new regression. **Procedural**: fixing local *first* would touch code shared with the wire sink, perturb P1/P2 mid-investigation, and void the pre-fix baselines pf-final depends on — destroying the attribution rather than adding to it. Sequencing behind keeps every counterfactual legible, and pf-final's full-matrix rerun would still surface any shared-code effect as a number.
   182	- Carried into pf-1 as a cheap check (the one way the two could touch): the local apply pipeline runs **one** worker by default (`transfer_session/local.rs:602`, `sink_workers` is 1 unless the hidden `--workers` flag sets `debug_mode`). If the unified session likewise changed the **remote receive** side's worker count versus old push, that WOULD be new, per-file, and a live P2 candidate. Establish it by reading the executed old path, not by assuming.
   183	- Supersedes: nothing. Adds `LOCAL_SMALL_FILE_PATH.md` to the `docs/STATE.md` queue behind item 1a.
   184	
   185	## D-2026-07-13-3 — Windows attribute/ADS loss is a real gap; fix it AFTER otp-12
   186	- Decision: `blit` silently drops Windows file attributes (ReadOnly/Hidden/System) and alternate data streams on the tar-shard path — **on both the local and the remote route**, exit code 0, no warning — and it will be **fixed after the current phase (otp-12) completes**, not now. Owner, 2026-07-13, verbatim: **"well that, while funny, makes sense. we started this as a linux alternative to robocopy, and full windows support was always a goal... but obviously not landed. so, good, let's address that. after this current phase is complete."** Finding, repro, and root cause: `docs/bugs/windows-attrs-and-ads-lost-on-tar-path.md`.
   187	- Framing (owner's, and it is the correct one): this is **unlanded Windows support**, NOT a regression. blit began as a Linux alternative to robocopy; full Windows parity was always a goal and the metadata half never shipped. It predates the unified session and is not P1, P2, or otp-11 fallout.
   188	- What makes it more than a missing feature: the loss is **conditional on file count**, so it is silent and non-obvious. `transfer_plan.rs:103-109` sends a transfer down the tar path when there are ≥2 small files AND (≥32 of them OR average ≤128 KiB); otherwise files go through `CopyFileExW`, which carries attributes and ADS for free. So the SAME file keeps its metadata when copied alone and loses it when copied alongside 39 siblings. Proven with identical 200 KiB files where only the count varied (40 → LOST, 3 → PRESERVED), locally and over the wire.
   189	- **Fixing it is a WIRE CONTRACT change.** The tar shard is the wire payload format for small files, so carrying attributes/ADS means extending the shard header or the manifest — a frame change, which trips the stop-and-amend rule: `docs/TRANSFER_SESSION.md` is amended through the codex loop BEFORE any code. Same-build-both-ends (D-2026-07-05-2) means no compatibility surface is created, but the contract doc still governs. The header-vs-manifest choice is a design decision reserved for the owner.
   190	- Sequencing: behind otp-12, and **planned together with `LOCAL_SMALL_FILE_PATH.md`** (D-2026-07-13-2) — they touch the same tar path and pull in opposite directions (a fidelity fix ADDS per-file work to a path already losing 1.9× to robocopy at equal thread count). Planning them separately would optimise one against the other.

exec
/bin/zsh -lc 'rg -n "00:01:d2|MAC|mac address|ether " .agents/machines.md docs/bench/otp12-macmac-2026-07-14 scripts/bench_otp12pf_mac.sh | head -100
rg -n "bench_otp12pf_mac|macmac" docs/DECISIONS.md docs/plan/OTP12_PERF_FINDINGS.md docs/STATE.md | head -100' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
scripts/bench_otp12pf_mac.sh:3:# bench_otp12pf_mac.sh — THE MAC<->MAC RIG (nagatha <-> q), the missing 2x2 cell
scripts/bench_otp12pf_mac.sh:58:#   * TIME MACHINE gate on BOTH Macs — the hole pf-0 found: the old quiet-gate
scripts/bench_otp12pf_mac.sh:219:  # must be the PEER's MAC, never our own — a host route on a directly-connected
.agents/machines.md:100:## `q` — THE DEDICATED BENCH MAC (new 2026-07-13; use this, not nagatha)
.agents/machines.md:122:  subnet installs a next hop of **the interface's own MAC** — a black hole. It
.agents/machines.md:124:  `interface: en8`. Verify with `arp -n <peer>`: the MAC must be the PEER's, not
.agents/machines.md:125:  `q`'s (`00:01:d2:19:04:a3`).
.agents/machines.md:142:## THE MAC IS A BENCH END — keep it quiet (recorded 2026-07-13, learned the hard way)
.agents/machines.md:186:  * Harnesses take the Mac IP via `MAC_HOST=` — pass **10.1.10.54**
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:40:whether P1 **must be fixed in code** or **could be accepted as platform residue**.
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:77:direction demonstrates the layout cost without a Windows peer.** Whether it is
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:167:- **QUIESCENCE, BOTH MACS.** Refuse to start if `codex`/`cargo`/`rustc` runs on
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:171:- **TIME MACHINE, BOTH MACS — FAIL-CLOSED, not a warning.** Refuse to start if a
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:177:- **SPOTLIGHT, BOTH MACS.** `mds_stores` is a recorded contaminant
docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:195:- **Link validity, measured not assumed**: peer ARP resolves to the **peer's** MAC
docs/STATE.md:5:- **⛔ THE MAC↔MAC RIG IS NOT CLEARED TO RUN — rev 3 required (codex round 2: 3 BLOCKER, 6 HIGH, 2 LOW; 11/11 accepted, `.review/results/macmac-prereg-r2.gpt-verdict.md`).** No rig time spent. Three things must land first: **(a)** rev 2's inference is STILL overclaimed — a reproduction on these two Macs could be **macOS/APFS or host×role residue**, not a "platform-general layout cost", and a null licenses only "did not reproduce on this pair", NOT "Windows required" (and the "platform-residue escape" it claims to close **does not exist** — the parent says P1 has no escape hatch); **(b)** the power gate is **broken** — `S = max−min` is a range, not an MDE, and codex's counterexample (`d = [0,180,180,190,190,200,200,200]`, 7/8 positive, effect 83% of the 230 ms reference) returns **"VANISHES, powered=yes"**; it needs a real paired **equivalence** test (distribution-free CI on median(d_i); at n=8 the order stats `[d₍₂₎,d₍₇₎]`); **(c)** `compute_verdicts` implements **none** of the registered rule (no control gate, no clustering, no six outcomes — just PASS/FAIL), so a human would apply it after seeing the numbers. **The harness must COMPUTE the verdict; the prose must only describe what the code does.**
docs/STATE.md:6:- **NEXT ACTION — rev 3 of the Mac↔Mac pre-registration + harness, then run it (Queue 1(ii); the second of the two experiments that come BEFORE any pf code).** Experiment (i), the A-B-B-A MTU run, is **DONE** (pf-0 below). Rig: nagatha `10.1.10.92` ↔ `q` `10.1.10.54`, both 10GbE/9000. Pre-registered (**rev 2**, codex round 1 = 9 findings, **9/9 accepted**): `docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md`; harness `scripts/bench_otp12pf_mac.sh`. **⚠ CORRECTED 2026-07-14 — it is NOT an H1 discriminator.** The earlier claim here ("reproduces ⇒ **H1 DIES**, H1 accuses the *Windows* accept branch") was **WRONG and is retracted**: H1 accuses **blit's own code paths** (`SourceSockets` Dial/Accept, `add_dialed_stream`, the dial-before-ACK at `transfer_session/mod.rs:3113`) — **the word "Windows" appears nowhere in H1**, and that code runs on macOS too. A Mac↔Mac reproduction is therefore *consistent with* H1, not fatal to it. What the rig **does** answer: **does P1 require the macOS↔Windows PAIRING, or is it a platform-general cost of the destination-initiated layout?** Reproduces ⇒ P1 is **not platform residue** (no Windows peer needed) → the "accept it as platform residue" escape closes and every code-level hypothesis strengthens. Vanishes ⇒ P1 is **pairing-dependent** → platform-agnostic code mechanisms weaken. Either way the hypothesis space moves, which is why it precedes pf-1. **Then `pf-1`** (the HARD GATE), which two pf-0 results now BIND: (a) **between-session grading is dead** (a 20% recovery = 46 ms sits under the 78 ms between-session floor) — pf-1 must **measure its own paired within-session floor and register a resolution check** before grading; (b) **the fast arm is BISTABLE** — grade the run distribution, not the median.

exec
/bin/zsh -lc "nl -ba scripts/bench_otp12pf_linux.sh | sed -n '105,145p;225,285p;450,535p'
nl -ba scripts/bench_otp12_win.sh | sed -n '390,455p;470,520p'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
   105	  local h="$1"; shift
   106	  if [[ "$h" == s ]]; then sssh "$@"; else mssh "$@"; fi
   107	}
   108	hblit()  { [[ "$1" == s ]] && echo "$S_BLIT"   || echo "$M_BLIT"; }
   109	hmod()   { [[ "$1" == s ]] && echo "$S_MODULE" || echo "$M_MODULE"; }
   110	hip()    { [[ "$1" == s ]] && echo "$S_IP"     || echo "$M_IP"; }
   111	hdisk()  { [[ "$1" == s ]] && echo "$S_DISK_RE" || echo "$M_DISK_RE"; }
   112	hname()  { [[ "$1" == s ]] && echo skippy      || echo magneto; }
   113	
   114	# ---- fixtures (otp-2 shapes; verified by count+bytes, never trusted) --------
   115	FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
   116	FIX_COUNT_small=10000; FIX_BYTES_small=40960000
   117	FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
   118	
   119	# ---- provenance: embed +sha AND reject +sha.dirty ---------------------------
   120	embeds_clean() {   # $1=host $2=path
   121	  hssh "$1" "grep -qa -- '+$EXPECT_SHA' '$2' && ! grep -qa -- '+$EXPECT_SHA.dirty' '$2'"
   122	}
   123	sha256_of() { hssh "$1" "sha256sum '$2' | cut -d' ' -f1" | nocr; }
   124	
   125	preflight() {
   126	  [[ "$RUNS" -ge 2 ]] || die "RUNS must be >= 2"
   127	  local h p
   128	  for h in s m; do
   129	    for p in "$(hblit "$h")" "$([[ $h == s ]] && echo "$S_DAEMON" || echo "$M_DAEMON")"; do
   130	      hssh "$h" "test -x '$p'" || die "$(hname "$h"): missing/not exec: $p"
   131	      embeds_clean "$h" "$p" || die "$(hname "$h"): $p does not embed a CLEAN +$EXPECT_SHA"
   132	    done
   133	    # cold-cache capability is METHODOLOGY, not a nicety — hard gate.
   134	    hssh "$h" "echo 1 | sudo -n /usr/bin/tee /proc/sys/vm/drop_caches >/dev/null" \
   135	      || die "$(hname "$h") cannot drop caches (need NOPASSWD /usr/bin/tee /proc/sys/vm/drop_caches) — runs would read WARM"
   136	    hssh "$h" "pgrep -x blit-daemon >/dev/null" && die "$(hname "$h"): a blit-daemon is already running — stop it first"
   137	  done
   138	  log "preflight OK  binary=$EXPECT_SHA (harness HEAD=$NEW_SHA)  runs/arm=$RUNS"
   139	}
   140	
   141	write_manifest() {
   142	  local f="$OUT_DIR/staging-manifest.txt" h
   143	  { echo "# harness_head=$NEW_SHA binary_identity=$EXPECT_SHA"
   144	    echo "host,role,sha,sha256,path"
   145	    for h in s m; do
   225	timed_run() {   # $1=initiating host  $2=src(spec)  $3=dst(spec)  $4=dest host  $5=flag
   226	  local ih="$1" src="$2" dst="$3" dh="$4" flag="${5:-}" out bin
   227	  bin="$(hblit "$ih")"
   228	  prep_run "$dh"
   229	  # The transfer window: bracketed on the initiating host, in ONE ssh, so the
   230	  # round trip stays outside it. NO sync here — see the note above.
   231	  out="$(hssh "$ih" "a=\$(awk '{print int(\$1*1000)}' /proc/uptime)
   232	'$bin' copy '$src' '$dst' --yes $flag >/dev/null 2>/tmp/pf-client.err; rc=\$?
   233	b=\$(awk '{print int(\$1*1000)}' /proc/uptime); echo \"R:\$((b-a)),\${rc}:R\"" | nocr \
   234	    | sed -n 's/.*R:\([0-9][0-9]*,[0-9][0-9]*\):R.*/\1/p' | head -1)"
   235	  if [[ "$out" == *,* ]]; then RUN_MS="${out%%,*}"; RUN_EXIT="${out##*,}"; else RUN_MS=0; RUN_EXIT=99; fi
   236	  # Durability: ALWAYS the destination host, identically for both arms.
   237	  RUN_FLUSH="$(flush_dest_ms "$dh")"
   238	  RUN_VALID=yes
   239	  if [[ "$RUN_FLUSH" == NA ]]; then RUN_VALID=no; RUN_FLUSH=0; fi
   240	  RUN_MS=$(( RUN_MS + RUN_FLUSH ))
   241	  [[ "$RUN_EXIT" == 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   242	  cold_ok || RUN_VALID=no
   243	}
   244	
   245	# ---- arms: the ONLY variable is which host's CLI runs -------------------------
   246	CUR_W=""; CUR_FLAG=""
   247	arm_srcinit()  {   # source host pushes to the dest daemon
   248	  local cell="$1" rid="$2" sh="$3" dh="$4"
   249	  timed_run "$sh" "$(hmod "$sh")/src_$CUR_W" "$(hip "$dh"):$PORT:/bench/${SESSION_TAG}_${cell}_${rid}/" "$dh" "$CUR_FLAG"
   250	  hssh "$dh" "rm -rf '$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}'" 2>/dev/null || true
   251	}
   252	arm_destinit() {   # dest host pulls from the source daemon
   253	  local cell="$1" rid="$2" sh="$3" dh="$4"
   254	  timed_run "$dh" "$(hip "$sh"):$PORT:/bench/src_$CUR_W" "$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}" "$dh" "$CUR_FLAG"
   255	  hssh "$dh" "rm -rf '$(hmod "$dh")/${SESSION_TAG}_${cell}_${rid}'" 2>/dev/null || true
   256	}
   257	
   258	CSV="$OUT_DIR/runs.csv"; echo "cell,arm,build,initiator,run,ms,flush_ms,exit,drain,cold,valid" > "$CSV"
   259	META="$OUT_DIR/meta.csv"; echo "cell,pairs_attempted,complete" > "$META"
   260	
   261	run_pair_loop() {   # cell src_host dest_host
   262	  local cell="$1" sh="$2" dh="$3"
   263	  local slot=1 attempts=0 valid=0 max=$(( 2 * RUNS ))
   264	  log "=== $cell (srcinit=$(hname "$sh") vs destinit=$(hname "$dh"), ABBA, $RUNS pairs) ==="
   265	  while (( valid < RUNS && attempts < max )); do
   266	    attempts=$(( attempts + 1 ))
   267	    local order pair=yes rowA="" rowB="" arm rid
   268	    if (( slot % 2 )); then order="A B"; else order="B A"; fi
   269	    for arm in $order; do
   270	      local aname init
   271	      if [[ "$arm" == A ]]; then aname=srcinit; init="$(hname "$sh")"; else aname=destinit; init="$(hname "$dh")"; fi
   272	      rid="${aname}_s${slot}a${attempts}"
   273	      if [[ "$aname" == srcinit ]]; then arm_srcinit "$cell" "$rid" "$sh" "$dh"; else arm_destinit "$cell" "$rid" "$sh" "$dh"; fi
   274	      [[ "$RUN_VALID" == yes ]] || pair=no
   275	      local row="$cell,$aname,$EXPECT_SHA,$init,$slot,$RUN_MS,$RUN_FLUSH,$RUN_EXIT,$RUN_DRAIN,$RUN_COLD"
   276	      [[ "$arm" == A ]] && rowA="$row" || rowB="$row"
   277	      log "  $cell/$aname slot $slot (att $attempts): ${RUN_MS}ms (dest-flush ${RUN_FLUSH}ms, exit $RUN_EXIT, $RUN_DRAIN, $RUN_COLD)"
   278	    done
   279	    echo "$rowA,$pair" >> "$CSV"; echo "$rowB,$pair" >> "$CSV"
   280	    if [[ "$pair" == yes ]]; then valid=$(( valid + 1 )); slot=$(( slot + 1 ))
   281	    else log "  $cell: pair at slot $slot VOIDED — re-running the slot"; fi
   282	  done
   283	  if (( valid < RUNS )); then echo "$cell,$attempts,no" >> "$META"; log "  $cell INCOMPLETE: $valid/$RUNS"
   284	  else echo "$cell,$attempts,yes" >> "$META"; fi
   285	}
   390	    RUN_DRAIN=${RUN_DRAIN// /_}
   391	    echo "$1: $RUN_DRAIN" >> "$OUT_DIR/drain.log"
   392	    [[ "$RUN_DRAIN" == drained* ]] || log "  WARNING: $1 window UNDRAINED ($RUN_DRAIN) — pair voided, will re-run"
   393	    sync
   394	    sudo -n /usr/sbin/purge
   395	    wssh "pwsh -NoProfile -File '$WIN_TEST\\purge-standby.ps1'" >/dev/null
   396	}
   397	
   398	# --- Fixtures (shape-verified; the otp-12a F2 rule) ----------------------
   399	FIX_COUNT_large=1;     FIX_BYTES_large=1073741824
   400	FIX_COUNT_small=10000; FIX_BYTES_small=40960000
   401	FIX_COUNT_mixed=5001;  FIX_BYTES_mixed=547110912
   402	fixture_shape() {
   403	    find "$1" -type f -exec stat -f%z {} + 2>/dev/null \
   404	        | awk '{ s += $1 } END { printf "%d,%d\n", NR, s }'
   405	}
   406	verify_fixture() {
   407	    local w="$1" want_count want_bytes got
   408	    want_count=$(eval echo "\$FIX_COUNT_$w")
   409	    want_bytes=$(eval echo "\$FIX_BYTES_$w")
   410	    got=$(fixture_shape "$MAC_WORK/src_$w")
   411	    [[ "$got" == "$want_count,$want_bytes" ]] \
   412	        || die "fixture src_$w has shape $got, want $want_count,$want_bytes — remove $MAC_WORK/src_$w and re-run"
   413	}
   414	gen_fixtures() {
   415	    if [[ ! -d "$MAC_WORK/src_large" ]]; then
   416	        mkdir -p "$MAC_WORK/src_large"
   417	        dd if=/dev/urandom of="$MAC_WORK/src_large/large_1024M.bin" bs=1m count=1024 2>/dev/null
   418	    fi
   419	    if [[ ! -d "$MAC_WORK/src_small" ]]; then
   420	        mkdir -p "$MAC_WORK/src_small"
   421	        for i in $(seq 1 10000); do
   422	            d="$MAC_WORK/src_small/d$(( i / 1000 ))"; mkdir -p "$d"
   423	            dd if=/dev/urandom of="$d/f${i}.dat" bs=4096 count=1 2>/dev/null
   424	        done
   425	    fi
   426	    if [[ ! -d "$MAC_WORK/src_mixed" ]]; then
   427	        mkdir -p "$MAC_WORK/src_mixed"
   428	        dd if=/dev/urandom of="$MAC_WORK/src_mixed/big.bin" bs=1m count=512 2>/dev/null
   429	        for i in $(seq 1 5000); do
   430	            d="$MAC_WORK/src_mixed/d$(( i / 500 ))"; mkdir -p "$d"
   431	            dd if=/dev/urandom of="$d/f${i}.dat" bs=2048 count=1 2>/dev/null
   432	        done
   433	    fi
   434	    local w
   435	    for w in large small mixed; do verify_fixture "$w"; done
   436	    log "fixtures verified (count + byte sum)"
   437	}
   438	
   439	win_module_count() {   # $1 = subpath under the module; prints file count
   440	    wssh "(Get-ChildItem -Path '$WIN_MODULE\\$1' -Recurse -File -ErrorAction SilentlyContinue | Measure-Object).Count" | tr -cd '0-9'
   441	}
   442	stage_pull_sources() {
   443	    # Shared across arms by design (D5); verified by remote file count;
   444	    # staged with the NEW pair; the same trees serve block 1 pulls and
   445	    # block 2 win_init pushes (one physical source per direction, F6).
   446	    log "staging pull sources on the Windows module (untimed, new pair)"
   447	    win_ensure new
   448	    local w want got
   449	    for w in large small mixed; do
   450	        want=$(eval echo "\$FIX_COUNT_$w")
   451	        got=$(win_module_count "pull_src_$w\\src_$w"); got=${got:-0}
   452	        if [[ "$got" == "$want" ]]; then
   453	            log "  pull_src_$w verified ($got files, kept)"
   454	            continue
   455	        fi
   470	echo "cell,pairs_attempted,complete" > "$META"
   471	
   472	RUN_MS=0; RUN_FLUSH=0; RUN_EXIT=0; RUN_VALID=yes
   473	
   474	# Mac-initiated runs (block 1 both arms; block 2 mac_init arms).
   475	mac_push_run() {   # blit_bin cell rid dest_remote src [flags...]
   476	    local blit="$1" cell="$2" rid="$3" dest="$4" src="$5"; shift 5
   477	    local start end rc=0
   478	    drop_caches "${cell}-$rid"
   479	    start=$(now_ms)
   480	    "$blit" copy "$src" "${dest}push_${SESSION_TAG}_${cell}_${rid}/" --yes "$@" \
   481	        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
   482	    end=$(now_ms)
   483	    if [[ "$dest" == "$WIN_REMOTE" ]]; then
   484	        RUN_FLUSH=$(flush_win_ms)
   485	        wssh "Remove-Item -Recurse -Force '$WIN_MODULE\\push_${SESSION_TAG}_${cell}_${rid}' -ErrorAction SilentlyContinue" || true
   486	    else
   487	        RUN_FLUSH=$(fsync_tree_ms "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}")
   488	        rm -rf "$MAC_MODULE_ROOT/push_${SESSION_TAG}_${cell}_${rid}"
   489	    fi
   490	    RUN_VALID=yes
   491	    [[ "$RUN_FLUSH" == NA ]] && { RUN_VALID=no; RUN_FLUSH=0; }
   492	    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc
   493	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   494	}
   495	mac_pull_run() {   # blit_bin cell rid remote_src [flags...]
   496	    local blit="$1" cell="$2" rid="$3" rsrc="$4"; shift 4
   497	    local start end rc=0
   498	    local dst="$MAC_WORK/dst_pull_${SESSION_TAG}_${cell}_${rid}"
   499	    mkdir -p "$dst"
   500	    drop_caches "${cell}-$rid"
   501	    start=$(now_ms)
   502	    "$blit" copy "$rsrc" "$dst" --yes "$@" \
   503	        > /dev/null 2> "$OUT_DIR/blit-logs/${cell}_${rid}.err" || rc=$?
   504	    end=$(now_ms)
   505	    RUN_FLUSH=$(fsync_tree_ms "$dst")
   506	    rm -rf "$dst"
   507	    RUN_MS=$(( end - start + RUN_FLUSH )); RUN_EXIT=$rc; RUN_VALID=yes
   508	    [[ $rc -eq 0 && "$RUN_DRAIN" == drained* ]] || RUN_VALID=no
   509	}
   510	# Windows-initiated runs (block 2 win_init arms): the transfer window is
   511	# a Stopwatch ON Windows printing "<ms>,<exit>"; CRLF-stripped.
   512	win_client_run() {   # cell rid src dst flags_string; sets T_MS/T_RC
   513	    # Sentinel-framed (codex otp-12b F7): anything but a clean
   514	    # "R:<ms>,<rc>:R" — pwsh noise, a crash, a negative exit — parses
   515	    # to T_RC=99 and voids the run; nothing can masquerade as a time.
   516	    local cell="$1" rid="$2" src="$3" dst="$4" flags="${5:-}"
   517	    local out
   518	    # ${rc} braces are load-bearing: PowerShell parses bare `$rc:R` as
   519	    # a SCOPE-qualified variable (like $env:PATH), so the sentinel
   520	    # never printed and every win-initiated run read rc=99 (found live

exec
/bin/zsh -lc "bash -c 'set -o pipefail; bash -c \"set -o | grep pipefail\"; env | grep "'^SHELLOPTS= || true; zsh -c "setopt | grep -E '"\\\"PIPE_FAIL|ERR_EXIT\\\" || true\" 2>/dev/null || true'
bash -c 'set -euo pipefail; x=\""'$(f(){ false; echo yes; }; f)"; printf "x=%s rc=%s'"\\n\" \""'$x" "$?"'"'
bash -c 'set -euo pipefail; f(){ local out; out=\""'$(false | sed -n x | head -1)"; echo "${out:-NA}"; }; x="$(f)"; printf "x=%s rc=%s'"\\n\" \""'$x" "$?"'"'" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
pipefail       	off
x=yes rc=0
x=NA rc=0

exec
/bin/zsh -lc "python3 - <<'PY'
src = [2000] * 8
ds = [0,180,180,190,190,200,200,200]
dst = [s+d for s,d in zip(src,ds)]
def med(v):
    v=sorted(v); n=len(v)
    return v[n//2] if n%2 else (v[n//2-1]+v[n//2])//2
s,d=med(src),med(dst)
D,S=med(ds),max(ds)-min(ds)
fast=min(s,d)
print({'src_med':s,'dst_med':d,'bar': 'PASS' if 10*max(s,d)<=11*min(s,d) else 'FAIL','D':D,'S':S,'powered':'yes' if S<=230 and 10*(fast+230)>11*fast else 'NO'})
PY
git diff --check e1e351d"'^ e1e351d -- scripts/bench_otp12pf_mac.sh
git show e1e351d:scripts/bench_otp12pf_mac.sh | bash -n' in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
zsh:1: can't create temp file for here document: operation not permitted
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-n7BwNsWx' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-lUxC3twD' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-h7Qt0svd' (errno=Operation not permitted)
git: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
git: error: couldn't create cache file '/tmp/xcrun_db-ZF9OI0Km' (errno=Operation not permitted)

exec
/bin/zsh -lc "python3 -c \"src=[2000]*8; ds=[0,180,180,190,190,200,200,200]; dst=[s+x for s,x in zip(src,ds)]; med=lambda v:(sorted(v)[3]+sorted(v)[4])//2; s=med(src); d=med(dst); D=med(ds); S=max(ds)-min(ds); fast=min(s,d); print({'src_med':s,'dst_med':d,'bar':'PASS' if 10*max(s,d)<=11*min(s,d) else 'FAIL','D':D,'S':S,'powered':'yes' if S<=230 and 10*(fast+230)>11*fast else 'NO'})\"" in /Users/michael/Dev/blit_v2
 succeeded in 0ms:
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-9qeD2U40' (errno=Operation not permitted)
python3: warning: confstr() failed with code 5: couldn't get path of DARWIN_USER_TEMP_DIR; using /tmp instead
python3: error: couldn't create cache file '/tmp/xcrun_db-fuNFk8YA' (errno=Operation not permitted)
{'src_med': 2000, 'dst_med': 2190, 'bar': 'PASS', 'D': 190, 'S': 200, 'powered': 'yes'}

codex
## Findings

- [PREREGISTRATION.md:125](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:125); [bench_otp12pf_mac.sh:403](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:403) — **BLOCKER** — The harness never computes the registered session verdict: no control-cell `RIG-VOID`, six-outcome decision, power-gate enforcement, clustering, or `UNSTABLE`. It emits per-cell `PASS/FAIL`; the prose tree is itself overlapping and incomplete.

- [PREREGISTRATION.md:94](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:94); [bench_otp12pf_mac.sh:453](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:453) — **BLOCKER** — `S=max(d)-min(d)` is not an MDE/equivalence bound. For `srcinit=2000×8`, `d=[0,180,180,190,190,200,200,200]`, the code reports bar `PASS`, `D=190`, `S=200`, `powered=yes`; repeated in both directions, the rule declares `VANISHES` despite 7/8 positive pairs and 83% of the reference effect.

- [PREREGISTRATION.md:29](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:29); [bench_otp12pf_mac.sh:14](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:14) — **BLOCKER** — The registered inference overreaches. Reproduction proves only that P1 can occur without Windows on this pair—not a platform-general layout cost; a null proves only non-reproduction on this pair—not that Windows is required.

- [bench_otp12pf_mac.sh:302](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:302) — **HIGH** — The push and pull landed paths are currently correct, but `os.walk()` on a missing, unreadable, or empty path emits a valid `F:0:F`. No destination count/byte sum is checked, so an exit-0 zero/partial-byte transfer becomes a valid fast row. Source fixtures are also verified only by count.

- [bench_otp12pf_mac.sh:323](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:323) — **HIGH** — The one-process transfer timer is sound, but transfer and fsync are disjoint intervals. Writeback during return/dispatch is free; a q-initiated run additionally returns through SSH before fsync starts, so the favored arm reverses by direction. [prep_run:280](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:280) also certifies drain before `sync; purge`, then never rechecks it.

- [bench_otp12pf_mac.sh:156](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:156) — **HIGH** — Environmental gates fail open: `pgrep` errors become “quiet”; `tmutil` errors/empty parses become zero; AutoBackup read errors explicitly become disabled; `top` failures become zero and the last idle `mds` row can overwrite a busy one; malformed/empty load becomes `0`. Local commands use Bash while remote commands use q’s configured shell, so inner-shell behavior is not pinned.

- [bench_otp12pf_mac.sh:218](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:218) — **HIGH** — The ARP gate ignores ping failure, accepts any complete nagatha→q MAC without comparing it to q’s known MAC, and never checks q→nagatha. The documented own-MAC black hole and wrong-NIC/1GbE route can pass.

- [bench_otp12pf_mac.sh:111](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:111) — **HIGH** — The registered protocol is unenforced: `RUNS>=2` is accepted, arbitrary or misspelled `CELLS` can omit every control—or measure zero cells—and blank `CELLS` runs 12 rather than six. Overridable drain thresholds are not recorded.

- [bench_otp12pf_mac.sh:92](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:92) — **HIGH** — Instrument provenance records only `HEAD`, so a modified harness still claims the reviewed commit. [sha256_of:151](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:151) accepts empty/malformed hashes, while `! grep` can also convert a dirty-marker read error into “clean.”

- [bench_otp12pf_mac.sh:243](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:243) — **MEDIUM** — `nc -z` proves only that a TCP handshake reached some listener’s backlog, not that the captured daemon PID accepts or speaks blit. During teardown, SSH/`ps` failure is logged as “verified gone,” and cleanup discards a positively detected survivor.

- [bench_otp12pf_mac.sh:423](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:423) — **LOW** — Even-sample medians are floored before the supposedly exact bar and `D>S` comparisons; the n=8 IQR is an unspecified `x₆−x₃` convention. The ordinary ABBA void retry, slot pairing, and `destinit-srcinit` sign are otherwise correct.

**VERDICT: NOT READY — the instrument can accept zero-byte/unflushed rows and cannot deterministically apply its preregistered rule.**
tokens used
116,175
## Findings

- [PREREGISTRATION.md:125](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:125); [bench_otp12pf_mac.sh:403](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:403) — **BLOCKER** — The harness never computes the registered session verdict: no control-cell `RIG-VOID`, six-outcome decision, power-gate enforcement, clustering, or `UNSTABLE`. It emits per-cell `PASS/FAIL`; the prose tree is itself overlapping and incomplete.

- [PREREGISTRATION.md:94](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:94); [bench_otp12pf_mac.sh:453](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:453) — **BLOCKER** — `S=max(d)-min(d)` is not an MDE/equivalence bound. For `srcinit=2000×8`, `d=[0,180,180,190,190,200,200,200]`, the code reports bar `PASS`, `D=190`, `S=200`, `powered=yes`; repeated in both directions, the rule declares `VANISHES` despite 7/8 positive pairs and 83% of the reference effect.

- [PREREGISTRATION.md:29](/Users/michael/Dev/blit_v2/docs/bench/otp12-macmac-2026-07-14/PREREGISTRATION.md:29); [bench_otp12pf_mac.sh:14](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:14) — **BLOCKER** — The registered inference overreaches. Reproduction proves only that P1 can occur without Windows on this pair—not a platform-general layout cost; a null proves only non-reproduction on this pair—not that Windows is required.

- [bench_otp12pf_mac.sh:302](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:302) — **HIGH** — The push and pull landed paths are currently correct, but `os.walk()` on a missing, unreadable, or empty path emits a valid `F:0:F`. No destination count/byte sum is checked, so an exit-0 zero/partial-byte transfer becomes a valid fast row. Source fixtures are also verified only by count.

- [bench_otp12pf_mac.sh:323](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:323) — **HIGH** — The one-process transfer timer is sound, but transfer and fsync are disjoint intervals. Writeback during return/dispatch is free; a q-initiated run additionally returns through SSH before fsync starts, so the favored arm reverses by direction. [prep_run:280](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:280) also certifies drain before `sync; purge`, then never rechecks it.

- [bench_otp12pf_mac.sh:156](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:156) — **HIGH** — Environmental gates fail open: `pgrep` errors become “quiet”; `tmutil` errors/empty parses become zero; AutoBackup read errors explicitly become disabled; `top` failures become zero and the last idle `mds` row can overwrite a busy one; malformed/empty load becomes `0`. Local commands use Bash while remote commands use q’s configured shell, so inner-shell behavior is not pinned.

- [bench_otp12pf_mac.sh:218](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:218) — **HIGH** — The ARP gate ignores ping failure, accepts any complete nagatha→q MAC without comparing it to q’s known MAC, and never checks q→nagatha. The documented own-MAC black hole and wrong-NIC/1GbE route can pass.

- [bench_otp12pf_mac.sh:111](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:111) — **HIGH** — The registered protocol is unenforced: `RUNS>=2` is accepted, arbitrary or misspelled `CELLS` can omit every control—or measure zero cells—and blank `CELLS` runs 12 rather than six. Overridable drain thresholds are not recorded.

- [bench_otp12pf_mac.sh:92](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:92) — **HIGH** — Instrument provenance records only `HEAD`, so a modified harness still claims the reviewed commit. [sha256_of:151](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:151) accepts empty/malformed hashes, while `! grep` can also convert a dirty-marker read error into “clean.”

- [bench_otp12pf_mac.sh:243](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:243) — **MEDIUM** — `nc -z` proves only that a TCP handshake reached some listener’s backlog, not that the captured daemon PID accepts or speaks blit. During teardown, SSH/`ps` failure is logged as “verified gone,” and cleanup discards a positively detected survivor.

- [bench_otp12pf_mac.sh:423](/Users/michael/Dev/blit_v2/scripts/bench_otp12pf_mac.sh:423) — **LOW** — Even-sample medians are floored before the supposedly exact bar and `D>S` comparisons; the n=8 IQR is an unspecified `x₆−x₃` convention. The ordinary ABBA void retry, slot pairing, and `destinit-srcinit` sign are otherwise correct.

**VERDICT: NOT READY — the instrument can accept zero-byte/unflushed rows and cannot deterministically apply its preregistered rule.**
