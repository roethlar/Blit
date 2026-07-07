# otp-6b — mirror on the session (the one delete rule)

**Plan**: `docs/plan/ONE_TRANSFER_PATH.md` (Active, D-2026-07-05-4), slice otp-6.
**Contract**: `docs/TRANSFER_SESSION.md`.
**Builds on**: otp-6a (filters — the scope the FilteredSubset mirror reuses).

## Staging

otp-6 = "mirror + filters (one delete rule)". otp-6a landed filters; **this slice
is otp-6b**: the mirror delete pass. It reuses otp-6a's filter for the
FilteredSubset scope, so filters had to land first.

## What

Until now the session refused any mirror-enabled open ("mirror is not implemented
… (otp-6)"). This slice implements the ONE delete rule for the unified session:
the DESTINATION (the end that holds the tree) computes the extraneous set from the
COMPLETE source manifest, filter-scoped and scan-complete-guarded, and deletes
locally. This replaces the three divergent per-direction purges (local engine,
daemon push, client pull) with a single rule on the session — the old paths keep
working until the otp-10 cutover deletes them.

## Predicted observable failure (closed by this slice)

- A mirror-enabled session leaves extraneous destination files in place
  (`entries_deleted == 0`, stale files remain) instead of deleting them.
- FilteredSubset would either delete out-of-scope files (data loss) or All would
  leave them.
- An incomplete source scan + mirror would delete files the source still has
  (they were merely unreadable mid-scan) — silent data loss.

Pinned by five role-suite tests (see below).

## Approach

- **Set computation reuses the tested planner.** New
  `MirrorPlanner::plan_session_deletions(dest_root, source_files, filter)`
  (`mirror_planner.rs`) enumerates the dest through `filter` and diffs against the
  source set via the existing `plan_from_sets` (CasefoldKey — Windows-safe). The
  session manifest is files-only (dirs implicit — `spawn_manifest_task`), so the
  method derives kept-directories from each source file's parent chain: a dir is
  extraneous iff no kept file lives under it. Same derivation the daemon's
  `plan_extraneous_entries` does, centralized as the session's rule.
- **Execution** (`transfer_session/mod.rs::mirror_delete_pass`, blocking pool):
  files then dirs deepest-first, each containment-checked against the canonical
  dest root before any FS op. `remove_dir` (not `remove_dir_all`) so out-of-scope
  content is never removed — under FilteredSubset an extraneous dir still holding
  filter-excluded files fails ENOTEMPTY and is left alone (engine/mirror.rs R58-F6
  idiom: `ErrorKind::DirectoryNotEmpty` || raw_os_error 66 for macOS/BSD); under
  All the tree was enumerated unfiltered, so a dir here is empty.
- **Wiring** (`destination_session`): accumulates `source_files` from every
  `ManifestEntry` when mirroring; at `ManifestComplete` refuses (fault) if the scan
  was incomplete; at `SourceDone` — after every payload is written, so the tree is
  final — runs the delete pass and fills `TransferSummary.entries_deleted`.
- **Scope**: FilteredSubset enumerates the dest through the user filter (out-of-
  scope entries never candidates); All enumerates the whole tree (`FileFilter::
  default()`) and deletes anything absent from the filter-produced source set —
  matching proto `MirrorMode` semantics.
- **Validation**: `destination_open_validator` stops refusing mirror; it now
  refuses `mirror_enabled` with an OFF/UNSPECIFIED kind (protocol violation) and
  validates the filter globs at OPEN (peer-notified), symmetric with the source.

Invariance holds: the delete runs on the DESTINATION regardless of which end
initiated; the mirror config lives in the open, which both ends share.

## Files changed

- `crates/blit-core/src/mirror_planner.rs` — `plan_session_deletions`.
- `crates/blit-core/src/transfer_session/mod.rs` — `destination_open_validator`
  (mirror validation replaces refusal); `mirror_delete_pass`; `destination_session`
  accumulate/guard/execute + `entries_deleted`; `MirrorMode` import.
- `crates/blit-core/tests/transfer_session_roles.rs` — 5 tests (below);
  `run_mirror_session` helper; removed the obsolete
  `mirror_request_is_refused_until_its_slice_lands`.

## Tests / guard proof

- `mirror_all_purges_extraneous_under_both_initiators` (file + nested file +
  pruned dir = 3; tree == source after; both initiator roles),
  `mirror_filtered_subset_preserves_out_of_scope`,
  `mirror_all_purges_out_of_scope_even_when_filtered`,
  `mirror_enabled_without_scope_is_refused`,
  `mirror_refused_when_source_scan_incomplete` (scripted peer).
- Guard proofs (run live 2026-07-06): (1) neutralize the delete pass (`if false`)
  → `mirror_all_purges_extraneous_under_both_initiators` FAILS (stale file remains,
  `entries_deleted != 3`); restored → PASS. (2) neutralize the scan-complete guard
  → `mirror_refused_when_source_scan_incomplete` no longer refuses (the dest
  proceeds; the scripted peer blocks awaiting the Error frame that never comes,
  i.e. no refusal is emitted); restored → PASS.
- Full gate green: fmt/clippy clean, `cargo test --workspace` → **1528 passed**
  (1524 − 1 obsolete refusal + 5 mirror tests), 2 ignored. Count did not drop.

## Known gaps

- The old per-direction purges (engine `apply_mirror_deletions`, daemon
  `purge_extraneous_entries`, client `delete_listed_paths`) still exist and run for
  the pre-cutover verbs; they are deleted at otp-10 when the verbs route through the
  session. The w7-1 mirror-executor consolidation is now largely subsumed by this
  rule (re-check at otp-10).
- Symlink handling follows the enumerator's default (files-only manifest); no
  special symlink-target reconciliation beyond what the old paths did.
- resume (otp-7) is still refused.
