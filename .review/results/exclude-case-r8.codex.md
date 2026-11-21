I would resolve one filter before a transfer starts and use it for the source walk, destination deletion, `check`, and daemon requests. The goal is for an operator to know, before confirming a mirror, which filenames a pattern protects. Exact matching should be the default on every host. `--ignore-case` should expand Unicode case variants in the compiled glob while preserving filename bytes, byte-wide wildcards, and literal `--files-from` entries. [Approved design](/home/michael/dev/gitea_repos/blit_v2/docs/plan/EXCLUDE_CASE_MATCHING.md:10)

**Verdict — `acceptable_with_changes`.** The change implements that core design, saves pattern-file contents for replay, and carries the case mode in a versioned daemon request. I would keep those choices and require four corrections.

1. **High — an exclude reported as unmatchable can still cause deletion.** I exercised the Windows resolver on Linux with a remote source and `--ignore-case`. It classified `\keep` as unmatchable but sent it as an active glob. A real Linux source daemon interpreted it as `keep`. With `--delete-scope all`, the mirror deleted destination `KEEP` even though source `KEEP` existed. Keep unmatchable patterns for diagnostics and omit them from executable filters on both ends. [Exclude resolution](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:543) · [Wire filter](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:312)

2. **High — the confirmation can omit or misstate which names are at risk.** With `--exclude '{Σ,σ}'`, the CLI showed only the ordinary delete prompt. Confirming deleted destination `ς`, which `--ignore-case` preserved. The ASCII pattern `{[a-c],A,B}` produced the same failure for `C`. Another pattern, `{[!a].1,A.1,[!b].2}`, attributed lost protection to `[!a]` and `A`, although `A.1` stayed protected and `B.2` lost protection. Verify examples against the complete pattern in both modes. When sampling finds no example, show the case rule unless equivalence is established. [Warning suppression](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:400) · [Example selection](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:529)

3. **Medium — one malformed list entry changes the outcome by route.** With an embedded NUL in one `--files-from` entry followed by `ok.txt`, local copy reported the missing entry and copied `ok.txt`. A daemon pull rejected the entire request with `PROTOCOL_VIOLATION`. Classify NUL-bearing entries as unnameable before building the request so valid entries still transfer. [Sender classification](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:108) · [Receiver validation](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/operation_spec.rs:292)

4. **Medium — the HTTP/2 stream cap does not establish the claimed unread-byte bound.** I retained nine unread one-row listing responses on one channel. A finished stream releases its slot while its body can remain buffered, so multiplying eight active streams by their windows does not establish the stated 16 MiB client ceiling. Enforce an aggregate receive limit that lasts until bodies are consumed or dropped, and test completed unread responses. [Connection-window policy](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/grpc_server.rs:62)

Linux formatting and strict workspace Clippy passed. Workspace tests had **1,714 passed and 2 ignored**. Windows cross-target Clippy passed. Native Windows and macOS execution remain unverified, and macOS cross-checking was blocked by missing Darwin headers.

For the independent guard, forcing outgoing `ignore_case` to `false` made the daemon mirror test fail on `aCache`. Restoring the pinned source made the same test pass. I removed the disposable export and verified the shared tree contained only the review’s existing untracked records.

```text
base: 3bc3ee47612a8d485bafa18101946882b02e85a0
head: f4203c3174ab8d16338fd9d4aceef04c2da697be
capability_ok: true
guard_confirmed: true
```