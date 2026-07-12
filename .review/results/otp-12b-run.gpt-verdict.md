# otp-12b recorded-run round — adjudication

**Reviewed range**: `f19776c..44c2046` (sentinel fix `e21cf84`, CR fix
`856af64`, evidence `44c2046`). **Raw review**:
`.review/results/otp-12b-run.codex.md` (gpt-5.6-sol, 130,937 tokens).
**Verdict**: FAIL — 3 findings (1 High, 2 Low), with everything else
explicitly confirmed: "arithmetic, CR-only sanitization, escalation,
cross-block disclosure, sentinel fix, and code-shaped wording otherwise
check out." All three ACCEPTED.
reviewer: gpt-5.6-sol

## F1 (High) — README self-adjudicated the D-2026-07-12-1 attribution

Confirmed: the cross-direction prose declared the platform attribution
(and criterion satisfaction) across all Mac→Win cells, but the
decision's discriminator shape — the old arm shows the SAME gap — holds
only for the large cells (1.979→1.951, 1.956→1.945); tcp_mixed
(1.946→1.408) and grpc_small (1.929→1.644) NARROWED, and how much of
their residual gap is platform belongs to the owner's otp-13 walk.
Fixed: the section now records the three gap shapes
(unchanged/narrowed/widened) per cell and draws no criterion
conclusion.

## F2 (Low) — cross-row range misquoted

Confirmed: the six Win→Mac cross rows span 0.760–0.990; 0.710 was a
per-arm converge ratio. Fixed.

## F3 (Low) — comment said 196 valid runs; the session has 192

Confirmed. Fixed.

## Fix commit

fix sha: `49dee5c` (`bash -n` clean; check-docs green; docs + one
comment line — suite stands at the recorded 1484).
