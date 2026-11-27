I would make `--files-from` an exact copy manifest. Someone handing Blit a PowerShell or `find` listing should be able to tell which entries landed and why any failed. An omitted name must never authorize deletion. I would read the list once, resolve it against the source, and carry each selected path through copying, reporting, and replay. The draft’s proposed first slice is the right one: refuse `mirror --files-from` before confirmation. I would also keep its rule that listing a directory selects the directory and only its explicitly listed descendants. [Draft plan](/home/michael/dev/gitea_repos/blit_v2/docs/plan/FILES_FROM_LISTS.md:18)

**Verdict — `acceptable_with_changes`.** I would require six revisions before approving the plan.

1. **Define how the producer conveys the format.** I verified that `2d 4e 78 00` is both the BOM-less UTF-16LE name `中x` and the NUL-terminated name `-Nx`. Neither proposed ambiguity test fires. I also verified that `find -print` emits identical bytes for one filename containing a newline and for two separate filenames. Specify when encoding, record separator, and path syntax must be declared. Limit the line-list guarantee to names it can represent, and preserve raw bytes for absolute Unix paths on the wire and in jobs. [Reader and wire rules](/home/michael/dev/gitea_repos/blit_v2/docs/plan/FILES_FROM_LISTS.md:90)

2. **Make source-rooted opening an explicit dependency.** The cited source-containment work handles per-file read failures. `FsTransferSource::open_file` still opens a reconstructed pathname and then checks whether the opened object is a regular file. S4 needs an open anchored to the source root, with the resulting handle used for hashing, packing, and copying. Exercise changes to a parent directory as well as the final entry. [Source opener](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/source.rs:541)

3. **Preserve destination collisions across retries.** If `A` lands and `a` is reported as colliding, a retry containing only `a` has lost the evidence that `A` already owns that destination name. It can overwrite the winner or clear the failure through a comparison skip. Resolve collisions against the whole selection before writing, and persist the losing entry as a terminal failure for automatic passes and `jobs retry`. [Retry selection](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/retry.rs:718)

4. **Specify preflight on the first daemon hop.** `DelegatedPull` accepts one request containing the operation spec. Chunking the subsequent transfer stream cannot get a large list through that first request. Specify how the CLI uploads the selection in bounded messages and receives acknowledgement of the complete list before saving the job or changing the destination. Test an interrupted upload alongside the 100,000-entry case. [Delegated RPC](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/proto/blit.proto:62)

5. **Give the deletion refusal precedence over legacy replay.** AC-11 promises that an old job replays as it did. A saved `mirror --delete-scope all --files-from` job makes that promise incompatible with S3. Preserve legacy parsing for safe copy jobs and explicitly refuse destructive list jobs at replay and at the daemon operation boundary. [Replay contract](/home/michael/dev/gitea_repos/blit_v2/docs/plan/FILES_FROM_LISTS.md:105)

6. **Define what success means for existing copy modes.** `--ignore-existing` can keep different destination contents, `--dry-run` makes no writes, and `--detach` returns when the daemon starts. Distinguish a policy skip, a preview, and an accepted submission from a completed delivery. Define “already in place” under the selected comparison mode. [Copy options](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/cli.rs:425) · [Detached execution](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/remote_remote_direct.rs:156)

I read `AGENTS.md` and resolved the requested head with `git rev-parse`. Both requested documentation checks passed on Linux. Removing the draft’s status header made `check-docs.sh` exit 1. Restoring its exact bytes made it exit 0. A separate guard for the destination-collision acceptance criterion failed on the earlier draft and passed at the pinned head. I removed the disposable export.

```text
base: d8689340813c82725efbee4b56813077bc3e963f
head: 6f70800021ecba4740af86df42fc3711f7c4d538
capability_ok: true
guard_confirmed: true
```