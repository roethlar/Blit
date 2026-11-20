I would resolve and validate the patterns once, then use that filter for source selection, `check`, the confirmation message, and a mirror’s filtered deletion pass. The goal is to let an operator predict which names a transfer will copy or preserve, whether it runs locally, through a daemon, or from a saved job. [Matching plan](/home/michael/dev/gitea_repos/blit_v2/docs/plan/EXCLUDE_CASE_MATCHING.md:8)

The change makes the right core choice. It matches exactly by default and widens literal letters in the pattern under `--ignore-case`. That gives Unicode simple case folding while `?` and bracket classes still consume one byte of the original filename. Saving the expanded pattern-file contents and the chosen case mode also gives replay the inputs the operator originally used.

**Verdict** `acceptable_with_changes`

### Required changes

1. **High — version the empty-list behavior.** The pinned CLI accepted a daemon built from `c35e43d4`, an earlier revision pushed to `origin/master`. With a comments-only `--files-from`, `--ignore-case`, and `--include '*.jpg'`, that daemon deleted destination-only `only.JPG` and returned success. The head daemon kept it. Bump both the session contract and delegated-spec version so peers that discard the empty-list marker refuse the operation. [Session contract](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:121) · [Delegated spec version](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/operation_spec.rs:59)

2. **High — validate the remote route before saving or exporting.** A remote `mirror -y --workers 1 --save nightly` replaced a working saved copy job before rejecting `--workers`. The saved spec omitted that diagnostic flag. I ran `jobs run nightly` and it executed the mirror, deleting a destination-only file. Move the existing remote option and endpoint checks ahead of recording and saving. [Pre-save validation](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/mod.rs:617) · [Late remote refusal](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/mod.rs:795)

3. **High — execute the filter shown at confirmation.** An absolute exclude through a source alias was displayed as `first/Keep`. When the alias switched to `second` while the question was open, the confirmed mirror copied `first/Keep`. Re-resolving the pattern can likewise expose a destination-only `first/Keep` to deletion. Carry the resolved excludes into dispatch. [Confirmation](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/mod.rs:689) · [Filter reconstruction](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:63)

4. **High — preserve required directory components in prompt examples.** For `--exclude '*/Cache'`, the prompt said `--ignore-case` would also match `cache`. Running that mirror with `--ignore-case` deleted root-level `cache` and kept `sub/cache`. Show a valid example such as `sub/cache`, or retain the full glob when displaying its other spellings. [Example builder](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:394)

5. **Medium — give an empty exact-name list an empty subset deletion plan.** With the pinned build at both ends, a comments-only list still caused the daemon to remove two unlisted empty directories. A local mirror also removed an empty directory present on both sides. Directories admitted for traversal become deletion candidates. Short-circuit subset deletion when the explicit list is empty. [Directory traversal](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/fs_enum.rs:1101) · [Deletion planner](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/mirror_planner.rs:489)

6. **Medium — explain `--delete-scope all` in the confirmation and README.** The README says the case rule applies equally to source selection and deletion. With `--delete-scope all --exclude '*Cache'`, the usual case note appeared and the mirror deleted the matching destination-only `caseCache`. State that this scope filters the source and permits deletion of excluded destination entries. [README](/home/michael/dev/gitea_repos/blit_v2/README.md:20) · [Deletion-scope selection](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:4252)

### Verification

On Linux, formatting and strict workspace Clippy passed. The full workspace suite passed with **1,681 passed and 2 ignored**. Strict cross-target Clippy passed for Windows GNU and MSVC. Native Windows and macOS behavior remains unverified at this SHA.

For the guard, I removed the receiver’s `files_from_set` check. The focused daemon integration test transferred a file, purged an entry, and failed with exit 101. Restoring the source byte-for-byte made the same test pass. I read `AGENTS.md`, resolved the requested head with `git rev-parse`, and removed the disposable copy.

```text
base_sha: 3bc3ee47612a8d485bafa18101946882b02e85a0
head_sha: 585d35990ccdf8a74aa84b1afa718c25f33feade
capability_ok: true
guard_confirmed: true
```