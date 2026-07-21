# ldt-4 rig-W live-controller evidence — 2026-07-21

**Status:** Structurally valid and independently recomputed, but
`REVIEW_REQUIRED`; this is not ldt-4 acceptance evidence.

## Identity

- Session: `ldt4-20260721T224319Z-96a4e3b03caf`
- Artifact build: `406a7e5854593b7a7a151f9b6d9cdf1be8a9cd77`
- Harness/analyzer: `96a4e3b03caf43ee368efadc779e3324248067f6`
- Matrix: 96 valid arms, 48 initiator-layout pairs, six direction/fixture
  cells
- Completion: `MEASUREMENTS-COMPLETE` is present, `SESSION-VOID` is absent,
  and the prior Windows daemon was restored normally and byte-for-byte

## Retained evidence

The copied session payload contains 1,226 files and 51,275,299 bytes before
this README. Its sorted `relative-path<TAB>byte-size<TAB>file-sha256<LF>`
inventory digest is:

`713cb4624e6f64a3863b67101fb9a3f3df288306d3e6f418c19501428711990b`

Key file SHA-256 values:

- `MEASUREMENTS-COMPLETE`:
  `a4437fce1b214c1296f98a9037d3245b2027bf426abfecdbac8731413568d643`
- `runs.csv`:
  `d3778b23bdbcf9bf5edb0c84c950aa0468af366dba5b6482d8dc01b86128fa8e`
- `runtime-gates.csv`:
  `a1115dedf84da65e9feab6fe4eab32b8721a1d20b22b6a7620dda7a479a9fedc`
- `FINAL-SHA256.csv`:
  `f7e43da69919e8c0204aa9943df41d4e7c23bb52cb6526ff6653bc42b826bbe5`
- `analysis/summary.json`:
  `8ebc9fe3bdade9c0b9b5f7ba9184ccffa41bb2dbc736d14c863ce0d4b0aaac58`
- `analysis/summary.md`:
  `01eda8ca74d9dd89877efc1ac174e28538ce6a2bacbd1f65a957fa83f72924e2`
- `analysis/arms.csv`:
  `66362095f80701befffe75829712beab92805947631b85f96d561c3c87fa07d3`
- `analysis/pairs.csv`:
  `ae97b19617c3cb3f77e3e67003eaf18f14e9d3fe685f037a912511fcb832811c`

The analyzer was also rerun from the exact pre-analysis inputs in a separate
tree. Its six generated analysis files matched the retained files byte for
byte and reproduced:

`ldt-4 analysis REVIEW_REQUIRED: 96 arms; arm_review=0, decision_review=14, performance_review=2`

## Controller result

Every arm remained at floor = peak = final = 4, with zero accepted ADDs,
zero accepted REMOVEs, and an observed receiver safety ceiling of 32. Of the
96 arms, 74 completed before producing any tuner sample and the remaining 22
produced exactly one sample. The workload therefore did not exercise an
adaptive membership transition.

Fourteen paired records require decision review. All eight
`windows_to_q_small` source-initiator arms emitted one `cheap-up` sample while
their destination-initiator mates emitted none. Six of eight
`q_to_windows_small` pairs differed among no sample, `cheap-up`, cooldown, and
hysteresis outcomes. This is a real role-timing asymmetry in these short
fixtures, but not proof that the two layouts would choose different adaptive
membership under sustained load.

## Performance result

| Cell | Source-init median (ms) | Destination-init median (ms) | Ratio | Verdict |
|---|---:|---:|---:|---|
| `q_to_windows_large` | 878 | 1051 | 1.1970 | `REVIEW_REQUIRED` |
| `windows_to_q_large` | 895 | 879 | 1.0182 | pass |
| `windows_to_q_small` | 648.5 | 641.5 | 1.0109 | pass |
| `q_to_windows_small` | 945.5 | 1015 | 1.0735 | pass |
| `q_to_windows_mixed` | 715 | 809 | 1.1315 | `REVIEW_REQUIRED` |
| `windows_to_q_mixed` | 578.5 | 559 | 1.0349 | pass |

Because no arm changed membership, the two performance-review cells cannot be
attributed to divergent adaptive decisions in this session.

## What remains

1. Own the insufficient controller-exercise horizon and short-fixture
   decision asymmetry as one finding. Add a dedicated sustained workload that
   can prove ADD/REMOVE behavior without replacing the existing performance
   cells or blindly enlarging the whole matrix.
2. Own the two performance-review cells separately. Do not mask their fixed
   overhead by grading only a much longer payload ratio.
3. Review the final ldt-4 evidence record after both findings are resolved.
