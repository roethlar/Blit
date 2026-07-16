# live-dial-tuning-plan round 1 — Claude adjudication

- Reviewer: `claude-fable-5` via Claude Code `2.1.211`, effort `max`
- Reviewed range: `35d7d1307d7a2a455756b372d3bf637f2a5a382c..554d080839e1419c2242921e444d40d02c947815`
- Retained worktree: `/tmp/blit-review-live-dial-554d080-r1`
- Raw result: `.review/results/live-dial-tuning-plan-r1.claude.json`
- Non-authoritative proxy attempt: `.review/results/live-dial-tuning-plan-r1.claude-attempt1-error.json`
- Independent verdict: `REOPENED`
- Guard confirmed: `true`
- Recorded: `2026-07-16T06:09:18Z`

The orchestrator accepted the envelope as authentic and schema-valid: exit zero,
exact dispatched base/head SHAs, literal `guard_confirmed=true`, and a clean
retained worktree at the reviewed head. The earlier proxy-routed invocation
returned no verdict and is excluded.

## Adjudication

1. **ADMITTED — MEDIUM: accepted resize versus need-completion is
   underspecified and internally contradictory.** The plan requires completion
   during a pending resize to work, but its current fault rule can turn an
   accepted late ADD/REMOVE into either a healthy-session fault or false
   membership settlement. The correction will specify one terminal rule for
   both layouts: once payload shutdown is draining/complete, normal END
   retirement satisfies an already accepted membership transition; only a live
   pipeline that refuses or errors on that accepted transition faults.
2. **ADMITTED — LOW: the growth guard does not prove the old maximum of 16 is
   absent.** A test that grows only from 8 to 9 could pass with a surviving
   clamp at 16. The deterministic guard will be strengthened to cross 16, or
   equivalently reach a lower advertised receiver ceiling.
3. **ACCEPTED OBSERVATION — no design change required:** the frozen initial
   blocked-ratio policy may grow toward the receiver ceiling on a source-bound
   transfer because idle workers do not create the shrink signal. That behavior
   is bounded and correctness-neutral. The existing default-off observer and
   later Mac-to-Mac evidence must expose it rather than silently grading it as a
   controller defect.

Claude otherwise verified the intended architecture: the workload shape table
is removed as worker-count authority, one SOURCE-owned controller is reused in
both connection layouts, epoch zero is bounded by the receiver-advertised
ceiling, tokenless REMOVE fits the existing wire and retirement substrate, the
slices avoid a dual-policy window, and the local-apply/in-stream exclusions are
resource boundaries rather than push/pull disparities.

## Independent guard

Claude ran the docs gate and diff check, then 33 semantic assertions over the
reviewed plan, parent plan, session contract, and STATE. Restoring only the two
corrected contract documents from the base SHA made 17 assertions fail on the
predicted shape-only contradiction. Restoring their exact reviewed blobs made
all assertions and the docs gate pass again. The retained worktree ended clean
at `554d080839e1419c2242921e444d40d02c947815`.

No endpoint, SSH, rig, push, branch, deletion, Time Machine setting, or mount
was touched.
