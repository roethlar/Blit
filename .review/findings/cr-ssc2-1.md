# cr-ssc2-1: Windows metadata hydration bypasses per-member shard containment

**Severity**: MEDIUM — on Windows a vanished shard member still aborts the run before the packer's containment (against D-2026-09-28-2); ssc-2's own Windows CI leg would fail
**Status**: Open — expected closed by ssc-4 (per-member hydration, D-E); verify on landing
**Branch**: — (default-branch mode; fixes land on master)
**Commit**: (filled after the fix commit)
**Reviewer**: codex / gpt-5.6-sol / xhigh / standard (D-2026-07-31-3 standing codereview; range 6bc3d08f..905ddb37, record .review/results/ssc-2-range.codex.json)

## Evidence
crates/blit-core/src/remote/transfer/payload.rs:70 — every tar member is still passed through `hydrate_payload_header(...)?` before `build_tar_shard`; default Windows scans populate `windows_metadata`, so a vanished member makes hydration return early instead of producing a `skipped` entry.

## Predicted observable failure
On Windows, deleting a small shard member after scanning aborts the entire transfer instead of reporting that file and landing its shard-mates. The newly added vanished-member and fully-skipped-shard integration tests will consequently fail under the repository's Windows `cargo test --workspace` CI job.

## Reviewer's suggested approach
Hydrate members individually inside the containment loop, converting per-file hydration/open failures into `FileFailure` entries and passing only successfully hydrated headers to the packer.

## What
(coder fills in)

## Guard proof
(red/green proof; mutation described)

## Known gaps
(none yet)
