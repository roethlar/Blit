# cr-ecm29-5: a staging directory made under a narrow umask was refused

**Severity**: MEDIUM — brought in by cr-ecm28-3: under a umask of 0200 the private staging directory was made 0500, the exact 0700 check refused it, and `copy --resume` exited 2 where the base copied the file
**Status**: Fixed on Unix, awaiting review
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: this commit (`Fix cr-ecm29-5`)
**Reviewer**: codex / gpt-6.2-astra (as dispatched) / max / frontier, codex-cli 0.171.0 headless one-shot, openreview `combined-r30` over `e3ea9517..fd603571` (verdict `acceptable_with_changes`, capability_ok and guard_confirmed true); `.review/results/combined-r30.codex.md`. It completes cr-ecm28-3 (`d3c390f8`).

## Evidence
"I ran `copy --resume` with a `0600` source, a `0644` destination, and umask `0200`. The head exited 2 because its newly created `0500` staging directory failed the exact `0700` check. The same CLI with the base `resume.rs` copied all 300 KiB and exited 0." The reviewer's required change: "Establish staging permissions through the held directory."

## Intake
Admitted as the reviewer put it.

## What
`open_stage` checks that the staging directory it opened grants no one else anything (`mode & 0o077 == 0`), rather than that it is exactly 0700. A mask only takes away, so the one this run made always passes. It still checks that the directory is a directory and owned by this run's user. It then sets the directory's mode to 0700 through the handle it opened.

## Guard proof
- `local_copy_mode::a_resume_under_a_narrow_mask_makes_its_fresh_partial` (`blit-cli`, Unix): `blit copy --resume` runs in a child with umask 0200. The source is 0600 and 300 KiB; the destination is 0644 with its first 1,000 bytes, too wide to reuse. The copy succeeds, the destination holds the source's bytes at 0600, and nothing is left beside it.
- Mutations, each restored byte for byte:
  - the mode not set through the handle → red (this run cannot write in a 0500 directory);
  - the exact-0700 check put back → red.
- The resume, sink and session tests (339) and the three local-copy-mode tests pass.

## Files changed
- `crates/blit-core/src/copy/file_copy/resume.rs` — `open_stage`.
- `crates/blit-cli/tests/local_copy_mode.rs` — guard.

## Known gaps
- Unix only.
