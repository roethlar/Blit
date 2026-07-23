# macOS test firewall cleanup — invalid review attempt and intake

- Reviewer: `claude-opus-4-8` via Claude Code `2.1.218`, effort `max`
- Reviewed range:
  `53deac254dd61d60293e7b25c8cb3c459378d07b..d35a0b12b0caa56447b9b5f2f9a69356f7c39bc0`
- Review session: `92a8246f-6619-4306-a074-509fbc6c8a90`
- Detached worktree: `/tmp/blit-openreview-mtfc-d35a0b1-r1`
- Raw envelope:
  `.review/results/macos-test-firewall-cleanup-r1-attempt1-invalid.claude.json`
- Recorded: `2026-07-23T18:07:43Z`
- Dispatch outcome: non-authoritative

The invocation exited zero, used only `claude-opus-4-8`, returned exact
base/head identities and one schema-valid candidate, and left the detached
worktree clean. It does not satisfy this repo's formal-review contract:
the portable schema omitted the repo-required literal
`guard_confirmed: true`, the launch denied the reviewer's attempted fake suite,
and the call used single-result JSON rather than the required observable
stream with reviewer heartbeats. It therefore cannot accept or reject the
range and will not be described as a formal verdict.

## Candidate intake

`mtfc-r1-f1-inventory-proof` — **ADMITTED (HIGH)**. The exact target's
read-only real `socketfilterfw --listapps` command exits zero without
administrator authorization and its current format matches the parser; the
reviewer's claimed blank line was not present on this machine. The underlying
defect is nevertheless real: the fake emits only the parser's assumed layout,
and after a successful `--add`, a zero exact-path count before cleanup can
currently skip `--remove`, clear the ledger, and claim success even if the
post-add ownership check never observed the rule. That recreates the
detectable stale-rule failure this slice must prevent.

The bounded correction tolerates harmless inventory whitespace while still
validating the declared complete entry count, exercises a realistic format
variant, and requires a rule observed after successful add before a zero-count
cleanup may clear the ledger. A simulated post-add undercount retains the
ledger, returns cleanup failure, and recovers only after the exact entry is
visible again. The correction landed in `68460a7b` and its mutation proof in
`65ae700d`.

No live firewall mutation, sudo command, hardware transfer, push, branch
operation, or retained-artifact deletion occurred.

## Closure

This attempt remains non-authoritative and no reviewer verdict is claimed.
Per D-2026-07-23-7, a fresh external review is not pending because the owner
did not authorize one. The admitted finding is closed by the local code and
guard evidence above.
