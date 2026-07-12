# otp-11b — codex verdict adjudication

**Reviewed**: commit `805e48c` (the local orchestration deletion;
docs rider `b1650c4` landed during the review). Raw review:
`.review/results/otp-11b.codex.md` (gpt-5.6-sol, 203k tokens,
VERDICT: CHANGES REQUESTED). Codex independently CONFIRMED the core:
"deletion, re-homes, converted coverage, remote-session behavior,
one-transfer-path structure, and the 1484-pass suite check out" — the
dial relocation verified blob-identical. Six findings, all
docs/presentation/record; 6/6 accepted and fixed.
reviewer: gpt-5.5-class (gpt-5.6-sol)

## B1 (Medium) — docs/STATE.md stale at the reviewed commit

**Accepted — resolved by the records commit** (the loop's cadence
lands STATE with the review round's records; at the pinned commit the
summary line and the "old orchestration in-tree" clause were indeed
stale). STATE now records 11b closed, suite 1484.

## B2 (Medium) — live-doc sweep incomplete

**Accepted — fixed.** `docs/WHITEPAPER.md`: the local→local
combination row now maps to `run_local_session` (the old orchestrator
row deleted); §5 (adaptive tuning) marked HISTORICAL with the otp-11b
retirement note; the bug-surface list's auto_tune and change_journal
items struck with dated notes; the no-op benchmark claim had already
been re-annotated at the addendum. `docs/ARCHITECTURE.md`: the
system-overview diagram's Orchestrator/ChangeJournal/AutoTune cells
replaced (Session/Dial/Copy/PerfHistory); module-table rows were
already swept in `b1650c4`. `.agents/repo-guidance.md`: the Style
exemplar (`TransferOrchestrator` → `TransferSession`) and the Project
Map's "transfer engine, orchestrator" line updated.
`diff_planner.rs`: the module doc rewritten — it no longer describes
the deleted local-mirror consolidation as current.

## B3 (Medium) — frontend text promises predictor training that no longer exists

**Accepted — fixed.** TUI F4 empty-state now says "populate the
history"; `blit profile`'s no-profile line states the training
retirement (persisted profiles still display); the `--null` clap doc
now describes the `null_sink` history lane instead of "the adaptive
predictor".

## B4 (Low) — "Workers used" prints the options default, not the effective count

**Accepted — fixed.** The throughput line prints the EFFECTIVE apply
worker count (1 unless the hidden debug limiter widened it); the
debug-cap line units corrected to worker(s).

## B5 (Low) — accounting derivation double-counted

**Accepted — fixed.** The finding doc now carries the exact equation
(1513 − 41 − 10 − 5 + 27 = 1484), states that the 25 in-place
conversions do not appear in the delta, and classifies
`planner_keeps_every_header` + the two manifest arm pins as NEW pins.

## B6 (Low) — dial relocation carries 17 tests, not 15

**Accepted — fixed** in the finding doc (accounting-only; codex
verified the relocation blob-identical).

## Disposition

6/6 accepted and fixed. Fix sha: `9e810ee` (gate green: fmt, clippy
-D warnings, cargo test --workspace 1484/0 (2 ignored), the
pre-existing blit_utils flake run in isolation 22/22).
