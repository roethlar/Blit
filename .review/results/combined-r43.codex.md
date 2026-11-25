I would make `jobs retry` use the exact filename the transfer granted and the destination’s account of whether the failed write left a partial copy. When a saved record is short, local and detached jobs should use one reader that verifies every contributing session before reconstructing the failed set. For ordinary and resumed writes, I would resolve the destination from the directory the transfer holds. The goal is to finish the files still missing while preserving files that already landed.

**Verdict `acceptable_with_changes`.** I would keep the granted-header identity, the failure-bounded disposition tracking, the explicit log-delivery field, and the independent `openat2` probe. I would require the following corrections.

1. **High — Enforce the end of the local log.** I gave `jobs retry` a truncated record and a log with two consecutively numbered file events after `run-end`. It exited 0, overwrote `landed`, and left `missing` absent. Both readers should require a single terminal `summary` followed by `run-end` and end of file. [Local retry reader](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/src/jobs.rs:635)

2. **High — Require delivery evidence from every detached session.** A first session whose summary lacked `all_events_logged` correctly read as incomplete. Adding a second, complete session made the combined result complete, and `settle` produced an untruncated record that retained the first session’s leftover. A missing or false delivery marker in any contributing session must keep the aggregate incomplete. [Session completion](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/job_record.rs:959) · [Summary handling](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/job_record.rs:1055)

3. **High — Confirm that the audit relay drained.** I queued one failure, interrupted a pending close, and retried the close. The log contained zero file failures, yet both `all_events_logged` and `LogReport.complete` were true. Shutting down the relay’s runtime before closing produced the same result. Preserve the pending drain across close attempts and propagate cancellation or failure from every relay in the chain. [Relay close](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/run_log.rs:702) · [Chained relays](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/run_log.rs:511)

4. **Medium — Resolve aliases to a missing file beneath the destination.** On Linux, I pointed `dst/entry` through an absolute alias to `dst/sub/leaf`. Ordinary copy and resume both succeeded with an existing leaf. Both exited 2 and reported that the path led outside when the leaf was absent. Resolve the dangling alias to its existing parent, verify that parent against the held root, and create the leaf there. [Alias resolution](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/copy/file_copy/resume.rs:959)

5. **Medium — Give the GNU `ls` reference a stable diagnostic name.** The pinned Linux CI runner found GNU `ls` as `gnuls`. Both parity tests failed because its diagnostic began `gnuls:` and the expected prefix was `ls:`. Invoke the selected reference with `argv[0]` set to `ls`, or normalize just that program-name prefix. [Reference invocation](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/tests/ls.rs:150) · [Linux CI job](https://github.com/roethlar/Blit/actions/runs/45715973925/job/139561984449)

6. **Medium — Control the sender guard’s manifest order.** Changing only fixture creation order so both byte-named siblings appeared first made the guard panic before it checked the sender’s output. Use a scripted source that releases the sibling after the first header has been requested. [Sender guard](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/tests/transfer_session_roles.rs:3867)

7. **Medium — Check mirror outcomes on case-folding filesystems.** The new probe detects folding independently, but both post-mirror loops then skip every entry. An incorrect deletion of an unambiguous name such as `xA` can pass. Assert the outcomes for distinct entries and exercise colliding spellings in separate fixtures. [Mirror guard](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/tests/case_note.rs:1070)

**Verification.** On Linux/Btrfs, formatting and strict Clippy passed. The workspace suite passed with 2,152 tests passed and 2 ignored. Windows GNU and macOS ARM64 cross-target Clippy also passed. For the independent guard, removing the detached reader’s sequence check made its test fail on two missing middle events. Restoring the exact pinned blob made the same test pass. I removed the disposable export.

The pinned macOS and Windows CI jobs also failed on pre-existing assertions. macOS rejected the `find` fixture’s non-UTF-8 filename. Windows failed the `du` block-count and `df` missing-file diagnostic assertions. [macOS CI job](https://github.com/roethlar/Blit/actions/runs/45715973925/job/139561984126) · [Windows CI job](https://github.com/roethlar/Blit/actions/runs/45715973925/job/139561984510)

```yaml
base_sha: e2c2e08da74fc910ed2ed1a67270af76443c04b8
head_sha: 4e12d761de2f6c897be7ead4baea9e158f27f061
capability_ok: true
guard_confirmed: true
```