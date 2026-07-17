# ldt-4-r3-f1 — promote canonical fixtures with the exclusive rename primitive

**Severity**: LOW — a cross-device `mv` fallback could leave a partial tree at
the stable source path during an already-failing fixture-staging launch.
**Status**: Fixed and mutation-proved; neutral whole-change re-review pending.
**Branch**: `master` (repo policy forbids agent-created branches)
**Commit**: `1302b906c4315b188bf48636da59c8879e443693`

## Evidence

`scripts/bench_ldt4_rigw.sh:800` previously promoted the exact-manifest-
validated incoming tree with `mv -n` into the stable staging directory. The
same harness already uses `rename_q_directory_exclusive`, implemented with
macOS `renameatx_np(RENAME_EXCL)`, for the analogous no-clobber transition from
an active destination to retained evidence.

The configured session and staging paths share a `/Users/michael` prefix, but
the harness does not assert that no separate filesystem is mounted below
either path. BSD `mv` can handle a cross-device rename as copy followed by
source removal instead of failing the operation as non-atomic.

## Predicted observable failure

If either configured subtree resides on another device, interruption during
promotion can leave a partial tree at the stable fixture source path. The next
launch correctly refuses its manifest mismatch, but the fixed path is poisoned
and requires manual intervention because the harness contains no cleanup path.

## What

Promote the validated incoming tree directly to the exact stable destination
with the harness's existing exclusive atomic rename helper. Remove `mv` from
the runtime prerequisite list because the harness no longer invokes it.

## Approach

`stage_fixtures` now calls
`rename_q_directory_exclusive "$incoming" "$local_destination"` after the
same manifest equality and destination-absence checks. `RENAME_EXCL` keeps the
no-clobber decision inside the rename syscall and fails closed on a cross-device
layout instead of degrading to a copy. The postcondition still requires the
incoming name to be absent and the stable path to pass the registered path
guard.

## Files changed

- `scripts/bench_ldt4_rigw.sh` — exclusive stable-path promotion, obsolete
  `mv` prerequisite removal, and the offline structural guard.

## Guard proof

- The 96-arm no-SSH self-test requires the exact exclusive-helper call after
  manifest comparison and rejects any `mv -n` fallback in `stage_fixtures`.
- Replacing only that helper call with the old `mv -n` form makes the self-test
  fail with `canonical fixture promotion is not an exclusive atomic rename`;
  exact restoration returns `PASS (96 arms, no SSH)`.
- Formatting, strict workspace clippy, the full workspace suite, all 75
  analyzer tests, documentation checks, and diff checks pass at this slice.

## Coder dispute

None.

## Known gaps

The live promotion path has not run. A clean final fixed-SHA whole-change review
and a fresh exact harness launch remain before hardware evidence.

## Reviewer comments

Claude Fable 5/max returned the candidate over exact `4e0fdc3..ef48920` with
`guard_confirmed=true`. Intake admitted the latent cross-device failure because
the existing accepted helper removes it without adding a new mechanism.
Final fixed-SHA whole-change re-review is pending.
