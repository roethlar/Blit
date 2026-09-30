# cr-ssc2-1: Windows metadata hydration bypasses per-member shard containment

**Severity**: MEDIUM — on Windows a vanished shard member still aborts the run before the packer's containment (against D-2026-09-28-2); ssc-2's own Windows CI leg would fail
**Status**: Verified — closed by ssc-4 `c3a38876` (per-member hydration, D-E) plus the guard added here
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: `19de5136 (closed by ssc-4 c3a38876)`
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 6bc3d08f..905ddb37, record .review/results/ssc-2-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/payload.rs:70 — every tar member is still passed through `hydrate_payload_header(...)?` before `build_tar_shard`; default Windows scans populate `windows_metadata`, so a vanished member makes hydration return early instead of producing a `skipped` entry.

## Predicted observable failure
On Windows, deleting a small shard member after scanning aborts the entire transfer instead of reporting that file and landing its shard-mates. The newly added vanished-member and fully-skipped-shard integration tests will consequently fail under the repository's Windows `cargo test --workspace` CI job.

## Reviewer's suggested approach
Hydrate members individually inside the containment loop, converting per-file hydration/open failures into `FileFailure` entries and passing only successfully hydrated headers to the packer.

## What
No production change in this commit: ssc-4 (`c3a38876`) already made shard-member hydration per member (`prepare_payload_with` in `remote/transfer/payload.rs` collects a failing member into the shard's `skipped` list instead of `?`). This commit adds the guard the finding asked for on the remote carriers (the local route already had `local_hydration_failure_is_a_per_file_skip_on_shards_and_single_files`).

## Guard proof
`{in_stream,data_plane}_shard_member_hydration_failure_is_skipped_and_reported` (`assert_shard_member_hydration_failure_skipped`: three small files planned as one shard; the hydrator deletes `vanished.txt` and fails with the Windows not-found text; the member is reported with a `source:` reason, both shard-mates land byte-exact, both ends agree, both initiators). Mutation: the pre-ssc-4 shape restored in the shard hydration loop (`hydrate(..)?`) → the data-plane guard FAILED (source pipeline faulted; `scratchpad/cr-ssc-mutations.txt`); restored → green.

## Known gaps
The Windows-only real named-stream variant is ssc-4's `cfg(windows)` guard, which runs on Windows CI only.
