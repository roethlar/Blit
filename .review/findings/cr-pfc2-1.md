# cr-pfc2-1: Root-wide write failures are incorrectly contained

**Severity**: HIGH — a mirror to a read-only destination volume can contain
every failed write and exit 0, silently leaving the backup incomplete.
**Status**: Verified
**Branch**: — (default-branch mode; this repo lands fixes on `master`)
**Commit**: filled at landing (fix commit on `master`, parent `9004535d`)

Reviewer provenance (generation pass): codex / gpt-5.6-sol / xhigh /
standard; codex-cli 0.146.0; range `4d2b888f..575e47e7`
(`.review/results/pfc-2-range.codex.json`, attempt 2, 2026-07-31).

## Evidence
`crates/blit-core/src/remote/transfer/sink.rs:140-146` —
`destination_root_live` treats any existing directory as a live root;
`:157-165` — `per_file_failure` therefore converts volume-level errors such
as EROFS into a per-file outcome; `transfer_session/mod.rs:4633-4650` — the
pfc-2 interim interlock makes contained outcomes fatal only for non-mirror
sessions, so a mirror absorbs them all.

## Predicted observable failure
Mirror to an existing read-only mount (NAS share remounted ro,
write-protected volume): every write fails, every failure is contained,
no extraneous entries exist to delete → session returns success, CLI exits
0, destination unchanged. Silent incomplete backup.

## What
The per-file/fatal classifier keys only on whether the destination ROOT
path is a live directory, not on whether the failure's *kind* indicates the
whole volume is unwritable. Volume-level unwritability (read-only
filesystem / write-protected media) is a root-wide condition wearing a
per-file error's clothes.

## Approach
As built: `VOLUME_UNWRITABLE_OS_ERRORS` (cfg-gated per platform — unix
`&[30]` EROFS, windows `&[19]` ERROR_WRITE_PROTECT; gated because the
namespaces collide: unix 19 is ENODEV, windows 30 is ERROR_READ_FAULT);
`io_error_says_volume_unwritable` (primary check
`ErrorKind::ReadOnlyFilesystem` — stable on the workspace's rustc
1.97.1 — with the raw codes as backstop); `volume_unwritable` walks
`eyre::Report::chain()` downcasting to `std::io::Error` (the write sites
wrap in `with_context`; matches the chain-walk idiom at
`remote/retry.rs:31`); `failure_is_containable` =
`destination_root_live && !volume_unwritable` — both halves needed: the
root check cannot see a read-only mount (it reads as a live directory,
which is what made the finding dangerous), the kind check cannot see a
vanished root. `per_file_failure` and the two sibling classification
sites (the `FileBlock` arm of `write_payload`, the prepared-destination
match in `write_file_stream`) all gate on the shared predicate.

## Files changed
- `crates/blit-core/src/remote/transfer/sink.rs` (+196/−11) — the
  constants/predicates at :149-200, the classifier gate at :210-221, the
  two sibling sites, and five tests.

## Guard proof
- `read_only_filesystem_error_refuses_containment`,
  `read_only_volume_refuses_containment_for_single_file_destination`,
  `write_protected_volume_raw_os_code_refuses_containment` — all three
  FAIL red with `failure_is_containable` reverted to
  `destination_root_live` alone (SHA-256-verified byte-identical
  restore; boundary tests and every pre-existing pfc-2 containment test
  stayed green under the revert, so the guards detect exactly the
  classification decision).
- Boundary the other way:
  `ordinary_permission_denied_still_contains_one_file` and
  `the_other_platforms_code_is_not_a_volume_signal` (pins the cfg
  gating — the other platform's number must still contain).
- Test-local `VOLUME_UNWRITABLE_CODE` pins the platform number
  independently of the production constant so editing the constant alone
  cannot make the tests vacuous.

## Coder dispute (if any)
None.

## Known gaps
`StorageFull` (disk fills mid-mirror) is arguably also root-wide but is
transient and not named by the finding; left contained, noted for pfc-4/5
surfacing.

## Reviewer comments
Reviewer: codex / gpt-5.6-sol / xhigh / standard
Harness: codex-cli 0.146.0 (detached dispatch, disposable detached
worktree at the fix commit; workspace-write scoped to the worktree).
Reviewed SHA: `2ad657430758285a165020ceffb096bdf2edeb2a`;
base SHA: `9004535d8bf3f2657b4472e52f89e2af7627c470`.
guard_confirmed: **true** (reviewer independently executed
revert → 3 volume tests red / boundary tests green → restore → green in
its own worktree). Verdict: **accepted**. 2026-07-31T03:37Z. Comments:
none. Record: `.review/results/cr-pfc2-1.codex.json` (committed).
