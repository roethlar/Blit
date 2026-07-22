# ldt-4 rig-W admission-horizon evidence — 2026-07-22

**Status:** Structurally valid and independently recomputed, but
`REVIEW_REQUIRED`; this is not ldt-4 acceptance evidence.

## Identity

- Session: `ldt4-20260722T022350Z-7050a2997ac5`
- Parent fixed-matrix session: `ldt4-20260721T224319Z-96a4e3b03caf`
- Parent copied-evidence inventory:
  `713cb4624e6f64a3863b67101fb9a3f3df288306d3e6f418c19501428711990b`
- Predecessor sustained session: `ldt4-20260722T001611Z-04e80082e12c`
- Predecessor copied-evidence inventory:
  `17348aaa261b936e04c104553d7b5c4bbcf008968306a29c4dea922535110eef`
- Artifact build: `406a7e5854593b7a7a151f9b6d9cdf1be8a9cd77`
- Harness/analyzer: `7050a2997ac597a1b8982e7f4acbfa0b12572340`
- Matrix: four valid 40-file/40-GiB arms, two initiator-layout pairs, two
  physical byte directions
- Completion: `MEASUREMENTS-COMPLETE` is present, `SESSION-VOID` is absent,
  Windows restored normally and byte-for-byte, and q/Windows port 9031 is
  closed

## Retained evidence

The copied session payload contains 76 files and 300,999 bytes before this
README. `FINAL-SHA256.csv` records and independently verifies every other
copied file (75 entries); its SHA-256 is:

`c6ed0cf96b9d888d0611d9264e6be4bd3e67433afbd604e74b2ca07cf89a031a`

The original evidence remains retained at
`/Users/michael/blit-ldt4-evidence/ldt4-20260722T022350Z-7050a2997ac5` on q.
The two stable 40-GiB sources and all four unique landed payloads remain
retained on their respective endpoints.

Key file SHA-256 values:

- `MEASUREMENTS-COMPLETE`:
  `dbedb57dd1b47e5a7c7750c2875abfb1612d530194a773abe98a68996bbab127`
- `runs.csv`:
  `ae6b62f3898e6426968fd46f20ad82541761680c23b282166bb284a8e089eb2f`
- `runtime-gates.csv`:
  `3deef47a18f3f38ea4d8cfaf2f8a3349142fcbf6c97c37ea082d2916f13e27b2`
- `analysis/summary.json`:
  `cc6eaec14903e87007eab354d335e20e4c81c78a7a7f783719be51ab78bf8411`
- `analysis/summary.md`:
  `42a56fb616d15bcb0f60aec3ff0aa2e1092d0908e83c19d731e075e5cd6c8d42`
- `analysis/arms.csv`:
  `4787682d650b16ed29946580554fc325d4efed7650c9ac50fb6c8fe2177eb7a7`
- `analysis/pairs.csv`:
  `846ed8993b3f1aff9a42be95b0170078dc5df6678329d1b12f190c3b54aef8bf`

## Result

The exact analyzer returned:

`REVIEW_REQUIRED: 4 arms; arm_review=3, decision_review=1, performance_review=0`

| Direction / initiator | Duration | Throughput | Samples | Accepted operations | Verdict |
|---|---:|---:|---:|---|---|
| q→Windows / source | 47,661 ms | 859.4 MiB/s | 43 | REMOVE `4→3→2→1` | arm review |
| q→Windows / destination | 47,740 ms | 858.0 MiB/s | 42 | REMOVE `4→3→2→1` | arm review |
| Windows→q / destination | 45,282 ms | 904.6 MiB/s | 31 | ADD `4→5→6→7→8→9→10` | ADD accepted |
| Windows→q / source | 34,710 ms | 1,180.1 MiB/s | 31 | REMOVE `4→3→2→1` | arm review |

The q→Windows role pair is transition-identical and differs by only 79 ms
(ratio 1.0017). The Windows→q role pair is materially different: final stream
counts 10 versus 1, opposite operation sequences, and a 10,572 ms duration gap
(ratio 1.3046). The arm that ADDed six streams was slower; the arm that removed
to one stream was faster.

The divergence is visible before any dial setting changes. At the first
500-ms sample, the Windows SOURCE accepted-socket layout transferred
459,276,288 bytes and reported 117,842,500 blocked nanoseconds across four
streams (`blocked_ratio=0.0588`). The connected-socket layout transferred more
bytes, 618,659,840, but reported 1,939,185,500 blocked nanoseconds
(`blocked_ratio=0.9688`). The policy therefore treated the lower-throughput
sample as capacity to grow and the higher-throughput sample as pressure to
shrink.

This repeats the retained diagnostic void session
`ldt4-20260722T013314Z-a0c3e3f18afd`: its Windows→q pair also split ADD 4→9
versus REMOVE 4→1 at 45,275 versus 34,720 ms. The fresh valid run differs by
only 7 and 10 ms on those two arms, so the outcome is not a one-run timing
fluke.

## Independent recomputation

The copied final inventory was decoded and checked for safe unique relative
paths, exact file count, plain-file type, byte size, and SHA-256. All 75
entries matched, and there were no unlisted files other than
`FINAL-SHA256.csv` itself.

The reviewed analyzer SHA-256
`3dd4cac47f03823e4b8e5c4a074f1f185911dce7026227217e03596911a552ca`
was then run independently from only the 68 recorded pre-analysis input files.
All six generated outputs matched the retained analysis byte-for-byte:

- `arms.csv` — `4787682d650b16ed29946580554fc325d4efed7650c9ac50fb6c8fe2177eb7a7`
- `dial-samples.csv` — `1194c9a9303bebe616e43c27719cf4fb2c679e26ead47f893c08e7f447c02987`
- `input-files.csv` — `1e281445822fac317e1739194a6c19505bd9295c892e7987d52d100e62d8f386`
- `pairs.csv` — `846ed8993b3f1aff9a42be95b0170078dc5df6678329d1b12f190c3b54aef8bf`
- `summary.json` — `cc6eaec14903e87007eab354d335e20e4c81c78a7a7f783719be51ab78bf8411`
- `summary.md` — `42a56fb616d15bcb0f60aec3ff0aa2e1092d0908e83c19d731e075e5cd6c8d42`

## What remains

1. Own the repeatable Windows SOURCE socket-origin/write-blocked divergence as
   a new reviewed policy finding. Do not tune thresholds: the two layouts
   report opposite blocked ratios for similar or higher byte progress, and the
   resulting stream change is anti-correlated with throughput.
2. Preserve the fixed-cell `q_to_windows_large` 1.197 and
   `q_to_windows_mixed` 1.131 findings separately; the 40-GiB horizon cannot
   hide their fixed overhead.
3. Re-run the four-arm horizon only after a role-invariant controller signal is
   implemented, mutation-proved, and reviewed.
