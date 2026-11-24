The goal is for a retry to act on exactly the files that failed and for a resumed write to reach the intended destination, even when filenames look identical or a directory has moved. I would carry each file’s exact identity and incomplete-copy status from the transfer into the log and saved record. On Linux, I would resolve writes from the held destination directory and verify directory identity whenever the fallback must reopen a parent.

**Verdict `acceptable_with_changes`.** The per-failure identity and Linux `openat2` fast path are worth keeping. A 72-directory-deep resume completed with a 64-descriptor limit and produced the correct bytes. I would require the following changes.

| Severity | Observed behavior | Required change |
|---|---|---|
| High | `jobs retry` accepted a crash-recovered log, overwrote an already-landed file, omitted a file that still needed copying, and exited 0. A log with a sequence-number gap produced the same result. | Check sequence continuity and terminal summary coverage before completing a short record. Recovery can append `run-end` to a partial log. [Retry reader](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/jobs.rs:629) · [Log recovery](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/job_log.rs:1129) |
| High | With an older record and a finished log containing no event for its failed file, `--ignore-existing` reported “Up to date” and left the partial bytes in place. | Require affirmative disposition evidence for each failed identity. The current check accepts any identity absent from `left_unknown`, including one the log never mentioned. [Disposition check](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/jobs.rs:452) |
| High | After an in-root parent was renamed and its old name reused, `Walk::up` returned the replacement directory’s inode. A subsequent write can land in the wrong directory. | Verify the identity of the reacquired parent before continuing through `..`. [Parent lookup](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/resume.rs:886) |
| Medium | In a push with 24 pairs of identically displayed names, 12 UTF-8 files landed. The sender logged all 12 as their rejected byte-named siblings. | Associate live events with the granted manifest entry. The sender currently announces raw names for rejected entries too, so a per-pass map still misattributes them. [Manifest reporting](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:2345) · [Log lookup](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/run_log.rs:541) |
| Medium | An absolute alias to the destination root worked. An alias directly to a subdirectory of that root caused both ordinary copy and resume to exit 2 for a file inside the destination. | Verify the resolved directory’s ancestry against the held root for aliases to descendants. [Alias lookup](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/resume.rs:936) |
| Medium | Before manifest resolution, the public encoder returned one path and `truncated=false` for two failures sharing a display name. After resolution it returned both. | Require resolved identities at the encoding boundary or mark the list incomplete when distinct failures collapse in the emitted set. [Failure encoder](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/sink.rs:815) |
| Medium | With `openat2` unavailable through the repository’s test seam, path resolution succeeded but the new fast-path test failed because the fallback held two descriptors. | Condition the fast-path assertion on syscall availability and retain the bounded-descriptor test for the fallback. [Fast-path test](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/mod.rs:1633) |

Linux formatting and strict Clippy passed. The first workspace run exposed two build-identity fixtures that found the outer repository. With Git discovery bounded to the export, **2,142 tests passed and 2 were ignored**. Strict Clippy also passed for Windows GNU and macOS ARM64 as cross-compilation checks.

For the independent guard, I weakened the whole-log check and watched `a_cut_list_is_completed_only_from_a_whole_log` fail. Restoring the source byte for byte made it pass. I verified the reviewed source files against the pinned Git blobs and removed the disposable export.

```text
base_sha: d7f5f5e9026c56ad1f3f04fe26c0f53238a7fb20
head_sha: b6e6fab9ef621f724928ddbed0d6af1bae773271
capability_ok: true
guard_confirmed: true
```