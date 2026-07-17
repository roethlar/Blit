# ldt-4 harness round 1 — Claude adjudication

- Reviewer: `claude-fable-5` via Claude Code `2.1.212`, effort `max`
- Reviewed range: `e41b87173f2073a9b6694a62813eddc14a7844ad..0e4872162f09120188404d5d23448ff3a6298133`
- Retained worktree: `/tmp/blit-openreview-ldt4-harness-0e48721-r1`
- Neutral prompt: `/tmp/ldt4-harness-fable-r1-neutral-prompt.md`
- Prompt SHA-256: `e1b5fc213929acae686ed466cc3e7ebda18dce844a3de489b81b4a41c8031cd3`
- Result schema: `/tmp/ldt4-harness-fable-r1-schema.json`
- Schema SHA-256: `02d943b7f907aa2b568b38a2d0633726aa96eaf64914f7d8cda3390a3a3091ab`
- Raw result: `.review/results/ldt-4-harness-r1.claude.json`
- Independent verdict: `FINDINGS` (seven candidates)
- Guard confirmed: `true`
- Claude-reported cost: `$24.275937`
- Recorded: `2026-07-17T01:58:26Z`

The authoritative call exited zero and returned a schema-valid payload with
the exact dispatched base/head SHAs, literal `guard_confirmed=true`, seven
fully populated candidates, no web use, and no permission denial. Two earlier
launches were rejected before a reviewer turn or model token because the
caller supplied unsupported JSON-schema forms; they are configuration errors,
not review attempts or verdicts. Claude's usage record attributes the formal
call to the requested Fable driver and an internal Opus delegation. The raw
envelope is retained verbatim.

Direct post-review checks confirm the retained detached worktree is clean at
exact `0e48721`. The three reviewed scripts are byte-unchanged between that
snapshot and the primary tree immediately before intake; later governance and
review-record commits did not alter them. The retained worktree, prompt, and
schema have not been deleted.

## Intake and adjudication

`ldt-4-r1-f1` — **ADMITTED (MEDIUM): strict Windows fetch framing is fragile.**
`fetch_windows_file` feeds all SSH stdout to strict base64 decoding. The
registered PowerShell write adds no newline by itself, so the review's
trailing-newline wording is too broad, but any profile/banner stdout makes the
fetch fail before its SHA check. That predictably voids every Windows evidence
fetch on a noisy channel. Use one exact tagged payload line, require exactly
one tag, decode it strictly, and retain the remote/local SHA comparison.

`ldt-4-r1-f2` — **ADMITTED (MEDIUM): a hard crash can reclassify the staged
daemon as the original baseline.** Swap intent and prior retention are scoped
to the current session tag. A crash that bypasses the exit trap leaves the
tested daemon active; a new tag then preserves that tested daemon as its
"prior" and strands the true original under the old retention name. Refuse a
new swap before intent creation when the active hash already equals the staged
hash or any prior-retention entry remains. No retained file may be removed.

`ldt-4-r1-f3` — **ADMITTED (MEDIUM): padded low PIDs bypass exact client
recovery.** Global `IFS` excludes spaces, while `ps -axo pid=` right-justifies
small PIDs. The fallback loop therefore rejects the common padded values with
its numeric regex and can miss a surviving owned client after PID-file loss.
Normalize `ps` output before the exact command/environment identity checks.

`ldt-4-r1-f4` — **DECLINED at intake.** The candidate conflates measurement
completion with acceptance. `MEASUREMENTS-COMPLETE` is deliberately written
before analysis and binds the exact 96-arm raw input. `REVIEW_REQUIRED` remains
machine-readable in `analysis/summary.json`, the arm/pair CSVs,
`analyzer-result.txt`, and analyzer stdout. No current consumer treats the
completion marker or exit zero as an acceptance verdict, so the predicted
silent pass depends on hypothetical tooling that ignores the registered status
interface.

`ldt-4-r1-f5` — **DECLINED as an analyzer defect.** The specific production
contract requires an accepted resize to reach its target membership or fault
the session. Source handling propagates post-ACK socket/membership failure,
checks the resulting logical count against the target, and only then records an
accepted settlement. A successful trace with `accepted=true` and a different
effective count is impossible evidence and the analyzer is right to refuse it.

`ldt-4-r1-f5a` — **ADMITTED separately (LOW): stale generic resize
documentation describes that impossible recovery.** `dial.rs` still says a
post-ACK local dial failure may settle at the unchanged count, contradicting
the active plan, transfer-session contract, production caller, and tests. The
comment can mislead future analyzer/runtime work and must state the actual
accept-or-fault rule. This does not weaken analyzer validation.

`ldt-4-r1-f6` — **ADMITTED, narrowed (LOW): the independent Python policy
replay lacks cooldown/sustain/bound branch guards.** Rust production tests
already cover all ten reasons, and a real-emitter fixture is not a prerequisite
for the first hardware run. The Python transcription itself has no positive
test for `cooldown`, `sustain`, or `bound`, so those branches can drift without
turning its 72-test suite red. Add a focused table-driven replay guard.

The bundled seventh candidate is split to preserve one-finding-per-commit:

- `ldt-4-r1-f7a` — **ADMITTED (LOW): duplicate JSON keys collapse
  last-wins.** A conflicting duplicate trace field reaches the analyzer as one
  ordinary key and can pass the exact-field checks. Reject duplicates in the
  central trace JSON loader.
- `ldt-4-r1-f7b` — **ADMITTED (LOW): the blocked-ratio tolerance can cross a
  policy threshold.** One floating-point step below `0.05` or above `0.30`
  changes replay policy while remaining far inside the current `1e-12`
  tolerance. The producer and analyzer derive the value from the same integer
  counters and JSON round-trips the `f64`, so require exact recomputation.

Seven bounded fixes are admitted: three Medium harness-safety defects and four
Low contract/test/evidence-strictness defects. They land one finding per commit,
with red/green or manual guard proof, before a fresh neutral whole-change Fable
review. The hardware run remains blocked on that clean final review.

## Independent guard

Claude reported a complete green → mutation red → exact restoration green
proof and returned `guard_confirmed=true`. Direct checks after exit confirm its
detached worktree is clean at the reviewed SHA. Intake did not rely on the
reviewer's conclusions alone: each admitted or declined candidate above was
checked against the current script, analyzer, runtime, tests, and registered
contract.

No endpoint artifact, staging path, process, daemon, transfer, evidence tree,
Time Machine setting, mount, remote ref, or retained review artifact changed.
No deletion occurred.
