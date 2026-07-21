# ldt-4-live-f4 — keep generated Windows start-command paths on one line

**Severity**: MEDIUM — every Windows-responder arm exits before daemon launch,
so the registered matrix cannot produce evidence.
**Status**: Fixed, mutation-proved, tactically reviewed clean, and additively staged; live retry pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `d53b5fdd3b85fd61f377de917e16ba19aa65d137`

## Evidence

Exact harness candidate `a39f0c5` passed additive staging, the q Bash 3.2
96-arm self-test, all three cross-endpoint fixture manifests, and the live
start environment gate in retained session
`ldt4-20260717T062334Z-a39f0c570191`. Its first arm then failed the Windows
launcher identity check before daemon launch or timing.

The retained `start.cmd` contains eight unintended CRLFs inside its two
dynamic commands. The `launch.ok` test spans three physical lines, and the
daemon command spans seven, splitting the config, stdout, and stderr paths.
PowerShell parsed the two unparenthesized concatenations in the `@(...)`
constructor as array concatenation; the requested CRLF join faithfully emitted
20 items instead of the intended 12 command lines. `cmd.exe` exited before the
two-second identity check, so the deliberately fail-closed observed launcher
identity was empty.

No timing row was accepted: `runs.csv` contains only its header. q and Windows
port 9031 are closed, no session-owned process remains, Windows' active daemon
was restored to exact prior SHA-256
`1510d8d04e503967baf250c19cfcd7af4363bc9a22038f68396ea6eb45890512`, and
the tested daemon remains retained separately.

## Predicted observable failure

Every arm with Windows as responder creates and flushes `start.cmd`, launches a
short-lived malformed `cmd.exe`, then fails with a blank launcher identity.
No `daemon.pid`, daemon identity record, listener, or transfer can exist.

## What

Make both concatenated `start.cmd` entries explicit PowerShell array elements
and pin both production forms in the no-SSH harness self-test.

## Approach

Parenthesize the complete `launch.ok` gate and daemon-command expressions
inside `$startText`'s `@(...)` constructor. The generated content stays 12
CRLF-separated batch lines, but the two dynamic lines can no longer be split
into path fragments by PowerShell's array-concatenation precedence.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — correct the two generated PowerShell array
  elements and guard their exact production forms.

## Guard proof

- Local PowerShell 7.6.3 reproduced 20 malformed items with the live form and
  exactly 12 intact lines with the corrected form.
- Actual Windows PowerShell 7.6.3 rendered the corrected expression entirely in
  memory and returned `LDT4-STARTTEXT|PASS|12`, including a path containing a
  space. The probe created no file or process.
- Removing only the two production parentheses made the 96-arm no-SSH
  self-test fail with
  `Windows start command paths are not explicit array elements`. Exact
  restoration returned `PASS (96 arms, no SSH)`.
- Bash syntax, all 75 analyzer tests, formatting, strict workspace clippy, the
  full workspace suite, documentation checks, and diff checks pass.

## Coder dispute

None.

## Known gaps

No completed/timed live arm or transfer datum exists. Exact `d53b5fd` is
additively staged and its q Bash 3.2 self-test passes. The first post-staging
launch failed closed in Windows path preflight before evidence reservation:
the harness-pinned `10.1.10.177` is absent while `NETWATCH-01` now owns
`10.1.10.173`. A fresh live run waits for the registered address to return (or
a separately planned and reviewed harness identity change). Formal Fable
openreviews remain on owner-directed capacity hold.

## Reviewer comments

Grok 4.5/high reviewed exact range `a39f0c5..d53b5fd`, reproduced old 20-item
versus fixed 12-line PowerShell generation, ran the complete Bash 3.2 no-SSH
self-test, independently mutated both production parenthesizations, audited the
start/teardown boundary, and returned `clean` with no findings. This is tactical
advisory review, not formal acceptance. Record:
`.review/results/ldt-4-live-f4-r1.grok-verdict.md`.
