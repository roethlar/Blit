# ldt-4 rig-W sustained-controller evidence — 2026-07-22

**Status:** Structurally valid and independently recomputed, but
`REVIEW_REQUIRED`; this is not ldt-4 acceptance evidence.

## Identity

- Session: `ldt4-20260722T001611Z-04e80082e12c`
- Parent fixed-matrix session: `ldt4-20260721T224319Z-96a4e3b03caf`
- Parent copied-evidence inventory:
  `713cb4624e6f64a3863b67101fb9a3f3df288306d3e6f418c19501428711990b`
- Artifact build: `406a7e5854593b7a7a151f9b6d9cdf1be8a9cd77`
- Harness/analyzer: `04e80082e12ce9836eda43afc70fb3b2d0eb07c9`
- Matrix: four valid 5 GiB arms, two initiator-layout pairs, two physical
  byte directions
- Completion: `MEASUREMENTS-COMPLETE` is present, `SESSION-VOID` is absent,
  and the prior Windows daemon was restored normally and byte-for-byte

## Retained evidence

The copied session payload contains 74 files and 106,932 bytes before this
README. `FINAL-SHA256.csv` records and independently verifies every other
copied file (73 entries); its SHA-256 is:

`17348aaa261b936e04c104553d7b5c4bbcf008968306a29c4dea922535110eef`

The payload remains retained at its original q path:
`/Users/michael/blit-ldt4-evidence/ldt4-20260722T001611Z-04e80082e12c`.
The 5 GiB sources and all four landed payloads remain retained on their
respective endpoints.

## Result

The exact analyzer returned:

`REVIEW_REQUIRED: 4 arms; arm_review=4, decision_review=0, performance_review=0`

All four payloads landed with exact five-file/5 GiB manifest identity. Role
pairs matched their empty accepted-operation sequences, so there is no
transition-parity finding. Every arm instead reported
`NO_ACCEPTED_ADD_ABOVE_FLOOR`, with floor = peak = final = 4 and zero tuner
samples:

| direction / initiator | duration | samples | accepted operations |
|---|---:|---:|---|
| q→Windows / source | 6,412 ms | 0 | `[]` |
| q→Windows / destination | 4,375 ms | 0 | `[]` |
| Windows→q / destination | 4,616 ms | 0 | `[]` |
| Windows→q / source | 20,670 ms | 0 | `[]` |

The long byte drain did not create a tuning horizon. Each SOURCE received
terminal demand for all five advertised files in 3.1–5.2 ms and sealed
membership in 3.3–5.4 ms, while data-plane completion followed 4.3–20.6
seconds later. Exact code stops the tuner once terminal demand is known and no
payload remains to queue. The five-file fixture therefore queued completely
before the first 500 ms tuner tick. This is a workload-shape defect: a future
diagnostic must keep SOURCE admission backpressured across the required busy
ticks, not merely keep already-queued bytes draining for longer.

## Independent recomputation

The copied final inventory was decoded and checked for safe unique relative
paths, exact file count, plain-file type, byte size, and SHA-256; all 73 entries
matched, and there were no unlisted files other than `FINAL-SHA256.csv` itself.

The reviewed analyzer SHA-256
`32e7be5880d6f248cb066b18056385f9ce945e0bf68f44cfa0f639de2a3b44d4`
was then run independently from only the 66 recorded input files. All six
generated outputs matched the retained analysis byte-for-byte:

- `arms.csv` — `f51a147d5bc066870986be4e66abbc15f10ee557946fc036bf0771687a40a084`
- `dial-samples.csv` — `bf4bda59742d174f21d1e4088746bf4d938638cc88850330b22f18f9bb304cb5`
- `input-files.csv` — `9e939800a282a057415e3019df19ba144e0d32639690d4744a2da0f1838b684c`
- `pairs.csv` — `6ac2f47482bb84a283fef4bf512bdabb47e895cd1d43b58197c084770730e20a`
- `summary.json` — `565d9e925766d65326c663bd7d76bb6578781af9c3776f51c88fb3d3bc0f90c1`
- `summary.md` — `72138b61f1c9edf529f048f06e3bfd49a6f4587064eced6faaade170cb1ea603`

The first recomputation scratch path had the wrong basename and was refused by
the analyzer's exact session-id gate before analysis. A fresh scratch parent
with the required session basename produced the byte-identical result above;
neither endpoint nor retained evidence was changed.
