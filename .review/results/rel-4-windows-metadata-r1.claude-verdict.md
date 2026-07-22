# rel-4 Windows metadata — Claude adjudication

- Reviewer: `claude-opus-4-8` via Claude Code `2.1.217`, effort `max`
- Reviewed range: `2ee9b630f8fe0ec95139e70e7d066a474b8532d0..3013e101ee45b973d9ffe59c11f4655857e6fa6e`
- Review session: `49148b3a-22be-456b-b41a-2f6c4eca4aab`
- Detached worktree: `/tmp/blit-rev-3013e10-r2`
- Independent verdict: `FINDINGS`
- Guard confirmed: `true`

`openreview claude (claude-opus-4-8 @ max, competitive) over 2ee9b63..3013e10: 6 material issues`

The first launcher attempt was discarded because its event pipe stopped being
drained before a verdict. The second attempt is authoritative. It ended at the
exact reviewed commit with a clean detached worktree. Its independent mutation
removed the metadata-mismatch repair arm: the focused guard turned red, then
the exact restore returned all 388 blit-core library tests to green.

All six findings are admitted and remain release-blocking until fixed:

1. **Fixed — tar metadata allocation budget.** The binary receiver now applies
   one aggregate header budget before each metadata allocation, the sender
   preflights the same bound, the member-count cap matches the planner, and tar
   planning/in-stream splitting include declared ADS payload bytes. The focused
   guard turned red when the budgeted metadata read was bypassed and returned
   green after exact restoration.
2. **Fixed — attribute convergence.** The contract now carries only durable
   READONLY, HIDDEN, SYSTEM, and ARCHIVE bits. The destination reads that mask
   back after applying it and fails the file on mismatch; a simulated
   successful setter that dropped HIDDEN turned the focused guard red when the
   convergence check was disabled.
3. **High — per-file ADS isolation.** A recoverable source stream problem must
   mark that file unreadable and preserve the mirror/move safety gate, not stop
   every unrelated file. An unrepresentable destination stream set should be
   treated as needing replacement rather than making comparison unusable.
4. **Medium — cross-platform policy.** Windows-to-non-Windows copies need an
   explicit, warned downgrade choice. Strict preservation remains the default;
   rejection must happen before any resume block can change the destination.
5. **Medium — local mtime precision.** Restamp local Windows files from the
   source `SystemTime`, preserving sub-second precision after ADS application.
6. **Low — resume hash clone.** Hydrate Windows metadata without cloning and
   discarding the destination block-hash vector on every platform.

No finding is being waived as bookkeeping. Each receives its own fix commit,
guard proof, and verification record before rel-4 can be considered accepted.
