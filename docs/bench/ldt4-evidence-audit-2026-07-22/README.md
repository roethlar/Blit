# ldt-4 retained-evidence audit — 2026-07-22

**Status:** Final classification of every retained ldt-4 live session recorded
in the repository through `43f156d`.

## Classification rule

`SESSION-VOID` records what the harness decided at the time. It does not erase
or invalidate complete, immutable raw evidence. A session is classified from
what was actually recorded:

- complete and unambiguous evidence may be reinterpreted by a corrected
  analyzer without rerunning the transfer;
- completed rows from an interrupted matrix remain factual evidence for those
  arms, but cannot grade the missing matrix as though it completed; and
- a preflight or launch failure with no transfer row contains no transfer
  measurement to salvage.

The original markers and payloads remain unchanged. This audit supersedes only
claims that a harness/analyzer rejection automatically makes every recorded
artifact invalid.

## Session ledger

| Session | What happened | Evidence classification |
|---|---|---|
| `ldt4-20260717T032327Z-4e0fdc307ba2` | Fixture preflight found unequal 1 GiB sources; zero arms | No transfer evidence; correct preflight refusal |
| `ldt4-20260717T052509Z-5a2265e202a4` | Windows launch generation failed; zero timing rows | No transfer evidence |
| `ldt4-20260717T062334Z-a39f0c570191` | Generated `start.cmd` split dynamic commands; zero timing rows | No transfer evidence |
| `ldt4-20260721T202216Z-322a1611230e` | Resolver-derived q hostname failed the machine gate; zero rows | No transfer evidence |
| `ldt4-20260721T204038Z-21fe468af129` | Windows console host was miscounted as a second daemon child; zero rows | No transfer evidence |
| `ldt4-20260721T210445Z-55fc5d5ff456` | Arm 1 completed a byte-identical 1 GiB transfer; arm 2 never launched | One valid arm; incomplete matrix |
| `ldt4-20260721T212142Z-c2e12846bcb1` | 25 rows were appended; arm 26 also completed before teardown mishandled an already-exited launcher | Valid completed-arm evidence; incomplete matrix |
| `ldt4-20260721T214439Z-ef9ef0b6f531` | 38 byte-identical arms completed before stale one-minute load history stopped the run | Valid completed-arm evidence; incomplete matrix |
| `ldt4-20260721T221543Z-c621e33fd9df` | 21 byte-identical arms completed; arm 22 never started after an ambiguous SSH directory reservation | Valid completed-arm evidence; incomplete matrix |
| `ldt4-20260721T224319Z-96a4e3b03caf` | All 96 arms completed and independently reproduced | Complete valid fixed-matrix evidence; controller was not exercised |
| `ldt4-20260722T001611Z-04e80082e12c` | All four 5 GiB arms completed and independently reproduced | Complete valid evidence; fixture admitted before the first tuner tick |
| `ldt4-20260722T013314Z-a0c3e3f18afd` | All four 40 GiB arms completed; analyzer expected `ADD`/`REMOVE` where production emitted exact protobuf enum names | Complete valid evidence after corrected-analyzer reanalysis; original `SESSION-VOID` is an analyzer outcome, not an evidence verdict |
| `ldt4-20260722T022350Z-7050a2997ac5` | Fresh four-arm 40 GiB repeat completed | Complete valid but redundant confirmation of the preceding session |

The endpoint-address preflight at `d53b5fd` failed before evidence reservation,
session creation, fixture staging, daemon swap, or transfer, so it has no
session tag or transfer evidence.

## Final live result

The corrected reanalysis of `ldt4-20260722T013314Z-a0c3e3f18afd` returned the
same material result later confirmed by `7050a29`: q→Windows matched REMOVE
`4→3→2→1`, while Windows→q split between ADD and REMOVE. The first session's
four durations were 47.061, 48.077, 45.275, and 34.720 seconds. The confirmation
was within 0.7 seconds on every arm and within 7/10 milliseconds on the two
Windows-source arms.

That result proves the live controller resized real transfers. It does not
prove that socket role caused the Windows→q split because both sessions used
the same cold/warm order. The ADD arm was slower than the REMOVE arm. No
controller policy change is authorized, and the causal follow-up is deferred
until after release.

## Write cost

The four complete session records prove approximately 389.5 GiB of destination
payload writes across the two endpoints: 49.5 GiB in the fixed matrix, 20 GiB
in the sustained supplement, and 160 GiB in each horizon session. Approximately
194.8 GiB landed on Mac-side destination paths and the same amount on Windows.
This excludes source staging and interrupted-session writes.

No further data-moving performance run is authorized before release. Any later
large SSD-write test requires explicit owner approval.

## Verification

At `43f156d`, the corrected analyzer requires production SOURCE actions
`DATA_PLANE_RESIZE_OP_ADD` and `DATA_PLANE_RESIZE_OP_REMOVE`. On 2026-07-22:

- `python3 scripts/ldt4_rigw_analyze_test.py` passed all 98 tests; and
- `SELFTEST=1 bash scripts/bench_ldt4_rigw.sh` passed four registered arms with
  no SSH.

No endpoint was contacted during this audit.
