# Reopened: audit-h1-mirror-relay-incomplete-scan

**Reviewer**: gpt (relayed via owner), 2026-06-05.

## Finding

The mirror destructive-confirmation prompt fires **before** the new audit-h1
reject-gate, so without `--yes` an operator can defeat the data-loss guard by
answering "no" (or hitting EOF / empty input).

**Code paths**:
- `crates/blit-cli/src/transfers/mod.rs:180-189` — mirror confirm-prompt runs first
  (`if mode.is_mirror() && !args.dry_run { confirm_destructive_operation(...) }`)
- `crates/blit-cli/src/transfers/mod.rs:230-263` — new audit-h1 reject-gate runs
  inside `select_transfer_route(...) → TransferRoute::RemoteToRemoteRelay`, which is
  reached **after** the confirm prompt.

Concrete reproduction:
```
$ blit mirror --relay-via-cli host-a:/m/ host-b:/m/   # no --yes, no stdin
starting mirror host-a:/m/ -> host-b:/m/
Mirror will delete extraneous files at destination 'host-b:/m/'. Continue? [y/N]: 
Aborted.
$ echo $?
0
```

The operator sees a normal abort, never learns the combination is unsafe, and the
process exits 0. A future automated wrapper that pipes `n\n` or runs without stdin
gets the same silent success, indistinguishable from a deliberate user no.

**Test gap**: `crates/blit-cli/src/transfers/mod.rs:788` — the
`detach_args` helper hardcodes `yes: true`, which makes the existing
`mirror_rejected_with_relay_via_cli_for_remote_to_remote` test skip the confirm
prompt entirely. The test passes because it never exercises the unsafe interleaving.

## Required fixes

1. Move the audit-h1 reject-gate **before** the mirror confirm prompt. It belongs
   with the other early-bail data-loss gates (`--null`, `--detach`, etc.). The
   conventional place is either:
   - inside `select_transfer_route`'s output processing path, or
   - as an explicit early check on `(mode.is_mirror() && args.relay_via_cli &&
     remote→remote)` before the prompt at line 180.
   The second option is closer to the symmetric `--detach` gate at line 168 and
   makes the no-prompt invariant obvious.
2. Add a regression test that asserts the gate fires with `yes: false`. Either:
   - Generalize `detach_args` to take a `yes` parameter, or
   - Add a small helper `transfer_args_yes(yes: bool, …)` and use it for the
     audit-h1 test family.

## Validation expected after fix

- New test asserts the bail fires regardless of `--yes`.
- The existing
  `mirror_rejected_with_relay_via_cli_for_remote_to_remote` continues to pass
  (or is replaced by the new yes=false version).
- The audit-h1 mirror test pinning the bail message remains.

## Scope

Same finding, fix-up. Original analysis at
`.review/findings/audit-h1-mirror-relay-incomplete-scan.md` still applies.
