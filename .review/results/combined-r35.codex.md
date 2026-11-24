Blit needs a failed copy or resume to leave a destination that can be retried reliably. I would validate the file handles used for I/O, retain the writable handle through cleanup, and allow a resume to write only where its comparison established that it could. The copy engine should report what it left behind against the file’s exact name bytes, and that result should survive through the wire, job log, and retry.

**Verdict — `acceptable_with_changes`.** The ordinary Linux copy now succeeds through a valid destination symlink and retains its writable handle. A Unix resume refuses a write when it can no longer establish the destination directory. The typed endpoint and resume-error classifications also fit the goal. The remaining problems are at identifiable points where the implementation drops a handle, a file identity, or a known cleanup result.

### Required changes

1. **High — Make acquisition cleanup identify its target.** After validation fails, `create_regular` drops its handle and unlinks `dst` by name. A replacement at that name can be deleted. In the other direction, I confirmed that opening through a dangling symlink can create its target, fail validation, and leave an unreported empty file because the fallback recorded `made = false`. Track creation at the resolved target and guard every fallible step through truncation. [Creation fallback](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/mod.rs:305) · [Failure cleanup](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/mod.rs:335)

2. **High — Retain the writable handle for exclusive local copies.** With `--ignore-existing`, an eight-byte read-only source, a directory that became unwritable after creation, and an injected metadata failure, cleanup left the destination at eight bytes. A later size-only comparison can skip it. Give this branch’s cleanup the writable handle as well, while preserving its exclusive-name check. [Exclusive-copy branch](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:3789)

3. **High — Carry the validated handles through copy fallbacks.** `mmap_copy_file` falls back to `std::fs::copy(src, dst)`, which resolves both names again. The macOS path copier also reopens the source and uses a blocking destination open for `fcopyfile`. A changed name can select a different file, and a FIFO can stall the copy. Use descriptor-based fallbacks and derive the length from the opened source. [Unix fallback](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/mmap.rs:83) · [macOS fallback](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/clone.rs:54)

4. **Medium — Preserve the destination’s cleanup result when the source aborts.** I exercised both a discarding stream writer and a resume with a pending destination error marked `LeftInPlace`. Both reported `left_in_place = false` after the source closed the record as failed. Keep the source’s diagnostic and propagate the destination’s known disposition into the outcome and progress event. [Stream abort](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:2290) · [Resume abort](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:1645)

5. **High — Preserve exact identity and disposition across summary conversion.** A wire round trip changed a reported failure’s flag from `true` to `false`. The separate leftover lists cover many cases, but `RunTotals` drops `failed_paths_raw`. In a summary-only log with 65 failures, the 65th file had a non-UTF-8 name and was left in place. Its log entry had neither its raw name nor its leftover flag. A saved retry can consequently address a different filename. Carry the raw identities past the 64-reason cap and preserve the disposition of each reported failure. [Wire conversion](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:231) · [Log reconstruction](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/run_log.rs:614)

6. **Medium — Treat missing legacy evidence as unknown.** New logs omit `false`, and old logs omit the field altogether. I confirmed that a complete old-format log caused an ambiguous raw-named failure to be classified as having left nothing. An `--ignore-existing` retry can then skip its own incomplete copy. Distinguish explicit `false` from historical absence and refuse the ambiguous retry when the saved record cannot settle it. [Event format](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/job_log.rs:280) · [Retry decision](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/jobs.rs:413)

7. **Medium — Give the unretried count its own record field.** A real file named `(not retried)` was the sole failure in my fixture. With its log unavailable, `jobs retry` discarded the recorded filename and reported that it knew zero of one failed files. Generate the label when rendering the count. [Retry report](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/retry.rs:273) · [Job reader](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/jobs.rs:440)

Two previously recorded Windows gaps also apply. **High:** the receiver discards an `Unjustified` resume baseline and can proceed against a changed destination. **Low:** a metadata read error whose path contains `Windows metadata changed` is reported as size drift. The former needs a validated Windows handle and the latter a typed error. [Resume acquisition](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:2941) · [Metadata classification](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/payload.rs:69)

### Verification

On the restored Linux snapshot, formatting and strict workspace Clippy passed. The workspace suite had **2,073 passed, 0 failed, 2 ignored**. Windows GNU and ARM macOS passed cross-target Clippy, with `blake3/pure` on macOS. Runtime verification was on Linux.

For the independent guard, I restored the old reason-suffix check and watched `a_retried_failure_is_marked_by_the_report_not_its_words` fail. I then restored the pinned blob, verified its hash, forced a rebuild, and watched the same test pass. The repository file read and requested `git rev-parse` both succeeded. I removed the disposable export.

```text
base: 0f912b062a883850a772ff21ae1d645cb62c3f99
head: dc14afc48f43b180668a61115ad3202a9e62ba05
capability_ok: true
guard_confirmed: true
```