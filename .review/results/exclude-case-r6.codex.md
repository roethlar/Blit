Blit’s goal is to let an operator predict what a filtered mirror will copy, keep, and delete before confirming it. I would resolve one filter per run, compile Unicode case alternatives into the glob, and match filenames in their original bytes. That preserves byte-wide `?` and bracket classes. The source walk, deletion planner, peer, confirmation, and saved job should all use that same interpretation.

**Verdict `acceptable_with_changes`.** The change’s compile-time case folding and explicit wire fields are the right foundation. I would require five changes.

1. **High — make local and peer includes agree.** With the Unix pattern `--include '{a\,**/x,b}'`, the local filter admitted `a/x` and rejected `a,/x`. The decoded wire filter did the reverse in both case modes. A remote mirror can therefore delete a destination-only `a,/x` that a local mirror keeps. Use one meaning-preserving representation and test both builders against the same paths. [Wire include construction](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:245)

2. **High — preserve required path components in the confirmation.** For `--exclude 'Cache/**'`, the prompt said `--ignore-case` would also match `cache`. I ran that mirror with `--ignore-case` and it deleted the destination-only file `cache`, exiting successfully. Keep `/**` in the example and verify suggested spellings against the matcher. [Prompt rendering](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:424)

3. **Medium — version the corrected directory-deletion behavior.** Given the same decoded empty `--files-from` list, head planned no deletion. The planner from `0cf36891` scheduled a destination-only empty directory for deletion. Both use session contract 10 and delegated spec 5, so both version gates admit the older behavior. Bump both versions for this change in deletion scope. [Session contract](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfer_session/mod.rs:127) · [Delegated spec](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/operation_spec.rs:64)

4. **Medium — use the captured exact-name list for execution.** I changed `--files-from` from `first.txt` to `second.txt` while the confirmation was open. The mirror copied `second.txt`. Its saved job replayed `first.txt`. Use the pre-prompt snapshot for local execution, the wire request, and the job record. [List capture](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/run_log.rs:714) · [Filter construction](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:103)

5. **Medium — bound HTTP/2 buffering across concurrent RPCs.** The separate small-frame fix raises the connection window to nearly 2 GiB. Its size proof covers two client streams or four server streams, while the builder permits higher concurrency. Simultaneous slow readers can accumulate substantially more buffered data than the former connection limit allowed. Size the windows and concurrency to an explicit aggregate memory budget and verify it under multiplexed traffic. This finding comes from code inspection. [HTTP/2 window policy](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/grpc_server.rs:62)

**Verification.** Disabling case widening made the mirror guard fail because it copied `aCache`. Restoring the pinned file made the same test pass. Linux formatting, strict workspace Clippy, and the workspace suite passed with 1,694 tests passed and 2 ignored. Windows GNU cross-Clippy passed. Windows and macOS runtime behavior remains unverified. I read `AGENTS.md`, resolved the head with `git rev-parse`, and removed the disposable export.

```text
base: 3bc3ee47612a8d485bafa18101946882b02e85a0
head: a13238b6e7492be8b173899978efb633f3d7d151
capability_ok: true
guard_confirmed: true
```