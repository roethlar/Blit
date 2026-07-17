# ldt-4-r1-f1 — frame Windows evidence fetches with one exact payload tag

**Severity**: MEDIUM — harmless SSH/profile stdout could void every Windows
evidence fetch before an arm completes.
**Status**: Fixed and mutation-proved; neutral whole-change re-review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: review-fix commit pending final record

## Evidence

`scripts/bench_ldt4_rigw.sh:526-543` previously piped all stdout from a
PowerShell SSH command directly into `base64.b64decode(..., validate=True)`.
The registered `[Console]::Out.Write` added no newline, but any profile or
banner output became part of the strict payload and raised a decode error.
`fetch_windows_file` is used for manifests, component logs, landed evidence,
and the durable runtime-swap record.

## Predicted observable failure

On a Windows SSH session that emits any profile/banner stdout, the first
Windows evidence fetch fails and the harness retains a void/non-start session.
No corrupt file can be accepted because the following SHA comparison remains,
but the registered 96-arm run cannot proceed.

## What

Make the remote producer emit one `LDT4-FILE-B64|` line and make the local
decoder select exactly one such line. Unrelated stdout is ignored; missing,
duplicate, or malformed tagged payloads fail before bytes are written. The
existing independently fetched remote SHA still has to match the local file.

## Approach

`decode_windows_file_payload` normalizes CRLF/CR line endings, requires exactly
one prefix-at-line-start payload, uses strict base64 validation, and writes only
decoded bytes. The PowerShell write adds a leading newline so even a banner
without a terminator cannot share the tagged line. `fetch_windows_file`
handles pipeline failure explicitly and retains its exclusive output path and
SHA guard.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — tagged producer, strict single-tag decoder,
  explicit fetch failure, and offline framing self-tests.

## Guard proof

- `SELFTEST=1 ... scripts/bench_ldt4_rigw.sh` accepts banner + CRLF + one valid
  tag and rejects missing, duplicate, and malformed tags without SSH.
- Production mutation from `len(payloads) != 1` to `< 1` returned exit 1 with
  `duplicate Windows fetch payload tags passed selftest`; exact restoration
  returns `PASS (96 arms, no SSH)`.
- The pre-review generated-script parser check exposed that the initial fix's
  PowerShell quotes/backticks were not escaped through Bash. The exact source
  form now protects both layers. Removing only the quote escapes makes the
  offline safety test exit 1 with `Windows fetch framing is not escaped
  through Bash`; exact restoration returns green.
- A fresh read-only SSH probe compiled the exact generated guard + fetch,
  guard + prepare, and guard + normal-restoration PowerShell with
  `[ScriptBlock]::Create` and returned `LDT4-PARSER-PROBE|PASS`. It invoked
  none of those blocks. The local fetch decoder also accepted the parser
  wrapper's tagged payload and matched its independent SHA.

## Coder dispute

None.

## Known gaps

Hosted Windows CI remains unobserved. The parser proof compiled but did not
execute the generated blocks, so the first endpoint fetch still belongs to the
reviewed live harness run rather than this read-only check.

## Reviewer comments

Claude Fable 5/max returned the candidate over exact
`e41b871..0e48721` with `guard_confirmed=true`. Intake corrected the overly
broad trailing-newline wording but admitted the profile/banner failure.
Final fixed-SHA whole-change re-review is pending.
