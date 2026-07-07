# sf-1-tripwire-harness — adjudication

reviewer: gpt-5.5 (codex, `.review/results/sf-1-tripwire-harness.codex.md`)
slice commit: `7202c1a` · fix commit: `80633df`
verdict: NEEDS FIXES — 6 findings, 6 accepted (one scoped), 0 rejected

## Findings

1. **High — teardown `rm -rf $SESSION` can delete a dir it didn't
   create** (`bench_tripwires.sh` teardown/setup). ACCEPTED. `mkdir -p`
   on the timestamped path would adopt a pre-existing dir. Fixed: the
   session dir is created with plain `mkdir` (fails if it exists, run
   aborts before `SESSION` is set) plus a `$$` suffix against
   same-second collisions; `SESSION` is only assigned after successful
   creation, so teardown can only ever remove a dir this invocation
   made.

2. **High — "clean" verdict without full matrix coverage** (summary
   awk). ACCEPTED. Skipped tools and all-failed rows (e.g. rclone auth)
   silently shrank the rival set. Fixed: the expected transport set is
   fixed by the plan (local: blit/rsync/rclone/cp; remote: blit/rsyncd/
   rsync_ssh/rclone_sftp); any expected tool with no successful run in
   a cell marks it `INCOMPLETE (no run: …)` and the run exits 4. Trips
   still take precedence (exit 3). Verified: loopback run without sftp
   auth now shows INCOMPLETE on every remote cell.

3. **Medium — `SPIN_DAEMONS=0` couldn't run rsyncd cells**. ACCEPTED,
   and adjudication found the flagged gating was the shallow half:
   external mode was broken for every remote cell, because tools write
   daemon-relative paths (module root) while the harness prepared
   `$SESSION` paths — coincident only when the harness spun the daemons
   itself (`--root $SESSION`). Fixed: external daemons are documented
   to export `REMOTE_ROOT` (blitd module + rsyncd module `bench`), a
   `REL` session prefix is applied to all daemon-relative paths, and
   rsyncd cells are gated on a live `--list-only` probe instead of
   `RSYNCD_STARTED`. Verified by execution against externally spun
   daemons on loopback.

4. **Medium — baseline diff didn't flag ±10% or missing cells**.
   ACCEPTED, scoped: cells outside the band now get an explicit
   `outside +/-10% noise band` flag and unrun baseline cells are
   listed. Deliberately NOT wired to the exit code — the committed
   baseline binds to the 10 GbE rig, and failing every non-rig run
   would make the harness unusable elsewhere; the pass/fail use of the
   band is sf-4's rig re-measure, read by a human/agent from the
   flagged output.

5. **Low — finding doc pointed at a verdict file the commit didn't
   contain**. ACCEPTED. The Tests section now states the result
   directly: fmt/clippy clean, `cargo test --workspace` 1479/0 across
   37 suites (baseline count held).

6. **Low — REVIEW.md sf-1 row had no commit sha**. ACCEPTED. Row now
   carries `7202c1a` + fix `80633df` and flips `[x]` with this record.

## Validation after fixes

- `bash -n` clean; `bash scripts/agent/check-docs.sh` OK.
- Local-only run: exit 3 (cp trips blit on tiny local copies on the
  dev box — tripwire working as designed; rig verdicts are sf-4).
- Loopback spun-daemons run: remote cells + scale probe green,
  INCOMPLETE correctly reported for the auth-less rclone_sftp cells,
  no stray daemons, session dir removed.
- Loopback external-daemons run (`SPIN_DAEMONS=0`): rsyncd probe found
  the daemon, all REL-prefixed paths worked, stream counts read from
  `BLITD_LOG`, teardown removed only the session dir.
- Cargo suite unaffected by the fix commit (scripts + docs only).
