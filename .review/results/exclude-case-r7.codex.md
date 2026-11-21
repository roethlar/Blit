I would resolve one filter before asking the user to confirm a mirror, then use that filter for copying, deletion, `check`, and peer requests. The goal is for users to know which names their patterns protect on this run. The change’s central choice is sound. Widening literal letters when compiling the glob preserves Unicode simple case folding while keeping `?` and bracket classes one byte wide. Saving the expanded pattern files and versioning both peer protocols also support that goal.

**Verdict** `acceptable_with_changes`

I would require four corrections.

1. **High — preserve literal path components across peers.** A Windows `--files-from` entry `sub\name.txt` names a nested file. The sender retains the native separator. I replayed that wire value through the Linux deletion planner with `sub/name.txt` in the source manifest. It selected the unrelated destination file literally named `sub\name.txt` for deletion. Sending `sub/name.txt` kept it. This encoding predates the change, but the new deletion rule relies on it to identify exactly what was listed. Use the existing component-wise POSIX encoder, validate components on receipt, and version the correction for mixed peers. [Sender](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/transfers/filter.rs:312) · [Receiver](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/remote/transfer/operation_spec.rs:239)
2. **High — preserve meaningful separators in prompt examples.** For `--exclude '**//Cache'`, the prompt said `--ignore-case` would also match `cache`. A real mirror with that option deleted the destination-only `cache` and exited successfully. The formatter strips every separator after `**`. Keep the full glob whenever shortening it would change what the example names. [Example formatter](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:415)
3. **Medium — disclose narrowing in later patterns.** With `--exclude Keep --exclude '[!a].dat'`, the prompt described the extra spellings of `Keep` and counted the second pattern. The exact-mode mirror kept `A.dat`. The ignore-case mirror deleted it. Preserve the one-line format and say when another exclusion would stop protecting names. [Multiple-pattern warning](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:121)
4. **Medium — choose an example whose matches actually change.** With `--exclude '{a,A}' --exclude '*.Doc'`, the prompt chose `{a,A}` and offered `{a,a}` and `{A,A}` as additional matches. Both were already covered. The actual change was that `x.doc` gained protection. Use a spelling that demonstrates a real difference, or use neutral wording when one cannot be established. [Example selection](/home/michael/dev/gitea_repos/blit_v2/crates/blit-core/src/case_fold.rs:218)

On Linux, formatting, strict workspace Clippy, and workspace tests passed with **1,704 passed and 2 ignored**. Windows GNU cross-Clippy passed. The mixed-platform finding used the Linux receiver with the Windows wire spelling. `git diff --check` reported trailing whitespace on 12 lines of the captured [benchmark output](/home/michael/dev/gitea_repos/blit_v2/docs/bench/h2-small-frames-2026-11-20/evidence.txt:4).

I read `AGENTS.md` and resolved the requested head with `git rev-parse`. For the guard, disabling literal-letter widening made the mirror test fail because it copied `aCache/`. Restoring the pinned blob made the same test pass. The disposable copy was removed. [Guard test](/home/michael/dev/gitea_repos/blit_v2/crates/blit-cli/tests/ignore_case.rs:61)

```text
base: 3bc3ee47612a8d485bafa18101946882b02e85a0
head: cbab0c39e56046cd926ce03942e08ce566543dbd
capability_ok: true
guard_confirmed: true
```