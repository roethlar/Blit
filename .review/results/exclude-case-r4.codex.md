The goal is to let a user predict which files a transfer will copy and which destination-only files a mirror will preserve. I would resolve and validate one filter per run, use it for copying, checking, deletion, and the confirmation message, and preserve its meaning across daemon calls and saved jobs. The change makes the right core choice: `--ignore-case` widens literal letters using Unicode simple case folding while `?` and bracket classes continue to count the original filename bytes. Versioning both the session and delegated request also protects against older peers silently ignoring the mode.

**Verdict** `acceptable_with_changes`

I would require five changes before accepting the snapshot.

1. **High — escape the whole filename in pattern-file advice.** For the line `- [1].jpg`, blit recommended `[-] [1].jpg`. Following that advice in a mirror deleted the literal `- [1].jpg` and preserved `- 1.jpg`. I verified that `[-] [[]1[]].jpg` preserves the intended file. Use the literal-glob encoder for the remainder of the name. [fs_enum.rs:322](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/fs_enum.rs:322)

2. **High — preserve an explicitly empty `--files-from` list on the wire.** With a comment-only list, `--include-from` containing `*.jpg`, and `--ignore-case`, the local filter selected no files. After encoding and peer decoding, the same filter selected `new.JPG` and put destination-only `only.JPG` in the deletion plan. The wire ambiguity predates this change, but it breaks the promised precedence of exact lists over the new patterns. Encode list presence independently of entry count. [filter.rs:216](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:216) · [operation_spec.rs:224](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/operation_spec.rs:224)

3. **High — derive the confirmation from the effective filter.** For `photo[!x].jpg`, the prompt said `--ignore-case` would also match `PHOTO[!x].JPG`. That displayed glob admits `PHOTOX.JPG`, which the mirror deleted. `photoX.jpg` also lost the protection it had in exact mode. Use a one-line explanation that accounts for negation. Also skip excludes already diagnosed as unmatchable when choosing the example. I observed the same outside-root pattern described as both unmatchable and matching. [case_fold.rs:251](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:251) · [transfers/mod.rs:268](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/transfers/mod.rs:268)

4. **Medium — validate before replacing a saved job.** Both `move --ignore-case --save nightly` and a mirror whose pattern file contained `[aé].txt` replaced a working `nightly` before reporting the error. `jobs run nightly` then failed. Apply the move-option and filter checks before `save_as`. [main.rs:276](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/main.rs:276)

5. **Medium — publish the machine ID atomically.** When only 16 of its 32 digits were present past the read timeout, blit saved a job under the prefix. Once the ID was complete, replay refused the job as belonging to another machine. Publish the complete ID in one operation, or fail an unfinished read at timeout. [job_log.rs:1591](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/job_log.rs:1591)

On Linux, formatting and strict workspace Clippy passed. The workspace suite passed with **1,673 passed and 2 ignored**. Strict Clippy also passed for the Windows GNU and MSVC targets. Runtime coverage was Linux only.

For the independent guard, I disconnected `--ignore-case` from transfer filter construction. The mirror test failed because it copied `aCache/`. I restored the pinned Git blob and the same test passed. I removed the disposable export and confirmed the shared checkout still contained only the review’s untracked records.

```text
base: 3bc3ee47612a8d485bafa18101946882b02e85a0
head: 0669279fb04dcd96454955ba52dd7bc5825030ea
capability_ok: true
guard_confirmed: true
```