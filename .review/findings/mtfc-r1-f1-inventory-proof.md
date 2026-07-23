# mtfc-r1-f1: Prove real inventory parsing and observed cleanup ownership

**Severity**: HIGH — an under-count after successful admission can leave an
Application Firewall rule live while clearing its recovery ledger.
**Status**: In progress
**Branch**: `master` (repo no-agent-branch rule)
**Commit**: `68460a7b`

## Evidence

`scripts/macos/with-temporary-firewall-rule.sh:146-180` parses the complete
inventory with rigid adjacent line pairs. The fake at
`scripts/macos/test-with-temporary-firewall-rule.sh:60-77` emits that same
assumed layout, so it cannot expose harmless real-format whitespace drift.

More importantly, `mtfc_cleanup_rule` skips removal whenever the parsed exact
count is zero, then accepts another zero and clears the ledger. If `--add`
succeeded but `after-add` never observed the exact rule, an under-count during
cleanup can therefore report `cleanup_verified=true` without ever issuing
`--remove`.

The target machine's read-only real inventory was captured on 2026-07-23:
`socketfilterfw --listapps` exited zero without sudo and currently has no blank
line after its header. That narrows the format claim but does not close the
self-mirroring fake or cleanup-ownership defect.

## Predicted observable failure

A harmless output-layout variation can reject every wrapped Mac test before
admission. More seriously, a successful add followed by an exact-path
under-count can leave the rule present, delete `owned-rule.v1`, and write
`cleanup_verified=true`; later tests then inherit stale firewall state with no
automatic recovery record.

## Approach

Keep the declared full-inventory count gate, but ignore blank lines and accept
surrounding whitespace in entry/status formatting before exact path equality.
Track whether successful admission ever observed the exact owned rule. Cleanup
may clear a successful-add ledger on a zero count only after that prior
observation; otherwise it fails closed with exit 90 and retains recovery state.
Add a realistic whitespace-layout case and a simulated post-add under-count
that proves the ledger cannot be cleared falsely.

## Files changed

- `scripts/macos/with-temporary-firewall-rule.sh` — tolerant complete parser
  and observed-ownership cleanup gate.
- `scripts/macos/test-with-temporary-firewall-rule.sh` — realistic formatting
  and under-count/recovery guards.

## Guard proof

- `mtfc_test_success_with_spaces_and_unrelated_entry` runs every inventory
  phase with blank lines plus expanded entry/status whitespace. Removing the
  production blank-line filter made the suite fail at case 1 with
  `malformed firewall inventory entry`; exact restoration returned 16/16.
- `mtfc_test_post_add_undercount_retains_ledger` hides the exact path only
  after successful add. Weakening `mtfc_rule_observed == 1` to the vacuous
  `>= 0` made the case return 71 and clear its ledger instead of required exit
  90; exact restoration retained the ledger, recovered the rule, and returned
  16/16.
- The production parser consumed the target's read-only real inventory with
  50 declared entries, 50 parsed entries, and one exact allowed match for the
  named stale test path.

Final gates on the restored bytes: Bash syntax, 16-case focused suite, docs,
`git diff --check`, formatting, strict workspace clippy, and the complete
workspace suite all pass. One earlier full workspace attempt had the unrelated
small-file timing aggregate report zero; its exact focused rerun and two later
complete workspace runs passed.

## Known gaps

No live mutation is authorized. The read-only target inventory validates
current command access and layout only; it is not committed because it contains
machine-specific application paths.
